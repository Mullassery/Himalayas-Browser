use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use crate::browser::navigator::CookieJar;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    pub current_url: String,
    pub history: Vec<String>,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
}

pub struct Session {
    state: parking_lot::RwLock<SessionState>,
    cookies: parking_lot::RwLock<CookieJar>,
    storage: parking_lot::RwLock<HashMap<String, String>>,
}

impl Session {
    pub fn new(id: String) -> Result<Self> {
        Ok(Self {
            state: parking_lot::RwLock::new(SessionState {
                id: id.clone(),
                current_url: String::new(),
                history: Vec::new(),
                created_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
            }),
            cookies: parking_lot::RwLock::new(CookieJar::new()),
            storage: parking_lot::RwLock::new(HashMap::new()),
        })
    }

    pub fn id(&self) -> String {
        self.state.read().id.clone()
    }

    pub fn current_url(&self) -> String {
        self.state.read().current_url.clone()
    }

    pub fn set_current_url(&self, url: String) {
        let mut state = self.state.write();
        if !url.is_empty() && state.current_url != url {
            state.history.push(url.clone());
        }
        state.current_url = url;
        state.last_accessed = SystemTime::now();
    }

    pub fn history(&self) -> Vec<String> {
        self.state.read().history.clone()
    }

    pub fn go_back(&self) -> Option<String> {
        let mut state = self.state.write();
        if state.history.len() > 1 {
            state.history.pop()
        } else {
            None
        }
    }

    pub fn go_forward(&self, url: String) -> Result<()> {
        let mut state = self.state.write();
        state.history.push(url);
        state.last_accessed = SystemTime::now();
        Ok(())
    }

    pub fn set_cookie(&self, name: String, value: String) {
        self.cookies.write().set(name, value);
    }

    pub fn get_cookie(&self, name: &str) -> Option<String> {
        self.cookies.read().get(name)
    }

    pub fn get_cookies(&self) -> HashMap<String, String> {
        self.cookies.read().get_all()
    }

    pub fn clear_cookies(&self) {
        self.cookies.write().clear();
    }

    pub fn set_storage(&self, key: String, value: String) {
        self.storage.write().insert(key, value);
    }

    pub fn get_storage(&self, key: &str) -> Option<String> {
        self.storage.read().get(key).cloned()
    }

    pub fn get_all_storage(&self) -> HashMap<String, String> {
        self.storage.read().clone()
    }

    pub fn delete_storage(&self, key: &str) {
        self.storage.write().remove(key);
    }

    pub fn clear_storage(&self) {
        self.storage.write().clear();
    }

    pub fn state_snapshot(&self) -> SessionState {
        self.state.read().clone()
    }

    pub fn uptime_secs(&self) -> u64 {
        let state = self.state.read();
        state
            .created_at
            .elapsed()
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-session".to_string()).unwrap();
        assert_eq!(session.id(), "test-session");
    }

    #[test]
    fn test_url_history() {
        let session = Session::new("test".to_string()).unwrap();
        session.set_current_url("https://example.com".to_string());
        session.set_current_url("https://example.com/page2".to_string());
        let history = session.history();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_cookies() {
        let session = Session::new("test".to_string()).unwrap();
        session.set_cookie("session_id".to_string(), "abc123".to_string());
        assert_eq!(session.get_cookie("session_id"), Some("abc123".to_string()));
    }

    #[test]
    fn test_storage() {
        let session = Session::new("test".to_string()).unwrap();
        session.set_storage("key1".to_string(), "value1".to_string());
        assert_eq!(session.get_storage("key1"), Some("value1".to_string()));
        session.delete_storage("key1");
        assert_eq!(session.get_storage("key1"), None);
    }

    #[test]
    fn test_go_back() {
        let session = Session::new("test".to_string()).unwrap();
        session.set_current_url("https://example.com".to_string());
        session.set_current_url("https://example.com/page2".to_string());
        let prev = session.go_back();
        assert!(prev.is_some());
    }
}
