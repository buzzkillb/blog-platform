use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::api::auth::{require_auth, require_site_member};
use crate::{util::generate_slug, ApiError, AppState, CreatePageRequest, Page, UpdatePageRequest};

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(site_id): Path<Uuid>,
) -> Result<Json<Vec<Page>>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    let pages = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            String,
            String,
            serde_json::Value,
            bool,
            bool,
            i32,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            serde_json::Value,
        ),
    >(
        "SELECT id, site_id, title, slug, content, is_homepage, show_in_nav, sort_order, created_at, updated_at, seo 
         FROM pages WHERE site_id = $1 ORDER BY is_homepage DESC, sort_order ASC, created_at DESC",
    )
    .bind(site_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to fetch pages: {}", e)))?;

    let pages: Vec<Page> = pages
        .into_iter()
        .map(|p| Page {
            id: p.0,
            site_id: p.1,
            title: p.2,
            slug: p.3,
            content: p.4,
            is_homepage: p.5,
            show_in_nav: p.6,
            sort_order: p.7,
            created_at: p.8,
            updated_at: p.9,
            seo: p.10,
        })
        .collect();

    Ok(Json(pages))
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((site_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Page>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    let page = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            String,
            String,
            serde_json::Value,
            bool,
            bool,
            i32,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            serde_json::Value,
        ),
    >(
        "SELECT id, site_id, title, slug, content, is_homepage, show_in_nav, sort_order, created_at, updated_at, seo 
         FROM pages WHERE site_id = $1 AND id = $2",
    )
    .bind(site_id)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Page not found"))?;

    Ok(Json(Page {
        id: page.0,
        site_id: page.1,
        title: page.2,
        slug: page.3,
        content: page.4,
        is_homepage: page.5,
        show_in_nav: page.6,
        sort_order: page.7,
        created_at: page.8,
        updated_at: page.9,
        seo: page.10,
    }))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(site_id): Path<Uuid>,
    Json(payload): Json<CreatePageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    if payload.title.is_empty() {
        return Err(ApiError::new("Title is required"));
    }

    let slug = payload
        .slug
        .unwrap_or_else(|| generate_slug(&payload.title));

    let is_homepage = payload.is_homepage.unwrap_or(false);
    let show_in_nav = payload.show_in_nav.unwrap_or(true);
    let sort_order = payload.sort_order.unwrap_or(0);

    if is_homepage {
        sqlx::query("UPDATE pages SET is_homepage = false WHERE site_id = $1")
            .bind(site_id)
            .execute(&state.db)
            .await
            .ok();
    }

    let content = payload.content.clone();
    let seo = payload.seo.clone().unwrap_or(serde_json::json!({}));

    let result = sqlx::query_as::<_, (
        Uuid, Uuid, String, String, serde_json::Value, bool,
        bool, i32, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, serde_json::Value
    )>(
        "INSERT INTO pages (site_id, title, slug, content, is_homepage, show_in_nav, sort_order, seo) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) 
         RETURNING id, site_id, title, slug, content, is_homepage, show_in_nav, sort_order, created_at, updated_at, seo"
    )
    .bind(site_id)
    .bind(&payload.title)
    .bind(&slug)
    .bind(&content)
    .bind(is_homepage)
    .bind(show_in_nav)
    .bind(sort_order)
    .bind(&seo)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to create page: {}", e)))?;

    let page = Page {
        id: result.0,
        site_id: result.1,
        title: result.2,
        slug: result.3,
        content: result.4,
        is_homepage: result.5,
        show_in_nav: result.6,
        sort_order: result.7,
        created_at: result.8,
        updated_at: result.9,
        seo: result.10,
    };

    Ok((StatusCode::CREATED, Json(page)))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((site_id, id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdatePageRequest>,
) -> Result<Json<Page>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    if payload.is_homepage == Some(true) {
        sqlx::query("UPDATE pages SET is_homepage = false WHERE site_id = $1 AND id != $2")
            .bind(site_id)
            .bind(id)
            .execute(&state.db)
            .await
            .ok();
    }

    let title = payload.title.clone();
    let content = payload.content.clone();
    let is_homepage = payload.is_homepage;
    let show_in_nav = payload.show_in_nav;
    let sort_order = payload.sort_order;
    let seo = payload.seo.clone();

    let result = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            String,
            String,
            serde_json::Value,
            bool,
            bool,
            i32,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            serde_json::Value,
        ),
    >(
        "UPDATE pages SET 
            title = COALESCE($3, title),
            content = COALESCE($4, content),
            is_homepage = COALESCE($5, is_homepage),
            show_in_nav = COALESCE($6, show_in_nav),
            sort_order = COALESCE($7, sort_order),
            seo = COALESCE($8, seo),
            updated_at = NOW()
         WHERE site_id = $1 AND id = $2
         RETURNING id, site_id, title, slug, content, is_homepage, show_in_nav, sort_order, created_at, updated_at, seo",
    )
    .bind(site_id)
    .bind(id)
    .bind(title)
    .bind(content)
    .bind(is_homepage)
    .bind(show_in_nav)
    .bind(sort_order)
    .bind(seo)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Page not found"))?;

    Ok(Json(Page {
        id: result.0,
        site_id: result.1,
        title: result.2,
        slug: result.3,
        content: result.4,
        is_homepage: result.5,
        show_in_nav: result.6,
        sort_order: result.7,
        created_at: result.8,
        updated_at: result.9,
        seo: result.10,
    }))
}

pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((site_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    sqlx::query("DELETE FROM pages WHERE site_id = $1 AND id = $2")
        .bind(site_id)
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::new(format!("Failed to delete page: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}
