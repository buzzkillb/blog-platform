use crate::render::{
    get_site_settings, make_error, make_response, render_blocks, render_footer, render_header,
    HtmlResponse,
};
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use sqlx::Row;
use uuid::Uuid;

#[allow(dead_code)]
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
            let _description = row
                .get::<Option<String>, _>("description")
                .unwrap_or_default();
            let homepage_type: String = row
                .get::<Option<String>, _>("homepage_type")
                .unwrap_or_else(|| "both".to_string());

            let settings = match get_site_settings(&state.db, site_id).await {
                Ok(s) => s,
                Err(_) => {
                    return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings")
                }
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
                    format!(
                        r#"
<h1 class="text-4xl font-bold mb-6">{}</h1>
<div class="prose mb-8">{}</div>"#,
                        page_title, content_html
                    )
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let _posts_section = if matches!(homepage_type.as_str(), "blog" | "both") {
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
                        format!(
                            r#"<h2 class="text-2xl font-bold mb-4">Latest Posts</h2>{}"#,
                            posts_html
                        )
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let footer_html = render_footer(&settings);

            let html = format!(
                r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}<div class="max-w-4xl mx-auto p-8">{}{}</div>{}</body></html>"#,
                name, header_html, main_content, _posts_section, footer_html
            );

            make_response(html)
        }
        _ => make_error(StatusCode::NOT_FOUND, "Site not found"),
    }
}

#[allow(dead_code)]
pub async fn view_post(
    axum::extract::Path(post_slug): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> HtmlResponse {
    let site = sqlx::query("SELECT id, name FROM sites LIMIT 1")
        .fetch_optional(&state.db)
        .await;

    match site {
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
                        Err(_) => {
                            return make_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to load settings",
                            )
                        }
                    };

                    let header_html = render_header(&settings, &name, "");
                    let content_html = render_blocks(&content);
                    let featured_html = if let Some(img) = featured_image {
                        format!(
                            r#"<img src="{}" class="w-full h-64 object-cover rounded-lg mb-6">"#,
                            img
                        )
                    } else {
                        String::new()
                    };
                    let footer_html = render_footer(&settings);

                    let html = format!(
                        r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">
<a href="/blog" class="text-blue-600">← Back to Blog</a>
{}
<h1 class="text-4xl font-bold mt-4 mb-6">{}</h1>
<div class="prose">{}</div>
</div>
{}</body></html>"#,
                        title, header_html, featured_html, title, content_html, footer_html
                    );

                    make_response(html)
                }
                _ => make_error(StatusCode::NOT_FOUND, "Post not found"),
            }
        }
        _ => make_error(StatusCode::NOT_FOUND, "No site configured"),
    }
}

pub async fn view_page(
    axum::extract::Path(slug): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> HtmlResponse {
    let site = sqlx::query("SELECT id, name FROM sites LIMIT 1")
        .fetch_optional(&state.db)
        .await;

    match site {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");

            let page = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT title, content FROM pages WHERE site_id = $1 AND slug = $2 LIMIT 1",
            )
            .bind(site_id)
            .bind(&slug)
            .fetch_optional(&state.db)
            .await;

            match page {
                Ok(Some((title, content))) => {
                    let settings = match get_site_settings(&state.db, site_id).await {
                        Ok(s) => s,
                        Err(_) => {
                            return make_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to load settings",
                            )
                        }
                    };

                    let header_html = render_header(&settings, &name, "");
                    let content_html = render_blocks(&content);
                    let footer_html = render_footer(&settings);

                    let html = format!(
                        r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">
<a href="/" class="text-blue-600">← Back</a>
<h1 class="text-4xl font-bold mt-4 mb-6">{}</h1>
<div class="prose">{}</div>
</div>
{}{}</body></html>"#,
                        title, header_html, title, content_html, footer_html, ""
                    );

                    make_response(html)
                }
                _ => make_error(StatusCode::NOT_FOUND, "Page not found"),
            }
        }
        _ => make_error(StatusCode::NOT_FOUND, "No site configured"),
    }
}

