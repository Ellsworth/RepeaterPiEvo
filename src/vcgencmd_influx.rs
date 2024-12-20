use chrono::{DateTime, Utc};
use influxdb::{InfluxDbWriteable, WriteQuery};

#[derive(InfluxDbWriteable)]
struct PiStatus {
    pub time: DateTime<Utc>,
    pub temperature_c: f64,
    pub clock_freq_hz: i64,
    #[influxdb(tag)]
    pub location: String,
}

/// Allowing `clippy::needless_pass_by_value` because this function
/// should match the format as the other WriteQuery helper functions.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn get_vcgencmd_stats(location: String) -> Vec<WriteQuery> {
    let time = Utc::now();
    let mut influx_query: Vec<WriteQuery> = vec![];

    let temp_c_result = vcgencmd::measure_temp();
    let clock_freq_result = vcgencmd::measure_clock(vcgencmd::Src::Clock(vcgencmd::ClockSrc::Arm));

    if let (Ok(temp_c), Ok(clock_freq)) = (temp_c_result, clock_freq_result) {
        influx_query.push(
            PiStatus {
                time,
                temperature_c: temp_c,
                clock_freq_hz: clock_freq as i64,
                location: location,
            }
            .into_query("pi_status"),
        );
    } else {
        log::error!("Failed to run vcgencmd!");
    }

    influx_query
}
