mod errors;
mod handlers;
mod models;
mod render;
mod state;

pub use errors::{
    ApiError, CreatePageRequest, CreatePostRequest, CreateSiteRequest, CreateUserRequest,
    LoginRequest, LoginResponse, UpdatePageRequest, UpdatePostRequest, UserResponse,
};
pub use models::{ContactSubmission, Media, Page, Post, Site, User};
pub use state::AppState;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    response::Response,
    routing::get,
    Router,
};
use bytes::Bytes;
use render::make_error;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

pub mod api;
pub mod ssg;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactFormRequest {
    pub name: String,
    pub email: String,
    pub message: String,
    pub honeypot: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "blog_platform=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://blog:changeme@localhost:5432/blog_platform".to_string());

    let db = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            tracing::info!("Database connected successfully");
            pool
        }
        Err(e) => {
            tracing::error!("Failed to connect to database: {}", e);
            panic!("Database connection failed: {}", e);
        }
    };

    run_migrations(&db).await;
    seed_default_pages(&db).await;

    let state = AppState { db };

    let output_dir = std::path::Path::new("output");
    let needs_build = !output_dir.exists() || std::fs::read_dir(output_dir).map(|mut e| e.next().is_none()).unwrap_or(true);
    if needs_build {
        if let Ok(site) = sqlx::query("SELECT id FROM sites LIMIT 1")
            .fetch_one(&state.db)
            .await
        {
            let site_id: uuid::Uuid = site.get("id");
            tracing::info!("Building static site for site_id: {}", site_id);
            if let Err(e) = ssg::build_site(&state.db, site_id).await {
                tracing::error!("Failed to build static site: {}", e);
            }
        }
    }

    let static_files = ServeDir::new(".");

    let media_files = ServeDir::new("media");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/admin", get(admin_handler))
        .route("/admin/{*path}", get(admin_handler))
        .route("/", get(|State(_state): State<AppState>| async move {
            let html_path = "output/index.html";
            if std::path::Path::new(html_path).exists() {
                if let Ok(content) = tokio::fs::read_to_string(html_path).await {
                    return axum::response::Html(content).into_response();
                }
            }
            axum::response::Html("Not found").into_response()
        }))
        .route("/{*path}", get(|axum::extract::Path(path): axum::extract::Path<String>| async move {
            // Try .html first
            let html_path = format!("output/{}.html", path);
            if std::path::Path::new(&html_path).exists() {
                if let Ok(content) = tokio::fs::read_to_string(&html_path).await {
                    return axum::response::Html(content).into_response();
                }
            }
            // Try exact file match (for sitemap.xml, feed.xml, etc)
            let file_path = format!("output/{}", path);
            if std::path::Path::new(&file_path).exists() {
                if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                    return axum::response::Html(content).into_response();
                }
            }
            axum::response::Html("Not found").into_response()
        }))
        .nest_service("/static", static_files.clone())
        .nest_service("/media", media_files)
        .merge(api::routes())
        .layer(cors)
        .with_state(state);

    let host = std::env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("APP_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], port));

    tracing::info!("Starting server on http://{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root_handler(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    let site = sqlx::query("SELECT id, name, description, homepage_type FROM sites WHERE subdomain IS NULL OR subdomain = '' LIMIT 1")
        .fetch_optional(&state.db)
        .await;

    match site {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");
            let homepage_type: String = row
                .get::<Option<String>, _>("homepage_type")
                .unwrap_or_else(|| "both".to_string());

            let settings = match render::get_site_settings(&state.db, site_id).await {
                Ok(s) => s,
                Err(_) => {
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "Error loading settings",
                    )
                        .into_response()
                }
            };

            let header_html = render::render_header(&settings, &name, "");

            // Get homepage page
            let homepage_page = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT title, content FROM pages WHERE site_id = $1 AND is_homepage = true LIMIT 1"
            )
            .bind(site_id)
            .fetch_optional(&state.db)
            .await;

            let show_homepage_page = matches!(homepage_type.as_str(), "landing" | "both")
                && homepage_page.is_ok()
                && homepage_page.as_ref().ok().is_some();

            let main_content = if show_homepage_page {
                if let Ok(Some((page_title, page_content))) = homepage_page {
                    format!("<h1 class=\"text-4xl font-bold mb-6\">{}</h1><div class=\"prose mb-8\">{}</div>", page_title, render::render_blocks(&page_content))
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let posts_section = if matches!(homepage_type.as_str(), "blog" | "both") {
                let posts = sqlx::query_as::<_, (String, String, Option<String>)>(
                    "SELECT title, slug, excerpt FROM posts WHERE site_id = $1 AND status = 'published' ORDER BY published_at DESC LIMIT 5"
                )
                .bind(site_id)
                .fetch_all(&state.db)
                .await;

                if let Ok(posts) = posts {
                    if !posts.is_empty() {
                        let posts_html: String = posts.iter().map(|p| {
                            format!("<article class=\"mb-4\"><h3 class=\"text-xl font-bold\"><a href=\"/post/{}\" class=\"text-blue-600\">{}</a></h3><p class=\"text-gray-600\">{}</p></article>", p.1, p.0, p.2.as_deref().unwrap_or(""))
                        }).collect();
                        format!(
                            "<h2 class=\"text-2xl font-bold mb-4\">Latest Posts</h2>{}",
                            posts_html
                        )
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let footer_html = render::render_footer(&settings);

            let html = format!(
                r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">{}{}</div>
{}</body></html>"#,
                name, header_html, main_content, posts_section, footer_html
            );

            axum::response::Html(html).into_response()
        }
        Ok(None) => {
            axum::response::Html("Blog Platform - No sites configured. Visit /admin to create one.")
                .into_response()
        }
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Error loading site",
        )
            .into_response(),
    }
}

async fn health_check(State(state): State<AppState>) -> impl axum::response::IntoResponse {
    match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => (axum::http::StatusCode::OK, "OK"),
        Err(_) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "Database unavailable",
        ),
    }
}

