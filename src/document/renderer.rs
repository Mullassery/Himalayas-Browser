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

/// PDF Renderer
pub struct PdfRenderer {
    cache: PageRenderCache,
    default_quality: RenderQuality,
}

impl PdfRenderer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cache: PageRenderCache::new(50, 500 * 1024 * 1024), // 50 pages, 500MB
            default_quality: RenderQuality::Medium,
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

        // MVP: Return placeholder rendered page
        // In production: use pdfium-render to actually render
        let dpi = quality.dpi();
        let page = RenderedPage {
            page_num,
            width: (8.5 * dpi as f32 / 72.0) as u32, // US Letter width
            height: (11.0 * dpi as f32 / 72.0) as u32, // US Letter height
            dpi,
            data: vec![255; (8.5 * 11.0 * dpi as f32 * dpi as f32 / 5184.0) as usize], // White page
        };

        self.cache.put(page.clone(), quality);
        info!("Page {} rendered at {}DPI", page_num, dpi);

        Ok(page)
    }

    pub async fn render_range(
        &self,
        path: &Path,
        start: u32,
        end: u32,
        quality: RenderQuality,
    ) -> Result<Vec<RenderedPage>> {
        let mut pages = Vec::new();
        for page_num in start..=end {
            pages.push(self.render_page(path, page_num, quality).await?);
        }
        Ok(pages)
    }

    pub async fn generate_thumbnail(&self, path: &Path, page_num: u32) -> Result<Vec<u8>> {
        debug!("Generating thumbnail for page {}", page_num);

        let page = self.render_page(path, page_num, RenderQuality::Low).await?;

        // MVP: Return scaled down version
        // In production: use proper image scaling
        let thumbnail_size = 150;
        let scale = thumbnail_size as f32 / page.width.max(page.height) as f32;
        let scaled_width = (page.width as f32 * scale) as u32;
        let scaled_height = (page.height as f32 * scale) as u32;

        Ok(vec![200; (scaled_width * scaled_height * 4) as usize])
    }

    pub fn get_page_count(&self, path: &Path) -> Result<u32> {
        debug!("Getting page count for: {}", path.display());

        // MVP: Return placeholder
        // In production: use pdfium to get actual page count
        Ok(10)
    }

    pub fn set_default_quality(&mut self, quality: RenderQuality) {
        self.default_quality = quality;
    }

    pub fn clear_cache(&self) {
        self.cache.clear();
        info!("Render cache cleared");
    }
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
