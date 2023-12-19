use chrono::{Utc, DateTime};
use influxdb::{Client, Error, InfluxDbWriteable};
use log::{info, error};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct Config {
    influxdb: InfluxDBConfig,
}

#[derive(Debug, Deserialize)]
struct InfluxDBConfig {
    endpoint: String,
    database_name: String,
    token: String,
    site_name: String,
}

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
struct Voltage {
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
    #[influxdb(tag)]
    pub location: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {

    env_logger::init();
    info!("Starting RepeaterPi Evo v{} by Erich Ellsworth, KG5KEY.", env!("CARGO_PKG_VERSION"));
    info!("This is free software licensed under the GNU General Public License v2.");

    info!("Reading config.toml.");
    let toml_str = fs::read_to_string("config.toml").expect("Failed to read Cargo.toml file");
    let config: Config = toml::from_str(&toml_str).expect("Failed to deserialize Cargo.toml");

    info!("Using the '{}' database at endpoint '{}'.", config.influxdb.database_name, config.influxdb.endpoint);
    
    let client = Client::new(config.influxdb.endpoint, config.influxdb.database_name).with_token(config.influxdb.token);

    let sensor_reading = vec![
        BME680 {
            time: Utc::now(),
            temperature_f: 72.0,
            humidity: 31.0,
            pressure: 1002.0,
            gas: 96.0,
            location: config.influxdb.site_name,
        }
        .into_query("bme680"),
    ];

    info!("Sending sensor readings to InfluxDB.");
    let result = client.query(sensor_reading).await?;

    if !result.is_empty() {
        error!("InfluxDB Response: {}", result);
    }

    Ok(())
}
