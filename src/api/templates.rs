use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use sqlx::Row;
use uuid::Uuid;

use crate::api::auth::{require_auth, require_site_member};
use crate::errors::{
    ApiError, CreateTemplateRequest, TemplateResponse, UpdateTemplateRequest,
};
use crate::models::TemplateListItem;
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/templates", get(list_templates).post(create_template))
        .route(
            "/api/templates/{id}",
            get(get_template).put(update_template).delete(delete_template),
        )
        .route("/api/sites/{site_id}/template", get(get_site_template).put(assign_template))
        .route("/api/sites/{site_id}/theme", put(update_theme))
        .route(
            "/api/sites/{site_id}/template-config",
            put(update_template_config),
        )
}

pub async fn list_templates(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TemplateListItem>>, ApiError> {
    let _ = require_auth(State(state.clone()), headers).await?;

    let rows = sqlx::query(
        "SELECT id, name, description, category, thumbnail_url, is_builtin FROM templates ORDER BY is_builtin DESC, name ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to fetch templates: {}", e)))?;

    let templates: Vec<TemplateListItem> = rows
        .iter()
        .map(|row| TemplateListItem {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            category: row.get("category"),
            thumbnail_url: row.get("thumbnail_url"),
            is_builtin: row.get("is_builtin"),
        })
        .collect();

    Ok(Json(templates))
}

pub async fn get_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<TemplateResponse>, ApiError> {
    let _ = require_auth(State(state.clone()), headers).await?;

    let row = sqlx::query(
        "SELECT id, name, description, category, thumbnail_url, html_content, css_content, js_content, default_config, is_builtin, created_at, updated_at FROM templates WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Template not found"))?;

    let template = template_from_row(row)?;

    Ok(Json(template))
}

pub async fn create_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;

    if payload.name.is_empty() {
        return Err(ApiError::new("Template name is required"));
    }

    let default_config = payload.default_config.unwrap_or(serde_json::json!({}));

    let row = sqlx::query(
        "INSERT INTO templates (name, description, category, thumbnail_url, html_content, css_content, js_content, default_config, is_builtin) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false) RETURNING id, name, description, category, thumbnail_url, html_content, css_content, js_content, default_config, is_builtin, created_at, updated_at",
    )
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.category)
    .bind(&payload.thumbnail_url)
    .bind(&payload.html_content)
    .bind(&payload.css_content)
    .bind(&payload.js_content)
    .bind(&default_config)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to create template: {}", e)))?;

    let template = template_from_row(row)?;

    tracing::info!(
        "User {} created template {}",
        current_user.user_id,
        template.id
    );

    Ok((axum::http::StatusCode::CREATED, Json(template)))
}

