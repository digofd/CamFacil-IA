# SDD-12 — Gerar sua versão

Status: **em produção** · Etapas 1 (receita e build local), 2 (fila e compilador), 3 (assistente no site) e 4 (retenção) feitas na branch `feat/gerar-sua-versao` · complementa o CamFácil IA Builder (`.claude/skills/camfacil-ia-builder/`), o [SDD-08](SDD-08-contas-e-isolamento.md) (contas, isolamento e auditoria), o [SDD-07](SDD-07-modo-estudio.md) (receitas e assistente do estúdio) e o [SDD-09](SDD-09-inteligencias-por-regra.md) (inteligências por regra) · criado em 2026-11-16

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

Página `/versao` do `cfia-api` (`web/versao.html`, `versao.js`, `versao.css`), com o esqueleto do estúdio (`estudio.css`): trilha de passos, uma tela só, sem rolagem do celular ao monitor. Sem login, mostra o que é e o botão "Entrar com Google".

1. **Identidade.** Nome da versão (ex.: "Vigia Silva"), nome da empresa, logo (PNG ou JPG, até 8 MB na entrada). O navegador **ajusta o logo sozinho**: acha o fundo pelos quatro cantos (transparente ou uma cor), recorta as bordas vazias, centraliza num quadrado de 256×256 com 10% de margem, preenche com o fundo da própria imagem (branco, se era transparente) e grava PNG (8 a 20 KB nos testes). O ícone da aba do navegador é o de sempre, com a cor de destaque da versão.
2. **Cor.** Uma das **8 cores fixas** (`cf_versao::receita::PALETA`): Verde (o oficial), Azul, Ciano, Amarelo, Laranja, Vermelho, Rosa e Roxo, todas com contraste de 4,5:1 ou mais sobre o fundo preto (um teste confere). Entra no lugar do verde (`--c-green`); a base continua preta (`DESIGN.md`). Não há seletor de cor livre: o servidor recusa qualquer cor fora da lista, e receita importada com cor antiga pede para escolher de novo. O assistente mostra uma tela de exemplo ao vivo.
3. **Inteligências.** As cinco do `cf_core::inteligencias::INTELIGENCIAS` (passagem, presença, densidade, permanência, calor), com o que cada uma faz e o peso (leve, médio, pesado). As não escolhidas somem dos menus.
4. **Detecções.** As classes do modelo padrão, em grupos (seção abaixo), com o aviso de foco.
5. **Revisão e termo.** Resumo das escolhas (uma linha só no celular), conferência no servidor (`POST /api/versoes/conferir`, que já recusa marca de fabricante), termo de responsabilidade e o botão **Gerar**, que só liga com a receita conferida e o "Li e aceito".
6. **Gerar.** Fila (posição), compilando (porcentagem), montando (sem barra) e pronta: baixar o executável e **baixar a receita**, com os avisos de guardar a receita (30 dias), do SmartScreen e de onde o programa mostra a versão. Em erro, mostra o motivo e volta à revisão. Fechar ou recarregar a página volta direto para o andamento.

