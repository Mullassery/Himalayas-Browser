use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;
use chrono::{DateTime, Local};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: String,
    pub author: Option<String>,
    pub created: DateTime<Local>,
    pub modified: DateTime<Local>,
    pub file_size: u64,
}

impl DocumentMetadata {
    pub fn from_file(path: &Path) -> Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?;
        let now = Local::now();

        Ok(Self {
            title: path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            author: None,
            created: now,
            modified: now,
            file_size: metadata.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_creation() {
        let metadata = DocumentMetadata {
            title: "Test".to_string(),
            author: None,
            created: Local::now(),
            modified: Local::now(),
            file_size: 1024,
        };
        assert_eq!(metadata.title, "Test");
    }
}
