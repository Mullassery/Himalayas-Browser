pub mod identity;
pub mod documents;
pub mod workflows;
pub mod ocr;
pub mod pdf_parser;
pub mod integration;

pub use identity::{IdentityProvider, AadhaarAuth, DigiLockerClient, eSignClient};
pub use documents::{DocumentProcessor, FormField, FormValidator};
pub use workflows::WorkflowExecutor;
pub use ocr::{OCRProcessor, OCREngine, OCRResult, TextBlock};
pub use pdf_parser::{PDFParser, PDFFormField, PDFParseResult};

use anyhow::Result;
use std::sync::Arc;

/// India Stack integration layer
///
/// Provides high-level APIs for government workflows:
/// - Authentication (Aadhaar, DigiLocker)
/// - Document handling (retrieval, signing)
/// - Form processing (filling, validation)
pub struct IndiaStack {
    identity: Arc<IdentityProvider>,
    documents: Arc<DocumentProcessor>,
    workflows: Arc<WorkflowExecutor>,
}

impl IndiaStack {
    pub fn new() -> Result<Self> {
        Ok(Self {
            identity: Arc::new(IdentityProvider::new()?),
            documents: Arc::new(DocumentProcessor::new()?),
            workflows: Arc::new(WorkflowExecutor::new()?),
        })
    }

    pub fn identity(&self) -> Arc<IdentityProvider> {
        self.identity.clone()
    }

    pub fn documents(&self) -> Arc<DocumentProcessor> {
        self.documents.clone()
    }

    pub fn workflows(&self) -> Arc<WorkflowExecutor> {
        self.workflows.clone()
    }
}

impl Default for IndiaStack {
    fn default() -> Self {
        Self::new().expect("Failed to create IndiaStack")
    }
}
