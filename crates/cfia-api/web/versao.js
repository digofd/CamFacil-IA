// CamFácil IA — "Gerar sua versão" (SDD-12). Assistente em seis passos numa
// tela só: identidade, cor, inteligências, detecções, revisão com o termo e a
// geração (fila → compilando → pronta). JS puro, sem nada inline (CSP).
"use strict";
(() => {
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];
  const el = (tag, attrs = {}, ...kids) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) {
      if (k === "class") e.className = v;
      else if (k.startsWith("on") && typeof v === "function") e[k] = v;
      else if (v === true) e.setAttribute(k, "");
      else if (v !== false && v != null) e.setAttribute(k, v);
    }
    e.append(...kids.filter((k) => k != null));
    return e;
  };

  async function api(caminho, metodo = "GET", corpo) {
    const opt = { method: metodo, headers: {}, credentials: "same-origin" };
    if (corpo !== undefined) { opt.headers["Content-Type"] = "application/json"; opt.body = JSON.stringify(corpo); }
    const r = await fetch(caminho, opt);
    const dados = await r.json().catch(() => ({}));
    if (!r.ok) { const e = new Error(dados.erro || r.statusText); e.status = r.status; throw e; }
    return dados;
  }
  const guardar = (v) => { try { localStorage.setItem("cf-versao-rascunho", JSON.stringify(v)); } catch (_) { /* sem armazenamento */ } };
  const ler = () => { try { return JSON.parse(localStorage.getItem("cf-versao-rascunho") || "null"); } catch (_) { return null; } };
  const maiuscula = (s) => s.charAt(0).toUpperCase() + s.slice(1);
  const msg = (id, texto, tipo = "") => { const m = $(id); m.className = "msg " + tipo; m.textContent = texto || ""; };

  // ------------------------------------------------------------ catálogo
  const INTEL = {
    passagem: ["Passagem", "Conta quem cruza a linha, com o sentido.", "leve"],
    presenca: ["Presença", "Avisa quando alguém entra e quando sai da área.", "leve"],
    densidade: ["Densidade", "Avisa quando passa de N dentro da área ao mesmo tempo.", "médio"],
    permanencia: ["Permanência", "Avisa quando fica parado na área por N segundos.", "médio"],
    calor: ["Mapa de calor", "Mostra por onde mais passa, só da área e do que foi marcado.", "pesado"],
  };
  const GRUPOS = [
    ["pessoas_veiculos", "Pessoas e veículos"],
    ["outros_veiculos", "Outros veículos"],
    ["animais", "Animais"],
    ["objetos_pessoas", "Objetos de pessoas"],
    ["rua", "Rua"],
    ["outros", "Outros"],
  ];
  // Cores fixas: vêm do servidor (cf_versao::receita::PALETA), que só aceita estas.
  const paleta = () => (CAT && CAT.paleta) || [{ nome: "Azul", cor: "#5aa9ff" }];
  const naPaleta = (hex) => paleta().some((p) => p.cor === String(hex).toLowerCase());
  let CAT = null;

  // ------------------------------------------------------------ estado
  const S = {
    passo: 0,
    nome: "", empresa: "", logo: null,
    destaque: "#5aa9ff",
    intel: new Set(["passagem", "presenca", "permanencia"]),
    classes: new Set(),
    grupo: "pessoas_veiculos",
    pagina: 0,
    conferida: false,
    versao: null, // id da versão na fila
  };
  function rascunho() {
    guardar({ nome: S.nome, empresa: S.empresa, logo: S.logo, destaque: S.destaque, intel: [...S.intel], classes: [...S.classes] });
  }
  function receita() {
    const r = {
      formato: "cfia-versao", versao_formato: 1,
      identidade: { nome: S.nome.trim(), empresa: S.empresa.trim() },
      cores: { destaque: S.destaque },
      inteligencias: Object.keys(INTEL).filter((i) => S.intel.has(i)),
      classes: [...S.classes].sort((a, b) => a - b),
    };
    if (S.logo) r.identidade.logo = S.logo;
    return r;
  }

  // ------------------------------------------------------------ navegação
  const PASSOS = ["identidade", "cor", "inteligencias", "deteccoes", "revisao", "gerar-passo"];
  const botoes = $$(".est-passos [data-ir]");
  function mostrar(modo) {
    $("#entrada").hidden = modo !== "entrada";
    $(".est-trilha").hidden = modo === "entrada";
    $(".est-nav").hidden = modo === "entrada";
    $("#versao").classList.toggle("em-casa", modo === "entrada");
    if (modo === "entrada") PASSOS.forEach((p) => { $("#" + p).hidden = true; });
  }
  function ir(n, focar) {
    if (n === 5 && !S.versao) n = 4;
    S.passo = Math.max(0, Math.min(PASSOS.length - 1, n));
    mostrar("assistente");
    PASSOS.forEach((p, i) => { $("#" + p).hidden = i !== S.passo; });
    botoes.forEach((b, i) => {
      b.setAttribute("aria-selected", String(i === S.passo));
      b.tabIndex = i === S.passo ? 0 : -1;
      b.classList.toggle("feito", i < S.passo);
      b.disabled = i === 5 && !S.versao;
    });
    const naGeracao = S.passo === 5;
    $(".est-nav").hidden = naGeracao;
    $("#pos").textContent = `${S.passo + 1} de 5`;
    $("#ant").disabled = S.passo === 0;
    $("#prox").disabled = S.passo >= 4;
    history.replaceState(null, "", "#" + PASSOS[S.passo].replace("-passo", ""));
    if (PASSOS[S.passo] === "cor") desenharCor();
    if (PASSOS[S.passo] === "deteccoes") desenharClasses();
    if (PASSOS[S.passo] === "revisao") abrirRevisao();
    if (focar) $("#" + PASSOS[S.passo] + " h1").focus({ preventScroll: true });
  }
  // Confere o passo antes de sair dele para a frente.
  function podeSair() {
    const id = PASSOS[S.passo];
    if (id === "identidade") {
      const n = S.nome.trim();
      if (n.length < 2) { msg("#identidade-msg", "Dê um nome à versão (pelo menos 2 letras).", "err"); return false; }
      if (/[<>&"\\]/.test(S.nome + S.empresa)) { msg("#identidade-msg", 'Não use os caracteres < > & " \\ no nome.', "err"); return false; }
    }
    if (id === "cor" && !naPaleta(S.destaque)) { msg("#cor-msg", "Escolha uma das cores da lista.", "err"); return false; }
    if (id === "inteligencias" && !S.intel.size) { msg("#intel-msg", "Escolha pelo menos uma inteligência.", "err"); return false; }
    if (id === "deteccoes" && !S.classes.size) { $("#det-conta").textContent = "Escolha pelo menos uma detecção."; return false; }
    return true;
  }
  function avancar(n) {
    for (let i = S.passo; i < Math.min(n, 5); i++) {
      ir(i);
      if (!podeSair()) return;
    }
    ir(n, true);
  }
  botoes.forEach((b, i) => {
    b.onclick = () => (i > S.passo ? avancar(i) : ir(i, true));
    b.onkeydown = (ev) => {
      const d = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[ev.key];
      if (d) { ev.preventDefault(); (d > 0 ? avancar : ir)(S.passo + d); botoes[S.passo].focus(); }
    };
  });
  $$(".est-passo h1").forEach((h) => { h.tabIndex = -1; });
  $("#ant").onclick = () => ir(S.passo - 1, true);
  $("#prox").onclick = () => avancar(S.passo + 1);

  // ------------------------------------------------------------ 1. identidade
  const fId = $("#form-identidade");
  fId.nome.oninput = () => { S.nome = fId.nome.value; S.conferida = false; msg("#identidade-msg"); rascunho(); };
  fId.empresa.oninput = () => { S.empresa = fId.empresa.value; S.conferida = false; rascunho(); };
  fId.onsubmit = (ev) => { ev.preventDefault(); avancar(1); };

  function desenharLogo() {
    const caixa = $("#logo-previa");
    caixa.replaceChildren(S.logo ? el("img", { src: S.logo, alt: "Logo escolhido" }) : el("span", { class: "tiny" }, "Sem logo"));
    $("#logo-tirar").hidden = !S.logo;
    $("#pv-logo").replaceChildren(S.logo ? el("img", { src: S.logo, alt: "" }) : "");
    $("#pv-logo").classList.toggle("vazio", !S.logo);
  }
  // Logo: o navegador recorta as bordas vazias, centraliza num quadrado com
  // margem e reduz para 256×256. Qualquer imagem fica igual no menu do programa.
  const LOGO_LADO = 256;
  const LOGO_MARGEM = 0.1;
  const ENTRADA_MAX = 8 * 1024 * 1024;
  async function adaptarLogo(arquivo) {
    const img = await createImageBitmap(arquivo);
    // Analisa numa cópia de até 1024 px (rápido mesmo com foto grande).
    const k = Math.min(1, 1024 / Math.max(img.width, img.height));
    const w = Math.max(1, Math.round(img.width * k)), h = Math.max(1, Math.round(img.height * k));
    const a = document.createElement("canvas");
    a.width = w; a.height = h;
    const ga = a.getContext("2d", { willReadFrequently: true });
    ga.drawImage(img, 0, 0, w, h);
    const px = ga.getImageData(0, 0, w, h).data;
    const em = (x, y) => (y * w + x) * 4;
    // Fundo: o que aparece nos quatro cantos (transparente ou uma cor).
    const cantos = [[0, 0], [w - 1, 0], [0, h - 1], [w - 1, h - 1]].map(([x, y]) => em(x, y));
    const transparente = cantos.filter((i) => px[i + 3] < 32).length >= 3;
    const media = (c) => Math.round(cantos.reduce((t, i) => t + px[i + c], 0) / 4);
    const fundo = [media(0), media(1), media(2)];
    const ehConteudo = (i) => px[i + 3] > 24 && (transparente
      || Math.abs(px[i] - fundo[0]) + Math.abs(px[i + 1] - fundo[1]) + Math.abs(px[i + 2] - fundo[2]) > 48);
    let x0 = w, y0 = h, x1 = -1, y1 = -1;
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
      if (ehConteudo(em(x, y))) { if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; }
    }
    if (x1 < 0) { x0 = 0; y0 = 0; x1 = w - 1; y1 = h - 1; } // só fundo: usa tudo
    const recorte = { x: x0 / k, y: y0 / k, w: (x1 - x0 + 1) / k, h: (y1 - y0 + 1) / k };
    // Quadrado com o fundo da própria imagem (ou branco, se era transparente).
    const q = document.createElement("canvas");
    q.width = q.height = LOGO_LADO;
    const g = q.getContext("2d");
    g.fillStyle = transparente ? "#ffffff" : `rgb(${fundo.join(",")})`;
    g.fillRect(0, 0, LOGO_LADO, LOGO_LADO);
    const util = LOGO_LADO * (1 - 2 * LOGO_MARGEM);
    const esc = Math.min(util / recorte.w, util / recorte.h);
    const dw = recorte.w * esc, dh = recorte.h * esc;
    g.imageSmoothingEnabled = true;
    g.imageSmoothingQuality = "high";
    g.drawImage(img, recorte.x, recorte.y, recorte.w, recorte.h, (LOGO_LADO - dw) / 2, (LOGO_LADO - dh) / 2, dw, dh);
    img.close();
    let url = q.toDataURL("image/png");
    if (url.length > (CAT ? CAT.logo_max : 524288)) url = q.toDataURL("image/jpeg", 0.9);
    return { url, cortou: x0 > 0 || y0 > 0 || x1 < w - 1 || y1 < h - 1 };
  }
  $("#logo").onchange = async (ev) => {
    const f = ev.target.files[0];
    ev.target.value = "";
    if (!f) return;
    if (!["image/png", "image/jpeg"].includes(f.type)) { msg("#identidade-msg", "O logo precisa ser PNG ou JPG.", "err"); return; }
    if (f.size > ENTRADA_MAX) { msg("#identidade-msg", "Essa imagem passa de 8 MB. Escolha uma menor.", "err"); return; }
    msg("#identidade-msg", "Ajustando o logo…");
    try {
      const { url, cortou } = await adaptarLogo(f);
      S.logo = url; S.conferida = false; desenharLogo(); rascunho();
      msg("#identidade-msg", cortou ? "Logo ajustado: tiramos as bordas vazias e centralizamos no quadrado." : "Logo ajustado e centralizado no quadrado.", "ok");
    } catch (_) {
      msg("#identidade-msg", "Não deu para ler essa imagem. Tente outro arquivo PNG ou JPG.", "err");
    }
  };
  $("#logo-tirar").onclick = () => { S.logo = null; S.conferida = false; desenharLogo(); rascunho(); };

  $("#importar").onchange = async (ev) => {
    const f = ev.target.files[0];
    ev.target.value = "";
    if (!f) return;
    try {
      const r = JSON.parse(await f.text());
      if (r.formato !== "cfia-versao") throw new Error("Esse arquivo não é uma receita de versão do CamFácil IA.");
      const id = r.identidade || {};
      S.nome = String(id.nome || ""); S.empresa = String(id.empresa || "");
      S.logo = typeof id.logo === "string" ? id.logo : null;
      let corFora = false;
      if (r.cores && /^#[0-9a-f]{6}$/i.test(r.cores.destaque)) {
        if (naPaleta(r.cores.destaque)) S.destaque = r.cores.destaque.toLowerCase();
        else corFora = true;
      }
      if (Array.isArray(r.inteligencias)) S.intel = new Set(r.inteligencias.filter((i) => INTEL[i]));
      if (Array.isArray(r.classes)) S.classes = new Set(r.classes.filter((c) => Number.isInteger(c) && c >= 0 && c < 80));
      S.conferida = false;
      preencher(); rascunho();
      msg("#identidade-msg", `Receita "${S.nome}" carregada.${corFora ? " A cor dela não está mais na lista: escolha outra no passo 2." : ""} Confira os passos e gere de novo.`, "ok");
    } catch (e) {
      msg("#identidade-msg", e instanceof SyntaxError ? "Esse arquivo não é um JSON válido." : e.message, "err");
    }
  };

  // ------------------------------------------------------------ 2. cor
  function rgb(hex) { const n = parseInt(hex.slice(1), 16); return [(n >> 16) & 255, (n >> 8) & 255, n & 255]; }
  function contraste(hex) { // contra o preto, fórmula do WCAG 2
    const l = rgb(hex).map((c) => { const s = c / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; });
    const lum = 0.2126 * l[0] + 0.7152 * l[1] + 0.0722 * l[2];
    return (lum + 0.05) / 0.05;
  }
  function escolherCor(hex) {
    S.destaque = hex.toLowerCase(); S.conferida = false;
    desenharCor(); rascunho();
  }
  function desenharCor() {
    if (!naPaleta(S.destaque)) S.destaque = paleta()[1] ? paleta()[1].cor : paleta()[0].cor;
    $("#paleta").replaceChildren(...paleta().map(({ nome, cor }) => {
      const amostra = el("span", { class: "ver-cor-amostra", "aria-hidden": "true" });
      amostra.style.setProperty("--amostra", cor);
      const b = el("button", { type: "button", class: "ver-cor", role: "radio", "aria-checked": String(cor === S.destaque) }, amostra, el("span", {}, nome));
      b.onclick = () => escolherCor(cor);
      return b;
    }));
    $("#previa").style.setProperty("--pv", S.destaque);
    $("#pv-nome").textContent = S.nome.trim() || "Sua versão";
    const p = paleta().find((x) => x.cor === S.destaque);
    msg("#cor-msg", `${p ? p.nome : ""} · contraste ${contraste(S.destaque).toFixed(1).replace(".", ",")}:1 sobre o preto.`, "ok");
  }

  // ------------------------------------------------------------ 3. inteligências
  function desenharIntel() {
    $("#lista-intel").replaceChildren(...Object.entries(INTEL).map(([id, [nome, desc, peso]]) => {
      const c = el("input", { type: "checkbox", value: id, checked: S.intel.has(id) });
      c.onchange = () => { c.checked ? S.intel.add(id) : S.intel.delete(id); S.conferida = false; msg("#intel-msg"); rascunho(); };
      return el("label", { class: "ver-intel-item" }, c,
        el("span", {}, el("strong", {}, nome), el("span", { class: "tiny" }, ` · ${peso}`), el("span", { class: "ver-desc" }, desc)));
    }));
  }

  // ------------------------------------------------------------ 4. detecções
  function desenharClasses() {
    if (!CAT) return;
    $("#grupos").replaceChildren(...GRUPOS.map(([g, nome]) => {
      const n = CAT.classes.filter((c) => c.grupo === g && S.classes.has(c.indice)).length;
      const b = el("button", { type: "button", role: "tab", "aria-selected": String(g === S.grupo) }, nome, n ? el("span", { class: "ver-n" }, ` ${n}`) : null);
      b.onclick = () => { S.grupo = g; S.pagina = 0; desenharClasses(); };
      return b;
    }));
    // Grupo inteiro de baixa precisão: o selo vai uma vez só, no aviso.
    const doGrupo = CAT.classes.filter((c) => c.grupo === S.grupo);
    const todasBaixas = doGrupo.every((c) => c.baixa_precisao);
    $("#grupo-baixa").hidden = !todasBaixas;
    const chips = doGrupo.map((c) => {
      const k = el("input", { type: "checkbox", value: c.indice, checked: S.classes.has(c.indice) });
      k.onchange = () => { k.checked ? S.classes.add(c.indice) : S.classes.delete(c.indice); S.conferida = false; rascunho(); contar(); desenharGrupoN(); };
      return el("label", { class: "ver-classe" }, k, el("span", {}, maiuscula(c.nome)),
        c.baixa_precisao && !todasBaixas ? el("span", { class: "ver-selo", title: "Baixa precisão: use para estudo" }, "baixa precisão") : null);
    });
    paginar(chips);
    contar();
  }
  function desenharGrupoN() {
    $$("#grupos button").forEach((b, i) => {
      const g = GRUPOS[i][0];
      const n = CAT.classes.filter((c) => c.grupo === g && S.classes.has(c.indice)).length;
      b.replaceChildren(GRUPOS[i][1], n ? el("span", { class: "ver-n" }, ` ${n}`) : "");
    });
  }
  // Zero rolagem: o grupo que não cabe vira páginas do tamanho do espaço.
  function paginar(chips) {
    const grade = $("#classes");
    grade.replaceChildren(...chips);
    $("#pag").hidden = true;
    if (!chips.length || grade.scrollHeight <= grade.clientHeight + 1) return;
    $("#pag").hidden = false; // a barra de páginas ocupa espaço: mede depois dela
    const colunas = getComputedStyle(grade).gridTemplateColumns.split(" ").length;
    const linha = chips[0].offsetHeight + 6;
    const porPagina = Math.max(colunas, colunas * Math.floor((grade.clientHeight + 6) / linha));
    const total = Math.ceil(chips.length / porPagina);
    S.pagina = Math.min(S.pagina, total - 1);
    grade.replaceChildren(...chips.slice(S.pagina * porPagina, (S.pagina + 1) * porPagina));
    $("#pag-pos").textContent = `${S.pagina + 1} de ${total}`;
    $("#pag-ant").disabled = S.pagina === 0;
    $("#pag-prox").disabled = S.pagina >= total - 1;
  }
  $("#pag-ant").onclick = () => { S.pagina--; desenharClasses(); };
  $("#pag-prox").onclick = () => { S.pagina++; desenharClasses(); };
  addEventListener("resize", () => { if (PASSOS[S.passo] === "deteccoes") desenharClasses(); });

  function contar() {
    const n = S.classes.size;
    $("#det-conta").textContent = n === 1 ? "1 detecção escolhida" : `${n} detecções escolhidas`;
  }

  // ------------------------------------------------------------ 5. revisão
  async function abrirRevisao() {
    const nomes = (ids) => ids.join(", ") || "—";
    const r = receita();
    const linhas = [
      ["Nome", r.identidade.nome],
      ["Empresa", r.identidade.empresa || "—"],
      ["Logo", S.logo ? "sim" : "sem logo"],
      ["Cor", S.destaque],
      ["Inteligências", nomes(r.inteligencias.map((i) => INTEL[i][0]))],
      ["Detecções", nomes(r.classes.map((i) => CAT.classes[i].nome))],
      ["Base", `CamFácil IA ${CAT.versao_base}`],
    ];
    $("#resumo").replaceChildren(...linhas.flatMap(([k, v]) => {
      const dd = el("dd", {}, v);
      if (k === "Cor") { const a = el("span", { class: "ver-amostra" }); a.style.setProperty("--amostra", S.destaque); dd.prepend(a, " "); }
      return [el("dt", {}, k), dd];
    }));
    const cor = el("span", { class: "ver-amostra" });
    cor.style.setProperty("--amostra", S.destaque);
    $("#resumo-curto").replaceChildren(el("strong", {}, r.identidade.nome), " · ", cor, " · ",
      `${r.inteligencias.length} inteligência${r.inteligencias.length === 1 ? "" : "s"} · `,
      `${r.classes.length} detecç${r.classes.length === 1 ? "ão" : "ões"}`);
    atualizarGerar();
    if (S.conferida) return;
    msg("#revisao-msg", "Conferindo a receita…");
    try {
      await api("/api/versoes/conferir", "POST", { receita: r });
      S.conferida = true;
      msg("#revisao-msg", "Receita conferida.", "ok");
    } catch (e) {
      S.conferida = false;
      msg("#revisao-msg", `Não dá para gerar assim: ${e.message}.`, "err");
    }
    atualizarGerar();
  }
  function atualizarGerar() { $("#gerar").disabled = !(S.conferida && $("#aceito").checked); }
  $("#aceito").onchange = atualizarGerar;

  $("#gerar").onclick = async () => {
    const b = $("#gerar");
    b.disabled = true;
    msg("#revisao-msg", "Mandando para a fila…");
    try {
      const v = await api("/api/versoes", "POST", { receita: receita(), termo: CAT.termo });
      S.versao = v.id;
      ir(5, true);
      acompanhar();
    } catch (e) {
      msg("#revisao-msg", e.status === 401 ? "Sua sessão terminou. Entre de novo." : e.message, "err");
      atualizarGerar();
    }
  };

  // ------------------------------------------------------------ 6. gerar
  let tAcomp = 0;
  const ETAPA = {
    preparando: "Preparando o código oficial",
    compilando: "Compilando",
    montando: "Montando o programa (essa parte não tem barra; falta pouco)",
  };
  async function acompanhar() {
    clearTimeout(tAcomp);
    if (!S.versao) return;
    let v;
    try { v = await api(`/api/versoes/${S.versao}`); } catch (e) {
      $("#estimativa").textContent = "Sem resposta do servidor; tentando de novo…";
      tAcomp = setTimeout(acompanhar, 5000);
      return;
    }
    const barra = $("#barra");
    barra.style.setProperty("--p", v.percentual);
    barra.setAttribute("aria-valuenow", v.percentual);
    barra.classList.toggle("parada", v.estado === "fila");
    $("#pronta").hidden = v.estado !== "pronta";
    $("#erro-bloco").hidden = v.estado !== "erro";
    $("#acoes-versao").hidden = !["pronta", "erro"].includes(v.estado);
    $("#t-gerar").textContent = v.nome;
    const min = v.tempo_medio_s ? Math.max(1, Math.round(v.tempo_medio_s / 60)) : null;
    if (v.estado === "fila") {
      $("#etapa").textContent = v.posicao > 1 ? `Na fila: ${v.posicao}º lugar` : "Na fila: você é o próximo";
      $("#estimativa").textContent = min ? `Cada versão leva cerca de ${min} min.` : "";
    } else if (v.estado === "compilando") {
      const nome = ETAPA[v.etapa] || "Compilando";
      $("#etapa").textContent = v.etapa === "compilando" ? `${nome}: ${v.percentual}%` : nome;
      $("#estimativa").textContent = min ? `Costuma levar cerca de ${min} min.` : "A primeira compilação do dia demora mais.";
    } else if (v.estado === "pronta") {
      $("#etapa").textContent = "Pronta";
      $("#gerar-sub").textContent = "Baixe o programa e guarde a receita.";
      $("#estimativa").textContent = v.executavel_bytes ? `${(v.executavel_bytes / 1048576).toFixed(1)} MB · hash da receita ${v.hash.slice(0, 12)}` : "";
      $("#baixar-exe").href = `/api/versoes/${v.id}/executavel`;
      $("#baixar-receita").href = `/api/versoes/${v.id}/receita.json`;
      $("#contador").textContent = v.downloads === 1 ? "Baixado 1 vez" : `Baixado ${v.downloads} vezes`;
      $("#link-publico").textContent = v.link ? location.origin + v.link : "—";
      // O contador sobe quando alguém baixa: confere de novo de tempos em tempos.
      tAcomp = setTimeout(acompanhar, 30000);
      return;
    } else {
      $("#etapa").textContent = "Não deu certo";
      $("#gerar-sub").textContent = "Nada foi cobrado de você: ajuste e gere de novo.";
      msg("#erro-msg", v.erro || "A compilação falhou.", "err");
      $("#estimativa").textContent = "";
      return;
    }
    tAcomp = setTimeout(acompanhar, 3000);
  }
  $("#tentar").onclick = () => { S.versao = null; S.conferida = false; ir(4, true); };
  $("#nova").onclick = () => { clearTimeout(tAcomp); S.versao = null; S.conferida = false; $("#aceito").checked = false; ir(0, true); };
  $("#baixar-exe").addEventListener("click", () => { setTimeout(acompanhar, 2500); });
  $("#copiar-link").onclick = async () => {
    const t = $("#link-publico").textContent;
    try { await navigator.clipboard.writeText(t); $("#copiar-link").textContent = "Copiado"; }
    catch (_) { getSelection().selectAllChildren($("#link-publico")); $("#copiar-link").textContent = "Selecionado"; }
    setTimeout(() => { $("#copiar-link").textContent = "Copiar link"; }, 2000);
  };
  // Limpar: apaga versões, receitas, executáveis e o link público (o registro de quem gerou fica).
  $("#limpar").onclick = async () => {
    if (!confirm("Limpar a versão? O programa, a receita e o link público serão apagados daqui e o link para de funcionar. Baixe a receita antes, se quiser gerar de novo depois.")) return;
    clearTimeout(tAcomp);
    try {
      await api("/api/versoes", "DELETE");
      try { localStorage.removeItem("cf-versao-rascunho"); } catch (_) { /* sem armazenamento */ }
      Object.assign(S, { versao: null, conferida: false, nome: "", empresa: "", logo: null, destaque: paleta()[1] ? paleta()[1].cor : S.destaque });
      S.classes = new Set(CAT.padrao);
      $("#aceito").checked = false;
      preencher();
      ir(0, true);
      msg("#identidade-msg", "Versão limpa. Monte a próxima quando quiser.", "ok");
    } catch (e) {
      msg("#limpar-msg", e.message, "err");
    }
  };

  // ------------------------------------------------------------ início
  function preencher() {
    fId.nome.value = S.nome; fId.empresa.value = S.empresa;
    desenharLogo(); desenharIntel();
  }
  async function iniciar() {
    let logado = false;
    try {
      [CAT, logado] = await Promise.all([
        api("/api/versoes/catalogo"),
        api("/api/eu").then(() => true, () => false),
      ]);
    } catch (_) {
      msg("#entrada-msg", "Não foi possível falar com o servidor. Tente de novo em instantes.", "err");
      return;
    }
    S.classes = new Set(CAT.padrao);
    if (!CAT.ligado) { msg("#entrada-msg", "Gerar sua versão ainda não está ligado neste servidor."); return; }
    if (!logado) {
      msg("#entrada-msg", "Entre com a sua conta do Google para gerar a sua versão.");
      $("#entrar").hidden = false;
      return;
    }
    const r = ler();
    if (r) {
      Object.assign(S, { nome: r.nome || "", empresa: r.empresa || "", logo: r.logo || null, destaque: r.destaque || S.destaque });
      if (Array.isArray(r.intel)) S.intel = new Set(r.intel.filter((i) => INTEL[i]));
      if (Array.isArray(r.classes) && r.classes.length) S.classes = new Set(r.classes);
    }
    preencher();
    // Versão ainda na fila ou compilando: volta direto para o andamento.
    try {
      const { versoes } = await api("/api/versoes");
      const ativa = versoes.find((v) => v.estado === "fila" || v.estado === "compilando");
      const recente = versoes[0];
      // A versão pronta fica: a página abre nela (download, contador, link).
      if (ativa || (recente && recente.estado === "pronta")) {
        S.versao = (ativa || recente).id;
        ir(5);
        acompanhar();
        return;
      }
    } catch (_) { /* segue para o assistente */ }
    const pedido = PASSOS.indexOf(location.hash.slice(1));
    ir(pedido >= 0 && pedido < 5 ? pedido : 0);
  }
  iniciar();
})();
