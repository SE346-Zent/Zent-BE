FROM rust:1.88-bookworm AS builder
WORKDIR /app
COPY . .
# Building natively for release mode targeting the core bin
RUN cargo build --release --bin zent-be

FROM debian:bookworm-slim
WORKDIR /app
# Establishing certificates efficiently for runtime HTTP traffic
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zent-be /usr/local/bin/zent-be

EXPOSE 3000
CMD ["zent-be"]
