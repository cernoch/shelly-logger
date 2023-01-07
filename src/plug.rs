use crate::point::Datum;
use crate::point::Measurement::*;
use chrono::{NaiveDateTime, Timelike};
use log::{debug, info, warn, error};
use serde::Deserialize;
use std::time::Duration;
use std::thread::JoinHandle;
use std::sync::mpsc::Sender;

/// Configuration of 1 Shelly Plug (S) device
#[derive(Deserialize, Debug, Clone)]
pub struct Config {

    /// Name of this device
    pub name: String,

    /// Host-name or IP of the device
    pub host: String,

    /// Interval between measurements of instantaneous power
    pub instantaneous_meter_interval_in_s: i32,
}

impl Config {

    /// URL of the (only) meter endpoint
    pub fn meter_endpoint_url(&self) -> String {
        format!("http://{}/meter/0", self.host)
    }

    /// Interval between measurements of instantaneous power
    pub fn instantaneous_meter_interval(&self) -> Option<Duration> {
        if self.instantaneous_meter_interval_in_s < 0 {
            None
        } else {
            Some(Duration::from_secs(self.instantaneous_meter_interval_in_s as u64))
        }
    }
}


/// Response from the Shelly Plug's "/meter/0" endpoint
#[derive(Deserialize)]
pub struct Measurement {
    /// Current real AC power being drawn, in Watts
    power: f32,
    /// Whether power metering self-checks OK
    is_valid: bool,
    /// Value in Watts, on which an overpower condition is detected
    #[allow(dead_code)]
    overpower: f32,
    /// Timestamp of the last energy counter value, with the applied timezone
    timestamp: i64,
    /// Energy counter value for the last 3 round minutes in Watt-minute
    counters: Vec<f32>,
    /// Total energy consumed by the attached electrical appliance in Watt-minute
    total: f32,
}

impl Measurement {

    /// Local time on the remote device
    pub fn local_device_time(&self) -> NaiveDateTime {
        NaiveDateTime::from_timestamp_opt(self.timestamp, 0)
                .expect("Shelly plug's time is not a UNIX time-stamp")
    }

    /// Duration till the next update of the 'counters' variable
    pub fn time_to_next_update(&self) -> Duration {
        let time = self.local_device_time();
        let seconds: u64 = time.second() as u64;
        let millis: u64 = time.nanosecond() as u64 / 1000;
        return Duration::from_secs(60) // time till next minute;
             - Duration::from_secs(seconds) // elapsed in the ...
             - Duration::from_millis(millis) // ... current minute;
             + Duration::from_secs(10) // some slack for time offsets
    }

    // Instantaneous power consumption
    pub fn instantaneous_consumption_in_w(&self) -> f32 {
        self.power
    }

    /// Consumption during the last 1 round minute
    pub fn last_minute_consumption_in_wh(&self) -> f32 {
        let value_in_ws: f32 = *self.counters.first()
            .expect("The 'counters' array is expected to have 3 entries");
        value_in_ws / 60.0
    }

    /// Consumption since the plug has restarted
    pub fn consumption_since_reboot_in_wh(&self) -> f32 {
        self.total / 60.0
    }
}

/// Measurement was not possible
enum MeterError {
    Recoverable(Duration),
    Unrecoverable(String)
}

// Meter measures the power consumption via a HTTP request
struct Meter {

    config: Config,

    timeout: Duration,
}

impl Meter {

    /// Create a new meter
    pub fn new(shelly_plug_config: &Config,
        network_timeout: Duration) -> Meter
    {
        Meter {
            config: shelly_plug_config.clone(),
            timeout: network_timeout,
        }
    }

    /// Parse the measurement from the HTTP response
    fn parse_http_response(&self, response: ureq::Response) -> Result<Measurement,MeterError>
    {
        let message: Measurement = match response.into_json() {
            Ok(parsed) => parsed,
            Err(_error) => {
                return Err(MeterError::Unrecoverable(format!(
                    "{} did not return JSON with the expected grammar. \
                    Measurements are stopped.", self.config.host)));
            }
        };

        let time_measured = chrono::Utc::now();
        let device_local_time = message.local_device_time();
        debug!("{} reports local time {}, server local time is {}, offset is {}ms",
            self.config.host, device_local_time, time_measured.naive_local(),
            (device_local_time - time_measured.naive_local()).num_milliseconds(),
        );

        Ok(message)
    }

