use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct PageContent {
    pub url: String,
    pub status_code: u16,
    pub html: String,
    pub headers: HashMap<String, String>,
    pub redirect_chain: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CookieJar {
    cookies: HashMap<String, String>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            cookies: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: String, value: String) {
        self.cookies.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.cookies.get(name).cloned()
    }

    pub fn get_all(&self) -> HashMap<String, String> {
        self.cookies.clone()
    }

    pub fn clear(&mut self) {
        self.cookies.clear();
    }

    pub fn delete(&mut self, name: &str) {
        self.cookies.remove(name);
    }
}

pub struct Navigator {
    user_agent: String,
}

impl Navigator {
    pub fn new() -> Result<Self> {
        Ok(Self {
            user_agent: "Himalayas/0.1.0 (Agent-Native Browser)".to_string(),
        })
    }

    pub async fn navigate(&self, url: &str, cookies: &CookieJar) -> Result<PageContent> {
        debug!("Navigating to: {}", url);

        // Validate URL
        let _parsed_url = url::Url::parse(url)?;

        // Simulate page load with HTTP client
        // (real implementation will use reqwest)
        let html = self.fetch_page(url, cookies).await?;

        info!("Page loaded: {} (bytes: {})", url, html.len());

        Ok(PageContent {
            url: url.to_string(),
            status_code: 200,
            html,
            headers: Default::default(),
            redirect_chain: vec![url.to_string()],
        })
    }

    pub async fn fetch_page(&self, url: &str, _cookies: &CookieJar) -> Result<String> {
        // TODO: Implement real HTTP client with reqwest
        // For now, return minimal HTML for testing
        Ok(format!(
            r#"<!DOCTYPE html>
<html>
<head><title>Test Page</title></head>
<body>
  <h1>Welcome to {}</h1>
  <p>This is a test page.</p>
  <button id="submit">Submit</button>
</body>
</html>"#,
            url
        ))
    }

    pub fn get_user_agent(&self) -> &str {
        &self.user_agent
    }

    pub fn follow_redirect(&self, location: &str, current_url: &str) -> Result<String> {
        debug!("Following redirect from {} to {}", current_url, location);

        // Handle relative redirects
        if location.starts_with('/') {
            let base = url::Url::parse(current_url)?;
            let url = base.join(location)?;
            Ok(url.to_string())
        } else if location.starts_with("http://") || location.starts_with("https://") {
            Ok(location.to_string())
        } else {
            let base = url::Url::parse(current_url)?;
            let url = base.join(location)?;
            Ok(url.to_string())
        }
    }

    pub fn extract_cookies_from_headers(&self, headers: &HashMap<String, String>) -> CookieJar {
        let mut jar = CookieJar::new();

        if let Some(set_cookie) = headers.get("set-cookie") {
            // Parse Set-Cookie header (simplified)
            if let Some(name_value) = set_cookie.split(';').next() {
                if let Some((name, value)) = name_value.split_once('=') {
                    jar.set(name.trim().to_string(), value.trim().to_string());
                }
            }
        }

        jar
    }
}

impl Default for Navigator {
    fn default() -> Self {
        Self::new().expect("Failed to create navigator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_jar() {
        let mut jar = CookieJar::new();
        jar.set("session".to_string(), "abc123".to_string());
        assert_eq!(jar.get("session"), Some("abc123".to_string()));
        jar.delete("session");
        assert_eq!(jar.get("session"), None);
    }

    #[test]
    fn test_navigator_creation() {
        let nav = Navigator::new().unwrap();
        assert!(nav.get_user_agent().contains("Himalayas"));
    }

    #[tokio::test]
    async fn test_navigate() {
        let nav = Navigator::new().unwrap();
        let jar = CookieJar::new();
        let result = nav.navigate("https://example.com", &jar).await;
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.status_code, 200);
        assert!(page.html.contains("example.com"));
    }

    #[test]
    fn test_redirect_handling() {
        let nav = Navigator::new().unwrap();
        let result = nav.follow_redirect("/path", "https://example.com/old").unwrap();
        assert!(result.contains("https://example.com/path"));
    }
}