async fn admin_handler(
    path: Option<axum::extract::Path<String>>,
) -> impl axum::response::IntoResponse {
    let _path = path.map(|p| p.0);
    match tokio::fs::read_to_string("admin.html").await {
        Ok(html) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/html")],
            html,
        ),
        Err(_) => (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            "Admin panel not found".to_string(),
        ),
    }
}

#[allow(dead_code)]
async fn sitemap_handler(
    slug: Option<axum::extract::Path<String>>,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    let slug = slug.map(|p| p.0);

    let site = if let Some(ref s) = slug {
        sqlx::query("SELECT id, name FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1")
            .bind(s)
            .fetch_optional(&state.db)
            .await
    } else {
        sqlx::query("SELECT id, name FROM sites LIMIT 1")
            .fetch_optional(&state.db)
            .await
    };

    match site {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let site_name: String = row.get("name");

            let posts = sqlx::query_as::<_, (String,)>(
                "SELECT slug FROM posts WHERE site_id = $1 AND status = 'published'",
            )
            .bind(site_id)
            .fetch_all(&state.db)
            .await;

            let domain = slug
                .map(|s| format!("https://{}.example.com", s))
                .unwrap_or_else(|| {
                    format!(
                        "https://{}.example.com",
                        site_name.to_lowercase().replace(' ', "-")
                    )
                });
            let mut sitemap = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<url><loc>{}</loc><changefreq>daily</changefreq></url>"#,
                domain
            );

            if let Ok(posts) = posts {
                for (slug,) in posts {
                    sitemap.push_str(&format!(
                        "\n<url><loc>{}/post/{}</loc><changefreq>weekly</changefreq></url>",
                        domain, slug
                    ));
                }
            }

            sitemap.push_str("\n</urlset>");

            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/xml")],
                sitemap,
            )
        }
        _ => (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            "Site not found".to_string(),
        ),
    }
}

#[allow(dead_code)]
async fn feed_handler(
    slug: Option<axum::extract::Path<String>>,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    let slug = slug.map(|p| p.0);

    let site = if let Some(ref s) = slug {
        sqlx::query("SELECT id, name FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1")
            .bind(s)
            .fetch_optional(&state.db)
            .await
    } else {
        sqlx::query("SELECT id, name FROM sites LIMIT 1")
            .fetch_optional(&state.db)
            .await
    };

    match site {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let site_name: String = row.get("name");

            let posts = sqlx::query_as::<_, (String, String, Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
                "SELECT title, slug, excerpt, published_at FROM posts WHERE site_id = $1 AND status = 'published' ORDER BY published_at DESC LIMIT 20"
            )
            .bind(site_id)
            .fetch_all(&state.db)
            .await;

            let domain = slug
                .map(|s| format!("https://{}.example.com", s))
                .unwrap_or_else(|| {
                    format!(
                        "https://{}.example.com",
                        site_name.to_lowercase().replace(' ', "-")
                    )
                });
            let mut feed = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
<title>{}</title>
<link>{}</link>
<description>{}</description>"#,
                site_name, domain, site_name
            );

            if let Ok(posts) = posts {
                for post in posts {
                    let title = post.0;
                    let slug = post.1;
                    let excerpt = post.2.unwrap_or_default();
                    let date = post.3.map(|d| d.to_rfc3339()).unwrap_or_default();
                    feed.push_str(&format!(
                        r#"
<item>
<title>{}</title>
<link>{}/post/{}</link>
<description>{}</description>
<pubDate>{}</pubDate>
</item>"#,
                        title, domain, slug, excerpt, date
                    ));
                }
            }

            feed.push_str("\n</channel>\n</rss>");

            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/rss+xml")],
                feed,
            )
        }
        _ => (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            "Site not found".to_string(),
        ),
    }
}

