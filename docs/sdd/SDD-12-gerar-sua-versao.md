# SDD-12 — Gerar sua versão

Status: **em produção** · Etapas 1 (receita e build local) e 2 (fila e compilador) feitas na branch `feat/gerar-sua-versao` · complementa o CamFácil IA Builder (`.claude/skills/camfacil-ia-builder/`), o [SDD-08](SDD-08-contas-e-isolamento.md) (contas, isolamento e auditoria), o [SDD-07](SDD-07-modo-estudio.md) (receitas e assistente do estúdio) e o [SDD-09](SDD-09-inteligencias-por-regra.md) (inteligências por regra) · criado em 2026-11-16

## Objetivo

Deixar o técnico que não programa montar a **versão dele** do CamFácil IA por um assistente em passos, dentro da plataforma (`cfia-api`), sem fork, sem `git`, sem `cargo` e sem assistente de código. Ele escolhe nome, logo, cores, inteligências e detecções. A VPS compila. Ele acompanha o progresso e baixa o executável e a **receita da versão**.

É o **Builder Express**: cobre o pedido "mudar a cara do programa" e a escolha do que aparece. Função nova, integração nova ou modelo novo continuam no Builder de chat.

## Por que compilar (e não configurar com o programa rodando)

A pasta `web/` entra no binário na compilação (`include_bytes!` em `crates/camfacil/src/assets.rs`). Tema (`web/tokens.css`), textos (`web/i18n/pt-BR.json`), logo e ícone ficam fixos no executável. Mudar a cara de verdade é compilar.

## Escopo da primeira versão

**Dentro:**

- Executável **Windows x86_64**.
- Nome da versão, nome da empresa, logo, ícone e cores.
- Escolha das inteligências por regra e das classes de detecção do modelo padrão.
- Fila de compilação na VPS, com progresso e tempo estimado.
- Receita para baixar e para importar (regerar a versão).
- Registro de quem gerou cada versão.

**Fora (etapas seguintes):**

- Linux x86_64, Linux ARM64 e APK Android (o `rust.yml` já sabe gerar; falta a VPS).
- "Salvar no meu Drive" (escopo `drive.file` do Google).
- Assinatura de código do executável. Sem ela o Windows SmartScreen mostra aviso; a tela de download explica isso.
- Qualquer mudança de comportamento, função ou integração. Isso é com o Builder de chat.

## Onde fica

Dentro do tenant do técnico, logado com o Google (SDD-08). Menu **Gerar sua versão** no `cfia-api`. Sem login, a página só apresenta a função.

## O assistente

Uma pergunta por tela, sem rolagem, como o assistente do estúdio.

1. **Identidade.** Nome da versão (ex.: "Vigia Silva"), nome da empresa, logo (PNG ou JPG, até 512 KB). O ícone da aba do navegador é o de sempre, com a cor de destaque da versão.
2. **Cor.** Cor de destaque, no lugar do verde (`--c-green`). A base continua preta (`DESIGN.md`). O assistente mostra uma tela de exemplo ao vivo e **recusa cor sem contraste**: pelo menos 4,5:1 sobre o fundo preto, que vale tanto para detalhe quanto para texto.
3. **Inteligências.** As cinco do `cf_core::inteligencias::INTELIGENCIAS` (passagem, presença, densidade, permanência, calor), com o que cada uma faz e o peso (leve, médio, pesado). As não escolhidas somem dos menus.
4. **Detecções.** As classes do modelo padrão, em grupos (seção abaixo), com o aviso de foco.
5. **Revisão e termo.** Resumo das escolhas, termo de responsabilidade da versão (seção "Responsabilidade") e o botão **Gerar**.
6. **Compilando.** Posição na fila, etapa, porcentagem e tempo estimado (seção "Progresso").
7. **Pronto.** Baixar o executável e **baixar a receita**, com o aviso de que o tenant é apagado depois de 30 dias sem login.

Atalho no começo: **Importar receita** pula para a revisão com tudo preenchido.

### Aviso das detecções

> Escolha só o que você vai usar com esse cliente. Marcar tudo não deixa o programa mais lento, porque o modelo sempre enxerga todas as classes. Mas enche os menus de opção que você não vende e aumenta os alertas falsos. Menos classe é mais foco.

## Detecções do modelo padrão

