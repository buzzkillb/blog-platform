# Blog Platform

A self-hosted, statically generated multi-tenant blog platform built with Rust (Axum).

## Features

- **Multi-tenant**: Host multiple blogs/sites from a single installation
- **Block-based editor**: WYSIWYG editor with drag-drop reordering
- **Block types**: Hero, Text, Image, Link, Video, Columns
- **Static site generation**: Fast, secure static HTML output
- **Media management**: Upload images via MinIO (S3-compatible)
- **Contact forms**: Built-in contact form with submissions
- **Default pages**: Home, About, Contact pages auto-created
- **Homepage types**: Blog, Landing Page, or Both
- **Navigation**: Configurable nav links (Home always, Blog/About/Contact toggleable)
- **Social links**: X, Facebook, Instagram, LinkedIn, YouTube, GitHub, TikTok
- **SEO optimized**: JSON-LD schemas, sitemaps, RSS feeds
- **Dark mode**: Beautiful dark-themed admin dashboard
- **Docker-ready**: Easy deployment with Docker Compose + Traefik

## Quick Start

### Prerequisites

- Docker & Docker Compose

### Setup

1. Clone the repository:

```bash
git clone git@github.com:yourusername/blog-platform.git
cd blog-platform
```

2. Copy environment template and configure:

```bash
cp .env.example .env
```

3. Edit `.env` with your settings (see Environment Variables below)

4. Start the services:

```bash
docker-compose up -d
```

5. Access the admin dashboard:

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
| Frontend | Vanilla JS |
| Database | PostgreSQL |
| Storage | MinIO (S3-compatible) |
| Reverse Proxy | Traefik (with Cloudflare DNS challenge) |
| Static Generation | minijinja |

## Environment Variables

### Required (must set in .env)

| Variable | Description |
|----------|-------------|
| DOMAIN | Your domain (e.g., yourdomain.com) |
| CF_API_EMAIL | Cloudflare email |
| CF_API_KEY | Cloudflare API key (for automatic HTTPS) |
| ACME_EMAIL | Email for Let's Encrypt certificates |
| SESSION_SECRET | Random 32+ character string |

### Database

| Variable | Default |
|----------|---------|
| POSTGRES_USER | blog |
| POSTGRES_PASSWORD | changeme |
| POSTGRES_DB | blog_platform |
| DATABASE_URL | postgres://blog:changeme@postgres:5432/blog_platform |

### MinIO

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
| RUST_LOG | info |

## Production Deployment

1. Set required values in `.env`:
   - `DOMAIN` - your domain
   - `CF_API_EMAIL` - Cloudflare email
   - `CF_API_KEY` - Cloudflare API key (needs DNS edit permission)
   - `SESSION_SECRET` - Generate a random 32+ character string

2. Update DNS:
   - Point your domain A record to your server IP
   - Create necessary records (@, www, etc.)

3. Start services:

```bash
docker-compose up -d
```

4. Traefik will automatically:
   - Detect your domain
   - Verify ownership via Cloudflare DNS
   - Get SSL certificate from Let's Encrypt
   - Force HTTPS

## Development

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Run locally (requires PostgreSQL)
cargo run
```

## Security Notes

- Passwords hashed with bcrypt (cost factor 12)
- Contact forms include honeypot spam protection
- Only ports 80/443 exposed via Traefik
- Use strong SESSION_SECRET in production
- Keep dependencies updated

## License

MIT
