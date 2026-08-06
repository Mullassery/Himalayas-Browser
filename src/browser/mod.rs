pub mod navigator;
pub mod session;
pub mod semantics;

pub use navigator::Navigator;
pub use session::Session;
pub use semantics::SemanticDOM;

use anyhow::Result;
use std::sync::Arc;

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
}

impl Default for Browser {
    fn default() -> Self {
        Self::new().expect("Failed to create browser")
    }
}
