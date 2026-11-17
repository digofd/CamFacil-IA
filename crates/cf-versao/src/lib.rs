//! "Gerar sua versão" (SDD-12): formato e validação da receita da versão e a
//! tabela das 80 classes do COCO. Usado pelo `build.rs` do `camfacil` (aplica a
//! receita na compilação), pelo próprio `camfacil` e pelo `cf-core`.

pub mod coco;
pub mod receita;

pub use receita::ReceitaVersao;
