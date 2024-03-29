#![deny(clippy::all, clippy::pedantic, clippy::allow_attributes_without_reason)]

use std::fs;

use chrono::{DateTime, Utc};
use influxdb::{Error, InfluxDbWriteable, WriteQuery};
use log::{error, info, warn};
use serde::Deserialize;

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

/* ----- BEGIN INFLUXDB STRUCTS ----- */

#[derive(InfluxDbWriteable)]
struct BME280 {
    pub time: DateTime<Utc>,
    pub temperature_f: f64,
    pub humidity: f64,
    pub pressure: f64,
    #[influxdb(tag)]
    pub location: String,
}

#[derive(InfluxDbWriteable)]
struct TMP36 {
    pub time: DateTime<Utc>,
    pub temperature_f: f64,
    #[influxdb(tag)]
    pub location: String,
}

#[derive(InfluxDbWriteable)]
struct SupplyVoltage {
    pub time: DateTime<Utc>,
    pub main: f64,
    pub amplifier: f64,
    #[influxdb(tag)]
    pub location: String,
}

#[derive(InfluxDbWriteable)]
struct RFPower {
    pub time: DateTime<Utc>,
    pub forward: f64,
    pub reverse: f64,
    pub swr: f64,
    #[influxdb(tag)]
    pub location: String,
}

pub fn calculate_swr(forward_power: f64, reverse_power: f64) -> f64 {
    let swr = (1f64 + (reverse_power / forward_power).sqrt())
        / (1f64 - (reverse_power / forward_power).sqrt());

    if swr.is_nan() {
        warn!(
            "Calculated SWR is NaN. Result set to zero instead. Forward: {}, Reverse {}",
            forward_power, reverse_power
        );
        return 0f64;
    }
    if swr.is_sign_negative() {
        warn!(
            "Calculated SWR is negative. Result set to zero instead. Forward: {}, Reverse {}",
            forward_power, reverse_power
        );
        return 0f64;
    }

    swr
}

/* ------ END INFLUXDB STRUCTS ------ */

/// # `send_sensor_data()`
pub async fn send_sensor_data(
    influx_client: influxdb::Client,
    sensor_readings: Vec<WriteQuery>,
) -> Result<(), Error> {
    info!("Sending sensor readings to InfluxDB.");
    let _result = influx_client.query(sensor_readings).await?;

    Ok(())
}

fn evaluate_polynomial(coefficients: &[f64], x: f64) -> f64 {
    let mut result = 0.0;
    let mut power_of_x = 1.0;

    for &coefficient in coefficients {
        result += coefficient * power_of_x;
        power_of_x *= x;
    }

    result
}

pub fn splice_sensor_readings(
    location: String,
    input_string: &str,
    calibration: &CalibrationConfig,
) -> Vec<WriteQuery> {
    let mut influx_query: Vec<WriteQuery> = vec![];

    // Split the string by commas
    let values: Vec<&str> = input_string.split(',').collect();
    let time = Utc::now();

    if values.len() != 8 {
        error!("Split values has unexpected size of {}", values.len());
    }

    influx_query.push(
        BME280 {
            time,
            temperature_f: values[0].parse().unwrap(),
            humidity: values[1].parse().unwrap(),
            pressure: values[2].parse().unwrap(),
            location: location.clone(),
        }
        .into_query("bme280"),
    );

    influx_query.push(
        TMP36 {
            time,
            temperature_f: values[3].parse().unwrap(),
            location: location.clone(),
        }
        .into_query("tmp36"),
    );

    let main_voltage: f64 = values[4].parse().unwrap();
    let amp_voltage: f64 = values[5].parse().unwrap();

    influx_query.push(
        SupplyVoltage {
            time,
            main: evaluate_polynomial(&calibration.voltage_main, main_voltage),
            amplifier: evaluate_polynomial(&calibration.voltage_amp, amp_voltage),
            location: location.clone(),
        }
        .into_query("voltage"),
    );

    let forward: f64 = values[6].parse().unwrap();
    let reverse: f64 = values[7].parse().unwrap();

    influx_query.push(
        RFPower {
            time,
            forward: evaluate_polynomial(&calibration.power_forward, forward),
            reverse: evaluate_polynomial(&calibration.power_reverse, reverse),
            swr: calculate_swr(forward, reverse),
            location,
        }
        .into_query("rf_power"),
    );

    influx_query
}
