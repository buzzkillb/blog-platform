use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub id: Uuid,
    pub subdomain: Option<String>,
    pub custom_domain: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub theme: String,
    pub nav_links: serde_json::Value,
    pub footer_text: Option<String>,
    pub social_links: serde_json::Value,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub contact_address: Option<String>,
    pub homepage_type: String,
    pub blog_path: Option<String>,
    pub landing_blocks: serde_json::Value,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub site_id: Uuid,
    pub author_id: Option<Uuid>,
    pub title: String,
    pub slug: String,
    pub content: serde_json::Value,
    pub excerpt: Option<String>,
    pub featured_image: Option<String>,
    pub status: String,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub seo: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: Uuid,
    pub site_id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: serde_json::Value,
    pub is_homepage: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub seo: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub id: Uuid,
    pub site_id: Uuid,
    pub filename: String,
    pub mime_type: Option<String>,
    pub size: Option<i32>,
    pub url: String,
    pub alt_text: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSubmission {
    pub id: Uuid,
    pub site_id: Uuid,
    pub name: String,
    pub email: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}

impl ApiError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSiteRequest {
    pub subdomain: Option<String>,
    pub custom_domain: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: User,
    pub site_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub slug: Option<String>,
    pub content: serde_json::Value,
    pub excerpt: Option<String>,
    pub featured_image: Option<String>,
    pub seo: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub content: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image: Option<String>,
    pub status: Option<String>,
    pub seo: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePageRequest {
    pub title: String,
    pub slug: Option<String>,
    pub content: serde_json::Value,
    pub is_homepage: Option<bool>,
    pub seo: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePageRequest {
    pub title: Option<String>,
    pub content: Option<serde_json::Value>,
    pub is_homepage: Option<bool>,
    pub seo: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactFormRequest {
    pub name: String,
    pub email: String,
    pub message: String,
    pub honeypot: Option<String>,
}

use axum::{
    extract::State,
    handler::HandlerWithoutStateExt,
    response::IntoResponse,
    http::StatusCode,
    http::header::HeaderMap,
    routing::get,
    Router,
};

use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod api;
pub mod ssg;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
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

    // Seed default pages for existing sites that don't have them
    seed_default_pages(&db).await;

    let state = AppState { db };

    let static_files = ServeDir::new(".")
        .not_found_service(static_handler.into_service());
    
    let media_files = ServeDir::new("media");

    // CORS layer - configure for your production domain
    let cors = CorsLayer::new()
        .allow_origin(Any)  // Change to specific origin in production
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_check))
        .route("/admin", get(admin_handler))
        .route("/admin/*path", get(admin_handler))
        .route("/site/:slug", get(view_site))
        .route("/site/:slug/post/:post_slug", get(view_post))
        .route("/site/:slug/page/:page_slug", get(view_page))
        .route("/site/:slug/*path", get(view_blog_at_path))
        .route("/sitemap.xml", get(sitemap_handler))
        .route("/feed.xml", get(feed_handler))
        .route("/output/:site_id/index.html", get(output_handler))
        .route("/output/:site_id/*path", get(output_handler))
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

async fn root_handler() -> impl IntoResponse {
    (StatusCode::OK, "Blog Platform API - Visit /admin for dashboard")
}

async fn admin_handler() -> impl IntoResponse {
    match tokio::fs::read_to_string("admin.html").await {
        Ok(content) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/html")],
            content,
        ),
        Err(_) => (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            "Admin dashboard not found. Please ensure admin.html exists.".to_string(),
        ),
    }
}

async fn static_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        "Not found",
    )
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").fetch_one(&state.db).await {
        Ok(_) => (StatusCode::OK, "Healthy"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "Unhealthy"),
    }
}

async fn sitemap_handler(State(state): State<AppState>) -> impl IntoResponse {
    let posts = sqlx::query_as::<_, (String, String, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT slug, title, published_at FROM posts WHERE status = 'published' ORDER BY published_at DESC LIMIT 100"
    )
    .fetch_all(&state.db)
    .await;

    match posts {
        Ok(posts) => {
            let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{}
</urlset>"#,
                posts.iter().map(|p| format!(
                    r#"  <url>
    <loc>https://example.com/{}</loc>
    <lastmod>{}</lastmod>
  </url>"#,
                    p.0,
                    p.2.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
                )).collect::<Vec<_>>().join("\n")
            );
            (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "application/xml")], xml)
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            "Failed to fetch posts".to_string(),
        ),
    }
}

async fn feed_handler(State(state): State<AppState>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT name, description FROM sites LIMIT 1"
    )
    .fetch_one(&state.db)
    .await;

    let posts = sqlx::query_as::<_, (String, String, Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT slug, title, excerpt, published_at FROM posts WHERE status = 'published' ORDER BY published_at DESC LIMIT 20"
    )
    .fetch_all(&state.db)
    .await;

    match (result, posts) {
        (Ok((name, _)), Ok(posts)) => {
            let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
  <title>{}</title>
  <link>https://example.com</link>
  <description>{}</description>
{}
</channel>
</rss>"#,
                name,
                name,
                posts.iter().map(|p| format!(
                    r#"  <item>
    <title>{}</title>
    <link>https://example.com/{}</link>
    <description>{}</description>
    <pubDate>{}</pubDate>
  </item>"#,
                    p.1,
                    p.0,
                    p.2.as_ref().map_or("", |v| v),
                    p.3.map(|d| d.to_rfc2822()).unwrap_or_default()
                )).collect::<Vec<_>>().join("\n")
            );
            (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "application/xml")], xml)
        }
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            "Failed to fetch feed".to_string(),
        ),
    }
}

