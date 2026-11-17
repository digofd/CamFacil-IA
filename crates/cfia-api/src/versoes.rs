//! "Gerar sua versão" (SDD-12): a fila de compilação.
//!
//! Técnico (logado):
//! - `POST /api/versoes` `{receita}`: valida, carimba versão base, autor e data,
//!   assina e põe na fila (uma por vez, até 3 por dia).
//! - `GET  /api/versoes`: as versões dele.
//! - `GET  /api/versoes/{id}`: estado, etapa, porcentagem, posição na fila e tempo estimado.
//! - `GET  /api/versoes/{id}/receita.json` e `/executavel`: downloads (só o dono).
//!
//! Compilador (`cfia-builder`, com `Authorization: Bearer <CFIA_BUILDER_TOKEN>`):
//! - `POST /api/builder/pegar`: o próximo da fila (204 se vazia).
//! - `POST /api/builder/{id}/progresso` `{etapa, percentual}`.
//! - `PUT  /api/builder/{id}/executavel`: o `.exe` pronto (corpo cru).
//! - `POST /api/builder/{id}/erro` `{mensagem}`.
//!
//! O compilador busca o trabalho: a máquina dele não abre porta nenhuma.

use crate::auth::Tecnico;
use crate::db::{self, Ctx};
use crate::util::sha256_hex;
use crate::{erro, interno, AppState, Falha};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use cf_versao::receita::{Autor, Base, ReceitaVersao};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use uuid::Uuid;

type Res = Result<Json<Value>, Falha>;

/// Builds por técnico em 24 horas.
const MAX_POR_DIA: i64 = 3;
/// Build parado há mais que isso é dado como perdido.
const TEMPO_MAXIMO_MIN: i32 = 40;
/// Maior executável aceito do compilador.
pub const EXECUTAVEL_MAX: usize = 40 * 1024 * 1024;
const ETAPAS: &[&str] = &["preparando", "compilando", "montando"];

/// Configuração da função, lida do ambiente.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Onde ficam os executáveis prontos (`CFIA_VERSOES_DIR`).
    pub dir: PathBuf,
    /// Token do compilador (`CFIA_BUILDER_TOKEN`, 32+ caracteres).
    pub token: Option<String>,
    /// Commit da versão oficial usada como base (`CFIA_BASE_COMMIT`).
    pub base_commit: Option<String>,
}

impl Config {
    pub fn do_ambiente() -> Self {
        let env = |n: &str| std::env::var(n).ok().filter(|v| !v.trim().is_empty());
        let token = env("CFIA_BUILDER_TOKEN").filter(|t| {
            let ok = t.len() >= 32;
            if !ok {
                eprintln!("aviso: CFIA_BUILDER_TOKEN curto (mínimo 32); compilador desligado");
            }
            ok
        });
        let base_commit = env("CFIA_BASE_COMMIT").filter(|c| {
            let ok = (7..=40).contains(&c.len()) && c.chars().all(|x| x.is_ascii_hexdigit());
            if !ok {
                eprintln!("aviso: CFIA_BASE_COMMIT inválido; Gerar sua versão desligado");
            }
            ok
        });
        Self {
            dir: env("CFIA_VERSOES_DIR")
                .unwrap_or_else(|| "/dados/versoes".into())
                .into(),
            token,
            base_commit,
        }
    }

    fn arquivo(&self, id: Uuid) -> PathBuf {
        self.dir.join(format!("{id}.exe"))
    }
}

/// Versão do termo de responsabilidade que o técnico aceita no assistente.
/// Mudou o texto do termo em `web/versao.html`, muda esta data.
pub const TERMO: &str = "2026-11-17";

#[derive(Deserialize)]
pub struct Pedido {
    receita: Value,
    /// A versão do termo aceito ([`TERMO`]).
    #[serde(default)]
    termo: String,
}

