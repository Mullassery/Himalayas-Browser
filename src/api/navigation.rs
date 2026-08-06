use anyhow::Result;

pub trait NavigationAPI: Send + Sync {
    fn navigate(&self, url: &str) -> Result<()>;
    fn go_back(&self) -> Result<()>;
    fn go_forward(&self) -> Result<()>;
    fn get_current_url(&self) -> String;
    fn get_history(&self) -> Vec<String>;
    fn reload(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_trait() {
        // Trait-based test
        let _: &dyn NavigationAPI;
    }
}
