use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::Result;
use tracing::{debug, info};

/// Rich clipboard data supporting multiple formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClipboardData {
    Text(String),
    Html(String),
    Image(Vec<u8>),
    Files(Vec<String>),
    Table(TableData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Clipboard manager
pub struct ClipboardManager {
    data: Arc<RwLock<Option<ClipboardData>>>,
    history: Arc<RwLock<Vec<ClipboardData>>>,
    max_history: usize,
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(None)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 20,
        }
    }

    pub fn copy(&self, data: ClipboardData) -> Result<()> {
        debug!("Clipboard: copy {:?}", std::mem::discriminant(&data));

        let mut history = self.history.write();
        let mut clipboard = self.data.write();

        // Add to history
        if history.len() >= self.max_history {
            history.remove(0);
        }
        history.push(data.clone());

        // Set current
        *clipboard = Some(data);

        info!("Clipboard: data copied");
        Ok(())
    }

    pub fn paste(&self) -> Result<Option<ClipboardData>> {
        let clipboard = self.data.read();
        Ok(clipboard.clone())
    }

    pub fn paste_text(&self) -> Result<Option<String>> {
        let clipboard = self.data.read();
        match clipboard.as_ref() {
            Some(ClipboardData::Text(text)) => Ok(Some(text.clone())),
            _ => Ok(None),
        }
    }

    pub fn clear(&self) -> Result<()> {
        debug!("Clipboard: clear");
        let mut clipboard = self.data.write();
        *clipboard = None;
        Ok(())
    }

    pub fn get_history(&self) -> Vec<ClipboardData> {
        self.history.read().clone()
    }

    pub fn clear_history(&self) {
        self.history.write().clear();
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_copy_paste() {
        let clipboard = ClipboardManager::new();
        let data = ClipboardData::Text("Hello, World!".to_string());

        clipboard.copy(data.clone()).unwrap();
        let pasted = clipboard.paste().unwrap();

        assert!(pasted.is_some());
        assert_eq!(pasted.unwrap(), data);
    }

    #[test]
    fn test_clipboard_text_paste() {
        let clipboard = ClipboardManager::new();
        clipboard.copy(ClipboardData::Text("Test".to_string())).unwrap();

        let text = clipboard.paste_text().unwrap();
        assert_eq!(text, Some("Test".to_string()));
    }

    #[test]
    fn test_clipboard_history() {
        let clipboard = ClipboardManager::new();

        clipboard.copy(ClipboardData::Text("First".to_string())).unwrap();
        clipboard.copy(ClipboardData::Text("Second".to_string())).unwrap();

        let history = clipboard.get_history();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_clipboard_clear() {
        let clipboard = ClipboardManager::new();
        clipboard.copy(ClipboardData::Text("Data".to_string())).unwrap();
        clipboard.clear().unwrap();

        let pasted = clipboard.paste().unwrap();
        assert!(pasted.is_none());
    }
}
