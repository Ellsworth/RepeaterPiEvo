use serde::Deserialize;
use std::fs;

/* ----- BEGIN CONFIG FILE STRUCTS ------ */
#[derive(Debug, Deserialize, Clone)]
pub struct Root {
    pub influxdb: InfluxDB,
    pub calibration: Calibration,
    pub serial: Serial,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InfluxDB {
    pub endpoint: String,
    pub database_name: String,
    pub token: String,
    pub site_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Calibration {
    pub voltage_main: Vec<f64>,
    pub voltage_amp: Vec<f64>,
    pub voltage_usb: Vec<f64>,
    pub power_forward: Vec<f64>,
    pub power_reverse: Vec<f64>,
    pub voltage_clamp: f64,
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
