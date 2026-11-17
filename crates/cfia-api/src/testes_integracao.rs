//! Testes de integração contra Postgres real (SDD-08, critérios de aceite).
//! Rodam quando `TEST_DATABASE_URL` está definida; senão, são ignorados.

use crate::auth::{criar_sessao_teste, COOKIE};
use crate::db::{self, Ctx};
use crate::util::{sha256_hex, B64};
use crate::{api, auth, hub, router, AppState, Inner};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use tower::ServiceExt;
use uuid::Uuid;

const BASE: &str = "https://teste.local";
const TOKEN_COMPILADOR: &str = "token-do-compilador-de-teste-0123456789";

async fn estado() -> Option<AppState> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::pool(&url).unwrap();
    db::migrate(&pool).await.unwrap();
    Some(AppState(Arc::new(Inner {
        db: pool,
        http: reqwest::Client::new(),
        google: None,
        jwks: auth::Jwks::default(),
        hub: hub::Hub::default(),
        limite: api::Limite::default(),
        base_url: BASE.into(),
        versoes: crate::versoes::Config {
            dir: std::env::temp_dir().join(format!("cfia-versoes-{}", Uuid::new_v4())),
            token: Some(TOKEN_COMPILADOR.into()),
            base_commit: Some("e46a8b7".into()),
        },
        correio: None,
    })))
}

