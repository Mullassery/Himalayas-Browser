/// Integration tests for India Stack Document Intelligence
#[cfg(test)]
mod tests {
    use crate::india_stack::*;

    #[tokio::test]
    async fn test_complete_document_workflow() {
        // Create document processor
        let processor = DocumentProcessor::new().unwrap();

        // Simulate processing a government form PDF
        let form_fields = processor.parse_form("license_renewal_form.pdf").await.unwrap();
        assert!(!form_fields.is_empty());

        // Validate form structure
        assert!(form_fields.iter().any(|f| f.name == "full_name"));
        assert!(form_fields.iter().any(|f| f.name == "phone_number"));
    }

    #[tokio::test]
    async fn test_ocr_and_validation_pipeline() {
        // Create OCR processor
        let ocr = ocr::OCRProcessor::new().unwrap();

        // Extract text from image
        let result = ocr.extract_text("test_image.png").await.unwrap();
        assert!(!result.text.is_empty());

        // Verify language detection works
        let language = ocr.detect_language(&result.text);
        assert!(!language.is_empty());
    }

    #[tokio::test]
    async fn test_pdf_form_type_identification() {
        let license_text = "LICENSE RENEWAL APPLICATION FORM";
        let form_type = pdf_parser::PDFParser::identify_form_type(license_text);
        assert_eq!(form_type, "license_renewal");

        let tax_text = "INCOME TAX RETURN FORM ITR-1";
        let form_type = pdf_parser::PDFParser::identify_form_type(tax_text);
        assert_eq!(form_type, "tax_filing");
    }

    #[tokio::test]
    async fn test_field_validation_pipeline() {
        let aadhaar_value = "1234 5678 9012".to_string();
        assert!(documents::FormValidator::validate_aadhaar(&aadhaar_value).unwrap());

        let field = FormField {
            name: "aadhaar".to_string(),
            field_type: documents::FieldType::Aadhaar,
            required: true,
            value: Some(aadhaar_value),
            validation_rules: vec![],
        };

        let processor = DocumentProcessor::new().unwrap();
        let mut fields = vec![field];

        // Create PAN field
        let pan_field = FormField {
            name: "pan".to_string(),
            field_type: documents::FieldType::PAN,
            required: true,
            value: Some("ABCDE1234F".to_string()),
            validation_rules: vec![],
        };
        fields.push(pan_field);

        let (valid, errors) = processor.validate_form(&fields).unwrap();
        assert!(valid);
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_multilingual_ocr() {
        let hindi_ocr = ocr::OCRProcessor::with_engine(ocr::OCREngine::MockHindi).unwrap();
        let hindi_result = hindi_ocr.extract_text("hindi_doc.png").await.unwrap();
        assert_eq!(hindi_result.language, "hindi");

        let tamil_ocr = ocr::OCRProcessor::with_engine(ocr::OCREngine::MockTamil).unwrap();
        let tamil_result = tamil_ocr.extract_text("tamil_doc.png").await.unwrap();
        assert_eq!(tamil_result.language, "tamil");

        let kannada_ocr = ocr::OCRProcessor::with_engine(ocr::OCREngine::MockKannada).unwrap();
        let kannada_result = kannada_ocr.extract_text("kannada_doc.png").await.unwrap();
        assert_eq!(kannada_result.language, "kannada");
    }

    #[tokio::test]
    async fn test_pdf_field_detection() {
        let fields = pdf_parser::PDFParser::detect_form_fields("test.pdf").await.unwrap();

        // Verify government form fields are detected
        let field_names: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(field_names.contains(&"full_name"));
        assert!(field_names.contains(&"pan_number"));
        assert!(field_names.contains(&"phone_number"));
        assert!(field_names.contains(&"signature_box"));
    }

    #[tokio::test]
    async fn test_table_detection_in_pdf() {
        let tables = pdf_parser::PDFParser::detect_tables("test.pdf").await.unwrap();
        assert!(!tables.is_empty());

        // Verify table structure
        let table = &tables[0];
        assert!(!table.headers.is_empty());
        assert!(!table.rows.is_empty());
    }

    #[tokio::test]
    async fn test_document_language_detection() {
        let hindi_text = "नमस्ते यह एक परीक्षण दस्तावेज है।";
        let processor = ocr::OCRProcessor::new().unwrap();
        let lang = processor.detect_language(hindi_text);
        assert_eq!(lang, "hindi");

        let english_text = "This is a test document.";
        let lang = processor.detect_language(english_text);
        assert_eq!(lang, "english");
    }
}
