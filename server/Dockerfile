# API web (axum) — reusa o núcleo Rust do app Tauri (opção A1). Por isso o
# build/runtime precisam das libs do WebKit (o crate `tauri`, dependência de
# `gedocs_lib`, é linkado mesmo sem executar a UI). A opção A2 (extrair
# `gedocs-core` sem Tauri) elimina essas libs — ver docs/plano-web.md.

# ---- builder ----
FROM rust:1-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config \
    libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
# Contexto = raiz do repo. Precisa do crate da API + do núcleo + da semente.
COPY src-tauri ./src-tauri
COPY server ./server
COPY config ./config
RUN cargo build --release --manifest-path server/Cargo.toml

# ---- runtime ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libwebkit2gtk-4.1-0 libgtk-3-0 libsoup-3.0-0 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/server/target/release/gedocs-server /usr/local/bin/gedocs-server
# Semente global de categorias (config, não PII).
COPY --from=builder /build/config ./config

ENV GEDOCS_BIND=0.0.0.0:8787 \
    GEDOCS_DATA_DIR=/tmp/gedocs-data \
    GEDOCS_SESSION_TTL=3600 \
    GEDOCS_SECURE_COOKIE=true \
    GEDOCS_CATEGORIAS_SEED=/app/config/categoria.json
# GEDOCS_CORS_ORIGIN e MISTRAL_API_KEY devem vir do ambiente/secret do deploy.
EXPOSE 8787
CMD ["gedocs-server"]