async fn output_handler(
    axum::extract::Path((site_id, path)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    let output_dir = std::path::Path::new("output").join(&site_id);
    let file_path = output_dir.join(&path);
    
    if file_path.exists() {
        match tokio::fs::read(&file_path).await {
            Ok(content) => {
                let mime = if path.ends_with(".html") {
                    "text/html"
                } else if path.ends_with(".xml") {
                    "application/xml"
                } else if path.ends_with(".css") {
                    "text/css"
                } else if path.ends_with(".js") {
                    "application/javascript"
                } else {
                    "text/plain"
                };
                (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, mime)], content)
            }
            Err(_) => (
                StatusCode::NOT_FOUND,
                [(axum::http::header::CONTENT_TYPE, "text/plain")],
                "File not found".as_bytes().to_vec(),
            ),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            "Not found".as_bytes().to_vec(),
        )
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
            created_at TIMESTAMPTZ DEFAULT NOW(),
            settings JSONB DEFAULT '{}'
        )"
    ).execute(db).await.expect("Failed to create sites table");

    // Add new columns if they don't exist (for existing databases)
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS nav_links JSONB DEFAULT '[]'").execute(db).await.ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS footer_text VARCHAR(500)").execute(db).await.ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS social_links JSONB DEFAULT '{}'").execute(db).await.ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS contact_phone VARCHAR(50)").execute(db).await.ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS contact_email VARCHAR(255)").execute(db).await.ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS contact_address VARCHAR(500)").execute(db).await.ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS homepage_type VARCHAR(20) DEFAULT 'blog'").execute(db).await.ok();
    sqlx::query("ALTER TABLE sites ADD COLUMN IF NOT EXISTS landing_blocks JSONB DEFAULT '[]'").execute(db).await.ok();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            name VARCHAR(255),
            created_at TIMESTAMPTZ DEFAULT NOW()
        )"
    ).execute(db).await.expect("Failed to create users table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS site_members (
            site_id UUID REFERENCES sites(id) ON DELETE CASCADE,
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            role VARCHAR(50) DEFAULT 'editor',
            PRIMARY KEY (site_id, user_id)
        )"
    ).execute(db).await.expect("Failed to create site_members table");

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
        )"
    ).execute(db).await.expect("Failed to create posts table");

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
        )"
    ).execute(db).await.expect("Failed to create pages table");

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
        )"
    ).execute(db).await.expect("Failed to create media table");

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
        )"
    ).execute(db).await.expect("Failed to create contact_submissions table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_posts_site_status ON posts(site_id, status)").execute(db).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_posts_published ON posts(site_id, published_at DESC)").execute(db).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_site ON pages(site_id)").execute(db).await.ok();
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_media_site ON media(site_id)").execute(db).await.ok();
}

async fn seed_default_pages(db: &sqlx::PgPool) {
    // Get all sites that don't have any pages
    let sites = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM sites WHERE id NOT IN (SELECT DISTINCT site_id FROM pages)"
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

            // Insert homepage page
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

            // Insert About page
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

            // Insert Contact page
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

    // Also update existing sites that don't have homepage_type set to 'both'
    sqlx::query("UPDATE sites SET homepage_type = 'both' WHERE homepage_type IS NULL OR homepage_type = ''")
        .execute(db)
        .await
        .ok();
    
    // Update nav_links for existing sites that have empty nav_links
    let default_nav = serde_json::json!([{"label": "Home", "url": "/"}, {"label": "Blog", "url": "/blog"}, {"label": "About", "url": "/about"}, {"label": "Contact", "url": "/contact"}]);
    sqlx::query("UPDATE sites SET nav_links = $1 WHERE nav_links IS NULL OR nav_links = '[]'::jsonb")
        .bind(&default_nav)
        .execute(db)
        .await
        .ok();
}

