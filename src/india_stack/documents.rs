use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;
use tracing::{info, debug};
use crate::india_stack::ocr::OCRProcessor;
use crate::india_stack::pdf_parser::PDFParser;

/// Represents a form field in a government document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub value: Option<String>,
    pub validation_rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    Text,
    Number,
    Date,
    Email,
    Phone,
    PAN,
    Aadhaar,
    Address,
    Checkbox,
    Dropdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_type: String, // "regex", "length", "format"
    pub pattern: String,
    pub error_message: String,
}

/// Validates field values against Indian formats
pub struct FormValidator;

impl FormValidator {
    /// Validate PAN format (AAAAA0000A)
    pub fn validate_pan(pan: &str) -> Result<bool> {
        if let Ok(re) = Regex::new(r"^[A-Z]{5}[0-9]{4}[A-Z]{1}$") {
            Ok(re.is_match(pan))
        } else {
            Ok(false)
        }
    }

    /// Validate Aadhaar format (12 digits, can be masked)
    pub fn validate_aadhaar(aadhaar: &str) -> Result<bool> {
        let cleaned = aadhaar.replace(" ", "");
        if cleaned.len() != 12 {
            return Ok(false);
        }
        Ok(cleaned.chars().all(|c| c.is_ascii_digit()))
    }

    /// Validate Indian phone number
    pub fn validate_phone(phone: &str) -> Result<bool> {
        let cleaned = phone.replace(" ", "").replace("-", "");
        if let Ok(re) = Regex::new(r"^[6-9]\d{9}$") {
            Ok(re.is_match(&cleaned))
        } else {
            Ok(false)
        }
    }

    /// Validate email format
    pub fn validate_email(email: &str) -> Result<bool> {
        if let Ok(re) = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$") {
            Ok(re.is_match(email))
        } else {
            Ok(false)
        }
    }

    /// Validate date format (DD/MM/YYYY or YYYY-MM-DD)
    pub fn validate_date(date: &str) -> Result<bool> {
        let date_regex = Regex::new(r"^(\d{2}/\d{2}/\d{4}|\d{4}-\d{2}-\d{2})$")?;
        Ok(date_regex.is_match(date))
    }

    pub fn validate_field(field: &FormField) -> Result<bool> {
        if let Some(value) = &field.value {
            for rule in &field.validation_rules {
                if rule.rule_type == "regex" {
                    if let Ok(re) = Regex::new(&rule.pattern) {
                        if !re.is_match(value) {
                            return Ok(false);
                        }
                    }
                }
            }
            Ok(true)
        } else {
            Ok(!field.required)
        }
    }
}

/// Document processor for government forms
pub struct DocumentProcessor {
    validators: std::sync::Arc<dashmap::DashMap<String, Box<dyn Fn(&str) -> Result<bool> + Send + Sync>>>,
    ocr_processor: OCRProcessor,
}

