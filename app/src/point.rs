use chrono::DateTime;
use chrono::Utc;

#[allow(non_camel_case_types)]
pub enum Measurement {
    last_minute_consumption_in_wh,
    instantaneous_consumption_in_w,
    consumption_since_reboot_in_wh,
}

impl std::fmt::Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Measurement::last_minute_consumption_in_wh =>
                write!(f, "last_minute_consumption_in_wh"),
            Measurement::instantaneous_consumption_in_w =>
                write!(f, "instantaneous_consumption_in_w"),
            Measurement::consumption_since_reboot_in_wh =>
                write!(f, "consumption_since_reboot_in_wh"),
        }
    }
}

pub struct Datum {
    pub measured_on: DateTime<Utc>,    
    pub measurement: Measurement,
    pub device_name: String,
    pub device_host: String,
    pub value: f32,
}
