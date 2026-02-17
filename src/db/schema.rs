use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of clipboard content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    Text,
    RichText,
    Html,
    Image,
}

/// Clipboard entry stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub content_type: ContentType,
    pub data: ClipboardData,
    pub preview: String, // Short preview for list display
}

/// Serializable clipboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardData {
    Text(String),
    RichText { plain: String, rtf: String },
    Html { plain: String, html: String },
    Image { data: Vec<u8>, thumbnail: Vec<u8> },
}

impl ClipboardEntry {
    pub fn new(content_type: ContentType, data: ClipboardData) -> Self {
        let preview = match &data {
            ClipboardData::Text(text) => truncate_text(text, 100),
            ClipboardData::RichText { plain, .. } => truncate_text(plain, 100),
            ClipboardData::Html { plain, .. } => truncate_text(plain, 100),
            ClipboardData::Image { .. } => "[Image]".to_string(),
        };

        Self {
            id: 0, // Will be set by database
            timestamp: Utc::now(),
            content_type,
            data,
            preview,
        }
    }

    /// Serialize entry to bytes for storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Deserialize entry from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let text = text.trim();
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}
