use himalayas::browser::Browser;
use himalayas::health::HealthMonitor;
use himalayas::intelligence::device_detection::DeviceTier;
use himalayas::metrics::MetricsCollector;
use himalayas::server::HealthServer;
use hyper::{Client, StatusCode};
use std::sync::Arc;
use tokio::time::Duration;

#[tokio::test]
async fn test_health_server_startup() {
    let health_monitor = Arc::new(HealthMonitor::new());
    let metrics_collector = Arc::new(MetricsCollector::new());
    let browser = Arc::new(Browser::new().unwrap());
    let server = HealthServer::new(health_monitor.clone(), metrics_collector.clone(), true, DeviceTier::Standard, browser);

    let addr = "127.0.0.1:8081".parse().unwrap();

    let server_handle = tokio::spawn(async move {
        let _ = server.start(addr).await;
    });

    tokio::time::sleep(Duration::from_millis(1500)).await;

    let client = Client::new();

    // Test root endpoint
    let res = client.get("http://127.0.0.1:8081/".parse().unwrap()).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), StatusCode::OK);

    // Test health endpoint
    let res = client
        .get("http://127.0.0.1:8081/health".parse().unwrap())
        .await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), StatusCode::OK);

    // Test ready endpoint
    let res = client
        .get("http://127.0.0.1:8081/ready".parse().unwrap())
        .await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), StatusCode::OK);

    // Test metrics endpoint
    let res = client
        .get("http://127.0.0.1:8081/metrics".parse().unwrap())
        .await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), StatusCode::OK);

    // Test stats endpoint
    let res = client
        .get("http://127.0.0.1:8081/stats".parse().unwrap())
        .await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), StatusCode::OK);

    // Test kubernetes probe
    let res = client
        .get("http://127.0.0.1:8081/healthz".parse().unwrap())
        .await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), StatusCode::OK);

    // Test 404
    let res = client
        .get("http://127.0.0.1:8081/notfound".parse().unwrap())
        .await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status(), StatusCode::NOT_FOUND);

    server_handle.abort();
}

#[test]
fn test_health_monitor() {
    let health_monitor = HealthMonitor::new();
    assert!(health_monitor.is_healthy());
    let initial_uptime = health_monitor.uptime_seconds();
    assert_eq!(initial_uptime, 0);

    std::thread::sleep(Duration::from_secs(1));
    let uptime_after_1sec = health_monitor.uptime_seconds();
    assert!(uptime_after_1sec >= 1);
}

#[test]
fn test_metrics_collector() {
    let metrics = MetricsCollector::new();

    assert_eq!(metrics.request_count(), 0);
    assert_eq!(metrics.error_count(), 0);

    metrics.record_request();
    assert_eq!(metrics.request_count(), 1);

    metrics.record_request();
    metrics.record_request();
    assert_eq!(metrics.request_count(), 3);

    metrics.record_error();
    metrics.record_error();
    assert_eq!(metrics.error_count(), 2);
}

#[test]
fn test_metrics_collector_clone() {
    let metrics1 = MetricsCollector::new();
    let metrics2 = metrics1.clone();

    metrics1.record_request();
    metrics2.record_request();

    // Both should share the same state
    assert_eq!(metrics1.request_count(), 2);
    assert_eq!(metrics2.request_count(), 2);
}
