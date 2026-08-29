#![deny(clippy::all, clippy::pedantic, clippy::allow_attributes_without_reason)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

fn default_threshold_value() -> f64 {
    0.5
}

fn default_tx_power_threshold() -> f64 {
    0.5
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "Serde field default requires Option<u64>"
)]
fn default_heartbeat_interval_secs() -> Option<u64> {
    Some(1800)
}

fn default_ignored_metrics() -> Vec<String> {
    Vec::new()
}

fn default_thresholds() -> HashMap<String, f64> {
    let mut m = HashMap::new();
    m.insert("bmp280_pressure".to_string(), 50.0);
    m.insert("bmp280_temperature_f".to_string(), 1.0);
    m.insert("pi_status_temperature_c".to_string(), 2.0);
    m.insert("tmp36_temperature_f".to_string(), 2.0);
    m.insert("voltage_amplifier".to_string(), 0.1);
    m.insert("voltage_main".to_string(), 0.1);
    m.insert("voltage_usb".to_string(), 0.1);
    m.insert("rf_power_forward".to_string(), 0.5);
    m.insert("rf_power_reverse".to_string(), 0.2);
    m
}

/// Configuration for time-series change detection and deadband filtering.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ChangeDetectorConfig {
    /// Whether change detection filtering is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Absolute difference thresholds required to trigger a transmission for specific metrics.
    #[serde(default = "default_thresholds")]
    pub thresholds: HashMap<String, f64>,

    /// Default deadband threshold for metrics not explicitly listed in `thresholds`.
    #[serde(default = "default_threshold_value")]
    pub default_threshold: f64,

    /// Metrics to ignore when checking deadband deltas (e.g. calculated/noisy metrics).
    #[serde(default = "default_ignored_metrics")]
    pub ignored_metrics: Vec<String>,

    /// Maximum inactivity interval (in seconds) before sending a heartbeat sample.
    #[serde(default = "default_heartbeat_interval_secs")]
    pub heartbeat_interval_secs: Option<u64>,

    /// Minimum interval (in seconds) between sent samples (rate limiting / debounce).
    #[serde(default)]
    pub min_interval_secs: Option<u64>,

    /// Whether to continuously emit all samples while RF transmission is active.
    #[serde(default = "default_true")]
    pub always_send_on_tx: bool,

    /// Forward RF power threshold (in Watts) to classify as active transmission.
    #[serde(default = "default_tx_power_threshold")]
    pub tx_power_threshold: f64,
}

impl Default for ChangeDetectorConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            thresholds: default_thresholds(),
            default_threshold: default_threshold_value(),
            ignored_metrics: default_ignored_metrics(),
            heartbeat_interval_secs: default_heartbeat_interval_secs(),
            min_interval_secs: None,
            always_send_on_tx: default_true(),
            tx_power_threshold: default_tx_power_threshold(),
        }
    }
}

/// A structured telemetry sample capturing all sensor and system metrics at a timestamp.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetrySample {
    pub time: DateTime<Utc>,
    pub bmp280_pressure: Option<f64>,
    pub bmp280_temperature_f: Option<f64>,
    pub pi_status_temperature_c: Option<f64>,
    pub tmp36_temperature_f: Option<f64>,
    pub voltage_amplifier: Option<f64>,
    pub voltage_main: Option<f64>,
    pub voltage_usb: Option<f64>,
    pub rf_power_forward: Option<f64>,
    pub rf_power_reverse: Option<f64>,
    pub rf_power_swr: Option<f64>,
}

