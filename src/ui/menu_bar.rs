use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Adaptive menu bar for AI-native browser
pub struct MenuBar {
    pub menus: Vec<Menu>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Menu {
    pub id: String,
    pub label: String,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub submenu: Option<Vec<MenuItem>>,
    pub action: Option<String>,
    pub requires_capability: Option<String>,
}

impl MenuBar {
    pub fn new() -> Result<Self> {
        debug!("Initializing Adaptive Menu Bar");
        info!("📊 Menu bar: 9 menus with context-aware items");

        Ok(Self {
            menus: Self::create_default_menus(),
        })
    }

    fn create_default_menus() -> Vec<Menu> {
        vec![
            Self::browser_menu(),
            Self::ai_menu(),
            Self::navigate_menu(),
            Self::workspace_menu(),
            Self::security_menu(),
            Self::tools_menu(),
            Self::developer_menu(),
            Self::window_menu(),
            Self::help_menu(),
        ]
    }

    fn browser_menu() -> Menu {
        Menu {
            id: "browser".to_string(),
            label: "Browser".to_string(),
            items: vec![
                MenuItem {
                    id: "new_window".to_string(),
                    label: "New Window".to_string(),
                    submenu: None,
                    action: Some("NewWindow".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "new_private".to_string(),
                    label: "New Private Window".to_string(),
                    submenu: None,
                    action: Some("NewPrivateWindow".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "profiles".to_string(),
                    label: "Profiles".to_string(),
                    submenu: None,
                    action: Some("ManageProfiles".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "settings".to_string(),
                    label: "Settings".to_string(),
                    submenu: None,
                    action: Some("OpenSettings".to_string()),
                    requires_capability: None,
                },
            ],
        }
    }

    fn ai_menu() -> Menu {
        Menu {
            id: "ai".to_string(),
            label: "AI".to_string(),
            items: vec![
                MenuItem {
                    id: "ai_assistant".to_string(),
                    label: "AI Assistant".to_string(),
                    submenu: None,
                    action: Some("OpenAssistant".to_string()),
                    requires_capability: Some("ai".to_string()),
                },
                MenuItem {
                    id: "page_intelligence".to_string(),
                    label: "Page Intelligence".to_string(),
                    submenu: Some(vec![
                        MenuItem {
                            id: "summarize".to_string(),
                            label: "Summarize".to_string(),
                            submenu: None,
                            action: Some("SummarizePage".to_string()),
                            requires_capability: Some("ai".to_string()),
                        },
                        MenuItem {
                            id: "explain".to_string(),
                            label: "Explain".to_string(),
                            submenu: None,
                            action: Some("ExplainPage".to_string()),
                            requires_capability: Some("ai".to_string()),
                        },
                    ]),
                    action: None,
                    requires_capability: Some("ai".to_string()),
                },
                MenuItem {
                    id: "agents".to_string(),
                    label: "Agents".to_string(),
                    submenu: Some(vec![
                        MenuItem {
                            id: "active_agents".to_string(),
                            label: "Active Agents".to_string(),
                            submenu: None,
                            action: Some("ShowAgents".to_string()),
                            requires_capability: Some("agents".to_string()),
                        },
                        MenuItem {
                            id: "permissions".to_string(),
                            label: "Permissions".to_string(),
                            submenu: None,
                            action: Some("ManageAgentPermissions".to_string()),
                            requires_capability: Some("agents".to_string()),
                        },
                    ]),
                    action: None,
                    requires_capability: Some("agents".to_string()),
                },
            ],
        }
    }

    fn navigate_menu() -> Menu {
        Menu {
            id: "navigate".to_string(),
            label: "Navigate".to_string(),
            items: vec![
                MenuItem {
                    id: "back".to_string(),
                    label: "Back".to_string(),
                    submenu: None,
                    action: Some("NavigateBack".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "forward".to_string(),
                    label: "Forward".to_string(),
                    submenu: None,
                    action: Some("NavigateForward".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "reload".to_string(),
                    label: "Reload".to_string(),
                    submenu: None,
                    action: Some("Reload".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "history".to_string(),
                    label: "History".to_string(),
                    submenu: None,
                    action: Some("ShowHistory".to_string()),
                    requires_capability: None,
                },
            ],
        }
    }

    fn workspace_menu() -> Menu {
        Menu {
            id: "workspace".to_string(),
            label: "Workspace".to_string(),
            items: vec![
                MenuItem {
                    id: "research".to_string(),
                    label: "Research".to_string(),
                    submenu: None,
                    action: Some("SelectWorkspace:research".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "coding".to_string(),
                    label: "Coding".to_string(),
                    submenu: None,
                    action: Some("SelectWorkspace:coding".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "writing".to_string(),
                    label: "Writing".to_string(),
                    submenu: None,
                    action: Some("SelectWorkspace:writing".to_string()),
                    requires_capability: None,
                },
            ],
        }
    }

    fn security_menu() -> Menu {
        Menu {
            id: "security".to_string(),
            label: "Security".to_string(),
            items: vec![
                MenuItem {
                    id: "privacy_dashboard".to_string(),
                    label: "Privacy Dashboard".to_string(),
                    submenu: None,
                    action: Some("OpenPrivacyDashboard".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "tracker_protection".to_string(),
                    label: "Tracker Protection".to_string(),
                    submenu: None,
                    action: Some("ShowTrackerProtection".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "permissions".to_string(),
                    label: "Permission Manager".to_string(),
                    submenu: None,
                    action: Some("ManagePermissions".to_string()),
                    requires_capability: None,
                },
            ],
        }
    }

    fn tools_menu() -> Menu {
        Menu {
            id: "tools".to_string(),
            label: "Tools".to_string(),
            items: vec![
                MenuItem {
                    id: "downloads".to_string(),
                    label: "Downloads".to_string(),
                    submenu: None,
                    action: Some("ShowDownloads".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "translate".to_string(),
                    label: "Translate".to_string(),
                    submenu: None,
                    action: Some("TranslateContent".to_string()),
                    requires_capability: Some("translation".to_string()),
                },
                MenuItem {
                    id: "screenshot".to_string(),
                    label: "Screenshot".to_string(),
                    submenu: None,
                    action: Some("TakeScreenshot".to_string()),
                    requires_capability: None,
                },
            ],
        }
    }

    fn developer_menu() -> Menu {
        Menu {
            id: "developer".to_string(),
            label: "Developer".to_string(),
            items: vec![
                MenuItem {
                    id: "devtools".to_string(),
                    label: "Developer Tools".to_string(),
                    submenu: None,
                    action: Some("OpenDevTools".to_string()),
                    requires_capability: Some("developer".to_string()),
                },
                MenuItem {
                    id: "inspector".to_string(),
                    label: "Inspector".to_string(),
                    submenu: None,
                    action: Some("OpenInspector".to_string()),
                    requires_capability: Some("developer".to_string()),
                },
                MenuItem {
                    id: "console".to_string(),
                    label: "Console".to_string(),
                    submenu: None,
                    action: Some("OpenConsole".to_string()),
                    requires_capability: Some("developer".to_string()),
                },
            ],
        }
    }

    fn window_menu() -> Menu {
        Menu {
            id: "window".to_string(),
            label: "Window".to_string(),
            items: vec![
                MenuItem {
                    id: "new_tab".to_string(),
                    label: "New Tab".to_string(),
                    submenu: None,
                    action: Some("NewTab".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "split_view".to_string(),
                    label: "Split View".to_string(),
                    submenu: None,
                    action: Some("SplitView".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "tab_groups".to_string(),
                    label: "Tab Groups".to_string(),
                    submenu: None,
                    action: Some("ManageTabGroups".to_string()),
                    requires_capability: None,
                },
            ],
        }
    }

    fn help_menu() -> Menu {
        Menu {
            id: "help".to_string(),
            label: "Help".to_string(),
            items: vec![
                MenuItem {
                    id: "getting_started".to_string(),
                    label: "Getting Started".to_string(),
                    submenu: None,
                    action: Some("ShowGettingStarted".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "shortcuts".to_string(),
                    label: "Keyboard Shortcuts".to_string(),
                    submenu: None,
                    action: Some("ShowShortcuts".to_string()),
                    requires_capability: None,
                },
                MenuItem {
                    id: "report_issue".to_string(),
                    label: "Report Issue".to_string(),
                    submenu: None,
                    action: Some("ReportIssue".to_string()),
                    requires_capability: None,
                },
            ],
        }
    }

    pub fn get_menu_count(&self) -> usize {
        self.menus.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_bar_creation() {
        let menu_bar = MenuBar::new().unwrap();
        assert_eq!(menu_bar.get_menu_count(), 9);
    }

    #[test]
    fn test_browser_menu() {
        let menu = MenuBar::browser_menu();
        assert!(!menu.items.is_empty());
    }

    #[test]
    fn test_ai_menu() {
        let menu = MenuBar::ai_menu();
        assert!(menu.items.iter().any(|i| i.requires_capability == Some("ai".to_string())));
    }
}
