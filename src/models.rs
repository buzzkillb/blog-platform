use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
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
pub struct SiteSettings {
    pub logo_url: String,
    pub nav_links: serde_json::Value,
    pub footer_text: String,
    pub social_links: serde_json::Value,
    pub contact_email: String,
    pub contact_phone: String,
    pub contact_address: String,
}

impl SiteSettings {
    pub fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        Self {
            logo_url: row.get::<Option<String>, _>("logo_url").unwrap_or_default(),
            nav_links: row.get::<serde_json::Value, _>("nav_links"),
            footer_text: row
                .get::<Option<String>, _>("footer_text")
                .unwrap_or_default(),
            social_links: row.get::<serde_json::Value, _>("social_links"),
            contact_email: row
                .get::<Option<String>, _>("contact_email")
                .unwrap_or_default(),
            contact_phone: row
                .get::<Option<String>, _>("contact_phone")
                .unwrap_or_default(),
            contact_address: row
                .get::<Option<String>, _>("contact_address")
                .unwrap_or_default(),
        }
    }
}
