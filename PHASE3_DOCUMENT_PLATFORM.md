# Phase 3: Document Platform Architecture & Implementation

**Phase**: 3 (Document Intelligence → Rich Document Handling)  
**Timeline**: Jan 2027 - Jun 2027 (26 weeks)  
**Status**: Architecture & Implementation Plan  
**Dependencies**: Phase 2 (Agent framework, OCR foundation)  

---

## Executive Summary

Phase 3 transforms Himalayas from "form automation platform" to **"comprehensive document platform"** competing with Adobe Acrobat, Microsoft Office, and specialized document tools.

**Goal**: Make Himalayas the default choice for any document workflow.

---

## Phase 3 Scope

### 1. PDF Engine (Core Component)

**Rendering**:
```rust
pub struct PdfRenderer {
    pdfium: PdfiumLibrary,
    cache: PageRenderCache,
    gpu_acceleration: bool,
}

pub struct PageRenderCache {
    max_pages: usize,
    cache_size: ByteSize,
    quality: RenderQuality,
}

pub enum RenderQuality {
    Low,      // 72 DPI (battery mode)
    Medium,   // 150 DPI (standard)
    High,     // 300 DPI (print)
}

impl PdfRenderer {
    pub async fn render_page(
        &self,
        doc: &PdfDocument,
        page_num: u32,
        quality: RenderQuality,
    ) -> Result<Image>;
    
    pub async fn render_range(
        &self,
        doc: &PdfDocument,
        start: u32,
        end: u32,
        quality: RenderQuality,
    ) -> Result<Vec<Image>>;
}
```

**Features**:
- Fast rendering (using pdfium-render or mupdf)
- Large document optimization (lazy loading)
- Page thumbnails generation
- Search within documents
- Text selection
- Zoom/navigation
- Multi-page view
- Bookmarks

**Performance Targets**:
- <100ms first page
- <50ms thumbnail generation
- <10MB memory per 100 pages

### 2. Document Editing

**Annotation System**:
```rust
pub enum AnnotationType {
    Highlight { color: Color },
    Underline { color: Color },
    StrikeThrough { color: Color },
    Note { text: String },
    Drawing { path: Vec<Point> },
    FreeformText { text: String },
    Stamp { type: StampType },
}

pub struct Annotation {
    id: String,
    annotation_type: AnnotationType,
    page: u32,
    bounds: Rectangle,
    created: DateTime,
    modified: DateTime,
    author: String,
}

pub struct AnnotationManager {
    annotations: Arc<DashMap<String, Annotation>>,
    undo_stack: VecDeque<AnnotationChange>,
}

impl AnnotationManager {
    pub fn add_annotation(&self, annotation: Annotation) -> Result<String>;
    pub fn remove_annotation(&self, id: &str) -> Result<()>;
    pub fn update_annotation(&self, id: &str, update: AnnotationUpdate) -> Result<()>;
    pub fn get_page_annotations(&self, page: u32) -> Vec<Annotation>;
    pub fn undo(&mut self) -> Result<()>;
    pub fn redo(&mut self) -> Result<()>;
}
```

**Form Filling**:
```rust
pub struct FormField {
    field_name: String,
    field_type: FormFieldType,
    page: u32,
    bounds: Rectangle,
    default_value: Option<String>,
    current_value: Option<String>,
}

pub enum FormFieldType {
    Text,
    Checkbox,
    Radio,
    Dropdown,
    Signature,
    Date,
    Time,
}

pub struct FormManager {
    fields: HashMap<String, FormField>,
    flatten_on_save: bool,
}

impl FormManager {
    pub fn fill_field(&mut self, name: &str, value: String) -> Result<()>;
    pub fn get_form_data(&self) -> HashMap<String, String>;
    pub fn validate_form(&self) -> Result<Vec<ValidationError>>;
    pub fn flatten_form(&self) -> Result<PdfDocument>;
}
```

### 3. AI Document Features

**Summarization**:
```rust
pub struct DocumentSummarizer {
    model: LocalLLM,           // From Phase 2
    chunk_size: usize,
    overlap: usize,
}

impl DocumentSummarizer {
    pub async fn summarize(
        &self,
        doc: &PdfDocument,
        summary_level: SummaryLevel,
    ) -> Result<String>;
    
    pub async fn extract_key_points(
        &self,
        doc: &PdfDocument,
        num_points: usize,
    ) -> Result<Vec<String>>;
    
    pub async fn generate_outline(
        &self,
        doc: &PdfDocument,
    ) -> Result<DocumentOutline>;
}

pub enum SummaryLevel {
    Brief,    // 1-2 sentences
    Medium,   // 1 paragraph
    Detailed, // Full summary
}
```

