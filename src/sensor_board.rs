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

pub fn calculate_swr(forward_power: f64, reverse_power: f64) -> f64 {
    let swr = (1f64 + (reverse_power / forward_power).sqrt())
        / (1f64 - (reverse_power / forward_power).sqrt());

    if (reverse_power > forward_power) | swr.is_sign_negative() {
        log::warn!("Calculated SWR is negative due to the reflected power being greater than the forward power. This should not be possible under normal operation.");
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

pub fn splice_sensor_readings(
    location: String,
    input_string: &str,
    calibration: &crate::config::Calibration,
) -> Vec<WriteQuery> {
    let mut influx_query: Vec<WriteQuery> = vec![];

    // Split the string by commas
    let values: Vec<&str> = input_string.split(',').collect();
    let time = Utc::now();

    if values.len() != 9 {
        log::error!("Split values has unexpected size of {}", values.len());
    }

    // bmp280_temp_f, bmp280_press, tmp36_temp_f, main_v, amp_v, forward, reverse

    influx_query.push(
        BMP280 {
            time,
            temperature_f: values[0].parse().unwrap(),
            pressure: values[1].parse().unwrap(),
            location: location.clone(),
        }
        .into_query("bmp280"),
    );

    influx_query.push(
        TMP36 {
            time,
            temperature_f: values[2].parse().unwrap(),
            location: location.clone(),
        }
        .into_query("tmp36"),
    );

    let main_voltage: f64 = values[3].parse().unwrap();
    let amp_voltage: f64 = values[4].parse().unwrap();
    let usb_voltage: f64 = values[5].parse().unwrap();

    influx_query.push(
        SupplyVoltage {
            time,
            main: evaluate_polynomial(&calibration.voltage_main, main_voltage),
            amplifier: evaluate_polynomial(&calibration.voltage_amp, amp_voltage),
            usb: evaluate_polynomial(&calibration.voltage_usb, usb_voltage),
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
