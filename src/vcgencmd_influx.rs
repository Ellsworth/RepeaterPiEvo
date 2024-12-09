use chrono::{DateTime, Utc};
use influxdb::{InfluxDbWriteable, WriteQuery};

#[derive(InfluxDbWriteable)]
struct PiStatus {
    pub time: DateTime<Utc>,
    pub temperature_c: f64,
    #[influxdb(tag)]
    pub location: String,
}

pub(crate) fn get_vcgencmd_stats(location: String) -> Vec<WriteQuery> {
    let time = Utc::now();
    let mut influx_query: Vec<WriteQuery> = vec![];

    let temp_c = vcgencmd::measure_temp().unwrap();



    influx_query.push(
        PiStatus {
            time,
            temperature_c: temp_c,
            location: location.clone(),
        }
        .into_query("pi_cpu_temp"),
    );

    influx_query
}