impl TelemetrySample {
    /// Retrieves a numeric metric value by name if present.
    #[must_use]
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        match name {
            "bmp280_pressure" => self.bmp280_pressure,
            "bmp280_temperature_f" => self.bmp280_temperature_f,
            "pi_status_temperature_c" => self.pi_status_temperature_c,
            "tmp36_temperature_f" => self.tmp36_temperature_f,
            "voltage_amplifier" => self.voltage_amplifier,
            "voltage_main" => self.voltage_main,
            "voltage_usb" => self.voltage_usb,
            "rf_power_forward" => self.rf_power_forward,
            "rf_power_reverse" => self.rf_power_reverse,
            _ => None,
        }
    }

    /// Returns a list of active numeric metrics present in this sample evaluated for changes.
    #[must_use]
    pub fn metrics(&self) -> Vec<(&'static str, f64)> {
        let mut list = Vec::with_capacity(9);
        if let Some(v) = self.bmp280_pressure {
            list.push(("bmp280_pressure", v));
        }
        if let Some(v) = self.bmp280_temperature_f {
            list.push(("bmp280_temperature_f", v));
        }
        if let Some(v) = self.pi_status_temperature_c {
            list.push(("pi_status_temperature_c", v));
        }
        if let Some(v) = self.tmp36_temperature_f {
            list.push(("tmp36_temperature_f", v));
        }
        if let Some(v) = self.voltage_amplifier {
            list.push(("voltage_amplifier", v));
        }
        if let Some(v) = self.voltage_main {
            list.push(("voltage_main", v));
        }
        if let Some(v) = self.voltage_usb {
            list.push(("voltage_usb", v));
        }
        if let Some(v) = self.rf_power_forward {
            list.push(("rf_power_forward", v));
        }
        if let Some(v) = self.rf_power_reverse {
            list.push(("rf_power_reverse", v));
        }
        list
    }
}

/// The decision produced by evaluating a telemetry sample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    /// Whether the sample should be emitted / written to the database.
    pub should_send: bool,
    /// Explanation of the decision.
    pub reason: String,
}

/// Stateful change detector that determines whether an incoming telemetry sample
/// differs significantly from the last emitted sample.
#[derive(Debug, Clone)]
pub struct TelemetryChangeDetector {
    config: ChangeDetectorConfig,
    last_sent_sample: Option<TelemetrySample>,
    last_sent_time: Option<DateTime<Utc>>,
}

impl TelemetryChangeDetector {
    /// Creates a new detector instance with the specified configuration.
    #[must_use]
    pub fn new(config: ChangeDetectorConfig) -> Self {
        Self {
            config,
            last_sent_sample: None,
            last_sent_time: None,
        }
    }

