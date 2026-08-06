use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Smart context detection from selection
pub struct ContextDetector {
    patterns: ContextPatterns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextType {
    Page,
    TextSelection,
    ImageSelection,
    LinkSelection,
    VideoSelection,
    CodeSelection,
    PDFSelection,
    AgentContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionContext {
    pub context_type: ContextType,
    pub content: String,
    pub content_length: usize,
    pub language: Option<String>,
    pub is_code: bool,
    pub media_type: Option<String>,
    pub url: Option<String>,
}

struct ContextPatterns {
    code_keywords: Vec<String>,
    code_extensions: Vec<String>,
}

impl ContextPatterns {
    fn new() -> Self {
        Self {
            code_keywords: vec![
                "function".to_string(), "class".to_string(), "def".to_string(),
                "let".to_string(), "const".to_string(), "var".to_string(),
                "import".to_string(), "export".to_string(), "return".to_string(),
                "async".to_string(), "await".to_string(), "try".to_string(),
            ],
            code_extensions: vec![
                ".py".to_string(), ".js".to_string(), ".ts".to_string(),
                ".java".to_string(), ".cpp".to_string(), ".rs".to_string(),
                ".go".to_string(), ".rb".to_string(), ".php".to_string(),
            ],
        }
    }
}

impl ContextDetector {
    pub fn new() -> Result<Self> {
        debug!("Initializing Context Detector");
        info!("🧠 Context intelligence: detecting selection types");

        Ok(Self {
            patterns: ContextPatterns::new(),
        })
    }

    /// Detect context type from selection
    pub async fn detect(&self, selection: Option<String>) -> Result<SelectionContext> {
        debug!("Detecting context from selection");

        let content = selection.unwrap_or_default();
        let content_length = content.len();

        // Detect if it's code
        let is_code = self.detect_code(&content);
        let language = if is_code { Some(self.detect_language(&content)) } else { None };

        // Determine context type
        let context_type = if content.is_empty() {
            ContextType::Page
        } else if is_code {
            ContextType::CodeSelection
        } else if self.is_url(&content) {
            ContextType::LinkSelection
        } else if content.contains("```") || content.contains("```") {
            ContextType::CodeSelection
        } else {
            ContextType::TextSelection
        };

        Ok(SelectionContext {
            context_type,
            content: content.clone(),
            content_length,
            language,
            is_code,
            media_type: None,
            url: self.extract_url(&content),
        })
    }

    /// Detect if selection is code
    fn detect_code(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();

        // Check for code keywords
        for keyword in &self.patterns.code_keywords {
            if text_lower.contains(keyword) {
                return true;
            }
        }

        // Check for code syntax patterns
        if text.contains("()") || text.contains("{}") || text.contains("[]") {
            if text.matches(|c| c == '(' || c == '{' || c == '[').count() > 3 {
                return true;
            }
        }

        // Check for common indentation patterns
        let lines: Vec<&str> = text.lines().collect();
        let indented_lines = lines.iter().filter(|l| l.starts_with("  ") || l.starts_with("\t")).count();
        if indented_lines > lines.len() / 2 && lines.len() > 3 {
            return true;
        }

        false
    }

    /// Detect programming language
    fn detect_language(&self, text: &str) -> String {
        let text_lower = text.to_lowercase();

        if text_lower.contains("def ") || text_lower.contains("import ") && text_lower.contains("from ") {
            "Python".to_string()
        } else if text_lower.contains("function") || text_lower.contains("const ") || text_lower.contains("let ") {
            if text_lower.contains("async") || text_lower.contains("await") {
                "JavaScript".to_string()
            } else {
                "JavaScript".to_string()
            }
        } else if text_lower.contains("class ") && text_lower.contains("public ") {
            "Java".to_string()
        } else if text_lower.contains("fn ") || text_lower.contains("mut ") {
            "Rust".to_string()
        } else if text_lower.contains("func ") || text_lower.contains("struct ") {
            "Go".to_string()
        } else {
            "Unknown".to_string()
        }
    }

    /// Check if text is a URL
    fn is_url(&self, text: &str) -> bool {
        text.starts_with("http://") || text.starts_with("https://") || text.contains("www.")
    }

    /// Extract URL from text if present
    fn extract_url(&self, text: &str) -> Option<String> {
        if self.is_url(text) {
            Some(text.to_string())
        } else {
            // Look for URL in text
            for word in text.split_whitespace() {
                if word.starts_with("http://") || word.starts_with("https://") {
                    return Some(word.to_string());
                }
            }
            None
        }
    }

    /// Get number of supported context types
    pub fn supported_types(&self) -> usize {
        8 // Page, Text, Image, Link, Video, Code, PDF, Agent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_detector_creation() {
        let detector = ContextDetector::new().unwrap();
        assert_eq!(detector.supported_types(), 8);
    }

    #[tokio::test]
    async fn test_empty_selection() {
        let detector = ContextDetector::new().unwrap();
        let context = detector.detect(None).await.unwrap();
        assert_eq!(context.context_type, ContextType::Page);
    }

    #[tokio::test]
    async fn test_text_selection() {
        let detector = ContextDetector::new().unwrap();
        let context = detector.detect(Some("This is some text".to_string())).await.unwrap();
        assert_eq!(context.context_type, ContextType::TextSelection);
    }

    #[tokio::test]
    async fn test_code_detection() {
        let detector = ContextDetector::new().unwrap();
        let code = "function hello() {\n  console.log('test');\n}".to_string();
        let context = detector.detect(Some(code)).await.unwrap();
        assert!(context.is_code);
        assert_eq!(context.context_type, ContextType::CodeSelection);
    }

    #[tokio::test]
    async fn test_language_detection() {
        let detector = ContextDetector::new().unwrap();
        let python_code = "def hello():\n    print('test')".to_string();
        let context = detector.detect(Some(python_code)).await.unwrap();
        assert_eq!(context.language, Some("Python".to_string()));
    }

    #[tokio::test]
    async fn test_url_detection() {
        let detector = ContextDetector::new().unwrap();
        let url = "https://example.com".to_string();
        let context = detector.detect(Some(url)).await.unwrap();
        assert!(context.url.is_some());
    }
}
