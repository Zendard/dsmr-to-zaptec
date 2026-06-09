use crate::error::{led::RGBLEDError, wifi::WifiError, zaptec::ZaptecError};

pub mod led;
pub mod wifi;
pub mod zaptec;

pub enum DSMRToZaptecError {
    Wifi(WifiError),
    RGBLED(RGBLEDError),
    Zaptec(ZaptecError),
}