O modelo padrão é o `modelos/yolo11n.onnx` (YOLO11n, COCO, 80 classes). Hoje o programa libera seis (`CLASSES_ESCOLHA` em `crates/cf-core/src/inteligencias.rs`). O assistente lista as 80 em grupos. O índice COCO é o id na receita.

| Grupo | Classes (índice COCO) | Começa |
|---|---|---|
| Pessoas e veículos | pessoa (0), bicicleta (1), carro (2), moto (3), ônibus (5), caminhão (7) | marcadas (as seis de hoje) |
| Outros veículos | avião (4), trem (6), barco (8) | desmarcadas |
| Animais | pássaro (14), gato (15), cachorro (16), cavalo (17), ovelha (18), vaca (19) | desmarcadas |
| Objetos de pessoas | mochila (24), guarda-chuva (25), bolsa (26), mala (28), celular (67) | desmarcadas |
| Rua | semáforo (9), hidrante (10), placa de pare (11), parquímetro (12), banco (13) | desmarcadas |
| Outros | as demais classes do COCO (animais de zoológico, esporte, cozinha, móveis, eletrônicos, objetos da casa) | grupo fechado e desmarcado |

- **Baixa precisão.** O YOLO11n é o menor modelo. Objeto pequeno ou longe erra muito. Celular, mochila, bolsa e todo o grupo "Outros" levam o selo "baixa precisão, use para estudo".
- **Biometria.** Nenhuma classe do COCO identifica pessoa. A camada 1 do Builder (LGPD, art. 11) continua garantida.
- **Mínimo.** Pelo menos uma classe. **Máximo por regra** continua 12 (`validar`).
- **Modo estúdio** não muda: modelo de estudo continua trazendo as classes da própria receita de estudo.

## Receita da versão

Um **JSON só**, validado por `cf_versao::receita` (crate `crates/cf-versao`). O logo vai dentro, em base64 (`data:image/png;base64,…`), para a receita ser um arquivo único fácil de guardar.

```json
{
  "formato": "cfia-versao",
  "versao_formato": 1,
  "base": { "versao": "0.9.9", "commit": "e46a8b7" },
  "identidade": { "nome": "Vigia Silva", "empresa": "Silva Segurança", "logo": "data:image/png;base64,…" },
  "cores": { "destaque": "#5aa9ff" },
  "inteligencias": ["passagem", "presenca", "permanencia"],
  "classes": [0, 2, 3, 16],
  "autor": { "nome": "Fulano da Silva", "gerado_em": "2026-11-16T10:00:00-03:00" },
  "hash": "SHA-256 do JSON canônico sem o campo hash"
}
```