/// O servidor decide a base, o autor e a data: o técnico não escolhe. Uma
/// receita importada (com base, autor e hash antigos) vira um pedido novo.
fn montar(mut v: Value, commit: &str, autor: String) -> Result<ReceitaVersao, Falha> {
    let obj = v
        .as_object_mut()
        .ok_or_else(|| erro(StatusCode::BAD_REQUEST, "receita inválida"))?;
    let base = Base {
        versao: env!("CARGO_PKG_VERSION").into(),
        commit: commit.into(),
    };
    let autor = Autor {
        nome: autor,
        gerado_em: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(interno)?,
    };
    obj.insert("base".into(), serde_json::to_value(base).map_err(interno)?);
    obj.insert(
        "autor".into(),
        serde_json::to_value(autor).map_err(interno)?,
    );
    obj.remove("hash");
    let r: ReceitaVersao = serde_json::from_value(v)
        .map_err(|e| erro(StatusCode::BAD_REQUEST, &format!("receita inválida: {e}")))?;
    let r = r.assinada();
    r.validar().map_err(|e| erro(StatusCode::BAD_REQUEST, &e))?;
    Ok(r)
}

/// `GET /api/versoes/catalogo` (público): o que o assistente oferece.
pub async fn catalogo(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "ligado": s.versoes.base_commit.is_some(),
        "versao_base": env!("CARGO_PKG_VERSION"),
        "inteligencias": cf_versao::receita::INTELIGENCIAS,
        "classes": &cf_versao::coco::CLASSES[..],
        "padrao": cf_versao::coco::PADRAO,
        "logo_max": cf_versao::receita::LOGO_MAX,
        "por_dia": MAX_POR_DIA,
        "termo": TERMO,
    }))
}

#[derive(Deserialize)]
pub struct Conferir {
    receita: Value,
}

/// `POST /api/versoes/conferir` (público): confere a receita, sem gravar nada.
pub async fn conferir(Json(c): Json<Conferir>) -> Res {
    let r = montar(c.receita, "0000000", "Técnico".into())?;
    Ok(Json(json!({ "ok": true, "nome": r.identidade.nome })))
}

/// Nome da conta como autor da receita, dentro das regras do campo.
fn nome_autor(t: &Tecnico) -> String {
    let base = if t.nome.trim().is_empty() {
        t.email.split('@').next().unwrap_or("")
    } else {
        t.nome.as_str()
    };
    let limpo: String = base
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '<' | '>' | '&' | '"' | '\\'))
        .take(60)
        .collect();
    let limpo = limpo.trim().to_string();
    if limpo.chars().count() < 2 {
        "Técnico".into()
    } else {
        limpo
    }
}

pub async fn pedir(
    State(s): State<AppState>,
    t: Tecnico,
    headers: HeaderMap,
    Json(p): Json<Pedido>,
) -> Res {
    let commit = s.versoes.base_commit.clone().ok_or_else(|| {
        erro(
            StatusCode::SERVICE_UNAVAILABLE,
            "Gerar sua versão ainda não está ligado neste servidor",
        )
    })?;
    if p.termo != TERMO {
        return Err(erro(
            StatusCode::BAD_REQUEST,
            "aceite o termo de responsabilidade da versão",
        ));
    }
    let r = montar(p.receita, &commit, nome_autor(&t))?;
    let receita = serde_json::to_value(&r).map_err(interno)?;
    let hash = r.hash.clone().unwrap_or_default();

    let mut c = s.db.get().await.map_err(interno)?;
    let tx = db::begin(&mut c, Ctx::Tecnico(t.id))
        .await
        .map_err(interno)?;
    let hoje: i64 = tx
        .query_one(
            "SELECT count(*) FROM versoes WHERE tecnico_id = $1 AND criado_em > now() - interval '1 day'",
            &[&t.id],
        )
        .await
        .map_err(interno)?
        .get(0);
    if hoje >= MAX_POR_DIA {
        return Err(erro(
            StatusCode::TOO_MANY_REQUESTS,
            "você já gerou 3 versões nas últimas 24 horas; tente amanhã",
        ));
    }
    let id = Uuid::new_v4();
    let ins = tx
        .execute(
            "INSERT INTO versoes(id, tecnico_id, nome, receita, receita_sha256, base_commit)
             VALUES ($1, $2, $3, $4, $5, $6)",
            &[&id, &t.id, &r.identidade.nome, &receita, &hash, &commit],
        )
        .await;
    if let Err(e) = ins {
        if e.code() == Some(&tokio_postgres::error::SqlState::UNIQUE_VIOLATION) {
            return Err(erro(
                StatusCode::CONFLICT,
                "você já tem uma versão na fila; espere ela ficar pronta",
            ));
        }
        return Err(interno(e));
    }
    db::auditar(
        &tx,
        Some(t.id),
        "versao_pedida",
        &format!("{hash} termo {TERMO}"),
        &crate::ip(&headers),
    )
    .await
    .map_err(interno)?;
    tx.commit().await.map_err(interno)?;
    Ok(Json(json!({ "id": id, "estado": "fila", "hash": hash })))
}

