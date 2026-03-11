use crate::models::{Page, Post, SiteSettings};
use crate::render::{get_site_settings, make_error, make_response, render_blocks, render_footer, render_header, HtmlResponse};
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use sqlx::Row;
use uuid::Uuid;

pub async fn view_site(
    axum::extract::Path(slug): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> HtmlResponse {
    let site_row = sqlx::query(
        "SELECT id, name, description, homepage_type FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    match site_row {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");
            let description = row.get::<Option<String>, _>("description").unwrap_or_default();
            let homepage_type: String = row.get::<Option<String>, _>("homepage_type").unwrap_or_else(|| "both".to_string());
            
            let settings = match get_site_settings(&state.db, site_id).await {
                Ok(s) => s,
                Err(_) => return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings"),
            };

            let homepage_page = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT title, content FROM pages WHERE site_id = $1 AND is_homepage = true LIMIT 1"
            )
            .bind(site_id)
            .fetch_optional(&state.db)
            .await;

            let show_homepage_page = matches!(homepage_type.as_str(), "landing" | "both") 
                && homepage_page.is_ok() 
                && homepage_page.as_ref().ok().is_some();

            let header_html = render_header(&settings, &name, &slug);

            let main_content = if show_homepage_page {
                if let Ok(Some((page_title, page_content))) = homepage_page {
                    let content_html = render_blocks(&page_content);
                    format!(r#"
<h1 class="text-4xl font-bold mb-6">{}</h1>
<div class="prose mb-8">{}</div>"#, page_title, content_html)
                } else { String::new() }
            } else {
                String::new()
            };

            let posts_section = if matches!(homepage_type.as_str(), "blog" | "both") {
                let posts = sqlx::query_as::<_, (String, String, Option<String>)>(
                    "SELECT title, slug, excerpt FROM posts WHERE site_id = $1 AND status = 'published' ORDER BY published_at DESC LIMIT 5"
                )
                .bind(site_id)
                .fetch_all(&state.db)
                .await;

                if let Ok(posts) = posts {
                    if !posts.is_empty() {
                        let posts_html = posts.iter()
                            .map(|p| format!(
                                r#"<article class="mb-4"><h3 class="text-xl font-bold"><a href="/site/{}/post/{}" class="text-blue-600">{}</a></h3><p class="text-gray-600">{}</p></article>"#,
                                slug, p.1, p.0, p.2.as_deref().unwrap_or("")
                            ))
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!(r#"<h2 class="text-2xl font-bold mb-4">Latest Posts</h2>{}"#, posts_html)
                    } else { String::new() }
                } else { String::new() }
            } else {
                String::new()
            };

            let footer_html = render_footer(&settings);

            let html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}<div class="max-w-4xl mx-auto p-8">{}</div>{}</body></html>"#,
                name, header_html, main_content, footer_html);

            make_response(html)
        }
        _ => make_error(StatusCode::NOT_FOUND, "Site not found"),
    }
}

pub async fn view_post(
    axum::extract::Path((slug, post_slug)): axum::extract::Path<(String, String)>,
    State(state): State<AppState>,
) -> HtmlResponse {
    let site_row = sqlx::query(
        "SELECT id, name FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    match site_row {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");

            let post = sqlx::query_as::<_, (String, String, serde_json::Value, Option<String>)>(
                "SELECT title, slug, content, featured_image FROM posts WHERE site_id = $1 AND slug = $2 AND status = 'published' LIMIT 1"
            )
            .bind(site_id)
            .bind(&post_slug)
            .fetch_optional(&state.db)
            .await;

            match post {
                Ok(Some((title, _slug, content, featured_image))) => {
                    let settings = match get_site_settings(&state.db, site_id).await {
                        Ok(s) => s,
                        Err(_) => return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings"),
                    };

                    let header_html = render_header(&settings, &name, &slug);
                    let content_html = render_blocks(&content);
                    
                    let featured_html = if let Some(img) = featured_image {
                        format!(r#"<img src="{}" class="w-full h-64 object-cover rounded-lg mb-6">"#, img)
                    } else { String::new() };

                    let footer_html = render_footer(&settings);

                    let html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">
<a href="/site/{}" class="text-blue-600">← Back</a>
<h1 class="text-4xl font-bold mt-4 mb-6">{}</h1>
{}
<div class="prose">{}</div>
</div>{}</body></html>"#,
                        title, header_html, slug, title, featured_html, content_html, footer_html);

                    make_response(html)
                }
                _ => make_error(StatusCode::NOT_FOUND, "Post not found"),
            }
        }
        _ => make_error(StatusCode::NOT_FOUND, "Site not found"),
    }
}