- Sem senha, token, e-mail do cliente ou qualquer dado de cliente.
- Texto livre (nome e empresa) entra **como dado**. Nunca vira código: o `build.rs` grava como literal Rust escapado. Até 40 caracteres, sem `< > & " \` nem caracteres de controle.
- Recusa: formato ou versão de formato errados, commit que não é hexadecimal, marca de fabricante no nome ou na empresa, logo que não é PNG/JPG de verdade (confere os primeiros bytes) ou passa de 512 KB, cor sem contraste, inteligência desconhecida ou repetida, classe fora de 0–79 ou repetida, lista vazia, e hash que não confere (receita alterada à mão).
- Receita importada de outra versão base é aceita se o formato bater; o assistente avisa o que mudou na versão nova.

## O que a receita muda no código

Regra: a receita mexe **só no crate `camfacil`**. Medido na KVM1: trocar de receita recompila só ele (1 min 19 s).

- **`crates/cf-versao`** (novo, sem dependências pesadas): formato e validação da receita, hash, contraste do WCAG, troca da cor no `tokens.css` e a tabela das 80 classes do COCO (`coco::CLASSES`: id, nome, plural, grupo, baixa precisão). Pequeno de propósito, porque o `build.rs` usa como dependência de compilação e não pode puxar o `cf-core`, que compila o SQLite.
- **`crates/camfacil/build.rs`** lê `CFIA_RECEITA=/caminho/receita.json`, valida de novo, assina e gera em `OUT_DIR`: `tokens.css` e `icon.svg` com a cor, `logo.bin`, `receita.json` e `versao.rs` (nome, empresa, autor, data, versão base, hash, inteligências e classes). Sem `CFIA_RECEITA`, gera os arquivos do programa oficial.
- **`crates/camfacil/src/versao.rs`** expõe as constantes, a receita embutida, o logo e o link do código.
- **`assets.rs`** serve o `tokens.css` e o `icon.svg` gerados, e o logo em `/marca` (só numa versão com logo).
- **Detector:** antes ele descartava tudo que não fosse as 6 classes. Agora `cf_core::deteccao::decodificar_classes` observa a lista da versão; `nome_classe` e `plural` cobrem as 80.
- **Regras:** `cf_core::inteligencias::validar_com` aceita as classes da versão. Criar regra com inteligência fora da versão dá erro 400. `GET /api/inteligencias` só lista o que a versão tem.
- **Tokens:** `--c-green-rgb` entrou no `tokens.css`, e as transparências do verde (`app.css` e o mapa de calor do `app.js`) passaram a usar ele. A opção de linha "verde" (`--linha-verde`) ficou fixa em verde, para não virar azul numa versão azul.
- **Tela:** nome da versão no título e no menu lateral, logo no lugar do ícone, e no Início (Máquina) as linhas "Versão modificada", "Versão base", "Receita" (hash e **Exportar receita**) e "Código-fonte". O programa oficial também passa a mostrar o link do código-fonte.
- **API:** `GET /api/versao` (informações do "Sobre") e `GET /api/versao/receita.json` (exporta a receita; 404 no oficial).
- **Sem receita, o binário sai igual ao oficial.** O teste `versao::sem_receita_e_igual_ao_oficial` confere inteligências, classes, `tokens.css`, `icon.svg` e a ausência de logo e receita.
- Impacto no binário: a tabela das 80 classes e a receita somam poucos KB; o logo vai até 512 KB. Executável Windows medido: 6,5 MB.

### Conferido na Etapa 1 (2026-11-17)

- Receita de exemplo (azul, logo PNG, 3 inteligências, 3 classes) compilada no Windows: título e menu com "Vigia Silva", logo no menu, cor azul, "Sobre" com o aviso e o hash, catálogo só com o escolhido, exportação baixa a receita.
- **Ida e volta:** a receita exportada pelo programa compilou na KVM1 (mingw-w64, release) e deu o mesmo hash. Uma segunda receita (laranja, pessoa, gato, cachorro e cavalo) recompilou só o `camfacil` em 1 min 19 s, com pico de 1,1 GB. O `.exe` rodou no Windows 11 com 23 MB parado.
- Corrigido junto: o `app.js` chamava `iniciarTela()` antes de o `tela.js` carregar, e a tela podia travar em "Conectando". O bug já existia no 0.9.9.

## Compilação na VPS

Contêiner **`cfia-builder`** numa **VPS própria** (KVM1: 4 GB de RAM, 1 vCPU), separada da VPS do site. Compilar não disputa CPU nem memória com o `cfia-api`.

- **Ele busca o trabalho.** O `cfia-builder` pergunta ao `cfia-api` por HTTPS, com um token só dele, se há receita na fila. Recebe a receita, manda o progresso e envia o executável pronto. A VPS do builder não abre porta nenhuma para a internet.
- **Isolado.** Sem acesso ao banco nem aos segredos do site; o token dele só serve para a fila. Baixa as dependências com `cargo fetch --locked` e compila com `cargo build --offline --locked`: a compilação em si não usa a rede.
- **Limites.** **Um build por vez**. Build parado há mais de 40 minutos vira erro na próxima consulta do compilador. Compila só o pacote `camfacil` (`-p camfacil`), nunca o workspace inteiro. Swap de 4 GB como folga para a etapa final.
- **Cache.** Volume `cfia-builder-cache` com o código, as dependências e a pasta `target/`.
- **Windows a partir do Linux.** Compilação cruzada com o **mingw-w64** (alvo `x86_64-pc-windows-gnu`), que é software livre. O `cargo-xwin` (alvo MSVC, o do build oficial) foi descartado: ele baixa o CRT e o SDK do Windows, que exigem aceitar a licença da Microsoft. O build oficial do GitHub continua em MSVC; só as versões geradas saem em GNU.

### Como está feito (Etapa 2)

**No `cfia-api`** (`crates/cfia-api/src/versoes.rs`, migração `0005_versoes.sql`):

| Rota | Quem | O que faz |
|---|---|---|
| `POST /api/versoes` `{receita}` | técnico | Carimba versão base (`CFIA_BASE_COMMIT`), autor (nome da conta) e data, assina, valida com o `cf-versao` e põe na fila. 409 se já tem uma na fila; 429 na 4ª em 24 h; 503 sem `CFIA_BASE_COMMIT`. |
| `GET /api/versoes` | técnico | As últimas 20 dele. |
| `GET /api/versoes/{id}` | técnico | Estado, etapa, porcentagem, posição na fila e tempo médio dos últimos 10 builds. |
| `GET /api/versoes/{id}/receita.json` | técnico | A receita assinada. |
| `GET /api/versoes/{id}/executavel` | técnico | O `.exe`, com o nome da versão (`vigia-silva.exe`). |
| `POST /api/builder/pegar` | compilador | O mais antigo da fila (`FOR UPDATE SKIP LOCKED`), ou 204. |
| `POST /api/builder/{id}/progresso` | compilador | `{etapa: preparando, compilando ou montando, percentual}`; a porcentagem só sobe. |
| `PUT /api/builder/{id}/executavel` | compilador | O `.exe` (até 40 MB, começa com `MZ`), gravado em `CFIA_VERSOES_DIR`. |
| `POST /api/builder/{id}/erro` | compilador | `{mensagem}` (até 300 caracteres). |

- Tabela `versoes` com RLS: o técnico lê e pede só as dele; só o sistema escreve o andamento. Índice único garante uma por vez na fila.
- O compilador entra com `Authorization: Bearer <CFIA_BUILDER_TOKEN>` (32+ caracteres, comparação de tempo constante). As rotas `/api/builder/` não passam pela checagem de `Origin`, porque não usam cookie.
- O executável fica no disco, não no banco. O volume de `CFIA_VERSOES_DIR` precisa ser gravável pelo contêiner da API.

**Na KVM1** (`deploy/builder/`): imagem `rust:1-bookworm` com mingw-w64 e jq, usuário sem privilégio, `trabalhador.sh` em laço. Para cada pedido, ele:
1. confere que o commit da base saiu do ramo oficial (`CFIA_REF`, padrão `main`) com `git merge-base --is-ancestor`, e recusa qualquer outro;
2. compila com a receita, contando as unidades do cargo (`--message-format=json`) para a porcentagem; quando falta só o programa, a etapa vira "montando" (98%);
3. envia o `.exe`, ou a primeira linha de erro do compilador.

Ligar: criar `deploy/builder/.env` com `CFIA_API` e `CFIA_BUILDER_TOKEN` e rodar `docker compose -f deploy/builder/docker-compose.yml up -d --build`. No `cfia-api`: `CFIA_BUILDER_TOKEN`, `CFIA_BASE_COMMIT` e `CFIA_VERSOES_DIR`.

### Conferido na Etapa 2 (2026-11-17)

- Testes de integração da fila contra Postgres 16 na KVM1: pedido, marca de fabricante recusada, uma por vez, isolamento entre técnicos, token errado, progresso, executável inválido recusado, download só do dono, limite de 3 por dia.
- **De ponta a ponta na KVM1:** `cfia-api` de teste, compilador no contêiner e um técnico de teste pedindo pela API.

| Build | Do pedido ao pronto | Etapas vistas pelo técnico |
|---|---|---|
| A frio (contêiner novo, sem cache) | 5 min 6 s | compilando 1% → 84% → pronta (sem "montando": o total de unidades ainda não era conhecido) |
| Com cache (outra receita) | 1 min 35 s | compilando 24% → montando 98% (1 min 33 s) → pronta |

- O `.exe` baixado pela rota do técnico tem o mesmo SHA-256 que o compilador enviou, abre no Windows e traz no "Sobre" o autor e a base carimbados pelo servidor e o hash da fila.
- Pico do contêiner: ~470 MB parado depois do build. A imagem tem 4 GB (Rust, mingw-w64), e o disco da KVM1 foi de 7,4 GB para 25 GB usados nos testes.
- O primeiro pedido falhou com "não foi possível baixar o código oficial": o repositório de teste (local, de outro dono) caiu na proteção "dubious ownership" do git. Em produção o compilador clona do GitHub.

### Medição na KVM1 (2026-11-17)

Ubuntu 24.04, 1 vCPU (AMD EPYC 9354P), 3,8 GB de RAM, swap de 4 GB, Rust 1.100, mingw-w64 13, commit `e46a8b7`, `cargo build --release -p camfacil --target x86_64-pc-windows-gnu`, medido com `/usr/bin/time -v`.

| Build | Tempo | Pico de memória |
|---|---|---|
| A frio (todas as dependências) | 4 min 36 s | 1,37 GB |
| Com cache, depois de trocar a cor em `web/tokens.css` (só o crate `camfacil` recompila) | 1 min 18 s | 1,10 GB |

- Executável: 6,5 MB. Pasta `target/`: 495 MB.
- Testado no Windows 11: `--version` responde, a dashboard abre, o `tokens.css` servido traz a cor nova e o processo fica em ~23 MB de memória parado.
- O Dokploy que já roda na KVM1 (~600 MB) cabe junto: sobram ~2,8 GB de RAM mais a swap, o dobro do pico medido.

**IA ligada no executável GNU** (Windows 11, câmera de demonstração, módulo Pessoas e veículos): o `.exe` gerado na KVM1 baixou e carregou o ONNX Runtime 1.20.1 (`ort` com `load-dynamic`) e o `yolo11n.onnx`, e detectou pessoas.

| Executável | Detector | Inferência | Memória com a IA ligada |
|---|---|---|---|
| GNU (mingw-w64, KVM1, receita laranja) | carregado, sem erro | 59 a 71 ms | 135 MB |
| MSVC (oficial, compilado no Windows) | carregado, sem erro | 71 ms | 127 MB |

Mesmo comportamento: o GNU serve para as versões geradas.

### Segurança da KVM1 (2026-11-17)

- SSH só com chave; root não entra por SSH (`/etc/ssh/sshd_config.d/01-camfacil.conf`). Administração pelo usuário `builder` com `sudo`.
- `ufw` ligado: entra só 22, 80 e 443.
- O painel do Dokploy (3000) é publicado pelo Docker por fora do `ufw`; uma regra na cadeia `DOCKER-USER` (serviço `camfacil-fw`) fecha a porta para a internet. O painel abre pelo túnel SSH (`ssh -L 3000:localhost:3000 kvm1`).
- Conferido de fora: 22, 80 e 443 abertas; 3000, 2377 e 7946 fechadas. Contêineres continuam saindo para a internet.
- **Base fixa.** Sempre a tag da versão oficial atual. Nunca código enviado pelo técnico.
- **Uso justo.** Um build na fila por técnico e até 3 builds por dia.

## Progresso

O técnico vê etapas, não log de compilador.

| Etapa | Como mede |
|---|---|
| Na fila | posição na fila e tempo estimado até começar |
| Preparando | validação da receita e geração dos arquivos |
| Compilando | porcentagem pelas unidades concluídas (`--message-format=json`) contra o total do último build com a mesma base |
| Montando o programa | etapa final (LTO), sem barra; mostra "leva cerca de N min" pela média dos últimos builds |
| Pronto | link de download |

Tempo estimado: média móvel dos últimos builds. A página atualiza por SSE ou consulta a cada 3 s. Fechar a página não cancela o build.

## Armazenamento e retenção

O armazenamento é gratuito. Por isso:

| O quê | Onde | Quanto tempo |
|---|---|---|
| Receitas, logos e ícones do técnico | tenant (Postgres com RLS + volume) | apagados após **30 dias sem login** |
| Executável gerado | volume, só o último de cada técnico | apagado após 30 dias sem login, ou quando gerar outro |
| **Registro de geração** | tabela própria, fora do tenant | **12 meses**, igual à auditoria do SDD-08 |

- **Aviso antes de apagar.** E-mail no 23º dia sem login: "sua versão e suas receitas serão apagadas em 7 dias; baixe a receita".
- **A receita é responsabilidade do técnico.** A tela "Pronto" e o e-mail pedem que ele guarde a receita (no Google Drive dele ou onde preferir).
- O download do executável é só para o dono, com link assinado de curta duração.

### Registro de geração

Uma linha por versão gerada: `tecnico_id`, nome e e-mail da conta no momento, data, versão base e commit, **hash da receita**, versão do termo aceito e IP.

- Não é apagado com o tenant, nem com a conta excluída a pedido, durante os 12 meses. Base: legítimo interesse e exercício regular de direito (LGPD, art. 7º, VI e IX). A política de privacidade passa a citar isso.
- **O hash vai dentro do executável.** Se aparecer uma versão problemática, o "Sobre" mostra o hash e o registro mostra quem gerou e o que escolheu.

## Responsabilidade

Antes de cada geração, o técnico aceita o **termo de responsabilidade da versão**:

- A versão gerada é dele: instalação, uso, suporte e LGPD com os clientes dele (termos, autorização e placa em `docs/legal/`).
- Não usa nome, logo ou identidade de fabricante (Lei 9.279/1996).
- Sabe que o CamFácil IA é ambiente de aprendizagem e validação, não sistema de segurança profissional.
- Sabe que tenant inativo há 30 dias é apagado e que guardar a receita é com ele.
- Sabe que a geração fica registrada com o hash.

**Marca de fabricante.** Nome e empresa são conferidos contra uma lista de marcas conhecidas (Intelbras, Hikvision, Dahua e outras). Se bater, o assistente recusa e explica. Logo não é conferido automaticamente; o termo cobre.

## Licença (AGPL-3.0)

Distribuir só o executável é recusado pelo Builder (`limites.md`, camada 2). Aqui a fonte correspondente é **a versão base mais a receita**, e as duas vão junto com o executável:

- **Receita embutida.** O "Sobre" mostra o aviso de versão modificada (seção 5(a)): "versão modificada do CamFácil IA por {autor}, em {data}". Mostra também o link do código da versão base (tag) e o botão **Exportar receita**, que salva o `.zip` a partir do próprio executável.
- Arquivo `LICENSE` e créditos continuam no programa.
- Assim a fonte não se perde, mesmo depois que o tenant for apagado.

## Atualizações

O `install.ps1` baixa a release oficial. Se rodasse por cima de uma versão personalizada, apagaria a marca do técnico.

- A versão personalizada se identifica como tal e **não se troca sozinha** pela oficial.
- Quando sai versão oficial nova, ela mostra: "saiu a versão X; gere a sua de novo com a mesma receita". O botão leva ao assistente com a receita importada.

## Segurança

- Receita validada duas vezes: no `cfia-api` (antes da fila) e no `build.rs` (antes de compilar).
- Imagens: só PNG e JPG, decodificadas e regravadas pelo servidor (some metadado e conteúdo escondido). SVG não é aceito.
- O `cfia-builder` não vê segredo nenhum, compila sem rede e só a partir da tag oficial.
- As regras da casa não mudam pela receita: PIN, `Host`/`Origin`, cofre, MCP, termos, modo LGPD e "nada vem ligado" ficam fora do alcance do assistente.

## Entrega em etapas

1. **Receita e build local** (feita). Formato da receita, validação no `cf-versao`, `build.rs` no `camfacil`, tabela das 80 classes, "Sobre" com receita e hash. Testável com `CFIA_RECEITA=… cargo build`.
2. **Compilador na VPS** (feita). Fila no `cfia-api`, contêiner `cfia-builder` com mingw-w64, cache e progresso. Falta ligar em produção: subir o `cfia-api` novo e o compilador com o token.
3. **Assistente no `cfia-api`.** As sete telas, importar receita, termo, download.
4. **Retenção.** Registro de geração, limpeza de 30 dias, e-mail no 23º dia, política de privacidade.
5. **Depois.** Linux, APK, "Salvar no meu Drive".

## Testes

- `cf-core`: validação da receita (classe fora de 0–79, inteligência desconhecida, texto longo, cor sem contraste, marca de fabricante), hash canônico estável.
- `camfacil`: sem receita = igual ao oficial; com receita, menus só com o escolhido; "Sobre" com aviso e hash; exportar receita devolve o mesmo hash.
- `cfia-api`: fila (um por técnico, limite diário), RLS das receitas, registro sobrevive à limpeza do tenant, limpeza só após 30 dias sem login.
- Fim a fim: receita → build no `cfia-builder` → executável abre no Windows com a cor, o logo e as classes escolhidas.

## Decisões em aberto

- **Classes perigosas de vender.** Faca (43), tesoura (76) e taco de beisebol (34) podem ser vendidas como "detecção de arma", com precisão baixa. Ficam no grupo "Outros" com o selo, ou saem da lista?
- **Serviço de e-mail.** O aviso do 23º dia entra na primeira versão. O `cfia-api` ainda não manda e-mail: escolher o serviço (SMTP próprio ou provedor).
- **"Salvar no meu Drive"** fica para depois da primeira versão.
