# ADR 0018 · Gerar sua versão: receita aplicada na compilação, numa VPS própria

**Situação:** aceita · 2026-11-17 · detalha o [SDD-12](../sdd/SDD-12-gerar-sua-versao.md) · complementa a [ADR 0002](0002-onnx-ort-e-licenca-agpl.md) (licença) e o [SDD-08](../sdd/SDD-08-contas-e-isolamento.md) (contas e isolamento)

## Contexto

O CamFácil IA Builder (skill de chat) exige fork, `git`, `cargo` e um assistente de código: muito para quem só quer o programa com a própria marca e só com as inteligências que vende. A interface (`web/`) vai dentro do binário (`include_bytes!`), então nome, logo e cor só mudam compilando. Compilar em servidor tem custo, risco (código de terceiros rodando) e uma obrigação: distribuir só o executável fere a AGPL-3.0, seções 5 e 6.

## Decisão

1. **Receita da versão** num JSON só (logo em base64), validada pelo crate novo e leve `cf-versao` (formato, hash SHA-256, contraste WCAG, marca de fabricante, logo PNG/JPG até 512 KB, tabela das 80 classes do COCO). O crate não depende do `cf-core`, porque o `build.rs` usa ele como dependência de compilação e o `cf-core` compila o SQLite.
2. **A receita entra pelo `build.rs` do `camfacil`** (`CFIA_RECEITA`) e mexe só nesse crate: troca de receita recompila só ele. Sem receita, o binário é o oficial, com um teste garantindo isso.
3. **A receita vai embutida no executável**, com o hash e o aviso de versão modificada no "Sobre" e o botão de exportar. Versão base (tag oficial) + receita = fonte correspondente da AGPL, mesmo depois que o espaço no site for apagado.
4. **O servidor decide** a versão base, o autor (nome da conta Google) e a data, e assina a receita. O técnico só escolhe identidade, cor, inteligências e detecções. O assistente só oferece opções permitidas: nada de biometria, nada que desligue PIN, termos ou modo LGPD.
5. **Compilação numa VPS separada** do site, em contêiner sem privilégio, com mingw-w64 (alvo `x86_64-pc-windows-gnu`). O `cargo-xwin` (MSVC) foi descartado porque baixa o SDK do Windows e exige aceitar a licença da Microsoft. O build oficial do GitHub continua MSVC.
6. **O compilador busca o trabalho** no `cfia-api` por HTTPS, com token próprio. A VPS não abre porta, não vê o banco nem os segredos do site, e só compila commits do ramo oficial (`git merge-base --is-ancestor`).
7. **Espaço gratuito com prazo:** versões apagadas após 30 dias sem entrar, aviso por e-mail (SMTP genérico) no 23º dia, só o último executável guardado. O **registro de geração** (conta, hash, base, termo, IP) fica 12 meses, sem chave estrangeira, e o técnico não lê nem altera.

## Consequências

- Binário oficial praticamente igual: a tabela das 80 classes soma poucos KB, e o detector passa a aceitar a lista da versão (`decodificar_classes`).
- Medido na KVM1 (1 vCPU, 4 GB): 5 min a frio, 1 min 35 s com cache, pico de 1,4 GB; executável de 6,5 MB com a IA funcionando igual ao MSVC.
- A imagem do compilador tem 4 GB e o cache cresce com o tempo; um build por vez, até 3 por técnico por dia.
- O executável gerado não tem assinatura digital: o Windows mostra o aviso do SmartScreen, e a tela de download explica isso.
- O site passa a tratar dado de quem gera versão (receita, registro) e a mandar e-mail: a política de privacidade cita os dois.
- Alternativas descartadas: configuração na hora de rodar (a interface é embutida no binário), GitHub Actions (token no navegador, minutos pagos e artefato público em repositório aberto) e compilar na mesma VPS do site (a etapa final da compilação disputaria CPU e memória com o site).