async fn view_site(
    axum::extract::Path(slug): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let site_result = sqlx::query(
        "SELECT id, name, description, homepage_type FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    match site_result {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");
            let description = row.get::<Option<String>, _>("description").unwrap_or_default();
            let homepage_type: String = row.get::<Option<String>, _>("homepage_type").unwrap_or_else(|| "both".to_string());
            
            let settings_result = sqlx::query(
                "SELECT logo_url, nav_links, footer_text, social_links, contact_email, contact_phone, contact_address FROM sites WHERE id = $1"
            )
            .bind(site_id)
            .fetch_one(&state.db)
            .await;

            let (logo_url, nav_links, footer_text, social_links, contact_email, contact_phone, contact_address) = match settings_result {
                Ok(sr) => (
                    sr.get::<Option<String>, _>("logo_url").unwrap_or_default(),
                    sr.get::<serde_json::Value, _>("nav_links"),
                    sr.get::<Option<String>, _>("footer_text").unwrap_or_default(),
                    sr.get::<serde_json::Value, _>("social_links"),
                    sr.get::<Option<String>, _>("contact_email").unwrap_or_default(),
                    sr.get::<Option<String>, _>("contact_phone").unwrap_or_default(),
                    sr.get::<Option<String>, _>("contact_address").unwrap_or_default(),
                ),
                Err(_) => (String::new(), serde_json::Value::Array(vec![]), String::new(), serde_json::Value::Object(serde_json::Map::new()), String::new(), String::new(), String::new()),
            };
            
            // Check for homepage page
            let homepage_page = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT title, content FROM pages WHERE site_id = $1 AND is_homepage = true LIMIT 1"
            )
            .bind(site_id)
            .fetch_optional(&state.db)
            .await;
            
            // Determine what to show on homepage
            let show_homepage_page = matches!(homepage_type.as_str(), "landing" | "both") && homepage_page.is_ok() && homepage_page.as_ref().ok().is_some();
            let site_path = format!("/site/{}", slug);
            
            let nav_html = if let Some(links) = nav_links.as_array() {
                links.iter().map(|link| {
                    let label = link.get("label").and_then(|l| l.as_str()).unwrap_or("");
                    let url = link.get("url").and_then(|u| u.as_str()).unwrap_or("#");
                    let full_url = if url.starts_with('/') {
                        format!("{}{}", site_path, url)
                    } else {
                        url.to_string()
                    };
                    format!("<a href=\"{}\" class=\"text-gray-700 hover:text-blue-600 px-3\">{}</a>", full_url, label)
                }).collect::<Vec<_>>().join("")
            } else { String::new() };

            let logo_img = if !logo_url.is_empty() {
                format!("<img src=\"{}\" class=\"h-10 w-auto\">", logo_url)
            } else { String::new() };

            let header_html = format!(r#"
<header class="bg-white shadow-sm">
    <div class="max-w-4xl mx-auto px-6 py-4 flex items-center justify-between">
        <div class="flex items-center gap-4">
            {}<a href="/site/{}" class="text-xl font-bold text-gray-800">{}</a>
        </div>
        <nav class="flex items-center gap-2">{}</nav>
    </div>
</header>"#, logo_img, slug, name, nav_html);

            let social_html = if let Some(social) = social_links.as_object() {
                social.iter().filter_map(|(platform, url)| {
                    let url_str = url.as_str()?;
                    if url_str.is_empty() { return None; }
                    let icon = match platform.as_str() {
                        "x" => "fa-x-twitter",
                        "facebook" => "fa-facebook", 
                        "instagram" => "fa-instagram",
                        "linkedin" => "fa-linkedin",
                        "youtube" => "fa-youtube",
                        "github" => "fa-github",
                        "tiktok" => "fa-tiktok",
                        _ => "fa-link"
                    };
                    Some(format!("<a href=\"{}\" target=\"_blank\" class=\"text-gray-500 hover:text-gray-700\"><i class=\"fab {}\"></i></a>", url_str, icon))
                }).collect::<Vec<_>>().join(" ")
            } else { String::new() };

            let mut contact_parts = Vec::new();
            if !contact_phone.is_empty() { contact_parts.push(contact_phone); }
            if !contact_email.is_empty() { contact_parts.push(format!("<a href=\"mailto:{}\">{}</a>", contact_email, contact_email)); }
            if !contact_address.is_empty() { contact_parts.push(contact_address); }
            let contact_html = contact_parts.join(" | ");

            let footer_html = format!(r#"
<footer class="bg-gray-100 mt-16">
    <div class="max-w-4xl mx-auto px-6 py-8">
        <div class="flex flex-col md:flex-row justify-between items-center gap-4">
            <div class="text-gray-600 text-sm">{}</div>
            <div class="flex gap-4">{}</div>
        </div>
        <div class="text-center text-gray-500 text-sm mt-4">{}</div>
    </div>
</footer>"#, 
                if !contact_html.is_empty() { format!("<div class=\"mb-2\">{}</div>", contact_html) } else { String::new() },
                social_html,
                footer_text
            );

            // Check for homepage page
            let homepage_page = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT title, content FROM pages WHERE site_id = $1 AND is_homepage = true LIMIT 1"
            )
            .bind(site_id)
            .fetch_optional(&state.db)
            .await;

            // Determine what to show based on homepage_type and homepage page existence
            let homepage_content = if show_homepage_page {
                if let Ok(Some((page_title, page_content))) = homepage_page {
                    Some((page_title, render_blocks(&page_content)))
                } else {
                    None
                }
            } else {
                None
            };

            // Fetch posts
            let posts_result = sqlx::query_as::<_, (String, String, Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
                "SELECT title, slug, excerpt, published_at FROM posts WHERE site_id = $1 AND status = 'published' ORDER BY published_at DESC LIMIT 20"
            )
            .bind(site_id)
            .fetch_all(&state.db)
            .await;

            let posts_html = match posts_result {
                Ok(posts) => posts.iter().map(|p| format!(
                    r#"<article class="mb-8"><h2 class="text-2xl font-bold mb-2"><a href="/site/{}/post/{}" class="text-blue-600">{}</a></h2><p class="text-gray-600">{}</p></article>"#,
                    slug, p.1, p.0, p.2.as_deref().unwrap_or("")
                )).collect::<Vec<_>>().join("\n"),
                _ => String::new(),
            };

            let html = if homepage_type == "landing" {
                // Show only homepage page (no blog posts)
                if let Some((page_title, page_content)) = homepage_content {
                    format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title><script src="https://cdn.tailwindcss.com"></script><link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"></head>
<body class="bg-gray-50">
{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-6">{}</h1>
<div class="prose">{}</div>
</div>
{}
</body></html>"#, name, header_html, page_title, page_content, footer_html)
                } else {
                    format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title><script src="https://cdn.tailwindcss.com"></script><link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"></head>
<body class="bg-gray-50">
{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-2">{}</h1>
<p class="text-gray-600 mb-8">{}</p>
{}
</div>
{}
</body></html>"#, name, header_html, name, description, posts_html, footer_html)
                }
            } else if homepage_type == "blog" {
                // Show only blog posts
                format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title><script src="https://cdn.tailwindcss.com"></script><link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"></head>
<body class="bg-gray-50">
{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-2">{}</h1>
<p class="text-gray-600 mb-8">{}</p>
{}
</div>
{}
</body></html>"#, name, header_html, name, description, posts_html, footer_html)
            } else {
                // Both: show homepage page followed by blog posts
                if let Some((page_title, page_content)) = homepage_content {
                    format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title><script src="https://cdn.tailwindcss.com"></script><link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"></head>
<body class="bg-gray-50">
{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-6">{}</h1>
<div class="prose mb-8">{}</div>
<h2 class="text-2xl font-bold mb-4">Latest Posts</h2>
{}
</div>
{}
</body></html>"#, name, header_html, page_title, page_content, posts_html, footer_html)
                } else {
                    format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title><script src="https://cdn.tailwindcss.com"></script><link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"></head>
<body class="bg-gray-50">
{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-2">{}</h1>
<p class="text-gray-600 mb-8">{}</p>
{}
</div>
{}
</body></html>"#, name, header_html, name, description, posts_html, footer_html)
                }
            };

            (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "text/html")], html)
        }
        _ => (StatusCode::NOT_FOUND, [(axum::http::header::CONTENT_TYPE, "text/plain")], "Site not found".to_string()),
    }
}

