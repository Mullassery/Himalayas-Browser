pub mod agent;
pub mod interaction;
pub mod navigation;
pub mod query;

pub use agent::AgentAPI;
pub use interaction::InteractionAPI;
pub use navigation::NavigationAPI;
pub use query::QueryAPI;

use anyhow::Result;
use std::sync::Arc;
use crate::browser::SemanticElement;

/// AgentContext holds the session and provides APIs
pub struct AgentContext {
    session: Arc<crate::browser::Session>,
    browser: Arc<crate::browser::Browser>,
}

impl AgentContext {
    pub fn new(session: Arc<crate::browser::Session>, browser: Arc<crate::browser::Browser>) -> Self {
        Self { session, browser }
    }

    pub fn session(&self) -> Arc<crate::browser::Session> {
        self.session.clone()
    }

    pub fn browser(&self) -> Arc<crate::browser::Browser> {
        self.browser.clone()
    }

    pub async fn navigate(&self, url: &str) -> Result<crate::browser::SemanticDOM> {
        let cookies = self.session.get_cookies();
        let jar = crate::browser::navigator::CookieJar::from_cookies(cookies);

        let page = self.browser.navigator().navigate(url, &jar).await?;
        self.session.set_current_url(page.url.clone());

        let response_cookies = self.browser.navigator().extract_cookies_from_headers(&page.headers);
        for (name, value) in response_cookies.get_all() {
            self.session.set_cookie(name, value);
        }

        let dom = crate::browser::SemanticDOM::from_html(page.url, &page.html)?;
        Ok(dom)
    }

    pub async fn query(&self, selector: &str) -> Result<Vec<SemanticElement>> {
        // This would query the current DOM
        // For now, return empty
        Ok(Vec::new())
    }

    pub async fn click(&self, element_id: &str) -> Result<()> {
        // Record the click action (will use for audit trail)
        tracing::info!("Agent clicked element: {}", element_id);
        Ok(())
    }

    pub async fn input(&self, element_id: &str, value: &str) -> Result<()> {
        // Record input action
        tracing::info!("Agent input to {}: {}", element_id, value);
        self.session.set_storage(format!("input_{}", element_id), value.to_string());
        Ok(())
    }

    pub async fn get_text(&self, element_id: &str) -> Result<String> {
        // Get text from element
        Ok(format!("Text from {}", element_id))
    }

    pub async fn submit_form(&self, form_id: &str) -> Result<()> {
        // Submit form
        tracing::info!("Agent submitted form: {}", form_id);
        Ok(())
    }

    pub fn go_back(&self) -> Result<()> {
        self.session.go_back();
        Ok(())
    }

    pub fn go_forward(&self, url: String) -> Result<()> {
        self.session.go_forward(url)?;
        Ok(())
    }

    pub fn get_current_url(&self) -> String {
        self.session.current_url()
    }

    pub fn get_history(&self) -> Vec<String> {
        self.session.history()
    }
}
