use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub value: Option<String>,
}

pub struct FormManager {
    fields: HashMap<String, FormField>,
}

impl FormManager {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    pub fn fill_field(&mut self, name: String, value: String) -> Result<()> {
        self.fields.insert(name.clone(), FormField {
            name,
            value: Some(value),
        });
        Ok(())
    }

    pub fn get_form_data(&self) -> HashMap<String, Option<String>> {
        self.fields
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }
}

impl Default for FormManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_manager_creation() {
        let manager = FormManager::new();
        assert_eq!(manager.fields.len(), 0);
    }
}
