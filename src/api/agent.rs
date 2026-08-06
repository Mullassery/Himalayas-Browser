use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub agent_id: String,
    pub session_id: String,
    pub action: String,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// AgentAPI trait for agent operations
pub trait AgentAPI: Send + Sync {
    fn agent_id(&self) -> &str;
    fn session_id(&self) -> &str;
}

/// Basic agent implementation
pub struct Agent {
    id: String,
    session_id: String,
}

impl Agent {
    pub fn new(id: String, session_id: String) -> Self {
        Self { id, session_id }
    }

    pub fn execute_request(&self, request: AgentRequest) -> Result<AgentResponse> {
        match request.action.as_str() {
            "navigate" => {
                if let Some(url) = request.parameters.get("url") {
                    if let Some(url_str) = url.as_str() {
                        return Ok(AgentResponse {
                            success: true,
                            data: Some(serde_json::json!({"url": url_str})),
                            error: None,
                        });
                    }
                }
                Ok(AgentResponse {
                    success: false,
                    data: None,
                    error: Some("Missing URL parameter".to_string()),
                })
            }
            "click" => {
                if let Some(element_id) = request.parameters.get("element_id") {
                    if let Some(id_str) = element_id.as_str() {
                        return Ok(AgentResponse {
                            success: true,
                            data: Some(serde_json::json!({"element_id": id_str})),
                            error: None,
                        });
                    }
                }
                Ok(AgentResponse {
                    success: false,
                    data: None,
                    error: Some("Missing element_id parameter".to_string()),
                })
            }
            _ => Ok(AgentResponse {
                success: false,
                data: None,
                error: Some(format!("Unknown action: {}", request.action)),
            }),
        }
    }
}

impl AgentAPI for Agent {
    fn agent_id(&self) -> &str {
        &self.id
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new("agent1".to_string(), "session1".to_string());
        assert_eq!(agent.agent_id(), "agent1");
        assert_eq!(agent.session_id(), "session1");
    }

    #[test]
    fn test_navigate_request() {
        let agent = Agent::new("agent1".to_string(), "session1".to_string());
        let mut params = std::collections::HashMap::new();
        params.insert("url".to_string(), serde_json::json!("https://example.com"));

        let request = AgentRequest {
            agent_id: "agent1".to_string(),
            session_id: "session1".to_string(),
            action: "navigate".to_string(),
            parameters: params,
        };

        let response = agent.execute_request(request).unwrap();
        assert!(response.success);
    }

    #[test]
    fn test_click_request() {
        let agent = Agent::new("agent1".to_string(), "session1".to_string());
        let mut params = std::collections::HashMap::new();
        params.insert("element_id".to_string(), serde_json::json!("button1"));

        let request = AgentRequest {
            agent_id: "agent1".to_string(),
            session_id: "session1".to_string(),
            action: "click".to_string(),
            parameters: params,
        };

        let response = agent.execute_request(request).unwrap();
        assert!(response.success);
    }
}
