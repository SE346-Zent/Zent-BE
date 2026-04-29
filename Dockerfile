# Stage 1: Planner
FROM lukemathwalker/cargo-chef:latest-rust-1.88-bookworm AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Cacher
FROM lukemathwalker/cargo-chef:latest-rust-1.88-bookworm AS cacher
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - cached based on platform
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Builder
FROM lukemathwalker/cargo-chef:latest-rust-1.88-bookworm AS builder
WORKDIR /app
COPY . .
# Copy dependencies from cacher stage for this specific architecture
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
# Build the application
# Removed --locked here as well to handle minor inconsistencies between local and container environments
RUN cargo build --release --package zent-be --package seeder -j 4

# Stage 4: Runtime
FROM debian:bookworm-slim AS runtime
WORKDIR /app
# Install runtime dependencies for the target architecture
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binaries
COPY --from=builder /app/target/release/zent-be /usr/local/bin/zent-be
COPY --from=builder /app/target/release/seeder /usr/local/bin/seeder
# Copy the resources
COPY --from=builder /app/seeder/resources/ /app/resources/

EXPOSE 3000
CMD ["zent-be"]
