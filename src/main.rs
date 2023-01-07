mod config;
mod plug;

use log::{debug, warn, error};
use std::thread::JoinHandle;

fn main() {
    env_logger::init();
    let config = config::Config::read_from_deafult_file();

    // Spawn all meter threads!
    let mut join_handles: Vec<JoinHandle<Result<(),String>>> = vec![];
    for shelly_plug_config in &config.shelly_plugs {
        
        // Metering per minute
        join_handles.push(plug::MinuteMeter::spawn(
            shelly_plug_config, config.timeout()));

        // Instantaneous metering
        plug::InstantaneousMeter::spawn(
                shelly_plug_config, config.timeout())
            .map(|handle| { join_handles.push(handle) });
    }
    
    debug!("{} meter threads were started", join_handles.len());

    // Wait for all threads to finish
    for join_handle in join_handles {
        match join_handle.join() {
            Ok(Ok(_)) => (),
            Ok(Err(msg)) => error!("metering was interrupted: {}", msg),
            Err(_) => warn!(" some meter thread could not be joined;\
                probably due to some internal/OS error"),
        }
    }
}
