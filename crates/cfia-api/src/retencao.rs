//! "Gerar sua versão" (SDD-12): retenção do espaço gratuito.
//!
//! De hora em hora:
//! - **23º dia sem entrar:** manda um e-mail ("sua versão some em 7 dias,
//!   baixe a receita"), uma vez por período de inatividade;
//! - **30 dias sem entrar:** apaga as versões do técnico (receitas, logos e
//!   executáveis), menos o que está compilando;
//! - **12 meses:** apaga o registro de geração.
//!
//! O e-mail sai por SMTP (`CFIA_SMTP_URL`, ex.: `smtps://usuario:senha@smtp.provedor.com:465`,
//! e `CFIA_EMAIL_DE`, ex.: `CamFácil IA <nao-responda@cfia.ysafe.com.br>`). Sem
//! isso, a limpeza roda igual e o aviso fica registrado como não enviado.

use crate::db::{self, Ctx};
use crate::AppState;
use lettre::message::{header::ContentType, Mailbox};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::time::Duration;
use uuid::Uuid;

/// Dias sem entrar até apagar, e até avisar.
pub const DIAS_APAGAR: i32 = 30;
pub const DIAS_AVISAR: i32 = 23;

/// Como o e-mail sai.
#[derive(Clone)]
pub struct Correio {
    smtp: AsyncSmtpTransport<Tokio1Executor>,
    de: Mailbox,
}

impl Correio {
    pub fn do_ambiente() -> Option<Self> {
        let env = |n: &str| std::env::var(n).ok().filter(|v| !v.trim().is_empty());
        let (url, de) = match (env("CFIA_SMTP_URL"), env("CFIA_EMAIL_DE")) {
            (Some(u), Some(d)) => (u, d),
            _ => {
                eprintln!("aviso: CFIA_SMTP_URL/CFIA_EMAIL_DE ausentes; o aviso do 23º dia não será enviado");
                return None;
            }
        };
        let smtp = AsyncSmtpTransport::<Tokio1Executor>::from_url(&url)
            .map_err(|e| eprintln!("aviso: CFIA_SMTP_URL inválida ({e}); e-mail desligado"))
            .ok()?
            .timeout(Some(Duration::from_secs(20)))
            .build();
        let de = de
            .parse()
            .map_err(|e| eprintln!("aviso: CFIA_EMAIL_DE inválido ({e}); e-mail desligado"))
            .ok()?;
        Some(Self { smtp, de })
    }

