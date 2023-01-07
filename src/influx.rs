use crate::point::Datum;

use core::time::Duration;
use influxdb2::Client;
use influxdb2::api::write::TimestampPrecision;
use influxdb2::models::DataPoint;
use log::{debug, info, warn, error};
use std::sync::mpsc::Receiver;
use std::thread;
use serde::Deserialize;
use std::thread::JoinHandle;


/// InfluxDB2 data-sink configuration
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    https: bool,
    host: String,
    port: u32,
    token: String,
    org: String,
    pub bucket: String,
}

impl Config {
    fn url(&self) -> String {
        let protocol = if self.https { "https" } else { "http" };
        format!("{}://{}:{}", protocol, self.host, self.port)
    }
}

/// Connection to the InfluxDB2 server
struct Connection {
    client: Client,
    bucket: String,
}

impl Connection {

    fn new(influxdb2_config: &Config) -> Connection {
        Connection{
            client: Client::new(
                influxdb2_config.url(),
                influxdb2_config.org.clone(),
                influxdb2_config.token.clone()),
            bucket: influxdb2_config.bucket.clone()}
    }

    #[tokio::main]
    async fn write_one_datapoint(&self, datum: Datum)
    -> Result<(), Box<dyn std::error::Error>> {

        let points = vec![
            DataPoint::builder(datum.measurement.to_string())
                .tag("device_name", datum.device_name)
                .tag("device_host", datum.device_host)
                .field("value", datum.value as f64)
                .timestamp(datum.measured_on.timestamp())
                .build()?
        ];
  
        self.client.write_with_precision(&self.bucket,
            futures::prelude::stream::iter(points),
            TimestampPrecision::Seconds).await?;

        Ok(())
    }
}

pub struct Pump;

impl Pump {

    pub fn spawn(influxdb2_config: Config,
        data_receiver: Receiver<Datum>)
    -> JoinHandle<Result<(),String>>
    {
        std::thread::spawn(move || {

            let mut connection = Connection::new(&influxdb2_config);
            let mut successful_connection_confirmed = false;
            loop {
                let datum = data_receiver.recv()
                    .expect("internal error, \
                    data not sent between threads");

                match connection.write_one_datapoint(datum) {
                    
                    Ok(_) => {
                        if !successful_connection_confirmed {
                            info!("Connection to InfluxDB2 established.");
                            successful_connection_confirmed = true;
                        }
                    },

                    Err(err) => {
                        warn!("We will have to reconnect in \
                            5 seconds, because: {}",  err.to_string());
                        thread::sleep(Duration::from_secs(5));
                        connection = Connection::new(&influxdb2_config);
                        successful_connection_confirmed = false;
                    }
                }
            }
        })
    }
}
