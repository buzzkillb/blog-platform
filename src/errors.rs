use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
    #[serde(skip)]
    pub status_code: StatusCode,
}

impl ApiError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_code: StatusCode::UNAUTHORIZED,
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_code: StatusCode::FORBIDDEN,
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = serde_json::json!({
            "message": self.message,
            "status_code": self.status_code.as_u16()
        });
        (self.status_code, axum::Json(body)).into_response()
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
    pub show_in_nav: Option<bool>,
    pub sort_order: Option<i32>,
    pub seo: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePageRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub content: Option<serde_json::Value>,
    pub is_homepage: Option<bool>,
    pub show_in_nav: Option<bool>,
    pub sort_order: Option<i32>,
    pub seo: Option<serde_json::Value>,
}
