use minijinja::Environment;
use sqlx::Row;
use uuid::Uuid;

/// Escape HTML special characters to prevent XSS attacks
fn escape_html<S: AsRef<str>>(s: S) -> String {
    let s = s.as_ref();
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Validate and sanitize URL to prevent XSS attacks via javascript:, data:, etc.
/// Returns None if URL is unsafe, otherwise returns the sanitized URL
fn sanitize_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }

    // Check for dangerous URL schemes
    let lower = url.to_lowercase();
    if lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
        || lower.starts_with("file:")
    {
        return None;
    }

    // Only allow http, https, and relative URLs
    if !lower.starts_with("http://")
        && !lower.starts_with("https://")
        && !lower.starts_with('/')
        && !lower.starts_with("data:")
    {
        // Allow common relative paths
        if !url.starts_with("..") && !url.contains("..") {
            Some(url.to_string())
        } else {
            None
        }
    } else {
        Some(url.to_string())
    }
}

fn extract_first_image(content: &serde_json::Value) -> Option<String> {
    if let Some(blocks) = content.as_array() {
        for block in blocks {
            if let Some(block_type) = block.get("block_type").and_then(|t| t.as_str()) {
                if block_type == "image" {
                    if let Some(img_content) = block.get("content") {
                        if let Some(url) = img_content.get("url").and_then(|u| u.as_str()) {
                            if !url.is_empty() {
                                return Some(url.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

type Context = std::collections::HashMap<String, minijinja::Value>;

#[allow(clippy::too_many_arguments)]
fn make_context(
    site_name: &str,
    site_description: &Option<String>,
    logo_url: &Option<String>,
    favicon_url: &Option<String>,
    nav_links: &[serde_json::Value],
    footer_text: &Option<String>,
    social_links: &serde_json::Value,
    contact_phone: &Option<String>,
    contact_email: &Option<String>,
    contact_address: &Option<String>,
) -> Context {
    let mut ctx = Context::new();
    ctx.insert("site_name".into(), minijinja::Value::from(site_name));
    ctx.insert(
        "build_timestamp".into(),
        minijinja::Value::from(chrono::Utc::now().to_rfc3339()),
    );
    ctx.insert(
        "site_description".into(),
        minijinja::Value::from_serialize(site_description),
    );
    ctx.insert(
        "logo_url".into(),
        minijinja::Value::from_serialize(logo_url),
    );
    ctx.insert(
        "favicon_url".into(),
        minijinja::Value::from_serialize(favicon_url),
    );
    ctx.insert(
        "nav_links".into(),
        minijinja::Value::from_serialize(nav_links),
    );
    ctx.insert(
        "footer_text".into(),
        minijinja::Value::from_serialize(footer_text),
    );
    ctx.insert(
        "social_links".into(),
        minijinja::Value::from_serialize(social_links),
    );
    ctx.insert(
        "contact_phone".into(),
        minijinja::Value::from_serialize(contact_phone),
    );
    ctx.insert(
        "contact_email".into(),
        minijinja::Value::from_serialize(contact_email),
    );
    ctx.insert(
        "contact_address".into(),
        minijinja::Value::from_serialize(contact_address),
    );
    ctx
}

pub async fn build_site(
    db: &sqlx::PgPool,
    site_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let site_row = sqlx::query(
        "SELECT id, name, description, logo_url, favicon_url, footer_text, social_links, contact_phone, contact_email, contact_address, custom_domain, homepage_type, blog_path, blog_sort_order FROM sites WHERE id = $1"
    )
    .bind(site_id)
    .fetch_one(db)
    .await?;

    // Get current pages from DB to know what to keep
    let pages: Vec<(String,)> = sqlx::query_as("SELECT slug FROM pages WHERE site_id = $1")
        .bind(site_id)
        .fetch_all(db)
        .await?;

    let page_slugs: Vec<String> = pages.iter().map(|p| p.0.clone()).collect();

    // Get homepage_type to determine if blog should exist
    let homepage_type: Option<String> = site_row.get("homepage_type");
    let homepage_type = homepage_type.unwrap_or_else(|| "both".to_string());
    let blog_enabled = homepage_type == "blog" || homepage_type == "both";

    // Clean up old output files - remove HTML files for deleted pages
    let output_dir = std::path::Path::new("output");
    if output_dir.exists() {
        // Keep these special files (blog.html only if blog is enabled)
        let mut keep_files = vec!["index.html", "feed.xml", "sitemap.xml"];
        if blog_enabled {
            keep_files.push("blog.html");
        }
        let keep_files: Vec<&str> = keep_files;

        if let Ok(entries) = std::fs::read_dir(output_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        // Skip special files
                        if keep_files.contains(&filename) || filename.starts_with("blog/") {
                            continue;
                        }
                        // Remove .html files not in the page list
                        if filename.ends_with(".html") {
                            let slug = filename.trim_end_matches(".html");
                            if !page_slugs.contains(&slug.to_string()) {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
    }

    let site_id: Uuid = site_row.get("id");
    let site_name: String = site_row.get("name");
    let site_description: Option<String> = site_row.get("description");
    let logo_url: Option<String> = site_row.get("logo_url");
    let favicon_url: Option<String> = site_row.get("favicon_url");
    let footer_text: Option<String> = site_row.get("footer_text");
    let social_links: serde_json::Value = site_row
        .get::<Option<serde_json::Value>, _>("social_links")
        .unwrap_or(serde_json::json!({}));
    let contact_phone: Option<String> = site_row.get("contact_phone");
    let contact_email: Option<String> = site_row.get("contact_email");
    let contact_address: Option<String> = site_row.get("contact_address");
    let domain: Option<String> = site_row.get("custom_domain");
    let homepage_type: Option<String> = site_row.get("homepage_type");
    let blog_path: Option<String> = site_row.get("blog_path");
    let blog_sort_order: Option<i32> = site_row.get("blog_sort_order");

    // Build site URL - use custom_domain as the primary domain
    let site_url = if let Some(d) = domain {
        if d.starts_with("http") {
            d
        } else {
            format!("https://{}", d)
        }
    } else {
        "https://example.com".to_string()
    };

    let posts = sqlx::query_as::<_, (
        String, String, serde_json::Value, Option<String>, Option<String>, Option<chrono::DateTime<chrono::Utc>>
    )>(
        "SELECT title, slug, content, excerpt, featured_image, published_at FROM posts WHERE site_id = $1 AND status = 'published' ORDER BY published_at DESC"
    )
    .bind(site_id)
    .fetch_all(db)
    .await?;

    let pages = sqlx::query_as::<_, (String, String, serde_json::Value, bool, bool, i32)>(
        "SELECT title, slug, content, is_homepage, show_in_nav, sort_order FROM pages WHERE site_id = $1 ORDER BY sort_order ASC",
    )
    .bind(site_id)
    .fetch_all(db)
    .await?;

    // Build nav_links from pages with show_in_nav = true (excluding homepage which is always at /)
    let nav_pages: Vec<_> = pages.iter().filter(|p| p.4 && !p.3).collect();
    let mut nav_links: Vec<serde_json::Value> = nav_pages
        .iter()
        .map(|p| {
            serde_json::json!({
                "label": p.0,
                "url": format!("/{}", p.1),
                "sort_order": p.5
            })
        })
        .collect();

    // Add Blog link if homepage_type is 'blog' or 'both'
    let homepage_type = homepage_type.unwrap_or_else(|| "both".to_string());
    let blog_order = blog_sort_order.unwrap_or(1);
    if homepage_type == "blog" || homepage_type == "both" {
        let blog_url = blog_path
            .filter(|p| !p.is_empty())
            .map(|p| {
                if p.starts_with('/') {
                    p
                } else {
                    format!("/{}", p)
                }
            })
            .unwrap_or_else(|| "/blog".to_string());
        nav_links
            .push(serde_json::json!({"label": "Blog", "url": blog_url, "sort_order": blog_order}));
    }

    // Sort all nav links by sort_order (default to large number for pages without explicit order)
    nav_links.sort_by(|a, b| {
        let order_a = a.get("sort_order").and_then(|v| v.as_i64()).unwrap_or(100) as i32;
        let order_b = b.get("sort_order").and_then(|v| v.as_i64()).unwrap_or(100) as i32;
        order_a.cmp(&order_b)
    });

    let mut env = Environment::new();

    // Add custom filter for JSON-LD escaping (prevents XSS in script tags)
    env.add_filter("json_escape", |s: String| {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
            .replace("</", "<\\/")
    });

    // Find templates directory
    let cwd = std::env::current_dir().unwrap_or_default();
    let template_dir = cwd.join("templates");

    let base_html = std::fs::read_to_string(template_dir.join("base.html"))
        .map_err(|e| format!("Failed to read base.html: {}", e))?;
    let page_html = std::fs::read_to_string(template_dir.join("page.html"))
        .map_err(|e| format!("Failed to read page.html: {}", e))?;
    let index_html = std::fs::read_to_string(template_dir.join("index.html"))
        .map_err(|e| format!("Failed to read index.html: {}", e))?;

    // Load templates
    env.add_template("base.html", &base_html)?;
    env.add_template("page.html", &page_html)?;
    env.add_template("index.html", &index_html)?;

    // Get non-homepage pages for sitemap (exclude 'blog' since it's handled specially)
    let sitemap_pages: Vec<String> = pages
        .iter()
        .filter(|p| !p.3 && p.1 != "blog") // exclude homepage and blog page
        .map(|p| p.1.clone())
        .collect();

    let sitemap_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<url>
    <loc>{}/</loc>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
</url>
<url>
    <loc>{}/blog</loc>
    <changefreq>daily</changefreq>
    <priority>0.9</priority>
</url>
{}
{}
</urlset>"#,
        site_url,
        site_url,
        sitemap_pages
            .iter()
            .map(|slug| format!(
                r#"<url>
    <loc>{}/{}</loc>
    <changefreq>monthly</changefreq>
    <priority>0.6</priority>
</url>"#,
                site_url, slug
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        posts
            .iter()
            .map(|p| format!(
                r#"<url>
    <loc>{}/blog/{}</loc>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
</url>"#,
                site_url, p.1
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let feed_items: Vec<String> = posts
        .iter()
        .map(|p| {
            format!(
                r#"<item>
        <title><![CDATA[{}]]></title>
        <link>{}/blog/{}</link>
        <guid isPermaLink="true">{}/blog/{}</guid>
        <pubDate>{}</pubDate>
        <description><![CDATA[{}]]></description>
    </item>"#,
                p.0,
                site_url,
                p.1,
                site_url,
                p.1,
                p.5.map(|dt| dt.format("%a, %d %b %Y %H:%M:%S +0000").to_string())
                    .unwrap_or_else(|| chrono::Utc::now()
                        .format("%a, %d %b %Y %H:%M:%S +0000")
                        .to_string()),
                p.3.as_deref().unwrap_or("")
            )
        })
        .collect();

    let feed_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
    <title>{}</title>
    <link>{}</link>
    <description>{}</description>
    <language>en-us</language>
    <lastBuildDate>{}</lastBuildDate>
    <atom:link href="{}/feed.xml" rel="self" type="application/rss+xml"/>
{}
</channel>
</rss>"#,
        site_name,
        site_url,
        site_description.as_deref().unwrap_or(&site_name),
        chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S +0000"),
        site_url,
        feed_items.join("\n")
    );

    let posts_data: Vec<serde_json::Value> = posts
        .iter()
        .map(|p| {
            serde_json::json!({
                "title": p.0,
                "slug": p.1,
                "content": render_blocks(&p.2),
                "excerpt": p.3,
                "featured_image": p.4,
                "published_at": p.5.map(|dt| dt.format("%Y-%m-%d").to_string()).unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
            })
        })
        .collect();

    let homepage = pages.iter().find(|p| p.3);
    let other_pages: Vec<_> = pages.iter().filter(|p| !p.3).collect();

    let output_dir = std::path::Path::new("output");
    std::fs::create_dir_all(output_dir)?;

    // Build base context once
    let mut ctx = make_context(
        &site_name,
        &site_description,
        &logo_url,
        &favicon_url,
        &nav_links,
        &footer_text,
        &social_links,
        &contact_phone,
        &contact_email,
        &contact_address,
    );
    ctx.insert(
        "posts".into(),
        minijinja::Value::from_serialize(&posts_data),
    );
    ctx.insert("url".into(), minijinja::Value::from("/".to_string()));

    let index_template = env.get_template("index.html")?;
    let index_html = index_template.render(&ctx)?;
    std::fs::write(output_dir.join("index.html"), index_html)?;

    // Individual blog post pages are generated in the page loop below

    for post in &posts {
        // Use featured_image from DB, or extract first image from content
        let featured_img = post.4.clone().or_else(|| extract_first_image(&post.2));

        let mut post_ctx = make_context(
            &site_name,
            &site_description,
            &logo_url,
            &favicon_url,
            &nav_links,
            &footer_text,
            &social_links,
            &contact_phone,
            &contact_email,
            &contact_address,
        );
        post_ctx.insert("title".into(), minijinja::Value::from(post.0.clone()));
        post_ctx.insert("slug".into(), minijinja::Value::from(post.1.clone()));
        post_ctx.insert(
            "content".into(),
            minijinja::Value::from(render_blocks(&post.2)),
        );
        post_ctx.insert("excerpt".into(), minijinja::Value::from_serialize(&post.3));
        post_ctx.insert(
            "featured_image".into(),
            minijinja::Value::from_serialize(&featured_img),
        );
        post_ctx.insert(
            "published_at".into(),
            minijinja::Value::from(
                post.5
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
            ),
        );
        post_ctx.insert(
            "url".into(),
            minijinja::Value::from(format!("/blog/{}", post.1)),
        );
        post_ctx.insert("is_blog_post".into(), minijinja::Value::from(true));

        let meta_desc = post
            .3
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| extract_plain_text(&post.2));
        post_ctx.insert(
            "meta_description".into(),
            minijinja::Value::from(meta_desc.clone()),
        );
        post_ctx.insert("description".into(), minijinja::Value::from(meta_desc));

        let post_template = env.get_template("page.html")?;
        let post_html = post_template.render(&post_ctx)?;
        let blog_dir = output_dir.join("blog");
        std::fs::create_dir_all(&blog_dir)?;
        std::fs::write(blog_dir.join(format!("{}.html", post.1)), post_html)?;
    }

    if let Some(home) = homepage {
        let mut page_ctx = make_context(
            &site_name,
            &site_description,
            &logo_url,
            &favicon_url,
            &nav_links,
            &footer_text,
            &social_links,
            &contact_phone,
            &contact_email,
            &contact_address,
        );
        page_ctx.insert("title".into(), minijinja::Value::from(home.0.clone()));
        page_ctx.insert("slug".into(), minijinja::Value::from(home.1.clone()));
        page_ctx.insert(
            "content".into(),
            minijinja::Value::from(render_blocks(&home.2)),
        );
        page_ctx.insert("url".into(), minijinja::Value::from("/".to_string()));

        let page_template = env.get_template("page.html")?;
        let page_html = page_template.render(&page_ctx)?;
        std::fs::write(output_dir.join("index.html"), page_html)?;
    }

    for page in other_pages {
        let is_blog = page.1 == "blog";

        let mut page_ctx = make_context(
            &site_name,
            &site_description,
            &logo_url,
            &favicon_url,
            &nav_links,
            &footer_text,
            &social_links,
            &contact_phone,
            &contact_email,
            &contact_address,
        );
        page_ctx.insert("title".into(), minijinja::Value::from(page.0.clone()));
        page_ctx.insert("slug".into(), minijinja::Value::from(page.1.clone()));
        page_ctx.insert(
            "content".into(),
            minijinja::Value::from(render_blocks(&page.2)),
        );
        page_ctx.insert("url".into(), minijinja::Value::from(format!("/{}", page.1)));

        if is_blog {
            page_ctx.insert(
                "posts".into(),
                minijinja::Value::from_serialize(&posts_data),
            );
        }

        let page_template = env.get_template("page.html")?;
        let page_html = page_template.render(&page_ctx)?;
        std::fs::write(output_dir.join(format!("{}.html", page.1)), page_html)?;
    }

    // Generate blog.html if blog is enabled but no blog page exists
    if blog_enabled && !page_slugs.contains(&"blog".to_string()) {
        let mut page_ctx = make_context(
            &site_name,
            &site_description,
            &logo_url,
            &favicon_url,
            &nav_links,
            &footer_text,
            &social_links,
            &contact_phone,
            &contact_email,
            &contact_address,
        );
        page_ctx.insert("title".into(), minijinja::Value::from("Blog".to_string()));
        page_ctx.insert("slug".into(), minijinja::Value::from("blog".to_string()));
        page_ctx.insert("content".into(), minijinja::Value::from("".to_string()));
        page_ctx.insert("url".into(), minijinja::Value::from("/blog".to_string()));
        page_ctx.insert(
            "posts".into(),
            minijinja::Value::from_serialize(&posts_data),
        );

        let page_template = env.get_template("page.html")?;
        let page_html = page_template.render(&page_ctx)?;
        std::fs::write(output_dir.join("blog.html"), page_html)?;
    }

    // Write sitemap - already generated inline above
    std::fs::write(output_dir.join("sitemap.xml"), &sitemap_xml)?;

    // Write feed - already generated inline above
    std::fs::write(output_dir.join("feed.xml"), &feed_xml)?;

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
                        let text = escape_html(block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or(""));
                        format!("<h{}>{}</h{}>", level, text, level)
                    }
                    "paragraph" => {
                        let text = escape_html(block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or(""));
                        format!("<p>{}</p>", text)
                    }
                    "image" => {
                        let url = sanitize_url(block_content.and_then(|c| c.get("url")).and_then(|u| u.as_str()).unwrap_or_default()).unwrap_or_default();
                        let alt = escape_html(block_content.and_then(|c| c.get("alt")).and_then(|a| a.as_str()).unwrap_or(""));
                        format!("<figure><img src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>", url, alt, alt)
                    }
                    "code" => {
                        let code = escape_html(block_content.and_then(|c| c.get("code")).and_then(|c| c.as_str()).unwrap_or(""));
                        let lang = block_content.and_then(|c| c.get("language")).and_then(|l| l.as_str()).unwrap_or("");
                        format!("<pre><code class=\"language-{}\">{}</code></pre>", lang, code)
                    }
                    "quote" => {
                        let text = escape_html(block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or(""));
                        let citation = escape_html(block_content.and_then(|c| c.get("citation")).and_then(|c| c.as_str()).unwrap_or(""));
                        format!("<blockquote>{}<cite>{}</cite></blockquote>", text, citation)
                    }
                    "hero" => {
                        let title = escape_html(block_content.and_then(|c| c.get("title")).and_then(|t| t.as_str()).unwrap_or(""));
                        let subtitle = escape_html(block_content.and_then(|c| c.get("subtitle")).and_then(|t| t.as_str()).unwrap_or(""));
                        let bg = sanitize_url(block_content.and_then(|c| c.get("backgroundImage")).and_then(|t| t.as_str()).unwrap_or_default()).unwrap_or_default();
                        let cta_text = escape_html(block_content.and_then(|c| c.get("ctaText")).and_then(|t| t.as_str()).unwrap_or(""));
                        let cta_link = sanitize_url(block_content.and_then(|c| c.get("ctaLink")).and_then(|t| t.as_str()).unwrap_or_default()).unwrap_or_else(|| "#".to_string());
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
                        let url = escape_html(block_content.and_then(|c| c.get("url")).and_then(|t| t.as_str()).unwrap_or(""));
                        let caption = escape_html(block_content.and_then(|c| c.get("caption")).and_then(|t| t.as_str()).unwrap_or(""));
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
                        let left = escape_html(block_content.and_then(|c| c.get("left")).and_then(|t| t.as_str()).unwrap_or(""));
                        let right = escape_html(block_content.and_then(|c| c.get("right")).and_then(|t| t.as_str()).unwrap_or(""));
                        let left_img = sanitize_url(block_content.and_then(|c| c.get("leftImage")).and_then(|t| t.as_str()).unwrap_or("")).unwrap_or_default();
                        let right_img = sanitize_url(block_content.and_then(|c| c.get("rightImage")).and_then(|t| t.as_str()).unwrap_or("")).unwrap_or_default();
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
                        let text = escape_html(block_content.and_then(|c| c.get("text")).and_then(|t| t.as_str()).unwrap_or(""));
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

fn extract_plain_text(content: &serde_json::Value) -> String {
    if let Some(blocks) = content.as_array() {
        let mut text = String::new();

        for block in blocks.iter() {
            let block_type = block
                .get("block_type")
                .and_then(|b| b.as_str())
                .unwrap_or("text");
            let block_content = block.get("content");

            let block_text: String = match block_type {
                "heading" => block_content
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string(),
                "paragraph" => block_content
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string(),
                "quote" => block_content
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string(),
                "hero" => {
                    let title = block_content
                        .and_then(|c| c.get("title"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    let subtitle = block_content
                        .and_then(|c| c.get("subtitle"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    format!("{} {}", title, subtitle)
                }
                "columns" => {
                    let left = block_content
                        .and_then(|c| c.get("left"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    let right = block_content
                        .and_then(|c| c.get("right"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    format!("{} {}", left, right)
                }
                _ => block_content
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string(),
            };

            if !block_text.trim().is_empty() {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(block_text.trim());
            }
        }

        if text.len() > 160 {
            format!("{}...", &text[..157])
        } else {
            text
        }
    } else {
        String::new()
    }
}

pub async fn deploy_to_cloudflare() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let project_name = std::env::var("CLOUDFLARE_PAGES_PROJECT")
        .map_err(|_| "CLOUDFLARE_PAGES_PROJECT not set")?;

    let output_dir = std::path::Path::canonicalize(std::path::Path::new("output"))
        .map_err(|_| "Output directory does not exist")?;

    if !output_dir.exists() {
        return Err("Output directory does not exist. Build the site first.".into());
    }

    tracing::info!(
        "Starting Cloudflare deployment using wrangler for project: {}",
        project_name
    );

    // Run wrangler pages deploy
    let output = tokio::process::Command::new("wrangler")
        .args([
            "pages",
            "deploy",
            output_dir.to_str().unwrap_or("output"),
            "--project-name",
            &project_name,
            "--branch",
            "main",
            "--no-bundle",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run wrangler: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    tracing::info!("Wrangler output: {}", stdout);

    if !output.status.success() {
        tracing::error!("Wrangler error: {}", stderr);
        return Err(format!("Wrangler deployment failed: {}", stderr).into());
    }

    // Try to extract the URL from wrangler output
    let deployment_url = stdout
        .lines()
        .find(|l| l.contains("pages.dev"))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| format!("https://{}.pages.dev", project_name));

    Ok(format!("Deployed successfully! Visit: {}", deployment_url))
}
