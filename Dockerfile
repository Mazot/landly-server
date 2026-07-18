FROM rust:latest as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY scripts ./scripts

RUN cargo install diesel_cli --no-default-features --features postgres
RUN cargo build --release -j 1

# Build the scripts/crates workspace
WORKDIR /app/scripts/crates
RUN cargo build --release -j 1

FROM debian:latest

# curl is required by the docker-compose healthcheck (/api/healthcheck)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libpq5 \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/landly-server /app/landly-server
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel
# May be need to modify the permissions like --chown=appuser:appuser

COPY migrations ./migrations
COPY diesel.toml ./diesel.toml

RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 8080

COPY --chown=appuser:appuser --from=builder /app/scripts/data /app/country_data
COPY --chown=appuser:appuser --from=builder /app/scripts/crates/target/release/country_loader /app/country_loader
COPY --chown=appuser:appuser --from=builder /app/scripts/crates/target/release/country_parser /app/country_parser
COPY --chown=appuser:appuser src/data/schema.rs /app/src/data/schema.rs
COPY --chown=appuser:appuser scripts/start.sh /app/start.sh

RUN chmod +x /app/start.sh
RUN chmod +x /app/country_loader
RUN chmod +x /app/country_parser

CMD ["./start.sh"]
