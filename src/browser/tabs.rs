use std::sync::Arc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use anyhow::{Result, bail};
use tracing::{debug, info};
use uuid::Uuid;
use chrono::DateTime;

/// Represents a browser tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub url: String,
    pub created: DateTime<chrono::Local>,
    pub last_accessed: DateTime<chrono::Local>,
    pub is_active: bool,
}

impl Tab {
    pub fn new(session_id: String, url: String) -> Self {
        Self {
            id: format!("tab_{}", Uuid::new_v4()),
            session_id,
            title: url.clone(),
            url,
            created: chrono::Local::now(),
            last_accessed: chrono::Local::now(),
            is_active: false,
        }
    }

    pub fn update_title(&mut self, title: String) {
        self.title = title;
        self.last_accessed = chrono::Local::now();
    }

    pub fn update_url(&mut self, url: String) {
        self.url = url;
        self.last_accessed = chrono::Local::now();
    }
}

/// Controls whether each tab gets its own isolated session (cookies, storage,
/// history) or shares one session with every other tab in the manager.
///
/// Isolated sessions cost one `Session` (cookie jar + storage map) per tab, so
/// they're only worth the memory on capable hardware — see
/// `DeviceTier` in `crate::intelligence::device_detection` for the default
/// policy per device tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationMode {
    Isolated,
    Shared,
}

/// Tab manager for multi-tab sessions
pub struct TabManager {
    tabs: Arc<DashMap<String, Tab>>,
    active_tab: Arc<parking_lot::RwLock<Option<String>>>,
    session_id: String,
}

impl TabManager {
    pub fn new(session_id: String) -> Self {
        Self {
            tabs: Arc::new(DashMap::new()),
            active_tab: Arc::new(parking_lot::RwLock::new(None)),
            session_id,
        }
    }

    pub fn create_tab(&self, url: String) -> Result<String> {
        self.create_tab_with_session(url, self.session_id.clone())
    }

    /// Create a tab bound to a specific session (used for isolated per-tab sessions).
    pub fn create_tab_with_session(&self, url: String, session_id: String) -> Result<String> {
        debug!("Creating tab with URL: {} (session: {})", url, session_id);

        let tab = Tab::new(session_id, url);
        let tab_id = tab.id.clone();

        self.tabs.insert(tab_id.clone(), tab);

        info!("Tab created: {}", tab_id);
        Ok(tab_id)
    }

    /// The default (shared) session id this manager was created with.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn close_tab(&self, tab_id: &str) -> Result<()> {
        debug!("Closing tab: {}", tab_id);

        if self.tabs.remove(tab_id).is_none() {
            bail!("Tab not found: {}", tab_id);
        }

        // Clear active if this was active
        let mut active = self.active_tab.write();
        if active.as_ref() == Some(&tab_id.to_string()) {
            *active = None;
        }

        info!("Tab closed: {}", tab_id);
        Ok(())
    }

    pub fn set_active_tab(&self, tab_id: &str) -> Result<()> {
        if !self.tabs.contains_key(tab_id) {
            bail!("Tab not found: {}", tab_id);
        }

        let mut active = self.active_tab.write();
        *active = Some(tab_id.to_string());

        debug!("Active tab set: {}", tab_id);
        Ok(())
    }

    pub fn get_active_tab(&self) -> Option<Tab> {
        let active = self.active_tab.read();
        active.as_ref().and_then(|id| {
            self.tabs.get(id).map(|entry| entry.clone())
        })
    }

    pub fn get_tab(&self, tab_id: &str) -> Option<Tab> {
        self.tabs.get(tab_id).map(|entry| entry.clone())
    }

    pub fn list_tabs(&self) -> Vec<Tab> {
        self.tabs
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn count_tabs(&self) -> usize {
        self.tabs.len()
    }

    pub fn update_tab_title(&self, tab_id: &str, title: String) -> Result<()> {
        if let Some(mut tab) = self.tabs.get_mut(tab_id) {
            tab.update_title(title);
            Ok(())
        } else {
            bail!("Tab not found: {}", tab_id)
        }
    }

    pub fn update_tab_url(&self, tab_id: &str, url: String) -> Result<()> {
        if let Some(mut tab) = self.tabs.get_mut(tab_id) {
            tab.update_url(url);
            Ok(())
        } else {
            bail!("Tab not found: {}", tab_id)
        }
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new("default_session".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tab() {
        let tm = TabManager::new("session_1".to_string());
        let tab_id = tm.create_tab("https://example.com".to_string()).unwrap();

        assert!(!tab_id.is_empty());
        assert_eq!(tm.count_tabs(), 1);
    }

    #[test]
    fn test_close_tab() {
        let tm = TabManager::new("session_1".to_string());
        let tab_id = tm.create_tab("https://example.com".to_string()).unwrap();

        tm.close_tab(&tab_id).unwrap();
        assert_eq!(tm.count_tabs(), 0);
    }

    #[test]
    fn test_set_active_tab() {
        let tm = TabManager::new("session_1".to_string());
        let tab_id = tm.create_tab("https://example.com".to_string()).unwrap();

        tm.set_active_tab(&tab_id).unwrap();
        let active = tm.get_active_tab();

        assert!(active.is_some());
        assert_eq!(active.unwrap().id, tab_id);
    }

    #[test]
    fn test_list_tabs() {
        let tm = TabManager::new("session_1".to_string());

        tm.create_tab("https://example.com".to_string()).unwrap();
        tm.create_tab("https://google.com".to_string()).unwrap();
        tm.create_tab("https://github.com".to_string()).unwrap();

        let tabs = tm.list_tabs();
        assert_eq!(tabs.len(), 3);
    }

    #[test]
    fn test_update_tab_title() {
        let tm = TabManager::new("session_1".to_string());
        let tab_id = tm.create_tab("https://example.com".to_string()).unwrap();

        tm.update_tab_title(&tab_id, "Example".to_string()).unwrap();
        let tab = tm.get_tab(&tab_id).unwrap();

        assert_eq!(tab.title, "Example");
    }
}
