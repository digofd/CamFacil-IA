# Deploy — https://cfia.ysafe.com.br

A API da nuvem (`crates/cfia-api`) roda no Dokploy com um Postgres ao lado. Ela serve o site, o login com Google, o pareamento e a sinalização dos Agentes-Cam-Elo-IA. Vídeo nunca passa por ela ([SDD-08](sdd/SDD-08-contas-e-isolamento.md)).

## 1. DNS
Registro `A` de `cfia.ysafe.com.br` apontando para o IP do servidor do Dokploy.

## 2. Projeto no Dokploy
1. Crie o projeto **CamFácil IA** e, dentro dele, um serviço do tipo **Docker Compose**.
2. Origem: repositório `emersoncassis/CamFacil-IA`, branch `main`, arquivo `deploy/docker-compose.dokploy.yml`.
3. **Environment** (o dono preenche):
   ```
   POSTGRES_PASSWORD=<senha longa e aleatória>
   GOOGLE_CLIENT_ID=<...apps.googleusercontent.com>
   GOOGLE_CLIENT_SECRET=<...>
   ```
4. **Domains:** `cfia.ysafe.com.br` → serviço `cfia-api`, porta `8080`, HTTPS com Let's Encrypt.
5. **Deploy.** Na primeira subida, a API espera o banco e cria as tabelas sozinha.

Sem `GOOGLE_CLIENT_ID`/`GOOGLE_CLIENT_SECRET`, o site sobe normalmente e o botão de login avisa que ainda não está configurado.

## Alternativa: pelo terminal (sem o painel)
No servidor do Dokploy, como root:
```sh
curl -fsSL https://raw.githubusercontent.com/emersoncassis/CamFacil-IA/main/scripts/deploy-servidor.sh | sh
```
O script clona em `/opt/camfacil`, gera a senha do banco em `/opt/camfacil/deploy/.env`, sobe API + Postgres com o Traefik do Dokploy (HTTPS automático) e testa `/api/saude`. Para colocar as chaves do Google, edite o `.env` e rode o script de novo. Esse modo não aparece no painel do Dokploy.

## 3. Google Auth Platform
- Cliente **Aplicativo da Web**.
- Origem autorizada: `https://cfia.ysafe.com.br`.
- Redirecionamento: `https://cfia.ysafe.com.br/api/auth/google/callback`.
- Escopos: `openid`, `email`, `profile`.
- Branding: links `https://cfia.ysafe.com.br/privacidade` e `https://cfia.ysafe.com.br/termos`.
- Publique o app (Audience → Publicar) antes de abrir para os técnicos.

## 4. Conferir
- `https://cfia.ysafe.com.br/api/saude` responde `{"ok":true,...}`.
- O login com Google leva de volta ao painel "Agentes".

## Gerar sua versão (SDD-12)

Duas peças: a fila no `cfia-api` (este servidor) e o compilador numa VPS separada ([ADR 0018](adr/0018-gerar-sua-versao-compilada-na-vps.md)).

**No `cfia-api`** (Environment do Dokploy, ou `/opt/camfacil/deploy/.env` no modo terminal):
```
CFIA_BASE_COMMIT=<commit do main que os técnicos recebem como base>
CFIA_BUILDER_TOKEN=<o mesmo token do compilador, 32+ caracteres>
CFIA_SMTP_URL=smtps://usuario:senha@smtp.provedor.com:465   # opcional
CFIA_EMAIL_DE=CamFácil IA <nao-responda@cfia.ysafe.com.br>   # opcional
```
Os executáveis ficam no volume `cfia-versoes` (`/dados/versoes`). Sem `CFIA_BASE_COMMIT`, a página `/versao` diz que a função não está ligada. A cada versão oficial nova, troque `CFIA_BASE_COMMIT` pelo commit dela.

**No compilador** (VPS própria, com Docker):
```sh
git clone https://github.com/emersoncassis/CamFacil-IA.git && cd CamFacil-IA
printf 'CFIA_API=https://cfia.ysafe.com.br
CFIA_BUILDER_TOKEN=<o mesmo token>
' > deploy/builder/.env
chmod 600 deploy/builder/.env
docker compose -f deploy/builder/docker-compose.yml up -d --build
```
O primeiro build baixa as dependências e leva uns 5 minutos; os seguintes, perto de 1 minuto e meio. Ele só sai para a internet: não abra porta nenhuma.

## Backup
Ative o backup do volume `cfia-db-dados` no Dokploy (diário). O banco guarda só contas, dispositivos, agentes e auditoria.

## Rodar localmente
```sh
DATABASE_URL="host=/caminho/do/socket port=5432 user=postgres dbname=cfia" \
BASE_URL=http://localhost:8080 cargo run -p cfia-api
```
Testes de integração (precisam de um Postgres):
```sh
TEST_DATABASE_URL="postgres://postgres@localhost/cfia_teste" cargo test -p cfia-api
```