    pub fn measure(&self) -> Result<Measurement,MeterError> {
        let url = self.config.meter_endpoint_url();
        match ureq::get(&url).timeout(self.timeout).call() {

            Ok(http_response) => {
                let message = self.parse_http_response(http_response)?;

                debug!("{} \
                        instant={:.2}W \
                        last_min={:.2}Wh \
                        since_reboot={:.1}Wh",
                    self.config.host,
                    message.instantaneous_consumption_in_w(),
                    message.last_minute_consumption_in_wh(),
                    message.consumption_since_reboot_in_wh(),
                );

                if message.is_valid {
                    Ok(message)
                } else {
                    error!("{} last measurement was invalid; \
                        retrying in 10 minutes", self.config.host);
                    Err(MeterError::Recoverable(Duration::from_secs(600)))
                }
            }

            Err(ureq::Error::Status(status, response)) => {
                warn!("{} responded with HTTP status \
                    {} {}; retrying in 10 minutes (GET {})",
                    self.config.host, status, response.status_text(), url);
                Err(MeterError::Recoverable(Duration::from_secs(600)))
            }

            Err(ureq::Error::Transport(err)) => {
                warn!("{} not connected; \
                    retrying in 1 minute ({})",
                    self.config.host, err.to_string() );
                Err(MeterError::Recoverable(Duration::from_secs(60)))
            }
        }
    }
}

/// Measure the cumulative consumption over the last minute
pub struct MinuteMeter;
impl MinuteMeter {

    pub fn spawn(
        shelly_plug_config: &Config,
        network_timeout: Duration,
        data_sender: Sender<Datum>)
    -> JoinHandle<Result<(),String>>
    {
        let meter = Meter::new(&shelly_plug_config, network_timeout);
        std::thread::spawn(move || {
            loop {
                let sleep_duration = match meter.measure() {
                    Ok(m) => {

                        let d1 = Datum{
                            measured_on: chrono::Utc::now(),
                            measurement: last_minute_consumption_in_wh,
                            device_name: meter.config.name.clone(),
                            device_host: meter.config.host.clone(),
                            value: m.last_minute_consumption_in_wh(),
                        };

                        let d2 = Datum{
                            measured_on: chrono::Utc::now(),
                            measurement: consumption_since_reboot_in_wh,
                            device_name: meter.config.name.clone(),
                            device_host: meter.config.host.clone(),
                            value: m.consumption_since_reboot_in_wh(),
                        };

                        if data_sender.send(d1).is_err() || data_sender.send(d2).is_err() {
                            debug!("channel to the DB thread closed, stopping");
                            return Ok(());
                        }

                        // Sleep until the next minute
                        m.time_to_next_update()
                    },
                    Err(MeterError::Recoverable(sleep_time)) => sleep_time,
                    Err(MeterError::Unrecoverable(message)) => return Err(message),
                };

                debug!("meter thread is going to sleep for {}ms",
                    sleep_duration.as_millis()); 
                std::thread::sleep(sleep_duration);
            }
        })
    }
}

/// Measure the instantaneous consumption
pub struct InstantaneousMeter;
impl InstantaneousMeter {

    /// Spawn the metering thread and return its handle
    pub fn spawn(
        shelly_plug_config: &Config,
        network_timeout: Duration,
        data_sender: Sender<Datum>)
    -> Option<JoinHandle<Result<(),String>>>
    {
        shelly_plug_config.instantaneous_meter_interval().map_or_else(
            || {
                info!("{} will not measure instantaneous consumption \
                    (instantaneous_meter_interval_in_s < 0)",
                    shelly_plug_config.host);
                return None;
            },           

            |instantaneous_meter_interval| {
                let meter = Meter::new(&shelly_plug_config, network_timeout);
                Some(std::thread::spawn(move || {
                    loop {
                        let sleep_duration = match meter.measure() {
                            Ok(m) => { 
                                let datum = Datum{
                                    measured_on: chrono::Utc::now(),
                                    measurement: instantaneous_consumption_in_w,
                                    device_name: meter.config.name.clone(),
                                    device_host: meter.config.host.clone(),
                                    value: m.instantaneous_consumption_in_w(),
                                };
                            
                                if data_sender.send(datum).is_err() {
                                    debug!("channel to the DB thread closed, stopping");
                                    return Ok(());
                                }
                            
                                // sleep according to the config file
                                instantaneous_meter_interval
                            },

                            // error prescribes sleep duration
                            Err(MeterError::Recoverable(sleep_time)) => sleep_time,

                            Err(MeterError::Unrecoverable(message)) => return Err(message),
                        };

                        debug!("meter thread is going to sleep for {}ms",
                                sleep_duration.as_millis());
                        std::thread::sleep(sleep_duration);
                    }
                }))    
            })
    }
}