const COLS: &str = "id, nome, estado, etapa, percentual, erro, receita_sha256, base_commit, \
     executavel_bytes, \
     to_char(criado_em AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), \
     to_char(terminado_em AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')";

fn resumo(r: &tokio_postgres::Row) -> Value {
    json!({
        "id": r.get::<_, Uuid>(0),
        "nome": r.get::<_, String>(1),
        "estado": r.get::<_, String>(2),
        "etapa": r.get::<_, String>(3),
        "percentual": r.get::<_, i16>(4),
        "erro": r.get::<_, Option<String>>(5),
        "hash": r.get::<_, String>(6),
        "base_commit": r.get::<_, String>(7),
        "executavel_bytes": r.get::<_, Option<i64>>(8),
        "criado_em": r.get::<_, String>(9),
        "terminado_em": r.get::<_, Option<String>>(10),
    })
}

pub async fn minhas(State(s): State<AppState>, t: Tecnico) -> Res {
    let mut c = s.db.get().await.map_err(interno)?;
    let tx = db::begin(&mut c, Ctx::Tecnico(t.id))
        .await
        .map_err(interno)?;
    let rows = tx
        .query(
            &format!("SELECT {COLS} FROM versoes ORDER BY criado_em DESC LIMIT 20"),
            &[],
        )
        .await
        .map_err(interno)?;
    tx.commit().await.map_err(interno)?;
    Ok(Json(
        json!({ "versoes": rows.iter().map(resumo).collect::<Vec<_>>() }),
    ))
}

pub async fn ver(State(s): State<AppState>, t: Tecnico, Path(id): Path<Uuid>) -> Res {
    let mut c = s.db.get().await.map_err(interno)?;
    let tx = db::begin(&mut c, Ctx::Tecnico(t.id))
        .await
        .map_err(interno)?;
    let row = tx
        .query_opt(&format!("SELECT {COLS} FROM versoes WHERE id = $1"), &[&id])
        .await
        .map_err(interno)?
        .ok_or_else(|| erro(StatusCode::NOT_FOUND, "versão não encontrada"))?;
    tx.commit().await.map_err(interno)?;
    let mut v = resumo(&row);
    // Posição e tempo estimado olham a fila de todos (só números, sem dono).
    let tx = db::begin(&mut c, Ctx::Sistema).await.map_err(interno)?;
    let fila: i64 = tx
        .query_one(
            "SELECT count(*) FROM versoes WHERE estado IN ('fila', 'compilando')
             AND criado_em < (SELECT criado_em FROM versoes WHERE id = $1)",
            &[&id],
        )
        .await
        .map_err(interno)?
        .get(0);
    let media: Option<f64> = tx
        .query_one(
            "SELECT avg(extract(epoch FROM terminado_em - iniciado_em))::float8 FROM
             (SELECT terminado_em, iniciado_em FROM versoes WHERE estado = 'pronta'
              ORDER BY terminado_em DESC LIMIT 10) ultimas",
            &[],
        )
        .await
        .map_err(interno)?
        .get(0);
    tx.commit().await.map_err(interno)?;
    if v["estado"] == "fila" {
        v["posicao"] = json!(fila + 1);
    }
    v["tempo_medio_s"] = json!(media.map(|m| m.round() as i64));
    Ok(Json(v))
}

