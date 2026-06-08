#[derive(Debug, defmt::Format)]
pub enum RGBLEDError {
    RmtError(esp_hal::rmt::Error),
    LedAdapterError(esp_hal_smartled::LedAdapterError),
    ConfigError(esp_hal::rmt::ConfigError),
}

impl From<esp_hal::rmt::Error> for RGBLEDError {
    fn from(e: esp_hal::rmt::Error) -> Self {
        Self::RmtError(e)
    }
}

impl From<esp_hal_smartled::LedAdapterError> for RGBLEDError {
    fn from(e: esp_hal_smartled::LedAdapterError) -> Self {
        Self::LedAdapterError(e)
    }
}

impl From<esp_hal::rmt::ConfigError> for RGBLEDError {
    fn from(e: esp_hal::rmt::ConfigError) -> Self {
        Self::ConfigError(e)
    }
}
