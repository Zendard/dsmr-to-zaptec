use esp_hal::rng::TrngError;
use mbedtls_rs::{SessionError, TlsError, sys::MbedtlsError};

#[derive(Debug, defmt::Format)]
pub enum WifiError {
    Wifi(esp_radio::wifi::WifiError),
    Trng(TrngError),
    Tls(TlsError),
    CertificateReading(SessionError),
    MbedTls(MbedtlsError),
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

impl From<TlsError> for WifiError {
    fn from(e: TlsError) -> Self {
        Self::Tls(e)
    }
}

impl From<MbedtlsError> for WifiError {
    fn from(e: MbedtlsError) -> Self {
        Self::MbedTls(e)
    }
}

impl From<SessionError> for WifiError {
    fn from(e: SessionError) -> Self {
        Self::CertificateReading(e)
    }
}
