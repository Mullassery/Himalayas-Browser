pub mod renderer;
pub mod annotations;
pub mod forms;
pub mod ai;
pub mod metadata;

pub use renderer::PdfRenderer;
pub use annotations::AnnotationManager;
pub use forms::FormManager;
pub use ai::DocumentAI;
pub use metadata::DocumentMetadata;

use anyhow::Result;
use std::path::Path;

/// Document manager coordinating all document operations
pub struct DocumentManager {
    renderer: PdfRenderer,
    annotations: AnnotationManager,
    forms: FormManager,
    ai: DocumentAI,
}

impl DocumentManager {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            renderer: PdfRenderer::new()?,
            annotations: AnnotationManager::new(),
            forms: FormManager::new(),
            ai: DocumentAI::new().await?,
        })
    }

    pub async fn open_document(&self, path: &Path) -> Result<Document> {
        let metadata = DocumentMetadata::from_file(path)?;
        let pages = self.renderer.get_page_count(path)?;

        Ok(Document {
            path: path.to_path_buf(),
            metadata,
            page_count: pages,
        })
    }

    pub fn renderer(&self) -> &PdfRenderer {
        &self.renderer
    }

    pub fn annotations(&self) -> &AnnotationManager {
        &self.annotations
    }

    pub fn forms(&self) -> &FormManager {
        &self.forms
    }

    pub fn ai(&self) -> &DocumentAI {
        &self.ai
    }
}

/// Represents an open document
#[derive(Debug, Clone)]
pub struct Document {
    pub path: std::path::PathBuf,
    pub metadata: DocumentMetadata,
    pub page_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_manager_creation() {
        let dm = DocumentManager::new().await.unwrap();
        assert!(true); // Manager created successfully
    }
}