**Extraction**:
```rust
pub struct DocumentExtractor {
    ocr: OcrEngine,            // From Phase 2
    ner_model: NerModel,
    table_extractor: TableExtractor,
}

impl DocumentExtractor {
    pub async fn extract_text(&self, doc: &PdfDocument) -> Result<String>;
    
    pub async fn extract_tables(&self, doc: &PdfDocument) -> Result<Vec<Table>>;
    
    pub async fn extract_entities(
        &self,
        doc: &PdfDocument,
    ) -> Result<HashMap<String, Vec<String>>>;  // e.g., "PERSON": ["John", "Jane"]
    
    pub async fn extract_by_field(
        &self,
        doc: &PdfDocument,
        field_names: Vec<&str>,
    ) -> Result<HashMap<String, String>>;
}
```

**Comparison**:
```rust
pub struct DocumentComparator {
    extractor: DocumentExtractor,
}

impl DocumentComparator {
    pub async fn compare_documents(
        &self,
        doc1: &PdfDocument,
        doc2: &PdfDocument,
    ) -> Result<ComparisonResult>;
}

pub struct ComparisonResult {
    added_text: Vec<TextSegment>,
    removed_text: Vec<TextSegment>,
    modified_text: Vec<(TextSegment, TextSegment)>,
    added_pages: usize,
    removed_pages: usize,
    similarity_score: f32,
}
```

**Q&A**:
```rust
pub struct DocumentQA {
    model: LocalLLM,
    rag: RagPipeline,
}

impl DocumentQA {
    pub async fn answer_question(
        &self,
        doc: &PdfDocument,
        question: &str,
    ) -> Result<Answer>;
}

pub struct Answer {
    text: String,
    confidence: f32,
    source_pages: Vec<u32>,
    evidence: Vec<String>,
}
```

### 4. Format Support

**DOCX (Microsoft Word)**:
```rust
pub struct DocxHandler {
    zip_processor: ZipArchive,
    xml_parser: XmlParser,
}

impl DocumentHandler for DocxHandler {
    fn load(path: &Path) -> Result<Document>;
    fn save(doc: &Document, path: &Path) -> Result<()>;
    fn convert_to_pdf(doc: &Document) -> Result<PdfDocument>;
}
```

**XLSX (Microsoft Excel)**:
```rust
pub struct XlsxHandler {
    spreadsheet_parser: SpreadsheetParser,
}

impl DocumentHandler for XlsxHandler {
    fn load(path: &Path) -> Result<Spreadsheet>;
    fn extract_tables(&self) -> Result<Vec<Table>>;
    fn perform_calculations(&self) -> Result<()>;
}
```

**PPTX (Microsoft PowerPoint)**:
```rust
pub struct PptxHandler {
    presentation_parser: PresentationParser,
}

impl DocumentHandler for PptxHandler {
    fn load(path: &Path) -> Result<Presentation>;
    fn render_slide(&self, slide_num: u32) -> Result<Image>;
    fn extract_speaker_notes(&self) -> Result<Vec<String>>;
}
```

### 5. Document Management

**Metadata**:
```rust
pub struct DocumentMetadata {
    title: String,
    author: String,
    created: DateTime,
    modified: DateTime,
    pages: u32,
    size: ByteSize,
    language: String,
    encoding: String,
    keywords: Vec<String>,
    properties: HashMap<String, String>,
}
```

**Versioning**:
```rust
pub struct DocumentVersion {
    version_id: String,
    timestamp: DateTime,
    author: String,
    change_summary: String,
    document_hash: String,
}

pub struct VersionManager {
    versions: Vec<DocumentVersion>,
    max_versions: usize,
}

impl VersionManager {
    pub fn save_version(&mut self, doc: &Document, summary: String) -> Result<String>;
    pub fn restore_version(&self, version_id: &str) -> Result<Document>;
    pub fn diff_versions(&self, v1: &str, v2: &str) -> Result<Diff>;
}
```

---

## Phase 3 Module Structure

```
src/document/
├── mod.rs                    # Public API
├── engine/
│   ├── mod.rs
│   ├── pdf_renderer.rs       # PDF rendering
│   ├── format_handlers.rs    # DOCX, XLSX, PPTX
│   └── cache.rs              # Page/rendering cache
├── editing/
│   ├── mod.rs
│   ├── annotations.rs        # Highlighting, notes, drawings
│   ├── forms.rs              # Form filling
│   └── undo_redo.rs          # Edit history
├── ai/
│   ├── mod.rs
│   ├── summarizer.rs         # Document summarization
│   ├── extractor.rs          # Text/table/entity extraction
│   ├── comparator.rs         # Document comparison
│   ├── qa.rs                 # Question answering
│   └── rag.rs                # RAG pipeline (retrieval)
├── management/
│   ├── mod.rs
│   ├── metadata.rs           # Document metadata
│   ├── versioning.rs         # Version management
│   └── search.rs             # Full-text search
└── ui/
    ├── mod.rs
    ├── viewer.rs             # Document viewer API
    ├── editor.rs             # Editor API
    └── toolbar.rs            # UI components
```

