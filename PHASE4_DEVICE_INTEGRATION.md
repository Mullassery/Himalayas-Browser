# Phase 4: Device Integration Platform Architecture & Implementation

**Phase**: 4 (Hardware Integration & Physical World Connection)  
**Timeline**: Apr 2027 - Oct 2027 (26 weeks)  
**Status**: Architecture & Implementation Plan  
**Dependencies**: Phase 2 (Agent framework), Phase 3 (Document platform)  

---

## Executive Summary

Phase 4 transforms Himalayas into a **universal device integration platform**, replacing Windows device management layer by providing native support for printers, scanners, cameras, audio, and hardware devices.

**Goal**: Make Himalayas the default control plane for physical device interaction.

---

## Phase 4 Scope

### 1. Printing Platform

**Architecture**:
```rust
pub struct PrintingEngine {
    backends: HashMap<String, PrintBackend>,
    queue_manager: PrintQueueManager,
    device_discovery: PrinterDiscovery,
}

pub enum PrintBackend {
    Cups,       // Linux
    Winspool,   // Windows
    Pmset,      // macOS
    Ipp,        // Internet Printing Protocol
    AirPrint,   // Apple
    Cloud,      // Google Cloud Print
}

pub struct Printer {
    name: String,
    model: String,
    capabilities: PrinterCapabilities,
    connection_method: ConnectionMethod,
    status: PrinterStatus,
}

pub struct PrinterCapabilities {
    color: bool,
    double_sided: bool,
    paper_sizes: Vec<PaperSize>,
    resolutions: Vec<u32>,
    max_pages_per_minute: u32,
    supports_booklet: bool,
    supports_collate: bool,
}

pub enum PaperSize {
    A4,
    Letter,
    Legal,
    A3,
    A5,
    Tabloid,
}

pub struct PrintJob {
    job_id: String,
    document: Document,
    printer: String,
    settings: PrintSettings,
    status: PrintStatus,
    progress: f32,
}

pub struct PrintSettings {
    paper_size: PaperSize,
    orientation: Orientation,
    color_mode: ColorMode,
    double_sided: bool,
    copies: u32,
    booklet: bool,
    page_range: Option<(u32, u32)>,
    quality: PrintQuality,
}

pub enum PrintQuality {
    Draft,
    Normal,
    High,
    Photo,
}
```

**Implementation**:
```rust
impl PrintingEngine {
    pub async fn discover_printers(&self) -> Result<Vec<Printer>>;
    
    pub async fn get_printer_status(&self, name: &str) -> Result<PrinterStatus>;
    
    pub async fn submit_job(
        &self,
        printer: &str,
        doc: &Document,
        settings: PrintSettings,
    ) -> Result<String>;
    
    pub async fn monitor_job(&self, job_id: &str) -> Result<PrintJobStatus>;
    
    pub async fn cancel_job(&self, job_id: &str) -> Result<()>;
    
    pub fn get_queue(&self, printer: &str) -> Result<Vec<PrintJob>>;
}
```

**Use Cases**:
- Document printing (from Phase 3)
- Report generation (from agents)
- Bulk printing (multi-document)
- Booklet creation
- Label printing
- Custom paper sizes

### 2. Scanning Platform

**Architecture**:
```rust
pub struct ScanningEngine {
    backends: HashMap<String, ScannerBackend>,
    device_discovery: ScannerDiscovery,
    ocr_engine: OcrEngine,
}

pub enum ScannerBackend {
    Twain,      // Windows
    Wia,        // Windows (newer)
    Sane,       // Linux
    Ica,        // macOS
    Usb,        // Direct USB
    Network,    // Network scanner
}

pub struct Scanner {
    name: String,
    model: String,
    capabilities: ScannerCapabilities,
    status: ScannerStatus,
}

pub struct ScannerCapabilities {
    resolutions: Vec<u32>,
    color_modes: Vec<ColorMode>,
    paper_sizes: Vec<PaperSize>,
    auto_document_feeder: bool,
    duplex_scanning: bool,
    max_scan_width_inches: f32,
    max_scan_height_inches: f32,
}

pub struct ScanJob {
    job_id: String,
    scanner: String,
    settings: ScanSettings,
    pages: Vec<ScannedPage>,
    status: ScanStatus,
}

pub struct ScanSettings {
    resolution: u32,        // DPI
    color_mode: ColorMode,
    paper_size: PaperSize,
    duplex: bool,
    detect_edges: bool,
    auto_crop: bool,
    auto_deskew: bool,
    output_format: OutputFormat,
}

pub enum OutputFormat {
    Pdf,
    Jpeg,
    Png,
    Tiff,
}

pub struct ScannedPage {
    image: ImageData,
    resolution: u32,
    text: Option<String>,  // OCR result
}
```