Atalho no passo 1: **Importar receita** preenche todos os passos a partir de uma receita baixada antes (o servidor carimba base, autor e data de novo). O rascunho fica no navegador (`localStorage`) até gerar. O catálogo vem de `GET /api/versoes/catalogo`, para a tela não repetir a tabela das classes.

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
- **Na tela.** Um grupo por vez, em abas, com a contagem de marcadas. Grupo que não cabe vira páginas do tamanho do espaço (no "Outros", 3 páginas de 20 numa tela de 1024×768). Grupo inteiro de baixa precisão leva o selo uma vez só.
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
- Recusa: formato ou versão de formato errados, commit que não é hexadecimal, marca de fabricante no nome ou na empresa, logo que não é PNG/JPG de verdade (confere os primeiros bytes) ou passa de 512 KB, cor fora da lista de 8, inteligência desconhecida ou repetida, classe fora de 0–79 ou repetida, lista vazia, e hash que não confere (receita alterada à mão).
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
- **Tela:** nome da versão no título e no menu lateral, logo no lugar do ícone. No Início (Máquina), a linha "Versão" ganha o link do código-fonte (também no oficial, sem linha nova) e a versão gerada ganha só duas linhas: "Versão modificada" (autor, data e base) e "Receita" (hash e **Exportar receita**). Com quatro linhas, o bloco passava 36 px em 360×640.
- **API:** `GET /api/versao` (informações do "Sobre") e `GET /api/versao/receita.json` (exporta a receita; 404 no oficial).
- **Sem receita, o binário sai igual ao oficial.** O teste `versao::sem_receita_e_igual_ao_oficial` confere inteligências, classes, `tokens.css`, `icon.svg` e a ausência de logo e receita.
- **Ficha do Windows:** no alvo Windows, o `build.rs` embute (com `embed-resource`: `rc.exe` no MSVC, `windres` no mingw) a ficha de versão, um manifesto (`asInvoker`, Windows 10+) e o ícone. No oficial: "CamFácil IA", "Projeto CamFácil IA (software livre)" e o ícone de `scripts/icones/camfacil.ico`. Na versão gerada: produto com o nome da versão, empresa do técnico, "versão modificada do CamFácil IA", autor, data, base, hash da receita e o link do código; o ícone é o logo da receita (PNG dentro do `.ico`). Motivo: o primeiro `.exe` gerado em produção teve 4 de 71 no VirusTotal, todos por modelo ou reputação (arquivo novo, sem assinatura, sem ficha). A ficha ajuda; a solução de verdade é a assinatura digital (decisão em aberto).
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

### Conferido na Etapa 3 (2026-11-17)

No painel do navegador, pelo túnel SSH até um `cfia-api` de teste na KVM1 com o compilador ligado, com uma sessão de teste no lugar do login com Google:

- Sem sessão: a entrada com "Entrar com Google". Com sessão: passo 1.
- Nome, empresa e logo PNG; cor laranja com a prévia mostrando logo, nome e destaque (contraste 11,6:1); 3 inteligências; as 6 detecções de sempre mais cachorro.
- Revisão conferida pelo servidor; o botão Gerar só ligou depois do aceite.
- Fila → compilando → montando → pronta. Recarregar a página no meio do build voltou para o andamento.
- O `.exe` baixado pelo link da tela abriu no Windows com o nome, o logo, o laranja, as 3 inteligências, as 7 detecções e o autor da conta ("Emerson Teste").
- Zero rolagem nos seis passos em 360×640, 390×844, 768×1024, 1366×768 e 1920×1080 (e em 375×812), conferida pelo DOM; a Revisão passava 2 px em 360×640 e o termo ficou mais compacto em telas baixas. A ferramenta nova na página inicial também cabe. Duas correções saíram daí: o grupo "Outros" passava da altura (agora pagina) e a Revisão passava 220 px no celular (agora o resumo vira uma linha).

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

### Como está feito (Etapa 4)

- **Atividade:** coluna `tecnicos.ultimo_acesso`, atualizada no login e em qualquer chamada logada (no máximo uma escrita por hora).
- **Laço de hora em hora** (`crates/cfia-api/src/retencao.rs`, primeira rodada 1 minuto depois de subir):
  1. **Aviso:** técnico com versões e de 23 a 30 dias sem entrar recebe um e-mail, uma vez por período de inatividade (`tecnicos.aviso_retencao_em`). O assunto e o texto trazem os dias reais ("Faz 25 dias…", "apagada em 5 dias") e a data prevista. O envio fica na auditoria (`versao_aviso_retencao`).
  2. **Limpeza:** com mais de 30 dias sem entrar, apaga as versões e os executáveis do disco, menos a que está compilando.
  3. **Registro:** apaga o registro de geração com mais de 12 meses.
