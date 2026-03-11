use crate::models::SiteSettings;
use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

pub type HtmlResponse = (StatusCode, HeaderMap, String);

pub fn render_nav_links(nav_links: &serde_json::Value, site_path: &str) -> String {
    if let Some(links) = nav_links.as_array() {
        links
            .iter()
            .map(|link| {
                let label = link.get("label").and_then(|l| l.as_str()).unwrap_or("");
                let url = link.get("url").and_then(|u| u.as_str()).unwrap_or("#");
                let full_url = if url.starts_with('/') {
                    format!("{}{}", site_path, url)
                } else {
                    url.to_string()
                };
                format!(
                    "<a href=\"{}\" class=\"text-gray-700 hover:text-blue-600 px-3\">{}</a>",
                    full_url, label
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        String::new()
    }
}

pub fn render_social_links(social_links: &serde_json::Value) -> String {
    if let Some(social) = social_links.as_object() {
        social.iter()
            .filter_map(|(platform, url)| {
                let url_str = url.as_str()?;
                if url_str.is_empty() { return None; }
                let icon = match platform.as_str() {
                    "x" => "fa-x-twitter",
                    "facebook" => "fa-facebook", 
                    "instagram" => "fa-instagram",
                    "linkedin" => "fa-linkedin",
                    "youtube" => "fa-youtube",
                    "github" => "fa-github",
                    "tiktok" => "fa-tiktok",
                    _ => "fa-link"
                };
                Some(format!(
                    "<a href=\"{}\" target=\"_blank\" class=\"text-gray-500 hover:text-gray-700\"><i class=\"fab {}\"></i></a>",
                    url_str, icon
                ))
            })
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        String::new()
    }
}

pub fn render_contact_info(settings: &SiteSettings) -> String {
    let mut parts = Vec::new();
    if !settings.contact_phone.is_empty() {
        parts.push(settings.contact_phone.clone());
    }
    if !settings.contact_email.is_empty() {
        parts.push(format!(
            "<a href=\"mailto:{}\">{}</a>",
            settings.contact_email, settings.contact_email
        ));
    }
    if !settings.contact_address.is_empty() {
        parts.push(settings.contact_address.clone());
    }
    parts.join(" | ")
}

pub fn render_header(settings: &SiteSettings, site_name: &str, slug: &str) -> String {
    let site_path = format!("/site/{}", slug);
    let nav_html = render_nav_links(&settings.nav_links, &site_path);
    let logo_img = if !settings.logo_url.is_empty() {
        format!("<img src=\"{}\" class=\"h-10 w-auto\">", settings.logo_url)
    } else {
        String::new()
    };

    format!(
        r#"
<header class="bg-white shadow-sm">
    <div class="max-w-4xl mx-auto px-6 py-4 flex items-center justify-between">
        <div class="flex items-center gap-4">
            {}<a href="{}" class="text-xl font-bold text-gray-800">{}</a>
        </div>
        <nav class="flex items-center gap-2">{}</nav>
    </div>
</header>"#,
        logo_img, site_path, site_name, nav_html
    )
}

pub fn render_footer(settings: &SiteSettings) -> String {
    let social_html = render_social_links(&settings.social_links);
    let contact_html = render_contact_info(settings);
    let has_contact = !contact_html.is_empty();

    format!(
        r#"
<footer class="bg-gray-100 mt-16">
    <div class="max-w-4xl mx-auto px-6 py-8">
        <div class="flex flex-col md:flex-row justify-between items-center gap-4">
            <div class="text-gray-600 text-sm">{}</div>
            <div class="flex gap-4">{}</div>
        </div>
        <div class="text-center text-gray-500 text-sm mt-4">{}</div>
    </div>
</footer>"#,
        if has_contact {
            format!("<div class=\"mb-2\">{}</div>", contact_html)
        } else {
            String::new()
        },
        social_html,
        settings.footer_text
    )
}

pub fn render_blocks(content: &serde_json::Value) -> String {
    if let Some(blocks) = content.as_array() {
        blocks.iter()
            .map(|block| {
                let block_type = block.get("block_type").and_then(|t| t.as_str()).unwrap_or("paragraph");
                let block_content = block.get("content");

                match block_type {
                    "heading" => {
                        let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or("");
                        format!("<h2 class=\"text-2xl font-bold mt-6 mb-4\">{}</h2>", text)
                    }
                    "paragraph" => {
                        let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or("");
                        format!("<p class=\"mb-4\">{}</p>", text)
                    }
                    "image" => {
                        let url = block_content.and_then(|c| c.get("url")).and_then(|u| u.as_str()).unwrap_or("");
                        let alt = block_content.and_then(|c| c.get("alt")).and_then(|a| a.as_str()).unwrap_or("");
                        let caption = block_content.and_then(|c| c.get("caption")).and_then(|c| c.as_str()).unwrap_or("");
                        if !url.is_empty() {
                            format!("<figure class=\"my-6\"><img src=\"{}\" alt=\"{}\" class=\"rounded-lg w-full\"></figure>", url, alt)
                        } else { String::new() }
                    }
                    "link" => {
                        let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or("");
                        let url = block_content.and_then(|c| c.get("url")).and_then(|u| u.as_str()).unwrap_or("#");
                        format!("<a href=\"{}\" class=\"text-blue-600 hover:underline\">{}</a>", url, text)
                    }
                    "hero" => {
                        let title = block_content.and_then(|c| c.get("title")).and_then(|t| t.as_str()).unwrap_or("");
                        let subtitle = block_content.and_then(|c| c.get("subtitle")).and_then(|s| s.as_str()).unwrap_or("");
                        let cta_text = block_content.and_then(|c| c.get("ctaText")).and_then(|c| c.as_str()).unwrap_or("");
                        let cta_link = block_content.and_then(|c| c.get("ctaLink")).and_then(|c| c.as_str()).unwrap_or("#");
                        let bg = block_content.and_then(|c| c.get("backgroundImage")).and_then(|b| b.as_str()).unwrap_or("");
                        let bg_style = if !bg.is_empty() {
                            format!("background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); background-image: url({});", bg)
                        } else {
                            "background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);".to_string()
                        };
                        format!(r#"
<div class="hero-section" style="{} padding: 80px 20px; text-align: center; color: white; border-radius: 12px; margin: 20px 0;">
    <h1 class="text-4xl font-bold mb-4">{}</h1>
    <p class="text-xl mb-6">{}</p>
    <a href="{}" class="inline-block bg-white text-purple-600 px-6 py-3 rounded-lg font-medium">{}</a>
</div>"#, bg_style, title, subtitle, cta_link, cta_text)
                    }
                    "video" => {
                        let url = block_content.and_then(|c| c.get("url")).and_then(|u| u.as_str()).unwrap_or("");
                        let caption = block_content.and_then(|c| c.get("caption")).and_then(|c| c.as_str()).unwrap_or("");
                        if !url.is_empty() {
                            let embed_url = if url.contains("youtube.com") || url.contains("youtu.be") {
                                let video_id = url.split("v=").nth(1).or_else(|| url.split('/').last()).unwrap_or("");
                                format!("https://youtube.com/embed/{}", video_id)
                            } else {
                                url.to_string()
                            };
                            format!(r#"<div class="my-6"><iframe src="{}" class="w-full aspect-video rounded-lg" frameborder="0" allowfullscreen></iframe><p class="text-gray-500 text-sm mt-2">{}</p></div>"#, embed_url, caption)
                        } else { String::new() }
                    }
                    "columns" => {
                        let left = block_content.and_then(|c| c.get("left")).and_then(|l| l.as_str()).unwrap_or("");
                        let right = block_content.and_then(|c| c.get("right")).and_then(|r| r.as_str()).unwrap_or("");
                        format!(r#"<div class="grid grid-cols-1 md:grid-cols-2 gap-6 my-6"><div>{}</div><div>{}</div></div>"#, left, right)
                    }
                    _ => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    }
}

pub fn make_response(html: String) -> HtmlResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "text/html".parse().unwrap(),
    );
    (StatusCode::OK, headers, html)
}

pub fn make_error(status: StatusCode, message: &str) -> HtmlResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "text/plain".parse().unwrap(),
    );
    (status, headers, message.to_string())
}

pub async fn get_site_settings(db: &PgPool, site_id: Uuid) -> Result<SiteSettings, sqlx::Error> {
    let row = sqlx::query(
        "SELECT logo_url, nav_links, footer_text, social_links, contact_email, contact_phone, contact_address 
         FROM sites WHERE id = $1"
    )
    .bind(site_id)
    .fetch_one(db)
    .await?;

    Ok(SiteSettings::from_row(&row))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_nav_links_with_valid_links() {
        let nav_links = json!([
            {"label": "Home", "url": "/"},
            {"label": "About", "url": "/about"},
            {"label": "Blog", "url": "/blog"}
        ]);

        let result = render_nav_links(&nav_links, "/mysite");

        assert!(result.contains("href=\"/mysite/\""));
        assert!(result.contains("Home"));
        assert!(result.contains("href=\"/mysite/about\""));
        assert!(result.contains("About"));
        assert!(result.contains("href=\"/mysite/blog\""));
        assert!(result.contains("Blog"));
    }

    #[test]
    fn test_render_nav_links_empty_array() {
        let nav_links = json!([]);
        let result = render_nav_links(&nav_links, "/mysite");
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_nav_links_null() {
        let nav_links = serde_json::Value::Null;
        let result = render_nav_links(&nav_links, "/mysite");
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_nav_links_external_url() {
        let nav_links = json!([
            {"label": "Google", "url": "https://google.com"}
        ]);

        let result = render_nav_links(&nav_links, "/mysite");
        assert!(result.contains("href=\"https://google.com\""));
    }

    #[test]
    fn test_render_social_links_valid() {
        let social_links = json!({
            "github": "https://github.com/test",
            "x": "https://x.com/test",
            "youtube": "https://youtube.com/test"
        });

        let result = render_social_links(&social_links);

        assert!(result.contains("github.com"));
        assert!(result.contains("fa-github"));
        assert!(result.contains("fa-x-twitter"));
        assert!(result.contains("fa-youtube"));
    }

    #[test]
    fn test_render_social_links_empty_values() {
        let social_links = json!({
            "github": "",
            "x": "https://x.com/test"
        });

        let result = render_social_links(&social_links);

        assert!(!result.contains("github"));
        assert!(result.contains("x.com"));
    }

    #[test]
    fn test_render_social_links_null() {
        let social_links = serde_json::Value::Null;
        let result = render_social_links(&social_links);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_contact_info_email_and_phone() {
        let settings = SiteSettings {
            logo_url: String::new(),
            nav_links: serde_json::Value::Null,
            footer_text: String::new(),
            social_links: serde_json::Value::Null,
            contact_email: "test@example.com".to_string(),
            contact_phone: "555-1234".to_string(),
            contact_address: String::new(),
        };

        let result = render_contact_info(&settings);

        assert!(result.contains("555-1234"));
        assert!(result.contains("test@example.com"));
        assert!(result.contains("mailto:test@example.com"));
    }

    #[test]
    fn test_render_contact_info_all_empty() {
        let settings = SiteSettings {
            logo_url: String::new(),
            nav_links: serde_json::Value::Null,
            footer_text: String::new(),
            social_links: serde_json::Value::Null,
            contact_email: String::new(),
            contact_phone: String::new(),
            contact_address: String::new(),
        };

        let result = render_contact_info(&settings);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_blocks_heading() {
        let blocks = json!([
            {"block_type": "heading", "content": {"text": "Hello World"}}
        ]);

        let result = render_blocks(&blocks);

        assert!(result.contains("<h2"));
        assert!(result.contains("Hello World"));
        assert!(result.contains("</h2>"));
    }

    #[test]
    fn test_render_blocks_paragraph() {
        let blocks = json!([
            {"block_type": "paragraph", "content": {"text": "This is a paragraph"}}
        ]);

        let result = render_blocks(&blocks);

        assert!(result.contains("<p"));
        assert!(result.contains("This is a paragraph"));
        assert!(result.contains("</p>"));
    }

    #[test]
    fn test_render_blocks_image() {
        let blocks = json!([
            {"block_type": "image", "content": {"url": "https://example.com/img.jpg", "alt": "Test image"}}
        ]);

        let result = render_blocks(&blocks);

        assert!(result.contains("img.jpg"));
        assert!(result.contains("Test image"));
    }

    #[test]
    fn test_render_blocks_image_empty_url() {
        let blocks = json!([
            {"block_type": "image", "content": {"url": "", "alt": "Test"}}
        ]);

        let result = render_blocks(&blocks);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_blocks_link() {
        let blocks = json!([
            {"block_type": "link", "content": {"text": "Click here", "url": "https://example.com"}}
        ]);

        let result = render_blocks(&blocks);

        assert!(result.contains("href=\"https://example.com\""));
        assert!(result.contains("Click here"));
    }

    #[test]
    fn test_render_blocks_hero() {
        let blocks = json!([
            {"block_type": "hero", "content": {
                "title": "Welcome",
                "subtitle": "Best site ever",
                "ctaText": "Get Started",
                "ctaLink": "/signup"
            }}
        ]);

        let result = render_blocks(&blocks);

        assert!(result.contains("Welcome"));
        assert!(result.contains("Best site ever"));
        assert!(result.contains("Get Started"));
    }

    #[test]
    fn test_render_blocks_video_youtube() {
        let blocks = json!([
            {"block_type": "video", "content": {"url": "https://youtube.com/watch?v=abc123", "caption": "My video"}}
        ]);

        let result = render_blocks(&blocks);

        assert!(result.contains("youtube.com/embed/abc123"));
        assert!(result.contains("My video"));
    }

    #[test]
    fn test_render_blocks_columns() {
        let blocks = json!([
            {"block_type": "columns", "content": {"left": "Left content", "right": "Right content"}}
        ]);

        let result = render_blocks(&blocks);

        assert!(result.contains("Left content"));
        assert!(result.contains("Right content"));
        assert!(result.contains("grid-cols-2"));
    }

    #[test]
    fn test_render_blocks_unknown_type() {
        let blocks = json!([
            {"block_type": "unknown_type", "content": {"text": "Should not render"}}
        ]);

        let result = render_blocks(&blocks);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_blocks_null_content() {
        let blocks = serde_json::Value::Null;
        let result = render_blocks(&blocks);
        assert!(result.is_empty());
    }

    #[test]
    fn test_render_header_with_logo() {
        let settings = SiteSettings {
            logo_url: "https://example.com/logo.png".to_string(),
            nav_links: serde_json::Value::Null,
            footer_text: String::new(),
            social_links: serde_json::Value::Null,
            contact_email: String::new(),
            contact_phone: String::new(),
            contact_address: String::new(),
        };

        let result = render_header(&settings, "My Site", "my-site");

        assert!(result.contains("logo.png"));
        assert!(result.contains("My Site"));
        assert!(result.contains("/site/my-site"));
    }

    #[test]
    fn test_render_header_without_logo() {
        let settings = SiteSettings {
            logo_url: String::new(),
            nav_links: serde_json::Value::Null,
            footer_text: String::new(),
            social_links: serde_json::Value::Null,
            contact_email: String::new(),
            contact_phone: String::new(),
            contact_address: String::new(),
        };

        let result = render_header(&settings, "My Site", "my-site");

        assert!(result.contains("My Site"));
        assert!(result.contains("<header"));
    }

    #[test]
    fn test_render_footer_with_contact() {
        let settings = SiteSettings {
            logo_url: String::new(),
            nav_links: serde_json::Value::Null,
            footer_text: "Copyright 2024".to_string(),
            social_links: serde_json::Value::Null,
            contact_email: "test@example.com".to_string(),
            contact_phone: String::new(),
            contact_address: String::new(),
        };

        let result = render_footer(&settings);

        assert!(result.contains("test@example.com"));
        assert!(result.contains("Copyright 2024"));
        assert!(result.contains("<footer"));
    }

    #[test]
    fn test_make_response() {
        let html = "<html><body>Test</body></html>".to_string();
        let (status, headers, body) = make_response(html);

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "<html><body>Test</body></html>");
        assert_eq!(
            headers.get("content-type").unwrap().to_str().unwrap(),
            "text/html"
        );
    }

    #[test]
    fn test_make_error() {
        let (status, headers, body) = make_error(StatusCode::NOT_FOUND, "Not found");

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, "Not found");
        assert_eq!(
            headers.get("content-type").unwrap().to_str().unwrap(),
            "text/plain"
        );
    }
}
