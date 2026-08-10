use anyhow::Result;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info, debug};

use crate::api::agent::{AgentRequest, AgentResponse};
use crate::api::AgentContext;
use crate::browser::Browser;
use crate::health::HealthMonitor;
use crate::intelligence::device_detection::DeviceTier;
use crate::metrics::MetricsCollector;

pub struct HealthServer {
    health_monitor: Arc<HealthMonitor>,
    metrics_collector: Arc<MetricsCollector>,
    ui_enabled: bool,
    device_tier: DeviceTier,
    /// Backs `POST /agent` — see `dispatch_agent_action`. The headless
    /// runtime an external agent process actually invokes Himalayas
    /// through, as opposed to `himalayas-desktop`'s in-process native tabs
    /// (a separate `Browser` instance entirely; the two don't share state).
    browser: Arc<Browser>,
    /// One persistent `AgentContext` per `session_id`, reused across
    /// requests — `AgentContext` holds `current_page` state (`query`/
    /// `click`/`get_text`/`submit_form` all operate on "whatever the last
    /// `navigate()` loaded"), so a fresh one per HTTP request would forget
    /// that between calls. Created lazily on first use of a `session_id`.
    agent_contexts: dashmap::DashMap<String, Arc<AgentContext>>,
}

impl HealthServer {
    pub fn new(
        health_monitor: Arc<HealthMonitor>,
        metrics_collector: Arc<MetricsCollector>,
        ui_enabled: bool,
        device_tier: DeviceTier,
        browser: Arc<Browser>,
    ) -> Self {
        Self {
            health_monitor,
            metrics_collector,
            ui_enabled,
            device_tier,
            browser,
            agent_contexts: dashmap::DashMap::new(),
        }
    }

    async fn handle_request(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        let path = req.uri().path().to_string();
        debug!("Request: {} {}", req.method(), path);

        if req.method() == Method::POST && path == "/agent" {
            return self.agent(req).await;
        }

        match path.as_str() {
            "/health" => self.health_check().await,
            "/healthz" => self.kubernetes_probe().await,
            "/stats" => self.stats().await,
            "/metrics" => self.prometheus_metrics().await,
            "/ready" => self.readiness_probe().await,
            "/device" => self.device_info().await,
            "/" => self.root().await,
            #[cfg(feature = "desktop_ui")]
            "/app" | "/app/style.css" | "/app/app.js" | "/app/vendor/vue.global.prod.js" => {
                self.ui_asset(&path).await
            }
            _ => self.not_found().await,
        }
    }

