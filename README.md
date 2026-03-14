# Blog Platform

Self-hosted, statically generated blog platform built with Rust (Axum).

## Features

- Single-site: One blog per installation
- Block-based editor with drag-drop reordering  
- Static site generation (fast, secure)
- Media management via MinIO (S3-compatible)
- Contact forms with submissions
- Auto-created pages: Home, About, Contact
- Configurable navigation and social links
- SEO: JSON-LD schemas, sitemaps, RSS feeds
- Dark-themed admin dashboard
- Deploy to Cloudflare Pages directly from admin

## Quick Start

### Option 1: Local Development (No Docker)

```bash
# Clone and setup
git clone https://github.com/buzzkillb/blog-platform.git
cd blog-platform

# Create .env (see .env.example for values)
cp .env.example .env
# Edit .env - minimum needed:
#   - POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB
#   - SESSION_SECRET=$(openssl rand -base64 32)

# Start PostgreSQL (required)
# e.g., via Homebrew: brew services start postgresql@16
# Or Docker: docker run -d -e POSTGRES_PASSWORD=pass -p 5432:5432 postgres:16

# Run
cargo run
```

Access admin at http://localhost:3000/admin

### Option 2: Docker Compose

```bash
git clone https://github.com/buzzkillb/blog-platform.git
cd blog-platform

cp .env.example .env
# Edit .env with your values

docker-compose up -d
```

Access admin at http://localhost:3000/admin

---

## Cloudflare Pages Deployment

To deploy directly from the admin dashboard:

### 1. Create Pages Project

Go to Cloudflare Dashboard → Workers & Pages → Create application → Direct upload

Note your **project name** (e.g., `my-blog`)

### 2. Get Account ID

Cloudflare Dashboard → Overview → Copy Account ID from URL

### 3. Create API Token (Least Privilege)

1. Go to Cloudflare Dashboard → Profile → API Tokens
2. Create Custom Token with:
   - **Permissions**: `Account` → `Cloudflare Pages` → `Edit`
3. Copy the token (shown once)

### 4. Add to .env

```
CLOUDFLARE_ACCOUNT_ID=your-account-id
CLOUDFLARE_API_TOKEN=your-api-token
CLOUDFLARE_PAGES_PROJECT=your-project-name
```

### 5. Deploy

In admin dashboard, click **Build Site** → **Deploy to Pages**

Your site will deploy to `https://your-project.pages.dev` (add custom domain in Cloudflare dashboard)

---

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| DOMAIN | Production | Your domain (e.g., example.com) |
| SESSION_SECRET | Yes | Random 32+ char string: `openssl rand -base64 32` |
| DATABASE_URL | Yes | PostgreSQL connection string |
| MINIO_* | Yes | S3-compatible storage |
| CLOUDFLARE_* | For deploy | Pages deployment credentials |

---

## Production (Docker Compose + Traefik)

1. Edit `.env` with your domain and secrets
2. Point your domain A record to server IP
3. Run `docker-compose up -d`
4. Traefik auto-configures HTTPS via Cloudflare DNS
