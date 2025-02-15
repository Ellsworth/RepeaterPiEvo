use chrono::{DateTime, Utc};
use influxdb::{InfluxDbWriteable, WriteQuery};
use std::process::Command;

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

    let output = Command::new("/opt/vc/bin/vcgencmd")
        .arg("measure_temp")
        .output();

    let output_str = match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Ok(output) => {
            log::error!("Command failed with status: {:?}", output.status.code());
            return influx_query;
        }
        Err(err) => {
            log::error!("Failed to execute vcgencmd command: {}", err);
            return influx_query;
        }
    };

    let temp_c = output_str
        .strip_prefix("temp=")
        .and_then(|s| s.strip_suffix("'C"))
        .and_then(|temp| temp.parse::<f64>().ok());

    match temp_c {
        Some(temp) => {
            influx_query.push(
                CPUTemp {
                    time,
                    temperature_c: temp,
                    location,
                }
                .into_query("pi_status"),
            );
        }
        None => log::error!("Unexpected output format: {}", output_str),
    }

    influx_query
}