async fn dono_e_receita(
    s: &AppState,
    t: &Tecnico,
    id: Uuid,
) -> Result<(String, String, Value), Falha> {
    let mut c = s.db.get().await.map_err(interno)?;
    let tx = db::begin(&mut c, Ctx::Tecnico(t.id))
        .await
        .map_err(interno)?;
    let row = tx
        .query_opt(
            "SELECT nome, estado, receita FROM versoes WHERE id = $1",
            &[&id],
        )
        .await
        .map_err(interno)?
        .ok_or_else(|| erro(StatusCode::NOT_FOUND, "versão não encontrada"))?;
    tx.commit().await.map_err(interno)?;
    Ok((row.get(0), row.get(1), row.get(2)))
}

/// Nome de arquivo seguro a partir do nome da versão.
fn arquivo_de(nome: &str) -> String {
    let mut s = String::new();
    for c in nome.to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() {
        "minha-versao".into()
    } else {
        s
    }
}

pub async fn baixar_receita(
    State(s): State<AppState>,
    t: Tecnico,
    Path(id): Path<Uuid>,
) -> Result<Response, Falha> {
    let (nome, _, receita) = dono_e_receita(&s, &t, id).await?;
    let corpo = serde_json::to_string_pretty(&receita).map_err(interno)?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/json; charset=utf-8".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!(
                    "attachment; filename=\"receita-{}.json\"",
                    arquivo_de(&nome)
                ),
            ),
        ],
        corpo,
    )
        .into_response())
}

pub async fn baixar_executavel(
    State(s): State<AppState>,
    t: Tecnico,
    Path(id): Path<Uuid>,
) -> Result<Response, Falha> {
    let (nome, estado, _) = dono_e_receita(&s, &t, id).await?;
    if estado != "pronta" {
        return Err(erro(StatusCode::CONFLICT, "a versão ainda não está pronta"));
    }
    let bytes = tokio::fs::read(s.versoes.arquivo(id)).await.map_err(|_| {
        erro(
            StatusCode::GONE,
            "o executável não está mais aqui; gere de novo pela receita",
        )
    })?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.microsoft.portable-executable".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}.exe\"", arquivo_de(&nome)),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        bytes,
    )
        .into_response())
}

// ---------- Compilador ----------

/// Confere o token do compilador sem vazar tempo de comparação.
fn compilador(s: &AppState, headers: &HeaderMap) -> Result<(), Falha> {
    let negado = || erro(StatusCode::UNAUTHORIZED, "compilador não autorizado");
    let esperado = s.versoes.token.as_deref().ok_or_else(negado)?;
    let veio = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(negado)?;
    let (a, b) = (sha256_hex(esperado.as_bytes()), sha256_hex(veio.as_bytes()));
    let diferenca = a
        .bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y));
    if diferenca == 0 {
        Ok(())
    } else {
        Err(negado())
    }
}

pub async fn pegar(State(s): State<AppState>, headers: HeaderMap) -> Result<Response, Falha> {
    compilador(&s, &headers)?;
    let mut c = s.db.get().await.map_err(interno)?;
    let tx = db::begin(&mut c, Ctx::Sistema).await.map_err(interno)?;
    tx.execute(
        &format!(
            "UPDATE versoes SET estado = 'erro', etapa = 'erro', terminado_em = now(),
                 erro = 'o compilador parou de responder; gere de novo'
             WHERE estado = 'compilando' AND iniciado_em < now() - interval '{TEMPO_MAXIMO_MIN} minutes'"
        ),
        &[],
    )
    .await
    .map_err(interno)?;
    let row = tx
        .query_opt(
            "SELECT id, receita, base_commit FROM versoes WHERE estado = 'fila'
             ORDER BY criado_em LIMIT 1 FOR UPDATE SKIP LOCKED",
            &[],
        )
        .await
        .map_err(interno)?;
    let Some(row) = row else {
        tx.commit().await.map_err(interno)?;
        return Ok(StatusCode::NO_CONTENT.into_response());
    };
    let id: Uuid = row.get(0);
    tx.execute(
        "UPDATE versoes SET estado = 'compilando', etapa = 'preparando', percentual = 0,
             iniciado_em = now() WHERE id = $1",
        &[&id],
    )
    .await
    .map_err(interno)?;
    tx.commit().await.map_err(interno)?;
    Ok(Json(json!({
        "id": id,
        "receita": row.get::<_, Value>(1),
        "commit": row.get::<_, String>(2),
    }))
    .into_response())
}

