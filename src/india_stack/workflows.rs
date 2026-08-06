use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, debug};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Initiated,
    DocumentsRetrieved,
    FormFilled,
    ApprovalRequested,
    Approved,
    Signed,
    Submitted,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub status: WorkflowStatus,
    pub description: String,
    pub timestamp: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub workflow_type: String, // "license_renewal", "tax_filing", etc.
    pub user_id: String,
    pub status: WorkflowStatus,
    pub steps: Vec<WorkflowStep>,
    pub result: Option<HashMap<String, String>>,
    pub audit_trail: Vec<String>,
}

impl Workflow {
    pub fn new(workflow_type: String, user_id: String) -> Self {
        Self {
            id: format!("workflow_{}", uuid::Uuid::new_v4()),
            workflow_type,
            user_id,
            status: WorkflowStatus::Initiated,
            steps: vec![],
            result: None,
            audit_trail: vec![],
        }
    }

    pub fn add_step(&mut self, name: String, description: String) {
        let step = WorkflowStep {
            name,
            status: WorkflowStatus::Initiated,
            description,
            timestamp: chrono::Local::now().to_rfc3339(),
            error: None,
        };
        self.steps.push(step);
    }

    pub fn mark_step_complete(&mut self, step_name: &str) -> Result<()> {
        if let Some(step) = self.steps.iter_mut().find(|s| s.name == step_name) {
            step.status = WorkflowStatus::Completed;
            self.audit_trail.push(format!("Completed: {}", step_name));
            Ok(())
        } else {
            anyhow::bail!("Step not found: {}", step_name)
        }
    }

    pub fn mark_step_failed(&mut self, step_name: &str, error: String) -> Result<()> {
        if let Some(step) = self.steps.iter_mut().find(|s| s.name == step_name) {
            step.status = WorkflowStatus::Failed;
            step.error = Some(error.clone());
            self.status = WorkflowStatus::Failed;
            self.audit_trail.push(format!("Failed: {} - {}", step_name, error));
            Ok(())
        } else {
            anyhow::bail!("Step not found: {}", step_name)
        }
    }
}

/// Workflow executor for government workflows
pub struct WorkflowExecutor {
    workflows: std::sync::Arc<dashmap::DashMap<String, Workflow>>,
}

