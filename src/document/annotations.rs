use serde::{Deserialize, Serialize};
use std::sync::Arc;
use dashmap::DashMap;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub page: u32,
    pub annotation_type: String,
    pub created: chrono::DateTime<chrono::Local>,
}

pub struct AnnotationManager {
    annotations: Arc<DashMap<String, Annotation>>,
}

impl AnnotationManager {
    pub fn new() -> Self {
        Self {
            annotations: Arc::new(DashMap::new()),
        }
    }

    pub fn add_annotation(&self, annotation: Annotation) -> Result<String> {
        let id = annotation.id.clone();
        self.annotations.insert(id.clone(), annotation);
        Ok(id)
    }

    pub fn get_annotations(&self, page: u32) -> Vec<Annotation> {
        self.annotations
            .iter()
            .filter(|a| a.value().page == page)
            .map(|a| a.value().clone())
            .collect()
    }
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
        assert_eq!(manager.annotations.len(), 0);
    }
}
