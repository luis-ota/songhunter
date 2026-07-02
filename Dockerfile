# Estágio 1: Builder
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache de dependências
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release && rm -rf src

# Build real
COPY . .
RUN touch src/main.rs
RUN cargo build --release

# Estágio 2: Runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    yt-dlp \
    ffmpeg \
    chromaprint-tools \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /app/data /app/tmp /app/static

COPY --from=builder /app/target/release/songhunter /app/songhunter
COPY --from=builder /app/static /app/static
COPY --from=builder /app/migrations /app/migrations

WORKDIR /app

ENV SONGFINDER_HOST=0.0.0.0
ENV SONGFINDER_PORT=3000
ENV SONGFINDER_TEMP_DIR=/app/tmp

EXPOSE 3000

CMD ["./songhunter"]
