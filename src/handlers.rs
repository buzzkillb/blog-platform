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

            let post = sqlx::query_as::<_, (String, String, serde_json::Value, Option<String>, Option<String>)>(
                "SELECT title, slug, content, excerpt, featured_image FROM posts WHERE site_id = $1 AND slug = $2 AND status = 'published'"
            )
            .bind(site_id)
            .bind(&post_slug)
            .fetch_optional(&state.db)
            .await;

            match post {
                Ok(Some(post_row)) => {
                    let settings = match get_site_settings(&state.db, site_id).await {
                        Ok(s) => s,
                        Err(_) => return make_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to load settings"),
                    };

                    let header_html = render_header(&settings, &name, "");
                    let footer_html = render_footer(&settings);

                    let title = post_row.0;
                    let content_html = render_blocks(&post_row.2);

                    let featured_html = if let Some(img) = post_row.4 {
                        format!(r#"<img src="{}" alt="{}" class="w-full h-64 object-cover rounded-lg mb-8">"#, img, title)
                    } else {
                        String::new()
                    };

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
