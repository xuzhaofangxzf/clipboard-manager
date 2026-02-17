use anyhow::Result;
use clipboard_rs::{Clipboard, ClipboardContext, common::RustImage};
use image::{DynamicImage, ImageFormat, imageops::FilterType};
use std::io::Cursor;

use crate::db::{ClipboardData, ContentType};

/// Extract clipboard data and determine content type
pub fn extract_clipboard_data() -> Result<Option<(ContentType, ClipboardData)>> {
    let ctx = ClipboardContext::new().map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;

    // Try image first
    if ctx.has(clipboard_rs::ContentFormat::Image) {
        if let Ok(img_data) = ctx.get_image() {
            return Ok(Some(extract_image_data(img_data)?));
        }
    }

    // Try HTML
    if ctx.has(clipboard_rs::ContentFormat::Html) {
        if let Ok(html) = ctx.get_html() {
            if !html.is_empty() {
                return Ok(Some((
                    ContentType::Html,
                    ClipboardData::Html {
                        plain: strip_html(&html),
                        html,
                    },
                )));
            }
        }
    }

    // Try rich text (RTF)
    if ctx.has(clipboard_rs::ContentFormat::Rtf) {
        if let Ok(rtf) = ctx.get_rich_text() {
            if !rtf.is_empty() {
                // Extract plain text from RTF if possible
                let plain = ctx.get_text().unwrap_or_else(|_| strip_rtf(&rtf));
                return Ok(Some((
                    ContentType::RichText,
                    ClipboardData::RichText { plain, rtf },
                )));
            }
        }
    }

    // Try plain text
    if ctx.has(clipboard_rs::ContentFormat::Text) {
        if let Ok(text) = ctx.get_text() {
            if !text.is_empty() {
                return Ok(Some((ContentType::Text, ClipboardData::Text(text))));
            }
        }
    }

    Ok(None)
}

/// Extract and process image data
fn extract_image_data(
    img_data: clipboard_rs::common::RustImageData,
) -> Result<(ContentType, ClipboardData)> {
    // Convert to DynamicImage
    let dynamic_img = DynamicImage::ImageRgba8(
        img_data
            .to_rgba8()
            .map_err(|_| anyhow::anyhow!("Failed to create image from clipboard data"))?,
    );

    // Create full-size PNG
    let mut full_data = Vec::new();
    dynamic_img.write_to(&mut Cursor::new(&mut full_data), ImageFormat::Png)?;

    // Create thumbnail (max 200x200)
    let thumbnail = dynamic_img.resize(200, 200, FilterType::Lanczos3);
    let mut thumb_data = Vec::new();
    thumbnail.write_to(&mut Cursor::new(&mut thumb_data), ImageFormat::Png)?;

    Ok((
        ContentType::Image,
        ClipboardData::Image {
            data: full_data,
            thumbnail: thumb_data,
        },
    ))
}

/// Strip HTML tags for plain text preview
fn strip_html(html: &str) -> String {
    // Simple HTML stripping - could be improved with a proper HTML parser
    let mut result = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result.trim().to_string()
}

/// Simple RTF to plain text conversion
fn strip_rtf(rtf: &str) -> String {
    // Very basic RTF stripping - just remove control words
    let mut result = String::new();
    let mut in_control = false;

    for c in rtf.chars() {
        match c {
            '\\' => in_control = true,
            ' ' | '\n' | '\r' if in_control => {
                in_control = false;
            }
            '{' | '}' => {}
            _ if !in_control => result.push(c),
            _ => {}
        }
    }

    result.trim().to_string()
}

/// Check if two clipboard data are the same (for deduplication)
pub fn is_same_content(a: &ClipboardData, b: &ClipboardData) -> bool {
    match (a, b) {
        (ClipboardData::Text(t1), ClipboardData::Text(t2)) => t1 == t2,
        (
            ClipboardData::RichText { plain: p1, rtf: r1 },
            ClipboardData::RichText { plain: p2, rtf: r2 },
        ) => p1 == p2 && r1 == r2,
        (
            ClipboardData::Html {
                plain: p1,
                html: h1,
            },
            ClipboardData::Html {
                plain: p2,
                html: h2,
            },
        ) => p1 == p2 && h1 == h2,
        (ClipboardData::Image { data: d1, .. }, ClipboardData::Image { data: d2, .. }) => d1 == d2,
        _ => false,
    }
}