---

## Phase 3 Implementation Phases

### Weeks 1-7: PDF Engine & Basic Rendering
**Deliverables**:
- PDF rendering engine
- Page caching
- Thumbnails
- Zoom/navigation
- Text selection
- Basic search

**LOC Target**: 800  
**Tests**: 20

### Weeks 8-14: Annotations & Forms
**Deliverables**:
- Highlighting
- Comments/notes
- Drawings
- Form filling
- Signature support
- Undo/redo

**LOC Target**: 600  
**Tests**: 18

### Weeks 15-21: AI Features
**Deliverables**:
- Summarization
- Key point extraction
- Table extraction
- Entity extraction
- Document Q&A
- Comparison

**LOC Target**: 800  
**Tests**: 25

### Weeks 22-26: Format Support & Polish
**Deliverables**:
- DOCX support
- XLSX support
- PPTX support
- Full-text search
- Metadata management
- Version control

**LOC Target**: 600  
**Tests**: 20

---

## Phase 3 Integration Points

### From Phase 2
- **OCR Engine**: Used for text extraction
- **Agent Framework**: Document processing agents
- **Permission Model**: Access control for documents
- **Storage**: Document persistence

### To Phase 4
- **Device Drivers**: Printing documents
- **Hardware Integration**: Scanner integration
- **Camera**: Document capture
- **GPU Acceleration**: Enhanced rendering

---

## Phase 3 Testing Strategy

```rust
#[cfg(test)]
mod document_tests {
    // PDF Rendering
    #[test]
    fn test_pdf_rendering_basic() { }
    
    #[test]
    fn test_page_caching() { }
    
    #[test]
    fn test_thumbnail_generation() { }
    
    // Annotations
    #[test]
    fn test_add_highlight() { }
    
    #[test]
    fn test_annotation_persistence() { }
    
    // Forms
    #[test]
    fn test_form_field_detection() { }
    
    #[test]
    fn test_form_validation() { }
    
    // AI Features
    #[test]
    fn test_document_summarization() { }
    
    #[test]
    fn test_table_extraction() { }
    
    #[test]
    fn test_document_qa() { }
    
    // Formats
    #[test]
    fn test_docx_loading() { }
    
    #[test]
    fn test_xlsx_parsing() { }
    
    #[test]
    fn test_pptx_rendering() { }
}
```

**Test Coverage Target**: 83 tests, >80% coverage

---

## Phase 3 Success Criteria

### By End of Phase 3
- ✅ PDF rendering working on all devices
- ✅ Annotation system complete
- ✅ AI features (summarization, extraction, Q&A) working
- ✅ DOCX, XLSX, PPTX support
- ✅ Full-text search operational
- ✅ 100% test pass rate
- ✅ Document platform ready for 100K users

### Performance Targets
- PDF rendering: <100ms first page
- AI summarization: <10s for 50-page document
- Search: <500ms on 1,000 pages
- Memory: <500MB per 500-page document

---

## Phase 3 Public API

```rust
pub mod document {
    // Viewer
    pub struct DocumentViewer {
        pub async fn open(&mut self, path: &Path) -> Result<Document>;
        pub async fn render_page(&self, page: u32) -> Result<Image>;
        pub async fn get_thumbnails(&self) -> Result<Vec<Image>>;
        pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>>;
    }
    
    // Editor
    pub struct DocumentEditor {
        pub fn add_annotation(&mut self, ann: Annotation) -> Result<String>;
        pub fn fill_form(&mut self, field: &str, value: String) -> Result<()>;
        pub async fn save(&self, path: &Path) -> Result<()>;
        pub fn undo(&mut self) -> Result<()>;
        pub fn redo(&mut self) -> Result<()>;
    }
    
    // AI
    pub struct DocumentAI {
        pub async fn summarize(&self, doc: &Document) -> Result<String>;
        pub async fn extract_tables(&self, doc: &Document) -> Result<Vec<Table>>;
        pub async fn answer_question(&self, doc: &Document, q: &str) -> Result<Answer>;
    }
    
    // Management
    pub struct DocumentManager {
        pub fn save_version(&mut self, summary: String) -> Result<String>;
        pub fn restore_version(&self, id: &str) -> Result<Document>;
        pub fn list_versions(&self) -> Vec<DocumentVersion>;
    }
}
```

---

## Conclusion

Phase 3 builds on Phase 2's document intelligence foundation to create a **comprehensive document platform** that:

- Renders PDFs beautifully on any device
- Enables rich annotation and editing
- Provides AI-powered insights (summarization, extraction, Q&A)
- Supports Microsoft Office formats
- Manages documents with versioning
- Integrates seamlessly with Phase 2 agents and Phase 4 devices

**Next**: Phase 4 builds on this with device integration (printing, scanning, capture).
