//! As 80 classes do COCO, que o modelo padrão (`modelos/yolo11n.onnx`)
//! enxerga, com nome em pt-BR, plural para o balão, grupo do assistente
//! "Gerar sua versão" (SDD-12) e o aviso de baixa precisão.

use serde::Serialize;

/// Grupo em que a classe aparece no assistente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Grupo {
    PessoasVeiculos,
    OutrosVeiculos,
    Animais,
    ObjetosPessoas,
    Rua,
    Outros,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Classe {
    /// Índice da classe na saída do modelo.
    pub indice: usize,
    /// Id curto, sem acento (chave de tradução `classe.<id>`).
    pub id: &'static str,
    pub nome: &'static str,
    pub plural: &'static str,
    pub grupo: Grupo,
    /// O YOLO11n erra muito com objeto pequeno ou longe: "use para estudo".
    pub baixa_precisao: bool,
}

/// Quantas classes o modelo padrão tem.
pub const TOTAL: usize = 80;

/// Classes liberadas no programa oficial (sem receita), na ordem da tela.
pub const PADRAO: &[usize] = &[0, 2, 3, 1, 5, 7];

const fn c(
    indice: usize,
    id: &'static str,
    nome: &'static str,
    plural: &'static str,
    grupo: Grupo,
) -> Classe {
    let baixa_precisao = matches!(grupo, Grupo::Outros) || matches!(indice, 24 | 26 | 67);
    Classe {
        indice,
        id,
        nome,
        plural,
        grupo,
        baixa_precisao,
    }
}

use Grupo::{Animais, ObjetosPessoas, Outros, OutrosVeiculos, PessoasVeiculos, Rua};

