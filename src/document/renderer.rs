use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use dashmap::DashMap;
use tracing::{debug, info};

/// Render quality for PDF pages
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RenderQuality {
    Low,    // 72 DPI (battery mode)
    Medium, // 150 DPI (standard)
    High,   // 300 DPI (print)
}

impl RenderQuality {
    pub fn dpi(&self) -> u32 {
        match self {
            RenderQuality::Low => 72,
            RenderQuality::Medium => 150,
            RenderQuality::High => 300,
        }
    }
}

/// Rendered page image
#[derive(Debug, Clone)]
pub struct RenderedPage {
    pub page_num: u32,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub data: Vec<u8>,  // RGBA pixel data
}

/// Page cache for efficient rendering
pub struct PageRenderCache {
    cache: Arc<DashMap<(u32, RenderQuality), RenderedPage>>,
    max_pages: usize,
    max_memory_bytes: usize,
}

impl PageRenderCache {
    pub fn new(max_pages: usize, max_memory_bytes: usize) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_pages,
            max_memory_bytes,
        }
    }

    pub fn get(&self, page_num: u32, quality: RenderQuality) -> Option<RenderedPage> {
        self.cache.get(&(page_num, quality)).map(|r| r.clone())
    }

    pub fn put(&self, page: RenderedPage, quality: RenderQuality) {
        if self.cache.len() >= self.max_pages {
            // Simple eviction: remove first entry
            if let Some(entry) = self.cache.iter().next() {
                let key = entry.key().clone();
                drop(entry);
                self.cache.remove(&key);
            }
        }

        self.cache.insert((page.page_num, quality), page);
    }

    pub fn clear(&self) {
        self.cache.clear();
    }

    pub fn memory_usage(&self) -> usize {
        self.cache
            .iter()
            .map(|r| r.value().data.len())
            .sum()
    }
}

/// PDF Renderer with caching and quality management
pub struct PdfRenderer {
    cache: PageRenderCache,
    default_quality: RenderQuality,
    document_cache: Arc<DashMap<String, DocumentInfo>>,
}

#[derive(Debug, Clone)]
struct DocumentInfo {
    path: String,
    page_count: u32,
    width: u32,
    height: u32,
    last_accessed: u64,
}

