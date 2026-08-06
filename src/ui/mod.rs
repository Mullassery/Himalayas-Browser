pub mod context_menu;
pub mod menu_bar;
pub mod context_detector;
pub mod menu_adapter;
pub mod interaction;

pub use context_menu::{ContextMenu, ContextMenuAction};
pub use menu_bar::{MenuBar, MenuItem};
pub use context_detector::{ContextDetector, SelectionContext, ContextType};
pub use menu_adapter::MenuAdapter;
pub use interaction::UIInteractionHandler;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, debug};

/// AI-Native UI Engine
/// Provides context-aware menus and adaptive interface based on selection, location, and device
pub struct UIEngine {
    context_detector: Arc<ContextDetector>,
    context_menu: Arc<ContextMenu>,
    menu_bar: Arc<MenuBar>,
    menu_adapter: Arc<MenuAdapter>,
    interaction_handler: Arc<UIInteractionHandler>,
    current_context: Arc<RwLock<Option<SelectionContext>>>,
}

impl UIEngine {
    pub async fn new() -> Result<Self> {
        info!("🎨 Initializing AI-Native UI Engine");

        let context_detector = Arc::new(ContextDetector::new()?);
        let context_menu = Arc::new(ContextMenu::new()?);
        let menu_bar = Arc::new(MenuBar::new()?);
        let menu_adapter = Arc::new(MenuAdapter::new()?);
        let interaction_handler = Arc::new(UIInteractionHandler::new()?);

        Ok(Self {
            context_detector,
            context_menu,
            menu_bar,
            menu_adapter,
            interaction_handler,
            current_context: Arc::new(RwLock::new(None)),
        })
    }

    /// Handle right-click event and generate context menu
    pub async fn on_right_click(&self, x: f32, y: f32, selection: Option<String>) -> Result<ContextMenu> {
        debug!("Right-click at ({}, {})", x, y);

        // Detect context type
        let context = self.context_detector.detect(selection).await?;
        *self.current_context.write() = Some(context.clone());

        // Generate appropriate menu for this context
        let menu = self.context_menu.generate_for_context(&context)?;

        info!("Generated context menu with {} items", menu.items().len());
        Ok(menu)
    }

    /// Get adaptive menu bar based on device capabilities and current profile
    pub async fn get_menu_bar(&self, profile: String) -> Result<MenuBar> {
        debug!("Generating menu bar for profile: {}", profile);

        // Adapt menu bar based on device/profile
        let adapted = self.menu_adapter.adapt_for_profile(&self.menu_bar, &profile)?;

        Ok(adapted)
    }

    /// Handle menu item selection
    pub async fn on_menu_action(&self, action: ContextMenuAction) -> Result<()> {
        debug!("Executing menu action: {:?}", action);

        self.interaction_handler.handle_action(action).await?;
        Ok(())
    }

    /// Get current UI context
    pub fn get_current_context(&self) -> Option<SelectionContext> {
        self.current_context.read().clone()
    }

    /// Get UI statistics
    pub fn get_stats(&self) -> UIStats {
        UIStats {
            context_menu_items: self.context_menu.get_item_count(),
            menu_bar_menus: self.menu_bar.get_menu_count(),
            context_types_supported: self.context_detector.supported_types(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIStats {
    pub context_menu_items: usize,
    pub menu_bar_menus: usize,
    pub context_types_supported: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ui_engine_creation() {
        let engine = UIEngine::new().await.unwrap();
        let stats = engine.get_stats();
        assert!(stats.context_types_supported > 0);
    }

    #[tokio::test]
    async fn test_context_detection() {
        let engine = UIEngine::new().await.unwrap();
        let menu = engine.on_right_click(100.0, 100.0, None).await.unwrap();
        assert!(menu.items().len() > 0);
    }

    #[tokio::test]
    async fn test_menu_bar_generation() {
        let engine = UIEngine::new().await.unwrap();
        let menu_bar = engine.get_menu_bar("Standard".to_string()).await.unwrap();
        assert!(menu_bar.menus.len() > 0);
    }
}
