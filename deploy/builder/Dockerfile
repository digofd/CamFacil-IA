# CamFácil IA · compilador do "Gerar sua versão" (SDD-12).
# Compila o executável do Windows com o mingw-w64 (software livre), a partir
# do código oficial e da receita que vem da fila do cfia-api.
FROM rust:1-bookworm
LABEL org.opencontainers.image.source=https://github.com/emersoncassis/CamFacil-IA \
      org.opencontainers.image.description="Compilador das versões geradas do CamFácil IA" \
      org.opencontainers.image.licenses=AGPL-3.0-only
RUN apt-get update \
 && apt-get install -y --no-install-recommends mingw-w64 jq \
 && rm -rf /var/lib/apt/lists/* \
 && rustup target add x86_64-pc-windows-gnu \
 && useradd --create-home --uid 10001 compilador \
 && mkdir -p /cache /trabalho && chown compilador:compilador /cache /trabalho
COPY --chmod=755 trabalhador.sh /usr/local/bin/trabalhador.sh
USER compilador
# /cache guarda o código, as dependências e a pasta target/ entre builds.
VOLUME ["/cache"]
ENTRYPOINT ["/usr/local/bin/trabalhador.sh"]
