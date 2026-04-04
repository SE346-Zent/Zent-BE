FROM rust:1.88-bookworm AS builder
WORKDIR /app
COPY . .
# Building both the main app and the seeder utility
RUN cargo build --release --bin zent-be --bin seeder

FROM debian:bookworm-slim
WORKDIR /app
# Establishing certificates and dependencies efficiently
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zent-be /usr/local/bin/zent-be
COPY --from=builder /app/target/release/seeder /usr/local/bin/seeder
# Copy the resources folder so the seeder can find parts.json
COPY --from=builder /app/seeder/resources/ /app/resources/

EXPOSE 3000
CMD ["zent-be"]