async fn output_handler(
    axum::extract::Path((site_id, path)): axum::extract::Path<(Uuid, String)>,
) -> Response {
    let clean_path = path.replace("..", "");
    let file_path = format!("output/{}/{}", site_id, clean_path);

    let canonical = match std::path::Path::new(&file_path).canonicalize() {
        Ok(p) => p,
        Err(_) => return make_error(StatusCode::NOT_FOUND, "File not found").into_response(),
    };

    let output_dir = std::env::current_dir()
        .map(|p| p.join("output").canonicalize().unwrap_or_default())
        .unwrap_or_else(|_| std::path::PathBuf::from("output"));

    if !canonical.starts_with(&output_dir) {
        return make_error(StatusCode::FORBIDDEN, "Access denied").into_response();
    }

    if let Ok(content) = std::fs::read(&file_path) {
        let content_type = if clean_path.ends_with(".html") {
            "text/html"
        } else if clean_path.ends_with(".css") {
            "text/css"
        } else if clean_path.ends_with(".js") {
            "application/javascript"
        } else if clean_path.ends_with(".png") {
            "image/png"
        } else if clean_path.ends_with(".jpg") || clean_path.ends_with(".jpeg") {
            "image/jpeg"
        } else {
            "text/plain"
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            content_type.parse().unwrap(),
        );

        let body = Bytes::from(content);
        (StatusCode::OK, headers, body).into_response()
    } else {
        make_error(StatusCode::NOT_FOUND, "File not found").into_response()
    }
}

async fn run_migrations(db: &sqlx::PgPool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sites (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            subdomain VARCHAR(63) UNIQUE,
            custom_domain VARCHAR(255) UNIQUE,
            name VARCHAR(255) NOT NULL,
            description TEXT,
            logo_url VARCHAR(1000),
            theme VARCHAR(100) DEFAULT 'default',
            nav_links JSONB DEFAULT '[]',
            footer_text VARCHAR(500),
            social_links JSONB DEFAULT '{}',
            contact_phone VARCHAR(50),
            contact_email VARCHAR(255),
            contact_address VARCHAR(500),
            homepage_type VARCHAR(20) DEFAULT 'both',
            landing_blocks JSONB DEFAULT '[]',
            blog_path VARCHAR(100) DEFAULT '/blog',
            created_at TIMESTAMPTZ DEFAULT NOW(),
            settings JSONB DEFAULT '{}'
        )",
    )
    .execute(db)
    .await
    .expect("Failed to create sites table");

    sqlx::query(
        "ALTER TABLE sites ADD COLUMN IF NOT EXISTS blog_path VARCHAR(100) DEFAULT '/blog'",
    )
    .execute(db)
    .await
    .ok();

    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS nav_links JSONB DEFAULT '[]'")
        .execute(db)
        .await
        .ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS footer_text VARCHAR(500)")
        .execute(db)
        .await
        .ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS social_links JSONB DEFAULT '{}'")
        .execute(db)
        .await
        .ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS contact_phone VARCHAR(50)")
        .execute(db)
        .await
        .ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS contact_email VARCHAR(255)")
        .execute(db)
        .await
        .ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS contact_address VARCHAR(500)")
        .execute(db)
        .await
        .ok();
    sqlx::query(
        "ALTER TABLE sites ADD COLUMN IF NOT EXISTS homepage_type VARCHAR(20) DEFAULT 'both'",
    )
    .execute(db)
    .await
    .ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS landing_blocks JSONB DEFAULT '[]'")
        .execute(db)
        .await
        .ok();
    sqlx::query(
        "ALTER TABLE sites ADD COLUMN IF NOT EXISTS blog_path VARCHAR(100) DEFAULT '/blog'",
    )
    .execute(db)
    .await
    .ok();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            name VARCHAR(255),
            created_at TIMESTAMPTZ DEFAULT NOW()
        )",
    )
    .execute(db)
    .await
    .expect("Failed to create users table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS auth_tokens (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            token VARCHAR(255) UNIQUE NOT NULL,
            expires_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )",
    )
    .execute(db)
    .await
    .expect("Failed to create auth_tokens table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_auth_tokens_token ON auth_tokens(token)")
        .execute(db)
        .await
        .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_auth_tokens_user ON auth_tokens(user_id)")
        .execute(db)
        .await
        .ok();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS site_members (
            site_id UUID REFERENCES sites(id) ON DELETE CASCADE,
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            role VARCHAR(50) DEFAULT 'editor',
            PRIMARY KEY (site_id, user_id)
        )",
    )
    .execute(db)
    .await
    .expect("Failed to create site_members table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            site_id UUID REFERENCES sites(id) ON DELETE CASCADE,
            author_id UUID REFERENCES users(id),
            title VARCHAR(500) NOT NULL,
            slug VARCHAR(500) NOT NULL,
            content JSONB NOT NULL DEFAULT '[]',
            excerpt TEXT,
            featured_image VARCHAR(1000),
            status VARCHAR(50) DEFAULT 'draft',
            published_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW(),
            seo JSONB DEFAULT '{}',
            UNIQUE(site_id, slug)
        )",
    )
    .execute(db)
    .await
    .expect("Failed to create posts table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pages (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            site_id UUID REFERENCES sites(id) ON DELETE CASCADE,
            title VARCHAR(500) NOT NULL,
            slug VARCHAR(500) NOT NULL,
            content JSONB NOT NULL DEFAULT '[]',
            is_homepage BOOLEAN DEFAULT FALSE,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW(),
            seo JSONB DEFAULT '{}',
            UNIQUE(site_id, slug)
        )",
    )
    .execute(db)
    .await
    .expect("Failed to create pages table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS media (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            site_id UUID REFERENCES sites(id) ON DELETE CASCADE,
            filename VARCHAR(500) NOT NULL,
            mime_type VARCHAR(100),
            size INTEGER,
            url VARCHAR(1000) NOT NULL,
            alt_text VARCHAR(500),
            created_at TIMESTAMPTZ DEFAULT NOW()
        )",
    )
    .execute(db)
    .await
    .expect("Failed to create media table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS contact_submissions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            site_id UUID REFERENCES sites(id) ON DELETE CASCADE,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL,
            message TEXT NOT NULL,
            honeypot VARCHAR(255) DEFAULT '',
            created_at TIMESTAMPTZ DEFAULT NOW(),
            read BOOLEAN DEFAULT FALSE
        )",
    )
    .execute(db)
    .await
    .expect("Failed to create contact_submissions table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_posts_site_status ON posts(site_id, status)")
        .execute(db)
        .await
        .ok();
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_posts_published ON posts(site_id, published_at DESC)",
    )
    .execute(db)
    .await
    .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_site ON pages(site_id)")
        .execute(db)
        .await
        .ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_media_site ON media(site_id)")
        .execute(db)
        .await
        .ok();
}

