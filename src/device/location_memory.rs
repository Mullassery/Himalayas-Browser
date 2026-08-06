use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use tracing::{debug, info};

use super::FusedLocation;

/// Location memory graph - persistent spatial intelligence
/// Stores location history with semantic relationships
pub struct LocationMemoryGraph {
    nodes: Arc<DashMap<String, LocationNode>>,
    edges: Arc<RwLock<Vec<LocationEdge>>>,
    places: Arc<DashMap<String, Place>>,
    trajectories: Arc<RwLock<Vec<Trajectory>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationNode {
    pub id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp_s: u64,
    pub place_name: Option<String>,
    pub semantic_tags: Vec<String>,
    pub visit_count: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
struct LocationEdge {
    from_id: String,
    to_id: String,
    distance_m: f32,
    time_elapsed_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub id: String,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub category: PlaceCategory,
    pub radius_m: f32,
    pub visit_history: Vec<Visit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaceCategory {
    Home,
    Work,
    Frequent,
    Transit,
    POI,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Visit {
    pub timestamp_s: u64,
    pub duration_s: u64,
}

#[derive(Debug, Clone)]
pub struct Trajectory {
    pub start_location: LocationNode,
    pub end_location: LocationNode,
    pub waypoints: Vec<LocationNode>,
    pub distance_m: f32,
    pub duration_s: u64,
    pub speed_average_ms: f32,
}

impl LocationMemoryGraph {
    pub fn new() -> Result<Self> {
        debug!("Initializing Location Memory Graph");
        info!("📍 Location memory graph: persistent spatial intelligence");

        Ok(Self {
            nodes: Arc::new(DashMap::new()),
            edges: Arc::new(RwLock::new(Vec::new())),
            places: Arc::new(DashMap::new()),
            trajectories: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Add location to memory graph
    pub fn add_location(&self, location: FusedLocation) -> Result<String> {
        let id = format!("{}-{}", location.timestamp_s, uuid::Uuid::new_v4());

        let node = LocationNode {
            id: id.clone(),
            latitude: location.latitude,
            longitude: location.longitude,
            timestamp_s: location.timestamp_s,
            place_name: self.identify_place(location.latitude, location.longitude),
            semantic_tags: vec![],
            visit_count: 1,
            confidence: location.confidence,
        };

        self.nodes.insert(id.clone(), node);

        // Link to previous location if exists
        let prev_data = self.nodes.iter().last().map(|ref_multi| {
            (ref_multi.key().clone(), ref_multi.value().clone())
        });

        if let Some((prev_id, prev)) = prev_data {

            let distance = self.calculate_distance(
                location.latitude,
                location.longitude,
                prev.latitude,
                prev.longitude,
            );

            let edge = LocationEdge {
                from_id: prev_id,
                to_id: id.clone(),
                distance_m: distance,
                time_elapsed_s: location.timestamp_s.saturating_sub(prev.timestamp_s),
            };

            self.edges.write().push(edge);
        }

        debug!("Added location node: {}", id);
        Ok(id)
    }

    /// Query nearby locations
    pub fn query_nearby(&self, latitude: f64, longitude: f64, radius_km: f32) -> Result<Vec<LocationNode>> {
        debug!("Querying locations within {:.1}km", radius_km);

        let radius_m = radius_km * 1000.0;
        let nearby: Vec<LocationNode> = self
            .nodes
            .iter()
            .filter(|entry| {
                let dist = self.calculate_distance(
                    latitude,
                    longitude,
                    entry.value().latitude,
                    entry.value().longitude,
                );
                dist <= radius_m
            })
            .map(|entry| entry.value().clone())
            .collect();

        Ok(nearby)
    }

    /// Identify if location is a known place
    fn identify_place(&self, latitude: f64, longitude: f64) -> Option<String> {
        for place_ref in self.places.iter() {
            let place = place_ref.value();
            let dist = self.calculate_distance(latitude, longitude, place.latitude, place.longitude);
            if dist <= place.radius_m {
                return Some(place.name.clone());
            }
        }
        None
    }

    /// Register a known place
    pub fn register_place(
        &self,
        name: String,
        latitude: f64,
        longitude: f64,
        category: PlaceCategory,
        radius_m: f32,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();

        let place = Place {
            id: id.clone(),
            name: name.clone(),
            latitude,
            longitude,
            category,
            radius_m,
            visit_history: Vec::new(),
        };

        self.places.insert(id.clone(), place);
        info!("Registered place: {}", name);
        Ok(id)
    }

    /// Get frequently visited places
    pub fn get_frequent_places(&self, min_visits: u32) -> Vec<Place> {
        self.places
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|place| place.visit_history.len() >= min_visits as usize)
            .collect()
    }

    /// Calculate trajectory between two locations
    pub fn calculate_trajectory(&self, start_id: &str, end_id: &str) -> Result<Option<Trajectory>> {
        debug!("Calculating trajectory from {} to {}", start_id, end_id);

        let start = self.nodes.get(start_id).map(|n| n.clone());
        let end = self.nodes.get(end_id).map(|n| n.clone());

        match (start, end) {
            (Some(start_node), Some(end_node)) => {
                let distance = self.calculate_distance(
                    start_node.latitude,
                    start_node.longitude,
                    end_node.latitude,
                    end_node.longitude,
                );

                let duration = end_node.timestamp_s.saturating_sub(start_node.timestamp_s);
                let speed = if duration > 0 {
                    distance / duration as f32
                } else {
                    0.0
                };

                Ok(Some(Trajectory {
                    start_location: start_node,
                    end_location: end_node,
                    waypoints: vec![],
                    distance_m: distance,
                    duration_s: duration,
                    speed_average_ms: speed,
                }))
            }
            _ => Ok(None),
        }
    }

    /// Haversine distance calculation
    fn calculate_distance(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f32 {
        const EARTH_RADIUS_M: f64 = 6_371_000.0;

        let lat1_rad = lat1.to_radians();
        let lat2_rad = lat2.to_radians();
        let delta_lat = (lat2 - lat1).to_radians();
        let delta_lon = (lon2 - lon1).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        (EARTH_RADIUS_M * c) as f32
    }

    /// Get location statistics
    pub fn get_stats(&self) -> LocationStats {
        LocationStats {
            total_locations: self.nodes.len(),
            total_places: self.places.len(),
            total_trajectories: self.trajectories.read().len(),
            memory_size_nodes: self.nodes.len(),
        }
    }

    /// Get memory graph size
    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    /// Export location history as trajectory summary
    pub fn export_summary(&self) -> LocationSummary {
        let nodes: Vec<_> = self.nodes.iter().map(|n| n.value().clone()).collect();

        let mut total_distance = 0.0;
        let mut total_time = 0u64;

        if nodes.len() > 1 {
            for i in 1..nodes.len() {
                let dist = self.calculate_distance(
                    nodes[i - 1].latitude,
                    nodes[i - 1].longitude,
                    nodes[i].latitude,
                    nodes[i].longitude,
                );
                total_distance += dist;
                total_time += nodes[i].timestamp_s.saturating_sub(nodes[i - 1].timestamp_s);
            }
        }

        LocationSummary {
            total_locations: nodes.len(),
            total_distance_km: total_distance / 1000.0,
            total_time_s: total_time,
            places_visited: self.places.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocationStats {
    pub total_locations: usize,
    pub total_places: usize,
    pub total_trajectories: usize,
    pub memory_size_nodes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationSummary {
    pub total_locations: usize,
    pub total_distance_km: f32,
    pub total_time_s: u64,
    pub places_visited: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_memory_creation() {
        let memory = LocationMemoryGraph::new().unwrap();
        assert_eq!(memory.size(), 0);
    }

    #[test]
    fn test_add_location() {
        let memory = LocationMemoryGraph::new().unwrap();
        let loc = FusedLocation {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_m: 0.0,
            accuracy_m: 5.0,
            confidence: 0.9,
            sensor_fusion_weights: super::super::location_fusion::FusionWeights {
                gnss_weight: 1.0,
                wifi_weight: 0.0,
                ble_weight: 0.0,
                imu_weight: 0.0,
                cellular_weight: 0.0,
            },
            timestamp_s: 1000,
        };

        let id = memory.add_location(loc).unwrap();
        assert_eq!(memory.size(), 1);
        assert!(!id.is_empty());
    }

    #[test]
    fn test_register_place() {
        let memory = LocationMemoryGraph::new().unwrap();
        let id = memory
            .register_place(
                "Home".to_string(),
                37.7749,
                -122.4194,
                PlaceCategory::Home,
                100.0,
            )
            .unwrap();

        assert!(!id.is_empty());
        assert_eq!(memory.get_frequent_places(0).len(), 1);
    }

    #[test]
    fn test_distance_calculation() {
        let memory = LocationMemoryGraph::new().unwrap();
        // Distance between two points in San Francisco
        let dist = memory.calculate_distance(37.7749, -122.4194, 37.7849, -122.4094);
        assert!(dist > 0.0 && dist < 20000.0); // Should be ~10km
    }

    #[test]
    fn test_query_nearby() {
        let memory = LocationMemoryGraph::new().unwrap();

        let loc1 = FusedLocation {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude_m: 0.0,
            accuracy_m: 5.0,
            confidence: 0.9,
            sensor_fusion_weights: super::super::location_fusion::FusionWeights {
                gnss_weight: 1.0,
                wifi_weight: 0.0,
                ble_weight: 0.0,
                imu_weight: 0.0,
                cellular_weight: 0.0,
            },
            timestamp_s: 1000,
        };

        memory.add_location(loc1).unwrap();

        let nearby = memory.query_nearby(37.7749, -122.4194, 1.0).unwrap();
        assert_eq!(nearby.len(), 1);
    }
}
