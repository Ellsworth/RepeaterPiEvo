#![deny(clippy::all, clippy::pedantic, clippy::allow_attributes_without_reason)]

use futures::StreamExt;
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Decoder;

mod config;
mod cpu_stats;
mod sensor_board;
mod serial_reader;

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

    let client = influxdb::Client::new(
        config_data.influxdb.endpoint,
        config_data.influxdb.database_name,
    )
    .with_token(config_data.influxdb.token);

    let tty_path = config_data.serial.port;
    let baud = config_data.serial.baud;

    log::info!("Opening serial port '{}' at {} baud.", tty_path, baud);

    let port = tokio_serial::new(tty_path, baud).open_native_async()?;
    let mut reader = serial_reader::LineCodec.framed(port);

    while let Some(line_result) = reader.next().await {
        let line = line_result.expect("Failed to read line");
        log::info!("Received data from sensorboard: {:?}", line);

        let sensor_readings = sensor_board::splice_sensor_readings(
            config_data.influxdb.site_name.clone(),
            &line,
            &config_data.calibration,
        );

        // Get CPU stats.
        let cpu_readings = cpu_stats::get_cpu_stats(config_data.influxdb.site_name.clone());

        // Combine the two Vec.
        let sensor_readings = [sensor_readings, cpu_readings].concat();

        log::debug!("{:?}", sensor_readings);

        let measurement_count = sensor_readings.len();

        match client.query(sensor_readings).await {
            Ok(_) => {
                log::info!(
                    "Successfully uploaded {} measurements to InfluxDB.",
                    measurement_count
                );
            }
            Err(error) => log::error!("InfluxDB: {:}", error),
        };
    }
    Ok(())
}
