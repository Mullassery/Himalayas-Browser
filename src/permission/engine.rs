use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub level: PermissionLevel,
    pub resource: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub permission: Permission,
    pub granted_at: SystemTime,
    pub expires_at: SystemTime,
    pub agent_id: String,
    pub session_id: String,
}

impl PermissionGrant {
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    pub fn time_remaining_secs(&self) -> u64 {
        self.expires_at
            .duration_since(SystemTime::now())
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

pub struct PermissionEngine {
    grants: Arc<DashMap<String, PermissionGrant>>,
}

impl PermissionEngine {
    pub fn new() -> Self {
        Self {
            grants: Arc::new(DashMap::new()),
        }
    }

    pub fn request_permission(
        &self,
        permission: Permission,
        agent_id: &str,
        session_id: &str,
    ) -> Result<PermissionGrant> {
        let duration = match permission.level {
            PermissionLevel::Low => Duration::from_secs(3600 * 24), // 24 hours
            PermissionLevel::Medium => Duration::from_secs(3600 * 24), // 24 hours
            PermissionLevel::High => Duration::from_secs(3600 * 2), // 2 hours
            PermissionLevel::Critical => Duration::from_secs(1800), // 30 minutes
        };

        let now = SystemTime::now();
        let grant = PermissionGrant {
            permission: permission.clone(),
            granted_at: now,
            expires_at: now + duration,
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
        };

        let grant_id = format!(
            "{}:{}:{}",
            agent_id, session_id, permission.id
        );
        self.grants.insert(grant_id, grant.clone());

        Ok(grant)
    }

    pub fn check_permission(
        &self,
        permission: &Permission,
        agent_id: &str,
        session_id: &str,
    ) -> Result<bool> {
        let grant_id = format!("{}:{}:{}", agent_id, session_id, permission.id);

        if let Some(grant) = self.grants.get(&grant_id) {
            if grant.is_expired() {
                // Remove expired grant
                drop(grant);
                self.grants.remove(&grant_id);
                return Ok(false);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn revoke_permission(
        &self,
        permission_id: &str,
        agent_id: &str,
        session_id: &str,
    ) -> Result<()> {
        let grant_id = format!("{}:{}:{}", agent_id, session_id, permission_id);
        self.grants.remove(&grant_id);
        Ok(())
    }

    pub fn revoke_all_for_session(&self, session_id: &str) -> Result<()> {
        let to_remove: Vec<String> = self
            .grants
            .iter()
            .filter(|entry| entry.value().session_id == session_id)
            .map(|entry| entry.key().clone())
            .collect();

        for key in to_remove {
            self.grants.remove(&key);
        }

        Ok(())
    }

    pub fn list_active_permissions(
        &self,
        agent_id: &str,
        session_id: &str,
    ) -> Result<Vec<PermissionGrant>> {
        let grants: Vec<PermissionGrant> = self
            .grants
            .iter()
            .filter(|entry| {
                let g = entry.value();
                g.agent_id == agent_id && g.session_id == session_id && !g.is_expired()
            })
            .map(|entry| entry.value().clone())
            .collect();

        Ok(grants)
    }

    pub fn cleanup_expired(&self) -> Result<usize> {
        let mut removed = 0;
        let to_remove: Vec<String> = self
            .grants
            .iter()
            .filter(|entry| entry.value().is_expired())
            .map(|entry| entry.key().clone())
            .collect();

        for key in to_remove {
            if self.grants.remove(&key).is_some() {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

impl Default for PermissionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let perm = Permission {
            id: "navigate".to_string(),
            level: PermissionLevel::Medium,
            resource: "browser".to_string(),
            action: "navigate".to_string(),
        };
        assert_eq!(perm.level, PermissionLevel::Medium);
    }

    #[test]
    fn test_engine_creation() {
        let engine = PermissionEngine::new();
        assert_eq!(engine.grants.len(), 0);
    }

    #[test]
    fn test_request_permission() {
        let engine = PermissionEngine::new();
        let perm = Permission {
            id: "navigate".to_string(),
            level: PermissionLevel::Medium,
            resource: "browser".to_string(),
            action: "navigate".to_string(),
        };

        let grant = engine
            .request_permission(perm, "agent1", "session1")
            .unwrap();
        assert!(!grant.is_expired());
    }

    #[test]
    fn test_check_permission() {
        let engine = PermissionEngine::new();
        let perm = Permission {
            id: "navigate".to_string(),
            level: PermissionLevel::Medium,
            resource: "browser".to_string(),
            action: "navigate".to_string(),
        };

        engine
            .request_permission(perm.clone(), "agent1", "session1")
            .unwrap();
        assert!(engine.check_permission(&perm, "agent1", "session1").unwrap());
    }

    #[test]
    fn test_revoke_permission() {
        let engine = PermissionEngine::new();
        let perm = Permission {
            id: "navigate".to_string(),
            level: PermissionLevel::Medium,
            resource: "browser".to_string(),
            action: "navigate".to_string(),
        };

        engine
            .request_permission(perm.clone(), "agent1", "session1")
            .unwrap();
        engine
            .revoke_permission(&perm.id, "agent1", "session1")
            .unwrap();
        assert!(!engine.check_permission(&perm, "agent1", "session1").unwrap());
    }

    #[test]
    fn test_list_permissions() {
        let engine = PermissionEngine::new();
        let perm = Permission {
            id: "navigate".to_string(),
            level: PermissionLevel::Medium,
            resource: "browser".to_string(),
            action: "navigate".to_string(),
        };

        engine
            .request_permission(perm, "agent1", "session1")
            .unwrap();

        let perms = engine.list_active_permissions("agent1", "session1").unwrap();
        assert_eq!(perms.len(), 1);
    }
}
