use serde::Deserialize;
use std::fs;

/* ----- BEGIN CONFIG FILE STRUCTS ------ */
#[derive(Debug, Deserialize, Clone)]
pub struct ConfigFile {
    pub influxdb: InfluxDBConfig,
    pub calibration: CalibrationConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InfluxDBConfig {
    pub endpoint: String,
    pub database_name: String,
    pub token: String,
    pub site_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CalibrationConfig {
    pub voltage_main: Vec<f64>,
    pub voltage_amp: Vec<f64>,
    pub power_forward: Vec<f64>,
    pub power_reverse: Vec<f64>,
}

/* ----- END CONFIG FILE STRUCTS ------ */

pub fn load_config(file_name: String) -> ConfigFile {
    let toml_str = fs::read_to_string(file_name).expect("Failed to read the configuration file.");
    toml::from_str(&toml_str).expect("Malformed configuration file.")
}
