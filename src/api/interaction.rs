use anyhow::Result;

pub trait InteractionAPI: Send + Sync {
    fn click(&self, element_id: &str) -> Result<()>;
    fn input(&self, element_id: &str, value: &str) -> Result<()>;
    fn submit_form(&self, form_id: &str) -> Result<()>;
    fn select_option(&self, element_id: &str, option: &str) -> Result<()>;
    fn hover(&self, element_id: &str) -> Result<()>;
    fn scroll(&self, x: f64, y: f64) -> Result<()>;
    fn execute_js(&self, script: &str) -> Result<serde_json::Value>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_trait() {
        // Trait-based test
        let _: &dyn InteractionAPI;
    }
}