**Implementation**:
```rust
impl ScanningEngine {
    pub async fn discover_scanners(&self) -> Result<Vec<Scanner>>;
    
    pub async fn scan_document(
        &self,
        scanner: &str,
        settings: ScanSettings,
    ) -> Result<String>;  // job_id
    
    pub async fn monitor_scan(&self, job_id: &str) -> Result<ScanProgress>;
    
    pub async fn retrieve_scanned_pages(
        &self,
        job_id: &str,
    ) -> Result<Vec<ScannedPage>>;
    
    pub async fn create_pdf_from_scan(
        &self,
        job_id: &str,
    ) -> Result<PdfDocument>;
    
    pub async fn ocr_scanned_pages(
        &self,
        job_id: &str,
        language: &str,
    ) -> Result<String>;  // Full OCR text
}
```

**Workflow Example**:
```
Scan invoices → Detect document boundaries → OCR → Extract fields
→ Save PDF → Store in database
```

### 3. Camera & Vision Platform

**Architecture**:
```rust
pub struct CameraEngine {
    devices: HashMap<String, CameraDevice>,
    vision_models: VisionModelManager,
}

pub struct CameraDevice {
    name: String,
    device_id: String,
    capabilities: CameraCapabilities,
}

pub struct CameraCapabilities {
    resolutions: Vec<(u32, u32)>,  // width, height
    frame_rates: Vec<u32>,
    pixel_formats: Vec<PixelFormat>,
    has_autofocus: bool,
    has_auto_white_balance: bool,
    has_auto_exposure: bool,
    has_zoom: bool,
    max_zoom: f32,
}

pub enum PixelFormat {
    Mjpeg,
    H264,
    Yuyv,
    Nv12,
}

pub struct VideoCapture {
    device: String,
    resolution: (u32, u32),
    frame_rate: u32,
    pixel_format: PixelFormat,
}

pub struct VisionPipeline {
    detector: ObjectDetector,
    extractor: FeatureExtractor,
    classifier: ImageClassifier,
}
```

**Use Cases**:
```rust
pub async fn use_cases(engine: &CameraEngine) -> Result<()> {
    // Use case 1: Document capture
    let doc_scan = engine.capture_document().await?;
    
    // Use case 2: Object detection
    let objects = engine.detect_objects().await?;
    
    // Use case 3: Face detection
    let faces = engine.detect_faces().await?;
    
    // Use case 4: QR code scanning
    let qr_data = engine.scan_qr_code().await?;
    
    // Use case 5: Real-time video
    let stream = engine.open_video_stream().await?;
    
    Ok(())
}
```

### 4. Audio Platform

**Architecture**:
```rust
pub struct AudioEngine {
    input_devices: HashMap<String, AudioDevice>,
    output_devices: HashMap<String, AudioDevice>,
    speech_recognizer: SpeechRecognizer,
    text_to_speech: TextToSpeech,
}

pub struct AudioDevice {
    name: String,
    device_id: String,
    channels: u32,
    sample_rate: u32,
    bit_depth: u32,
}

pub struct SpeechRecognizer {
    model: WhisperModel,  // OpenAI Whisper
    supported_languages: Vec<String>,
}

pub struct TextToSpeech {
    model: TTSModel,
    voices: Vec<Voice>,
}

pub struct Voice {
    id: String,
    language: String,
    name: String,
    gender: Gender,
}
```

**Implementation**:
```rust
impl AudioEngine {
    pub async fn transcribe_audio(
        &self,
        audio_data: &[u8],
        language: &str,
    ) -> Result<String>;
    
    pub async fn synthesize_speech(
        &self,
        text: &str,
        voice: &Voice,
    ) -> Result<Vec<u8>>;
    
    pub async fn translate_speech(
        &self,
        audio_data: &[u8],
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String>;
    
    pub async fn open_microphone_stream(
        &self,
        device_id: &str,
    ) -> Result<AudioStream>;
    
    pub async fn open_speaker_stream(
        &self,
        device_id: &str,
    ) -> Result<AudioStream>;
}
```

### 5. File System Integration

**Architecture**:
```rust
pub struct FileSystemEngine {
    local_storage: LocalFileSystem,
    cloud_backends: HashMap<String, CloudBackend>,
    indexer: FileIndexer,
}

pub enum CloudBackend {
    GoogleDrive,
    MicrosoftOneDrive,
    Dropbox,
    Aws,
    Custom,
}

pub struct FileIndexer {
    index: FullTextIndex,
    metadata_cache: MetadataCache,
}

pub struct FileMetadata {
    path: PathBuf,
    size: ByteSize,
    modified: DateTime,
    mime_type: String,
    owner: String,
    permissions: FilePermissions,
}
```

