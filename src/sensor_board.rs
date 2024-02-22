use std::fs;

use chrono::{DateTime, Utc};
use influxdb::{Client, Error, InfluxDbWriteable, WriteQuery};
use log::{info, error, warn};
use serde::Deserialize;

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


pub fn read_sensor_data(sensor_readings: &mut Vec<WriteQuery>) {
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
pub async fn send_sensor_data(influx_client: influxdb::Client, sensor_readings: &Vec<WriteQuery>) -> Result<(), Error> {

    info!("Sending sensor readings to InfluxDB.");
    let result = influx_client.query(sensor_readings).await?;
    
    Ok(())
}

#[derive(Debug)]
struct SensorboardData {
    temperature_f: f32,
    pressure_pascals: f32,
    main_voltage: f32,
    amp_voltage: f32,
    forward_power: f32,
    reflected_power: f32,
    tmp36_voltage: f32,
}

impl SensorboardData {
    pub fn from_csv(csv_string: &str) -> Result<SensorboardData, &'static str> {
        let mut values = csv_string.split(',');

        // Parse individual values
        let temperature_f    = values.next().ok_or("Missing temperature_f")?.trim().parse::<f32>().map_err(|_| "Invalid temperature_f")?;
        let pressure_pascals = values.next().ok_or("Missing pressure_pascals")?.trim().parse::<f32>().map_err(|_| "Invalid pressure_pascals")?;
        let main_voltage     = values.next().ok_or("Missing main_voltage")?.trim().parse::<f32>().map_err(|_| "Invalid main_voltage")?;
        let amp_voltage      = values.next().ok_or("Missing amp_voltage")?.trim().parse::<f32>().map_err(|_| "Invalid amp_voltage")?;
        let forward_power    = values.next().ok_or("Missing forward_power")?.trim().parse::<f32>().map_err(|_| "Invalid forward_power")?;
        let reflected_power  = values.next().ok_or("Missing reflected_power")?.trim().parse::<f32>().map_err(|_| "Invalid reflected_power")?;
        let tmp36_voltage    = values.next().ok_or("Missing tmp36_voltage")?.trim().parse::<f32>().map_err(|_| "Invalid tmp36_voltage")?;

        // Ensure no extra values are present
        if values.next().is_some() {
            return Err("Too many fields");
        }

        Ok(SensorboardData {temperature_f, pressure_pascals, main_voltage, amp_voltage, forward_power, reflected_power, tmp36_voltage})
    }
}

pub fn sensorboard_parse() {
    let rx_string: String = "72.4,1002.3,13.2,13.8,74,13,72.1".to_string();

    match SensorboardData::from_csv(&rx_string) {
        Ok(my_struct) => {
            println!("{:?}", my_struct);
        }
        Err(err) => {
            eprintln!("Error: {}", err);
        }
    }

}