# Build stage
FROM rust:latest AS builder

WORKDIR /app

# Install build dependencies and Node.js for wrangler
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# Copy source and build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY admin.html ./

RUN cargo build --release

# Final stage
FROM debian:bookworm

RUN apt-get update && apt-get install -y ca-certificates libssl3 curl && \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g wrangler && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/blog-platform ./
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/admin.html ./

EXPOSE 3000

ENV RUST_LOG=info
ENV APP_HOST=0.0.0.0
ENV APP_PORT=3000

# Note: All secrets should be passed via environment variables
# See .env.example for required variables
# For local development, you can mount a .env file
# For production, use docker-compose with environment variables

CMD ["./blog-platform"]