async fn view_post(
    axum::extract::Path((slug, post_slug)): axum::extract::Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let site_row = sqlx::query(
        "SELECT id, name, description FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    match site_row {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");
            let _description = row.get::<Option<String>, _>("description").unwrap_or_default();
            
            let settings_result = sqlx::query(
                "SELECT logo_url, nav_links, footer_text, social_links, contact_email, contact_phone, contact_address FROM sites WHERE id = $1"
            )
            .bind(site_id)
            .fetch_one(&state.db)
            .await;

            let logo_url: String = settings_result.as_ref().ok().and_then(|sr| sr.get("logo_url")).unwrap_or_default();
            let nav_links: serde_json::Value = settings_result.as_ref().ok().and_then(|sr| sr.get("nav_links")).unwrap_or(serde_json::Value::Array(vec![]));
            let footer_text: String = settings_result.as_ref().ok().and_then(|sr| sr.get("footer_text")).unwrap_or_default();
            let social_links: serde_json::Value = settings_result.as_ref().ok().and_then(|sr| sr.get("social_links")).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let contact_email: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_email")).unwrap_or_default();
            let contact_phone: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_phone")).unwrap_or_default();
            let contact_address: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_address")).unwrap_or_default();
            let site_path = format!("/site/{}", slug);
            
            let nav_html = if let Some(links) = nav_links.as_array() {
                links.iter().map(|link| {
                    let label = link.get("label").and_then(|l| l.as_str()).unwrap_or("");
                    let url = link.get("url").and_then(|u| u.as_str()).unwrap_or("#");
                    let full_url = if url.starts_with('/') {
                        format!("{}{}", site_path, url)
                    } else {
                        url.to_string()
                    };
                    format!("<a href=\"{}\" class=\"text-gray-700 hover:text-blue-600 px-3\">{}</a>", full_url, label)
                }).collect::<Vec<_>>().join("")
            } else { String::new() };

            let logo_img = if !logo_url.is_empty() {
                format!("<img src=\"{}\" class=\"h-10 w-auto\">", logo_url)
            } else { String::new() };

            let header_html = format!(r#"
<header class="bg-white shadow-sm">
    <div class="max-w-4xl mx-auto px-6 py-4 flex items-center justify-between">
        <div class="flex items-center gap-4">
            {}<a href="/site/{}" class="text-xl font-bold text-gray-800">{}</a>
        </div>
        <nav class="flex items-center gap-2">{}</nav>
    </div>
</header>"#, logo_img, slug, name, nav_html);

            let social_html = if let Some(social) = social_links.as_object() {
                social.iter().filter_map(|(platform, url)| {
                    let url_str = url.as_str()?;
                    if url_str.is_empty() { return None; }
                    let icon = match platform.as_str() {
                        "x" => "fa-x-twitter",
                        "facebook" => "fa-facebook", 
                        "instagram" => "fa-instagram",
                        "linkedin" => "fa-linkedin",
                        "youtube" => "fa-youtube",
                        "github" => "fa-github",
                        "tiktok" => "fa-tiktok",
                        _ => "fa-link"
                    };
                    Some(format!("<a href=\"{}\" target=\"_blank\" class=\"text-gray-500 hover:text-gray-700\"><i class=\"fab {}\"></i></a>", url_str, icon))
                }).collect::<Vec<_>>().join(" ")
            } else { String::new() };

            let mut contact_parts = Vec::new();
            if !contact_phone.is_empty() { contact_parts.push(contact_phone); }
            if !contact_email.is_empty() { contact_parts.push(format!("<a href=\"mailto:{}\">{}</a>", contact_email, contact_email)); }
            if !contact_address.is_empty() { contact_parts.push(contact_address); }
            let contact_html = contact_parts.join(" | ");

            let footer_html = format!(r#"
<footer class="bg-gray-100 mt-16">
    <div class="max-w-4xl mx-auto px-6 py-8">
        <div class="flex flex-col md:flex-row justify-between items-center gap-4">
            <div class="text-gray-600 text-sm">{}</div>
            <div class="flex gap-4">{}</div>
        </div>
        <div class="text-center text-gray-500 text-sm mt-4">{}</div>
    </div>
</footer>"#, 
                if !contact_html.is_empty() { format!("<div class=\"mb-2\">{}</div>", contact_html) } else { String::new() },
                social_html,
                footer_text
            );

            let post = sqlx::query_as::<_, (String, serde_json::Value, Option<String>, chrono::DateTime<chrono::Utc>)>(
                "SELECT title, content, excerpt, published_at FROM posts WHERE site_id = $1 AND slug = $2 AND status = 'published'"
            )
            .bind(site_id)
            .bind(&post_slug)
            .fetch_optional(&state.db)
            .await;

            match post {
                Ok(Some((title, content, _excerpt, published_at))) => {
                    let content_html = render_blocks(&content);
                    let html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title><script src="https://cdn.tailwindcss.com"></script><link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"></head>
<body class="bg-gray-50">
{}
<div class="max-w-2xl mx-auto p-8">
<a href="/site/{}" class="text-blue-600">← Back</a>
<h1 class="text-4xl font-bold mt-4 mb-2">{}</h1>
<p class="text-gray-500 mb-8">{}</p>
<div class="prose">{}</div>
</div>
{}
</body></html>"#, title, header_html, slug, title, published_at.format("%Y-%m-%d"), content_html, footer_html);

                    (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "text/html")], html)
                }
                _ => (StatusCode::NOT_FOUND, [(axum::http::header::CONTENT_TYPE, "text/plain")], "Post not found".to_string()),
            }
        }
        _ => (StatusCode::NOT_FOUND, [(axum::http::header::CONTENT_TYPE, "text/plain")], "Site not found".to_string()),
    }
}

