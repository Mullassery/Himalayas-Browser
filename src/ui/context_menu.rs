use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::context_detector::SelectionContext;
use super::ContextType;

/// Context-aware right-click menu system
pub struct ContextMenu {
    default_items: Vec<MenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub category: String,
    pub action: ContextMenuAction,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextMenuAction {
    // AI Actions
    Summarize,
    Explain,
    ExplainTechnically,
    Translate,
    Rewrite,
    FactCheck,
    ExtractEntities,
    GenerateQuestions,

    // Knowledge Actions
    SaveToNotes,
    AddToResearchProject,
    CreateFlashcards,
    AddToKnowledgeGraph,

    // Search Actions
    SearchWeb,
    SearchLocal,
    SearchPreviousBrowsing,

    // Developer Actions
    DebugCode,
    OptimizeCode,
    GenerateTests,
    FindSecurityIssues,
    GenerateDocumentation,

    // Image Actions
    DescribeImage,
    IdentifyObjects,
    OCRText,
    ReverseImageSearch,

    // Browser Actions
    OpenLink,
    OpenInNewTab,
    Bookmark,
    Print,
    Cast,
    Screenshot,

    // Privacy Actions
    CheckPageSafety,
    InspectTrackers,
    RemovePageTraces,

    // Generic Actions
    Copy,
    Paste,
    SelectAll,
}

impl ContextMenu {
    pub fn new() -> Result<Self> {
        debug!("Initializing Context Menu System");
        info!("📋 Context menu: 8 selection types supported");

        Ok(Self {
            default_items: Self::create_default_items(),
        })
    }

    /// Generate context menu for detected selection type
    pub fn generate_for_context(&self, context: &SelectionContext) -> Result<ContextMenu> {
        debug!("Generating menu for context type: {:?}", context.context_type);

        let items = match context.context_type {
            ContextType::Page => Self::page_menu(),
            ContextType::TextSelection => Self::text_menu(),
            ContextType::ImageSelection => Self::image_menu(),
            ContextType::LinkSelection => Self::link_menu(),
            ContextType::VideoSelection => Self::video_menu(),
            ContextType::CodeSelection => Self::code_menu(),
            ContextType::PDFSelection => Self::pdf_menu(),
            ContextType::AgentContext => Self::agent_menu(),
        };

        Ok(ContextMenu { default_items: items })
    }

