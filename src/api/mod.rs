pub mod auth;
pub mod media;
pub mod pages;
pub mod posts;
pub mod sites;

use crate::{ApiError, AppState};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
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
