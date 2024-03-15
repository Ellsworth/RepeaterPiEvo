mod sensor_board;

use influxdb::{Client, WriteQuery};
use log::{error, info};

// TODO: Investigate removing this. This lets us run the async fn's as blocking...
use tokio::runtime;

fn main() {
    env_logger::init();
    info!(
        "Starting RepeaterPi Evo v{} by Erich Ellsworth, KG5KEY.",
        env!("CARGO_PKG_VERSION")
    );
    info!("This is free software licensed under the GNU General Public License v2.");

    let config_data = sensor_board::load_config("config.toml".to_string());

    let client = Client::new(
        config_data.influxdb.endpoint,
        config_data.influxdb.database_name,
    )
    .with_token(config_data.influxdb.token);

    // Idea: Don't modify sensor_readings, but instead nuke the vec when we're done.
    let mut sensor_readings: Vec<WriteQuery>;

    let rt = runtime::Runtime::new().unwrap();

    sensor_readings = sensor_board::splice_sensor_readings(
        "kg5key".into(),
        "0.0,1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0",
    );

    println!("{sensor_readings:?}");

    match rt.block_on(sensor_board::send_sensor_data(
        client,
        sensor_readings.clone(),
    )) {
        Ok(()) => {
            info!("Successfully uploaded data to InfluxDB.");
            sensor_readings.clear();
        }
        Err(err) => {
            error!("{}", err);
        }
    }

    assert!(sensor_readings.is_empty());
}
