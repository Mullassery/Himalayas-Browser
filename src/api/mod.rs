pub mod agent;
pub mod interaction;
pub mod navigation;
pub mod query;

pub use agent::AgentAPI;
pub use interaction::InteractionAPI;
pub use navigation::NavigationAPI;
pub use query::QueryAPI;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::browser::navigator::CookieJar;
use crate::browser::{ElementRole, SemanticDOM, SemanticElement};

/// AgentContext holds the session and provides APIs
pub struct AgentContext {
    session: Arc<crate::browser::Session>,
    browser: Arc<crate::browser::Browser>,
    /// The raw HTML and parsed `SemanticDOM` of the last successful
    /// `navigate()`/`submit_form()`/link-`click()` — what `query`, `click`,
    /// `get_text`, and `submit_form` all operate against. `None` until the
    /// first navigation. Raw HTML is kept alongside the parsed `SemanticDOM`
    /// because `query`'s CSS-selector matching (`SemanticDOM::query_selector_all`)
    /// re-parses fresh rather than filtering the fixed button/link/form
    /// extraction `SemanticDOM::from_html` already did.
    current_page: RwLock<Option<(String, SemanticDOM)>>,
}

impl AgentContext {
    pub fn new(session: Arc<crate::browser::Session>, browser: Arc<crate::browser::Browser>) -> Self {
        Self { session, browser, current_page: RwLock::new(None) }
    }

    pub fn session(&self) -> Arc<crate::browser::Session> {
        self.session.clone()
    }

    pub fn browser(&self) -> Arc<crate::browser::Browser> {
        self.browser.clone()
    }

    /// Absorb a freshly-fetched page: capture its cookies into the session,
    /// parse it, and store it as the current page every other method
    /// operates against. Shared by `navigate` and `submit_form` — both end
    /// with "we have a `PageContent`, now make it the live page."
    fn absorb_page(&self, page: crate::browser::navigator::PageContent) -> Result<SemanticDOM> {
        self.session.set_current_url(page.url.clone());

        let response_cookies = self.browser.navigator().extract_cookies_from_headers(&page.headers);
        for (name, value) in response_cookies.get_all() {
            self.session.set_cookie(name, value);
        }

        let dom = SemanticDOM::from_html(page.url.clone(), &page.html)?;
        *self.current_page.write() = Some((page.html, dom.clone()));
        Ok(dom)
    }

    pub async fn navigate(&self, url: &str) -> Result<SemanticDOM> {
        let cookies = self.session.get_cookies();
        let jar = CookieJar::from_cookies(cookies);
        let page = self.browser.navigator().navigate(url, &jar).await?;
        self.absorb_page(page)
    }

    /// Real CSS-selector query against the current page — see
    /// `SemanticDOM::query_selector_all` for what "real" covers (any valid
    /// CSS selector, loosely-classified roles) and doesn't (structural
    /// nuance the role-specific parsers in `SemanticDOM::from_html` capture).
    pub async fn query(&self, selector: &str) -> Result<Vec<SemanticElement>> {
        let guard = self.current_page.read();
        let Some((html, _)) = guard.as_ref() else {
            anyhow::bail!("no page loaded — call navigate() first");
        };
        SemanticDOM::query_selector_all(html, selector)
    }

    /// Click an element by id, resolved against the current page's
    /// `SemanticDOM`:
    /// - a link (`<a href>`) navigates to its resolved `href`, same as a
    ///   real click;
    /// - a button inside a `<form>` submits that form (see
    ///   `SemanticDOM::find_enclosing_form_id` for the "inside a form"
    ///   lookup and its id-attribute caveat);
    /// - anything else is reported as not clickable, rather than silently
    ///   succeeding — an agent relying on a click actually doing something
    ///   needs to know when it didn't.
    pub async fn click(&self, element_id: &str) -> Result<SemanticDOM> {
        tracing::info!("Agent clicked element: {}", element_id);

        let (role, href, current_url, html) = {
            let guard = self.current_page.read();
            let Some((html, dom)) = guard.as_ref() else {
                anyhow::bail!("no page loaded — call navigate() first");
            };
            let element = dom
                .find_by_id(element_id)
                .ok_or_else(|| anyhow::anyhow!("element not found: {element_id}"))?;
            (element.role.clone(), element.attributes.get("href").cloned(), dom.url.clone(), html.clone())
        };

        match role {
            ElementRole::Link => {
                let href = href.ok_or_else(|| anyhow::anyhow!("link {element_id} has no href"))?;
                let resolved = url::Url::parse(&current_url)?.join(&href)?.to_string();
                self.navigate(&resolved).await
            }
            ElementRole::Button => {
                match SemanticDOM::find_enclosing_form_id(&html, element_id) {
                    Some(form_id) => self.submit_form(&form_id).await,
                    None => anyhow::bail!(
                        "button {element_id} isn't inside a form with an id — nothing to do on click"
                    ),
                }
            }
            other => anyhow::bail!("element {element_id} (role {other:?}) is not clickable"),
        }
    }

    /// Record a value for a form field, keyed by the field's `name`
    /// attribute (what actually gets submitted — `SemanticForm::inputs` is a
    /// list of names, not element ids) when the element is found on the
    /// current page; falls back to `element_id` itself otherwise, so this
    /// still works if the caller passes a field's `name` directly.
    pub async fn input(&self, element_id: &str, value: &str) -> Result<()> {
        tracing::info!("Agent input to {}: {}", element_id, value);

        let name = {
            let guard = self.current_page.read();
            guard
                .as_ref()
                .and_then(|(_, dom)| dom.find_by_id(element_id))
                .and_then(|e| e.attributes.get("name").cloned())
                .unwrap_or_else(|| element_id.to_string())
        };

        self.session.set_storage(format!("form_field_{name}"), value.to_string());
        Ok(())
    }

