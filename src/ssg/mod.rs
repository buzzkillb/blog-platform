use minijinja::{context, Environment};
use sqlx::Row;
use uuid::Uuid;

pub async fn build_site(
    db: &sqlx::PgPool,
    site_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let site_row = sqlx::query(
        "SELECT id, subdomain, custom_domain, name, description, logo_url, theme, settings, nav_links, footer_text, social_links, contact_phone, contact_email, contact_address FROM sites WHERE id = $1"
    )
    .bind(site_id)
    .fetch_one(db)
    .await?;

    let site_id: Uuid = site_row.get("id");
    let _subdomain: Option<String> = site_row.get("subdomain");
    let _custom_domain: Option<String> = site_row.get("custom_domain");
    let site_name: String = site_row.get("name");
    let site_description: Option<String> = site_row.get("description");
    let logo_url: Option<String> = site_row.get("logo_url");
    let _theme: String = site_row
        .get::<Option<String>, _>("theme")
        .unwrap_or_default();
    let _settings: serde_json::Value = site_row
        .get::<Option<serde_json::Value>, _>("settings")
        .unwrap_or(serde_json::json!({}));
    let nav_links: serde_json::Value = site_row
        .get::<Option<serde_json::Value>, _>("nav_links")
        .unwrap_or(serde_json::json!([]));
    let footer_text: Option<String> = site_row.get("footer_text");
    let social_links: serde_json::Value = site_row
        .get::<Option<serde_json::Value>, _>("social_links")
        .unwrap_or(serde_json::json!({}));
    let contact_phone: Option<String> = site_row.get("contact_phone");
    let contact_email: Option<String> = site_row.get("contact_email");
    let contact_address: Option<String> = site_row.get("contact_address");

    let posts = sqlx::query_as::<_, (
        String, String, serde_json::Value, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>
    )>(
        "SELECT title, slug, content, excerpt, featured_image, published_at FROM posts WHERE site_id = $1 AND status = 'published' ORDER BY published_at DESC"
    )
    .bind(site_id)
    .fetch_all(db)
    .await?;

    let pages = sqlx::query_as::<_, (String, String, serde_json::Value, bool)>(
        "SELECT title, slug, content, is_homepage FROM pages WHERE site_id = $1",
    )
    .bind(site_id)
    .fetch_all(db)
    .await?;

    let mut env = Environment::new();

    // Find templates directory
    let cwd = std::env::current_dir().unwrap_or_default();
    let template_dir = cwd.join("templates");

    let base_html = std::fs::read_to_string(template_dir.join("base.html"))
        .map_err(|e| format!("Failed to read base.html: {}", e))?;
    let post_html = std::fs::read_to_string(template_dir.join("post.html"))
        .map_err(|e| format!("Failed to read post.html: {}", e))?;
    let page_html = std::fs::read_to_string(template_dir.join("page.html"))
        .map_err(|e| format!("Failed to read page.html: {}", e))?;
    let index_html = std::fs::read_to_string(template_dir.join("index.html"))
        .map_err(|e| format!("Failed to read index.html: {}", e))?;

    // Load all templates first so inheritance works
    env.add_template("base", &base_html)?;
    env.add_template("base.html", &base_html)?;
    env.add_template("post", &post_html)?;
    env.add_template("page", &page_html)?;
    // Load index last since it extends base
    env.add_template("index", &index_html)?;

    let sitemap_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{% for post in posts %}
  <url>
    <loc>https://example.com/{{ post }}</loc>
    <changefreq>weekly</changefreq>
  </url>
{% endfor %}
</urlset>"#;
    env.add_template("sitemap", sitemap_xml)?;

    let feed_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
  <title>Blog</title>
  <link>https://example.com</link>
</channel>
</rss>"#;
    env.add_template("feed", feed_xml)?;

    let posts_data: Vec<serde_json::Value> = posts
        .iter()
        .map(|p| {
            serde_json::json!({
                "title": p.0,
                "slug": p.1,
                "content": render_blocks(&p.2),
                "excerpt": p.3,
                "featured_image": p.4,
                "published_at": p.5.format("%Y-%m-%d").to_string(),
            })
        })
        .collect();

    let homepage = pages.iter().find(|p| p.3);
    let other_pages: Vec<_> = pages.iter().filter(|p| !p.3).collect();

    let output_dir = std::path::Path::new("output");
    std::fs::create_dir_all(&output_dir)?;

    let ctx = context! {
        site_name => site_name,
        site_description => site_description,
        logo_url => logo_url,
        nav_links => nav_links,
        footer_text => footer_text,
        social_links => social_links,
        contact_phone => contact_phone,
        contact_email => contact_email,
        contact_address => contact_address,
        posts => posts_data.clone(),
    };

    let index_template = env.get_template("index")?;
    let index_html = index_template.render(ctx)?;
    std::fs::write(output_dir.join("index.html"), index_html)?;

    // Generate blog listing page using page template for consistent styling
    let blog_ctx = context! {
        site_name => site_name,
        site_description => site_description,
        logo_url => logo_url,
        nav_links => nav_links,
        footer_text => footer_text,
        social_links => social_links,
        contact_phone => contact_phone,
        contact_email => contact_email,
        contact_address => contact_address,
        posts => posts_data.clone(),
        title => "Blog",
    };
    let page_template = env.get_template("page")?;
    let blog_html = page_template.render(blog_ctx)?;
    std::fs::write(output_dir.join("blog.html"), blog_html)?;

    for post in &posts {
        let post_ctx = context! {
            site_name => site_name,
            site_description => site_description,
            logo_url => logo_url,
            nav_links => nav_links,
            footer_text => footer_text,
            social_links => social_links,
            contact_phone => contact_phone,
            contact_email => contact_email,
            contact_address => contact_address,
            title => &post.0,
            slug => &post.1,
            content => render_blocks(&post.2),
            excerpt => &post.3,
            featured_image => &post.4,
            published_at => post.5.format("%Y-%m-%d").to_string(),
            url => format!("/{}", post.1),
        };
        let post_template = env.get_template("page")?;
        let post_html = post_template.render(post_ctx)?;
        std::fs::write(output_dir.join(format!("{}.html", post.1)), post_html)?;
    }

    if let Some(home) = homepage {
        let page_ctx = context! {
            site_name => site_name,
            site_description => site_description,
            logo_url => logo_url,
            nav_links => nav_links,
            footer_text => footer_text,
            social_links => social_links,
            contact_phone => contact_phone,
            contact_email => contact_email,
            contact_address => contact_address,
            title => &home.0,
            slug => &home.1,
            content => render_blocks(&home.2),
        };
        let page_template = env.get_template("page")?;
        let page_html = page_template.render(page_ctx)?;
        std::fs::write(output_dir.join("index.html"), page_html)?;
    }

    for page in other_pages {
        let page_ctx = context! {
            site_name => site_name,
            site_description => site_description,
            logo_url => logo_url,
            nav_links => nav_links,
            footer_text => footer_text,
            social_links => social_links,
            contact_phone => contact_phone,
            contact_email => contact_email,
            contact_address => contact_address,
            title => &page.0,
            slug => &page.1,
            content => render_blocks(&page.2),
            url => format!("/{}", page.1),
        };
        let page_template = env.get_template("page")?;
        let page_html = page_template.render(page_ctx)?;
        std::fs::write(output_dir.join(format!("{}.html", page.1)), page_html)?;
    }

    let sitemap_ctx = context! {
        posts => posts_data.iter().map(|p| p.get("slug").and_then(|s| s.as_str()).unwrap_or("")).collect::<Vec<_>>(),
    };
    let sitemap_template = env.get_template("sitemap")?;
    let sitemap_xml = sitemap_template.render(sitemap_ctx)?;
    std::fs::write(output_dir.join("sitemap.xml"), sitemap_xml)?;

    let feed_ctx = context! {
        site_name => site_name,
        site_description => site_description,
        posts => posts_data,
    };
    let feed_template = env.get_template("feed")?;
    let feed_xml = feed_template.render(feed_ctx)?;
    std::fs::write(output_dir.join("feed.xml"), feed_xml)?;

    tracing::info!("Built static site for site_id: {}", site_id);
    Ok(())
}