    /// Page context menu (right-click empty area)
    fn page_menu() -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "summarize_page".to_string(),
                label: "Summarize this page".to_string(),
                icon: "🤖".to_string(),
                category: "AI Actions".to_string(),
                action: ContextMenuAction::Summarize,
                enabled: true,
            },
            MenuItem {
                id: "explain_page".to_string(),
                label: "Explain this page".to_string(),
                icon: "💡".to_string(),
                category: "AI Actions".to_string(),
                action: ContextMenuAction::Explain,
                enabled: true,
            },
            MenuItem {
                id: "extract_data".to_string(),
                label: "Extract key information".to_string(),
                icon: "📊".to_string(),
                category: "AI Actions".to_string(),
                action: ContextMenuAction::ExtractEntities,
                enabled: true,
            },
            MenuItem {
                id: "bookmark".to_string(),
                label: "Bookmark".to_string(),
                icon: "📌".to_string(),
                category: "Save".to_string(),
                action: ContextMenuAction::Bookmark,
                enabled: true,
            },
            MenuItem {
                id: "check_safety".to_string(),
                label: "Check page safety".to_string(),
                icon: "🔒".to_string(),
                category: "Security".to_string(),
                action: ContextMenuAction::CheckPageSafety,
                enabled: true,
            },
        ]
    }

    /// Text selection context menu
    fn text_menu() -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "summarize_text".to_string(),
                label: "Summarize Selection".to_string(),
                icon: "🤖".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::Summarize,
                enabled: true,
            },
            MenuItem {
                id: "explain_text".to_string(),
                label: "Explain Simply".to_string(),
                icon: "💡".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::Explain,
                enabled: true,
            },
            MenuItem {
                id: "translate".to_string(),
                label: "Translate".to_string(),
                icon: "🌐".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::Translate,
                enabled: true,
            },
            MenuItem {
                id: "fact_check".to_string(),
                label: "Fact Check".to_string(),
                icon: "✓".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::FactCheck,
                enabled: true,
            },
            MenuItem {
                id: "save_notes".to_string(),
                label: "Save to Notes".to_string(),
                icon: "📝".to_string(),
                category: "Knowledge".to_string(),
                action: ContextMenuAction::SaveToNotes,
                enabled: true,
            },
            MenuItem {
                id: "copy".to_string(),
                label: "Copy".to_string(),
                icon: "📋".to_string(),
                category: "Basic".to_string(),
                action: ContextMenuAction::Copy,
                enabled: true,
            },
        ]
    }

    /// Image selection context menu
    fn image_menu() -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "describe_image".to_string(),
                label: "Describe Image".to_string(),
                icon: "🖼️".to_string(),
                category: "Vision AI".to_string(),
                action: ContextMenuAction::DescribeImage,
                enabled: true,
            },
            MenuItem {
                id: "identify_objects".to_string(),
                label: "Identify Objects".to_string(),
                icon: "🎯".to_string(),
                category: "Vision AI".to_string(),
                action: ContextMenuAction::IdentifyObjects,
                enabled: true,
            },
            MenuItem {
                id: "ocr".to_string(),
                label: "OCR Text".to_string(),
                icon: "📄".to_string(),
                category: "Vision AI".to_string(),
                action: ContextMenuAction::OCRText,
                enabled: true,
            },
            MenuItem {
                id: "reverse_search".to_string(),
                label: "Reverse Image Search".to_string(),
                icon: "🔍".to_string(),
                category: "Search".to_string(),
                action: ContextMenuAction::ReverseImageSearch,
                enabled: true,
            },
        ]
    }

    /// Link selection context menu
    fn link_menu() -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "open_link".to_string(),
                label: "Open Link".to_string(),
                icon: "🔗".to_string(),
                category: "Navigation".to_string(),
                action: ContextMenuAction::OpenLink,
                enabled: true,
            },
            MenuItem {
                id: "new_tab".to_string(),
                label: "Open in New Tab".to_string(),
                icon: "📑".to_string(),
                category: "Navigation".to_string(),
                action: ContextMenuAction::OpenInNewTab,
                enabled: true,
            },
            MenuItem {
                id: "summarize_link".to_string(),
                label: "Summarize Linked Page".to_string(),
                icon: "🤖".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::Summarize,
                enabled: true,
            },
            MenuItem {
                id: "check_reputation".to_string(),
                label: "Check Reputation".to_string(),
                icon: "🔒".to_string(),
                category: "Security".to_string(),
                action: ContextMenuAction::CheckPageSafety,
                enabled: true,
            },
        ]
    }

    /// Video selection context menu
    fn video_menu() -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "summarize_video".to_string(),
                label: "Summarize Video".to_string(),
                icon: "🎬".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::Summarize,
                enabled: true,
            },
            MenuItem {
                id: "transcript".to_string(),
                label: "Generate Transcript".to_string(),
                icon: "📝".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::ExtractEntities,
                enabled: true,
            },
            MenuItem {
                id: "translate_audio".to_string(),
                label: "Translate Audio".to_string(),
                icon: "🌐".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::Translate,
                enabled: true,
            },
        ]
    }

    /// Code selection context menu
    fn code_menu() -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "explain_code".to_string(),
                label: "Explain Code".to_string(),
                icon: "👨‍💻".to_string(),
                category: "Developer".to_string(),
                action: ContextMenuAction::Explain,
                enabled: true,
            },
            MenuItem {
                id: "debug_code".to_string(),
                label: "Debug Code".to_string(),
                icon: "🐛".to_string(),
                category: "Developer".to_string(),
                action: ContextMenuAction::DebugCode,
                enabled: true,
            },
            MenuItem {
                id: "optimize".to_string(),
                label: "Optimize Code".to_string(),
                icon: "⚡".to_string(),
                category: "Developer".to_string(),
                action: ContextMenuAction::OptimizeCode,
                enabled: true,
            },
            MenuItem {
                id: "gen_tests".to_string(),
                label: "Generate Tests".to_string(),
                icon: "✓".to_string(),
                category: "Developer".to_string(),
                action: ContextMenuAction::GenerateTests,
                enabled: true,
            },
            MenuItem {
                id: "security_check".to_string(),
                label: "Find Security Issues".to_string(),
                icon: "🔒".to_string(),
                category: "Developer".to_string(),
                action: ContextMenuAction::FindSecurityIssues,
                enabled: true,
            },
        ]
    }

    /// PDF/Document selection context menu
    fn pdf_menu() -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "summarize_doc".to_string(),
                label: "Summarize Document".to_string(),
                icon: "📄".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::Summarize,
                enabled: true,
            },
            MenuItem {
                id: "extract_tables".to_string(),
                label: "Extract Tables".to_string(),
                icon: "📊".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::ExtractEntities,
                enabled: true,
            },
            MenuItem {
                id: "compare_docs".to_string(),
                label: "Compare Versions".to_string(),
                icon: "🔀".to_string(),
                category: "AI".to_string(),
                action: ContextMenuAction::Summarize,
                enabled: true,
            },
        ]
    }

    /// Agent context menu
    fn agent_menu() -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: "delegate_research".to_string(),
                label: "Delegate to Research Agent".to_string(),
                icon: "🤖".to_string(),
                category: "Agents".to_string(),
                action: ContextMenuAction::Summarize,
                enabled: true,
            },
            MenuItem {
                id: "delegate_coding".to_string(),
                label: "Delegate to Coding Agent".to_string(),
                icon: "👨‍💻".to_string(),
                category: "Agents".to_string(),
                action: ContextMenuAction::GenerateTests,
                enabled: true,
            },
            MenuItem {
                id: "agent_permissions".to_string(),
                label: "Manage Permissions".to_string(),
                icon: "🔐".to_string(),
                category: "Agents".to_string(),
                action: ContextMenuAction::Summarize,
                enabled: true,
            },
        ]
    }

    fn create_default_items() -> Vec<MenuItem> {
        Self::page_menu()
    }

    pub fn get_item_count(&self) -> usize {
        self.default_items.len()
    }

    pub fn items(&self) -> &[MenuItem] {
        &self.default_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_menu_creation() {
        let menu = ContextMenu::new().unwrap();
        assert!(menu.get_item_count() > 0);
    }

    #[test]
    fn test_page_menu() {
        let items = ContextMenu::page_menu();
        assert!(items.len() > 0);
    }

    #[test]
    fn test_text_menu() {
        let items = ContextMenu::text_menu();
        assert!(items.len() > 0);
    }

    #[test]
    fn test_code_menu() {
        let items = ContextMenu::code_menu();
        assert!(items.iter().any(|i| i.action == ContextMenuAction::DebugCode));
    }
}
