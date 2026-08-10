pub mod navigator;
pub mod session;
pub mod semantics;
pub mod clipboard;
pub mod tabs;

pub use navigator::Navigator;
pub use session::Session;
pub use semantics::{ElementRole, SemanticDOM, SemanticElement};
pub use clipboard::ClipboardManager;
pub use tabs::{TabManager, IsolationMode};

use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

/// Browser is the core headless browser instance
pub struct Browser {
    navigator: Arc<Navigator>,
    sessions: Arc<dashmap::DashMap<String, Arc<Session>>>,
}

impl Browser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            navigator: Arc::new(Navigator::new()?),
            sessions: Arc::new(dashmap::DashMap::new()),
        })
    }

    pub fn create_session(&self, session_id: String) -> Result<Arc<Session>> {
        let session = Arc::new(Session::new(session_id.clone())?);
        self.sessions.insert(session_id, session.clone());
        Ok(session)
    }

    pub fn get_session(&self, session_id: &str) -> Option<Arc<Session>> {
        self.sessions.get(session_id).map(|s| s.value().clone())
    }

    pub fn close_session(&self, session_id: &str) -> Result<()> {
        self.sessions.remove(session_id);
        Ok(())
    }

    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.iter().map(|s| s.key().clone()).collect()
    }

    pub fn navigator(&self) -> Arc<Navigator> {
        self.navigator.clone()
    }

    /// Open a new tab under `tab_manager`, creating a fresh isolated session
    /// per tab (`IsolationMode::Isolated`) or reusing the manager's shared
    /// session (`IsolationMode::Shared`).
    pub fn open_tab(&self, tab_manager: &TabManager, url: String, mode: IsolationMode) -> Result<String> {
        let session_id = match mode {
            IsolationMode::Isolated => format!("session_{}", Uuid::new_v4()),
            IsolationMode::Shared => tab_manager.session_id().to_string(),
        };

        if self.get_session(&session_id).is_none() {
            self.create_session(session_id.clone())?;
        }

        tab_manager.create_tab_with_session(url, session_id)
    }

    /// Close a tab and, if it held an isolated session no other tab is using,
    /// close that session too so its cookies/storage don't leak.
    pub fn close_tab(&self, tab_manager: &TabManager, tab_id: &str) -> Result<()> {
        let session_id = tab_manager.get_tab(tab_id).map(|t| t.session_id);
        tab_manager.close_tab(tab_id)?;

        if let Some(session_id) = session_id {
            let still_referenced = tab_manager
                .list_tabs()
                .iter()
                .any(|t| t.session_id == session_id);
            if !still_referenced {
                self.close_session(&session_id)?;
            }
        }

        Ok(())
    }
}

impl Default for Browser {
    fn default() -> Self {
        Self::new().expect("Failed to create browser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_tabs_do_not_share_cookies() {
        let browser = Browser::new().unwrap();
        let tab_manager = TabManager::new("default_session".to_string());

        let tab_a = browser
            .open_tab(&tab_manager, "https://a.example".to_string(), IsolationMode::Isolated)
            .unwrap();
        let tab_b = browser
            .open_tab(&tab_manager, "https://b.example".to_string(), IsolationMode::Isolated)
            .unwrap();

        let session_a = browser.get_session(&tab_manager.get_tab(&tab_a).unwrap().session_id).unwrap();
        let session_b = browser.get_session(&tab_manager.get_tab(&tab_b).unwrap().session_id).unwrap();

        session_a.set_cookie("id".to_string(), "a".to_string());
        session_b.set_cookie("id".to_string(), "b".to_string());

        assert_eq!(session_a.get_cookie("id"), Some("a".to_string()));
        assert_eq!(session_b.get_cookie("id"), Some("b".to_string()));
        assert_ne!(session_a.id(), session_b.id());
    }

    #[test]
    fn shared_tabs_see_the_same_cookies() {
        let browser = Browser::new().unwrap();
        let tab_manager = TabManager::new("default_session".to_string());

        let tab_a = browser
            .open_tab(&tab_manager, "https://a.example".to_string(), IsolationMode::Shared)
            .unwrap();
        let tab_b = browser
            .open_tab(&tab_manager, "https://b.example".to_string(), IsolationMode::Shared)
            .unwrap();

        let session_a = browser.get_session(&tab_manager.get_tab(&tab_a).unwrap().session_id).unwrap();
        session_a.set_cookie("id".to_string(), "shared".to_string());

        let session_b = browser.get_session(&tab_manager.get_tab(&tab_b).unwrap().session_id).unwrap();
        assert_eq!(session_b.get_cookie("id"), Some("shared".to_string()));
        assert_eq!(session_a.id(), session_b.id());
    }

    #[test]
    fn closing_last_tab_of_an_isolated_session_closes_the_session() {
        let browser = Browser::new().unwrap();
        let tab_manager = TabManager::new("default_session".to_string());

        let tab_a = browser
            .open_tab(&tab_manager, "https://a.example".to_string(), IsolationMode::Isolated)
            .unwrap();
        let session_id = tab_manager.get_tab(&tab_a).unwrap().session_id;

        browser.close_tab(&tab_manager, &tab_a).unwrap();

        assert!(browser.get_session(&session_id).is_none());
    }
}
