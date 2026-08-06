use anyhow::Result;
use tracing::{debug, info};

use super::menu_bar::MenuBar;

/// Adapts menu bar based on device capabilities and user profile
pub struct MenuAdapter;

impl MenuAdapter {
    pub fn new() -> Result<Self> {
        debug!("Initializing Menu Adapter");
        info!("🎯 Menu adaptation: profile-aware feature enablement");

        Ok(Self)
    }

    /// Adapt menu bar for device profile and capabilities
    pub fn adapt_for_profile(&self, menu_bar: &MenuBar, profile: &str) -> Result<MenuBar> {
        debug!("Adapting menu bar for profile: {}", profile);

        let mut adapted_menus = menu_bar.menus.clone();

        match profile {
            "UltraCapability" => {
                // All features enabled
                // Keep everything
            }
            "HighCapability" => {
                // Most features enabled, may limit heavy AI models
                adapted_menus = adapted_menus
                    .into_iter()
                    .map(|mut menu| {
                        if menu.id == "ai" {
                            menu.items.retain(|item| {
                                item.requires_capability.as_deref() != Some("advanced_ai")
                            });
                        }
                        menu
                    })
                    .collect();
            }
            "Standard" => {
                // Cloud-based AI, limited developer tools
                adapted_menus = adapted_menus
                    .into_iter()
                    .map(|mut menu| {
                        if menu.id == "developer" {
                            menu.items.retain(|item| {
                                item.id != "network_analyzer" && item.id != "performance_monitor"
                            });
                        }
                        menu
                    })
                    .collect();
            }
            "LowMemory" => {
                // Minimal AI, no local models, limited tools
                adapted_menus = adapted_menus
                    .into_iter()
                    .filter(|menu| {
                        menu.id != "developer" && menu.id != "ai"
                    })
                    .collect();
            }
            "PowerSaver" => {
                // Only essential browsing
                adapted_menus = adapted_menus
                    .into_iter()
                    .filter(|menu| {
                        menu.id == "navigate" || menu.id == "window" || menu.id == "help"
                    })
                    .collect();
            }
            _ => {}
        }

        Ok(MenuBar {
            menus: adapted_menus,
        })
    }

    /// Filter menu items based on capabilities
    pub fn filter_by_capability(menu: &super::menu_bar::Menu, capabilities: &[String]) -> super::menu_bar::Menu {
        let mut filtered_menu = menu.clone();

        filtered_menu.items.retain(|item| {
            if let Some(ref required) = item.requires_capability {
                capabilities.contains(required)
            } else {
                true
            }
        });

        filtered_menu
    }

    /// Get capabilities for profile
    pub fn get_profile_capabilities(profile: &str) -> Vec<String> {
        match profile {
            "UltraCapability" => vec![
                "ai".to_string(),
                "advanced_ai".to_string(),
                "local_models".to_string(),
                "vision".to_string(),
                "agents".to_string(),
                "developer".to_string(),
                "gpu_acceleration".to_string(),
                "translation".to_string(),
            ],
            "HighCapability" => vec![
                "ai".to_string(),
                "local_models".to_string(),
                "agents".to_string(),
                "developer".to_string(),
                "translation".to_string(),
            ],
            "Standard" => vec![
                "ai".to_string(),
                "translation".to_string(),
            ],
            "LowMemory" => vec![
                "basic_browsing".to_string(),
            ],
            "PowerSaver" => vec![
                "basic_browsing".to_string(),
            ],
            _ => vec!["basic_browsing".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_adapter_creation() {
        let adapter = MenuAdapter::new().unwrap();
        assert!(true);
    }

    #[test]
    fn test_ultra_capability_profile() {
        let menu_bar = MenuBar::new().unwrap();
        let adapter = MenuAdapter::new().unwrap();
        let adapted = adapter.adapt_for_profile(&menu_bar, "UltraCapability").unwrap();
        assert_eq!(adapted.menus.len(), 9); // All menus present
    }

    #[test]
    fn test_power_saver_profile() {
        let menu_bar = MenuBar::new().unwrap();
        let adapter = MenuAdapter::new().unwrap();
        let adapted = adapter.adapt_for_profile(&menu_bar, "PowerSaver").unwrap();
        assert!(adapted.menus.len() < 9); // Some menus removed
    }

    #[test]
    fn test_get_profile_capabilities() {
        let caps = MenuAdapter::get_profile_capabilities("UltraCapability");
        assert!(caps.contains(&"ai".to_string()));
        assert!(caps.contains(&"local_models".to_string()));
    }

    #[test]
    fn test_low_memory_capabilities() {
        let caps = MenuAdapter::get_profile_capabilities("LowMemory");
        assert!(!caps.contains(&"ai".to_string()));
        assert!(caps.contains(&"basic_browsing".to_string()));
    }
}