    /// Resets the internal tracking state.
    #[allow(
        dead_code,
        reason = "Part of stateful detector API and used in testing"
    )]
    pub fn reset(&mut self) {
        self.last_sent_sample = None;
        self.last_sent_time = None;
    }

    /// Accesses the detector configuration.
    #[allow(dead_code, reason = "Part of public detector API")]
    #[must_use]
    pub fn config(&self) -> &ChangeDetectorConfig {
        &self.config
    }

    /// Accesses the last sent sample, if any.
    #[allow(
        dead_code,
        reason = "Part of state inspection API and used in testing"
    )]
    #[must_use]
    pub fn last_sent_sample(&self) -> Option<&TelemetrySample> {
        self.last_sent_sample.as_ref()
    }

    /// Accesses the timestamp of the last sent sample, if any.
    #[allow(
        dead_code,
        reason = "Part of state inspection API and used in testing"
    )]
    #[must_use]
    pub fn last_sent_time(&self) -> Option<DateTime<Utc>> {
        self.last_sent_time
    }

    /// Evaluates a single sample against the last transmitted sample.
    ///
    /// Evaluates in strict priority order:
    /// 1. Change detector disabled check.
    /// 2. Initial observation (first sample).
    /// 3. Active transmission (`always_send_on_tx`).
    /// 4. Rate limiting / debounce (`min_interval_secs`).
    /// 5. Heartbeat (`heartbeat_interval_secs`).
    /// 6. Metric deadband check.
    /// 7. No significant change (suppression).
    #[allow(
        clippy::cast_possible_wrap,
        reason = "Interval seconds fits comfortably within i64"
    )]
    pub fn evaluate_sample(&mut self, sample: &TelemetrySample) -> Evaluation {
        if !self.config.enabled {
            return Evaluation {
                should_send: true,
                reason: "change_detector_disabled".to_string(),
            };
        }

        // 1. Initial sample: Always send first observation
        if self.last_sent_sample.is_none() {
            self.last_sent_sample = Some(sample.clone());
            self.last_sent_time = Some(sample.time);
            return Evaluation {
                should_send: true,
                reason: "initial_sample".to_string(),
            };
        }

        // 2. Always send full stream while transmitter is active
        let fwd_pwr = sample.rf_power_forward.unwrap_or(0.0);
        if self.config.always_send_on_tx && fwd_pwr >= self.config.tx_power_threshold {
            self.last_sent_sample = Some(sample.clone());
            self.last_sent_time = Some(sample.time);
            return Evaluation {
                should_send: true,
                reason: format!("transmitting (rf_power_forward={fwd_pwr:.1}W)"),
            };
        }

        // 3. Debounce / Minimum Interval check (rate limiting)
        if let (Some(min_secs), Some(last_time)) =
            (self.config.min_interval_secs, self.last_sent_time)
            && sample.time.signed_duration_since(last_time).num_seconds() < min_secs as i64
        {
            return Evaluation {
                should_send: false,
                reason: "rate_limited".to_string(),
            };
        }

        // 4. Heartbeat / Maximum Interval check
        if let (Some(hb_secs), Some(last_time)) =
            (self.config.heartbeat_interval_secs, self.last_sent_time)
            && sample.time.signed_duration_since(last_time).num_seconds() >= hb_secs as i64
        {
            self.last_sent_sample = Some(sample.clone());
            self.last_sent_time = Some(sample.time);
            return Evaluation {
                should_send: true,
                reason: "heartbeat".to_string(),
            };
        }

        // 5. Metric Deadband evaluation against last emitted sample
        let mut reasons = Vec::new();
        if let Some(ref prev_sample) = self.last_sent_sample {
            for (key, val) in sample.metrics() {
                if self.config.ignored_metrics.iter().any(|m| m == key) {
                    continue;
                }

                if let Some(prev_val) = prev_sample.get_metric(key) {
                    let threshold = self
                        .config
                        .thresholds
                        .get(key)
                        .copied()
                        .unwrap_or(self.config.default_threshold);
                    let delta = (val - prev_val).abs();

                    if delta >= threshold {
                        reasons.push(format!("{key} (Δ={delta:.2} >= {threshold})"));
                    }
                }
            }
        }

        if !reasons.is_empty() {
            self.last_sent_sample = Some(sample.clone());
            self.last_sent_time = Some(sample.time);
            return Evaluation {
                should_send: true,
                reason: reasons.join("; "),
            };
        }

        Evaluation {
            should_send: false,
            reason: "no_significant_change".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_at(time: DateTime<Utc>) -> TelemetrySample {
        TelemetrySample {
            time,
            bmp280_pressure: Some(1013.25),
            bmp280_temperature_f: Some(72.0),
            pi_status_temperature_c: Some(45.0),
            tmp36_temperature_f: Some(71.5),
            voltage_amplifier: Some(13.8),
            voltage_main: Some(13.6),
            voltage_usb: Some(5.1),
            rf_power_forward: Some(0.0),
            rf_power_reverse: Some(0.0),
            rf_power_swr: Some(1.0),
        }
    }

    #[test]
    fn test_initial_sample() {
        let mut detector = TelemetryChangeDetector::new(ChangeDetectorConfig::default());
        let t0 = Utc::now();
        let s0 = sample_at(t0);

        let eval = detector.evaluate_sample(&s0);
        assert!(eval.should_send);
        assert_eq!(eval.reason, "initial_sample");
        assert_eq!(detector.last_sent_time(), Some(t0));
    }

    #[test]
    fn test_no_significant_change_suppressed() {
        let mut detector = TelemetryChangeDetector::new(ChangeDetectorConfig::default());
        let t0 = Utc::now();
        let s0 = sample_at(t0);
        detector.evaluate_sample(&s0);

        // Minimal jitter below deadband thresholds
        let t1 = t0 + Duration::seconds(10);
        let mut s1 = sample_at(t1);
        s1.bmp280_pressure = Some(1013.30); // delta 0.05 < 50.0
        s1.bmp280_temperature_f = Some(72.2); // delta 0.2 < 1.0
        s1.pi_status_temperature_c = Some(45.5); // delta 0.5 < 2.0
        s1.tmp36_temperature_f = Some(72.0); // delta 0.5 < 2.0
        s1.voltage_main = Some(13.62); // delta 0.02 < 0.1

        let eval = detector.evaluate_sample(&s1);
        assert!(!eval.should_send);
        assert_eq!(eval.reason, "no_significant_change");
        // State should still point to s0
        assert_eq!(detector.last_sent_time(), Some(t0));
    }

    #[test]
    fn test_active_transmission_always_sends() {
        let mut detector = TelemetryChangeDetector::new(ChangeDetectorConfig::default());
        let t0 = Utc::now();
        let s0 = sample_at(t0);
        detector.evaluate_sample(&s0);

        let t1 = t0 + Duration::seconds(5);
        let mut s1 = sample_at(t1);
        s1.rf_power_forward = Some(25.4);
        s1.rf_power_reverse = Some(0.8);

        let eval = detector.evaluate_sample(&s1);
        assert!(eval.should_send);
        assert!(eval.reason.starts_with("transmitting (rf_power_forward=25.4W)"));
        assert_eq!(detector.last_sent_time(), Some(t1));

        // Subsequent sample during transmission also sends
        let t2 = t1 + Duration::seconds(1);
        let mut s2 = sample_at(t2);
        s2.rf_power_forward = Some(25.3);
        let eval2 = detector.evaluate_sample(&s2);
        assert!(eval2.should_send);
        assert!(eval2.reason.starts_with("transmitting"));
    }

    #[test]
    fn test_transmission_end_key_down_triggers_deadband() {
        let mut detector = TelemetryChangeDetector::new(ChangeDetectorConfig::default());
        let t0 = Utc::now();
        let mut s0 = sample_at(t0);
        s0.rf_power_forward = Some(25.0);
        detector.evaluate_sample(&s0);

        // Key down (forward power drops to 0.0 W)
        let t1 = t0 + Duration::seconds(1);
        let mut s1 = sample_at(t1);
        s1.rf_power_forward = Some(0.0);

        let eval = detector.evaluate_sample(&s1);
        assert!(eval.should_send);
        assert!(eval.reason.contains("rf_power_forward (Δ=25.00 >= 0.5)"));
    }

    #[test]
    fn test_deadband_metric_change() {
        let mut detector = TelemetryChangeDetector::new(ChangeDetectorConfig::default());
        let t0 = Utc::now();
        let s0 = sample_at(t0);
        detector.evaluate_sample(&s0);

        // Voltage main drops significantly
        let t1 = t0 + Duration::seconds(15);
        let mut s1 = sample_at(t1);
        s1.voltage_main = Some(13.4); // delta = 0.2 >= 0.1

        let eval = detector.evaluate_sample(&s1);
        assert!(eval.should_send);
        assert_eq!(eval.reason, "voltage_main (Δ=0.20 >= 0.1)");
        assert_eq!(detector.last_sent_time(), Some(t1));
    }

    #[test]
    fn test_ignored_metrics_do_not_trigger() {
        let mut detector = TelemetryChangeDetector::new(ChangeDetectorConfig::default());
        let t0 = Utc::now();
        let s0 = sample_at(t0);
        detector.evaluate_sample(&s0);

        // Change swr significantly (ignored metric)
        let t1 = t0 + Duration::seconds(10);
        let mut s1 = sample_at(t1);
        s1.rf_power_swr = Some(5.0); // delta = 4.0

        let eval = detector.evaluate_sample(&s1);
        assert!(!eval.should_send);
        assert_eq!(eval.reason, "no_significant_change");
    }

    #[test]
    fn test_heartbeat_timeout() {
        let config = ChangeDetectorConfig {
            heartbeat_interval_secs: Some(60),
            ..Default::default()
        };
        let mut detector = TelemetryChangeDetector::new(config);

        let t0 = Utc::now();
        let s0 = sample_at(t0);
        detector.evaluate_sample(&s0);

        // Quiet sample before heartbeat
        let t1 = t0 + Duration::seconds(30);
        let s1 = sample_at(t1);
        assert!(!detector.evaluate_sample(&s1).should_send);

        // Quiet sample at or after heartbeat
        let t2 = t0 + Duration::seconds(60);
        let s2 = sample_at(t2);
        let eval = detector.evaluate_sample(&s2);
        assert!(eval.should_send);
        assert_eq!(eval.reason, "heartbeat");
        assert_eq!(detector.last_sent_time(), Some(t2));
    }

    #[test]
    fn test_rate_limiting_debounce() {
        let config = ChangeDetectorConfig {
            min_interval_secs: Some(5),
            ..Default::default()
        };
        let mut detector = TelemetryChangeDetector::new(config);

        let t0 = Utc::now();
        let s0 = sample_at(t0);
        detector.evaluate_sample(&s0);

        // Metric changed but arrived within 2 seconds (< min_interval 5s)
        let t1 = t0 + Duration::seconds(2);
        let mut s1 = sample_at(t1);
        s1.voltage_main = Some(11.0); // Big change

        let eval = detector.evaluate_sample(&s1);
        assert!(!eval.should_send);
        assert_eq!(eval.reason, "rate_limited");

        // After debounce interval expires (6s > 5s)
        let t2 = t0 + Duration::seconds(6);
        let mut s2 = sample_at(t2);
        s2.voltage_main = Some(11.0);

        let eval2 = detector.evaluate_sample(&s2);
        assert!(eval2.should_send);
        assert_eq!(eval2.reason, "voltage_main (Δ=2.60 >= 0.1)");
    }

    #[test]
    fn test_reset() {
        let mut detector = TelemetryChangeDetector::new(ChangeDetectorConfig::default());
        let t0 = Utc::now();
        let s0 = sample_at(t0);
        detector.evaluate_sample(&s0);
        assert!(detector.last_sent_sample().is_some());

        detector.reset();
        assert!(detector.last_sent_sample().is_none());
        assert!(detector.last_sent_time().is_none());

        let eval = detector.evaluate_sample(&s0);
        assert!(eval.should_send);
        assert_eq!(eval.reason, "initial_sample");
    }

    #[test]
    fn test_disabled_change_detector() {
        let config = ChangeDetectorConfig {
            enabled: false,
            ..Default::default()
        };
        let mut detector = TelemetryChangeDetector::new(config);

        let t0 = Utc::now();
        let s0 = sample_at(t0);
        let eval = detector.evaluate_sample(&s0);
        assert!(eval.should_send);
        assert_eq!(eval.reason, "change_detector_disabled");
    }

    #[test]
    fn test_multiple_metrics_changed() {
        let mut detector = TelemetryChangeDetector::new(ChangeDetectorConfig::default());
        let t0 = Utc::now();
        let s0 = sample_at(t0);
        detector.evaluate_sample(&s0);

        let t1 = t0 + Duration::seconds(10);
        let mut s1 = sample_at(t1);
        s1.bmp280_temperature_f = Some(75.0); // delta = 3.0 >= 1.0
        s1.bmp280_pressure = Some(1100.0); // delta = 86.75 >= 50.0

        let eval = detector.evaluate_sample(&s1);
        assert!(eval.should_send);
        assert!(eval.reason.contains("bmp280_temperature_f (Δ=3.00 >= 1)"));
        assert!(eval.reason.contains("bmp280_pressure (Δ=86.75 >= 50)"));
        assert!(eval.reason.contains("; "));
    }

    #[test]
    fn test_config_deserialize_toml() {
        let toml_str = r#"
            enabled = true
            always_send_on_tx = true
            tx_power_threshold = 1.0
            heartbeat_interval_secs = 600
            default_threshold = 0.25
            ignored_metrics = ["rf_power_swr", "dummy"]

            [thresholds]
            voltage_main = 0.05
        "#;
        let config: ChangeDetectorConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert!((config.tx_power_threshold - 1.0).abs() < f64::EPSILON);
        assert_eq!(config.heartbeat_interval_secs, Some(600));
        assert!((config.default_threshold - 0.25).abs() < f64::EPSILON);
        assert_eq!(config.thresholds.get("voltage_main"), Some(&0.05));
        assert_eq!(config.ignored_metrics, vec!["rf_power_swr", "dummy"]);
    }
}