fn render_blocks(content: &serde_json::Value) -> String {
    if let Some(blocks) = content.as_array() {
        blocks.iter().map(|block| {
            let block_type = block.get("block_type").and_then(|b| b.as_str()).unwrap_or("text");
            let block_content = block.get("content");
            match block_type {
                "heading" => {
                    let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or("");
                    format!("<h2 class=\"text-2xl font-bold mt-6 mb-4\">{}</h2>", text)
                }
                "paragraph" => {
                    let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or("");
                    format!("<p class=\"mb-4\">{}</p>", text)
                }
                "image" => {
                    let url = block_content.and_then(|c| c.get("url")).and_then(|u| u.as_str()).unwrap_or("");
                    let alt = block_content.and_then(|c| c.get("alt")).and_then(|a| a.as_str()).unwrap_or("");
                    if !url.is_empty() {
                        format!("<img src=\"{}\" alt=\"{}\" class=\"w-full h-auto rounded-lg mb-6\">", url, alt)
                    } else { String::new() }
                }
                "link" => {
                    let href = block_content.and_then(|c| c.get("href")).and_then(|h| h.as_str()).unwrap_or("");
                    let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or(href);
                    if !href.is_empty() {
                        format!("<a href=\"{}\" class=\"text-blue-600\">{}</a>", href, text)
                    } else { String::new() }
                }
                "hero" => {
                    let title = block_content.and_then(|c| c.get("title")).and_then(|t| t.as_str()).unwrap_or("");
                    let subtitle = block_content.and_then(|c| c.get("subtitle")).and_then(|t| t.as_str()).unwrap_or("");
                    let bg = block_content.and_then(|c| c.get("backgroundImage")).and_then(|t| t.as_str()).unwrap_or("");
                    let cta_text = block_content.and_then(|c| c.get("ctaText")).and_then(|t| t.as_str()).unwrap_or("");
                    let cta_link = block_content.and_then(|c| c.get("ctaLink")).and_then(|t| t.as_str()).unwrap_or("#");
                    let bg_style = if !bg.is_empty() {
                        format!("background-image: linear-gradient(rgba(0,0,0,0.6), rgba(0,0,0,0.6)), url('{}'); background-size: cover; background-position: center;", bg)
                    } else {
                        "background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);".to_string()
                    };
                    format!(r#"<div class="hero-section" style="{}; padding: 80px 20px; text-align: center; color: white; border-radius: 12px; margin: 20px 0;">
                        <h1 class="text-4xl font-bold mb-4">{}</h1>
                        <p class="text-xl mb-6">{}</p>
                        {}
                    </div>"#, bg_style, title, subtitle, if !cta_text.is_empty() { format!("<a href=\"{}\" class=\"inline-block bg-white text-purple-600 px-6 py-3 rounded-lg font-medium\">{}</a>", cta_link, cta_text) } else { String::new() })
                }
                "video" => {
                    let url = block_content.and_then(|c| c.get("url")).and_then(|t| t.as_str()).unwrap_or("");
                    let caption = block_content.and_then(|c| c.get("caption")).and_then(|t| t.as_str()).unwrap_or("");
                    let embed_html = if url.contains("youtube.com") || url.contains("youtu.be") {
                        let video_id = if url.contains("v=") {
                            url.split("v=").nth(1).unwrap_or("").split('&').next().unwrap_or("")
                        } else {
                            url.split('/').next_back().unwrap_or("").split('?').next().unwrap_or("")
                        };
                        format!("<iframe width=\"100%\" height=\"400\" src=\"https://www.youtube.com/embed/{}\" frameborder=\"0\" allowfullscreen></iframe>", video_id)
                    } else if url.contains("vimeo.com") {
                        let video_id = url.split('/').next_back().unwrap_or("");
                        format!("<iframe width=\"100%\" height=\"400\" src=\"https://player.vimeo.com/video/{}\" frameborder=\"0\" allowfullscreen></iframe>", video_id)
                    } else if url.ends_with(".mp4") {
                        format!("<video width=\"100%\" controls><source src=\"{}\" type=\"video/mp4\">Your browser does not support video.</video>", url)
                    } else {
                        String::new()
                    };
                    if !url.is_empty() {
                        format!("<div class=\"video-block my-6\">{}</div>{}", embed_html, if !caption.is_empty() { format!("<p class=\"text-gray-500 text-center mt-2\">{}</p>", caption) } else { String::new() })
                    } else { String::new() }
                }
                "columns" => {
                    let left = block_content.and_then(|c| c.get("left")).and_then(|t| t.as_str()).unwrap_or("");
                    let right = block_content.and_then(|c| c.get("right")).and_then(|t| t.as_str()).unwrap_or("");
                    let left_img = block_content.and_then(|c| c.get("leftImage")).and_then(|t| t.as_str()).unwrap_or("");
                    let right_img = block_content.and_then(|c| c.get("rightImage")).and_then(|t| t.as_str()).unwrap_or("");
                    format!(r#"<div class="columns-block grid grid-cols-1 md:grid-cols-2 gap-8 my-8">
                        <div class="left-col">
                            {} {}
                        </div>
                        <div class="right-col">
                            {} {}
                        </div>
                    </div>"#, 
                        if !left_img.is_empty() { format!("<img src=\"{}\" class=\"w-full rounded-lg mb-4\">", left_img) } else { String::new() },
                        if !left.is_empty() { format!("<p>{}</p>", left) } else { String::new() },
                        if !right_img.is_empty() { format!("<img src=\"{}\" class=\"w-full rounded-lg mb-4\">", right_img) } else { String::new() },
                        if !right.is_empty() { format!("<p>{}</p>", right) } else { String::new() }
                    )
                }
                _ => String::new(),
            }
        }).collect()
    } else {
        String::new()
    }
}