pub async fn update_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTemplateRequest>,
) -> Result<Json<TemplateResponse>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;

    let is_builtin: bool = sqlx::query("SELECT is_builtin FROM templates WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::new("Template not found"))?
        .get("is_builtin");

    if is_builtin {
        return Err(ApiError::new("Cannot modify builtin templates"));
    }

    let name = payload.name.as_ref().filter(|s| !s.is_empty());
    let description = payload.description.as_ref().filter(|s| !s.is_empty());
    let category = payload.category.as_ref().filter(|s| !s.is_empty());
    let thumbnail_url = payload.thumbnail_url.as_ref().filter(|s| !s.is_empty());
    let html_content = payload.html_content.as_ref().filter(|s| !s.is_empty());
    let css_content = payload.css_content.as_ref().filter(|s| !s.is_empty());
    let js_content = payload.js_content.as_ref().filter(|s| !s.is_empty());
    let default_config = payload.default_config.as_ref();

    let row = sqlx::query(
        "UPDATE templates SET 
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            category = COALESCE($4, category),
            thumbnail_url = COALESCE($5, thumbnail_url),
            html_content = COALESCE($6, html_content),
            css_content = COALESCE($7, css_content),
            js_content = COALESCE($8, js_content),
            default_config = COALESCE($9, default_config),
            updated_at = NOW()
         WHERE id = $1
         RETURNING id, name, description, category, thumbnail_url, html_content, css_content, js_content, default_config, is_builtin, created_at, updated_at",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(category)
    .bind(thumbnail_url)
    .bind(html_content)
    .bind(css_content)
    .bind(js_content)
    .bind(default_config)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Template not found"))?;

    let template = template_from_row(row)?;

    tracing::info!(
        "User {} updated template {}",
        current_user.user_id,
        template.id
    );

    Ok(Json(template))
}

pub async fn delete_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;

    let is_builtin: bool = sqlx::query("SELECT is_builtin FROM templates WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| ApiError::new("Template not found"))?
        .get("is_builtin");

    if is_builtin {
        return Err(ApiError::new("Cannot delete builtin templates"));
    }

    let affected = sqlx::query("DELETE FROM templates WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::new(format!("Failed to delete template: {}", e)))?
        .rows_affected();

    if affected == 0 {
        return Err(ApiError::new("Template not found"));
    }

    tracing::info!(
        "User {} deleted template {}",
        current_user.user_id,
        id
    );

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn assign_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(site_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    let template_id = payload
        .get("template_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    if let Some(ref tid) = template_id {
        let exists: bool = sqlx::query("SELECT EXISTS(SELECT 1 FROM templates WHERE id = $1)")
            .bind(tid)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::new(format!("Failed to verify template: {}", e)))?
            .get("exists");

        if !exists {
            return Err(ApiError::new("Template not found"));
        }
    }

    let new_template_id: Option<Uuid> = if template_id.is_some() {
        template_id
    } else {
        None
    };

    sqlx::query("UPDATE sites SET template_id = $2 WHERE id = $1")
        .bind(site_id)
        .bind(new_template_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::new(format!("Failed to assign template: {}", e)))?;

    tracing::info!(
        "User {} assigned template {:?} to site {}",
        current_user.user_id,
        new_template_id,
        site_id
    );

    Ok(Json(serde_json::json!({
        "message": "Template assigned successfully",
        "template_id": new_template_id
    })))
}

pub async fn get_site_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(site_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    let row = sqlx::query(
        "SELECT t.id, t.name, t.description, t.category, t.thumbnail_url, t.html_content, t.css_content, t.js_content, t.default_config, t.is_builtin, t.created_at, t.updated_at, s.template_config FROM sites s LEFT JOIN templates t ON s.template_id = t.id WHERE s.id = $1",
    )
    .bind(site_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to fetch site template: {}", e)))?;

    let template_id: Option<Uuid> = row.try_get("id").ok();
    let template_config: serde_json::Value = row.get("template_config");

    if template_id.is_some() {
        let template = TemplateResponse {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            category: row.get("category"),
            thumbnail_url: row.get("thumbnail_url"),
            html_content: row.get("html_content"),
            css_content: row.get("css_content"),
            js_content: row.get("js_content"),
            default_config: row.get("default_config"),
            is_builtin: row.get("is_builtin"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(Json(serde_json::json!({
            "template": template,
            "config": template_config
        })))
    } else {
        Ok(Json(serde_json::json!({
            "template": null,
            "config": template_config
        })))
    }
}

pub async fn update_theme(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(site_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    let theme = payload
        .get("theme")
        .and_then(|v| v.as_str())
        .filter(|s| *s == "light" || *s == "dark")
        .unwrap_or("light");

    sqlx::query("UPDATE sites SET theme = $2 WHERE id = $1")
        .bind(site_id)
        .bind(theme)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::new(format!("Failed to update theme: {}", e)))?;

    tracing::info!(
        "User {} updated theme to {} for site {}",
        current_user.user_id,
        theme,
        site_id
    );

    Ok(Json(serde_json::json!({
        "message": "Theme updated successfully",
        "theme": theme
    })))
}

pub async fn update_template_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(site_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    let template_config = payload.get("config");

    if template_config.is_none() {
        return Err(ApiError::new("Config is required"));
    }

    sqlx::query("UPDATE sites SET template_config = $2 WHERE id = $1")
        .bind(site_id)
        .bind(template_config)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::new(format!("Failed to update template config: {}", e)))?;

    tracing::info!(
        "User {} updated template config for site {}",
        current_user.user_id,
        site_id
    );

    Ok(Json(serde_json::json!({
        "message": "Template config updated successfully",
        "config": template_config
    })))
}

fn template_from_row(row: sqlx::postgres::PgRow) -> Result<TemplateResponse, ApiError> {
    Ok(TemplateResponse {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        category: row.get("category"),
        thumbnail_url: row.get("thumbnail_url"),
        html_content: row.get("html_content"),
        css_content: row.get("css_content"),
        js_content: row.get("js_content"),
        default_config: row.get("default_config"),
        is_builtin: row.get("is_builtin"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
