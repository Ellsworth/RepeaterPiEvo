#![deny(clippy::all, clippy::pedantic, clippy::allow_attributes_without_reason)]

use chrono::{DateTime, Utc};
use influxdb::{InfluxDbWriteable, WriteQuery};

/* ----- BEGIN INFLUXDB STRUCTS ----- */

#[derive(InfluxDbWriteable)]
struct BMP280 {
    pub time: DateTime<Utc>,
    pub temperature_f: f64,
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
    pub usb: f64,
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

#[derive(InfluxDbWriteable)]
struct CPUTemp {
    pub time: DateTime<Utc>,
    pub temperature_c: f64,
    #[influxdb(tag)]
    pub location: String,
}

#[must_use]
pub fn calculate_swr(forward_power: f64, reverse_power: f64) -> f64 {
    let swr = (1f64 + (reverse_power / forward_power).sqrt())
        / (1f64 - (reverse_power / forward_power).sqrt());

    if (reverse_power > forward_power) | swr.is_sign_negative() {
        log::warn!(
            "Calculated SWR is negative due to the reflected power being greater than the forward power. This should not be possible under normal operation."
        );
    }

    swr
}

/* ------ END INFLUXDB STRUCTS ------ */

fn evaluate_polynomial(coefficients: &[f64], x: f64) -> f64 {
    let mut result = 0.0;
    let mut power_of_x = 1.0;

    for &coefficient in coefficients {
        result += coefficient * power_of_x;
        power_of_x *= x;
    }

    result
}

fn clamp_value(input: f64, limit: f64) -> f64 {
    if input < limit { 0.0 } else { input }
}

/// Parses raw serial input string and optional CPU temperature into a structured `TelemetrySample`.
///
/// # Errors
/// Returns an error message string if parsing fails or input has insufficient fields.
pub fn parse_to_sample(
    input_string: &str,
    calibration: &crate::config::Calibration,
    cpu_temp_c: Option<f64>,
) -> Result<crate::change_detector::TelemetrySample, String> {
    let values: Vec<&str> = input_string.split(',').collect();
    if values.len() < 8 {
        return Err(format!(
            "Split values has unexpected size of {} (expected at least 8)",
            values.len()
        ));
    }

    let bmp280_temp: f64 = values[0]
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse bmp280_temp_f: {e}"))?;
    let bmp280_press: f64 = values[1]
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse bmp280_press: {e}"))?;
    let tmp36_temp: f64 = values[2]
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse tmp36_temp_f: {e}"))?;
    let main_voltage_raw: f64 = values[3]
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse main_voltage: {e}"))?;
    let amp_voltage_raw: f64 = values[4]
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse amp_voltage: {e}"))?;
    let usb_voltage_raw: f64 = values[5]
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse usb_voltage: {e}"))?;
    let forward_raw: f64 = values[6]
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse forward_power: {e}"))?;
    let reverse_raw: f64 = values[7]
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse reverse_power: {e}"))?;

    let voltage_main = clamp_value(
        evaluate_polynomial(&calibration.voltage_main, main_voltage_raw),
        calibration.voltage_clamp,
    );
    let voltage_amp = clamp_value(
        evaluate_polynomial(&calibration.voltage_amp, amp_voltage_raw),
        calibration.voltage_clamp,
    );
    let voltage_usb = evaluate_polynomial(&calibration.voltage_usb, usb_voltage_raw);

    let power_fwd = clamp_value(
        evaluate_polynomial(&calibration.power_forward, forward_raw),
        calibration.power_forward_clamp,
    );
    let power_rev = clamp_value(
        evaluate_polynomial(&calibration.power_reverse, reverse_raw),
        calibration.power_reverse_clamp,
    );
    let swr = clamp_value(calculate_swr(forward_raw, reverse_raw), 0.0);

    Ok(crate::change_detector::TelemetrySample {
        time: Utc::now(),
        bmp280_pressure: Some(bmp280_press),
        bmp280_temperature_f: Some(bmp280_temp),
        pi_status_temperature_c: cpu_temp_c,
        tmp36_temperature_f: Some(tmp36_temp),
        voltage_amplifier: Some(voltage_amp),
        voltage_main: Some(voltage_main),
        voltage_usb: Some(voltage_usb),
        rf_power_forward: Some(power_fwd),
        rf_power_reverse: Some(power_rev),
        rf_power_swr: Some(swr),
    })
}

/// Converts a `TelemetrySample` into a batch of `WriteQuery` records for `InfluxDB`.
#[must_use]
pub fn sample_to_write_queries(
    sample: &crate::change_detector::TelemetrySample,
    location: &str,
) -> Vec<WriteQuery> {
    let mut influx_query: Vec<WriteQuery> = vec![];

    if let (Some(temp), Some(press)) = (sample.bmp280_temperature_f, sample.bmp280_pressure) {
        influx_query.push(
            BMP280 {
                time: sample.time,
                temperature_f: temp,
                pressure: press,
                location: location.to_string(),
            }
            .try_into_query("bmp280")
            .unwrap(),
        );
    }

    if let Some(temp) = sample.tmp36_temperature_f {
        influx_query.push(
            TMP36 {
                time: sample.time,
                temperature_f: temp,
                location: location.to_string(),
            }
            .try_into_query("tmp36")
            .unwrap(),
        );
    }

    if let (Some(main), Some(amp), Some(usb)) = (
        sample.voltage_main,
        sample.voltage_amplifier,
        sample.voltage_usb,
    ) {
        influx_query.push(
            SupplyVoltage {
                time: sample.time,
                main,
                amplifier: amp,
                usb,
                location: location.to_string(),
            }
            .try_into_query("voltage")
            .unwrap(),
        );
    }

    if let (Some(fwd), Some(rev), Some(swr)) = (
        sample.rf_power_forward,
        sample.rf_power_reverse,
        sample.rf_power_swr,
    ) {
        influx_query.push(
            RFPower {
                time: sample.time,
                forward: fwd,
                reverse: rev,
                swr,
                location: location.to_string(),
            }
            .try_into_query("rf_power")
            .unwrap(),
        );
    }

    if let Some(temp_c) = sample.pi_status_temperature_c {
        influx_query.push(
            CPUTemp {
                time: sample.time,
                temperature_c: temp_c,
                location: location.to_string(),
            }
            .try_into_query("pi_status")
            .unwrap(),
        );
    }

    influx_query
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_calibration() -> crate::config::Calibration {
        crate::config::Calibration {
            voltage_main: vec![0.0, 1.0],
            voltage_amp: vec![0.0, 1.0],
            voltage_clamp: 1.0,
            voltage_usb: vec![0.0, 1.0],
            power_forward: vec![0.0, 1.0],
            power_forward_clamp: 0.1,
            power_reverse: vec![0.0, 1.0],
            power_reverse_clamp: 0.1,
        }
    }

    #[test]
    fn test_parse_to_sample_success() {
        let cal = test_calibration();
        let line = "72.5, 1013.25, 71.0, 13.8, 13.8, 5.0, 25.0, 0.5, 0.0";
        let sample = parse_to_sample(line, &cal, Some(45.0)).unwrap();

        assert_eq!(sample.bmp280_temperature_f, Some(72.5));
        assert_eq!(sample.bmp280_pressure, Some(1013.25));
        assert_eq!(sample.tmp36_temperature_f, Some(71.0));
        assert_eq!(sample.voltage_main, Some(13.8));
        assert_eq!(sample.voltage_amplifier, Some(13.8));
        assert_eq!(sample.voltage_usb, Some(5.0));
        assert_eq!(sample.rf_power_forward, Some(25.0));
        assert_eq!(sample.rf_power_reverse, Some(0.5));
        assert_eq!(sample.pi_status_temperature_c, Some(45.0));
        assert!(sample.rf_power_swr.is_some());

        let queries = sample_to_write_queries(&sample, "test_site");
        assert_eq!(queries.len(), 5);
    }

    #[test]
    fn test_parse_to_sample_invalid_format() {
        let cal = test_calibration();
        let line = "72.5, 1013.25";
        let result = parse_to_sample(line, &cal, None);
        assert!(result.is_err());
    }
}
