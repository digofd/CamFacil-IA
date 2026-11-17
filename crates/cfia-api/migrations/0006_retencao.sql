-- "Gerar sua versão" (SDD-12), retenção:
-- 1. Atividade do técnico, para apagar as versões após 30 dias sem entrar e
--    avisar por e-mail no 23º dia.
-- 2. Registro de geração: quem gerou qual versão (hash da receita), por 12
--    meses. Sem chave estrangeira de propósito: sobrevive à limpeza do tenant
--    e à exclusão da conta (LGPD, art. 7º, VI e IX; ver a política de privacidade).

ALTER TABLE tecnicos ADD COLUMN IF NOT EXISTS ultimo_acesso timestamptz NOT NULL DEFAULT now();
ALTER TABLE tecnicos ADD COLUMN IF NOT EXISTS aviso_retencao_em timestamptz;

CREATE TABLE IF NOT EXISTS registro_versoes (
    id              bigserial PRIMARY KEY,
    versao_id       uuid NOT NULL,
    tecnico_id      uuid,
    conta_nome      text NOT NULL DEFAULT '',
    conta_email     text NOT NULL,
    receita_sha256  text NOT NULL,
    base_versao     text NOT NULL,
    base_commit     text NOT NULL,
    termo           text NOT NULL,
    ip              text NOT NULL DEFAULT '',
    gerado_em       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS registro_versoes_hash ON registro_versoes(receita_sha256);
CREATE INDEX IF NOT EXISTS registro_versoes_data ON registro_versoes(gerado_em);

GRANT SELECT, INSERT, DELETE ON registro_versoes TO cfia_app;
GRANT USAGE, SELECT ON SEQUENCE registro_versoes_id_seq TO cfia_app;
ALTER TABLE registro_versoes ENABLE ROW LEVEL SECURITY;

-- O técnico só escreve o próprio registro (ao pedir a versão); não lê, não
-- altera, não apaga. Consulta e expiração (12 meses) são do sistema.
DROP POLICY IF EXISTS insercao ON registro_versoes;
CREATE POLICY insercao ON registro_versoes FOR INSERT
    WITH CHECK (cfia_sistema() OR tecnico_id = cfia_tecnico());
DROP POLICY IF EXISTS leitura ON registro_versoes;
CREATE POLICY leitura ON registro_versoes FOR SELECT USING (cfia_sistema());
DROP POLICY IF EXISTS remocao ON registro_versoes;
CREATE POLICY remocao ON registro_versoes FOR DELETE USING (cfia_sistema());
