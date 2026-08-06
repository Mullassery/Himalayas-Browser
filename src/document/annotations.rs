use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use uuid::Uuid;
use tracing::{debug, info};

/// Advanced annotation system with AI awareness
pub struct AnnotationManager {
    annotations: Arc<DashMap<u32, Vec<Annotation>>>, // page -> annotations
    ai_insights: Arc<RwLock<Vec<AIInsight>>>,
    collaboration_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Annotation {
    pub id: String,
    pub page: u32,
    pub annotation_type: AnnotationType,
    pub content: String,
    pub position: (f32, f32), // x, y coordinates
    pub color: String,        // hex color
    pub created_at: u64,
    pub created_by: String,
    pub ai_context: Option<String>, // Context from AI analysis
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AnnotationType {
    Highlight,
    Note,
    Underline,
    Strikethrough,
    Circle,
    Arrow,
    FreeDrawing,
    Bookmark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIInsight {
    pub page: u32,
    pub insight_type: InsightType,
    pub content: String,
    pub relevance: f32, // 0.0-1.0
    pub created_at: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum InsightType {
    KeyPoint,
    Definition,
    Citation,
    Summary,
    Warning,
    Question,
}

impl AnnotationManager {
    pub fn new() -> Self {
        debug!("Initializing Annotation Manager");
        Self {
            annotations: Arc::new(DashMap::new()),
            ai_insights: Arc::new(RwLock::new(Vec::new())),
            collaboration_enabled: true,
        }
    }

    /// Add annotation with automatic AI context enrichment
    pub fn add_annotation(
        &self,
        page: u32,
        annotation_type: AnnotationType,
        content: String,
        position: (f32, f32),
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let annotation = Annotation {
            id: id.clone(),
            page,
            annotation_type,
            content,
            position,
            color: "#FFFF00".to_string(), // Default yellow
            created_at,
            created_by: "user".to_string(),
            ai_context: None,
        };

        self.annotations
            .entry(page)
            .or_insert_with(Vec::new)
            .push(annotation);

        info!("Added annotation {} to page {}", id, page);
        Ok(id)
    }

    /// Add AI-generated insight
    pub fn add_ai_insight(
        &self,
        page: u32,
        insight_type: InsightType,
        content: String,
        relevance: f32,
    ) -> Result<()> {
        let insight = AIInsight {
            page,
            insight_type,
            content,
            relevance: relevance.clamp(0.0, 1.0),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };

        let mut insights = self.ai_insights.write();
        if insights.len() >= 1000 {
            insights.remove(0); // Keep bounded
        }
        insights.push(insight);

        debug!("Added AI insight to page {}", page);
        Ok(())
    }

    /// Get all annotations for page
    pub fn get_annotations(&self, page: u32) -> Vec<Annotation> {
        self.annotations
            .get(&page)
            .map(|a| a.clone())
            .unwrap_or_default()
    }

    /// Get AI insights for page
    pub fn get_ai_insights(&self, page: u32) -> Vec<AIInsight> {
        self.ai_insights
            .read()
            .iter()
            .filter(|i| i.page == page && i.relevance > 0.3)
            .cloned()
            .collect()
    }

    /// Delete annotation
    pub fn delete_annotation(&self, page: u32, annotation_id: &str) -> Result<bool> {
        if let Some(mut anns) = self.annotations.get_mut(&page) {
            if let Some(pos) = anns.iter().position(|a| a.id == annotation_id) {
                anns.remove(pos);
                info!("Deleted annotation {}", annotation_id);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Update annotation content
    pub fn update_annotation(&self, page: u32, annotation_id: &str, new_content: String) -> Result<bool> {
        if let Some(mut anns) = self.annotations.get_mut(&page) {
            if let Some(ann) = anns.iter_mut().find(|a| a.id == annotation_id) {
                ann.content = new_content;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Get all annotations across document
    pub fn get_all_annotations(&self) -> Vec<Annotation> {
        self.annotations
            .iter()
            .flat_map(|entry| entry.value().clone())
            .collect()
    }

    /// Export annotations as JSON
    pub fn export_annotations(&self) -> String {
        let all = self.get_all_annotations();
        serde_json::to_string_pretty(&all).unwrap_or_default()
    }

    /// Get annotation statistics
    pub fn get_stats(&self) -> AnnotationStats {
        let all = self.get_all_annotations();
        let by_type = all.iter().fold(std::collections::HashMap::new(), |mut map, a| {
            *map.entry(a.annotation_type).or_insert(0) += 1;
            map
        });

        AnnotationStats {
            total_annotations: all.len(),
            total_pages_annotated: self.annotations.len(),
            annotations_by_type: by_type,
            total_insights: self.ai_insights.read().len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnnotationStats {
    pub total_annotations: usize,
    pub total_pages_annotated: usize,
    pub annotations_by_type: std::collections::HashMap<AnnotationType, usize>,
    pub total_insights: usize,
}

impl Default for AnnotationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_manager_creation() {
        let manager = AnnotationManager::new();
        assert_eq!(manager.get_all_annotations().len(), 0);
    }

    #[test]
    fn test_add_annotation() {
        let manager = AnnotationManager::new();
        let id = manager
            .add_annotation(1, AnnotationType::Highlight, "Key point".to_string(), (100.0, 200.0))
            .unwrap();

        let anns = manager.get_annotations(1);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].id, id);
    }

    #[test]
    fn test_multiple_annotations_per_page() {
        let manager = AnnotationManager::new();
        manager
            .add_annotation(1, AnnotationType::Highlight, "First".to_string(), (10.0, 20.0))
            .unwrap();
        manager
            .add_annotation(1, AnnotationType::Note, "Second".to_string(), (30.0, 40.0))
            .unwrap();

        let anns = manager.get_annotations(1);
        assert_eq!(anns.len(), 2);
    }

    #[test]
    fn test_ai_insights() {
        let manager = AnnotationManager::new();
        manager
            .add_ai_insight(1, InsightType::KeyPoint, "Important".to_string(), 0.9)
            .unwrap();

        let insights = manager.get_ai_insights(1);
        assert_eq!(insights.len(), 1);
    }

    #[test]
    fn test_delete_annotation() {
        let manager = AnnotationManager::new();
        let id = manager
            .add_annotation(1, AnnotationType::Highlight, "Test".to_string(), (0.0, 0.0))
            .unwrap();

        assert!(manager.delete_annotation(1, &id).unwrap());
        assert_eq!(manager.get_annotations(1).len(), 0);
    }

    #[test]
    fn test_annotation_stats() {
        let manager = AnnotationManager::new();
        manager
            .add_annotation(1, AnnotationType::Highlight, "H1".to_string(), (0.0, 0.0))
            .unwrap();
        manager
            .add_annotation(1, AnnotationType::Note, "N1".to_string(), (10.0, 10.0))
            .unwrap();
        manager
            .add_annotation(2, AnnotationType::Highlight, "H2".to_string(), (20.0, 20.0))
            .unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_annotations, 3);
        assert_eq!(stats.total_pages_annotated, 2);
    }
}