/// Atualiza um build que está compilando; 409 se ele não está mais.
async fn atualizar(s: &AppState, id: Uuid, sql: &str, extra: &str) -> Result<(), Falha> {
    let mut c = s.db.get().await.map_err(interno)?;
    let tx = db::begin(&mut c, Ctx::Sistema).await.map_err(interno)?;
    let n = tx
        .execute(
            &format!("UPDATE versoes SET {sql} WHERE id = $1 AND estado = 'compilando'"),
            &[&id, &extra],
        )
        .await
        .map_err(interno)?;
    tx.commit().await.map_err(interno)?;
    if n == 0 {
        return Err(erro(StatusCode::CONFLICT, "esse build não está compilando"));
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct Progresso {
    etapa: String,
    percentual: i16,
}

pub async fn progresso(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(p): Json<Progresso>,
) -> Res {
    compilador(&s, &headers)?;
    if !ETAPAS.contains(&p.etapa.as_str()) {
        return Err(erro(StatusCode::BAD_REQUEST, "etapa desconhecida"));
    }
    let pct = p.percentual.clamp(0, 99);
    atualizar(
        &s,
        id,
        &format!("etapa = $2, percentual = GREATEST(percentual, {pct})"),
        &p.etapa,
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn executavel(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    corpo: Bytes,
) -> Res {
    compilador(&s, &headers)?;
    if corpo.len() > EXECUTAVEL_MAX || !corpo.starts_with(b"MZ") {
        return Err(erro(
            StatusCode::BAD_REQUEST,
            "isso não é um executável do Windows",
        ));
    }
    tokio::fs::create_dir_all(&s.versoes.dir)
        .await
        .map_err(interno)?;
    let destino = s.versoes.arquivo(id);
    let tmp = destino.with_extension("parcial");
    tokio::fs::write(&tmp, &corpo).await.map_err(interno)?;
    tokio::fs::rename(&tmp, &destino).await.map_err(interno)?;
    let hash = sha256_hex(&corpo);
    let r = atualizar(
        &s,
        id,
        &format!(
            "estado = 'pronta', etapa = 'pronta', percentual = 100, terminado_em = now(),
             executavel_bytes = {}, executavel_sha256 = $2",
            corpo.len()
        ),
        &hash,
    )
    .await;
    if r.is_err() {
        let _ = tokio::fs::remove_file(&destino).await;
    }
    r?;
    Ok(Json(json!({ "ok": true, "sha256": hash })))
}

#[derive(Deserialize)]
pub struct Erro {
    mensagem: String,
}

pub async fn falhou(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(e): Json<Erro>,
) -> Res {
    compilador(&s, &headers)?;
    let msg: String = e
        .mensagem
        .chars()
        .filter(|c| !c.is_control())
        .take(300)
        .collect();
    atualizar(
        &s,
        id,
        "estado = 'erro', etapa = 'erro', terminado_em = now(), erro = $2",
        &msg,
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nome_de_arquivo() {
        assert_eq!(arquivo_de("Vigia Silva"), "vigia-silva");
        assert_eq!(arquivo_de("  Câmeras & Cia. "), "cameras-cia");
        assert_eq!(arquivo_de("../../etc"), "etc");
        assert_eq!(arquivo_de("***"), "minha-versao");
    }

    #[test]
    fn autor_vem_da_conta() {
        let t = |nome: &str, email: &str| Tecnico {
            id: Uuid::nil(),
            email: email.into(),
            nome: nome.into(),
        };
        assert_eq!(nome_autor(&t("Fulano \"Zé\" <x>", "f@x")), "Fulano Zé x");
        assert_eq!(nome_autor(&t("", "joao.silva@gmail.com")), "joao.silva");
        assert_eq!(nome_autor(&t("", "a@b")), "Técnico");
    }
}
