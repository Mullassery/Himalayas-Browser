use anyhow::Result;
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
        let mut dom = Self::new(url, Self::extract_title(html));

        // Extract elements from HTML
        // TODO: Implement proper HTML parsing with html5ever or similar
        dom.parse_elements(html);

        Ok(dom)
    }

    fn extract_title(html: &str) -> String {
        // Simple title extraction (improve with proper parsing)
        if let Some(start) = html.find("<title>") {
            if let Some(end) = html[start..].find("</title>") {
                let title = &html[start + 7..start + end];
                return title.to_string();
            }
        }
        "Untitled".to_string()
    }

    fn parse_elements(&mut self, html: &str) {
        // Simple button extraction
        let button_pattern = r#"<button[^>]*id="([^"]*)"[^>]*>([^<]*)</button>"#;
        if let Ok(re) = regex::Regex::new(button_pattern) {
            for cap in re.captures_iter(html) {
                if let (Some(id), Some(label)) = (cap.get(1), cap.get(2)) {
                    self.buttons.push(SemanticButton {
                        id: id.as_str().to_string(),
                        label: label.as_str().to_string(),
                        button_type: "button".to_string(),
                    });

                    let mut attrs = HashMap::new();
                    attrs.insert("id".to_string(), id.as_str().to_string());

                    self.elements.push(SemanticElement {
                        id: id.as_str().to_string(),
                        role: ElementRole::Button,
                        label: label.as_str().to_string(),
                        selector: format!(r#"button#{}"#, id.as_str()),
                        attributes: attrs,
                        text: label.as_str().to_string(),
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 40.0,
                        visible: true,
                    });
                }
            }
        }

        // Simple link extraction
        let link_pattern = r#"<a[^>]*href="([^"]*)"[^>]*>([^<]*)</a>"#;
        if let Ok(re) = regex::Regex::new(link_pattern) {
            let mut link_id = 0;
            for cap in re.captures_iter(html) {
                if let (Some(href), Some(text)) = (cap.get(1), cap.get(2)) {
                    let id = format!("link_{}", link_id);
                    self.links.push(SemanticLink {
                        id: id.clone(),
                        href: href.as_str().to_string(),
                        text: text.as_str().to_string(),
                    });

                    let mut attrs = HashMap::new();
                    attrs.insert("href".to_string(), href.as_str().to_string());

                    self.elements.push(SemanticElement {
                        id: id.clone(),
                        role: ElementRole::Link,
                        label: text.as_str().to_string(),
                        selector: format!(r#"a[href="{}"]"#, href.as_str()),
                        attributes: attrs,
                        text: text.as_str().to_string(),
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 20.0,
                        visible: true,
                    });
                    link_id += 1;
                }
            }
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
}