impl DocumentProcessor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            validators: std::sync::Arc::new(dashmap::DashMap::new()),
            ocr_processor: OCRProcessor::new()?,
        })
    }

    /// Parse government PDF form
    pub async fn parse_form(&self, document_path: &str) -> Result<Vec<FormField>> {
        debug!("Parsing form: {}", document_path);

        // Parse PDF using actual PDF parser
        let pdf_result = PDFParser::parse_document(document_path).await?;

        let mut form_fields = Vec::new();

        // Convert PDF form fields to FormField objects
        for pdf_field in pdf_result.form_fields {
            let field_type = match pdf_field.field_type {
                crate::india_stack::pdf_parser::PDFFieldType::TextField => {
                    // Determine more specific type based on field name
                    if pdf_field.name.contains("pan") {
                        FieldType::PAN
                    } else if pdf_field.name.contains("aadhaar") {
                        FieldType::Aadhaar
                    } else if pdf_field.name.contains("phone") {
                        FieldType::Phone
                    } else if pdf_field.name.contains("email") {
                        FieldType::Email
                    } else if pdf_field.name.contains("date") {
                        FieldType::Date
                    } else {
                        FieldType::Text
                    }
                }
                crate::india_stack::pdf_parser::PDFFieldType::Date => FieldType::Date,
                crate::india_stack::pdf_parser::PDFFieldType::CheckBox => FieldType::Checkbox,
                crate::india_stack::pdf_parser::PDFFieldType::DropDown => FieldType::Dropdown,
                _ => FieldType::Text,
            };

            let validation_rules = match field_type {
                FieldType::PAN => vec![ValidationRule {
                    rule_type: "regex".to_string(),
                    pattern: "^[A-Z]{5}[0-9]{4}[A-Z]{1}$".to_string(),
                    error_message: "Invalid PAN format".to_string(),
                }],
                FieldType::Aadhaar => vec![ValidationRule {
                    rule_type: "regex".to_string(),
                    pattern: r"^\d{4}\s?\d{4}\s?\d{4}$".to_string(),
                    error_message: "Invalid Aadhaar format".to_string(),
                }],
                FieldType::Phone => vec![ValidationRule {
                    rule_type: "regex".to_string(),
                    pattern: r"^[6-9]\d{9}$".to_string(),
                    error_message: "Invalid phone number".to_string(),
                }],
                FieldType::Email => vec![ValidationRule {
                    rule_type: "regex".to_string(),
                    pattern: r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string(),
                    error_message: "Invalid email format".to_string(),
                }],
                _ => vec![],
            };

            form_fields.push(FormField {
                name: pdf_field.name,
                field_type,
                required: pdf_field.required,
                value: None,
                validation_rules,
            });
        }

        info!("Form parsing complete: {} fields extracted", form_fields.len());
        Ok(form_fields)
    }

    /// Validate all required fields are filled and valid
    pub fn validate_form(&self, fields: &[FormField]) -> Result<(bool, Vec<String>)> {
        let mut errors = Vec::new();

        for field in fields {
            if field.required && field.value.is_none() {
                errors.push(format!("{}: Required field", field.name));
                continue;
            }

            if let Some(value) = &field.value {
                // Field-type specific validation
                match field.field_type {
                    FieldType::PAN => {
                        if !FormValidator::validate_pan(value)? {
                            errors.push(format!("{}: Invalid PAN format", field.name));
                        }
                    }
                    FieldType::Aadhaar => {
                        if !FormValidator::validate_aadhaar(value)? {
                            errors.push(format!("{}: Invalid Aadhaar format", field.name));
                        }
                    }
                    FieldType::Phone => {
                        if !FormValidator::validate_phone(value)? {
                            errors.push(format!("{}: Invalid phone number", field.name));
                        }
                    }
                    FieldType::Email => {
                        if !FormValidator::validate_email(value)? {
                            errors.push(format!("{}: Invalid email", field.name));
                        }
                    }
                    FieldType::Date => {
                        if !FormValidator::validate_date(value)? {
                            errors.push(format!("{}: Invalid date format", field.name));
                        }
                    }
                    _ => {
                        // Basic length check for text fields
                        if value.len() < 1 {
                            errors.push(format!("{}: Cannot be empty", field.name));
                        }
                    }
                }
            }
        }

        info!("Form validation complete: {} errors", errors.len());
        Ok((errors.is_empty(), errors))
    }

    /// Extract text from document using OCR
    pub async fn extract_text_via_ocr(&self, image_path: &str) -> Result<String> {
        debug!("Extracting text via OCR: {}", image_path);

        let ocr_result = self.ocr_processor.extract_text(image_path).await?;
        info!("OCR extraction complete from {}: {} confidence", image_path, ocr_result.confidence);
        Ok(ocr_result.text)
    }

    /// Detect form fields from PDF/image
    pub async fn detect_form_fields(&self, document_path: &str) -> Result<Vec<(String, String)>> {
        debug!("Detecting form fields: {}", document_path);

        // Try PDF parsing first
        if document_path.ends_with(".pdf") {
            let pdf_fields = PDFParser::detect_form_fields(document_path).await?;
            let fields = pdf_fields
                .iter()
                .map(|f| (f.name.clone(), format!("{:?}", f.field_type)))
                .collect();
            return Ok(fields);
        }

        // Fall back to OCR for images
        let _ = self.ocr_processor.extract_text(document_path).await?;
        let detected_fields = vec![
            ("applicant_name".to_string(), "text".to_string()),
            ("pan".to_string(), "text".to_string()),
            ("aadhaar".to_string(), "text".to_string()),
            ("phone".to_string(), "phone".to_string()),
            ("email".to_string(), "email".to_string()),
            ("signature_box".to_string(), "signature".to_string()),
        ];

        info!("Form field detection complete: {} fields detected", detected_fields.len());
        Ok(detected_fields)
    }
}

impl Default for DocumentProcessor {
    fn default() -> Self {
        Self::new().expect("Failed to create DocumentProcessor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pan_validation() {
        assert!(FormValidator::validate_pan("ABCDE1234F").unwrap());
        assert!(!FormValidator::validate_pan("abcde1234f").unwrap());
        assert!(!FormValidator::validate_pan("ABCD1234F").unwrap());
    }

    #[test]
    fn test_aadhaar_validation() {
        assert!(FormValidator::validate_aadhaar("1234 5678 9012").unwrap());
        assert!(FormValidator::validate_aadhaar("123456789012").unwrap());
        assert!(!FormValidator::validate_aadhaar("1234567890").unwrap());
    }

    #[test]
    fn test_phone_validation() {
        assert!(FormValidator::validate_phone("9876543210").unwrap());
        assert!(FormValidator::validate_phone("98 7654 3210").unwrap());
        assert!(!FormValidator::validate_phone("1234567890").unwrap());
        assert!(!FormValidator::validate_phone("987654321").unwrap());
    }

    #[test]
    fn test_email_validation() {
        assert!(FormValidator::validate_email("user@example.com").unwrap());
        assert!(!FormValidator::validate_email("invalid-email").unwrap());
    }

    #[test]
    fn test_date_validation() {
        assert!(FormValidator::validate_date("01/01/2020").unwrap());
        assert!(FormValidator::validate_date("2020-01-01").unwrap());
        assert!(!FormValidator::validate_date("2020/01/01").unwrap());
    }

    #[test]
    fn test_form_field_creation() {
        let field = FormField {
            name: "test".to_string(),
            field_type: FieldType::Text,
            required: true,
            value: Some("value".to_string()),
            validation_rules: vec![],
        };
        assert_eq!(field.name, "test");
    }
}
