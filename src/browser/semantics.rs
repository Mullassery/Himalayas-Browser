use anyhow::Result;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ElementRole {
    Button,
    Link,
    Input,
    Form,
    Text,
    Container,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticElement {
    pub id: String,
    pub role: ElementRole,
    pub label: String,
    pub selector: String,
    pub attributes: HashMap<String, String>,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDOM {
    pub url: String,
    pub title: String,
    pub elements: Vec<SemanticElement>,
    pub forms: Vec<SemanticForm>,
    pub links: Vec<SemanticLink>,
    pub buttons: Vec<SemanticButton>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticForm {
    pub id: String,
    pub action: String,
    pub method: String,
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticLink {
    pub id: String,
    pub href: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticButton {
    pub id: String,
    pub label: String,
    pub button_type: String,
}

fn attrs_map(el: ElementRef) -> HashMap<String, String> {
    el.value()
        .attrs()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

fn own_text(el: ElementRef) -> String {
    el.text().collect::<Vec<_>>().join("").trim().to_string()
}

impl SemanticDOM {
    pub fn new(url: String, title: String) -> Self {
        Self {
            url,
            title,
            elements: Vec::new(),
            forms: Vec::new(),
            links: Vec::new(),
            buttons: Vec::new(),
        }
    }

    pub fn from_html(url: String, html: &str) -> Result<Self> {
        let document = Html::parse_document(html);
        let title = Self::extract_title_from_doc(&document);
        let mut dom = Self::new(url, title);
        dom.parse_elements(&document);
        Ok(dom)
    }

    /// Real title extraction via HTML parsing (handles attributes, nesting,
    /// entities, and whitespace correctly, unlike naive substring search).
    fn extract_title(html: &str) -> String {
        Self::extract_title_from_doc(&Html::parse_document(html))
    }

    fn extract_title_from_doc(document: &Html) -> String {
        let selector = Selector::parse("title").expect("static selector is valid");
        document
            .select(&selector)
            .next()
            .map(own_text)
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    fn parse_elements(&mut self, document: &Html) {
        self.parse_buttons(document);
        self.parse_links(document);
        self.parse_forms(document);
    }

    fn parse_buttons(&mut self, document: &Html) {
        // <button> elements and button-role <input> elements (submit/button/reset).
        let button_selector = Selector::parse("button").expect("static selector is valid");
        let input_button_selector = Selector::parse(
            r#"input[type="submit"], input[type="button"], input[type="reset"]"#,
        )
        .expect("static selector is valid");

        for (idx, el) in document
            .select(&button_selector)
            .chain(document.select(&input_button_selector))
            .enumerate()
        {
            let has_real_id = el.value().attr("id").is_some();
            let id = el
                .value()
                .attr("id")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("button_{}", idx));

            let label = if el.value().name() == "input" {
                el.value()
                    .attr("value")
                    .unwrap_or("Submit")
                    .trim()
                    .to_string()
            } else {
                let text = own_text(el);
                if text.is_empty() {
                    el.value().attr("value").unwrap_or("").trim().to_string()
                } else {
                    text
                }
            };

            let button_type = el.value().attr("type").unwrap_or("button").to_string();

            self.buttons.push(SemanticButton {
                id: id.clone(),
                label: label.clone(),
                button_type,
            });

            let selector = if has_real_id {
                format!("{}#{}", el.value().name(), id)
            } else {
                el.value().name().to_string()
            };

            self.elements.push(SemanticElement {
                id: id.clone(),
                role: ElementRole::Button,
                label: label.clone(),
                selector,
                attributes: attrs_map(el),
                text: label,
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 40.0,
                visible: true,
            });
        }
    }

    fn parse_links(&mut self, document: &Html) {
        let selector = Selector::parse("a[href]").expect("static selector is valid");

        for (idx, el) in document.select(&selector).enumerate() {
            let href = el.value().attr("href").unwrap_or("").to_string();
            let text = own_text(el);
            let has_real_id = el.value().attr("id").is_some();
            let id = el
                .value()
                .attr("id")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("link_{}", idx));

            self.links.push(SemanticLink {
                id: id.clone(),
                href: href.clone(),
                text: text.clone(),
            });

            let selector_str = if has_real_id {
                format!("a#{}", id)
            } else {
                format!(r#"a[href="{}"]"#, href)
            };

            self.elements.push(SemanticElement {
                id: id.clone(),
                role: ElementRole::Link,
                label: text.clone(),
                selector: selector_str,
                attributes: attrs_map(el),
                text,
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
                visible: true,
            });
        }
    }

    fn parse_forms(&mut self, document: &Html) {
        let form_selector = Selector::parse("form").expect("static selector is valid");
        let field_selector =
            Selector::parse("input, textarea, select").expect("static selector is valid");

        for (idx, form_el) in document.select(&form_selector).enumerate() {
            let has_real_id = form_el.value().attr("id").is_some();
            let id = form_el
                .value()
                .attr("id")
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("form_{}", idx));
            let action = form_el.value().attr("action").unwrap_or("").to_string();
            let method = form_el
                .value()
                .attr("method")
                .unwrap_or("get")
                .to_uppercase();

            let mut inputs = Vec::new();
            for field_el in form_el.select(&field_selector) {
                let field_type = field_el.value().attr("type").unwrap_or("text");
                if matches!(field_type, "submit" | "button" | "reset") {
                    // Already captured as a button above.
                    continue;
                }
                let name = field_el
                    .value()
                    .attr("name")
                    .or_else(|| field_el.value().attr("id"))
                    .unwrap_or("")
                    .to_string();

                let field_id = field_el
                    .value()
                    .attr("id")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("input_{}", inputs.len()));

                self.elements.push(SemanticElement {
                    id: field_id,
                    role: ElementRole::Input,
                    label: name.clone(),
                    selector: field_el
                        .value()
                        .attr("id")
                        .map(|i| format!("{}#{}", field_el.value().name(), i))
                        .unwrap_or_else(|| field_el.value().name().to_string()),
                    attributes: attrs_map(field_el),
                    text: String::new(),
                    x: 0.0,
                    y: 0.0,
                    width: 150.0,
                    height: 30.0,
                    visible: true,
                });

                if !name.is_empty() {
                    inputs.push(name);
                }
            }

            self.forms.push(SemanticForm {
                id: id.clone(),
                action: action.clone(),
                method: method.clone(),
                inputs,
            });

            let selector = if has_real_id {
                format!("form#{}", id)
            } else {
                "form".to_string()
            };

            self.elements.push(SemanticElement {
                id: id.clone(),
                role: ElementRole::Form,
                label: format!("Form ({})", method),
                selector,
                attributes: attrs_map(form_el),
                text: String::new(),
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 100.0,
                visible: true,
            });
        }
    }

    pub fn find_by_text(&self, text: &str) -> Option<&SemanticElement> {
        self.elements
            .iter()
            .find(|e| e.text.to_lowercase().contains(&text.to_lowercase()))
    }

    pub fn find_by_role(&self, role: ElementRole) -> Vec<&SemanticElement> {
        self.elements.iter().filter(|e| e.role == role).collect()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&SemanticElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    pub fn find_forms(&self) -> &[SemanticForm] {
        &self.forms
    }

    pub fn find_buttons(&self) -> &[SemanticButton] {
        &self.buttons
    }

    pub fn find_links(&self) -> &[SemanticLink] {
        &self.links
    }

    /// Real CSS-selector query (`AgentContext::query` in `src/api/mod.rs`) —
    /// re-parses `html` fresh rather than filtering the already-extracted
    /// `elements`/`forms`/`links`/`buttons`, since those only cover the
    /// specific structural roles `parse_elements` looks for (buttons, links,
    /// form fields), not arbitrary selectors like `.price` or `div.card > a`.
    /// Role is classified loosely by tag name, not full HTML semantics (any
    /// matched element that isn't a recognized interactive tag becomes
    /// `Container`) — good enough for an agent to locate and read/act on a
    /// match, not a substitute for `parse_buttons`/`parse_links`/`parse_forms`'s
    /// more careful per-role extraction.
    pub fn query_selector_all(html: &str, selector: &str) -> Result<Vec<SemanticElement>> {
        let document = Html::parse_document(html);
        let parsed_selector = Selector::parse(selector)
            .map_err(|e| anyhow::anyhow!("invalid selector {selector:?}: {e:?}"))?;

        Ok(document
            .select(&parsed_selector)
            .enumerate()
            .map(|(idx, el)| {
                let tag = el.value().name();
                let role = match tag {
                    "button" => ElementRole::Button,
                    "a" if el.value().attr("href").is_some() => ElementRole::Link,
                    "input" | "textarea" | "select" => ElementRole::Input,
                    "form" => ElementRole::Form,
                    _ => ElementRole::Container,
                };
                let has_real_id = el.value().attr("id").is_some();
                let id = el
                    .value()
                    .attr("id")
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("match_{idx}"));
                let text = own_text(el);
                let selector_str = if has_real_id { format!("{tag}#{id}") } else { tag.to_string() };

                SemanticElement {
                    id,
                    role,
                    label: text.clone(),
                    selector: selector_str,
                    attributes: attrs_map(el),
                    text,
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                    visible: true,
                }
            })
            .collect())
    }

    /// Walk up from the element with id `element_id` to the nearest
    /// ancestor `<form>`, returning its id — used by `AgentContext::click`
    /// to turn "click this submit button" into "submit its form", since the
    /// flat `SemanticForm`/`SemanticElement` extraction doesn't otherwise
    /// track parent/child relationships. Only works when `element_id` is a
    /// real HTML `id` attribute (usable directly as a CSS `#id` selector) —
    /// the synthetic `button_{n}`-style ids `parse_buttons` assigns to
    /// id-less buttons won't resolve here, a known limitation matching the
    /// synthetic-id caveat already documented on `SemanticElement`/`parse_buttons`.
    pub fn find_enclosing_form_id(html: &str, element_id: &str) -> Option<String> {
        let document = Html::parse_document(html);
        let selector = Selector::parse(&format!("#{element_id}")).ok()?;
        let start = document.select(&selector).next()?;

        let mut node = start.parent();
        while let Some(n) = node {
            if let Some(el) = ElementRef::wrap(n) {
                if el.value().name() == "form" {
                    return el.value().attr("id").map(str::to_string);
                }
            }
            node = n.parent();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_dom_creation() {
        let dom = SemanticDOM::new("https://example.com".to_string(), "Test".to_string());
        assert_eq!(dom.url, "https://example.com");
        assert_eq!(dom.title, "Test");
    }

    #[test]
    fn test_parse_title() {
        let html = "<html><head><title>My Page</title></head></html>";
        let title = SemanticDOM::extract_title(html);
        assert_eq!(title, "My Page");
    }

    #[test]
    fn test_parse_title_missing_falls_back_to_untitled() {
        let html = "<html><head></head><body>no title here</body></html>";
        assert_eq!(SemanticDOM::extract_title(html), "Untitled");
    }

    #[test]
    fn test_find_by_id() {
        let mut dom = SemanticDOM::new("https://example.com".to_string(), "Test".to_string());
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "btn1".to_string());
        dom.elements.push(SemanticElement {
            id: "btn1".to_string(),
            role: ElementRole::Button,
            label: "Click Me".to_string(),
            selector: "button#btn1".to_string(),
            attributes: attrs,
            text: "Click Me".to_string(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
            visible: true,
        });

        let elem = dom.find_by_id("btn1");
        assert!(elem.is_some());
        assert_eq!(elem.unwrap().label, "Click Me");
    }

    #[test]
    fn test_find_by_role() {
        let mut dom = SemanticDOM::new("https://example.com".to_string(), "Test".to_string());
        dom.elements.push(SemanticElement {
            id: "btn1".to_string(),
            role: ElementRole::Button,
            label: "Button 1".to_string(),
            selector: "button#btn1".to_string(),
            attributes: HashMap::new(),
            text: "Button 1".to_string(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 40.0,
            visible: true,
        });

        let buttons = dom.find_by_role(ElementRole::Button);
        assert_eq!(buttons.len(), 1);
    }

    #[test]
    fn test_parse_html_with_buttons() {
        let html = r#"<html><body><button id="submit">Submit</button></body></html>"#;
        let dom = SemanticDOM::from_html("https://example.com".to_string(), html).unwrap();
        assert_eq!(dom.buttons.len(), 1);
        assert_eq!(dom.buttons[0].label, "Submit");
    }

    #[test]
    fn test_parse_button_without_id_is_not_dropped() {
        // The old regex-based parser required a captured `id="..."` group,
        // so id-less buttons were silently invisible to the whole DOM. Real
        // parsing must not drop them.
        let html = r#"<html><body><button class="cta">No Id Here</button></body></html>"#;
        let dom = SemanticDOM::from_html("https://example.com".to_string(), html).unwrap();
        assert_eq!(dom.buttons.len(), 1);
        assert_eq!(dom.buttons[0].label, "No Id Here");
    }

    #[test]
    fn test_parse_nested_and_malformed_html() {
        // Real markup is rarely as clean as `<button id="x">text</button>`.
        // A regex parser breaks on nested tags and unclosed elements; a real
        // HTML5 parser (via html5ever/scraper) tolerates and recovers both.
        let html = r#"
            <html><head><title>Nested &amp; Broken</title>
            <body>
              <button id="go"><span>Go <b>Now</b></span></button>
              <p>Unclosed paragraph
              <a href="/next">Next <em>page</em></a>
        "#;
        let dom = SemanticDOM::from_html("https://example.com".to_string(), html).unwrap();
        assert_eq!(dom.title, "Nested & Broken");
        assert_eq!(dom.buttons.len(), 1);
        assert_eq!(dom.buttons[0].label, "Go Now");
        assert_eq!(dom.links.len(), 1);
        assert_eq!(dom.links[0].href, "/next");
        assert_eq!(dom.links[0].text, "Next page");
    }

    #[test]
    fn test_parse_form_with_inputs() {
        let html = r#"
            <html><body>
              <form id="login" action="/login" method="post">
                <input type="text" name="username" />
                <input type="password" name="password" />
                <input type="submit" value="Log In" />
              </form>
            </body></html>
        "#;
        let dom = SemanticDOM::from_html("https://example.com".to_string(), html).unwrap();

        assert_eq!(dom.forms.len(), 1);
        let form = &dom.forms[0];
        assert_eq!(form.id, "login");
        assert_eq!(form.action, "/login");
        assert_eq!(form.method, "POST");
        assert_eq!(form.inputs, vec!["username".to_string(), "password".to_string()]);

        // The submit input is reported as a button, not a duplicate form field.
        assert_eq!(dom.buttons.len(), 1);
        assert_eq!(dom.buttons[0].label, "Log In");

        let input_elements = dom.find_by_role(ElementRole::Input);
        assert_eq!(input_elements.len(), 2);

        let form_elements = dom.find_by_role(ElementRole::Form);
        assert_eq!(form_elements.len(), 1);
    }

    #[test]
    fn test_query_selector_all_matches_by_class() {
        let html = r#"<html><body>
            <div class="card"><span class="price">$10</span></div>
            <div class="card"><span class="price">$20</span></div>
            <div class="other"><span class="price">$30</span></div>
        </body></html>"#;

        let cards = SemanticDOM::query_selector_all(html, ".card").unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].role, ElementRole::Container);

        let prices = SemanticDOM::query_selector_all(html, ".card .price").unwrap();
        assert_eq!(prices.len(), 2);
        assert_eq!(prices[0].text, "$10");
        assert_eq!(prices[1].text, "$20");
    }

    #[test]
    fn test_query_selector_all_rejects_invalid_selector() {
        let html = "<html><body></body></html>";
        assert!(SemanticDOM::query_selector_all(html, ":::not-a-selector").is_err());
    }

    #[test]
    fn test_find_enclosing_form_id_resolves_button_inside_form() {
        let html = r#"<html><body>
            <form id="login">
                <input id="user" name="username" />
                <button id="submit-btn">Log In</button>
            </form>
        </body></html>"#;

        assert_eq!(
            SemanticDOM::find_enclosing_form_id(html, "submit-btn"),
            Some("login".to_string())
        );
    }

    #[test]
    fn test_find_enclosing_form_id_none_outside_form() {
        let html = r#"<html><body><button id="lonely">Click</button></body></html>"#;
        assert_eq!(SemanticDOM::find_enclosing_form_id(html, "lonely"), None);
    }
}
