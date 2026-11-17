//! Receita da versão (SDD-12): o que o técnico escolheu no "Gerar sua versão".
//! Um JSON só, com o logo dentro (base64), validado aqui antes de compilar e
//! embutido no executável. Junto com a versão base, é a fonte correspondente
//! da versão modificada (AGPL-3.0).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FORMATO: &str = "cfia-versao";
pub const VERSAO_FORMATO: u32 = 1;

/// Inteligências por regra que a receita pode escolher (as mesmas de
/// `cf_core::inteligencias::INTELIGENCIAS`; um teste no `cf-core` confere).
pub const INTELIGENCIAS: &[&str] = &["passagem", "presenca", "densidade", "permanencia", "calor"];

/// Logo de até 512 KB depois de decodificado.
pub const LOGO_MAX: usize = 512 * 1024;

/// Marcas de fabricante que não podem aparecer no nome (Lei 9.279/1996).
/// Nomes longos valem dentro de outra palavra ("HikvisionPro"); os curtos só
/// como palavra inteira, para não barrar "jflores" nem "axismundi".
const MARCAS_LONGAS: &[&str] = &[
    "intelbras",
    "hikvision",
    "dahua",
    "ezviz",
    "tecvoz",
    "hanwha",
    "uniview",
    "tplink",
    "motorola",
    "samsung",
    "honeywell",
];
const MARCAS_CURTAS: &[&str] = &["jfl", "axis", "imou", "bosch", "vigi", "giga", "hilook"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Base {
    /// Versão oficial de onde a versão foi gerada (ex.: "0.9.9").
    pub versao: String,
    /// Commit da versão oficial.
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identidade {
    /// Nome da versão, que aparece no programa (ex.: "Vigia Silva").
    pub nome: String,
    #[serde(default)]
    pub empresa: String,
    /// `data:image/png;base64,...` ou `data:image/jpeg;base64,...`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cores {
    /// Cor de destaque no lugar do verde (`#rrggbb`).
    pub destaque: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Autor {
    pub nome: String,
    /// Data e hora da geração (ISO 8601).
    pub gerado_em: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceitaVersao {
    pub formato: String,
    pub versao_formato: u32,
    pub base: Base,
    pub identidade: Identidade,
    pub cores: Cores,
    pub inteligencias: Vec<String>,
    pub classes: Vec<usize>,
    pub autor: Autor,
    /// SHA-256 do conteúdo canônico (a própria receita sem este campo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl ReceitaVersao {
    /// Lê e valida. Se a receita traz `hash`, ele precisa bater.
    pub fn ler(json: &str) -> Result<Self, String> {
        let r: Self = serde_json::from_str(json).map_err(|e| format!("receita inválida: {e}"))?;
        r.validar()?;
        Ok(r)
    }

    /// JSON sem o hash, sempre na mesma ordem de campos: é o que se assina.
    pub fn canonico(&self) -> String {
        let mut sem = self.clone();
        sem.hash = None;
        serde_json::to_string(&sem).expect("receita serializa")
    }

    pub fn calcular_hash(&self) -> String {
        hex(&Sha256::digest(self.canonico().as_bytes()))
    }

    /// Devolve a receita com o hash preenchido.
    pub fn assinada(mut self) -> Self {
        self.hash = Some(self.calcular_hash());
        self
    }

    /// Bytes do logo, já conferidos (PNG ou JPEG, até 512 KB).
    pub fn logo_bytes(&self) -> Result<Option<(&'static str, Vec<u8>)>, String> {
        let Some(url) = &self.identidade.logo else {
            return Ok(None);
        };
        let (mime, b64) = if let Some(r) = url.strip_prefix("data:image/png;base64,") {
            ("image/png", r)
        } else if let Some(r) = url.strip_prefix("data:image/jpeg;base64,") {
            ("image/jpeg", r)
        } else {
            return Err("o logo precisa ser PNG ou JPG".into());
        };
        if b64.len() > LOGO_MAX / 3 * 4 + 8 {
            return Err("o logo passa de 512 KB".into());
        }
        let bytes = base64(b64).ok_or("o logo não está em base64 válido")?;
        if bytes.len() > LOGO_MAX {
            return Err("o logo passa de 512 KB".into());
        }
        let ok = match mime {
            "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            _ => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        };
        if !ok {
            return Err("o arquivo do logo não é a imagem que diz ser".into());
        }
        Ok(Some((mime, bytes)))
    }

    pub fn validar(&self) -> Result<(), String> {
        if self.formato != FORMATO {
            return Err(format!(
                "formato \"{}\" não é uma receita de versão",
                self.formato
            ));
        }
        if self.versao_formato != VERSAO_FORMATO {
            return Err(format!(
                "receita no formato {}; este programa lê o formato {VERSAO_FORMATO}",
                self.versao_formato
            ));
        }
        texto("versão base", &self.base.versao, 1, 20)?;
        let commit = &self.base.commit;
        if !(7..=40).contains(&commit.len()) || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("commit da versão base inválido".into());
        }
        texto("nome da versão", &self.identidade.nome, 2, 40)?;
        texto("nome da empresa", &self.identidade.empresa, 0, 40)?;
        for t in [&self.identidade.nome, &self.identidade.empresa] {
            if let Some(m) = marca_de_fabricante(t) {
                return Err(format!(
                    "\"{t}\" usa a marca {m}, que é de um fabricante; use o nome da sua empresa"
                ));
            }
        }
        self.logo_bytes()?;
        let cor = rgb(&self.cores.destaque).ok_or("cor de destaque inválida (use #rrggbb)")?;
        let c = contraste(cor, (0, 0, 0));
        if c < 4.5 {
            return Err(format!(
                "a cor de destaque some no fundo preto (contraste {c:.1}:1; o mínimo é 4,5:1); escolha uma mais clara"
            ));
        }
        if self.inteligencias.is_empty() {
            return Err("escolha pelo menos uma inteligência".into());
        }
        for (i, id) in self.inteligencias.iter().enumerate() {
            if !INTELIGENCIAS.contains(&id.as_str()) {
                return Err(format!("inteligência desconhecida: {id}"));
            }
            if self.inteligencias[..i].contains(id) {
                return Err(format!("inteligência repetida: {id}"));
            }
        }
        if self.classes.is_empty() {
            return Err("escolha pelo menos uma detecção".into());
        }
        for (i, c) in self.classes.iter().enumerate() {
            if *c >= crate::coco::TOTAL {
                return Err(format!("detecção desconhecida: {c}"));
            }
            if self.classes[..i].contains(c) {
                return Err(format!("detecção repetida: {c}"));
            }
        }
        texto("nome do autor", &self.autor.nome, 2, 60)?;
        texto("data da geração", &self.autor.gerado_em, 10, 40)?;
        if let Some(h) = &self.hash {
            if *h != self.calcular_hash() {
                return Err("a receita foi alterada depois de gerada (hash não confere)".into());
            }
        }
        Ok(())
    }
}

fn texto(campo: &str, t: &str, min: usize, max: usize) -> Result<(), String> {
    let n = t.chars().count();
    if n < min || n > max {
        return Err(format!("{campo}: use de {min} a {max} caracteres"));
    }
    if t.trim() != t {
        return Err(format!("{campo}: tire os espaços do começo e do fim"));
    }
    if t.chars()
        .any(|c| c.is_control() || matches!(c, '<' | '>' | '&' | '"' | '\\'))
    {
        return Err(format!("{campo}: não use os caracteres < > & \" \\"));
    }
    Ok(())
}

/// Marca de fabricante citada no texto, se houver.
pub fn marca_de_fabricante(t: &str) -> Option<&'static str> {
    let minusculo = sem_acento(&t.to_lowercase());
    let junto: String = minusculo
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if let Some(m) = MARCAS_LONGAS.iter().find(|m| junto.contains(*m)) {
        return Some(m);
    }
    let palavras: Vec<&str> = minusculo
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|p| !p.is_empty())
        .collect();
    MARCAS_CURTAS.iter().copied().find(|m| palavras.contains(m))
}

fn sem_acento(t: &str) -> String {
    t.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            _ => c,
        })
        .collect()
}