async fn view_page(
    axum::extract::Path((slug, page_slug)): axum::extract::Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let site_row = sqlx::query(
        "SELECT id, name, description FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    match site_row {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");
            let _description = row.get::<Option<String>, _>("description").unwrap_or_default();
            
            let settings_result = sqlx::query(
                "SELECT logo_url, nav_links, footer_text, social_links, contact_email, contact_phone, contact_address FROM sites WHERE id = $1"
            )
            .bind(site_id)
            .fetch_one(&state.db)
            .await;

            let logo_url: String = settings_result.as_ref().ok().and_then(|sr| sr.get("logo_url")).unwrap_or_default();
            let nav_links: serde_json::Value = settings_result.as_ref().ok().and_then(|sr| sr.get("nav_links")).unwrap_or(serde_json::Value::Array(vec![]));
            let footer_text: String = settings_result.as_ref().ok().and_then(|sr| sr.get("footer_text")).unwrap_or_default();
            let social_links: serde_json::Value = settings_result.as_ref().ok().and_then(|sr| sr.get("social_links")).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let contact_email: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_email")).unwrap_or_default();
            let contact_phone: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_phone")).unwrap_or_default();
            let contact_address: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_address")).unwrap_or_default();
            let site_path = format!("/site/{}", slug);
            
            let nav_html = if let Some(links) = nav_links.as_array() {
                links.iter().map(|link| {
                    let label = link.get("label").and_then(|l| l.as_str()).unwrap_or("");
                    let url = link.get("url").and_then(|u| u.as_str()).unwrap_or("#");
                    let full_url = if url.starts_with('/') {
                        format!("{}{}", site_path, url)
                    } else {
                        url.to_string()
                    };
                    format!("<a href=\"{}\" class=\"text-gray-700 hover:text-blue-600 px-3\">{}</a>", full_url, label)
                }).collect::<Vec<_>>().join("")
            } else { String::new() };

            let logo_img = if !logo_url.is_empty() {
                format!("<img src=\"{}\" class=\"h-10 w-auto\">", logo_url)
            } else { String::new() };

            let header_html = format!(r#"
<header class="bg-white shadow-sm">
    <div class="max-w-4xl mx-auto px-6 py-4 flex items-center justify-between">
        <div class="flex items-center gap-4">
            {}<a href="/site/{}" class="text-xl font-bold text-gray-800">{}</a>
        </div>
        <nav class="flex items-center gap-2">{}</nav>
    </div>
</header>"#, logo_img, slug, name, nav_html);

            let social_html = if let Some(social) = social_links.as_object() {
                social.iter().filter_map(|(platform, url)| {
                    let url_str = url.as_str()?;
                    if url_str.is_empty() { return None; }
                    let icon = match platform.as_str() {
                        "x" => "fa-x-twitter",
                        "facebook" => "fa-facebook", 
                        "instagram" => "fa-instagram",
                        "linkedin" => "fa-linkedin",
                        "youtube" => "fa-youtube",
                        "github" => "fa-github",
                        "tiktok" => "fa-tiktok",
                        _ => "fa-link"
                    };
                    Some(format!("<a href=\"{}\" target=\"_blank\" class=\"text-gray-500 hover:text-gray-700\"><i class=\"fab {}\"></i></a>", url_str, icon))
                }).collect::<Vec<_>>().join(" ")
            } else { String::new() };

            let mut contact_parts = Vec::new();
            if !contact_phone.is_empty() { contact_parts.push(contact_phone); }
            if !contact_email.is_empty() { contact_parts.push(format!("<a href=\"mailto:{}\">{}</a>", contact_email, contact_email)); }
            if !contact_address.is_empty() { contact_parts.push(contact_address); }
            let contact_html = contact_parts.join(" | ");

            let footer_html = format!(r#"
<footer class="bg-gray-100 mt-16">
    <div class="max-w-4xl mx-auto px-6 py-8">
        <div class="flex flex-col md:flex-row justify-between items-center gap-4">
            <div class="text-gray-600 text-sm">{}</div>
            <div class="flex gap-4">{}</div>
        </div>
        <div class="text-center text-gray-500 text-sm mt-4">{}</div>
    </div>
</footer>"#, 
                if !contact_html.is_empty() { format!("<div class=\"mb-2\">{}</div>", contact_html) } else { String::new() },
                social_html,
                footer_text
            );

            let page = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT title, content FROM pages WHERE site_id = $1 AND slug = $2"
            )
            .bind(site_id)
            .bind(&page_slug)
            .fetch_optional(&state.db)
            .await;

            match page {
                Ok(Some((title, content))) => {
                    let content_html = render_blocks(&content);
                    let html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title><script src="https://cdn.tailwindcss.com"></script><link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"></head>
<body class="bg-gray-50">
{}
<div class="max-w-4xl mx-auto p-8">
<a href="/site/{}" class="text-blue-600">← Back</a>
<h1 class="text-4xl font-bold mt-4 mb-6">{}</h1>
<div class="prose">{}</div>
</div>
{}
</body></html>"#, title, header_html, slug, title, content_html, footer_html);

                    (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "text/html")], html)
                }
                _ => (StatusCode::NOT_FOUND, [(axum::http::header::CONTENT_TYPE, "text/plain")], "Page not found".to_string()),
            }
        }
        _ => (StatusCode::NOT_FOUND, [(axum::http::header::CONTENT_TYPE, "text/plain")], "Site not found".to_string()),
    }
}

