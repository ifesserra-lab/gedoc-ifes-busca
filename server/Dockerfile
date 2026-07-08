# API web (axum) — reusa `gedocs-core` (SEM Tauri, opção A2). Sem WebKit:
# imagem/build pequenos. `aws-lc-sys` (via rustls) precisa de cmake + perl no
# builder.

# ---- builder ----
FROM rust:1-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config cmake perl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
# Contexto = raiz do repo. A API depende do núcleo (core) e da semente (config).
COPY core ./core
COPY server ./server
COPY config ./config
RUN cargo build --release --manifest-path server/Cargo.toml

# ---- runtime ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
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
