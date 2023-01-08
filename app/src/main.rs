mod config;
mod influx;
mod plug;
mod point;

use log::{debug, warn, error};
use std::thread::JoinHandle;
use std::sync::mpsc::channel;

fn main() {
    env_logger::init();
    let app_config = config::Config::read_from_deafult_file();

    //
    let (tx, rx) = channel::<point::Datum>();

    // Spawn all meter threads!
    let mut join_handles: Vec<JoinHandle<Result<(),String>>> = vec![];
    for shelly_plug_config in &app_config.shelly_plugs {
        
        // Metering per minute
        join_handles.push(plug::MinuteMeter::spawn(
            shelly_plug_config,
            app_config.network_timeout(), 
            tx.clone()));

        // Instantaneous metering
        plug::InstantaneousMeter::spawn(
                shelly_plug_config,
                app_config.network_timeout(),
                 tx.clone())
            .map(|handle| { join_handles.push(handle) });
    }
    
    debug!("{} meter threads were started", join_handles.len());

    join_handles.push(influx::Pump::spawn(
        app_config.influxdb2.clone(), rx));

    // Wait for all threads to finish
    for join_handle in join_handles {
        match join_handle.join() {
            Ok(Ok(_)) => (),
            Ok(Err(msg)) => error!("{msg}"),
            Err(_) => warn!("some thread could not \
                be joined; internal error likely"),
        }
    }
}