async fn view_blog_at_path(
    axum::extract::Path((slug, path)): axum::extract::Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let clean_path = format!("/{}", path.trim_start_matches('/'));
    
    let site_row = sqlx::query(
        "SELECT id, name FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    let mut headers = HeaderMap::new();
    headers.insert(axum::http::header::CONTENT_TYPE, "text/html".parse().unwrap());

    match site_row {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");
            
            // Check if it's the blog path
            if clean_path == "/blog" {
                let (status, _, html) = view_blog_listing(&state, site_id, &slug, &name).await;
                return (status, headers, html);
            }
            
            // Otherwise try to find a page with this slug
            let page_slug = path.trim_start_matches('/');
            let page = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT title, content FROM pages WHERE site_id = $1 AND slug = $2"
            )
            .bind(site_id)
            .bind(page_slug)
            .fetch_optional(&state.db)
            .await;

            match page {
                Ok(Some((page_title, page_content))) => {
                    // Render the page
                    let (status, _, html) = view_page_content(&state, site_id, &slug, &name, &page_title, &page_content).await;
                    (status, headers, html)
                }
                _ => {
                    // Check for homepage_type to decide what to show
                    let site_info = sqlx::query(
                        "SELECT homepage_type FROM sites WHERE id = $1"
                    )
                    .bind(site_id)
                    .fetch_optional(&state.db)
                    .await;
                    
                    let homepage_type: String = site_info.ok().flatten().and_then(|sr| sr.get("homepage_type")).unwrap_or_else(|| "both".to_string());
                    
                    // If blog mode and root path, show blog
                    if clean_path == "/" && homepage_type == "blog" {
                        let (status, _, html) = view_blog_listing(&state, site_id, &slug, &name).await;
                        return (status, headers, html);
                    }
                    
                    (StatusCode::NOT_FOUND, headers, "Not found".to_string())
                }
            }
        }
        _ => (StatusCode::NOT_FOUND, headers, "Site not found".to_string()),
    }
}