fn render_blocks(content: &serde_json::Value) -> String {
    if let Some(blocks) = content.as_array() {
        blocks.iter()
            .map(|block| {
                let block_type = block.get("block_type").and_then(|b| b.as_str()).unwrap_or("text");
                let block_content = block.get("content");

                match block_type {
                    "heading" => {
                        let level = block.get("level").and_then(|l| l.as_i64()).unwrap_or(2);
                        let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or("");
                        format!("<h{}>{}</h{}>", level, text, level)
                    }
                    "paragraph" => {
                        let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or("");
                        format!("<p>{}</p>", text)
                    }
                    "image" => {
                        let url = block_content.and_then(|c| c.get("url")).and_then(|u| u.as_str()).unwrap_or("");
                        let alt = block_content.and_then(|c| c.get("alt")).and_then(|a| a.as_str()).unwrap_or("");
                        format!("<figure><img src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>", url, alt, alt)
                    }
                    "code" => {
                        let code = block_content.and_then(|c| c.get("code")).and_then(|c| c.as_str()).unwrap_or("");
                        let lang = block_content.and_then(|c| c.get("language")).and_then(|l| l.as_str()).unwrap_or("");
                        format!("<pre><code class=\"language-{}\">{}</code></pre>", lang, code)
                    }
                    "quote" => {
                        let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or("");
                        let citation = block_content.and_then(|c| c.get("citation")).and_then(|c| c.as_str()).unwrap_or("");
                        format!("<blockquote>{}<cite>{}</cite></blockquote>", text, citation)
                    }
                    "hero" => {
                        let title = block_content.and_then(|c| c.get("title")).and_then(|t| t.as_str()).unwrap_or("");
                        let subtitle = block_content.and_then(|c| c.get("subtitle")).and_then(|t| t.as_str()).unwrap_or("");
                        let bg = block_content.and_then(|c| c.get("backgroundImage")).and_then(|t| t.as_str()).unwrap_or("");
                        let cta_text = block_content.and_then(|c| c.get("ctaText")).and_then(|t| t.as_str()).unwrap_or("");
                        let cta_link = block_content.and_then(|c| c.get("ctaLink")).and_then(|t| t.as_str()).unwrap_or("#");
                        let bg_style = if !bg.is_empty() {
                            format!("background-image: linear-gradient(rgba(0,0,0,0.6), rgba(0,0,0,0.6)), url('{}'); background-size: cover; background-position: center;", bg)
                        } else {
                            "background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);".to_string()
                        };
                        format!(r#"<div class="hero-section" style="{} padding: 80px 20px; text-align: center; color: white; border-radius: 12px; margin: 20px 0;">
                            <h1>{}</h1>
                            <p>{}</p>
                            {}
                        </div>"#, bg_style, title, subtitle, if !cta_text.is_empty() { format!("<a href=\"{}\" class=\"button\">{}</a>", cta_link, cta_text) } else { String::new() })
                    }
                    "video" => {
                        let url = block_content.and_then(|c| c.get("url")).and_then(|t| t.as_str()).unwrap_or("");
                        let caption = block_content.and_then(|c| c.get("caption")).and_then(|t| t.as_str()).unwrap_or("");
                        let embed_html = if url.contains("youtube.com") || url.contains("youtu.be") {
                            let video_id = if url.contains("v=") {
                                url.split("v=").nth(1).unwrap_or("").split('&').next().unwrap_or("")
                            } else {
                                url.split('/').next_back().unwrap_or("").split('?').next().unwrap_or("")
                            };
                            format!("<iframe width=\"100%\" height=\"400\" src=\"https://www.youtube.com/embed/{}\" frameborder=\"0\" allowfullscreen></iframe>", video_id)
                        } else if url.contains("vimeo.com") {
                            let video_id = url.split('/').next_back().unwrap_or("");
                            format!("<iframe width=\"100%\" height=\"400\" src=\"https://player.vimeo.com/video/{}\" frameborder=\"0\" allowfullscreen></iframe>", video_id)
                        } else if url.ends_with(".mp4") {
                            format!("<video width=\"100%\" controls><source src=\"{}\" type=\"video/mp4\">Your browser does not support video.</video>", url)
                        } else {
                            String::new()
                        };
                        if !url.is_empty() {
                            format!("<div class=\"video-block\">{}{}", embed_html, if !caption.is_empty() { format!("<p class=\"caption\">{}</p>", caption) } else { String::new() })
                        } else { String::new() }
                    }
                    "columns" => {
                        let left = block_content.and_then(|c| c.get("left")).and_then(|t| t.as_str()).unwrap_or("");
                        let right = block_content.and_then(|c| c.get("right")).and_then(|t| t.as_str()).unwrap_or("");
                        let left_img = block_content.and_then(|c| c.get("leftImage")).and_then(|t| t.as_str()).unwrap_or("");
                        let right_img = block_content.and_then(|c| c.get("rightImage")).and_then(|t| t.as_str()).unwrap_or("");
                        format!(r#"<div class="columns-block" style="display: grid; grid-template-columns: 1fr 1fr; gap: 2rem; margin: 2rem 0;">
                            <div class="left-col">
                                {} {}
                            </div>
                            <div class="right-col">
                                {} {}
                            </div>
                        </div>"#, 
                            if !left_img.is_empty() { format!("<img src=\"{}\" style=\"max-width:100%%; border-radius:8px;\">", left_img) } else { String::new() },
                            if !left.is_empty() { format!("<p>{}</p>", left) } else { String::new() },
                            if !right_img.is_empty() { format!("<img src=\"{}\" style=\"max-width:100%%; border-radius:8px;\">", right_img) } else { String::new() },
                            if !right.is_empty() { format!("<p>{}</p>", right) } else { String::new() }
                        )
                    }
                    _ => {
                        let text = block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or("");
                        format!("<p>{}</p>", text)
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    }
}
