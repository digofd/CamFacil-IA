//! "Gerar sua versão" (SDD-12): aplica a receita da versão na compilação.
//!
//! Sem `CFIA_RECEITA`, gera exatamente o programa oficial. Com
//! `CFIA_RECEITA=/caminho/receita.json`, valida a receita (de novo; o site já
//! validou) e gera em `OUT_DIR`:
//! - `tokens.css` e `icon.svg` com a cor de destaque escolhida;
//! - `logo.bin` (vazio sem logo) e `receita.json` (a receita assinada);
//! - `versao.rs`, com nome, autor, hash, inteligências e classes permitidas.
//!
//! Só o crate `camfacil` recompila quando a receita muda: o cache do resto
//! do workspace continua valendo.

use cf_versao::coco;
use cf_versao::receita::{self, ReceitaVersao};
use std::path::{Path, PathBuf};
use std::{env, fs};

const VERDE: &str = "#06cb3f";

fn main() {
    let manifesto = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let web = manifesto.join("../../web");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-env-changed=CFIA_RECEITA");
    println!("cargo:rerun-if-changed=../../web/tokens.css");
    println!("cargo:rerun-if-changed=../../web/icon.svg");

    let tokens = ler(&web.join("tokens.css"));
    let icone = ler(&web.join("icon.svg"));

    let receita = match env::var("CFIA_RECEITA") {
        Ok(p) if !p.trim().is_empty() => {
            println!("cargo:rerun-if-changed={p}");
            let json = ler(Path::new(&p));
            let r = ReceitaVersao::ler(&json)
                .unwrap_or_else(|e| panic!("CFIA_RECEITA ({p}): {e}"))
                .assinada();
            Some(r)
        }
        _ => None,
    };

    let (tokens, icone, logo, mime, json) = match &receita {
        None => (tokens, icone, Vec::new(), "", String::new()),
        Some(r) => {
            let cor = r.cores.destaque.to_ascii_lowercase();
            let tokens = receita::tokens_css(&tokens, &cor)
                .unwrap_or_else(|e| panic!("web/tokens.css: {e}"));
            assert!(
                icone.contains(VERDE),
                "web/icon.svg mudou: o verde {VERDE} não está mais nele"
            );
            let icone = icone.replace(VERDE, &cor);
            let (mime, logo) = r
                .logo_bytes()
                .expect("logo já validado")
                .unwrap_or(("", Vec::new()));
            let json = serde_json::to_string_pretty(r).unwrap();
            (tokens, icone, logo, mime, json)
        }
    };
    escrever(&out.join("tokens.css"), tokens.as_bytes());
    escrever(&out.join("icon.svg"), icone.as_bytes());
    escrever(&out.join("logo.bin"), &logo);
    escrever(&out.join("receita.json"), json.as_bytes());
    escrever(
        &out.join("versao.rs"),
        versao_rs(receita.as_ref(), mime).as_bytes(),
    );
}

fn versao_rs(r: Option<&ReceitaVersao>, logo_mime: &str) -> String {
    let todas: Vec<String> = receita::INTELIGENCIAS
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (nome, empresa, autor, gerado_em, base_versao, base_commit, hash, intel, classes) = match r
    {
        None => (
            "",
            "",
            "",
            "",
            "",
            "",
            String::new(),
            todas,
            coco::PADRAO.to_vec(),
        ),
        Some(r) => (
            r.identidade.nome.as_str(),
            r.identidade.empresa.as_str(),
            r.autor.nome.as_str(),
            r.autor.gerado_em.as_str(),
            r.base.versao.as_str(),
            r.base.commit.as_str(),
            r.hash.clone().unwrap_or_default(),
            r.inteligencias.clone(),
            r.classes.clone(),
        ),
    };
    format!(
        "// Gerado pelo build.rs a partir de CFIA_RECEITA (SDD-12). Não edite.\n\
         pub const PERSONALIZADA: bool = {personalizada};\n\
         pub const NOME: &str = {nome:?};\n\
         pub const EMPRESA: &str = {empresa:?};\n\
         pub const AUTOR: &str = {autor:?};\n\
         pub const GERADO_EM: &str = {gerado_em:?};\n\
         pub const BASE_VERSAO: &str = {base_versao:?};\n\
         pub const BASE_COMMIT: &str = {base_commit:?};\n\
         pub const HASH: &str = {hash:?};\n\
         pub const INTELIGENCIAS: &[&str] = &{intel:?};\n\
         pub const CLASSES: &[usize] = &{classes:?};\n\
         pub const LOGO_MIME: &str = {logo_mime:?};\n",
        personalizada = r.is_some(),
    )
}

fn ler(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// Só regrava se mudou, para não forçar recompilação à toa.
fn escrever(p: &Path, conteudo: &[u8]) {
    if fs::read(p).ok().as_deref() != Some(conteudo) {
        fs::write(p, conteudo).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    }
}