// Helper function to render blog listing
async fn view_blog_listing(state: &AppState, site_id: Uuid, slug: &str, name: &str) -> (StatusCode, HeaderMap, String) {
    let settings_result = sqlx::query(
        "SELECT logo_url, nav_links, footer_text, social_links, contact_email, contact_phone, contact_address FROM sites WHERE id = $1"
    )
    .bind(site_id)
    .fetch_one(&state.db)
    .await;

    let logo_url: String = settings_result.as_ref().ok().and_then(|sr| sr.get("logo_url")).unwrap_or_default();
    let nav_links: serde_json::Value = settings_result.as_ref().ok().and_then(|sr| sr.get("nav_links")).unwrap_or(serde_json::Value::Array(vec![]));
    let footer_text: String = settings_result.as_ref().ok().and_then(|sr| sr.get("footer_text")).unwrap_or_default();
    let social_links: serde_json::Value = settings_result.as_ref().ok().and_then(|sr| sr.get("social_links")).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    let contact_email: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_email")).unwrap_or_default();
    let contact_phone: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_phone")).unwrap_or_default();
    let contact_address: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_address")).unwrap_or_default();
    let site_path = format!("/site/{}", slug);
    
    let nav_html = if let Some(links) = nav_links.as_array() {
        links.iter().map(|link| {
            let label = link.get("label").and_then(|l| l.as_str()).unwrap_or("");
            let url = link.get("url").and_then(|u| u.as_str()).unwrap_or("#");
            let full_url = if url.starts_with('/') {
                format!("{}{}", site_path, url)
            } else {
                url.to_string()
            };
            format!("<a href=\"{}\" class=\"text-gray-700 hover:text-blue-600 px-3\">{}</a>", full_url, label)
        }).collect::<Vec<_>>().join("")
    } else { String::new() };

    let logo_img = if !logo_url.is_empty() {
        format!("<img src=\"{}\" class=\"h-10 w-auto\">", logo_url)
    } else { String::new() };

    let header_html = format!(r#"
<header class="bg-white shadow-sm">
    <div class="max-w-4xl mx-auto px-6 py-4 flex items-center justify-between">
        <div class="flex items-center gap-4">
            {}<a href="/site/{}" class="text-xl font-bold text-gray-800">{}</a>
        </div>
        <nav class="flex items-center gap-2">{}</nav>
    </div>
</header>"#, logo_img, slug, name, nav_html);

    let social_html = if let Some(social) = social_links.as_object() {
        social.iter().filter_map(|(platform, url)| {
            let url_str = url.as_str()?;
            if url_str.is_empty() { return None; }
            let icon = match platform.as_str() {
                "x" => "fa-x-twitter",
                "facebook" => "fa-facebook", 
                "instagram" => "fa-instagram",
                "linkedin" => "fa-linkedin",
                "youtube" => "fa-youtube",
                "github" => "fa-github",
                "tiktok" => "fa-tiktok",
                _ => "fa-link"
            };
            Some(format!("<a href=\"{}\" target=\"_blank\" class=\"text-gray-500 hover:text-gray-700\"><i class=\"fab {}\"></i></a>", url_str, icon))
        }).collect::<Vec<_>>().join(" ")
    } else { String::new() };

    let mut contact_parts = Vec::new();
    if !contact_phone.is_empty() { contact_parts.push(contact_phone); }
    if !contact_email.is_empty() { contact_parts.push(format!("<a href=\"mailto:{}\">{}</a>", contact_email, contact_email)); }
    if !contact_address.is_empty() { contact_parts.push(contact_address); }
    let contact_html = contact_parts.join(" | ");

    let footer_html = format!(r#"
<footer class="bg-gray-100 mt-16">
    <div class="max-w-4xl mx-auto px-6 py-8">
        <div class="flex flex-col md:flex-row justify-between items-center gap-4">
            <div class="text-gray-600 text-sm">{}</div>
            <div class="flex gap-4">{}</div>
        </div>
        <div class="text-center text-gray-500 text-sm mt-4">{}</div>
    </div>
</footer>"#, 
        if !contact_html.is_empty() { format!("<div class=\"mb-2\">{}</div>", contact_html) } else { String::new() },
        social_html,
        footer_text
    );

    let posts = sqlx::query_as::<_, (String, String, Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT title, slug, excerpt, published_at FROM posts WHERE site_id = $1 AND status = 'published' ORDER BY published_at DESC LIMIT 20"
    )
    .bind(site_id)
    .fetch_all(&state.db)
    .await;

    let posts_html = match posts {
        Ok(posts) => posts.iter().map(|p| format!(
            r#"<article class="mb-8"><h2 class="text-2xl font-bold mb-2"><a href="/site/{}/post/{}" class="text-blue-600">{}</a></h2><p class="text-gray-600">{}</p></article>"#,
            slug, p.1, p.0, p.2.as_deref().unwrap_or("")
        )).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    };

    let html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>Blog - {}</title><script src="https://cdn.tailwindcss.com"></script><link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"></head>
<body class="bg-gray-50">
{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-8">Blog</h1>
{}
</div>
{}
</body></html>"#, name, header_html, posts_html, footer_html);

    let mut headers = HeaderMap::new();
    headers.insert(axum::http::header::CONTENT_TYPE, "text/html".parse().unwrap());
    (StatusCode::OK, headers, html)
}

// Helper function to render page content
async fn view_page_content(state: &AppState, site_id: Uuid, slug: &str, name: &str, page_title: &str, page_content: &serde_json::Value) -> (StatusCode, HeaderMap, String) {
    let settings_result = sqlx::query(
        "SELECT logo_url, nav_links, footer_text, social_links, contact_email, contact_phone, contact_address FROM sites WHERE id = $1"
    )
    .bind(site_id)
    .fetch_one(&state.db)
    .await;

    let logo_url: String = settings_result.as_ref().ok().and_then(|sr| sr.get("logo_url")).unwrap_or_default();
    let nav_links: serde_json::Value = settings_result.as_ref().ok().and_then(|sr| sr.get("nav_links")).unwrap_or(serde_json::Value::Array(vec![]));
    let footer_text: String = settings_result.as_ref().ok().and_then(|sr| sr.get("footer_text")).unwrap_or_default();
    let social_links: serde_json::Value = settings_result.as_ref().ok().and_then(|sr| sr.get("social_links")).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    let contact_email: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_email")).unwrap_or_default();
    let contact_phone: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_phone")).unwrap_or_default();
    let contact_address: String = settings_result.as_ref().ok().and_then(|sr| sr.get("contact_address")).unwrap_or_default();
    let site_path = format!("/site/{}", slug);
    
    let nav_html = if let Some(links) = nav_links.as_array() {
        links.iter().map(|link| {
            let label = link.get("label").and_then(|l| l.as_str()).unwrap_or("");
            let url = link.get("url").and_then(|u| u.as_str()).unwrap_or("#");
            let full_url = if url.starts_with('/') {
                format!("{}{}", site_path, url)
            } else {
                url.to_string()
            };
            format!("<a href=\"{}\" class=\"text-gray-700 hover:text-blue-600 px-3\">{}</a>", full_url, label)
        }).collect::<Vec<_>>().join("")
    } else { String::new() };

    let logo_img = if !logo_url.is_empty() {
        format!("<img src=\"{}\" class=\"h-10 w-auto\">", logo_url)
    } else { String::new() };

    let header_html = format!(r#"
<header class="bg-white shadow-sm">
    <div class="max-w-4xl mx-auto px-6 py-4 flex items-center justify-between">
        <div class="flex items-center gap-4">
            {}<a href="/site/{}" class="text-xl font-bold text-gray-800">{}</a>
        </div>
        <nav class="flex items-center gap-2">{}</nav>
    </div>
</header>"#, logo_img, slug, name, nav_html);

    let social_html = if let Some(social) = social_links.as_object() {
        social.iter().filter_map(|(platform, url)| {
            let url_str = url.as_str()?;
            if url_str.is_empty() { return None; }
            let icon = match platform.as_str() {
                "x" => "fa-x-twitter",
                "facebook" => "fa-facebook", 
                "instagram" => "fa-instagram",
                "linkedin" => "fa-linkedin",
                "youtube" => "fa-youtube",
                "github" => "fa-github",
                "tiktok" => "fa-tiktok",
                _ => "fa-link"
            };
            Some(format!("<a href=\"{}\" target=\"_blank\" class=\"text-gray-500 hover:text-gray-700\"><i class=\"fab {}\"></i></a>", url_str, icon))
        }).collect::<Vec<_>>().join(" ")
    } else { String::new() };

    let mut contact_parts = Vec::new();
    if !contact_phone.is_empty() { contact_parts.push(contact_phone); }
    if !contact_email.is_empty() { contact_parts.push(format!("<a href=\"mailto:{}\">{}</a>", contact_email, contact_email)); }
    if !contact_address.is_empty() { contact_parts.push(contact_address); }
    let contact_html = contact_parts.join(" | ");

    let footer_html = format!(r#"
<footer class="bg-gray-100 mt-16">
    <div class="max-w-4xl mx-auto px-6 py-8">
        <div class="flex flex-col md:flex-row justify-between items-center gap-4">
            <div class="text-gray-600 text-sm">{}</div>
            <div class="flex gap-4">{}</div>
        </div>
        <div class="text-center text-gray-500 text-sm mt-4">{}</div>
    </div>
</footer>"#, 
        if !contact_html.is_empty() { format!("<div class=\"mb-2\">{}</div>", contact_html) } else { String::new() },
        social_html,
        footer_text
    );

    let content_html = render_blocks(page_content);
    let html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title><script src="https://cdn.tailwindcss.com"></script><link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css"></head>
<body class="bg-gray-50">
{}
<div class="max-w-4xl mx-auto p-8">
<a href="/site/{}" class="text-blue-600">← Back</a>
<h1 class="text-4xl font-bold mt-4 mb-6">{}</h1>
<div class="prose">{}</div>
</div>
{}
</body></html>"#, page_title, header_html, slug, page_title, content_html, footer_html);

    let mut headers = HeaderMap::new();
    headers.insert(axum::http::header::CONTENT_TYPE, "text/html".parse().unwrap());
    (StatusCode::OK, headers, html)
}
