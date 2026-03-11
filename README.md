# Blog Platform

A self-hosted, statically generated blog platform built with Rust (Axum + Leptos).

## Features

- **Multi-tenant**: Host multiple blogs/sites from a single installation
- **Block-based editor**: WYSIWYG editor for posts and pages
- **Static site generation**: Fast, secure static HTML output
- **Media management**: Upload images via MinIO (S3-compatible)
- **Contact forms**: Built-in contact form with submissions dashboard
- **SEO optimized**: JSON-LD schemas, sitemaps, RSS feeds
- **Dark mode**: Beautiful dark-themed admin dashboard
- **Docker-ready**: Easy deployment with Docker Compose

## Quick Start

### Prerequisites

- Docker & Docker Compose
- ~15GB RAM for initial build (subsequent builds are faster)

### Setup

1. Clone and setup environment:

```bash
# Copy environment template
cp .env.example .env

# Start infrastructure (Postgres, MinIO)
docker-compose up -d postgres minio
```

2. Build and run:

```bash
# Build the application (first build takes ~10-15 minutes)
docker-compose build app

# Start the application
docker-compose up -d app
```

3. Access the admin dashboard:

```
http://localhost:3000/admin
```

### First-time Setup

1. Create your first site via the admin dashboard
2. Add posts and pages
3. Click "Publish" to generate static files

## Architecture

| Component | Technology |
|-----------|------------|
| Backend | Axum (Rust) |
| Frontend | Leptos (Rust WASM) |
| Database | PostgreSQL |
| Storage | MinIO (S3-compatible) |
| Reverse Proxy | Traefik |
| Static Generation | minijinja |

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| DATABASE_URL | PostgreSQL connection string | postgres://blog:changeme@postgres:5432/blog_platform |
| MINIO_ENDPOINT | MinIO server endpoint | minio:9000 |
| MINIO_BUCKET | MinIO bucket name | blog-media |
| MINIO_ACCESS_KEY | MinIO access key | minioadmin |
| MINIO_SECRET_KEY | MinIO secret key | minioadmin |
| APP_HOST | Server bind address | 0.0.0.0 |
| APP_PORT | Server port | 3000 |
| SESSION_SECRET | Session encryption secret | (must be set) |

## Development

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cargo-leptos
cargo install cargo-leptos

# Run with hot reload
cargo leptos watch
```

## Production Deployment

1. Set secure values in `.env`:
   - `SESSION_SECRET`: Generate a random 32+ character string
   - `DATABASE_URL`: Production PostgreSQL credentials
   - `MINIO_*`: Production S3 credentials or use AWS S3

2. Update `docker-compose.yml` with your domain in Traefik labels

3. Deploy behind Cloudflare for SSL and DDoS protection

## Security Notes

- All endpoints require authentication except static site files
- Passwords are hashed with bcrypt (cost factor 12)
- Rate limiting is enabled (in-memory)
- Contact forms include honeypot spam protection
- Only necessary ports exposed via Traefik

## License

MIT
