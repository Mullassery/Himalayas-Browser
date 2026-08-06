use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub agent_id: String,
    pub session_id: String,
    pub permission_id: String,
    pub reason: String,
    pub requested_at: SystemTime,
    pub approved: bool,
    pub approval_reason: Option<String>,
}

impl PermissionRequest {
    pub fn new(
        agent_id: String,
        session_id: String,
        permission_id: String,
        reason: String,
    ) -> Self {
        Self {
            id: format!("req_{}_{}", agent_id, uuid::Uuid::new_v4()),
            agent_id,
            session_id,
            permission_id,
            reason,
            requested_at: SystemTime::now(),
            approved: false,
            approval_reason: None,
        }
    }

    pub fn approve(&mut self, reason: Option<String>) {
        self.approved = true;
        self.approval_reason = reason;
    }

    pub fn deny(&mut self, reason: Option<String>) {
        self.approved = false;
        self.approval_reason = reason;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let req = PermissionRequest::new(
            "agent1".to_string(),
            "session1".to_string(),
            "navigate".to_string(),
            "User requested navigation".to_string(),
        );
        assert_eq!(req.agent_id, "agent1");
        assert_eq!(req.session_id, "session1");
        assert!(!req.approved);
    }

    #[test]
    fn test_approve_request() {
        let mut req = PermissionRequest::new(
            "agent1".to_string(),
            "session1".to_string(),
            "navigate".to_string(),
            "User requested navigation".to_string(),
        );
        req.approve(Some("User approved".to_string()));
        assert!(req.approved);
        assert_eq!(req.approval_reason, Some("User approved".to_string()));
    }

    #[test]
    fn test_deny_request() {
        let mut req = PermissionRequest::new(
            "agent1".to_string(),
            "session1".to_string(),
            "navigate".to_string(),
            "User requested navigation".to_string(),
        );
        req.deny(Some("User denied".to_string()));
        assert!(!req.approved);
        assert_eq!(req.approval_reason, Some("User denied".to_string()));
    }
}