**Implementation**:
```rust
impl FileSystemEngine {
    pub async fn open_file(&self, path: &Path) -> Result<FileHandle>;
    
    pub async fn save_document(
        &self,
        doc: &Document,
        path: &Path,
    ) -> Result<()>;
    
    pub async fn search_files(
        &self,
        query: &str,
        locations: Vec<&Path>,
    ) -> Result<Vec<FileMetadata>>;
    
    pub async fn sync_cloud_storage(&self) -> Result<SyncResult>;
    
    pub async fn get_file_versions(
        &self,
        path: &Path,
    ) -> Result<Vec<FileVersion>>;
}
```

### 6. Hardware Device Framework

**USB Devices**:
```rust
pub struct UsbEngine {
    bus: UsbBus,
    devices: HashMap<String, UsbDevice>,
}

pub struct UsbDevice {
    vendor_id: u16,
    product_id: u16,
    name: String,
    capabilities: UsbCapabilities,
}

impl UsbEngine {
    pub async fn discover_devices(&self) -> Result<Vec<UsbDevice>>;
    pub async fn send_command(&self, device: &str, cmd: &[u8]) -> Result<Vec<u8>>;
}
```

**Bluetooth Devices**:
```rust
pub struct BluetoothEngine {
    adapter: BluetoothAdapter,
    devices: HashMap<String, BluetoothDevice>,
}

pub struct BluetoothDevice {
    name: String,
    address: String,
    rssi: i32,
    services: Vec<BluetoothService>,
}

impl BluetoothEngine {
    pub async fn scan_devices(&self) -> Result<Vec<BluetoothDevice>>;
    pub async fn pair_device(&self, address: &str) -> Result<()>;
    pub async fn connect_device(&self, address: &str) -> Result<()>;
}
```

**Smart Cards**:
```rust
pub struct SmartCardEngine {
    readers: HashMap<String, CardReader>,
}

pub struct CardReader {
    name: String,
    card: Option<SmartCard>,
}

pub struct SmartCard {
    atr: Vec<u8>,           // Answer-to-reset
    protocols: Vec<String>,  // T0, T1, etc.
}

impl SmartCardEngine {
    pub async fn list_readers(&self) -> Result<Vec<CardReader>>;
    pub async fn transmit_apdu(&self, reader: &str, apdu: &[u8]) -> Result<Vec<u8>>;
}
```

---

## Phase 4 Module Structure

```
src/devices/
├── mod.rs                      # Public API
├── printing/
│   ├── mod.rs
│   ├── engine.rs               # Print engine
│   ├── backends.rs             # CUPS, Winspool, AirPrint, IPP
│   ├── queue.rs                # Print queue management
│   └── jobs.rs                 # Job tracking
├── scanning/
│   ├── mod.rs
│   ├── engine.rs               # Scan engine
│   ├── backends.rs             # TWAIN, WIA, SANE, ICA
│   ├── document_detection.rs   # Boundary detection
│   └── batch_operations.rs     # Multi-page scanning
├── camera/
│   ├── mod.rs
│   ├── engine.rs               # Camera control
│   ├── capture.rs              # Photo/video capture
│   └── vision.rs               # Object detection, etc.
├── audio/
│   ├── mod.rs
│   ├── engine.rs               # Audio I/O
│   ├── speech.rs               # Speech recognition & synthesis
│   └── stream.rs               # Audio streaming
├── filesystem/
│   ├── mod.rs
│   ├── local.rs                # Local file access
│   ├── cloud.rs                # Cloud storage integration
│   ├── indexer.rs              # Full-text search
│   └── sync.rs                 # Sync management
└── hardware/
    ├── mod.rs
    ├── usb.rs                  # USB devices
    ├── bluetooth.rs            # Bluetooth devices
    ├── smartcard.rs            # Smart card readers
    └── discovery.rs            # Device discovery
```

---

## Phase 4 Implementation Phases

### Weeks 1-6: Printing Platform
**Deliverables**:
- Printer discovery & management
- CUPS backend (Linux)
- IPP support (network printers)
- Print queue management
- Job monitoring
- Cloud print integration

**LOC Target**: 700  
**Tests**: 18

### Weeks 7-12: Scanning Platform
**Deliverables**:
- Scanner discovery
- SANE backend (Linux)
- Document boundary detection
- Batch scanning
- OCR integration
- PDF output

**LOC Target**: 600  
**Tests**: 16

### Weeks 13-18: Camera & Audio
**Deliverables**:
- Camera enumeration
- Photo/video capture
- Object detection
- Speech recognition
- Text-to-speech
- Real-time streams

**LOC Target**: 800  
**Tests**: 20

