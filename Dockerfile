# syntax=docker/dockerfile:1

# Estágio 1: Planner (cargo-chef)
FROM rust:1.92-slim-bookworm AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked

# Estágio 2: Gera o recipe.json
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Estágio 3: Builder com cache de dependências
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build de dependências (camada cacheada)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

# Build real do projeto
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && cp /app/target/release/songhunter /app/songhunter

# Estágio 4: Runtime enxuto
FROM python:3.11-slim-bookworm AS runtime

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    ffmpeg \
    libchromaprint-tools \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --no-cache-dir yt-dlp shazamio

RUN mkdir -p /app/data /app/tmp /app/static /app/scripts

COPY --from=builder /app/songhunter /app/songhunter
COPY static /app/static
COPY config /app/config
COPY migrations /app/migrations
COPY scripts /app/scripts

WORKDIR /app

ENV SONGFINDER_HOST=0.0.0.0
ENV SONGFINDER_PORT=3000
ENV SONGFINDER_TEMP_DIR=/app/tmp

EXPOSE 3000

CMD ["./songhunter"]