pub async fn view_page(
    axum::extract::Path((slug, page_slug)): axum::extract::Path<(String, String)>,
    State(state): State<AppState>,
) -> HtmlResponse {
    let site_row = sqlx::query(
        "SELECT id, name FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    match site_row {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");

            let page = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT title, content FROM pages WHERE site_id = $1 AND slug = $2"
            )
            .bind(site_id)
            .bind(&page_slug)
            .fetch_optional(&state.db)
            .await;

            match page {
                Ok(Some((title, content))) => {
                    let settings = match get_site_settings(&state.db, site_id).await {
                        Ok(s) => s,
                        Err(_) => return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings"),
                    };

                    let header_html = render_header(&settings, &name, &slug);
                    let content_html = render_blocks(&content);
                    let footer_html = render_footer(&settings);

                    let html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">
<a href="/site/{}" class="text-blue-600">← Back</a>
<h1 class="text-4xl font-bold mt-4 mb-6">{}</h1>
<div class="prose">{}</div>
</div>{}</body></html>"#,
                        title, header_html, slug, title, content_html, footer_html);

                    make_response(html)
                }
                _ => make_error(StatusCode::NOT_FOUND, "Page not found"),
            }
        }
        _ => make_error(StatusCode::NOT_FOUND, "Site not found"),
    }
}

pub async fn view_blog_at_path(
    axum::extract::Path((slug, path)): axum::extract::Path<(String, String)>,
    State(state): State<AppState>,
) -> HtmlResponse {
    let clean_path = format!("/{}", path.trim_start_matches('/'));
    
    let site_row = sqlx::query(
        "SELECT id, name FROM sites WHERE subdomain = $1 OR custom_domain = $1 LIMIT 1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    match site_row {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");
            
            if clean_path == "/blog" {
                return view_blog_listing(&state, site_id, &slug, &name).await;
            }
            
            let page_slug = path.trim_start_matches('/');
            let page = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT title, content FROM pages WHERE site_id = $1 AND slug = $2"
            )
            .bind(site_id)
            .bind(page_slug)
            .fetch_optional(&state.db)
            .await;

            match page {
                Ok(Some((page_title, page_content))) => {
                    return view_page_content(&state, site_id, &slug, &name, &page_title, &page_content).await;
                }
                _ => {
                    let homepage_type: String = sqlx::query(
                        "SELECT homepage_type FROM sites WHERE id = $1"
                    )
                    .bind(site_id)
                    .fetch_optional(&state.db)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|sr| sr.get::<Option<String>, _>("homepage_type"))
                    .unwrap_or_else(|| "both".to_string());
                    
                    if clean_path == "/" && homepage_type == "blog" {
                        return view_blog_listing(&state, site_id, &slug, &name).await;
                    }
                    
                    return make_error(StatusCode::NOT_FOUND, "Not found");
                }
            }
        }
        _ => make_error(StatusCode::NOT_FOUND, "Site not found"),
    }
}

pub async fn view_blog_listing(
    state: &AppState,
    site_id: Uuid,
    slug: &str,
    name: &str,
) -> HtmlResponse {
    let settings = match get_site_settings(&state.db, site_id).await {
        Ok(s) => s,
        Err(_) => return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings"),
    };

    let header_html = render_header(&settings, name, slug);

    let posts = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT title, slug, excerpt FROM posts WHERE site_id = $1 AND status = 'published' ORDER BY published_at DESC"
    )
    .bind(site_id)
    .fetch_all(&state.db)
    .await;

    let posts_html = match posts {
        Ok(posts) => posts.iter()
            .map(|p| format!(
                r#"<article class="mb-8"><h2 class="text-2xl font-bold mb-2"><a href="/site/{}/post/{}" class="text-blue-600">{}</a></h2><p class="text-gray-600">{}</p></article>"#,
                slug, p.1, p.0, p.2.as_deref().unwrap_or("")
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };

    let footer_html = render_footer(&settings);

    let html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>Blog - {}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-8">Blog</h1>
{}
</div>
{}</body></html>"#, name, header_html, posts_html, footer_html);

    make_response(html)
}

pub async fn view_page_content(
    state: &AppState,
    site_id: Uuid,
    slug: &str,
    name: &str,
    page_title: &str,
    page_content: &serde_json::Value,
) -> HtmlResponse {
    let settings = match get_site_settings(&state.db, site_id).await {
        Ok(s) => s,
        Err(_) => return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings"),
    };

    let header_html = render_header(&settings, name, slug);
    let content_html = render_blocks(page_content);
    let footer_html = render_footer(&settings);

    let html = format!(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">
<a href="/site/{}" class="text-blue-600">← Back</a>
<h1 class="text-4xl font-bold mt-4 mb-6">{}</h1>
<div class="prose">{}</div>
</div>
{}</body></html>"#, page_title, header_html, slug, page_title, content_html, footer_html);

    make_response(html)
}
