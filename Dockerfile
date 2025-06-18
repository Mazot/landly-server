# Многоэтапная сборка для оптимизации размера
FROM rust:1.87 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Установка Diesel CLI для миграций
RUN cargo install diesel_cli --no-default-features --features postgres

# Сборка в release режиме
RUN cargo build --release

# Финальный образ
FROM debian:bookworm-slim

# Установка необходимых зависимостей
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Копирование собранного приложения и diessel
COPY --from=builder /app/target/release/landly-server /app/landly-server
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

# Копирование миграций и конфигов
COPY migrations ./migrations
COPY diesel.toml ./diesel.toml

# Создание пользователя без root привилегий
RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 8080

# Скрипт для запуска с миграциями
COPY --chown=appuser:appuser scripts/start.sh /app/start.sh
RUN chmod +x /app/start.sh

CMD ["./landly-server"]
