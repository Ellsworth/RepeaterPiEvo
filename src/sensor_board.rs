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

impl crate::config::PchipCurve {
    /// Evaluates the cubic Hermite interpolant at raw ADC count `x`.
    #[must_use]
    pub fn evaluate(&self, x: f64) -> f64 {
        let n = self.x.len();
        if n == 0 || self.y.is_empty() {
            return 0.0;
        }
        if n == 1 || self.y.len() == 1 || self.d.len() < 2 {
            return self.y[0];
        }

        let idx = if x <= self.x[0] {
            0
        } else if x >= self.x[n - 1] {
            n - 2
        } else {
            let pos = self.x.partition_point(|&val| val <= x);
            pos.saturating_sub(1).min(n - 2)
        };

        let x0 = self.x[idx];
        let x1 = self.x[idx + 1];
        let y0 = self.y[idx];
        let y1 = self.y[idx + 1];
        let d0 = self.d[idx];
        let d1 = self.d[idx + 1];

        let h = x1 - x0;
        if h == 0.0 {
            return y0;
        }

        let dx = x - x0;
        let delta = (y1 - y0) / h;
        let c0 = y0;
        let c1 = d0;
        let c2 = (3.0 * delta - 2.0 * d0 - d1) / h;
        let c3 = (d0 + d1 - 2.0 * delta) / (h * h);

        c0 + dx * (c1 + dx * (c2 + dx * c3))
    }
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
        calibration.voltage_main.evaluate(main_voltage_raw),
        calibration.voltage_clamp,
    );
    let voltage_amp = clamp_value(
        calibration.voltage_amp.evaluate(amp_voltage_raw),
        calibration.voltage_clamp,
    );
    let voltage_usb = calibration.voltage_usb.evaluate(usb_voltage_raw);

    let power_fwd = clamp_value(
        calibration.power_forward.evaluate(forward_raw),
        calibration.power_forward_clamp,
    );
    let power_rev = clamp_value(
        calibration.power_reverse.evaluate(reverse_raw),
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
        let linear_curve = crate::config::PchipCurve {
            x: vec![0.0, 100.0],
            y: vec![0.0, 100.0],
            d: vec![1.0, 1.0],
        };

        crate::config::Calibration {
            voltage_main: linear_curve.clone(),
            voltage_amp: linear_curve.clone(),
            voltage_clamp: 1.0,
            voltage_usb: linear_curve.clone(),
            power_forward: linear_curve.clone(),
            power_forward_clamp: 0.1,
            power_reverse: linear_curve,
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

    #[test]
    fn test_pchip_evaluate_linear() {
        let curve = crate::config::PchipCurve {
            x: vec![0.0, 10.0, 20.0],
            y: vec![0.0, 50.0, 100.0],
            d: vec![5.0, 5.0, 5.0],
        };

        // Knot points
        assert!((curve.evaluate(0.0) - 0.0).abs() < 1e-9);
        assert!((curve.evaluate(10.0) - 50.0).abs() < 1e-9);
        assert!((curve.evaluate(20.0) - 100.0).abs() < 1e-9);

        // Midpoints
        assert!((curve.evaluate(5.0) - 25.0).abs() < 1e-9);
        assert!((curve.evaluate(15.0) - 75.0).abs() < 1e-9);

        // Extrapolations
        assert!((curve.evaluate(-2.0) - (-10.0)).abs() < 1e-9);
        assert!((curve.evaluate(25.0) - 125.0).abs() < 1e-9);
    }

    #[test]
    fn test_pchip_evaluate_edge_cases() {
        let empty_curve = crate::config::PchipCurve {
            x: vec![],
            y: vec![],
            d: vec![],
        };
        assert!((empty_curve.evaluate(5.0) - 0.0).abs() < 1e-9);

        let single_point_curve = crate::config::PchipCurve {
            x: vec![10.0],
            y: vec![42.0],
            d: vec![0.0],
        };
        assert!((single_point_curve.evaluate(5.0) - 42.0).abs() < 1e-9);
    }

    #[test]
    fn test_parse_to_sample_with_real_config() {
        let root = crate::config::load("config.toml".to_string());
        // Real ADC readings from Sensorboard_SN0.xlsx:
        // Main ADC: 11057 (Row 38, ground truth 13.797V)
        // Amplifier ADC: 11216 (Row 38, ground truth 13.797V)
        // USB ADC: 279.508197 (5.0V full scale)
        // Forward Power ADC: 7891 (29.5W)
        // Reverse Power ADC: 163 (0.5W)
        let line = "72.5, 1013.25, 71.0, 11057, 11216, 279.508197, 7891, 163, 0.0";
        let sample = parse_to_sample(line, &root.calibration, Some(45.0)).unwrap();

        let v_main = sample.voltage_main.unwrap();
        let v_amp = sample.voltage_amplifier.unwrap();
        let v_usb = sample.voltage_usb.unwrap();
        let p_fwd = sample.rf_power_forward.unwrap();
        let p_rev = sample.rf_power_reverse.unwrap();

        assert!((v_main - 13.797).abs() < 1e-3, "Expected ~13.797V, got {v_main}");
        assert!((v_amp - 13.797).abs() < 1e-3, "Expected ~13.797V, got {v_amp}");
        assert!((v_usb - 5.0).abs() < 1e-3, "Expected ~5.0V, got {v_usb}");
        assert!((p_fwd - 29.5).abs() < 1e-3, "Expected ~29.5W, got {p_fwd}");
        assert!((p_rev - 0.5).abs() < 1e-3, "Expected ~0.5W, got {p_rev}");
    }

    #[test]
    fn test_pchip_evaluate_realistic_voltage_spline() {
        let root = crate::config::load("config.toml".to_string());
        let main_curve = &root.calibration.voltage_main;

        // Exact knot points from Sensorboard_SN0.xlsx
        assert!((main_curve.evaluate(479.0) - 0.5).abs() < 1e-4);
        assert!((main_curve.evaluate(959.0) - 1.0).abs() < 1e-4);
        assert!((main_curve.evaluate(11057.0) - 13.797).abs() < 1e-4);
        assert!((main_curve.evaluate(11093.0) - 13.897).abs() < 1e-4);

        // Interpolated midpoint between 11057.0 (13.797V) and 11093.0 (13.897V)
        // Computed by SciPy PCHIP as 13.8467048...
        let mid_val = main_curve.evaluate(11075.0);
        assert!(
            (mid_val - 13.8467).abs() < 1e-3,
            "Expected midpoint ~13.8467V, got {mid_val}"
        );
    }
}
