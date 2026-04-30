# Builder stage
FROM rust:1.95-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
# Copy SQLx offline cache so builds work without a live DB
COPY .sqlx/ .sqlx/

# Build both the API server and CLI
RUN SQLX_OFFLINE=true cargo build --release --package agentforge-api --package agentforge-cli

# ─── API server runtime ────────────────────────────────────────────────────────
FROM alpine:3.21 AS api

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/target/release/agentforge-api .

EXPOSE 8080
CMD ["./agentforge-api"]

# ─── CLI runtime (used by GitHub Action and direct usage) ─────────────────────
FROM alpine:3.21 AS cli

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/target/release/agentforge .

ENTRYPOINT ["./agentforge"]
