FROM rust:debian-bookworm-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Pre-build dependencies to leverage Docker cache
COPY Cargo.toml ./
RUN mkdir -p src/api

# Copy all source files
COPY src ./src

# Build the project
RUN cargo build --release --bin blog-platform

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/blog-platform ./
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/admin.html .

EXPOSE 3000

ENV RUST_LOG=info
ENV APP_HOST=0.0.0.0
ENV APP_PORT=3000

CMD ["./blog-platform"]
