//! Performance metrics collection and reporting
//!
//! This module provides phase-level timing for profiling `stacy` execution.
//! It tracks:
//! - Spawn time: Time to launch Stata process
//! - Execution time: Wall-clock time for Stata to run
//! - Parse time: Time to parse log file for errors
//! - Total time: End-to-end CLI execution time
//!
//! Usage:
//! ```ignore
//! let mut metrics = Metrics::new();
//! metrics.start_phase("spawn");
//! // ... spawn process
//! metrics.end_phase("spawn");
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance metrics for stacy execution
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    /// Individual phase timings (spawn, execution, parse)
    phases: HashMap<String, Duration>,

    /// Overall start time
    start_time: Option<Instant>,

    /// Overall end time
    end_time: Option<Instant>,

    /// Currently active phase
    active_phase: Option<(String, Instant)>,
}

impl Metrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            phases: HashMap::new(),
            start_time: None,
            end_time: None,
            active_phase: None,
        }
    }

    /// Start overall timing
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// End overall timing
    pub fn end(&mut self) {
        self.end_time = Some(Instant::now());
    }

    /// Start timing a phase
    pub fn start_phase(&mut self, phase: &str) {
        self.active_phase = Some((phase.to_string(), Instant::now()));
    }

    /// End timing the current phase
    pub fn end_phase(&mut self, phase: &str) {
        if let Some((active_name, start)) = &self.active_phase {
            if active_name == phase {
                let duration = start.elapsed();
                self.phases.insert(phase.to_string(), duration);
                self.active_phase = None;
            } else {
                eprintln!(
                    "Warning: Ending phase '{}' but active phase is '{}'",
                    phase, active_name
                );
            }
        }
    }

    /// Record a phase timing manually
    pub fn record_phase(&mut self, phase: &str, duration: Duration) {
        self.phases.insert(phase.to_string(), duration);
    }

    /// Get timing for a specific phase
    pub fn get_phase(&self, phase: &str) -> Option<Duration> {
        self.phases.get(phase).copied()
    }

    /// Get total execution time
    pub fn total_duration(&self) -> Option<Duration> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            _ => None,
        }
    }

    /// Calculate overhead (total - sum of phases)
    pub fn overhead(&self) -> Duration {
        let total = self.total_duration().unwrap_or(Duration::ZERO);
        let phases_sum: Duration = self.phases.values().sum();
        total.saturating_sub(phases_sum)
    }

    /// Get all phase timings
    pub fn all_phases(&self) -> &HashMap<String, Duration> {
        &self.phases
    }

    /// Format metrics for human-readable display
    pub fn format_display(&self) -> String {
        let mut output = String::new();
        output.push_str("📊 Performance Profile:\n");

        // Show individual phases
        let mut phase_names: Vec<&String> = self.phases.keys().collect();
        phase_names.sort();

        for phase in phase_names {
            if let Some(duration) = self.phases.get(phase) {
                output.push_str(&format!(
                    "  {:12} {:>8.2}ms\n",
                    format!("{}:", phase),
                    duration.as_secs_f64() * 1000.0
                ));
            }
        }

        // Show overhead
        let overhead = self.overhead();
        output.push_str(&format!(
            "  {:12} {:>8.2}ms\n",
            "overhead:",
            overhead.as_secs_f64() * 1000.0
        ));

        // Show total
        if let Some(total) = self.total_duration() {
            output.push_str(&format!(
                "  {:12} {:>8.2}ms\n",
                "total:",
                total.as_secs_f64() * 1000.0
            ));
        }

        output
    }

    /// Convert metrics to JSON-serializable format
    pub fn to_json_value(&self) -> serde_json::Value {
        use serde_json::json;

        let mut phases_ms = serde_json::Map::new();
        for (name, duration) in &self.phases {
            phases_ms.insert(
                name.clone(),
                json!(format!("{:.2}", duration.as_secs_f64() * 1000.0)),
            );
        }

        json!({
            "phases_ms": phases_ms,
            "overhead_ms": format!("{:.2}", self.overhead().as_secs_f64() * 1000.0),
            "total_ms": self.total_duration()
                .map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0))
                .unwrap_or_else(|| "0.00".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_basic_metrics() {
        let mut metrics = Metrics::new();
        metrics.start();

        metrics.start_phase("test");
        thread::sleep(Duration::from_millis(10));
        metrics.end_phase("test");

        metrics.end();

        assert!(metrics.get_phase("test").is_some());
        assert!(metrics.get_phase("test").unwrap() >= Duration::from_millis(10));
        assert!(metrics.total_duration().is_some());
    }

    #[test]
    fn test_manual_recording() {
        let mut metrics = Metrics::new();
        metrics.record_phase("spawn", Duration::from_millis(45));
        metrics.record_phase("execution", Duration::from_millis(5000));
        metrics.record_phase("parse", Duration::from_millis(23));

        assert_eq!(metrics.get_phase("spawn"), Some(Duration::from_millis(45)));
        assert_eq!(
            metrics.get_phase("execution"),
            Some(Duration::from_millis(5000))
        );
        assert_eq!(metrics.get_phase("parse"), Some(Duration::from_millis(23)));
    }

    #[test]
    fn test_overhead_calculation() {
        let mut metrics = Metrics::new();
        metrics.start();

        // Simulate some work with overhead
        thread::sleep(Duration::from_millis(50));

        metrics.record_phase("phase1", Duration::from_millis(10));
        metrics.record_phase("phase2", Duration::from_millis(10));

        thread::sleep(Duration::from_millis(50));
        metrics.end();

        // Overhead should be approximately total - (phase1 + phase2)
        let overhead = metrics.overhead();
        assert!(overhead >= Duration::from_millis(80)); // ~100ms total - 20ms phases
    }

    #[test]
    fn test_format_display() {
        let mut metrics = Metrics::new();
        metrics.record_phase("spawn", Duration::from_millis(45));
        metrics.record_phase("execution", Duration::from_millis(5120));
        metrics.record_phase("parse", Duration::from_millis(23));

        let display = metrics.format_display();
        assert!(display.contains("spawn:"));
        assert!(display.contains("execution:"));
        assert!(display.contains("parse:"));
        assert!(display.contains("45.00ms"));
        assert!(display.contains("5120.00ms"));
        assert!(display.contains("23.00ms"));
    }

    #[test]
    fn test_json_serialization() {
        let mut metrics = Metrics::new();
        metrics.record_phase("spawn", Duration::from_millis(45));
        metrics.record_phase("execution", Duration::from_millis(5120));

        let json = metrics.to_json_value();
        assert!(json["phases_ms"]["spawn"].is_string());
        assert!(json["phases_ms"]["execution"].is_string());
        assert!(json["overhead_ms"].is_string());
        assert!(json["total_ms"].is_string());
    }
}
