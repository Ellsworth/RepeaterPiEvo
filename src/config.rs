use serde::Deserialize;
use std::fs;

/* ----- BEGIN CONFIG FILE STRUCTS ------ */
#[derive(Debug, Deserialize, Clone)]
pub struct Root {
    pub influxdb: InfluxDB,
    pub calibration: Calibration,
    pub serial: Serial,
    #[serde(default)]
    pub change_detector: crate::change_detector::ChangeDetectorConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InfluxDB {
    pub endpoint: String,
    pub database_name: String,
    pub token: String,
    pub site_name: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct PchipCurve {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub d: Vec<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Calibration {
    pub voltage_main: PchipCurve,
    pub voltage_amp: PchipCurve,
    pub voltage_clamp: f64,
    pub voltage_usb: PchipCurve,
    pub power_forward: PchipCurve,
    pub power_forward_clamp: f64,
    pub power_reverse: PchipCurve,
    pub power_reverse_clamp: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Serial {
    pub port: String,
    pub baud: u32,
}

/* ----- END CONFIG FILE STRUCTS ------ */

pub fn load(file_name: String) -> Root {
    let toml_str = fs::read_to_string(file_name).expect("Failed to read the configuration file.");
    toml::from_str(&toml_str).expect("Malformed configuration file.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_file() {
        let root = load("config.toml".to_string());
        assert!(!root.calibration.voltage_main.x.is_empty());
        assert_eq!(
            root.calibration.voltage_main.x.len(),
            root.calibration.voltage_main.y.len()
        );
        assert_eq!(
            root.calibration.voltage_main.x.len(),
            root.calibration.voltage_main.d.len()
        );
        assert!((root.calibration.voltage_clamp - 1.0).abs() < 1e-9);
    }
}
