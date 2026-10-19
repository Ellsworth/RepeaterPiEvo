#![deny(clippy::all, clippy::pedantic, clippy::allow_attributes_without_reason)]

use futures::StreamExt;
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Decoder;

mod change_detector;
mod config;
mod cpu_stats;
mod sensor_board;
mod serial_reader;

async fn log_endpoint_resolution(endpoint: &str) {
    if let Ok(url) = url::Url::parse(endpoint)
        && let Some(host) = url.host_str()
    {
        let port = url.port_or_known_default().unwrap_or(443);
        let addr_str = format!("{host}:{port}");
        match tokio::net::lookup_host(&addr_str).await {
            Ok(addrs) => {
                let ips: Vec<_> = addrs.collect();
                log::info!("System DNS resolution for '{host}': {ips:?}");
            }
            Err(e) => {
                log::warn!("System DNS resolution failed for '{host}': {e}");
            }
        }
    }
}

fn create_influx_client(influx_config: &config::InfluxDB) -> influxdb::Client {
    let mut builder = reqwest::Client::builder();

    if !influx_config.resolve_ips.is_empty()
        && let Ok(url) = url::Url::parse(&influx_config.endpoint)
        && let Some(host) = url.host_str()
    {
        let port = url.port_or_known_default().unwrap_or(443);
        let socket_addrs: Vec<std::net::SocketAddr> = influx_config
            .resolve_ips
            .iter()
            .map(|ip| std::net::SocketAddr::new(*ip, port))
            .collect();

        log::info!("Overriding DNS for InfluxDB host '{host}' with static IPs: {socket_addrs:?}");
        builder = builder.resolve_to_addrs(host, &socket_addrs);
    }

    let http_client = builder
        .build()
        .expect("Failed to create reqwest HTTP client");

    influxdb::Client::new(
        influx_config.endpoint.clone(),
        influx_config.database_name.clone(),
    )
    .with_token(influx_config.token.clone())
    .with_http_client(http_client)
}

#[tokio::main]
async fn main() -> tokio_serial::Result<()> {
    env_logger::init();

    log::info!(
        "RepeaterPi Evo v{}, Copyright (C) 2024 Erich Ellsworth, KG5KEY.",
        env!("CARGO_PKG_VERSION")
    );

    log::info!(
        "This program is free software; you can redistribute it and/or \
     modify it under the terms of the GNU General Public License \
     as published by the Free Software Foundation; either version 2 \
     of the License, or (at your option) any later version."
    );

    let config_data = config::load("config.toml".to_string());

    let client = create_influx_client(&config_data.influxdb);
    log_endpoint_resolution(&config_data.influxdb.endpoint).await;

    let mut change_detector =
        change_detector::TelemetryChangeDetector::new(config_data.change_detector.clone());

    let tty_path = config_data.serial.port.clone();
    let baud = config_data.serial.baud;

    log::info!("Opening serial port '{tty_path}' at {baud} baud.");

    let port = tokio_serial::new(tty_path, baud).open_native_async()?;
    let mut reader = serial_reader::LineCodec.framed(port);

    while let Some(line_result) = reader.next().await {
        let line = line_result.expect("Failed to read line");
        log::info!("Received data from sensorboard: {line:?}");

        let cpu_temp_c = cpu_stats::get_cpu_temperature_c();
        let sample =
            match sensor_board::parse_to_sample(&line, &config_data.calibration, cpu_temp_c) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to parse sensorboard data: {e}");
                    continue;
                }
            };

        let evaluation = change_detector.evaluate_sample(&sample);
        if !evaluation.should_send {
            log::debug!("Telemetry suppressed ({})", evaluation.reason);
            continue;
        }

        let sensor_readings =
            sensor_board::sample_to_write_queries(&sample, &config_data.influxdb.site_name);

        log::debug!("{sensor_readings:?}");

        let measurement_count = sensor_readings.len();
        log::info!(
            "Telemetry change detected ({}): uploading {measurement_count} measurements to InfluxDB.",
            evaluation.reason
        );

        match client.query(sensor_readings).await {
            Ok(_) => {
                log::info!("Successfully uploaded {measurement_count} measurements to InfluxDB.");
            }
            Err(error) => {
                log::error!("InfluxDB: {error:}");
                log_endpoint_resolution(&config_data.influxdb.endpoint).await;
            }
        }
    }
    Ok(())
}
