use chrono::{DateTime, Utc};
use influxdb::{InfluxDbWriteable, WriteQuery};
use std::fs;

#[derive(InfluxDbWriteable)]
struct CPUTemp {
    pub time: DateTime<Utc>,
    pub temperature_c: f64,
    #[influxdb(tag)]
    pub location: String,
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "WriteQuery helpers should take the same args."
)]
pub(crate) fn get_cpu_stats(location: String) -> Vec<WriteQuery> {
    let time = Utc::now();
    let mut influx_query: Vec<WriteQuery> = vec![];

    // Read the temperature from the system file
    match fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        Ok(temp_str) => {
            if let Ok(temp_raw) = temp_str.trim().parse::<f64>() {
                // Convert temperature from millidegrees Celsius to Celsius
                let temp_c = temp_raw / 1000.0;
                influx_query.push(
                    CPUTemp {
                        time,
                        temperature_c: temp_c,
                        location,
                    }
                    .into_query("pi_status"),
                );
            } else {
                log::error!("Failed to parse temperature value.");
            }
        }
        Err(err) => {
            log::error!("Failed to read CPU temperature: {}", err);
        }
    }

    influx_query
}