pub async fn view_blog(State(state): State<AppState>) -> HtmlResponse {
    let site = sqlx::query("SELECT id, name FROM sites LIMIT 1")
        .fetch_optional(&state.db)
        .await;

    match site {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");
            view_blog_listing(&state, site_id, "blog", &name).await
        }
        _ => make_error(StatusCode::NOT_FOUND, "No site configured"),
    }
}

async fn view_blog_listing(
    state: &AppState,
    site_id: Uuid,
    _blog_path: &str,
    name: &str,
) -> HtmlResponse {
    let settings = match get_site_settings(&state.db, site_id).await {
        Ok(s) => s,
        Err(_) => return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings"),
    };

    let posts = sqlx::query_as::<_, (String, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT title, slug, excerpt, published_at FROM posts WHERE site_id = $1 AND status = 'published' ORDER BY published_at DESC"
    )
    .bind(site_id)
    .fetch_all(&state.db)
    .await;

    let posts_html = match posts {
        Ok(rows) => {
            if rows.is_empty() {
                "<p>No posts yet.</p>".to_string()
            } else {
                rows.iter().map(|p| {
                    let date = p.3.format("%Y-%m-%d").to_string();
                    format!(
                        r#"<article class="mb-6 p-4 bg-white rounded-lg shadow"><h2 class="text-2xl font-bold mb-2"><a href="/post/{}" class="text-blue-600">{}</a></h2><p class="text-gray-600 mb-2">{}</p><small class="text-gray-500">{}</small></article>"#,
                        p.1, p.0, p.2.as_deref().unwrap_or(""), date
                    )
                }).collect::<Vec<_>>().join("\n")
            }
        }
        Err(_) => "<p>Failed to load posts.</p>".to_string(),
    };

    let header_html = render_header(&settings, name, "");
    let footer_html = render_footer(&settings);

    let html = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{} - Blog</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-6">Blog</h1>
{}
</div>{}</body></html>"#,
        name, header_html, posts_html, footer_html
    );

    make_response(html)
}

pub async fn view_about(State(state): State<AppState>) -> HtmlResponse {
    let site = sqlx::query("SELECT id, name FROM sites LIMIT 1")
        .fetch_optional(&state.db)
        .await;

    match site {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");

            let settings = match get_site_settings(&state.db, site_id).await {
                Ok(s) => s,
                Err(_) => {
                    return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings")
                }
            };

            let header_html = render_header(&settings, &name, "");
            let footer_html = render_footer(&settings);

            let html = format!(
                r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>About - {}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-6">About</h1>
<div class="prose">
<p>Welcome to our blog! We're glad you're here.</p>
</div>
</div>{}</body></html>"#,
                name, header_html, footer_html
            );

            make_response(html)
        }
        _ => make_error(StatusCode::NOT_FOUND, "No site configured"),
    }
}

pub async fn view_contact(State(state): State<AppState>) -> HtmlResponse {
    let site = sqlx::query("SELECT id, name FROM sites LIMIT 1")
        .fetch_optional(&state.db)
        .await;

    match site {
        Ok(Some(row)) => {
            let site_id: Uuid = row.get("id");
            let name: String = row.get("name");

            let settings = match get_site_settings(&state.db, site_id).await {
                Ok(s) => s,
                Err(_) => {
                    return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings")
                }
            };

            let header_html = render_header(&settings, &name, "");
            let footer_html = render_footer(&settings);

            let html = format!(
                r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>Contact - {}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
</head><body class="bg-gray-50">{}
<div class="max-w-4xl mx-auto p-8">
<h1 class="text-4xl font-bold mb-6">Contact</h1>
<div class="prose">
<p>Get in touch with us!</p>
<form method="POST" action="/api/sites/{}/contact" class="mt-4">
  <input type="text" name="name" placeholder="Your Name" required class="border p-2 w-full mb-2">
  <input type="email" name="email" placeholder="Your Email" required class="border p-2 w-full mb-2">
  <textarea name="message" placeholder="Your Message" required class="border p-2 w-full mb-2"></textarea>
  <button type="submit" class="bg-blue-600 text-white px-4 py-2 rounded">Send</button>
</form>
</div>
</div>{}</body></html>"#,
                name, header_html, site_id, footer_html
            );

            make_response(html)
        }
        _ => make_error(StatusCode::NOT_FOUND, "No site configured"),
    }
}
