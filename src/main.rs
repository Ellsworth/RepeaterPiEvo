mod sensor_board;

use influxdb::{Client, WriteQuery};
use log::{info, error};

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

    let client =     Client::new(
        config_data.influxdb.endpoint,
        config_data.influxdb.database_name,
    )
    .with_token(config_data.influxdb.token);

    let mut sensor_readings: Vec<WriteQuery> = vec![];

    let rt = runtime::Runtime::new().unwrap();

    // Call the asynchronous function in a blocking manner because reasons...
    sensor_board::read_sensor_data(&mut sensor_readings);

    match rt.block_on(sensor_board::send_sensor_data(client, &sensor_readings)) {
        Ok(()) => {
            info!("Successfully uploaded data to InfluxDB.");
            sensor_readings.clear();
        }
        Err(err) => {
            error!("{}", err);
        }
    }

    println!("{:?}", sensor_readings);

    sensor_board::sensorboard_parse();
}