/// `#rrggbb` → (r, g, b).
pub fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some((v(0)?, v(2)?, v(4)?))
}

/// Razão de contraste do WCAG 2 entre duas cores (1 a 21).
pub fn contraste(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    fn lum((r, g, b): (u8, u8, u8)) -> f64 {
        let canal = |c: u8| {
            let s = f64::from(c) / 255.0;
            if s <= 0.039_28 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * canal(r) + 0.7152 * canal(g) + 0.0722 * canal(b)
    }
    let (la, lb) = (lum(a), lum(b));
    let (claro, escuro) = if la > lb { (la, lb) } else { (lb, la) };
    (claro + 0.05) / (escuro + 0.05)
}

/// Troca o verde de destaque do `web/tokens.css` pela cor da receita.
pub fn tokens_css(original: &str, destaque: &str) -> Result<String, String> {
    let (r, g, b) = rgb(destaque).ok_or("cor de destaque inválida")?;
    let mut saida = String::with_capacity(original.len());
    let (mut cor, mut canais) = (false, false);
    for linha in original.split_inclusive('\n') {
        let t = linha.trim_start();
        let recuo = &linha[..linha.len() - t.len()];
        let fim = if linha.ends_with("\r\n") {
            "\r\n"
        } else if linha.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        if t.starts_with("--c-green:") {
            saida.push_str(&format!(
                "{recuo}--c-green: {};{fim}",
                destaque.to_ascii_lowercase()
            ));
            cor = true;
        } else if t.starts_with("--c-green-rgb:") {
            saida.push_str(&format!("{recuo}--c-green-rgb: {r}, {g}, {b};{fim}"));
            canais = true;
        } else {
            saida.push_str(linha);
        }
    }
    if !(cor && canais) {
        return Err("tokens.css sem --c-green e --c-green-rgb".into());
    }
    Ok(saida)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Base64 padrão (com `=`), sem espaços.
fn base64(s: &str) -> Option<Vec<u8>> {
    fn valor(c: u8) -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    }
    let s = s.as_bytes();
    if s.is_empty() || !s.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    for (n, bloco) in s.chunks(4).enumerate() {
        let ultimo = n == s.len() / 4 - 1;
        let pad = bloco.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && !ultimo) {
            return None;
        }
        let mut v = 0u32;
        for &c in &bloco[..4 - pad] {
            v = (v << 6) | valor(c)?;
        }
        v <<= 6 * pad as u32;
        let b = v.to_be_bytes();
        out.extend_from_slice(&b[1..4 - pad]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn exemplo() -> ReceitaVersao {
        ReceitaVersao {
            formato: FORMATO.into(),
            versao_formato: VERSAO_FORMATO,
            base: Base {
                versao: "0.9.9".into(),
                commit: "e46a8b7".into(),
            },
            identidade: Identidade {
                nome: "Vigia Silva".into(),
                empresa: "Silva Segurança".into(),
                logo: None,
            },
            cores: Cores {
                destaque: "#5aa9ff".into(),
            },
            inteligencias: vec!["passagem".into(), "presenca".into()],
            classes: vec![0, 2, 16],
            autor: Autor {
                nome: "Fulano da Silva".into(),
                gerado_em: "2026-11-17T10:00:00-03:00".into(),
            },
            hash: None,
        }
    }

    #[test]
    fn receita_boa_passa_e_hash_e_estavel() {
        let r = exemplo().assinada();
        assert!(r.validar().is_ok());
        let json = serde_json::to_string(&r).unwrap();
        let lida = ReceitaVersao::ler(&json).unwrap();
        assert_eq!(lida.hash, r.hash);
        assert_eq!(lida.calcular_hash(), r.calcular_hash());
        assert_eq!(r.hash.as_ref().unwrap().len(), 64);
    }

    #[test]
    fn hash_alterado_e_recusado() {
        let mut r = exemplo().assinada();
        r.classes.push(1);
        assert!(r.validar().unwrap_err().contains("hash"));
    }

    type Mudar = fn(&mut ReceitaVersao);

    #[test]
    fn recusa_o_que_nao_pode() {
        let casos: Vec<(Mudar, &str)> = vec![
            (|r| r.formato = "outra".into(), "formato"),
            (|r| r.classes = vec![80], "detecção desconhecida"),
            (|r| r.classes = vec![2, 2], "repetida"),
            (|r| r.classes.clear(), "pelo menos uma detecção"),
            (|r| r.inteligencias = vec!["rosto".into()], "desconhecida"),
            (|r| r.inteligencias.clear(), "pelo menos uma inteligência"),
            (|r| r.identidade.nome = "x".repeat(41), "de 2 a 40"),
            (|r| r.identidade.nome = "<b>Oi</b>".into(), "não use"),
            (
                |r| r.identidade.nome = "Vigia Intelbras".into(),
                "fabricante",
            ),
            (
                |r| r.identidade.empresa = "HIK-VISION Pro".into(),
                "fabricante",
            ),
            (|r| r.identidade.nome = "JFL Câmeras".into(), "fabricante"),
            (|r| r.cores.destaque = "#123456".into(), "contraste"),
            (|r| r.cores.destaque = "azul".into(), "#rrggbb"),
            (|r| r.base.commit = "xyz".into(), "commit"),
            (
                |r| r.identidade.logo = Some("data:image/gif;base64,R0lG".into()),
                "PNG ou JPG",
            ),
        ];
        for (mudar, esperado) in casos {
            let mut r = exemplo();
            mudar(&mut r);
            let e = r.validar().unwrap_err();
            assert!(
                e.contains(esperado),
                "esperava \"{esperado}\", veio \"{e}\""
            );
        }
    }

    #[test]
    fn nomes_parecidos_com_marca_curta_passam() {
        assert_eq!(marca_de_fabricante("Jflores Segurança"), None);
        assert_eq!(marca_de_fabricante("Gigante Monitoramento"), None);
        assert_eq!(marca_de_fabricante("Câmeras Giga"), Some("giga"));
    }

    #[test]
    fn logo_png_valido() {
        // PNG mínimo: só a assinatura e um pedaço, o suficiente para conferir.
        let png = b"\x89PNG\r\n\x1a\nresto";
        let b64 = enc(png);
        let mut r = exemplo();
        r.identidade.logo = Some(format!("data:image/png;base64,{b64}"));
        let (mime, bytes) = r.logo_bytes().unwrap().unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(bytes, png);
        r.identidade.logo = Some(format!("data:image/jpeg;base64,{b64}"));
        assert!(r.validar().unwrap_err().contains("não é a imagem"));
    }

    #[test]
    fn contraste_do_wcag() {
        assert!((contraste((255, 255, 255), (0, 0, 0)) - 21.0).abs() < 0.01);
        // O verde oficial passa com folga sobre o preto.
        assert!(contraste((6, 203, 63), (0, 0, 0)) > 9.0);
    }

    #[test]
    fn troca_o_verde_no_tokens() {
        let css = ":root {\r\n  --c-green: #06cb3f;\r\n  --c-green-rgb: 6, 203, 63;\r\n  --linha-verde: #06cb3f;\r\n}\r\n";
        let novo = tokens_css(css, "#5AA9FF").unwrap();
        assert!(novo.contains("  --c-green: #5aa9ff;\r\n"));
        assert!(novo.contains("--c-green-rgb: 90, 169, 255;"));
        assert!(novo.contains("--linha-verde: #06cb3f;"));
        assert!(tokens_css(":root {}", "#5aa9ff").is_err());
    }

    #[test]
    fn base64_ida_e_volta() {
        for t in [&b""[..], b"a", b"ab", b"abc", b"abcd", b"\x00\xff\x10"] {
            if t.is_empty() {
                continue;
            }
            assert_eq!(base64(&enc(t)).unwrap(), t);
        }
        assert!(base64("abc").is_none());
        assert!(base64("a=bc").is_none());
    }

    fn enc(b: &[u8]) -> String {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::new();
        for ch in b.chunks(3) {
            let v = (u32::from(ch[0]) << 16)
                | (u32::from(*ch.get(1).unwrap_or(&0)) << 8)
                | u32::from(*ch.get(2).unwrap_or(&0));
            for i in 0..4 {
                if i <= ch.len() {
                    s.push(T[(v >> (18 - 6 * i) & 63) as usize] as char);
                } else {
                    s.push('=');
                }
            }
        }
        s
    }
}