pub const CLASSES: [Classe; TOTAL] = [
    c(0, "pessoa", "pessoa", "pessoas", PessoasVeiculos),
    c(1, "bicicleta", "bicicleta", "bicicletas", PessoasVeiculos),
    c(2, "carro", "carro", "carros", PessoasVeiculos),
    c(3, "moto", "moto", "motos", PessoasVeiculos),
    c(4, "aviao", "avião", "aviões", OutrosVeiculos),
    c(5, "onibus", "ônibus", "ônibus", PessoasVeiculos),
    c(6, "trem", "trem", "trens", OutrosVeiculos),
    c(7, "caminhao", "caminhão", "caminhões", PessoasVeiculos),
    c(8, "barco", "barco", "barcos", OutrosVeiculos),
    c(9, "semaforo", "semáforo", "semáforos", Rua),
    c(10, "hidrante", "hidrante", "hidrantes", Rua),
    c(11, "placa_pare", "placa de pare", "placas de pare", Rua),
    c(12, "parquimetro", "parquímetro", "parquímetros", Rua),
    c(13, "banco", "banco", "bancos", Rua),
    c(14, "passaro", "pássaro", "pássaros", Animais),
    c(15, "gato", "gato", "gatos", Animais),
    c(16, "cachorro", "cachorro", "cachorros", Animais),
    c(17, "cavalo", "cavalo", "cavalos", Animais),
    c(18, "ovelha", "ovelha", "ovelhas", Animais),
    c(19, "vaca", "vaca", "vacas", Animais),
    c(20, "elefante", "elefante", "elefantes", Outros),
    c(21, "urso", "urso", "ursos", Outros),
    c(22, "zebra", "zebra", "zebras", Outros),
    c(23, "girafa", "girafa", "girafas", Outros),
    c(24, "mochila", "mochila", "mochilas", ObjetosPessoas),
    c(
        25,
        "guarda_chuva",
        "guarda-chuva",
        "guarda-chuvas",
        ObjetosPessoas,
    ),
    c(26, "bolsa", "bolsa", "bolsas", ObjetosPessoas),
    c(27, "gravata", "gravata", "gravatas", Outros),
    c(28, "mala", "mala", "malas", ObjetosPessoas),
    c(29, "frisbee", "frisbee", "frisbees", Outros),
    c(30, "esqui", "esqui", "esquis", Outros),
    c(31, "snowboard", "snowboard", "snowboards", Outros),
    c(32, "bola", "bola", "bolas", Outros),
    c(33, "pipa", "pipa", "pipas", Outros),
    c(
        34,
        "taco_beisebol",
        "taco de beisebol",
        "tacos de beisebol",
        Outros,
    ),
    c(
        35,
        "luva_beisebol",
        "luva de beisebol",
        "luvas de beisebol",
        Outros,
    ),
    c(36, "skate", "skate", "skates", Outros),
    c(
        37,
        "prancha_surfe",
        "prancha de surfe",
        "pranchas de surfe",
        Outros,
    ),
    c(38, "raquete", "raquete", "raquetes", Outros),
    c(39, "garrafa", "garrafa", "garrafas", Outros),
    c(40, "taca", "taça", "taças", Outros),
    c(41, "copo", "copo", "copos", Outros),
    c(42, "garfo", "garfo", "garfos", Outros),
    c(43, "faca", "faca", "facas", Outros),
    c(44, "colher", "colher", "colheres", Outros),
    c(45, "tigela", "tigela", "tigelas", Outros),
    c(46, "banana", "banana", "bananas", Outros),
    c(47, "maca", "maçã", "maçãs", Outros),
    c(48, "sanduiche", "sanduíche", "sanduíches", Outros),
    c(49, "laranja", "laranja", "laranjas", Outros),
    c(50, "brocolis", "brócolis", "brócolis", Outros),
    c(51, "cenoura", "cenoura", "cenouras", Outros),
    c(
        52,
        "cachorro_quente",
        "cachorro-quente",
        "cachorros-quentes",
        Outros,
    ),
    c(53, "pizza", "pizza", "pizzas", Outros),
    c(54, "rosquinha", "rosquinha", "rosquinhas", Outros),
    c(55, "bolo", "bolo", "bolos", Outros),
    c(56, "cadeira", "cadeira", "cadeiras", Outros),
    c(57, "sofa", "sofá", "sofás", Outros),
    c(
        58,
        "vaso_planta",
        "vaso de planta",
        "vasos de planta",
        Outros,
    ),
    c(59, "cama", "cama", "camas", Outros),
    c(60, "mesa", "mesa", "mesas", Outros),
    c(
        61,
        "vaso_sanitario",
        "vaso sanitário",
        "vasos sanitários",
        Outros,
    ),
    c(62, "tv", "TV", "TVs", Outros),
    c(63, "notebook", "notebook", "notebooks", Outros),
    c(64, "mouse", "mouse", "mouses", Outros),
    c(
        65,
        "controle_remoto",
        "controle remoto",
        "controles remotos",
        Outros,
    ),
    c(66, "teclado", "teclado", "teclados", Outros),
    c(67, "celular", "celular", "celulares", ObjetosPessoas),
    c(68, "micro_ondas", "micro-ondas", "micro-ondas", Outros),
    c(69, "forno", "forno", "fornos", Outros),
    c(70, "torradeira", "torradeira", "torradeiras", Outros),
    c(71, "pia", "pia", "pias", Outros),
    c(72, "geladeira", "geladeira", "geladeiras", Outros),
    c(73, "livro", "livro", "livros", Outros),
    c(74, "relogio", "relógio", "relógios", Outros),
    c(75, "vaso", "vaso", "vasos", Outros),
    c(76, "tesoura", "tesoura", "tesouras", Outros),
    c(
        77,
        "urso_pelucia",
        "urso de pelúcia",
        "ursos de pelúcia",
        Outros,
    ),
    c(
        78,
        "secador",
        "secador de cabelo",
        "secadores de cabelo",
        Outros,
    ),
    c(
        79,
        "escova_dentes",
        "escova de dentes",
        "escovas de dentes",
        Outros,
    ),
];

pub fn achar(indice: usize) -> Option<&'static Classe> {
    CLASSES.get(indice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_batem_com_a_posicao_e_ids_sao_unicos() {
        for (i, c) in CLASSES.iter().enumerate() {
            assert_eq!(c.indice, i);
            assert!(c.id.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_'));
        }
        let mut ids: Vec<_> = CLASSES.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), TOTAL);
    }

    #[test]
    fn padrao_sao_as_seis_de_sempre() {
        let ids: Vec<_> = PADRAO.iter().map(|&i| CLASSES[i].id).collect();
        assert_eq!(
            ids,
            ["pessoa", "carro", "moto", "bicicleta", "onibus", "caminhao"]
        );
        assert!(PADRAO.iter().all(|&i| !CLASSES[i].baixa_precisao));
    }
}
