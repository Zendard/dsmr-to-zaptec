use crate::error::{led::RGBLEDError, wifi::WifiError};

pub mod led;
pub mod wifi;

pub enum DSMRToZaptecError {
    Wifi(WifiError),
    RGBLED(RGBLEDError),
}
