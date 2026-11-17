-- "Gerar sua versão" (SDD-12): a fila de compilação das versões dos técnicos.
-- A receita (JSON validado pelo cf-versao, com o logo em base64) e o estado
-- do build. O executável fica no disco (CFIA_VERSOES_DIR), não no banco.

CREATE TABLE IF NOT EXISTS versoes (
    id                 uuid PRIMARY KEY,
    tecnico_id         uuid NOT NULL REFERENCES tecnicos(id) ON DELETE CASCADE,
    nome               text NOT NULL,
    receita            jsonb NOT NULL,
    receita_sha256     text NOT NULL,
    base_commit        text NOT NULL,
    estado             text NOT NULL DEFAULT 'fila'
                       CHECK (estado IN ('fila', 'compilando', 'pronta', 'erro')),
    etapa              text NOT NULL DEFAULT 'fila',
    percentual         smallint NOT NULL DEFAULT 0 CHECK (percentual BETWEEN 0 AND 100),
    erro               text,
    executavel_bytes   bigint,
    executavel_sha256  text,
    criado_em          timestamptz NOT NULL DEFAULT now(),
    iniciado_em        timestamptz,
    terminado_em       timestamptz,
    -- Logo de até 512 KB em base64 cabe com folga.
    CHECK (octet_length(receita::text) <= 800000)
);
CREATE INDEX IF NOT EXISTS versoes_fila ON versoes(criado_em) WHERE estado = 'fila';
CREATE INDEX IF NOT EXISTS versoes_tecnico ON versoes(tecnico_id, criado_em DESC);
-- Uma versão por vez na fila (ou compilando) para cada técnico.
CREATE UNIQUE INDEX IF NOT EXISTS versoes_uma_por_vez ON versoes(tecnico_id)
    WHERE estado IN ('fila', 'compilando');

GRANT SELECT, INSERT, UPDATE, DELETE ON versoes TO cfia_app;
ALTER TABLE versoes ENABLE ROW LEVEL SECURITY;

-- O técnico vê e pede as dele; o andamento do build só o sistema escreve.
DROP POLICY IF EXISTS leitura ON versoes;
CREATE POLICY leitura ON versoes FOR SELECT
    USING (cfia_sistema() OR tecnico_id = cfia_tecnico());
DROP POLICY IF EXISTS insercao ON versoes;
CREATE POLICY insercao ON versoes FOR INSERT
    WITH CHECK (cfia_sistema() OR (tecnico_id = cfia_tecnico() AND estado = 'fila'));
DROP POLICY IF EXISTS alteracao ON versoes;
CREATE POLICY alteracao ON versoes FOR UPDATE
    USING (cfia_sistema()) WITH CHECK (cfia_sistema());
DROP POLICY IF EXISTS remocao ON versoes;
CREATE POLICY remocao ON versoes FOR DELETE
    USING (cfia_sistema() OR tecnico_id = cfia_tecnico());