- **Só o último executável:** quando uma versão fica pronta, os executáveis anteriores do mesmo técnico saem do disco (`executavel_bytes` vira nulo; o download responde 410).
- **E-mail por SMTP genérico** (`lettre`, com `aws-lc-rs`, o mesmo provedor de criptografia do `reqwest`): `CFIA_SMTP_URL` (ex.: `smtps://usuario:senha@smtp.provedor.com:465`) e `CFIA_EMAIL_DE` (ex.: `CamFácil IA <nao-responda@cfia.ysafe.com.br>`). Serve qualquer provedor. Sem eles, a limpeza roda igual e o aviso conta como não enviado.
- **Política de privacidade** (`web/privacidade.html`): o que o "Gerar sua versão" guarda, a base legal do registro, o provedor de e-mail e os prazos.

### Registro de geração

Tabela `registro_versoes` (migração `0006_retencao.sql`), gravada na mesma transação do pedido: versão, técnico, nome e e-mail da conta no momento, hash da receita, versão e commit da base, termo aceito, IP e data.

- **Sem chave estrangeira:** sobrevive à limpeza do tenant e à conta excluída a pedido. Base: legítimo interesse e exercício regular de direito (LGPD, art. 7º, VI e IX), citados na política de privacidade.
- **RLS:** o técnico só insere o próprio registro; não lê, não altera, não apaga. Ler e expirar é do sistema. O papel da aplicação não tem `UPDATE` na tabela.
- **O hash vai dentro do executável.** Se aparecer uma versão problemática, o "Sobre" mostra o hash e o registro mostra quem gerou e o que escolheu.

### Conferido na Etapa 4 (2026-11-17)

- Testes de integração contra Postgres na KVM1 (28 no total): registro gravado no pedido e invisível ao técnico, aviso no 25º dia sem apagar, acesso atualizando `ultimo_acesso`, limpeza no 31º dia (versão e executável), registro sobrevivendo à limpeza e expirando aos 13 meses, só o último executável no disco.
- **E-mail de verdade** com o Mailpit (SMTP de teste) e um técnico com 25 dias sem entrar: chegou "Sua versão do CamFácil IA será apagada em 5 dias", com "Faz 25 dias…" e a data prevista. Na rodada seguinte, nenhum e-mail novo. Com 31 dias, a versão foi apagada.
- Achado no teste: o texto dizia "23 dias" e "7 dias" fixos; agora usa os dias reais.

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
3. **Assistente no `cfia-api`** (feita). Página `/versao`, importar receita, termo (versão `2026-11-17`, exigida no pedido e gravada na auditoria), download e link na seção de ferramentas do site.
4. **Retenção** (feita). Registro de geração, limpeza de 30 dias, e-mail no 23º dia por SMTP genérico, só o último executável, política de privacidade.
5. **Depois.** Linux, APK, "Salvar no meu Drive".

## Testes

- `cf-core`: validação da receita (classe fora de 0–79, inteligência desconhecida, texto longo, cor sem contraste, marca de fabricante), hash canônico estável.
- `camfacil`: sem receita = igual ao oficial; com receita, menus só com o escolhido; "Sobre" com aviso e hash; exportar receita devolve o mesmo hash.
- `cfia-api`: fila (um por técnico, limite diário), RLS das receitas, registro sobrevive à limpeza do tenant, limpeza só após 30 dias sem login.
- Fim a fim: receita → build no `cfia-builder` → executável abre no Windows com a cor, o logo e as classes escolhidas.

## Decisões em aberto

- **Assinatura digital (Authenticode)** dos executáveis, oficiais e gerados: resolve o SmartScreen e faz a reputação ser da assinatura, não de cada arquivo (cada versão gerada é um arquivo novo). Opções: serviço de assinatura da Microsoft no Azure (mensal, assina do compilador) ou certificado tradicional (anual). Custo e exigência de identidade a confirmar.

- **Classes perigosas de vender.** Faca (43), tesoura (76) e taco de beisebol (34) podem ser vendidas como "detecção de arma", com precisão baixa. Ficam no grupo "Outros" com o selo, ou saem da lista?
- **Provedor de e-mail.** O código fala SMTP com qualquer provedor; falta escolher qual (e configurar SPF/DKIM do domínio) e preencher `CFIA_SMTP_URL` e `CFIA_EMAIL_DE` no servidor.
- **"Salvar no meu Drive"** fica para depois da primeira versão.
