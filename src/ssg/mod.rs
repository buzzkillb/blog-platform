use minijinja::Environment;
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DbTemplate {
    pub id: Uuid,
    pub name: String,
    pub html_content: Option<String>,
    pub css_content: Option<String>,
    pub default_config: serde_json::Value,
}

pub async fn load_template_by_id(
    db: &sqlx::PgPool,
    template_id: Uuid,
) -> Result<DbTemplate, Box<dyn std::error::Error + Send + Sync>> {
    let row = sqlx::query(
        "SELECT id, name, html_content, css_content, default_config FROM templates WHERE id = $1",
    )
    .bind(template_id)
    .fetch_one(db)
    .await?;

    Ok(DbTemplate {
        id: row.get("id"),
        name: row.get("name"),
        html_content: row.get("html_content"),
        css_content: row.get("css_content"),
        default_config: row.get("default_config"),
    })
}

fn merge_configs(
    default_config: &serde_json::Value,
    site_config: &serde_json::Value,
) -> serde_json::Value {
    let mut merged = default_config.clone();

    if let (Some(def_obj), Some(site_obj)) = (merged.as_object(), site_config.as_object()) {
        let mut new_merged = def_obj.clone();
        for (key, value) in site_obj {
            new_merged.insert(key.clone(), value.clone());
        }
        merged = serde_json::Value::Object(new_merged);
    } else if site_config != &serde_json::Value::Null {
        merged = site_config.clone();
    }

    merged
}

fn get_theme_css(config: &serde_json::Value, theme: &str) -> String {
    let mut css_vars = String::new();

    // Handle nested structure (dark/colors keys)
    let theme_data = if theme == "dark" {
        config.get("dark").or_else(|| config.get("colors"))
    } else {
        config.get("light").or_else(|| config.get("colors"))
    };

    if let Some(colors) = theme_data {
        if let Some(obj) = colors.as_object() {
            for (key, value) in obj {
                if let Some(color) = value.as_str() {
                    css_vars.push_str(&format!("  --{}: {};\n", key, color));
                }
            }
        }
    }

    // Also check flat structure (direct keys like accent_color)
    if let Some(obj) = config.as_object() {
        for (key, value) in obj {
            let key_lower = key.to_lowercase();
            if key_lower.contains("color") || key_lower.contains("background") || key_lower.contains("text") {
                if let Some(color) = value.as_str() {
                    css_vars.push_str(&format!("  --{}: {};\n", key, color));
                }
            }
        }
    }

    // Add fonts
    if let Some(fonts) = config.get("fonts") {
        if let Some(obj) = fonts.as_object() {
            for (key, value) in obj {
                if let Some(font) = value.as_str() {
                    css_vars.push_str(&format!("  --font-{}: {};\n", key, font));
                }
            }
        }
    }

    // Add direct font keys
    if let Some(obj) = config.as_object() {
        for (key, value) in obj {
            let key_lower = key.to_lowercase();
            if key_lower.contains("font") {
                if let Some(font) = value.as_str() {
                    css_vars.push_str(&format!("  --{}: {};\n", key, font));
                }
            }
        }
    }

    css_vars
}

fn inject_theme_vars(html: &str, css_vars: &str, theme: &str) -> String {
    let theme_attr = format!("data-theme=\"{}\"", theme);
    let style_block = format!("<style>\n:root {{\n{}}}\n</style>", css_vars);

    html.replace("<html", &format!("<html {}", theme_attr))
        .replace("<head>", &format!("<head>\n{}", style_block))
}

