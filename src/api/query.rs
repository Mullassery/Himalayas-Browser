use anyhow::Result;

#[derive(Debug, Clone)]
pub enum QueryType {
    ById(String),
    ByText(String),
    ByRole(String),
    BySelector(String),
}

pub trait QueryAPI: Send + Sync {
    fn find(&self, query: QueryType) -> Result<Vec<String>>;
    fn get_text(&self, element_id: &str) -> Result<String>;
    fn get_attribute(&self, element_id: &str, attribute: &str) -> Result<String>;
    fn is_visible(&self, element_id: &str) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_types() {
        let _by_id = QueryType::ById("button1".to_string());
        let _by_text = QueryType::ByText("Submit".to_string());
        let _by_role = QueryType::ByRole("button".to_string());
        let _by_selector = QueryType::BySelector("button#submit".to_string());
    }
}