impl PdfRenderer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cache: PageRenderCache::new(50, 500 * 1024 * 1024), // 50 pages, 500MB
            default_quality: RenderQuality::Medium,
            document_cache: Arc::new(DashMap::new()),
        })
    }

    pub async fn render_page(
        &self,
        path: &Path,
        page_num: u32,
        quality: RenderQuality,
    ) -> Result<RenderedPage> {
        debug!("Rendering page {} at quality {:?}", page_num, quality);

        // Check cache first
        if let Some(cached) = self.cache.get(page_num, quality) {
            debug!("Cache hit for page {}", page_num);
            return Ok(cached);
        }

        // Get document info or create new
        let path_str = path.to_string_lossy().to_string();
        let doc_info = if let Some(info) = self.document_cache.get(&path_str) {
            info.clone()
        } else {
            // Analyze document (simulated)
            let info = DocumentInfo {
                path: path_str.clone(),
                page_count: 10, // In production: actual PDF parsing
                width: 612,     // US Letter width in points
                height: 792,    // US Letter height in points
                last_accessed: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            };
            self.document_cache.insert(path_str, info.clone());
            info
        };

        // Render page
        let dpi = quality.dpi();
        let width = (doc_info.width as f32 * dpi as f32 / 72.0) as u32;
        let height = (doc_info.height as f32 * dpi as f32 / 72.0) as u32;

        // In production: use actual PDF rendering library
        // For MVP: generate realistic placeholder with page number visible
        let mut data = vec![255; (width as usize * height as usize * 4) as usize];

        // Add page number text (simple: just vary shading)
        if page_num % 2 == 0 {
            for i in 0..data.len().min(1000) {
                data[i] = 240; // Slightly gray pages
            }
        }

        let page = RenderedPage {
            page_num,
            width,
            height,
            dpi,
            data,
        };

        self.cache.put(page.clone(), quality);
        info!("Page {} rendered at {}DPI ({}x{})", page_num, dpi, width, height);

        Ok(page)
    }

    pub async fn render_range(
        &self,
        path: &Path,
        start: u32,
        end: u32,
        quality: RenderQuality,
    ) -> Result<Vec<RenderedPage>> {
        debug!("Rendering pages {}-{} at quality {:?}", start, end, quality);
        let mut pages = Vec::new();
        for page_num in start..=end {
            pages.push(self.render_page(path, page_num, quality).await?);
        }
        info!("Rendered {} pages", pages.len());
        Ok(pages)
    }

    pub async fn generate_thumbnail(&self, path: &Path, page_num: u32) -> Result<Vec<u8>> {
        debug!("Generating thumbnail for page {}", page_num);

        let page = self.render_page(path, page_num, RenderQuality::Low).await?;

        // Scale down to thumbnail size (150x200 typical)
        let thumbnail_width = 150;
        let aspect_ratio = page.height as f32 / page.width as f32;
        let thumbnail_height = (thumbnail_width as f32 * aspect_ratio) as u32;

        // Generate thumbnail (RGBA format)
        let thumbnail_data = vec![200; (thumbnail_width * thumbnail_height * 4) as usize];

        info!("Generated thumbnail {}x{} for page {}", thumbnail_width, thumbnail_height, page_num);
        Ok(thumbnail_data)
    }

    pub fn get_page_count(&self, path: &Path) -> Result<u32> {
        debug!("Getting page count for: {}", path.display());

        let path_str = path.to_string_lossy().to_string();
        if let Some(info) = self.document_cache.get(&path_str) {
            Ok(info.page_count)
        } else {
            // In production: actual PDF parsing
            Ok(10)
        }
    }

    pub fn get_page_dimensions(&self, path: &Path, page_num: u32, quality: RenderQuality) -> Result<(u32, u32)> {
        let dpi = quality.dpi();
        let base_width = 612u32;  // US Letter
        let base_height = 792u32;

        let width = (base_width as f32 * dpi as f32 / 72.0) as u32;
        let height = (base_height as f32 * dpi as f32 / 72.0) as u32;

        Ok((width, height))
    }

    pub fn set_default_quality(&mut self, quality: RenderQuality) {
        self.default_quality = quality;
    }

    pub fn clear_cache(&self) {
        self.cache.clear();
        info!("Render cache cleared");
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            cached_pages: self.cache.cache.len(),
            memory_usage_bytes: self.cache.memory_usage(),
            cached_documents: self.document_cache.len(),
        }
    }

    pub fn preload_pages(&self, path: &Path, start: u32, end: u32, quality: RenderQuality) -> Result<()> {
        debug!("Preloading pages {}-{}", start, end);
        // In production: async preload in background
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub cached_pages: usize,
    pub memory_usage_bytes: usize,
    pub cached_documents: usize,
}

impl Default for PdfRenderer {
    fn default() -> Self {
        Self::new().expect("Failed to create PDF renderer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_quality_dpi() {
        assert_eq!(RenderQuality::Low.dpi(), 72);
        assert_eq!(RenderQuality::Medium.dpi(), 150);
        assert_eq!(RenderQuality::High.dpi(), 300);
    }

    #[test]
    fn test_page_cache() {
        let cache = PageRenderCache::new(10, 10 * 1024 * 1024);

        let page = RenderedPage {
            page_num: 1,
            width: 612,
            height: 792,
            dpi: 150,
            data: vec![255; 1000],
        };

        cache.put(page.clone(), RenderQuality::Medium);

        let retrieved = cache.get(1, RenderQuality::Medium);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().page_num, 1);
    }

    #[tokio::test]
    async fn test_pdf_renderer_creation() {
        let renderer = PdfRenderer::new().unwrap();
        assert_eq!(renderer.default_quality, RenderQuality::Medium);
    }
}
