pub fn generate_slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub fn is_valid_url(url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() {
        return true; // Empty is allowed (optional field)
    }

    let lower = url.to_lowercase();

    // Reject dangerous URL schemes
    if lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
        || lower.starts_with("file:")
    {
        return false;
    }

    // Allow http, https, or relative URLs
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || (!url.contains("://") && !url.starts_with("javascript"))
    {
        true
    } else {
        false
    }
}

pub fn validate_file_content(content: &[u8], filename: &str) -> Result<(), String> {
    let lower = filename.to_lowercase();

    // Check magic bytes for common file types
    if lower.ends_with(".png") {
        if content.len() < 8 || &content[0..8] != b"\x89PNG\r\n\x1a\n" {
            return Err("Invalid PNG file".to_string());
        }
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        if content.len() < 3 || &content[0..3] != b"\xff\xd8\xff" {
            return Err("Invalid JPEG file".to_string());
        }
    } else if lower.ends_with(".gif") {
        if content.len() < 6 || (&content[0..6] != b"GIF87a" && &content[0..6] != b"GIF89a") {
            return Err("Invalid GIF file".to_string());
        }
    } else if lower.ends_with(".svg") {
        // SVG is XML, just check it starts with <svg
        let content_str = String::from_utf8_lossy(content);
        if !content_str.trim_start().starts_with("<svg") {
            return Err("Invalid SVG file".to_string());
        }
    } else if lower.ends_with(".webp") {
        if content.len() < 12 || &content[0..4] != b"RIFF" || &content[8..12] != b"WEBP" {
            return Err("Invalid WebP file".to_string());
        }
    } else if lower.ends_with(".pdf") {
        if content.len() < 5 || &content[0..5] != b"%PDF-" {
            return Err("Invalid PDF file".to_string());
        }
    }

    Ok(())
}
