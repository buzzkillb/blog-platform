use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use sqlx::Row;
use uuid::Uuid;

use crate::api::auth::{require_auth, require_site_member};
use crate::{ApiError, AppState, ContactSubmission, CreateSiteRequest, Site};

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Site>>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;

    let rows = sqlx::query(
        "SELECT id, subdomain, custom_domain, name, description, logo_url, favicon_url, theme, nav_links, footer_text, social_links, contact_phone, contact_email, contact_address, homepage_type, blog_path, landing_blocks, settings, created_at FROM sites WHERE id IN (SELECT site_id FROM site_members WHERE user_id = $1) ORDER BY created_at DESC"
    )
    .bind(current_user.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to fetch sites: {}", e)))?;

    let mut sites_vec = Vec::new();
    for row in rows {
        sites_vec.push(Site {
            id: row.get("id"),
            subdomain: row.get("subdomain"),
            custom_domain: row.get("custom_domain"),
            name: row.get("name"),
            description: row.get("description"),
            logo_url: row.get("logo_url"),
            favicon_url: row.get("favicon_url"),
            theme: row.get("theme"),
            nav_links: row.get("nav_links"),
            footer_text: row.get("footer_text"),
            social_links: row.get("social_links"),
            contact_phone: row.get("contact_phone"),
            contact_email: row.get("contact_email"),
            contact_address: row.get("contact_address"),
            homepage_type: row.get("homepage_type"),
            blog_path: row.get("blog_path"),
            landing_blocks: row.get("landing_blocks"),
            settings: row.get("settings"),
            created_at: row.get("created_at"),
        });
    }

    Ok(Json(sites_vec))
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Site>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, id, current_user.user_id).await?;

    let row = sqlx::query(
        "SELECT id, subdomain, custom_domain, name, description, logo_url, favicon_url, theme, nav_links, footer_text, social_links, contact_phone, contact_email, contact_address, homepage_type, blog_path, landing_blocks, settings, created_at FROM sites WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Site not found"))?;

    let site = Site {
        id: row.get("id"),
        subdomain: row.get("subdomain"),
        custom_domain: row.get("custom_domain"),
        name: row.get("name"),
        description: row.get("description"),
        logo_url: row.get("logo_url"),
        favicon_url: row.get("favicon_url"),
        theme: row.get("theme"),
        nav_links: row.get("nav_links"),
        footer_text: row.get("footer_text"),
        social_links: row.get("social_links"),
        contact_phone: row.get("contact_phone"),
        contact_email: row.get("contact_email"),
        contact_address: row.get("contact_address"),
        homepage_type: row.get("homepage_type"),
        blog_path: row.get("blog_path"),
        landing_blocks: row.get("landing_blocks"),
        settings: row.get("settings"),
        created_at: row.get("created_at"),
    };

    Ok(Json(site))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateSiteRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;

    if payload.name.is_empty() {
        return Err(ApiError::new("Site name is required"));
    }

    let subdomain = payload.subdomain.filter(|s| !s.is_empty());
    let custom_domain = payload.custom_domain.filter(|s| !s.is_empty());

    let row = sqlx::query(
        "INSERT INTO sites (subdomain, custom_domain, name, description, logo_url, favicon_url, homepage_type, nav_links, blog_path) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id, subdomain, custom_domain, name, description, logo_url, favicon_url, theme, nav_links, footer_text, social_links, contact_phone, contact_email, contact_address, homepage_type, landing_blocks, settings, created_at, blog_path"
    )
    .bind(subdomain)
    .bind(custom_domain)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.logo_url)
    .bind(&payload.logo_url)
    .bind("both")
    .bind(serde_json::json!([{"label": "Home", "url": "/"}, {"label": "Blog", "url": "/blog"}, {"label": "About", "url": "/about"}, {"label": "Contact", "url": "/contact"}]))
    .bind("/blog")
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to create site: {}", e)))?;

    let site_id: Uuid = row.get("id");
    let site_name = &payload.name;

    // Create default pages: Homepage, About, Contact
    let homepage_content = serde_json::json!([
        {"block_type": "hero", "content": {"title": format!("Welcome to {}", site_name), "subtitle": "Your amazing blog starts here", "ctaText": "Read More", "ctaLink": "/blog"}}
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
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to create homepage: {}", e)))?;

    // Insert About page
    sqlx::query(
        "INSERT INTO pages (site_id, title, slug, content, is_homepage) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(site_id)
    .bind("About")
    .bind("about")
    .bind(&about_content)
    .bind(false)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to create about page: {}", e)))?;

    // Insert Contact page
    sqlx::query(
        "INSERT INTO pages (site_id, title, slug, content, is_homepage) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(site_id)
    .bind("Contact")
    .bind("contact")
    .bind(&contact_content)
    .bind(false)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to create contact page: {}", e)))?;

    // Add creator as admin member of the site
    sqlx::query("INSERT INTO site_members (site_id, user_id, role) VALUES ($1, $2, 'admin')")
        .bind(site_id)
        .bind(current_user.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::new(format!("Failed to add site member: {}", e)))?;

    let site = Site {
        id: row.get("id"),
        subdomain: row.get("subdomain"),
        custom_domain: row.get("custom_domain"),
        name: row.get("name"),
        description: row.get("description"),
        logo_url: row.get("logo_url"),
        favicon_url: row.get("favicon_url"),
        theme: row.get("theme"),
        nav_links: row.get("nav_links"),
        footer_text: row.get("footer_text"),
        social_links: row.get("social_links"),
        contact_phone: row.get("contact_phone"),
        contact_email: row.get("contact_email"),
        contact_address: row.get("contact_address"),
        homepage_type: row.get("homepage_type"),
        blog_path: row.get("blog_path"),
        landing_blocks: row.get("landing_blocks"),
        settings: row.get("settings"),
        created_at: row.get("created_at"),
    };

    Ok((StatusCode::CREATED, Json(site)))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<Site>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, id, current_user.user_id).await?;

    let name = payload.get("name").and_then(|v| v.as_str());
    let description = payload.get("description").and_then(|v| v.as_str());
    let subdomain = payload
        .get("subdomain")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let custom_domain = payload
        .get("custom_domain")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let logo_url = payload.get("logo_url").and_then(|v| v.as_str());
    let theme = payload.get("theme").and_then(|v| v.as_str());
    let nav_links = payload.get("nav_links");
    let footer_text = payload.get("footer_text").and_then(|v| v.as_str());
    let social_links = payload.get("social_links");
    let contact_phone = payload.get("contact_phone").and_then(|v| v.as_str());
    let contact_email = payload.get("contact_email").and_then(|v| v.as_str());
    let contact_address = payload.get("contact_address").and_then(|v| v.as_str());
    let homepage_type = payload.get("homepage_type").and_then(|v| v.as_str());
    let blog_path = payload.get("blog_path").and_then(|v| v.as_str());
    let landing_blocks = payload.get("landing_blocks");
    let settings = payload.get("settings");
    let favicon_url = payload.get("favicon_url").and_then(|v| v.as_str());

    let row = sqlx::query(
        "UPDATE sites SET 
            name = COALESCE($2, name), 
            description = COALESCE($3, description), 
            subdomain = COALESCE($4, subdomain),
            custom_domain = COALESCE($5, custom_domain),
            logo_url = COALESCE($6, logo_url), 
            favicon_url = COALESCE($18, favicon_url),
            theme = COALESCE($7, theme),
            nav_links = COALESCE($8, nav_links),
            footer_text = COALESCE($9, footer_text),
            social_links = COALESCE($10, social_links),
            contact_phone = COALESCE($11, contact_phone),
            contact_email = COALESCE($12, contact_email),
            contact_address = COALESCE($13, contact_address),
            homepage_type = COALESCE($14, homepage_type),
            blog_path = $15,
            landing_blocks = COALESCE($16, landing_blocks),
            settings = COALESCE($17, settings)
         WHERE id = $1 
         RETURNING id, subdomain, custom_domain, name, description, logo_url, favicon_url, theme, nav_links, footer_text, social_links, contact_phone, contact_email, contact_address, homepage_type, blog_path, landing_blocks, settings, created_at"
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(subdomain)
    .bind(custom_domain)
    .bind(logo_url)
    .bind(theme)
    .bind(nav_links)
    .bind(footer_text)
    .bind(social_links)
    .bind(contact_phone)
    .bind(contact_email)
    .bind(contact_address)
    .bind(homepage_type)
    .bind(blog_path)
    .bind(landing_blocks)
    .bind(settings)
    .bind(favicon_url)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Site not found"))?;

    Ok(Json(Site {
        id: row.get("id"),
        subdomain: row.get("subdomain"),
        custom_domain: row.get("custom_domain"),
        name: row.get("name"),
        description: row.get("description"),
        logo_url: row.get("logo_url"),
        favicon_url: row.get("favicon_url"),
        theme: row.get("theme"),
        nav_links: row.get("nav_links"),
        footer_text: row.get("footer_text"),
        social_links: row.get("social_links"),
        contact_phone: row.get("contact_phone"),
        contact_email: row.get("contact_email"),
        contact_address: row.get("contact_address"),
        homepage_type: row.get("homepage_type"),
        blog_path: row.get("blog_path"),
        landing_blocks: row.get("landing_blocks"),
        settings: row.get("settings"),
        created_at: row.get("created_at"),
    }))
}

pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, id, current_user.user_id).await?;

    sqlx::query("DELETE FROM sites WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::new(format!("Failed to delete site: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn submit_contact(
    State(state): State<AppState>,
    Path(site_id): Path<Uuid>,
    Json(payload): Json<crate::ContactFormRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.honeypot.is_some() && !payload.honeypot.as_ref().unwrap().is_empty() {
        return Ok(StatusCode::OK);
    }

    if payload.name.is_empty() || payload.email.is_empty() || payload.message.is_empty() {
        return Err(ApiError::new("Name, email, and message are required"));
    }

    sqlx::query(
        "INSERT INTO contact_submissions (site_id, name, email, message, honeypot) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(site_id)
    .bind(&payload.name)
    .bind(&payload.email)
    .bind(&payload.message)
    .bind(&payload.honeypot)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to save submission: {}", e)))?;

    Ok(StatusCode::CREATED)
}

pub async fn list_contact_submissions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(site_id): Path<Uuid>,
) -> Result<Json<Vec<ContactSubmission>>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    let submissions = sqlx::query_as::<_, (
        Uuid, Uuid, String, String, String, chrono::DateTime<chrono::Utc>, bool
    )>(
        "SELECT id, site_id, name, email, message, created_at, read FROM contact_submissions WHERE site_id = $1 ORDER BY created_at DESC"
    )
    .bind(site_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to fetch submissions: {}", e)))?;

    let submissions: Vec<ContactSubmission> = submissions
        .into_iter()
        .map(|s| ContactSubmission {
            id: s.0,
            site_id: s.1,
            name: s.2,
            email: s.3,
            message: s.4,
            created_at: s.5,
            read: s.6,
        })
        .collect();

    Ok(Json(submissions))
}
