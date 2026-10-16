# ==============================================================================
# Estágio 1: Builder Estático com Rust e Musl
# ==============================================================================
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev build-base perl make

WORKDIR /usr/src/mocambo

# Copia arquivos do projeto
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Compilação estática musl em modo release
RUN cargo build --release --target x86_64-unknown-linux-musl && \
    strip target/x86_64-unknown-linux-musl/release/mocambo

# ==============================================================================
# Estágio 2: Imagem Minimalista de Runtime (< 25 MB)
# ==============================================================================
FROM alpine:3.20

RUN apk add --no-cache ca-certificates tzdata && \
    addgroup -g 10001 -S mocambo && \
    adduser -u 10001 -S mocambo -G mocambo && \
    mkdir -p /data/storage && \
    chown -R mocambo:mocambo /data

COPY --from=builder /usr/src/mocambo/target/x86_64-unknown-linux-musl/release/mocambo /usr/local/bin/mocambo

USER mocambo:mocambo

ENV HOST=0.0.0.0 \
    PORT=8080 \
    STORAGE_DIR=/data/storage \
    DB_PATH=/data/mocambo.db \
    RUST_LOG=info

VOLUME ["/data"]

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD wget -q --spider http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/mocambo"]
