use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub id: Uuid,
    pub subdomain: Option<String>,
    pub custom_domain: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    pub theme: String,
    pub nav_links: serde_json::Value,
    pub footer_text: Option<String>,
    pub social_links: serde_json::Value,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub contact_address: Option<String>,
    pub homepage_type: String,
    pub blog_path: Option<String>,
    pub blog_sort_order: i32,
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
    pub show_in_nav: bool,
    pub sort_order: i32,
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

// ============================================================================
// SQL Query Row Types
// These type aliases are used with sqlx::query_as to map SQL results to tuples
// ============================================================================

/// Row type for Post queries: SELECT id, site_id, author_id, title, slug, content,
/// excerpt, featured_image, status, published_at, created_at, updated_at, seo
pub type PostRow = (
    Uuid,
    Uuid,
    Option<Uuid>,
    String,
    String,
    serde_json::Value,
    Option<String>,
    Option<String>,
    String,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
    DateTime<Utc>,
    serde_json::Value,
);

/// Row type for Page queries: SELECT id, site_id, title, slug, content,
/// is_homepage, show_in_nav, sort_order, created_at, updated_at, seo
pub type PageRow = (
    Uuid,
    Uuid,
    String,
    String,
    serde_json::Value,
    bool,
    bool,
    i32,
    DateTime<Utc>,
    DateTime<Utc>,
    serde_json::Value,
);

/// Row type for Media queries: SELECT id, site_id, filename, mime_type,
/// size, url, alt_text, created_at
pub type MediaRow = (
    Uuid,
    Uuid,
    String,
    Option<String>,
    Option<i32>,
    String,
    Option<String>,
    DateTime<Utc>,
);

/// Row type for ContactSubmission queries: SELECT id, site_id, name, email,
/// message, created_at, read
pub type ContactSubmissionRow = (Uuid, Uuid, String, String, String, DateTime<Utc>, bool);
