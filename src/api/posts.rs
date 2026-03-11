use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::api::auth::{require_auth, require_site_member};
use crate::{ssg, ApiError, AppState, CreatePostRequest, Post, UpdatePostRequest};

pub async fn list(
    State(state): State<AppState>,
    Path(site_id): Path<Uuid>,
) -> Result<Json<Vec<Post>>, ApiError> {
    let posts = sqlx::query_as::<_, (
        Uuid, Uuid, Option<Uuid>, String, String, serde_json::Value,
        Option<String>, Option<String>, String, Option<chrono::DateTime<chrono::Utc>>,
        chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, serde_json::Value
    )>(
        "SELECT id, site_id, author_id, title, slug, content, excerpt, featured_image, status, published_at, created_at, updated_at, seo 
         FROM posts WHERE site_id = $1 ORDER BY published_at DESC NULLS LAST"
    )
    .bind(site_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to fetch posts: {}", e)))?;

    let posts: Vec<Post> = posts
        .into_iter()
        .map(|p| Post {
            id: p.0,
            site_id: p.1,
            author_id: p.2,
            title: p.3,
            slug: p.4,
            content: p.5,
            excerpt: p.6,
            featured_image: p.7,
            status: p.8,
            published_at: p.9,
            created_at: p.10,
            updated_at: p.11,
            seo: p.12,
        })
        .collect();

    Ok(Json(posts))
}

pub async fn get(
    State(state): State<AppState>,
    Path((site_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Post>, ApiError> {
    let post = sqlx::query_as::<_, (
        Uuid, Uuid, Option<Uuid>, String, String, serde_json::Value,
        Option<String>, Option<String>, String, Option<chrono::DateTime<chrono::Utc>>,
        chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, serde_json::Value
    )>(
        "SELECT id, site_id, author_id, title, slug, content, excerpt, featured_image, status, published_at, created_at, updated_at, seo 
         FROM posts WHERE site_id = $1 AND id = $2"
    )
    .bind(site_id)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Post not found"))?;

    Ok(Json(Post {
        id: post.0,
        site_id: post.1,
        author_id: post.2,
        title: post.3,
        slug: post.4,
        content: post.5,
        excerpt: post.6,
        featured_image: post.7,
        status: post.8,
        published_at: post.9,
        created_at: post.10,
        updated_at: post.11,
        seo: post.12,
    }))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(site_id): Path<Uuid>,
    Json(payload): Json<CreatePostRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    if payload.title.is_empty() {
        return Err(ApiError::new("Title is required"));
    }

    let slug = payload.slug.unwrap_or_else(|| {
        payload
            .title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    });

    let content = payload.content.clone();
    let excerpt = payload.excerpt.clone();
    let featured_image = payload.featured_image.clone();
    let seo = payload.seo.clone().unwrap_or(serde_json::json!({}));

    let result = sqlx::query_as::<_, (
        Uuid, Uuid, Option<Uuid>, String, String, serde_json::Value,
        Option<String>, Option<String>, String, Option<chrono::DateTime<chrono::Utc>>,
        chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, serde_json::Value
    )>(
        "INSERT INTO posts (site_id, title, slug, content, excerpt, featured_image, seo) VALUES ($1, $2, $3, $4, $5, $6, $7) 
         RETURNING id, site_id, author_id, title, slug, content, excerpt, featured_image, status, published_at, created_at, updated_at, seo"
    )
    .bind(site_id)
    .bind(&payload.title)
    .bind(&slug)
    .bind(&content)
    .bind(&excerpt)
    .bind(&featured_image)
    .bind(&seo)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::new(format!("Failed to create post: {}", e)))?;

    let post = Post {
        id: result.0,
        site_id: result.1,
        author_id: result.2,
        title: result.3,
        slug: result.4,
        content: result.5,
        excerpt: result.6,
        featured_image: result.7,
        status: result.8,
        published_at: result.9,
        created_at: result.10,
        updated_at: result.11,
        seo: result.12,
    };

    Ok((StatusCode::CREATED, Json(post)))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((site_id, id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdatePostRequest>,
) -> Result<Json<Post>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    let title = payload.title.clone();
    let content = payload.content.clone();
    let excerpt = payload.excerpt.clone();
    let featured_image = payload.featured_image.clone();
    let status = payload.status.clone();
    let seo = payload.seo.clone();

    let result = sqlx::query_as::<_, (
        Uuid, Uuid, Option<Uuid>, String, String, serde_json::Value,
        Option<String>, Option<String>, String, Option<chrono::DateTime<chrono::Utc>>,
        chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, serde_json::Value
    )>(
        "UPDATE posts SET 
            title = COALESCE($3, title),
            content = COALESCE($4, content),
            excerpt = COALESCE($5, excerpt),
            featured_image = COALESCE($6, featured_image),
            status = COALESCE($7, status),
            seo = COALESCE($8, seo),
            updated_at = NOW()
         WHERE site_id = $1 AND id = $2
         RETURNING id, site_id, author_id, title, slug, content, excerpt, featured_image, status, published_at, created_at, updated_at, seo"
    )
    .bind(site_id)
    .bind(id)
    .bind(title)
    .bind(content)
    .bind(excerpt)
    .bind(featured_image)
    .bind(status)
    .bind(seo)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Post not found"))?;

    Ok(Json(Post {
        id: result.0,
        site_id: result.1,
        author_id: result.2,
        title: result.3,
        slug: result.4,
        content: result.5,
        excerpt: result.6,
        featured_image: result.7,
        status: result.8,
        published_at: result.9,
        created_at: result.10,
        updated_at: result.11,
        seo: result.12,
    }))
}

pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((site_id, id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    sqlx::query("DELETE FROM posts WHERE site_id = $1 AND id = $2")
        .bind(site_id)
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::new(format!("Failed to delete post: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((site_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Post>, ApiError> {
    let current_user = require_auth(State(state.clone()), headers).await?;
    require_site_member(&state, site_id, current_user.user_id).await?;

    let result = sqlx::query_as::<_, (
        Uuid, Uuid, Option<Uuid>, String, String, serde_json::Value,
        Option<String>, Option<String>, String, Option<chrono::DateTime<chrono::Utc>>,
        chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, serde_json::Value
    )>(
        "UPDATE posts SET status = 'published', published_at = NOW(), updated_at = NOW() 
         WHERE site_id = $1 AND id = $2
         RETURNING id, site_id, author_id, title, slug, content, excerpt, featured_image, status, published_at, created_at, updated_at, seo"
    )
    .bind(site_id)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| ApiError::new("Post not found"))?;

    let post = Post {
        id: result.0,
        site_id: result.1,
        author_id: result.2,
        title: result.3,
        slug: result.4,
        content: result.5,
        excerpt: result.6,
        featured_image: result.7,
        status: result.8,
        published_at: result.9,
        created_at: result.10,
        updated_at: result.11,
        seo: result.12,
    };

    tokio::spawn(async move {
        let db = state.db.clone();
        if let Err(e) = ssg::build_site(&db, site_id).await {
            tracing::error!("Failed to build static site: {}", e);
        }
    });

    Ok(Json(post))
}
