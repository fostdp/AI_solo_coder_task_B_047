FROM rust:1.75-bullseye as builder

WORKDIR /app

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock 2>/dev/null || true
COPY src src

RUN apt-get update && apt-get install -y \
    libpq-dev \
    postgresql-client \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/ancient-city-morphology .
COPY .env.example .env

EXPOSE 8080

CMD ["./ancient-city-morphology"]
