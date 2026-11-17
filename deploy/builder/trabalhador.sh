#!/usr/bin/env bash
# CamFácil IA · compilador do "Gerar sua versão" (SDD-12).
#
# Pergunta ao cfia-api se há receita na fila, compila o executável do Windows
# com ela e devolve. Só sai conexão daqui: a máquina não abre porta nenhuma.
#
# Ambiente:
#   CFIA_API            ex.: https://cfia.ysafe.com.br
#   CFIA_BUILDER_TOKEN  o mesmo token configurado no cfia-api
#   CFIA_REPO           repositório oficial (padrão: GitHub do projeto)
#   CFIA_REF            ramo de onde a base precisa ter saído (padrão: main)
#   CFIA_ESPERA         segundos entre consultas com a fila vazia (padrão: 10)
set -uo pipefail

: "${CFIA_API:?defina CFIA_API}"
: "${CFIA_BUILDER_TOKEN:?defina CFIA_BUILDER_TOKEN}"
REPO="${CFIA_REPO:-https://github.com/emersoncassis/CamFacil-IA.git}"
REF="${CFIA_REF:-main}"
ESPERA="${CFIA_ESPERA:-10}"
SRC=/cache/src
TRABALHO=/trabalho
ALVO=x86_64-pc-windows-gnu
export CARGO_TARGET_DIR=/cache/target
export CARGO_HOME=/cache/cargo

log() { echo "[$(date -u +%H:%M:%S)] $*"; }

api() { # api MÉTODO ROTA [argumentos do curl...]
  local metodo="$1" rota="$2"
  shift 2
  curl -sS --max-time 120 -X "$metodo" -H "Authorization: Bearer $CFIA_BUILDER_TOKEN" \
    "$@" "$CFIA_API$rota"
}

progresso() { # progresso ID ETAPA PERCENTUAL
  api POST "/api/builder/$1/progresso" -H 'Content-Type: application/json' \
    -d "$(jq -cn --arg e "$2" --argjson p "$3" '{etapa: $e, percentual: $p}')" >/dev/null || true
}

falhar() { # falhar ID MENSAGEM
  log "build $1 falhou: $2"
  api POST "/api/builder/$1/erro" -H 'Content-Type: application/json' \
    -d "$(jq -cn --arg m "$2" '{mensagem: $m}')" >/dev/null || true
}

preparar_fonte() { # preparar_fonte COMMIT
  if [ ! -d "$SRC/.git" ]; then
    git clone --quiet "$REPO" "$SRC" || return 1
  fi
  git -C "$SRC" fetch --quiet origin "$REF" || return 1
  # Só compila código que saiu do ramo oficial: nunca um commit qualquer.
  git -C "$SRC" cat-file -e "$1^{commit}" 2>/dev/null || return 2
  git -C "$SRC" merge-base --is-ancestor "$1" FETCH_HEAD || return 2
  git -C "$SRC" checkout --quiet --force --detach "$1" && git -C "$SRC" clean -qfdx
}

compilar() { # compilar ID COMMIT
  local id="$1" commit="$2" total feitos=0 ultimo=-1 pct linha
  total=$(cat "/cache/unidades-$commit" 2>/dev/null || cat /cache/unidades 2>/dev/null || echo 250)
  progresso "$id" compilando 1
  # Dependências baixadas antes; a compilação em si roda sem rede (--offline).
  (cd "$SRC" && cargo fetch --locked --quiet) || { falhar "$id" "não foi possível baixar as dependências"; return 1; }
  : >"$TRABALHO/build.log"
  (
    cd "$SRC" && CFIA_RECEITA="$TRABALHO/receita.json" cargo build --offline --locked --release \
      -p camfacil --target "$ALVO" --message-format=json-render-diagnostics 2>>"$TRABALHO/build.log"
    echo "{\"reason\":\"fim\",\"status\":$?}"
  ) | while IFS= read -r linha; do
    case "$linha" in
      *'"reason":"compiler-artifact"'*)
        feitos=$((feitos + 1))
        pct=$((feitos * 97 / total))
        [ "$pct" -gt 97 ] && pct=97
        if [ $((feitos + 1)) -ge "$total" ]; then
          # Falta só o programa: é a etapa longa (LTO), sem como medir.
          [ "$ultimo" -ne 98 ] && progresso "$id" montando 98 && ultimo=98
        elif [ "$pct" -ge $((ultimo + 5)) ]; then
          progresso "$id" compilando "$pct"
          ultimo=$pct
        fi
        ;;
      *'"reason":"fim"'*)
        echo "$feitos" >"$TRABALHO/unidades"
        exit "$(echo "$linha" | jq -r .status)"
        ;;
    esac
  done
}

trabalhar() { # trabalhar JSON_DO_PEDIDO
  local pedido="$1" id commit exe
  id=$(jq -r .id <<<"$pedido")
  commit=$(jq -r .commit <<<"$pedido")
  log "build $id (base $commit)"
  find "$TRABALHO" -mindepth 1 -delete
  jq .receita <<<"$pedido" >"$TRABALHO/receita.json"
  progresso "$id" preparando 0
  preparar_fonte "$commit"
  case $? in
    0) ;;
    2) falhar "$id" "a versão base não é um commit oficial"; return ;;
    *) falhar "$id" "não foi possível baixar o código oficial"; return ;;
  esac
  local inicio=$SECONDS
  if ! compilar "$id" "$commit"; then
    falhar "$id" "a compilação falhou: $(grep -m1 -E '^error|panicked' "$TRABALHO/build.log" | cut -c1-200)"
    return
  fi
  [ -s "$TRABALHO/unidades" ] && cp "$TRABALHO/unidades" "/cache/unidades-$commit" \
    && cp "$TRABALHO/unidades" /cache/unidades
  exe="$CARGO_TARGET_DIR/$ALVO/release/camfacil.exe"
  [ -f "$exe" ] || { falhar "$id" "o executável não saiu"; return; }
  if api PUT "/api/builder/$id/executavel" -H 'Content-Type: application/octet-stream' \
    --data-binary "@$exe" --fail-with-body >/dev/null; then
    log "build $id pronto em $((SECONDS - inicio)) s ($(stat -c %s "$exe") bytes)"
  else
    falhar "$id" "não foi possível enviar o executável"
  fi
}

log "compilador do CamFácil IA ligado; fila em $CFIA_API, base de $REF"
while true; do
  resposta=$(api POST /api/builder/pegar -w '\n%{http_code}') || { sleep "$ESPERA"; continue; }
  codigo=$(tail -n1 <<<"$resposta")
  corpo=$(sed '$d' <<<"$resposta")
  case "$codigo" in
    200) trabalhar "$corpo" ;;
    204) sleep "$ESPERA" ;;
    401) log "token recusado pelo cfia-api; confira CFIA_BUILDER_TOKEN"; sleep 60 ;;
    *) log "fila respondeu $codigo"; sleep "$ESPERA" ;;
  esac
done