### Weeks 19-26: Hardware & Polish
**Deliverables**:
- USB device support
- Bluetooth integration
- Smart card readers
- File system integration
- Cloud storage sync
- Full-text search

**LOC Target**: 700  
**Tests**: 18

---

## Phase 4 Device Workflows

### Workflow 1: Scan to PDF

```
User action: "Scan document and save"
  ↓
ScanningEngine.discover_scanners()
  ↓
User selects scanner
  ↓
ScanningEngine.scan_document(settings)
  ↓
Detect document boundaries
  ↓
OCR text extraction
  ↓
Create PDF
  ↓
DocumentManager.save_version()
```

### Workflow 2: Print with AI

```
Agent request: "Print invoice as booklet"
  ↓
DocumentAI.detect_invoice()
  ↓
PrintingEngine.discover_printers()
  ↓
Select capable printer
  ↓
Set booklet mode, duplex
  ↓
Submit print job
  ↓
Monitor completion
  ↓
Audit trail log
```

### Workflow 3: Capture & Process

```
User action: "Take photo of whiteboard"
  ↓
CameraEngine.capture_image()
  ↓
VisionEngine.detect_text()
  ↓
OCR on detected text region
  ↓
Return structured text
```

---

## Phase 4 Integration Points

### From Phase 3
- **Document Rendering**: Sending documents to printer
- **Document AI**: Processing scanned documents
- **Format Support**: Saving scanned documents

### To Phase 5
- **Enterprise Policies**: Device restrictions
- **Permission Management**: Hardware access control
- **Audit Trail**: Device usage logging

### To Phase 6
- **Agent Resource Management**: Device quotas per agent
- **Multi-agent Coordination**: Shared device access
- **Marketplace**: Device integration plugins

---

## Phase 4 Testing Strategy

```rust
#[cfg(test)]
mod device_tests {
    // Printing
    #[test]
    fn test_printer_discovery() { }
    
    #[test]
    fn test_print_job_submission() { }
    
    #[test]
    fn test_print_job_monitoring() { }
    
    // Scanning
    #[test]
    fn test_scanner_discovery() { }
    
    #[test]
    fn test_document_boundary_detection() { }
    
    #[test]
    fn test_batch_scanning() { }
    
    // Camera
    #[test]
    fn test_camera_enumeration() { }
    
    #[test]
    fn test_photo_capture() { }
    
    #[test]
    fn test_object_detection() { }
    
    // Audio
    #[test]
    fn test_microphone_capture() { }
    
    #[test]
    fn test_speech_recognition() { }
    
    #[test]
    fn test_text_to_speech() { }
    
    // Workflows
    #[test]
    fn test_scan_to_pdf_workflow() { }
    
    #[test]
    fn test_print_document_workflow() { }
}
```

**Test Coverage Target**: 72 tests, >80% coverage

---

## Phase 4 Success Criteria

### By End of Phase 4
- ✅ Printing fully operational (5+ printer types)
- ✅ Scanning with OCR working
- ✅ Camera capture & object detection
- ✅ Audio input/output & speech recognition
- ✅ File system integration complete
- ✅ Hardware device support (USB, Bluetooth, smartcard)
- ✅ 100% test pass rate
- ✅ Enterprise customers using devices

### Performance Targets
- Printer discovery: <1s
- Scan document: <10s (8.5x11")
- Photo capture: <100ms
- Speech recognition: <500ms per 10s audio
- OCR on scan: <5s per page

---

## Phase 4 Public API

```rust
pub mod devices {
    // Printing
    pub use printing::{PrintingEngine, Printer, PrintJob};
    
    // Scanning
    pub use scanning::{ScanningEngine, Scanner, ScanJob};
    
    // Camera
    pub use camera::{CameraEngine, CameraDevice};
    
    // Audio
    pub use audio::{AudioEngine, AudioDevice};
    
    // File System
    pub use filesystem::{FileSystemEngine, FileMetadata};
    
    // Hardware
    pub use hardware::{UsbEngine, BluetoothEngine, SmartCardEngine};
}
```

---

## Competitive Advantage

**After Phase 4**, Himalayas replaces:
- **Windows Device Manager** (device discovery & management)
- **Printer vendor software** (native support)
- **Scanner vendor software** (native support)
- **Specialized capture apps** (camera integration)
- **Dictation software** (speech recognition)

**Result**: One unified interface for all physical device interaction.

---

## Conclusion

Phase 4 transforms Himalayas into a **complete device integration platform** by:

- Supporting all major printer types
- Enabling document scanning with OCR
- Integrating cameras for capture & vision
- Adding audio input/output & speech
- Connecting file systems (local + cloud)
- Supporting hardware devices (USB, Bluetooth, smartcard)

**Next**: Phase 5 adds enterprise governance, Phase 6 adds multi-agent coordination and marketplace.
