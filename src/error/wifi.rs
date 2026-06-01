use esp_hal::rng::TrngError;
use esp_radio::InitializationError;

#[derive(Debug, defmt::Format)]
pub enum WifiError {
    ControllerInit(InitializationError),
    Wifi(esp_radio::wifi::WifiError),
    Trng(TrngError),
}
impl From<InitializationError> for WifiError {
    fn from(e: InitializationError) -> Self {
        Self::ControllerInit(e)
    }
}

impl From<esp_radio::wifi::WifiError> for WifiError {
    fn from(e: esp_radio::wifi::WifiError) -> Self {
        Self::Wifi(e)
    }
}

impl From<TrngError> for WifiError {
    fn from(e: TrngError) -> Self {
        Self::Trng(e)
    }
}
