//! Versão gerada pelo "Gerar sua versão" (SDD-12). As constantes vêm do
//! `build.rs`: sem receita, valem as do programa oficial (`PERSONALIZADA =
//! false`, todas as inteligências e as seis classes de sempre).

include!(concat!(env!("OUT_DIR"), "/versao.rs"));

/// A receita assinada, embutida no executável (vazia no programa oficial).
/// Com a versão base, é a fonte correspondente da versão modificada (AGPL-3.0).
pub const RECEITA_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/receita.json"));

/// Logo da receita (vazio sem logo).
pub const LOGO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/logo.bin"));

/// Código-fonte da versão base: a tag oficial de onde esta versão saiu.
pub fn codigo_url() -> String {
    let versao = if PERSONALIZADA {
        BASE_VERSAO
    } else {
        env!("CARGO_PKG_VERSION")
    };
    format!(
        "{}/tree/v{versao}",
        env!("CARGO_PKG_REPOSITORY").trim_end_matches('/')
    )
}

/// A inteligência por regra pode ser usada nesta versão?
pub fn intel_permitida(id: &str) -> bool {
    INTELIGENCIAS.contains(&id)
}

/// O que a tela mostra no "Sobre" (e o que o MCP e o suporte podem ler).
pub fn info() -> serde_json::Value {
    serde_json::json!({
        "personalizada": PERSONALIZADA,
        "nome": NOME,
        "empresa": EMPRESA,
        "autor": AUTOR,
        "gerado_em": GERADO_EM,
        "base_versao": if PERSONALIZADA { BASE_VERSAO } else { env!("CARGO_PKG_VERSION") },
        "base_commit": BASE_COMMIT,
        "hash": HASH,
        "logo": !LOGO.is_empty(),
        "codigo": codigo_url(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sem `CFIA_RECEITA` (como no CI e na release), o binário é o oficial.
    #[test]
    fn sem_receita_e_igual_ao_oficial() {
        if std::hint::black_box(PERSONALIZADA) {
            return;
        }
        assert!(RECEITA_JSON.is_empty() && LOGO.is_empty() && HASH.is_empty());
        let oficiais: Vec<_> = cf_core::inteligencias::INTELIGENCIAS
            .iter()
            .map(|i| i.id)
            .collect();
        assert_eq!(INTELIGENCIAS, oficiais.as_slice());
        assert_eq!(CLASSES, cf_core::deteccao::CLASSES_PADRAO);
        let tokens = crate::assets::find("/tokens.css").unwrap().body;
        assert_eq!(tokens, include_bytes!("../../../web/tokens.css"));
        let icone = crate::assets::find("/icon.svg").unwrap().body;
        assert_eq!(icone, include_bytes!("../../../web/icon.svg"));
        assert!(crate::assets::find("/marca").is_none());
    }

    #[test]
    fn codigo_aponta_para_a_tag() {
        assert!(codigo_url().starts_with("https://github.com/emersoncassis/CamFacil-IA/tree/v"));
    }
}
