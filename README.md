# Blog Platform

Self-hosted, statically generated multi-tenant blog platform built with Rust (Axum).

## Features

- Multi-tenant: Host multiple blogs from one installation
- Block-based editor with drag-drop reordering
- Static site generation (fast, secure)
- Media management via MinIO (S3-compatible)
- Contact forms with submissions
- Auto-created pages: Home, About, Contact
- Configurable navigation and social links
- SEO: JSON-LD schemas, sitemaps, RSS feeds
- Dark-themed admin dashboard
- Docker Compose + Traefik deployment

## Quick Start

### Prerequisites
- Docker & Docker Compose

### Setup

```bash
git clone https://github.com/buzzkillb/blog-platform.git
cd blog-platform
cp .env.example .env
```

Start services:
```bash
docker-compose up -d
```

Access admin: http://localhost:3000/admin

### First-time Setup

1. Create your first site via the admin dashboard
2. Add posts and pages
3. Click "Publish" to generate static files

## Environment Variables

### Required

| Variable | Description |
|----------|-------------|
| DOMAIN | Your domain (e.g., yourdomain.com) |
| CF_API_EMAIL | Cloudflare email |
| CF_API_KEY | Cloudflare API key |
| ACME_EMAIL | Email for Let's Encrypt |
| SESSION_SECRET | Random 32+ character string |

Generate a session secret:
```bash
openssl rand -base64 32
```

### Database (defaults work for dev)

| Variable | Default |
|----------|---------|
| POSTGRES_USER | blog |
| POSTGRES_PASSWORD | changeme |
| POSTGRES_DB | blog_platform |

### MinIO (defaults work for dev)

| Variable | Default |
|----------|---------|
| MINIO_ENDPOINT | minio:9000 |
| MINIO_BUCKET | blog-media |
| MINIO_ACCESS_KEY | minioadmin |
| MINIO_SECRET_KEY | minioadmin |

### App

| Variable | Default |
|----------|---------|
| APP_HOST | 0.0.0.0 |
| APP_PORT | 3000 |

## Production

1. Set required values in `.env`
2. Point your domain A record to your server IP
3. Run `docker-compose up -d`
4. Traefik auto-configures HTTPS via Cloudflare DNS

## Development

```bash
cargo run
```

Requires PostgreSQL running locally.