    /// `POST /agent` — the HTTP surface an external agent process actually
    /// invokes Himalayas through, headless (no GUI involved at any point).
    /// Body is an `AgentRequest` JSON object; response is `AgentResponse`.
    /// Malformed JSON gets a 400 with an `AgentResponse{success:false,...}`
    /// body rather than a bare HTTP error, so callers always get the same
    /// response shape back regardless of what went wrong.
    async fn agent(&self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        let bytes = match hyper::body::to_bytes(req.into_body()).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return Self::json_response(
                    StatusCode::BAD_REQUEST,
                    &AgentResponse { success: false, data: None, error: Some("failed to read request body".to_string()) },
                );
            }
        };

        let request: AgentRequest = match serde_json::from_slice(&bytes) {
            Ok(request) => request,
            Err(e) => {
                return Self::json_response(
                    StatusCode::BAD_REQUEST,
                    &AgentResponse { success: false, data: None, error: Some(format!("invalid AgentRequest JSON: {e}")) },
                );
            }
        };

        let response = self.dispatch_agent_action(request).await;
        let status = if response.success { StatusCode::OK } else { StatusCode::UNPROCESSABLE_ENTITY };
        Self::json_response(status, &response)
    }

    /// Look up (or lazily create) the persistent `AgentContext` for
    /// `session_id`, then run `request.action` against it. Every action an
    /// `AgentContext` exposes is reachable here — see that type for what
    /// each one actually does (all real, not stubs — `query`/`click`/
    /// `get_text`/`submit_form` do genuine CSS-selector matching, link/form
    /// navigation, and live DOM reads).
    async fn dispatch_agent_action(&self, request: AgentRequest) -> AgentResponse {
        let ctx = self
            .agent_contexts
            .entry(request.session_id.clone())
            .or_insert_with(|| {
                let session = self
                    .browser
                    .get_session(&request.session_id)
                    .unwrap_or_else(|| self.browser.create_session(request.session_id.clone()).expect("Session::new is infallible in practice"));
                Arc::new(AgentContext::new(session, self.browser.clone()))
            })
            .clone();

        let param_str = |name: &str| -> Result<String, AgentResponse> {
            request
                .parameters
                .get(name)
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .ok_or_else(|| AgentResponse { success: false, data: None, error: Some(format!("missing '{name}' parameter")) })
        };

        let ok = |data: serde_json::Value| AgentResponse { success: true, data: Some(data), error: None };
        let err = |e: anyhow::Error| AgentResponse { success: false, data: None, error: Some(e.to_string()) };

        match request.action.as_str() {
            "navigate" => {
                let url = match param_str("url") {
                    Ok(url) => url,
                    Err(response) => return response,
                };
                match ctx.navigate(&url).await {
                    Ok(dom) => ok(serde_json::to_value(dom).unwrap_or_default()),
                    Err(e) => err(e),
                }
            }
            "query" => {
                let selector = match param_str("selector") {
                    Ok(selector) => selector,
                    Err(response) => return response,
                };
                match ctx.query(&selector).await {
                    Ok(elements) => ok(serde_json::to_value(elements).unwrap_or_default()),
                    Err(e) => err(e),
                }
            }
            "click" => {
                let element_id = match param_str("element_id") {
                    Ok(id) => id,
                    Err(response) => return response,
                };
                match ctx.click(&element_id).await {
                    Ok(dom) => ok(serde_json::to_value(dom).unwrap_or_default()),
                    Err(e) => err(e),
                }
            }
            "input" => {
                let element_id = match param_str("element_id") {
                    Ok(id) => id,
                    Err(response) => return response,
                };
                let value = match param_str("value") {
                    Ok(v) => v,
                    Err(response) => return response,
                };
                match ctx.input(&element_id, &value).await {
                    Ok(()) => ok(json!({})),
                    Err(e) => err(e),
                }
            }
            "get_text" => {
                let element_id = match param_str("element_id") {
                    Ok(id) => id,
                    Err(response) => return response,
                };
                match ctx.get_text(&element_id).await {
                    Ok(text) => ok(json!({ "text": text })),
                    Err(e) => err(e),
                }
            }
            "submit_form" => {
                let form_id = match param_str("form_id") {
                    Ok(id) => id,
                    Err(response) => return response,
                };
                match ctx.submit_form(&form_id).await {
                    Ok(dom) => ok(serde_json::to_value(dom).unwrap_or_default()),
                    Err(e) => err(e),
                }
            }
            "go_back" => match ctx.go_back() {
                Ok(()) => ok(json!({})),
                Err(e) => err(e),
            },
            "go_forward" => {
                let url = match param_str("url") {
                    Ok(url) => url,
                    Err(response) => return response,
                };
                match ctx.go_forward(url) {
                    Ok(()) => ok(json!({})),
                    Err(e) => err(e),
                }
            }
            "current_url" => ok(json!({ "url": ctx.get_current_url() })),
            "history" => ok(json!({ "history": ctx.get_history() })),
            other => AgentResponse { success: false, data: None, error: Some(format!("unknown action: {other}")) },
        }
    }

    fn json_response<T: serde::Serialize>(status: StatusCode, body: &T) -> Result<Response<Body>, hyper::Error> {
        Ok(Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(body).unwrap_or_default()))
            .unwrap())
    }

    async fn device_info(&self) -> Result<Response<Body>, hyper::Error> {
        let body = json!({
            "device_tier": format!("{:?}", self.device_tier),
            "ui_enabled": self.ui_enabled,
            "desktop_ui_compiled": cfg!(feature = "desktop_ui"),
        });
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap())
    }

    #[cfg(feature = "desktop_ui")]
    async fn ui_asset(&self, path: &str) -> Result<Response<Body>, hyper::Error> {
        if !self.ui_enabled {
            let body = json!({
                "error": "Browser UI is not enabled on this device tier",
                "device_tier": format!("{:?}", self.device_tier),
                "hint": "restart with --ui to force it on",
            });
            return Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap());
        }

        let (content_type, body): (&str, &str) = match path {
            "/app" => ("text/html; charset=utf-8", include_str!("ui/web/index.html")),
            "/app/style.css" => ("text/css", include_str!("ui/web/style.css")),
            "/app/app.js" => ("application/javascript", include_str!("ui/web/app.js")),
            "/app/vendor/vue.global.prod.js" => {
                ("application/javascript", include_str!("ui/web/vendor/vue.global.prod.js"))
            }
            _ => unreachable!(),
        };

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", content_type)
            .body(Body::from(body))
            .unwrap())
    }

    async fn health_check(&self) -> Result<Response<Body>, hyper::Error> {
        if self.health_monitor.is_healthy() {
            let body = json!({
                "status": "healthy",
                "uptime_seconds": self.health_monitor.uptime_seconds(),
            });
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap())
        } else {
            Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::from("unhealthy"))
                .unwrap())
        }
    }

    async fn kubernetes_probe(&self) -> Result<Response<Body>, hyper::Error> {
        if self.health_monitor.is_healthy() {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from("OK"))
                .unwrap())
        } else {
            Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::from("Not Ready"))
                .unwrap())
        }
    }

    async fn readiness_probe(&self) -> Result<Response<Body>, hyper::Error> {
        let uptime = self.health_monitor.uptime_seconds();
        // Ready after 1 second of uptime
        if uptime >= 1 && self.health_monitor.is_healthy() {
            let body = json!({
                "ready": true,
                "uptime_seconds": uptime,
            });
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap())
        } else {
            let body = json!({
                "ready": false,
                "uptime_seconds": uptime,
            });
            Ok(Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap())
        }
    }

    async fn stats(&self) -> Result<Response<Body>, hyper::Error> {
        let body = json!({
            "daemon": {
                "uptime_seconds": self.health_monitor.uptime_seconds(),
                "healthy": self.health_monitor.is_healthy(),
            },
            "metrics": {
                "request_count": self.metrics_collector.request_count(),
                "error_count": self.metrics_collector.error_count(),
                "error_rate": {
                    "total_requests": self.metrics_collector.request_count(),
                    "errors": self.metrics_collector.error_count(),
                    "percentage": if self.metrics_collector.request_count() > 0 {
                        (self.metrics_collector.error_count() as f64 / self.metrics_collector.request_count() as f64) * 100.0
                    } else {
                        0.0
                    }
                },
                "memory_mb": self.metrics_collector.memory_mb(),
            },
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap())
    }

    async fn prometheus_metrics(&self) -> Result<Response<Body>, hyper::Error> {
        let uptime = self.health_monitor.uptime_seconds();
        let request_count = self.metrics_collector.request_count();
        let error_count = self.metrics_collector.error_count();
        let memory_mb = self.metrics_collector.memory_mb();

        let metrics = format!(
            "# HELP himalayas_uptime_seconds Daemon uptime in seconds\n\
             # TYPE himalayas_uptime_seconds gauge\n\
             himalayas_uptime_seconds {}\n\
             \n\
             # HELP himalayas_requests_total Total number of requests\n\
             # TYPE himalayas_requests_total counter\n\
             himalayas_requests_total {}\n\
             \n\
             # HELP himalayas_errors_total Total number of errors\n\
             # TYPE himalayas_errors_total counter\n\
             himalayas_errors_total {}\n\
             \n\
             # HELP himalayas_memory_mb Memory usage in MB\n\
             # TYPE himalayas_memory_mb gauge\n\
             himalayas_memory_mb {}\n",
            uptime, request_count, error_count, memory_mb
        );

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain; version=0.0.4")
            .body(Body::from(metrics))
            .unwrap())
    }

    async fn root(&self) -> Result<Response<Body>, hyper::Error> {
        let body = json!({
            "name": "Himalayas Browser - Phase 0",
            "version": "0.1.0",
            "endpoints": {
                "/": "This help message",
                "/health": "Health check (JSON)",
                "/healthz": "Kubernetes probe (text)",
                "/ready": "Readiness probe (JSON)",
                "/stats": "Detailed statistics (JSON)",
                "/metrics": "Prometheus metrics (text)",
                "/device": "Device tier & UI status (JSON)",
                "/app": "Browser UI shell (if enabled)",
                "POST /agent": "Headless agent API — see AgentRequest/AgentResponse in src/api/agent.rs",
            }
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap())
    }

    async fn not_found(&self) -> Result<Response<Body>, hyper::Error> {
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap())
    }

    pub async fn start(self, addr: std::net::SocketAddr) -> Result<()> {
        let server_self = Arc::new(self);

        let make_service = make_service_fn(move |_conn| {
            let server = server_self.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let server = server.clone();
                    async move { server.handle_request(req).await }
                }))
            }
        });

        let server = Server::bind(&addr)
            .serve(make_service)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c()
                    .await
                    .expect("failed to install CTRL+C signal handler");
            });

        info!("Health monitoring server listening on {}", addr);

        if let Err(e) = server.await {
            error!("Server error: {}", e);
            return Err(e.into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::device_detection::DeviceTier;

    fn test_server() -> HealthServer {
        let health_monitor = Arc::new(HealthMonitor::new());
        let metrics_collector = Arc::new(MetricsCollector::new());
        let browser = Arc::new(Browser::new().unwrap());
        HealthServer::new(health_monitor, metrics_collector, true, DeviceTier::Standard, browser)
    }

    async fn agent_request(server: &HealthServer, request: &AgentRequest) -> (StatusCode, AgentResponse) {
        let req = Request::builder()
            .method(Method::POST)
            .uri("/agent")
            .body(Body::from(serde_json::to_vec(request).unwrap()))
            .unwrap();
        let response = server.handle_request(req).await.unwrap();
        let status = response.status();
        let bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    fn request(session_id: &str, action: &str, params: &[(&str, &str)]) -> AgentRequest {
        AgentRequest {
            agent_id: "test-agent".to_string(),
            session_id: session_id.to_string(),
            action: action.to_string(),
            parameters: params.iter().map(|(k, v)| (k.to_string(), json!(v))).collect(),
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        let server = test_server();

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = server.handle_request(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_kubernetes_probe() {
        let server = test_server();

        let req = Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let response = server.handle_request(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stats() {
        let server = test_server();
        server.metrics_collector.record_request();
        server.metrics_collector.record_request();
        server.metrics_collector.record_error();

        let req = Request::builder()
            .uri("/stats")
            .body(Body::empty())
            .unwrap();

        let response = server.handle_request(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_agent_endpoint_rejects_malformed_json() {
        let server = test_server();
        let req = Request::builder()
            .method(Method::POST)
            .uri("/agent")
            .body(Body::from("not json"))
            .unwrap();

        let response = server.handle_request(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let parsed: AgentResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(!parsed.success);
    }

    #[tokio::test]
    async fn test_agent_endpoint_rejects_unknown_action() {
        let server = test_server();
        let (status, response) = agent_request(&server, &request("s1", "not_a_real_action", &[])).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!response.success);
        assert!(response.error.unwrap().contains("unknown action"));
    }

    #[tokio::test]
    async fn test_agent_endpoint_navigate_query_get_text_full_flow() {
        let mut mock_server = mockito::Server::new_async().await;
        mock_server
            .mock("GET", "/")
            .with_status(200)
            .with_body(r#"<html><head><title>Test Page</title></head><body><a id="link1" href="/next">Go</a></body></html>"#)
            .create_async()
            .await;

        let server = test_server();

        let (status, response) = agent_request(&server, &request("agent-session-1", "navigate", &[("url", &mock_server.url())])).await;
        assert_eq!(status, StatusCode::OK);
        assert!(response.success);
        assert_eq!(response.data.unwrap()["title"], "Test Page");

        // A second call with the *same* session_id must reuse the same
        // AgentContext — otherwise query() would fail with "no page loaded".
        let (status, response) = agent_request(&server, &request("agent-session-1", "query", &[("selector", "#link1")])).await;
        assert_eq!(status, StatusCode::OK);
        let elements = response.data.unwrap();
        assert_eq!(elements[0]["text"], "Go");

        let (status, response) = agent_request(&server, &request("agent-session-1", "get_text", &[("element_id", "link1")])).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.data.unwrap()["text"], "Go");
    }

    #[tokio::test]
    async fn test_agent_endpoint_query_before_navigate_fails_cleanly() {
        let server = test_server();
        let (status, response) = agent_request(&server, &request("fresh-session", "query", &[("selector", "#anything")])).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!response.success);
        assert!(response.error.unwrap().contains("no page loaded"));
    }
}
