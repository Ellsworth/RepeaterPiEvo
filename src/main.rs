#![deny(clippy::all, clippy::pedantic, clippy::allow_attributes_without_reason)]

use futures::StreamExt;
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Decoder;

mod config;
mod sensor_board;
mod serial_reader;
mod vcgencmd_influx;

#[tokio::main]
async fn main() -> tokio_serial::Result<()> {
    env_logger::init();

    log::info!(
        "Starting RepeaterPi Evo v{} by Erich Ellsworth, KG5KEY.",
        env!("CARGO_PKG_VERSION")
    );
    log::info!("This is free software licensed under the GNU General Public License v2.");

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
        log::debug!("Received data from sensorboard: {:?}", line);

        let sensor_readings = sensor_board::splice_sensor_readings(
            config_data.influxdb.site_name.clone(),
            &line,
            &config_data.calibration,
        );

        // Get the readings from vcgencmd.
        let vcgencmd_readings =
            vcgencmd_influx::get_vcgencmd_stats(config_data.influxdb.site_name.to_string());

        // Combine the two Vec.
        let sensor_readings = [sensor_readings, vcgencmd_readings].concat();

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
