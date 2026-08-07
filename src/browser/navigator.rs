use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

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

    pub fn from_cookies(cookies: HashMap<String, String>) -> Self {
        Self { cookies }
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

    fn to_cookie_header(&self) -> Option<String> {
        if self.cookies.is_empty() {
            return None;
        }
        Some(
            self.cookies
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

const MAX_REDIRECTS: usize = 10;

pub struct Navigator {
    user_agent: String,
    client: reqwest::Client,
}

impl Navigator {
    pub fn new() -> Result<Self> {
        let user_agent = "Himalayas/0.1.0 (Agent-Native Browser)".to_string();
        let client = reqwest::Client::builder()
            .user_agent(user_agent.clone())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            // Redirects are followed manually below so we can record the
            // chain and re-apply cookies at each hop.
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self { user_agent, client })
    }

    pub async fn navigate(&self, url: &str, cookies: &CookieJar) -> Result<PageContent> {
        debug!("Navigating to: {}", url);

        let _parsed_url = url::Url::parse(url)?;

        let mut current_url = url.to_string();
        let mut redirect_chain = vec![current_url.clone()];
        let mut jar = cookies.clone();

        loop {
            let (status, html, headers) = self.fetch_page(&current_url, &jar).await?;

            for (name, value) in Self::parse_set_cookie_headers(&headers) {
                jar.set(name, value);
            }

            if (300..400).contains(&status) {
                if let Some(location) = headers.get("location") {
                    if redirect_chain.len() > MAX_REDIRECTS {
                        anyhow::bail!("Too many redirects starting from {}", url);
                    }
                    let next_url = self.follow_redirect(location, &current_url)?;
                    debug!("Redirecting {} -> {}", current_url, next_url);
                    redirect_chain.push(next_url.clone());
                    current_url = next_url;
                    continue;
                }
                warn!("Status {} with no Location header at {}", status, current_url);
            }

            info!("Page loaded: {} (status: {}, bytes: {})", current_url, status, html.len());

            return Ok(PageContent {
                url: current_url,
                status_code: status,
                html,
                headers,
                redirect_chain,
            });
        }
    }

    /// Fetches a single URL (no redirect-following). Returns the raw status
    /// code, response body, and lowercased response headers.
    async fn fetch_page(
        &self,
        url: &str,
        cookies: &CookieJar,
    ) -> Result<(u16, String, HashMap<String, String>)> {
        let mut request = self.client.get(url);
        if let Some(cookie_header) = cookies.to_cookie_header() {
            request = request.header(reqwest::header::COOKIE, cookie_header);
        }

        let response = request.send().await?;
        let status = response.status().as_u16();

        let mut headers = HashMap::new();
        for (name, value) in response.headers().iter() {
            if let Ok(value_str) = value.to_str() {
                headers
                    .entry(name.as_str().to_lowercase())
                    .and_modify(|existing: &mut String| {
                        existing.push_str(", ");
                        existing.push_str(value_str);
                    })
                    .or_insert_with(|| value_str.to_string());
            }
        }

        let html = response.text().await?;
        Ok((status, html, headers))
    }

    /// Extracts individual `name=value` pairs out of a (possibly merged)
    /// `set-cookie` header value. Multiple Set-Cookie headers on the same
    /// response get folded together upstream with ", " as a separator by
    /// `fetch_page`; since cookie `Expires` attributes also contain commas,
    /// we don't try to split on that. We only need the name/value pair for
    /// this jar (not full attribute parsing), so we take everything before
    /// the first `;` of the raw header value.
    fn parse_set_cookie_headers(headers: &HashMap<String, String>) -> Vec<(String, String)> {
        let Some(raw) = headers.get("set-cookie") else {
            return Vec::new();
        };
        raw.split(';')
            .next()
            .and_then(|pair| pair.split_once('='))
            .map(|(name, value)| vec![(name.trim().to_string(), value.trim().to_string())])
            .unwrap_or_default()
    }

    pub fn get_user_agent(&self) -> &str {
        &self.user_agent
    }

    pub fn follow_redirect(&self, location: &str, current_url: &str) -> Result<String> {
        debug!("Following redirect from {} to {}", current_url, location);

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
        for (name, value) in Self::parse_set_cookie_headers(headers) {
            jar.set(name, value);
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
    async fn test_navigate_fetches_real_response_body() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body("<html><head><title>Mock Page</title></head><body>hi</body></html>")
            .create_async()
            .await;

        let nav = Navigator::new().unwrap();
        let jar = CookieJar::new();
        let page = nav.navigate(&server.url(), &jar).await.unwrap();

        mock.assert_async().await;
        assert_eq!(page.status_code, 200);
        assert!(page.html.contains("Mock Page"));
        assert_eq!(page.redirect_chain.len(), 1);
    }

    #[tokio::test]
    async fn test_navigate_follows_redirects_and_records_chain() {
        let mut server = mockito::Server::new_async().await;
        let target = format!("{}/final", server.url());

        let redirect_mock = server
            .mock("GET", "/start")
            .with_status(302)
            .with_header("location", &target)
            .create_async()
            .await;
        let final_mock = server
            .mock("GET", "/final")
            .with_status(200)
            .with_body("<html><body>done</body></html>")
            .create_async()
            .await;

        let nav = Navigator::new().unwrap();
        let jar = CookieJar::new();
        let start_url = format!("{}/start", server.url());
        let page = nav.navigate(&start_url, &jar).await.unwrap();

        redirect_mock.assert_async().await;
        final_mock.assert_async().await;
        assert_eq!(page.status_code, 200);
        assert_eq!(page.url, target);
        assert_eq!(page.redirect_chain, vec![start_url, target]);
    }

    #[tokio::test]
    async fn test_navigate_sends_and_captures_cookies() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .match_header("cookie", "session=abc123")
            .with_status(200)
            .with_header("set-cookie", "new_session=xyz789; Path=/; HttpOnly")
            .with_body("<html><body>ok</body></html>")
            .create_async()
            .await;

        let nav = Navigator::new().unwrap();
        let mut jar = CookieJar::new();
        jar.set("session".to_string(), "abc123".to_string());

        let page = nav.navigate(&server.url(), &jar).await.unwrap();

        mock.assert_async().await;
        let received_cookies = nav.extract_cookies_from_headers(&page.headers);
        assert_eq!(received_cookies.get("new_session"), Some("xyz789".to_string()));
    }

    #[test]
    fn test_redirect_handling() {
        let nav = Navigator::new().unwrap();
        let result = nav.follow_redirect("/path", "https://example.com/old").unwrap();
        assert!(result.contains("https://example.com/path"));
    }

    /// Live network smoke test — not run by default (`cargo test` skips
    /// `#[ignore]`d tests). Run explicitly with:
    ///   cargo test --package himalayas test_navigate_live_network -- --ignored
    #[tokio::test]
    #[ignore]
    async fn test_navigate_live_network() {
        let nav = Navigator::new().unwrap();
        let jar = CookieJar::new();
        let page = nav.navigate("https://example.com", &jar).await.unwrap();
        assert_eq!(page.status_code, 200);
        assert!(page.html.to_lowercase().contains("example"));
    }
}
