use anyhow::Result;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration_ms: f64,
    pub memory_mb: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStats {
    pub name: String,
    pub count: usize,
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub median_ms: f64,
    pub std_dev_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

pub struct Benchmarker {
    results: Vec<BenchmarkResult>,
}

impl Benchmarker {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn measure<F>(mut self, name: &str, iterations: usize, mut f: F) -> Result<Self>
    where
        F: FnMut() -> Result<()>,
    {
        let mut durations = Vec::new();

        for _ in 0..iterations {
            let start = Instant::now();
            f()?;
            let duration = start.elapsed();
            durations.push(duration.as_secs_f64() * 1000.0);
        }

        let memory_mb = get_memory_usage_mb();
        let mean_duration = durations.iter().sum::<f64>() / durations.len() as f64;

        self.results.push(BenchmarkResult {
            name: name.to_string(),
            duration_ms: mean_duration,
            memory_mb,
            timestamp: chrono::Local::now().to_rfc3339(),
        });

        Ok(self)
    }

    pub fn startup_benchmark() -> Result<BenchmarkStats> {
        let mut durations = Vec::new();
        const ITERATIONS: usize = 5;

        for _ in 0..ITERATIONS {
            let start = Instant::now();

            // Simulate daemon startup (health monitor + metrics collector initialization)
            let _health = crate::health::HealthMonitor::new();
            let _metrics = crate::metrics::MetricsCollector::new();

            let duration = start.elapsed();
            durations.push(duration.as_secs_f64() * 1000.0);
        }

        Ok(BenchmarkStats {
            name: "daemon_startup".to_string(),
            count: ITERATIONS,
            min_ms: durations.iter().cloned().fold(f64::INFINITY, f64::min),
            max_ms: durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            mean_ms: durations.iter().sum::<f64>() / ITERATIONS as f64,
            median_ms: calculate_percentile(&durations, 50.0),
            std_dev_ms: calculate_std_dev(&durations),
            p95_ms: calculate_percentile(&durations, 95.0),
            p99_ms: calculate_percentile(&durations, 99.0),
        })
    }

    pub fn memory_benchmark() -> Result<BenchmarkStats> {
        let measurements = 10;
        let mut memory_samples = Vec::new();

        let _health = crate::health::HealthMonitor::new();
        let _metrics = crate::metrics::MetricsCollector::new();

        for i in 0..measurements {
            memory_samples.push(get_memory_usage_mb() as f64);
            if i < measurements - 1 {
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        Ok(BenchmarkStats {
            name: "memory_footprint".to_string(),
            count: measurements,
            min_ms: memory_samples.iter().cloned().fold(f64::INFINITY, f64::min),
            max_ms: memory_samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            mean_ms: memory_samples.iter().sum::<f64>() / measurements as f64,
            median_ms: calculate_percentile(&memory_samples, 50.0),
            std_dev_ms: calculate_std_dev(&memory_samples),
            p95_ms: calculate_percentile(&memory_samples, 95.0),
            p99_ms: calculate_percentile(&memory_samples, 99.0),
        })
    }

    pub fn metrics_overhead_benchmark() -> Result<BenchmarkStats> {
        let metrics = crate::metrics::MetricsCollector::new();
        let iterations = 100_000;
        let mut durations = Vec::new();

        for _ in 0..10 {
            let start = Instant::now();
            for _ in 0..iterations {
                metrics.record_request();
            }
            let duration = start.elapsed();
            durations.push((duration.as_secs_f64() / iterations as f64) * 1_000_000.0); // Convert to microseconds
        }

        Ok(BenchmarkStats {
            name: "metrics_overhead_us".to_string(),
            count: iterations * 10,
            min_ms: durations.iter().cloned().fold(f64::INFINITY, f64::min),
            max_ms: durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            mean_ms: durations.iter().sum::<f64>() / durations.len() as f64,
            median_ms: calculate_percentile(&durations, 50.0),
            std_dev_ms: calculate_std_dev(&durations),
            p95_ms: calculate_percentile(&durations, 95.0),
            p99_ms: calculate_percentile(&durations, 99.0),
        })
    }

    pub fn http_request_latency_benchmark() -> Result<BenchmarkStats> {
        // Simulated HTTP request latency (for now, measuring JSON serialization overhead)
        let iterations = 1000;
        let mut durations = Vec::new();

        for _ in 0..iterations {
            let start = Instant::now();
            let _json = serde_json::json!({
                "status": "healthy",
                "uptime_seconds": 1,
                "metrics": {
                    "request_count": 1000,
                    "error_count": 5,
                }
            });
            let _str = _json.to_string();
            let duration = start.elapsed();
            durations.push(duration.as_secs_f64() * 1_000_000.0); // microseconds
        }

        Ok(BenchmarkStats {
            name: "http_response_us".to_string(),
            count: iterations,
            min_ms: durations.iter().cloned().fold(f64::INFINITY, f64::min),
            max_ms: durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            mean_ms: durations.iter().sum::<f64>() / durations.len() as f64,
            median_ms: calculate_percentile(&durations, 50.0),
            std_dev_ms: calculate_std_dev(&durations),
            p95_ms: calculate_percentile(&durations, 95.0),
            p99_ms: calculate_percentile(&durations, 99.0),
        })
    }

    pub fn run_all_benchmarks() -> Result<Vec<BenchmarkStats>> {
        let mut results = Vec::new();

        println!("\n📊 Running Himalayas Browser Phase 0 Benchmarks\n");

        println!("🚀 Startup Time Benchmark...");
        let startup = Self::startup_benchmark()?;
        results.push(startup.clone());
        print_stats(&startup);

        println!("\n💾 Memory Footprint Benchmark...");
        let memory = Self::memory_benchmark()?;
        results.push(memory.clone());
        print_stats(&memory);

        println!("\n⚡ Metrics Collection Overhead...");
        let metrics_overhead = Self::metrics_overhead_benchmark()?;
        results.push(metrics_overhead.clone());
        print_stats(&metrics_overhead);

        println!("\n🌐 HTTP Response Latency...");
        let http_latency = Self::http_request_latency_benchmark()?;
        results.push(http_latency.clone());
        print_stats(&http_latency);

        println!("\n✅ All benchmarks complete!\n");

        Ok(results)
    }

    pub fn save_results(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.results)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_results(path: &str) -> Result<Vec<BenchmarkResult>> {
        if !Path::new(path).exists() {
            return Ok(Vec::new());
        }
        let json = fs::read_to_string(path)?;
        let results = serde_json::from_str(&json)?;
        Ok(results)
    }
}

pub fn get_memory_usage_mb() -> u64 {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
        {
            if let Ok(rss_str) = String::from_utf8(output.stdout) {
                if let Ok(rss_kb) = rss_str.trim().parse::<u64>() {
                    return rss_kb / 1024; // Convert KB to MB
                }
            }
        }
        0
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
        {
            if let Ok(rss_str) = String::from_utf8(output.stdout) {
                if let Ok(rss_kb) = rss_str.trim().parse::<u64>() {
                    return rss_kb / 1024; // Convert KB to MB
                }
            }
        }
        0
    }

    #[cfg(target_os = "windows")]
    {
        0 // TODO: Implement for Windows
    }
}

fn calculate_percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let index = ((percentile / 100.0) * (sorted.len() - 1) as f64).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn calculate_std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance =
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

fn print_stats(stats: &BenchmarkStats) {
    println!("  {} (n={})", stats.name, stats.count);
    println!("    Min:    {:.2}", stats.min_ms);
    println!("    Max:    {:.2}", stats.max_ms);
    println!("    Mean:   {:.2}", stats.mean_ms);
    println!("    Median: {:.2}", stats.median_ms);
    println!("    StdDev: {:.2}", stats.std_dev_ms);
    println!("    P95:    {:.2}", stats.p95_ms);
    println!("    P99:    {:.2}", stats.p99_ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_benchmark() {
        let result = Benchmarker::startup_benchmark().unwrap();
        assert!(result.mean_ms < 500.0, "Startup should be < 500ms");
        assert!(result.count > 0);
    }

    #[test]
    fn test_memory_benchmark() {
        let result = Benchmarker::memory_benchmark().unwrap();
        assert!(result.mean_ms < 200.0, "Memory should be < 200MB");
        assert!(result.count > 0);
    }

    #[test]
    fn test_metrics_overhead() {
        let result = Benchmarker::metrics_overhead_benchmark().unwrap();
        assert!(result.mean_ms < 1.0, "Metrics overhead should be < 1 microsecond");
        assert!(result.count > 0);
    }

    #[test]
    fn test_percentile_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let p50 = calculate_percentile(&values, 50.0);
        assert!(p50 >= 2.0 && p50 <= 4.0);
    }

    #[test]
    fn test_std_dev_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let std_dev = calculate_std_dev(&values);
        assert!(std_dev > 0.0);
    }
}