/// Escape HTML special characters to prevent XSS attacks
fn escape_html<S: AsRef<str>>(s: S) -> String {
    let s = s.as_ref();
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
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
        "SELECT id, name, description, logo_url, favicon_url, footer_text, social_links, contact_phone, contact_email, contact_address, custom_domain, COALESCE(homepage_type, 'both') as homepage_type, COALESCE(blog_path, '/blog') as blog_path, COALESCE(blog_sort_order, 1) as blog_sort_order FROM sites WHERE id = $1"
    )
    .bind(site_id)
    .fetch_one(db)
    .await?;

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
    let blog_sort_order: i32 = site_row.get("blog_sort_order");
    let homepage_type = homepage_type.unwrap_or_else(|| "both".to_string());
    let blog_path = blog_path.unwrap_or_else(|| "/blog".to_string());

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

    // Build nav_links from pages with show_in_nav = true
    // Include homepage at position 0, then other pages sorted by sort_order
    let mut nav_links: Vec<serde_json::Value> = Vec::new();

    // Add homepage first if show_in_nav is true
    if let Some(homepage) = pages.iter().find(|p| p.3) {
        if homepage.4 {
            nav_links.push(serde_json::json!({
                "label": homepage.0,
                "url": "/"
            }));
        }
    }

    // Add other pages sorted by sort_order
    let other_pages: Vec<(String, String, serde_json::Value, bool, bool, i32)> = pages
        .iter()
        .filter(|p| !p.3)
        .map(|p| (p.0.clone(), p.1.clone(), p.2.clone(), p.3, p.4, p.5))
        .collect();
    for page in &other_pages {
        if page.4 {
            nav_links.push(serde_json::json!({
                "label": page.0,
                "url": format!("/{}", page.1)
            }));
        }
    }

    // Add blog link at the correct position based on blog_sort_order if homepage_type is "blog" or "both"
    if homepage_type == "blog" || homepage_type == "both" {
        let blog_link = serde_json::json!({
            "label": "Blog",
            "url": blog_path
        });
        // Insert at blog_sort_order position (convert from 1-based to 0-based)
        let insert_pos = (blog_sort_order as usize)
            .saturating_sub(1)
            .min(nav_links.len());
        nav_links.insert(insert_pos, blog_link);
    }

    let mut env = Environment::new();
    let mut template_base: Option<String> = None;
    let mut template_page: Option<String> = None;
    let mut template_index: Option<String> = None;

    // Skip database template loading for now - use file templates
    // TODO: Re-enable after debugging
    /*
    if let Some(tid) = template_id {
        match load_template_by_id(db, tid).await {
            Ok(db_template) => {
                let merged_config = merge_configs(&db_template.default_config, &template_config);
                let theme_css = get_theme_css(&merged_config, &theme);

                if let Some(html_content) = db_template.html_content {
                    // Check if template uses markers (<!--TEMPLATE:xxx-->)
                    if html_content.contains("<!--TEMPLATE:") {
                        // Multi-part template
                        for template_part in html_content.split("<!--TEMPLATE:").skip(1) {
                            if let Some((name, rest)) = template_part.split_once("-->") {
                                let name = name.trim();
                                if let Some((content, _)) = rest.split_once("<!--END_TEMPLATE-->") {
                                    let content = content.trim();
                                    let processed = inject_theme_vars(content, &theme_css, &theme);

                                    match name {
                                        "base.html" => {
                                            template_base = Some(processed.clone());
                                            template_page = Some(processed.clone());
                                            template_index = Some(processed);
                                        }
                                        "page.html" => {
                                            template_page = Some(processed.clone());
                                        }
                                        "index.html" => {
                                            template_index = Some(processed);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    } else {
                        // Single HTML template - use for all
                        let processed = inject_theme_vars(&html_content, &theme_css, &theme);
                        template_base = Some(processed.clone());
                        template_page = Some(processed.clone());
                        template_index = Some(processed);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load template {}: {}, falling back to file templates",
                    tid,
                    e
                );
            }
        }
    }
    */

    // Load file-based templates if no database templates
    if template_base.is_none() {
        let cwd = std::env::current_dir().unwrap_or_default();
        let template_dir = cwd.join("templates");

        template_base = Some(
            std::fs::read_to_string(template_dir.join("base.html"))
                .map_err(|e| format!("Failed to read base.html: {}", e))?,
        );
        template_page = Some(
            std::fs::read_to_string(template_dir.join("page.html"))
                .map_err(|e| format!("Failed to read page.html: {}", e))?,
        );
        template_index = Some(
            std::fs::read_to_string(template_dir.join("index.html"))
                .map_err(|e| format!("Failed to read index.html: {}", e))?,
        );
    }

    // Add templates to environment (they now live long enough)
    if let Some(ref t) = template_base {
        env.add_template("base.html", t)?;
    }
    if let Some(ref t) = template_page {
        env.add_template("page.html", t)?;
    }
    if let Some(ref t) = template_index {
        env.add_template("index.html", t)?;
    }

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
    let other_pages: Vec<(String, String, serde_json::Value, bool, bool, i32)> = pages
        .iter()
        .filter(|p| !p.3)
        .map(|p| (p.0.clone(), p.1.clone(), p.2.clone(), p.3, p.4, p.5))
        .collect();

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
    ctx.insert("url".into(), minijinja::Value::from("/"));

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
                let block_type = block.get("type").or_else(|| block.get("block_type")).and_then(|b| b.as_str()).unwrap_or("text");
                let block_content = block.get("content");

                match block_type {
                    "heading" => {
                        let level = block.get("level").and_then(|l| l.as_i64()).unwrap_or(2);
                        let text = block_content
                            .and_then(|c| c.get("text"))
                            .and_then(|t| t.as_str())
                            .unwrap_or("");
                        let text = escape_html(text);
                        format!("<h{}>{}</h{}>", level, text, level)
                    }
                    "paragraph" => {
                        let text = if let Some(s) = block_content.and_then(|c| c.as_str()) {
                            s.to_string()
                        } else {
                            block_content
                                .and_then(|c| c.get("text"))
                                .and_then(|t| t.as_str())
                                .map(escape_html)
                                .unwrap_or_default()
                        };
                        if text.is_empty() {
                            String::new()
                        } else if text.contains('<') && text.contains('>') {
                            text.clone()
                        } else {
                            format!("<p>{}</p>", text)
                        }
                    }
                    "image" => {
                        let url = if let Some(s) = block_content.and_then(|c| c.as_str()) {
                            escape_html(s)
                        } else {
                            escape_html(block_content.and_then(|c| c.get("url")).and_then(|u| u.as_str()).unwrap_or(""))
                        };
                        let alt = block_content
                            .and_then(|c| c.get("alt"))
                            .and_then(|a| a.as_str())
                            .map(escape_html)
                            .unwrap_or_default();
                        if url.is_empty() {
                            String::new()
                        } else {
                            format!("<figure><img src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>", url, alt, alt)
                        }
                    }
                    "code" => {
                        let code = escape_html(block_content.and_then(|c| c.get("code")).and_then(|c| c.as_str()).unwrap_or(""));
                        let lang = block_content.and_then(|c| c.get("language")).and_then(|l| l.as_str()).unwrap_or("");
                        format!("<pre><code class=\"language-{}\">{}</code></pre>", lang, code)
                    }
                    "quote" => {
                        let text = block_content
                            .and_then(|c| c.get("text"))
                            .and_then(|t| t.as_str())
                            .map(escape_html)
                            .unwrap_or_default();
                        let citation = block_content
                            .and_then(|c| c.get("citation"))
                            .and_then(|c| c.as_str())
                            .map(escape_html)
                            .unwrap_or_default();
                        format!("<blockquote>{}<cite>{}</cite></blockquote>", text, citation)
                    }
                    "hero" => {
                        let title = escape_html(block_content.and_then(|c| c.get("title")).and_then(|t| t.as_str()).unwrap_or(""));
                        let subtitle = escape_html(block_content.and_then(|c| c.get("subtitle")).and_then(|t| t.as_str()).unwrap_or(""));
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
                        let left = block_content
                            .and_then(|c| c.get("left"))
                            .and_then(|t| t.as_str())
                            .map(escape_html)
                            .unwrap_or_default();
                        let right = block_content
                            .and_then(|c| c.get("right"))
                            .and_then(|t| t.as_str())
                            .map(escape_html)
                            .unwrap_or_default();
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
                        let text = if let Some(s) = block_content.and_then(|c| c.as_str()) {
                            s.to_string()
                        } else {
                            block_content
                                .and_then(|c| c.get("text"))
                                .and_then(|t| t.as_str())
                                .map(escape_html)
                                .unwrap_or_default()
                        };
                        if text.is_empty() {
                            String::new()
                        } else {
                            format!("<p>{}</p>", text)
                        }
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
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

    let deployment_url = stdout
        .lines()
        .find(|l| l.contains("pages.dev"))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| format!("https://{}.pages.dev", project_name));

    Ok(format!("Deployed successfully! Visit: {}", deployment_url))
}
