use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}

impl ApiError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.message).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSiteRequest {
    pub subdomain: String,
    pub custom_domain: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub site_id: Option<Uuid>,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub slug: Option<String>,
    pub content: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image: Option<String>,
    pub status: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub seo: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub content: Option<serde_json::Value>,
    pub excerpt: Option<String>,
    pub featured_image: Option<String>,
    pub status: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub seo: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePageRequest {
    pub title: String,
    pub slug: Option<String>,
    pub content: Option<serde_json::Value>,
    pub is_homepage: Option<bool>,
    pub seo: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePageRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub content: Option<serde_json::Value>,
    pub is_homepage: Option<bool>,
    pub seo: Option<serde_json::Value>,
}
