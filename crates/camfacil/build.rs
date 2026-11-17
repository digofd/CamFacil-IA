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
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        recursos_windows(&manifesto, &out, receita.as_ref(), mime, &logo);
    }
}

/// Ficha do executável no Windows (Propriedades → Detalhes), manifesto e
/// ícone. Programa sem nada disso parece "anônimo" para os antivírus que
/// julgam por modelo (SDD-12). Na versão gerada, a ficha diz de quem é e que
/// é uma versão modificada do CamFácil IA; o ícone é o logo da receita.
fn recursos_windows(
    manifesto: &Path,
    out: &Path,
    r: Option<&ReceitaVersao>,
    logo_mime: &str,
    logo: &[u8],
) {
    let oficial = manifesto.join("../../scripts/icones/camfacil.ico");
    println!("cargo:rerun-if-changed=../../scripts/icones/camfacil.ico");
    let ico = if logo_mime == "image/png" {
        ico_de_png(logo)
    } else {
        fs::read(&oficial).unwrap_or_else(|e| panic!("{}: {e}", oficial.display()))
    };
    escrever(&out.join("camfacil.ico"), &ico);

    let versao = env::var("CARGO_PKG_VERSION").unwrap();
    let repo = env::var("CARGO_PKG_REPOSITORY").unwrap();
    let mut n: Vec<u16> = versao.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    n.resize(4, 0);
    let numero = format!("{},{},{},{}", n[0], n[1], n[2], n[3]);
    let pontos = format!("{}.{}.{}.{}", n[0], n[1], n[2], n[3]);

    let (empresa, descricao, produto, copyright, comentario) = match r {
        None => (
            "Projeto CamFácil IA (software livre)".to_string(),
            "CamFácil IA".to_string(),
            "CamFácil IA".to_string(),
            format!(
                "AGPL-3.0 · {}",
                env::var("CARGO_PKG_AUTHORS").unwrap_or_default()
            ),
            format!("IA de vídeo para CFTV, para aprender e validar. Código: {repo}"),
        ),
        Some(r) => {
            let quem = if r.identidade.empresa.is_empty() {
                r.autor.nome.clone()
            } else {
                r.identidade.empresa.clone()
            };
            (
                quem,
                format!("{} (versão modificada do CamFácil IA)", r.identidade.nome),
                r.identidade.nome.clone(),
                format!("AGPL-3.0 · CamFácil IA e {}", r.autor.nome),
                format!(
                    "Versão modificada do CamFácil IA por {}, em {}. Base {} ({}). Receita {}. Código: {}/tree/v{}",
                    r.autor.nome,
                    r.autor.gerado_em.get(..10).unwrap_or(&r.autor.gerado_em),
                    r.base.versao,
                    r.base.commit.get(..7).unwrap_or(&r.base.commit),
                    r.hash.as_deref().unwrap_or("").get(..12).unwrap_or(""),
                    repo.trim_end_matches('/'),
                    r.base.versao,
                ),
            )
        }
    };

    let manifesto_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="CamFacilIA.camfacil" version="{pontos}" processorArchitecture="amd64"/>
  <description>CamFacil IA</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application><supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"/></application>
  </compatibility>
</assembly>
"#
    );
    escrever(&out.join("camfacil.exe.manifest"), manifesto_xml.as_bytes());

    let caminho = |nome: &str| out.join(nome).display().to_string().replace('\\', "/");
    let campos = [
        ("CompanyName", empresa),
        ("FileDescription", descricao),
        ("FileVersion", versao.clone()),
        ("InternalName", "camfacil".to_string()),
        ("LegalCopyright", copyright),
        ("OriginalFilename", "camfacil.exe".to_string()),
        ("ProductName", produto),
        ("ProductVersion", versao.clone()),
        ("Comments", comentario),
    ];
    let mut linhas = String::new();
    for (chave, valor) in &campos {
        linhas.push_str(&format!(
            "      VALUE \"{chave}\", L\"{}\"\n",
            rc_texto(valor)
        ));
    }
    let rc = format!(
        "// Gerado pelo build.rs (SDD-12). Não edite.\n\
         1 ICON \"{ico}\"\n\
         1 24 \"{man}\"\n\
         1 VERSIONINFO\n\
         FILEVERSION {numero}\n\
         PRODUCTVERSION {numero}\n\
         FILEFLAGSMASK 0x3fL\n\
         FILEFLAGS 0x0L\n\
         FILEOS 0x40004L\n\
         FILETYPE 0x1L\n\
         FILESUBTYPE 0x0L\n\
         BEGIN\n\
           BLOCK \"StringFileInfo\"\n\
           BEGIN\n\
             BLOCK \"041604B0\"\n\
             BEGIN\n\
         {linhas}\
             END\n\
           END\n\
           BLOCK \"VarFileInfo\"\n\
           BEGIN\n\
             VALUE \"Translation\", 0x416, 1200\n\
           END\n\
         END\n",
        ico = caminho("camfacil.ico"),
        man = caminho("camfacil.exe.manifest"),
    );
    let rc_arquivo = out.join("camfacil.rc");
    escrever(&rc_arquivo, rc.as_bytes());
    embed_resource::compile(&rc_arquivo, embed_resource::NONE)
        .manifest_required()
        .unwrap_or_else(|e| panic!("recursos do Windows (rc/windres): {e}"));
}

/// Texto para string larga do .rc: todo caractere como `\xHHHH`. Funciona
/// igual no rc.exe (MSVC) e no windres (mingw), sem depender de página de
/// código, e nenhum dígito seguinte é lido como parte do escape.
fn rc_texto(s: &str) -> String {
    s.encode_utf16().map(|u| format!("\\x{u:04x}")).collect()
}

/// Ícone do Windows com uma imagem PNG dentro (aceito desde o Vista). O logo
/// da receita já sai do assistente como PNG de 256×256.
fn ico_de_png(png: &[u8]) -> Vec<u8> {
    let dim = |i: usize| {
        let v = u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
        if v >= 256 {
            0
        } else {
            v as u8
        }
    };
    let (w, h) = if png.len() > 24 {
        (dim(16), dim(20))
    } else {
        (0, 0)
    };
    let mut ico = vec![0, 0, 1, 0, 1, 0, w, h, 0, 0, 1, 0, 32, 0];
    ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(png);
    ico
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
