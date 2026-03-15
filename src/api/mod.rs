pub mod auth;
pub mod media;
pub mod pages;
pub mod posts;
pub mod sites;
pub mod templates;

use crate::{ApiError, AppState};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{delete, get, post, put},
    Json, Router,
};

pub fn routes() -> Router<crate::AppState> {
    Router::new()
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/sites", get(sites::list).post(sites::create))
        .route(
            "/api/sites/{id}",
            get(sites::get).put(sites::update).delete(sites::delete),
        )
        .route(
            "/api/sites/{site_id}/posts",
            get(posts::list).post(posts::create),
        )
        .route(
            "/api/sites/{site_id}/posts/{id}",
            get(posts::get).put(posts::update).delete(posts::delete),
        )
        .route(
            "/api/sites/{site_id}/posts/{id}/publish",
            post(posts::publish),
        )
        .route(
            "/api/sites/{site_id}/pages",
            get(pages::list).post(pages::create),
        )
        .route(
            "/api/sites/{site_id}/pages/{id}",
            get(pages::get).put(pages::update).delete(pages::delete),
        )
        .route(
            "/api/sites/{site_id}/media",
            get(media::list).post(media::upload),
        )
        .route(
            "/api/sites/{site_id}/media/{id}",
            get(media::get).delete(media::delete),
        )
        .route("/api/sites/{site_id}/contact", post(sites::submit_contact))
        .route(
            "/api/sites/{site_id}/contact",
            get(sites::list_contact_submissions),
        )
        .route("/api/sites/{site_id}/build", post(build_site))
        .route("/api/sites/{site_id}/deploy", post(deploy_pages))
        .route("/api/templates", get(templates::list_templates).post(templates::create_template))
        .route(
            "/api/templates/{id}",
            get(templates::get_template).put(templates::update_template).delete(templates::delete_template),
        )
        .route(
            "/api/sites/{site_id}/template",
            get(templates::get_site_template).put(templates::assign_template),
        )
        .route("/api/sites/{site_id}/theme", put(templates::update_theme))
        .route(
            "/api/sites/{site_id}/template-config",
            put(templates::update_template_config),
        )
}

async fn build_site(
    Path(site_id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let current_user = auth::require_auth(State(state.clone()), headers)
        .await
        .map_err(|e| ApiError::new(e.message))?;

    auth::require_site_member(&state, site_id, current_user.user_id)
        .await
        .map_err(|e| ApiError::new(e.message))?;

    // Build the site synchronously so we can return result
    let db = state.db.clone();
    match crate::ssg::build_site(&db, site_id).await {
        Ok(_) => Ok(Json(
            serde_json::json!({ "message": "Site built successfully" }),
        )),
        Err(e) => {
            tracing::error!("Failed to build site: {}", e);
            Err(ApiError::new(format!("Failed to build site: {}", e)))
        }
    }
}

async fn deploy_pages(
    Path(site_id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let current_user = auth::require_auth(State(state.clone()), headers)
        .await
        .map_err(|e| ApiError::new(e.message))?;

    auth::require_site_member(&state, site_id, current_user.user_id)
        .await
        .map_err(|e| ApiError::new(e.message))?;

    // Check if Cloudflare credentials are configured
    if std::env::var("CLOUDFLARE_ACCOUNT_ID").is_err()
        || std::env::var("CLOUDFLARE_API_TOKEN").is_err()
        || std::env::var("CLOUDFLARE_PAGES_PROJECT").is_err()
    {
        return Err(ApiError::new(
            "Cloudflare Pages not configured. Set CLOUDFLARE_ACCOUNT_ID, CLOUDFLARE_API_TOKEN, and CLOUDFLARE_PAGES_PROJECT environment variables.".to_string()
        ));
    }

    // First build the site, then deploy
    let db = state.db.clone();
    if let Err(e) = crate::ssg::build_site(&db, site_id).await {
        tracing::error!("Failed to build site: {}", e);
        return Err(ApiError::new(format!("Failed to build site: {}", e)));
    }

    // Deploy to Cloudflare Pages
    match crate::ssg::deploy_to_cloudflare().await {
        Ok(message) => Ok(Json(serde_json::json!({ "message": message }))),
        Err(e) => {
            tracing::error!("Failed to deploy to Cloudflare: {}", e);
            Err(ApiError::new(format!("Failed to deploy: {}", e)))
        }
    }
}
