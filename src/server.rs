use anyhow::Result;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info, debug};

use crate::health::HealthMonitor;
use crate::metrics::MetricsCollector;

pub struct HealthServer {
    health_monitor: Arc<HealthMonitor>,
    metrics_collector: Arc<MetricsCollector>,
}

impl HealthServer {
    pub fn new(
        health_monitor: Arc<HealthMonitor>,
        metrics_collector: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            health_monitor,
            metrics_collector,
        }
    }

    async fn handle_request(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        let path = req.uri().path();
        debug!("Request: {} {}", req.method(), path);

        match path {
            "/health" => self.health_check().await,
            "/healthz" => self.kubernetes_probe().await,
            "/stats" => self.stats().await,
            "/metrics" => self.prometheus_metrics().await,
            "/ready" => self.readiness_probe().await,
            "/" => self.root().await,
            _ => self.not_found().await,
        }
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

    #[tokio::test]
    async fn test_health_check() {
        let health_monitor = Arc::new(HealthMonitor::new());
        let metrics_collector = Arc::new(MetricsCollector::new());
        let server = HealthServer::new(health_monitor, metrics_collector);

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = server.handle_request(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_kubernetes_probe() {
        let health_monitor = Arc::new(HealthMonitor::new());
        let metrics_collector = Arc::new(MetricsCollector::new());
        let server = HealthServer::new(health_monitor, metrics_collector);

        let req = Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap();

        let response = server.handle_request(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stats() {
        let health_monitor = Arc::new(HealthMonitor::new());
        let metrics_collector = Arc::new(MetricsCollector::new());
        metrics_collector.record_request();
        metrics_collector.record_request();
        metrics_collector.record_error();

        let server = HealthServer::new(health_monitor, metrics_collector);

        let req = Request::builder()
            .uri("/stats")
            .body(Body::empty())
            .unwrap();

        let response = server.handle_request(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
