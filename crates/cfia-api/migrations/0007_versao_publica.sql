-- "Gerar sua versão" (SDD-12): contador de downloads e link público.
-- O link é um código aleatório próprio (não o id da versão): dá para trocar
-- ou apagar sem mexer no resto, e quem tem o link baixa sem login.

ALTER TABLE versoes ADD COLUMN IF NOT EXISTS downloads integer NOT NULL DEFAULT 0;
ALTER TABLE versoes ADD COLUMN IF NOT EXISTS link text;
CREATE UNIQUE INDEX IF NOT EXISTS versoes_link ON versoes(link) WHERE link IS NOT NULL;