    async fn enviar(&self, para: &str, assunto: &str, texto: String) -> Result<(), String> {
        let para: Mailbox = para.parse().map_err(|e| format!("e-mail inválido: {e}"))?;
        let msg = Message::builder()
            .from(self.de.clone())
            .to(para)
            .subject(assunto)
            .header(ContentType::TEXT_PLAIN)
            .body(texto)
            .map_err(|e| e.to_string())?;
        self.smtp.send(msg).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Assunto do aviso, com os dias que faltam de verdade (o servidor pode ter
/// ficado fora do ar e avisar no 25º dia, por exemplo).
pub fn assunto_aviso(dias_restantes: i32) -> String {
    match dias_restantes {
        ..=1 => "Sua versão do CamFácil IA será apagada amanhã".into(),
        n => format!("Sua versão do CamFácil IA será apagada em {n} dias"),
    }
}

/// Texto do aviso. Só o nome (dado da conta), os dias e a data entram.
pub fn texto_aviso(nome: &str, dias_sem_entrar: i32, data_apagar: &str, base_url: &str) -> String {
    let saudacao = if nome.trim().is_empty() {
        "Olá,".to_string()
    } else {
        format!("Olá, {},", nome.trim())
    };
    format!(
        "{saudacao}\n\n\
         Faz {dias_sem_entrar} dias que você não entra no CamFácil IA. O espaço do \"Gerar sua versão\" \
         é gratuito e, sem acesso por {DIAS_APAGAR} dias, as suas versões são apagadas: receitas, \
         logos e executáveis.\n\n\
         Data prevista: {data_apagar}.\n\n\
         Para não perder nada, entre em {base_url}/versao e baixe a receita de cada versão. \
         Guarde no seu Google Drive ou onde preferir: com a receita você gera a mesma versão de \
         novo quando quiser. Entrar no site já adia a limpeza por mais {DIAS_APAGAR} dias.\n\n\
         O registro de quem gerou cada versão (nome da conta e hash da receita) continua guardado \
         por 12 meses, como diz a política de privacidade: {base_url}/privacidade\n\n\
         CamFácil IA · software livre (AGPL-3.0)\n\
         Este é um aviso automático; não precisa responder.\n"
    )
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Resumo {
    pub avisados: usize,
    pub sem_envio: usize,
    pub versoes_apagadas: usize,
    pub registros_expirados: u64,
}

/// Uma rodada da limpeza (o laço de hora em hora chama; os testes também).
pub async fn rodar(s: &AppState) -> db::Result<Resumo> {
    let mut r = Resumo::default();
    let mut c = s.db.get().await?;

    // 1. Aviso do 23º dia, uma vez por período sem entrar.
    let tx = db::begin(&mut c, Ctx::Sistema).await?;
    let avisar = tx
        .query(
            &format!(
                "SELECT t.id, t.email, t.nome,
                        to_char((t.ultimo_acesso + interval '{DIAS_APAGAR} days') AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY'),
                        floor(extract(epoch FROM now() - t.ultimo_acesso) / 86400)::int
                 FROM tecnicos t
                 WHERE t.ultimo_acesso < now() - interval '{DIAS_AVISAR} days'
                   AND t.ultimo_acesso >= now() - interval '{DIAS_APAGAR} days'
                   AND (t.aviso_retencao_em IS NULL OR t.aviso_retencao_em < t.ultimo_acesso)
                   AND EXISTS (SELECT 1 FROM versoes v WHERE v.tecnico_id = t.id)"
            ),
            &[],
        )
        .await?;
    tx.commit().await?;
    for linha in avisar {
        let (id, email, nome, data, dias): (Uuid, String, String, String, i32) = (
            linha.get(0),
            linha.get(1),
            linha.get(2),
            linha.get(3),
            linha.get(4),
        );
        let enviado = match &s.correio {
            Some(correio) => {
                let texto = texto_aviso(&nome, dias, &data, &s.base_url);
                let assunto = assunto_aviso(DIAS_APAGAR - dias);
                match correio.enviar(&email, &assunto, texto).await {
                    Ok(()) => true,
                    Err(e) => {
                        eprintln!("retenção: aviso para {id} não saiu: {e}");
                        false
                    }
                }
            }
            None => false,
        };
        if enviado {
            r.avisados += 1;
            let tx = db::begin(&mut c, Ctx::Sistema).await?;
            tx.execute(
                "UPDATE tecnicos SET aviso_retencao_em = now() WHERE id = $1",
                &[&id],
            )
            .await?;
            db::auditar(&tx, Some(id), "versao_aviso_retencao", &data, "").await?;
            tx.commit().await?;
        } else {
            r.sem_envio += 1;
        }
    }

    // 2. 30 dias sem entrar: apaga as versões (menos a que está compilando).
    let tx = db::begin(&mut c, Ctx::Sistema).await?;
    let apagar: Vec<Uuid> = tx
        .query(
            &format!(
                "DELETE FROM versoes v USING tecnicos t
                 WHERE t.id = v.tecnico_id AND v.estado <> 'compilando'
                   AND t.ultimo_acesso < now() - interval '{DIAS_APAGAR} days'
                 RETURNING v.id"
            ),
            &[],
        )
        .await?
        .iter()
        .map(|l| l.get(0))
        .collect();
    tx.commit().await?;
    for id in &apagar {
        let _ = tokio::fs::remove_file(s.versoes.dir.join(format!("{id}.exe"))).await;
    }
    r.versoes_apagadas = apagar.len();

    // 3. Registro de geração: 12 meses.
    let tx = db::begin(&mut c, Ctx::Sistema).await?;
    r.registros_expirados = tx
        .execute(
            "DELETE FROM registro_versoes WHERE gerado_em < now() - interval '12 months'",
            &[],
        )
        .await?;
    tx.commit().await?;
    Ok(r)
}

/// Laço de fundo: primeira rodada 1 minuto depois de subir, depois de hora em hora.
pub fn iniciar(s: AppState) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let mut relogio = tokio::time::interval(Duration::from_secs(3600));
        loop {
            relogio.tick().await;
            match rodar(&s).await {
                Ok(r) if r != Resumo::default() => println!("retenção: {r:?}"),
                Ok(_) => {}
                Err(e) => eprintln!("retenção: {e}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texto_do_aviso() {
        let t = texto_aviso("Fulano", 25, "20/12/2026", "https://cfia.ysafe.com.br");
        assert!(t.starts_with("Olá, Fulano,"));
        assert!(t.contains("Data prevista: 20/12/2026."));
        assert!(t.contains("https://cfia.ysafe.com.br/versao"));
        assert!(t.contains("12 meses"));
        assert!(t.contains("Faz 25 dias"));
        assert!(assunto_aviso(7).ends_with("em 7 dias"));
        assert!(assunto_aviso(5).ends_with("em 5 dias"));
        assert!(assunto_aviso(1).ends_with("amanhã"));
        assert!(texto_aviso("", 23, "x", "y").starts_with("Olá,\n"));
    }
}
