FROM rust:1.87 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo install diesel_cli --no-default-features --features postgres
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/landly-server /app/landly-server
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

COPY migrations ./migrations
COPY diesel.toml ./diesel.toml

RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 8080

COPY --chown=appuser:appuser scripts/start.sh /app/start.sh
RUN chmod +x /app/start.sh

CMD ["./landly-server"]
