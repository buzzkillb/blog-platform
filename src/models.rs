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