async fn seed_default_pages(db: &sqlx::PgPool) {
    let sites = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM sites WHERE id NOT IN (SELECT DISTINCT site_id FROM pages)",
    )
    .fetch_all(db)
    .await;

    if let Ok(sites) = sites {
        let site_count = sites.len();
        for (site_id,) in sites {
            let homepage_content = serde_json::json!([
                {"block_type": "hero", "content": {"title": "Welcome to Our Site", "subtitle": "Your amazing blog starts here", "ctaText": "Read More", "ctaLink": "/blog"}}
            ]);

            let about_content = serde_json::json!([
                {"block_type": "heading", "content": {"text": "About Us"}},
                {"block_type": "paragraph", "content": {"text": "Welcome to our about page! We are a company that does amazing things."}}
            ]);

            let contact_content = serde_json::json!([
                {"block_type": "heading", "content": {"text": "Contact Us"}},
                {"block_type": "paragraph", "content": {"text": "Get in touch with us!"}}
            ]);

            sqlx::query(
                "INSERT INTO pages (site_id, title, slug, content, is_homepage) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(site_id)
            .bind("Home")
            .bind("home")
            .bind(&homepage_content)
            .bind(true)
            .execute(db)
            .await.ok();

            sqlx::query(
                "INSERT INTO pages (site_id, title, slug, content, is_homepage) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(site_id)
            .bind("About")
            .bind("about")
            .bind(&about_content)
            .bind(false)
            .execute(db)
            .await.ok();

            sqlx::query(
                "INSERT INTO pages (site_id, title, slug, content, is_homepage) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(site_id)
            .bind("Contact")
            .bind("contact")
            .bind(&contact_content)
            .bind(false)
            .execute(db)
            .await.ok();
        }
        if site_count > 0 {
            tracing::info!("Seeded default pages for {} existing sites", site_count);
        }
    }

    sqlx::query(
        "UPDATE sites SET homepage_type = 'both' WHERE homepage_type IS NULL OR homepage_type = ''",
    )
    .execute(db)
    .await
    .ok();

    let default_nav = serde_json::json!([{"label": "Home", "url": "/"}, {"label": "Blog", "url": "/blog"}, {"label": "About", "url": "/about"}, {"label": "Contact", "url": "/contact"}]);
    sqlx::query(
        "UPDATE sites SET nav_links = $1 WHERE nav_links IS NULL OR nav_links = '[]'::jsonb",
    )
    .bind(&default_nav)
    .execute(db)
    .await
    .ok();
}