async fn chamar(
    s: &AppState,
    metodo: &str,
    rota: &str,
    sessao: Option<&str>,
    corpo: Option<Value>,
) -> (StatusCode, Value) {
    let mut r = Request::builder()
        .method(metodo)
        .uri(rota)
        .header("origin", BASE);
    if let Some(t) = sessao {
        r = r.header("cookie", format!("{COOKIE}={t}"));
    }
    let req = match corpo {
        Some(v) => r
            .header("content-type", "application/json")
            .body(Body::from(v.to_string())),
        None => r.body(Body::empty()),
    }
    .unwrap();
    let res = router(s.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn chave_dispositivo_fake() -> String {
    let mut spki = vec![0x30, 0x59, 0x30, 0x13];
    let mut resto = [0u8; 87];
    getrandom::fill(&mut resto).unwrap();
    spki.extend_from_slice(&resto);
    B64.encode(spki)
}

async fn agente_de(s: &AppState, dono: Option<Uuid>) -> Uuid {
    let mut c = s.db.get().await.unwrap();
    let tx = db::begin(&mut c, Ctx::Sistema).await.unwrap();
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO agentes(id, chave_publica, dono_id) VALUES ($1, $2, $3)",
        &[&id, &format!("chave-{id}"), &dono],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

macro_rules! ou_pula {
    ($e:expr) => {
        match $e {
            Some(s) => s,
            None => {
                eprintln!("TEST_DATABASE_URL ausente: teste de integração ignorado");
                return;
            }
        }
    };
}

#[tokio::test]
async fn tecnico_b_nao_ve_nada_do_tecnico_a() {
    let s = ou_pula!(estado().await);
    let (a, sa) = criar_sessao_teste(&s, &format!("a-{}", Uuid::new_v4()), "a@teste").await;
    let (_b, sb) = criar_sessao_teste(&s, &format!("b-{}", Uuid::new_v4()), "b@teste").await;
    let agente = agente_de(&s, Some(a)).await;

    // A vê o próprio agente.
    let (st, v) = chamar(&s, "GET", "/api/agentes", Some(&sa), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(v["agentes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == agente.to_string()));

    // B não vê, e editar dá 404 (não revela que existe).
    let (_, v) = chamar(&s, "GET", "/api/agentes", Some(&sb), None).await;
    assert!(!v["agentes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == agente.to_string()));
    let (st, _) = chamar(
        &s,
        "PATCH",
        &format!("/api/agentes/{agente}"),
        Some(&sb),
        Some(json!({"apelido_cliente": "invasor"})),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Dispositivo de A: B não revoga.
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/dispositivos",
        Some(&sa),
        Some(json!({"chave_publica": chave_dispositivo_fake(), "apelido": "notebook"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let disp = v["id"].as_str().unwrap().to_string();
    let (st, _) = chamar(
        &s,
        "DELETE",
        &format!("/api/dispositivos/{disp}"),
        Some(&sb),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = chamar(
        &s,
        "DELETE",
        &format!("/api/dispositivos/{disp}"),
        Some(&sa),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
}

#[tokio::test]
async fn rls_bloqueia_mesmo_sem_where() {
    let s = ou_pula!(estado().await);
    let (a, _) = criar_sessao_teste(&s, &format!("a-{}", Uuid::new_v4()), "a@teste").await;
    let (b, _) = criar_sessao_teste(&s, &format!("b-{}", Uuid::new_v4()), "b@teste").await;
    agente_de(&s, Some(a)).await;
    let mut c = s.db.get().await.unwrap();
    let tx = db::begin(&mut c, Ctx::Tecnico(b)).await.unwrap();
    // Consulta "esquecendo" o filtro: o banco ainda assim só devolve o que é de B.
    let n: i64 = tx
        .query_one("SELECT count(*) FROM agentes WHERE dono_id = $1", &[&a])
        .await
        .unwrap()
        .get(0);
    assert_eq!(n, 0);
    let n: i64 = tx
        .query_one("SELECT count(*) FROM tecnicos", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(n, 1, "B só enxerga a si mesmo");
    let n: i64 = tx
        .query_one("SELECT count(*) FROM pareamentos", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(n, 0, "pareamentos só no contexto do sistema");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn sem_login_e_sem_origem() {
    let s = ou_pula!(estado().await);
    let (st, _) = chamar(&s, "GET", "/api/agentes", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    // POST sem Origin é recusado mesmo com sessão.
    let (_, sa) = criar_sessao_teste(&s, &format!("a-{}", Uuid::new_v4()), "a@teste").await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/sair")
        .header("cookie", format!("{COOKIE}={sa}"))
        .body(Body::empty())
        .unwrap();
    let res = router(s.clone()).oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn pareamento_completo_com_confirmacao_local() {
    let s = ou_pula!(estado().await);
    let (a, sa) = criar_sessao_teste(&s, &format!("a-{}", Uuid::new_v4()), "a@teste").await;
    let (_b, sb) = criar_sessao_teste(&s, &format!("b-{}", Uuid::new_v4()), "b@teste").await;
    let agente = agente_de(&s, None).await;

    // Agente "on-line" com um canal falso.
    let (tx_ag, mut rx_ag) = unbounded_channel::<String>();
    s.hub.registrar_agente_teste(agente, tx_ag.clone());

    // O agente publica o hash do código.
    let codigo = format!("COD-{}", Uuid::new_v4());
    hub::tratar_agente(
        &s,
        agente,
        &json!({"tipo": "novo_codigo", "codigo_hash": sha256_hex(codigo.as_bytes())}),
        &tx_ag,
    )
    .await
    .unwrap();
    assert!(rx_ag.recv().await.unwrap().contains("codigo_ok"));

    // Código errado: 400.
    let (_, v) = chamar(
        &s,
        "POST",
        "/api/dispositivos",
        Some(&sa),
        Some(json!({"chave_publica": chave_dispositivo_fake()})),
    )
    .await;
    let disp_a = v["id"].as_str().unwrap().to_string();
    let (st, _) = chamar(
        &s,
        "POST",
        "/api/parear",
        Some(&sa),
        Some(json!({"codigo": "errado", "dispositivo_id": disp_a})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // B não pode usar o dispositivo de A.
    let (st, _) = chamar(
        &s,
        "POST",
        "/api/parear",
        Some(&sb),
        Some(json!({"codigo": codigo, "dispositivo_id": disp_a})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // A pede o pareamento: o agente recebe o pedido com o e-mail de A.
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/parear",
        Some(&sa),
        Some(json!({"codigo": codigo, "dispositivo_id": disp_a})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let pedido: Value = serde_json::from_str(&rx_ag.recv().await.unwrap()).unwrap();
    assert_eq!(pedido["tipo"], "pedido_pareamento");
    assert_eq!(pedido["email"], "a@teste");

    // Código já consumido: segunda tentativa falha.
    let (st, _) = chamar(
        &s,
        "POST",
        "/api/parear",
        Some(&sa),
        Some(json!({"codigo": codigo, "dispositivo_id": disp_a})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Confirmação na tela local.
    hub::tratar_agente(
        &s,
        agente,
        &json!({"tipo": "confirmar_pareamento", "pedido_id": pedido["pedido_id"], "aceito": true}),
        &tx_ag,
    )
    .await
    .unwrap();
    let mut c = s.db.get().await.unwrap();
    let tx = db::begin(&mut c, Ctx::Sistema).await.unwrap();
    let dono: Option<Uuid> = tx
        .query_one("SELECT dono_id FROM agentes WHERE id = $1", &[&agente])
        .await
        .unwrap()
        .get(0);
    tx.commit().await.unwrap();
    assert_eq!(dono, Some(a));

    // Já pareado: novo código é recusado pelo próprio servidor.
    hub::tratar_agente(
        &s,
        agente,
        &json!({"tipo": "novo_codigo", "codigo_hash": sha256_hex(b"outro")}),
        &tx_ag,
    )
    .await
    .unwrap();
    let _confirmacao = rx_ag.recv().await.unwrap();
    assert!(rx_ag.recv().await.unwrap().contains("já pareado"));

    // O agente manda o resumo: só o dono enxerga, já filtrado.
    hub::tratar_agente(
        &s,
        agente,
        &json!({"tipo": "estado", "estado": {"cameras": [{"nome": "Portão", "online": true, "fps": 5.0, "url": "rtsp://x"}], "cpu": 20.0}}),
        &tx_ag,
    )
    .await
    .unwrap();
    let (_, v) = chamar(&s, "GET", "/api/agentes", Some(&sa), None).await;
    let meu = v["agentes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == json!(agente))
        .unwrap();
    assert_eq!(meu["estado"]["cameras"][0]["nome"], "Portão");
    assert!(!meu["estado"].to_string().contains("rtsp://"));
    let (_, v) = chamar(&s, "GET", "/api/agentes", Some(&sb), None).await;
    assert!(!v.to_string().contains("Portão"));
}

#[tokio::test]
async fn selo_da_escola_exige_login_e_e_publico_por_codigo() {
    let Some(s) = estado().await else { return };
    let (_a, sa) = criar_sessao_teste(&s, &format!("a-{}", Uuid::new_v4()), "a@teste").await;
    let (_b, sb) = criar_sessao_teste(&s, &format!("b-{}", Uuid::new_v4()), "b@teste").await;

    // Sem login não gera.
    let (st, _) = chamar(&s, "POST", "/api/escola/selo", None, Some(json!({}))).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Gera com Instagram limpo (sem @, minúsculo) e autorização.
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/escola/selo",
        Some(&sa),
        Some(json!({"instagram": "@Emerson.Assis", "autoriza_repost": true})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["selo"]["instagram"], "emerson.assis");
    assert_eq!(v["selo"]["autoriza_repost"], true);
    let codigo = v["selo"]["codigo"].as_str().unwrap().to_string();

    // Instagram inválido cai fora e, sem Instagram, não há o que repostar.
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/escola/selo",
        Some(&sa),
        Some(json!({"instagram": "tem espaço!", "autoriza_repost": true})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(v["selo"]["instagram"].is_null());
    assert_eq!(v["selo"]["autoriza_repost"], false);
    // Um por técnico: o código não muda ao regerar.
    assert_eq!(v["selo"]["codigo"], json!(codigo));

    // O outro técnico não vê o selo como seu; a página pública vê pelo código.
    let (_, v) = chamar(&s, "GET", "/api/escola/selo", Some(&sb), None).await;
    assert!(v["selo"].is_null());
    let (st, v) = chamar(&s, "GET", &format!("/api/escola/selo/{codigo}"), None, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["selo"]["codigo"], json!(codigo));
    let (st, _) = chamar(&s, "GET", "/api/escola/selo/nao-existe", None, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Modo estúdio: receitas compartilhadas (SDD-07, ADR 0013)

fn receita_teste(slug: &str) -> Value {
    let mut r = cf_core::receita::exemplo();
    r.id = slug.into();
    r.estudo.metricas = Some(cf_core::receita::Metricas {
        map50: Some(0.81),
        ..Default::default()
    });
    serde_json::from_str(&r.canonico()).unwrap()
}

async fn chamar_texto(s: &AppState, rota: &str) -> (StatusCode, String) {
    let req = Request::builder().uri(rota).body(Body::empty()).unwrap();
    let res = router(s.clone()).oneshot(req).await.unwrap();
    let st = res.status();
    let b = res.into_body().collect().await.unwrap().to_bytes();
    (st, String::from_utf8_lossy(&b).into_owned())
}

#[tokio::test]
async fn receitas_publicar_listar_baixar_e_apagar() {
    let s = ou_pula!(estado().await);
    let (_a, sa) = criar_sessao_teste(&s, &format!("a-{}", Uuid::new_v4()), "a@teste").await;
    let (_b, sb) = criar_sessao_teste(&s, &format!("b-{}", Uuid::new_v4()), "b@teste").await;
    let slug = format!("teste-{}", &Uuid::new_v4().simple().to_string()[..12]);

    // Sem login não publica.
    let corpo = json!({"receita": receita_teste(&slug), "autor": "Zé das Câmeras"});
    let (st, _) = chamar(
        &s,
        "POST",
        "/api/estudio/receitas",
        None,
        Some(corpo.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Publica; a resposta traz o hash e o aviso de estudo.
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/estudio/receitas",
        Some(&sa),
        Some(corpo.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["nova"], true);
    assert_eq!(v["sha256"].as_str().unwrap().len(), 64);
    assert!(v["aviso"].as_str().unwrap().contains("Somente para estudo"));
    let id = v["id"].as_str().unwrap().to_string();

    // Publicar de novo com o mesmo id de receita atualiza (não duplica).
    let (_, v) = chamar(&s, "POST", "/api/estudio/receitas", Some(&sa), Some(corpo)).await;
    assert_eq!(v["nova"], false);
    assert_eq!(v["id"], json!(id));

    // Lista pública (sem login): aparece com métricas e autor, sem e-mail.
    let (st, v) = chamar(
        &s,
        "GET",
        &format!("/api/estudio/receitas?q={slug}"),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let item = v["receitas"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == json!(id))
        .expect("receita pública listada")
        .clone();
    assert_eq!(item["autor"], "Zé das Câmeras");
    assert_eq!(item["classes"], json!(["caixa de banana"]));
    assert!((item["metricas"]["map50"].as_f64().unwrap() - 0.81).abs() < 1e-6);
    assert!(!v.to_string().contains("a@teste"));

    // Outro técnico baixa (conta download) e recebe a receita válida.
    let (st, txt) = chamar_texto(&s, &format!("/api/estudio/receitas/{id}/receita.json")).await;
    assert_eq!(st, StatusCode::OK);
    let baixada = cf_core::receita::ler(&txt).expect("receita baixada é válida");
    assert_eq!(baixada.id, slug);
    let (_, v) = chamar(
        &s,
        "GET",
        &format!("/api/estudio/receitas?q={slug}"),
        Some(&sb),
        None,
    )
    .await;
    assert_eq!(v["receitas"][0]["downloads"], 1);

    // B não apaga a receita de A (404); A apaga.
    let (st, _) = chamar(
        &s,
        "DELETE",
        &format!("/api/estudio/receitas/{id}"),
        Some(&sb),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (_, v) = chamar(&s, "GET", "/api/estudio/receitas/minhas", Some(&sa), None).await;
    assert_eq!(v["receitas"].as_array().unwrap().len(), 1);
    let (_, v) = chamar(&s, "GET", "/api/estudio/receitas/minhas", Some(&sb), None).await;
    assert!(v["receitas"].as_array().unwrap().is_empty());
    let (st, _) = chamar(
        &s,
        "DELETE",
        &format!("/api/estudio/receitas/{id}"),
        Some(&sa),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = chamar_texto(&s, &format!("/api/estudio/receitas/{id}/receita.json")).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn receitas_recusam_fotos_biometria_e_email() {
    let s = ou_pula!(estado().await);
    let (_a, sa) = criar_sessao_teste(&s, &format!("a-{}", Uuid::new_v4()), "a@teste").await;

    // Campo de fotos não existe no formato.
    let mut r = receita_teste("com-fotos");
    r["fotos"] = json!(["/9j/4AAQSkZJRgABAQ..."]);
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/estudio/receitas",
        Some(&sa),
        Some(json!({"receita": r})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert!(v["erro"]
        .as_str()
        .unwrap()
        .contains("campo desconhecido `fotos`"));

    // Foto escondida num texto.
    let mut r = receita_teste("foto-escondida");
    r["estudo"]["notas"] = json!(format!("data:image/jpeg;base64,{}", "A".repeat(300)));
    let (st, _) = chamar(
        &s,
        "POST",
        "/api/estudio/receitas",
        Some(&sa),
        Some(json!({"receita": r})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Reconhecimento facial / biometria.
    let mut r = receita_teste("portaria");
    r["nome"] = json!("Reconhecimento facial na portaria");
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/estudio/receitas",
        Some(&sa),
        Some(json!({"receita": r})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert!(v["erro"].as_str().unwrap().contains("LGPD art. 11"));

    // Autor não pode ser e-mail.
    let (st, _) = chamar(
        &s,
        "POST",
        "/api/estudio/receitas",
        Some(&sa),
        Some(json!({"receita": receita_teste("autor-email"), "autor": "eu@empresa.com"})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Nada foi gravado.
    let (_, v) = chamar(&s, "GET", "/api/estudio/receitas/minhas", Some(&sa), None).await;
    assert!(v["receitas"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn receitas_rls_privada_e_contexto_publico() {
    let s = ou_pula!(estado().await);
    let (a, sa) = criar_sessao_teste(&s, &format!("a-{}", Uuid::new_v4()), "a@teste").await;
    let (b, _) = criar_sessao_teste(&s, &format!("b-{}", Uuid::new_v4()), "b@teste").await;
    let (_, v) = chamar(
        &s,
        "POST",
        "/api/estudio/receitas",
        Some(&sa),
        Some(json!({"receita": receita_teste("privada"), "publica": false})),
    )
    .await;
    let privada = Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();
    let (_, v) = chamar(
        &s,
        "POST",
        "/api/estudio/receitas",
        Some(&sa),
        Some(json!({"receita": receita_teste("publica")})),
    )
    .await;
    let publica = Uuid::parse_str(v["id"].as_str().unwrap()).unwrap();

    // Privada não aparece nem baixa para ninguém além do dono.
    let (st, _) = chamar_texto(&s, &format!("/api/estudio/receitas/{privada}/receita.json")).await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    let mut c = s.db.get().await.unwrap();
    // Visitante: sem WHERE, só enxerga público; não escreve.
    let tx = db::begin(&mut c, Ctx::Publico).await.unwrap();
    let n: i64 = tx
        .query_one(
            "SELECT count(*) FROM receitas_estudo WHERE id = ANY($1)",
            &[&vec![privada, publica]],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(n, 1, "visitante só vê a pública");
    let n = tx
        .execute(
            "UPDATE receitas_estudo SET nome = 'x' WHERE id = $1",
            &[&publica],
        )
        .await
        .unwrap();
    assert_eq!(n, 0, "visitante não altera");
    let ins = tx
        .execute(
            "INSERT INTO receitas_estudo(id, tecnico_id, slug, nome, classes, receita, receita_sha256)
             VALUES ($1, $2, 'invasao', 'x', '{}', '{}', '')",
            &[&Uuid::new_v4(), &a],
        )
        .await;
    assert!(ins.is_err(), "visitante não insere");
    tx.rollback().await.unwrap();

    // Técnico B: vê a pública de A, mas não altera nem apaga, mesmo sem WHERE de dono.
    let tx = db::begin(&mut c, Ctx::Tecnico(b)).await.unwrap();
    let n: i64 = tx
        .query_one(
            "SELECT count(*) FROM receitas_estudo WHERE id = ANY($1)",
            &[&vec![privada, publica]],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(n, 1);
    let n = tx
        .execute(
            "UPDATE receitas_estudo SET nome = 'x' WHERE tecnico_id = $1",
            &[&a],
        )
        .await
        .unwrap();
    assert_eq!(n, 0);
    let n = tx
        .execute("DELETE FROM receitas_estudo WHERE tecnico_id = $1", &[&a])
        .await
        .unwrap();
    assert_eq!(n, 0);
    // E não publica em nome de A.
    let ins = tx
        .execute(
            "INSERT INTO receitas_estudo(id, tecnico_id, slug, nome, classes, receita, receita_sha256)
             VALUES ($1, $2, 'em-nome-de-a', 'x', '{}', '{}', '')",
            &[&Uuid::new_v4(), &a],
        )
        .await;
    assert!(ins.is_err());
    tx.rollback().await.unwrap();

    // O dono vê as duas.
    let tx = db::begin(&mut c, Ctx::Tecnico(a)).await.unwrap();
    let n: i64 = tx
        .query_one(
            "SELECT count(*) FROM receitas_estudo WHERE id = ANY($1)",
            &[&vec![privada, publica]],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(n, 2);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn notebook_e_esquema_publicos() {
    let s = ou_pula!(estado().await);
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/estudio/notebook",
        None,
        Some(json!({"receita": receita_teste("nb-teste")})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["nbformat"], 4);
    assert_eq!(v["metadata"]["camfacil"]["somente_estudo"], true);
    let (st, txt) = chamar_texto(&s, "/esquemas/receita-v1.schema.json").await;
    assert_eq!(st, StatusCode::OK);
    assert!(txt.contains("camfacil-receita"));
}

// ---------- Gerar sua versão (SDD-12) ----------

/// Pedido como o assistente manda: sem base, autor nem hash (o servidor põe).
fn pedido_versao(nome: &str) -> Value {
    json!({"receita": {
        "formato": "cfia-versao",
        "versao_formato": 1,
        "identidade": {"nome": nome, "empresa": "Silva Segurança"},
        "cores": {"destaque": "#5aa9ff"},
        "inteligencias": ["passagem", "presenca"],
        "classes": [0, 16],
    }, "termo": crate::versoes::TERMO})
}

/// Chamada do compilador: token no `Authorization`, sem cookie nem `Origin`.
async fn compilador(
    s: &AppState,
    metodo: &str,
    rota: &str,
    token: Option<&str>,
    corpo: Body,
    json: bool,
) -> (StatusCode, Value) {
    let mut r = Request::builder().method(metodo).uri(rota);
    if let Some(t) = token {
        r = r.header("authorization", format!("Bearer {t}"));
    }
    if json {
        r = r.header("content-type", "application/json");
    }
    let res = router(s.clone())
        .oneshot(r.body(corpo).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn baixar(s: &AppState, rota: &str, sessao: &str) -> (StatusCode, Vec<u8>) {
    let req = Request::builder()
        .uri(rota)
        .header("cookie", format!("{COOKIE}={sessao}"))
        .body(Body::empty())
        .unwrap();
    let res = router(s.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn versao_vai_da_fila_ao_executavel() {
    let s = ou_pula!(estado().await);
    // Limpa a fila de execuções anteriores do teste.
    {
        let mut c = s.db.get().await.unwrap();
        let tx = db::begin(&mut c, Ctx::Sistema).await.unwrap();
        tx.execute(
            "UPDATE versoes SET estado = 'erro' WHERE estado IN ('fila', 'compilando')",
            &[],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }
    let (_a, sa) = criar_sessao_teste(&s, &format!("va-{}", Uuid::new_v4()), "va@teste").await;
    let (_b, sb) = criar_sessao_teste(&s, &format!("vb-{}", Uuid::new_v4()), "vb@teste").await;
    let tk = Some(TOKEN_COMPILADOR);

    // Sem aceitar o termo, não entra na fila.
    let mut sem_termo = pedido_versao("Vigia Silva");
    sem_termo["termo"] = json!("");
    let (st, _) = chamar(&s, "POST", "/api/versoes", Some(&sa), Some(sem_termo)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // Catálogo e conferência são públicos e não gravam nada.
    let (st, v) = chamar(&s, "GET", "/api/versoes/catalogo", None, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["classes"].as_array().unwrap().len(), 80);
    assert_eq!(v["ligado"], true);
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/versoes/conferir",
        None,
        Some(pedido_versao("Vigia Hikvision")),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert!(v["erro"].as_str().unwrap().contains("fabricante"));
    let (st, _) = chamar(
        &s,
        "POST",
        "/api/versoes/conferir",
        None,
        Some(pedido_versao("Vigia Silva")),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Marca de fabricante no nome é recusada antes de entrar na fila.
    let (st, v) = chamar(
        &s,
        "POST",
        "/api/versoes",
        Some(&sa),
        Some(pedido_versao("Vigia Intelbras")),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "{v}");

    let (st, v) = chamar(
        &s,
        "POST",
        "/api/versoes",
        Some(&sa),
        Some(pedido_versao("Vigia Silva")),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let id = v["id"].as_str().unwrap().to_string();
    let hash = v["hash"].as_str().unwrap().to_string();

    // O pedido deixa o registro de geração; o técnico não lê nem apaga.
    {
        let vid = Uuid::parse_str(&id).unwrap();
        let mut c = s.db.get().await.unwrap();
        let tx = db::begin(&mut c, Ctx::Sistema).await.unwrap();
        let r = tx
            .query_one(
                "SELECT conta_email, receita_sha256, termo FROM registro_versoes WHERE versao_id = $1",
                &[&vid],
            )
            .await
            .unwrap();
        assert_eq!(r.get::<_, String>(0), "va@teste");
        assert_eq!(r.get::<_, String>(1), hash);
        assert_eq!(r.get::<_, String>(2), crate::versoes::TERMO);
        tx.commit().await.unwrap();
        let tx = db::begin(&mut c, Ctx::Tecnico(_a)).await.unwrap();
        let n: i64 = tx
            .query_one(
                "SELECT count(*) FROM registro_versoes WHERE versao_id = $1",
                &[&vid],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(n, 0, "o técnico não lê o registro");
        let apagou = tx
            .execute("DELETE FROM registro_versoes WHERE versao_id = $1", &[&vid])
            .await
            .unwrap();
        assert_eq!(apagou, 0, "o técnico não apaga o registro");
        tx.commit().await.unwrap();
    }

    // Uma por vez.
    let (st, _) = chamar(
        &s,
        "POST",
        "/api/versoes",
        Some(&sa),
        Some(pedido_versao("Outra")),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);

    // B não vê a versão de A.
    let (st, _) = chamar(&s, "GET", &format!("/api/versoes/{id}"), Some(&sb), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, v) = chamar(&s, "GET", &format!("/api/versoes/{id}"), Some(&sa), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["estado"], "fila");
    assert_eq!(v["posicao"], 1);

    // Compilador sem token, ou com o errado, não pega nada.
    let (st, _) = compilador(&s, "POST", "/api/builder/pegar", None, Body::empty(), false).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = compilador(
        &s,
        "POST",
        "/api/builder/pegar",
        Some("x".repeat(40).as_str()),
        Body::empty(),
        false,
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    let (st, v) = compilador(&s, "POST", "/api/builder/pegar", tk, Body::empty(), false).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["id"], id.as_str());
    assert_eq!(v["commit"], "e46a8b7");
    // A receita entregue ao compilador é válida, assinada e com o autor da conta.
    let r = cf_versao::ReceitaVersao::ler(&v["receita"].to_string()).unwrap();
    assert_eq!(r.hash.as_deref(), Some(hash.as_str()));
    assert_eq!(r.base.commit, "e46a8b7");
    assert_eq!(r.autor.nome, "va");
    let (st, _) = compilador(&s, "POST", "/api/builder/pegar", tk, Body::empty(), false).await;
    assert_eq!(st, StatusCode::NO_CONTENT);

    let prog =
        |etapa: &str, pct: i64| Body::from(json!({"etapa": etapa, "percentual": pct}).to_string());
    let rota = format!("/api/builder/{id}/progresso");
    let (st, _) = compilador(&s, "POST", &rota, tk, prog("invasao", 10), true).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = compilador(&s, "POST", &rota, tk, prog("compilando", 40), true).await;
    assert_eq!(st, StatusCode::OK);
    let (_, v) = chamar(&s, "GET", &format!("/api/versoes/{id}"), Some(&sa), None).await;
    assert_eq!(
        (v["estado"].as_str(), v["percentual"].as_i64()),
        (Some("compilando"), Some(40))
    );

    // Executável: só um PE do Windows.
    let rota = format!("/api/builder/{id}/executavel");
    let (st, _) = compilador(&s, "PUT", &rota, tk, Body::from("não é exe"), false).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let exe = b"MZ\x90\x00 executavel de teste".to_vec();
    let (st, v) = compilador(&s, "PUT", &rota, tk, Body::from(exe.clone()), false).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let (_, v) = chamar(&s, "GET", &format!("/api/versoes/{id}"), Some(&sa), None).await;
    assert_eq!(
        (v["estado"].as_str(), v["percentual"].as_i64()),
        (Some("pronta"), Some(100))
    );

    let rota = format!("/api/versoes/{id}/executavel");
    let (st, bytes) = baixar(&s, &rota, &sa).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(bytes, exe);
    let (st, _) = baixar(&s, &rota, &sb).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, bytes) = baixar(&s, &format!("/api/versoes/{id}/receita.json"), &sa).await;
    assert_eq!(st, StatusCode::OK);
    let devolvida: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(devolvida["hash"], hash.as_str());

    // Até 3 por dia: mais duas (que falham no compilador) e a quarta é barrada.
    for n in 0..2 {
        let (st, v) = chamar(
            &s,
            "POST",
            "/api/versoes",
            Some(&sa),
            Some(pedido_versao("Vigia Silva")),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "pedido {n}: {v}");
        let (_, v) = compilador(&s, "POST", "/api/builder/pegar", tk, Body::empty(), false).await;
        let outro = v["id"].as_str().unwrap().to_string();
        let corpo = Body::from(json!({"mensagem": "falhou de propósito"}).to_string());
        let (st, _) = compilador(
            &s,
            "POST",
            &format!("/api/builder/{outro}/erro"),
            tk,
            corpo,
            true,
        )
        .await;
        assert_eq!(st, StatusCode::OK);
    }
    let (st, _) = chamar(
        &s,
        "POST",
        "/api/versoes",
        Some(&sa),
        Some(pedido_versao("Vigia Silva")),
    )
    .await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);
    let (st, v) = chamar(&s, "GET", "/api/versoes", Some(&sa), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["versoes"].as_array().unwrap().len(), 3);
    assert_eq!(v["versoes"][0]["estado"], "erro");
}

// ---------- Retenção do Gerar sua versão (SDD-12) ----------

/// Versão já pronta, gravada direto (sem passar pela fila, para não cruzar
/// com o teste da fila rodando em paralelo).
async fn versao_pronta(s: &AppState, tecnico: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let mut c = s.db.get().await.unwrap();
    let tx = db::begin(&mut c, Ctx::Sistema).await.unwrap();
    tx.execute(
        "INSERT INTO versoes(id, tecnico_id, nome, receita, receita_sha256, base_commit,
             estado, etapa, percentual, executavel_bytes, iniciado_em, terminado_em)
         VALUES ($1, $2, 'Teste', '{}', 'h', 'e46a8b7', 'pronta', 'pronta', 100, 4, now(), now())",
        &[&id, &tecnico],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    tokio::fs::create_dir_all(&s.versoes.dir).await.unwrap();
    tokio::fs::write(s.versoes.dir.join(format!("{id}.exe")), b"MZ..")
        .await
        .unwrap();
    id
}

async fn sistema(s: &AppState, sql: &str, tecnico: Uuid) {
    let mut c = s.db.get().await.unwrap();
    let tx = db::begin(&mut c, Ctx::Sistema).await.unwrap();
    tx.execute(sql, &[&tecnico]).await.unwrap();
    tx.commit().await.unwrap();
}

async fn contar(s: &AppState, sql: &str, id: Uuid) -> i64 {
    let mut c = s.db.get().await.unwrap();
    let tx = db::begin(&mut c, Ctx::Sistema).await.unwrap();
    let n: i64 = tx.query_one(sql, &[&id]).await.unwrap().get(0);
    tx.commit().await.unwrap();
    n
}

#[tokio::test]
async fn retencao_avisa_no_23o_dia_e_apaga_no_30o() {
    let s = ou_pula!(estado().await);
    let (t, sessao) = criar_sessao_teste(&s, &format!("ret-{}", Uuid::new_v4()), "ret@teste").await;
    let v = versao_pronta(&s, t).await;
    let exe = s.versoes.dir.join(format!("{v}.exe"));
    let existe = "SELECT count(*) FROM versoes WHERE id = $1";

    // Ativo: nada acontece.
    crate::retencao::rodar(&s).await.unwrap();
    assert_eq!(contar(&s, existe, v).await, 1);

    // 25 dias sem entrar: avisaria (sem SMTP, conta como não enviado) e não apaga.
    sistema(
        &s,
        "UPDATE tecnicos SET ultimo_acesso = now() - interval '25 days' WHERE id = $1",
        t,
    )
    .await;
    let r = crate::retencao::rodar(&s).await.unwrap();
    assert!(r.sem_envio >= 1, "{r:?}");
    assert_eq!(contar(&s, existe, v).await, 1);
    assert!(exe.exists());

    // Entrar de novo (qualquer chamada logada) zera a contagem.
    let (st, _) = chamar(&s, "GET", "/api/eu", Some(&sessao), None).await;
    assert_eq!(st, StatusCode::OK);
    let recente = "SELECT count(*) FROM tecnicos WHERE id = $1 AND ultimo_acesso > now() - interval '1 minute'";
    assert_eq!(
        contar(&s, recente, t).await,
        1,
        "o acesso atualiza ultimo_acesso"
    );

    // 31 dias sem entrar: a versão e o executável somem; o registro fica.
    sistema(
        &s,
        "INSERT INTO registro_versoes(versao_id, tecnico_id, conta_email, receita_sha256,
             base_versao, base_commit, termo) VALUES (gen_random_uuid(), $1, 'ret@teste', 'h',
             '0.9.9', 'e46a8b7', 'x')",
        t,
    )
    .await;
    sistema(
        &s,
        "UPDATE tecnicos SET ultimo_acesso = now() - interval '31 days' WHERE id = $1",
        t,
    )
    .await;
    let r = crate::retencao::rodar(&s).await.unwrap();
    assert!(r.versoes_apagadas >= 1, "{r:?}");
    assert_eq!(contar(&s, existe, v).await, 0);
    assert!(!exe.exists());
    let registro = "SELECT count(*) FROM registro_versoes WHERE tecnico_id = $1";
    assert_eq!(
        contar(&s, registro, t).await,
        1,
        "o registro sobrevive à limpeza"
    );

    // Registro com mais de 12 meses expira. O papel da aplicação não altera
    // registro nenhum (só insere e, como sistema, apaga): envelhece como dono do banco.
    s.db.get()
        .await
        .unwrap()
        .execute(
            "UPDATE registro_versoes SET gerado_em = now() - interval '13 months' WHERE tecnico_id = $1",
            &[&t],
        )
        .await
        .unwrap();
    let r = crate::retencao::rodar(&s).await.unwrap();
    assert!(r.registros_expirados >= 1, "{r:?}");
    assert_eq!(contar(&s, registro, t).await, 0);
}

#[tokio::test]
async fn so_o_ultimo_executavel_fica_no_disco() {
    let s = ou_pula!(estado().await);
    let (t, _) = criar_sessao_teste(&s, &format!("ult-{}", Uuid::new_v4()), "ult@teste").await;
    let antiga = versao_pronta(&s, t).await;
    // Uma nova chega pelo compilador: precisa estar compilando.
    let nova = Uuid::new_v4();
    {
        let mut c = s.db.get().await.unwrap();
        let tx = db::begin(&mut c, Ctx::Sistema).await.unwrap();
        tx.execute(
            "INSERT INTO versoes(id, tecnico_id, nome, receita, receita_sha256, base_commit, estado, iniciado_em)
             VALUES ($1, $2, 'Nova', '{}', 'h2', 'e46a8b7', 'compilando', now())",
            &[&nova, &t],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }
    let (st, v) = compilador(
        &s,
        "PUT",
        &format!("/api/builder/{nova}/executavel"),
        Some(TOKEN_COMPILADOR),
        Body::from(b"MZ nova".to_vec()),
        false,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert!(s.versoes.dir.join(format!("{nova}.exe")).exists());
    assert!(!s.versoes.dir.join(format!("{antiga}.exe")).exists());
    let sem_exe = "SELECT count(*) FROM versoes WHERE id = $1 AND executavel_bytes IS NULL";
    assert_eq!(contar(&s, sem_exe, antiga).await, 1);
}
