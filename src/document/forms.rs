use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::{Result, anyhow};
use tracing::{debug, info};

/// Form field with validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormField {
    pub name: String,
    pub field_type: FieldType,
    pub value: Option<String>,
    pub required: bool,
    pub validation_pattern: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FieldType {
    Text,
    Email,
    Phone,
    Date,
    Number,
    Checkbox,
    RadioButton,
    Dropdown,
    TextArea,
    Signature,
}

/// Form manager with validation and extraction
pub struct FormManager {
    fields: Arc<RwLock<HashMap<String, FormField>>>,
    form_type: Option<String>,
    autofill_enabled: bool,
}

impl FormManager {
    pub fn new() -> Self {
        debug!("Initializing Form Manager");
        Self {
            fields: Arc::new(RwLock::new(HashMap::new())),
            form_type: None,
            autofill_enabled: true,
        }
    }

    /// Add form field with validation rules
    pub fn add_field(
        &self,
        name: String,
        field_type: FieldType,
        required: bool,
    ) -> Result<()> {
        let field = FormField {
            name: name.clone(),
            field_type,
            value: None,
            required,
            validation_pattern: Self::default_pattern(field_type),
            error_message: None,
        };

        let mut fields = self.fields.write();
        fields.insert(name.clone(), field);
        info!("Added form field: {}", name);
        Ok(())
    }

    /// Fill field with validation
    pub fn fill_field(&self, name: String, value: String) -> Result<()> {
        let mut fields = self.fields.write();
        let field = fields
            .get_mut(&name)
            .ok_or_else(|| anyhow!("Field not found: {}", name))?;

        // Validate
        if let Some(pattern) = &field.validation_pattern {
            let re = regex::Regex::new(pattern)?;
            if !re.is_match(&value) {
                field.error_message = Some(format!("Invalid format for field: {}", name));
                return Err(anyhow!("Validation failed for field: {}", name));
            }
        }

        field.value = Some(value);
        field.error_message = None;
        info!("Filled field: {}", name);
        Ok(())
    }

    /// Validate all required fields
    pub fn validate(&self) -> Result<bool> {
        let fields = self.fields.read();
        for (name, field) in fields.iter() {
            if field.required && field.value.is_none() {
                return Err(anyhow!("Required field missing: {}", name));
            }
        }
        Ok(true)
    }

    /// Get form data
    pub fn get_form_data(&self) -> HashMap<String, Option<String>> {
        self.fields
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }

    /// Get field value
    pub fn get_field_value(&self, name: &str) -> Option<String> {
        self.fields.read().get(name).and_then(|f| f.value.clone())
    }

    /// Clear all fields
    pub fn clear_all(&self) -> Result<()> {
        let mut fields = self.fields.write();
        for field in fields.values_mut() {
            field.value = None;
            field.error_message = None;
        }
        info!("Cleared all form fields");
        Ok(())
    }

    /// Export as JSON
    pub fn export_as_json(&self) -> String {
        serde_json::to_string_pretty(&self.get_form_data()).unwrap_or_default()
    }

    /// Detect form type from fields
    pub fn detect_form_type(&self) -> FormType {
        let fields = self.fields.read();
        let field_count = fields.len();

        if field_count == 0 {
            return FormType::Unknown;
        }

        let has_email = fields.values().any(|f| f.field_type == FieldType::Email);
        let has_phone = fields.values().any(|f| f.field_type == FieldType::Phone);
        let has_signature = fields.values().any(|f| f.field_type == FieldType::Signature);
        let has_date = fields.values().any(|f| f.field_type == FieldType::Date);

        if has_signature {
            FormType::Agreement
        } else if has_email && has_phone {
            FormType::Registration
        } else if has_date {
            FormType::Application
        } else if has_email {
            FormType::Contact
        } else {
            FormType::Generic
        }
    }

    /// Get form statistics
    pub fn get_stats(&self) -> FormStats {
        let fields = self.fields.read();
        let total_fields = fields.len();
        let filled_fields = fields.values().filter(|f| f.value.is_some()).count();
        let required_fields = fields.values().filter(|f| f.required).count();

        FormStats {
            total_fields,
            filled_fields,
            required_fields,
            completion_percentage: if total_fields == 0 {
                0.0
            } else {
                (filled_fields as f32 / total_fields as f32) * 100.0
            },
        }
    }

    fn default_pattern(field_type: FieldType) -> Option<String> {
        match field_type {
            FieldType::Email => Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string()),
            FieldType::Phone => Some(r"^\d{10,}$".to_string()),
            FieldType::Date => Some(r"^\d{4}-\d{2}-\d{2}$".to_string()),
            FieldType::Number => Some(r"^-?\d+(\.\d+)?$".to_string()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormType {
    Registration,
    Contact,
    Application,
    Agreement,
    Generic,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct FormStats {
    pub total_fields: usize,
    pub filled_fields: usize,
    pub required_fields: usize,
    pub completion_percentage: f32,
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
        assert_eq!(manager.get_form_data().len(), 0);
    }

    #[test]
    fn test_add_field() {
        let manager = FormManager::new();
        manager.add_field("name".to_string(), FieldType::Text, true).unwrap();

        let data = manager.get_form_data();
        assert_eq!(data.len(), 1);
    }

    #[test]
    fn test_fill_field() {
        let manager = FormManager::new();
        manager.add_field("email".to_string(), FieldType::Email, true).unwrap();
        manager.fill_field("email".to_string(), "test@example.com".to_string()).unwrap();

        assert_eq!(manager.get_field_value("email"), Some("test@example.com".to_string()));
    }

    #[test]
    fn test_email_validation() {
        let manager = FormManager::new();
        manager.add_field("email".to_string(), FieldType::Email, true).unwrap();

        let invalid = manager.fill_field("email".to_string(), "not-an-email".to_string());
        assert!(invalid.is_err());
    }

    #[test]
    fn test_form_validation() {
        let manager = FormManager::new();
        manager.add_field("name".to_string(), FieldType::Text, true).unwrap();
        manager.add_field("optional".to_string(), FieldType::Text, false).unwrap();

        let result = manager.validate();
        assert!(result.is_err()); // Required field missing

        manager.fill_field("name".to_string(), "John".to_string()).unwrap();
        assert!(manager.validate().unwrap());
    }

    #[test]
    fn test_form_type_detection() {
        let manager = FormManager::new();
        manager.add_field("email".to_string(), FieldType::Email, true).unwrap();
        manager.add_field("phone".to_string(), FieldType::Phone, true).unwrap();

        assert_eq!(manager.detect_form_type(), FormType::Registration);
    }

    #[test]
    fn test_form_stats() {
        let manager = FormManager::new();
        manager.add_field("f1".to_string(), FieldType::Text, true).unwrap();
        manager.add_field("f2".to_string(), FieldType::Text, false).unwrap();

        manager.fill_field("f1".to_string(), "value1".to_string()).unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_fields, 2);
        assert_eq!(stats.filled_fields, 1);
        assert_eq!(stats.required_fields, 1);
    }
}