    pub async fn get_text(&self, element_id: &str) -> Result<String> {
        let guard = self.current_page.read();
        let Some((_, dom)) = guard.as_ref() else {
            anyhow::bail!("no page loaded — call navigate() first");
        };
        dom.find_by_id(element_id)
            .map(|e| e.text.clone())
            .ok_or_else(|| anyhow::anyhow!("element not found: {element_id}"))
    }

    /// Submit a form by id: builds the field map from whatever `input()`
    /// calls have recorded on the session (unset fields submit empty, same
    /// as a real browser submitting untouched form fields), resolves the
    /// form's `action` against the current page URL, and either navigates
    /// with a query string (GET) or POSTs (`Navigator::submit_form`,
    /// anything not explicitly "GET").
    pub async fn submit_form(&self, form_id: &str) -> Result<SemanticDOM> {
        tracing::info!("Agent submitted form: {}", form_id);

        let (form, base_url) = {
            let guard = self.current_page.read();
            let Some((_, dom)) = guard.as_ref() else {
                anyhow::bail!("no page loaded — call navigate() first");
            };
            let form = dom
                .find_forms()
                .iter()
                .find(|f| f.id == form_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("form not found: {form_id}"))?;
            (form, dom.url.clone())
        };

        let mut fields = HashMap::new();
        for name in &form.inputs {
            let value = self.session.get_storage(&format!("form_field_{name}")).unwrap_or_default();
            fields.insert(name.clone(), value);
        }

        let action_url = if form.action.is_empty() {
            base_url.clone()
        } else {
            url::Url::parse(&base_url)?.join(&form.action)?.to_string()
        };

        let cookies = self.session.get_cookies();
        let jar = CookieJar::from_cookies(cookies);

        let page = if form.method.eq_ignore_ascii_case("GET") {
            let mut target = url::Url::parse(&action_url)?;
            target.query_pairs_mut().clear().extend_pairs(&fields);
            self.browser.navigator().navigate(target.as_str(), &jar).await?
        } else {
            self.browser.navigator().submit_form(&action_url, &fields, &jar).await?
        };

        self.absorb_page(page)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::Browser;

    fn context() -> AgentContext {
        let browser = Arc::new(Browser::new().unwrap());
        let session = browser.create_session("test-session".to_string()).unwrap();
        AgentContext::new(session, browser)
    }

    #[tokio::test]
    async fn test_query_before_navigate_errors() {
        let ctx = context();
        assert!(ctx.query(".anything").await.is_err());
    }

    #[tokio::test]
    async fn test_full_agent_flow_navigate_input_submit_click() {
        let mut server = mockito::Server::new_async().await;

        let page_mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("set-cookie", "visited=1; Path=/")
            .with_body(
                r#"<html><head><title>Login</title></head><body>
                    <form id="login" action="/login" method="post">
                        <input id="user" name="username" />
                        <input id="pass" name="password" />
                        <button id="submit-btn">Log In</button>
                    </form>
                    <a id="help-link" href="/help">Need help?</a>
                </body></html>"#,
            )
            .create_async()
            .await;

        let login_mock = server
            .mock("POST", "/login")
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("username".into(), "alice".into()),
                mockito::Matcher::UrlEncoded("password".into(), "hunter2".into()),
            ]))
            .with_status(200)
            .with_body("<html><head><title>Dashboard</title></head><body>Welcome alice</body></html>")
            .create_async()
            .await;

        let ctx = context();

        let dom = ctx.navigate(&server.url()).await.unwrap();
        page_mock.assert_async().await;
        assert_eq!(dom.title, "Login");
        // Set-Cookie on the page fetch made it into the session.
        assert_eq!(ctx.session().get_cookie("visited"), Some("1".to_string()));

        // query() does a real CSS-selector lookup against the live page.
        let matches = ctx.query("#help-link").await.unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "Need help?");

        // get_text() reads the real element, not a fabricated string.
        assert_eq!(ctx.get_text("help-link").await.unwrap(), "Need help?");

        ctx.input("user", "alice").await.unwrap();
        ctx.input("pass", "hunter2").await.unwrap();

        // click() on a submit button inside a form submits that form.
        let after_submit = ctx.click("submit-btn").await.unwrap();
        login_mock.assert_async().await;
        assert_eq!(after_submit.title, "Dashboard");
        assert_eq!(ctx.get_current_url(), format!("{}/login", server.url()));
    }

    #[tokio::test]
    async fn test_click_on_link_navigates() {
        let mut server = mockito::Server::new_async().await;
        let start = server
            .mock("GET", "/")
            .with_status(200)
            .with_body(r#"<html><body><a id="next" href="/next">Next</a></body></html>"#)
            .create_async()
            .await;
        let next = server
            .mock("GET", "/next")
            .with_status(200)
            .with_body("<html><head><title>Next Page</title></head><body>here</body></html>")
            .create_async()
            .await;

        let ctx = context();
        ctx.navigate(&server.url()).await.unwrap();
        start.assert_async().await;

        let dom = ctx.click("next").await.unwrap();
        next.assert_async().await;
        assert_eq!(dom.title, "Next Page");
    }

    #[tokio::test]
    async fn test_click_on_non_interactive_element_errors() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/")
            .with_status(200)
            .with_body(r#"<html><body><p id="text">just text</p></body></html>"#)
            .create_async()
            .await;

        let ctx = context();
        ctx.navigate(&server.url()).await.unwrap();

        // "text" isn't captured by from_html's role-specific extraction
        // (only buttons/links/form fields are), so find_by_id won't even
        // find it — still a real error, not a silent no-op.
        assert!(ctx.click("text").await.is_err());
    }
}
