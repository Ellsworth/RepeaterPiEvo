#![warn(rust_2018_idioms)]

use config::CalibrationConfig;
use futures::stream::StreamExt;
use std::{io, str};
use tokio_util::codec::{Decoder, Encoder};

use bytes::BytesMut;
use tokio_serial::SerialPortBuilderExt;

mod config;
mod sensor_board;

struct LineCodec;

impl Decoder for LineCodec {
    type Item = Vec<influxdb::WriteQuery>;
    type Error = io::Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
        //calibration: CalibrationConfig,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let newline = src.as_ref().iter().position(|b| *b == b'\n');
        if let Some(n) = newline {
            let line = src.split_to(n + 1);
            return match str::from_utf8(line.as_ref()) {
                Ok(s) => {
                    //Ok(Some(s.to_string()))

                    let config_data = config::load_config("config.toml".to_string());

                    Ok(Some(sensor_board::splice_sensor_readings(
                        "kg5key".into(),
                        s,
                        &config_data.calibration,
                    )))
                }
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Invalid String")),
            };
        }
        Ok(None)
    }
}

impl Encoder<String> for LineCodec {
    type Error = io::Error;

    fn encode(&mut self, _item: String, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> tokio_serial::Result<()> {
    let tty_path: String = "/dev/ttyACM0".into();

    let mut port = tokio_serial::new(tty_path, 9600).open_native_async()?;

    #[cfg(unix)]
    port.set_exclusive(false)
        .expect("Unable to set serial port exclusive to false");

    let mut reader = LineCodec.framed(port);

    while let Some(line_result) = reader.next().await {
        let line = line_result.expect("Failed to read line");
        println!("RX: {:?}", line);
    }
    Ok(())
}