impl WorkflowExecutor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            workflows: std::sync::Arc::new(dashmap::DashMap::new()),
        })
    }

    /// Create a license renewal workflow
    pub async fn create_license_renewal_workflow(&self, user_id: &str) -> Result<Workflow> {
        info!("Creating license renewal workflow for user: {}", user_id);

        let mut workflow = Workflow::new("license_renewal".to_string(), user_id.to_string());

        workflow.add_step(
            "authenticate".to_string(),
            "Authenticate with Aadhaar/DigiLocker".to_string(),
        );
        workflow.add_step(
            "retrieve_documents".to_string(),
            "Retrieve identity and address proof from DigiLocker".to_string(),
        );
        workflow.add_step(
            "validate_eligibility".to_string(),
            "Validate license renewal eligibility".to_string(),
        );
        workflow.add_step(
            "fill_application".to_string(),
            "Fill online application form".to_string(),
        );
        workflow.add_step(
            "request_approval".to_string(),
            "Request user approval for submission".to_string(),
        );
        workflow.add_step(
            "sign_application".to_string(),
            "Sign application with digital signature".to_string(),
        );
        workflow.add_step(
            "submit_application".to_string(),
            "Submit to RTO portal".to_string(),
        );
        workflow.add_step(
            "get_receipt".to_string(),
            "Get submission receipt and tracking number".to_string(),
        );

        let workflow_id = workflow.id.clone();
        self.workflows.insert(workflow_id, workflow.clone());

        Ok(workflow)
    }

    /// Create an income tax filing workflow
    pub async fn create_tax_filing_workflow(&self, user_id: &str) -> Result<Workflow> {
        info!("Creating tax filing workflow for user: {}", user_id);

        let mut workflow = Workflow::new("tax_filing".to_string(), user_id.to_string());

        workflow.add_step(
            "authenticate".to_string(),
            "Authenticate with PAN/Aadhaar".to_string(),
        );
        workflow.add_step(
            "retrieve_documents".to_string(),
            "Retrieve income documents from DigiLocker".to_string(),
        );
        workflow.add_step(
            "calculate_tax".to_string(),
            "Calculate tax liability".to_string(),
        );
        workflow.add_step(
            "fill_itr_form".to_string(),
            "Fill ITR form (auto-populate from documents)".to_string(),
        );
        workflow.add_step(
            "request_approval".to_string(),
            "Request user approval".to_string(),
        );
        workflow.add_step(
            "sign_itr".to_string(),
            "Sign ITR with digital signature".to_string(),
        );
        workflow.add_step(
            "submit_to_income_tax".to_string(),
            "Submit to income tax portal".to_string(),
        );
        workflow.add_step(
            "get_filing_receipt".to_string(),
            "Get ITR filing receipt".to_string(),
        );

        let workflow_id = workflow.id.clone();
        self.workflows.insert(workflow_id, workflow.clone());

        Ok(workflow)
    }

    /// Get workflow by ID
    pub fn get_workflow(&self, workflow_id: &str) -> Option<Workflow> {
        self.workflows.get(workflow_id).map(|w| w.clone())
    }

    /// List all workflows for a user
    pub fn list_user_workflows(&self, user_id: &str) -> Vec<Workflow> {
        self.workflows
            .iter()
            .filter(|entry| entry.value().user_id == user_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Execute workflow step
    pub async fn execute_step(&self, workflow_id: &str, step_name: &str) -> Result<()> {
        debug!("Executing workflow step: {} in workflow: {}", step_name, workflow_id);

        if let Some(mut workflow) = self.workflows.get_mut(workflow_id) {
            workflow.mark_step_complete(step_name)?;
            info!("Step completed: {}", step_name);
            Ok(())
        } else {
            anyhow::bail!("Workflow not found: {}", workflow_id)
        }
    }

    /// Mark workflow as complete
    pub fn complete_workflow(&self, workflow_id: &str, result: HashMap<String, String>) -> Result<()> {
        if let Some(mut workflow) = self.workflows.get_mut(workflow_id) {
            workflow.status = WorkflowStatus::Completed;
            workflow.result = Some(result);
            workflow.audit_trail.push("Workflow completed successfully".to_string());
            info!("Workflow completed: {}", workflow_id);
            Ok(())
        } else {
            anyhow::bail!("Workflow not found: {}", workflow_id)
        }
    }
}

impl Default for WorkflowExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create WorkflowExecutor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let workflow = Workflow::new("license_renewal".to_string(), "user_123".to_string());
        assert_eq!(workflow.workflow_type, "license_renewal");
        assert_eq!(workflow.user_id, "user_123");
    }

    #[test]
    fn test_workflow_steps() {
        let mut workflow = Workflow::new("test".to_string(), "user_123".to_string());
        workflow.add_step("step1".to_string(), "First step".to_string());
        workflow.add_step("step2".to_string(), "Second step".to_string());
        assert_eq!(workflow.steps.len(), 2);
    }

    #[test]
    fn test_mark_step_complete() {
        let mut workflow = Workflow::new("test".to_string(), "user_123".to_string());
        workflow.add_step("step1".to_string(), "First step".to_string());
        assert!(workflow.mark_step_complete("step1").is_ok());
        assert_eq!(workflow.audit_trail.len(), 1);
    }

    #[tokio::test]
    async fn test_workflow_executor_creation() {
        let executor = WorkflowExecutor::new().unwrap();
        assert_eq!(executor.workflows.len(), 0);
    }

    #[tokio::test]
    async fn test_license_renewal_workflow() {
        let executor = WorkflowExecutor::new().unwrap();
        let workflow = executor.create_license_renewal_workflow("user_123").await.unwrap();
        assert_eq!(workflow.workflow_type, "license_renewal");
        assert_eq!(workflow.steps.len(), 8);
    }

    #[tokio::test]
    async fn test_tax_filing_workflow() {
        let executor = WorkflowExecutor::new().unwrap();
        let workflow = executor.create_tax_filing_workflow("user_123").await.unwrap();
        assert_eq!(workflow.workflow_type, "tax_filing");
        assert_eq!(workflow.steps.len(), 8);
    }

    #[test]
    fn test_workflow_audit_trail() {
        let mut workflow = Workflow::new("test".to_string(), "user_123".to_string());
        workflow.add_step("step1".to_string(), "First step".to_string());
        workflow.mark_step_complete("step1").unwrap();
        assert!(!workflow.audit_trail.is_empty());
    }
}
