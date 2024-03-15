use std::{fs, thread::current};

use chrono::{DateTime, Utc};
use influxdb::{Client, Error, InfluxDbWriteable, WriteQuery};
use log::{info, error, warn};
use serde::Deserialize;

//mod voltage_cal;

/* ----- BEGIN CONFIG FILE STRUCTS ------ */
#[derive(Debug, Deserialize, Clone)]
pub struct ConfigFile {
    pub influxdb: InfluxDBConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InfluxDBConfig {
    pub endpoint: String,
    pub database_name: String,
    pub token: String,
    pub site_name: String,
}

/* ----- END CONFIG FILE STRUCTS ------ */

pub fn load_config(file_name: String) -> ConfigFile {
    let toml_str = fs::read_to_string(file_name).expect("Failed to read the configuration file.");
    return toml::from_str(&toml_str).expect("Malformed configuration file.");
}

/* ----- BEGIN INFLUXDB STRUCTS ----- */

#[derive(InfluxDbWriteable)]
struct BME680 {
    pub time: DateTime<Utc>,
    pub temperature_f: f32,
    pub humidity: f32,
    pub pressure: f32,
    pub gas: f32,
    #[influxdb(tag)]
    pub location: String,
}

#[derive(InfluxDbWriteable)]
struct BME280 {
    pub time: DateTime<Utc>,
    pub temperature_f: f32,
    pub humidity: f32,
    pub pressure: f32,
    #[influxdb(tag)]
    pub location: String,
}

#[derive(InfluxDbWriteable)]
struct TMP36 {
    pub time: DateTime<Utc>,
    pub temperature_f: f32,
    #[influxdb(tag)]
    pub location: String,
}

#[derive(InfluxDbWriteable)]
struct SupplyVoltage {
    pub time: DateTime<Utc>,
    pub main: f32,
    pub amplifier: f32,
    #[influxdb(tag)]
    pub location: String,
}

#[derive(InfluxDbWriteable)]
struct RFPower {
    pub time: DateTime<Utc>,
    pub forward: f32,
    pub reverse: f32,
    pub swr: f32,
    #[influxdb(tag)]
    pub location: String,
}

pub fn calculate_swr(forward_power: f32, reverse_power: f32) -> f32 {

    let swr = (1f32 + (reverse_power / forward_power).sqrt()) / (1f32 - (reverse_power / forward_power).sqrt());

    if swr.is_nan() {
        warn!("Calculated SWR is NaN. Result set to zero instead. Forward: {}, Reverse {}", forward_power, reverse_power);
        return 0f32;
    }
    if swr.is_sign_negative() {
        warn!("Calculated SWR is negative. Result set to zero instead. Forward: {}, Reverse {}", forward_power, reverse_power);
        return 0f32;
    }

    return swr;
}



pub fn parse_serial(serial_buf: Vec<u8>) {
    
}


/* ------ END INFLUXDB STRUCTS ------ */


pub fn read_sensor_data_test(sensor_readings: &mut Vec<WriteQuery>) {
    sensor_readings.push(
        BME680 {
            time: Utc::now(),
            temperature_f: 72.0,
            humidity: 31.0,
            pressure: 1002.0,
            gas: 96.0,
            location: "kg5key".to_string(),
        }
        .into_query("bme680"));

}


/// # send_sensor_data()
pub async fn send_sensor_data(influx_client: influxdb::Client, sensor_readings: Vec<WriteQuery>) -> Result<(), Error> {

    info!("Sending sensor readings to InfluxDB.");
    let result = influx_client.query(sensor_readings).await?;
    
    Ok(())
}


pub fn splice_sensor_readings(location: String, input_string: String) -> Vec<WriteQuery> {
    
    let mut influx_query: Vec<WriteQuery> = vec![];

    // Split the string by commas
    let values: Vec<&str> = input_string.split(',').collect();

    let time = Utc::now();

    influx_query.push(
        BME280 {
            time,
            temperature_f: values[0].parse().unwrap(),
            humidity: values[1].parse().unwrap(),
            pressure: values[2].parse().unwrap(),
            location: location.clone(),
        }
        .into_query("bme280")
    );
    
    influx_query.push(
        TMP36 {
            time,
            temperature_f: values[3].parse().unwrap(),
            location: location.clone(),
        }
        .into_query("bme280")
    );

    // TODO: Calibrate & scale the voltage sensor readings.
    let main_voltage: f32 = values[4].parse().unwrap();
    let amp_voltage: f32 = values[5].parse().unwrap();

    influx_query.push(
        SupplyVoltage {
            time,
            main: main_voltage,
            amplifier: amp_voltage,
            location: location.clone(),
        }
        .into_query("voltage")
    );

    let rf_forward: f32 = values[6].parse().unwrap();
    let rf_reverse: f32 = values[7].parse().unwrap();

    influx_query.push(
        RFPower {
            time,
            forward: rf_forward,
            reverse: rf_reverse,
            swr: calculate_swr(rf_forward, rf_reverse),
            location,
        }
        .into_query("rf_power")
    );

    influx_query
}
