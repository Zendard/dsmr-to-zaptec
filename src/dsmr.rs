use crate::DSMR_BUFFER_SIZE;
use defmt::info;
use dsmr5::{OBIS, ReaderError, Readout};
use embassy_time::Timer;
use esp_hal::{
    Async,
    uart::{RxError, UartRx},
};

#[embassy_executor::task]
pub async fn read_to_buffer(
    mut dsmr_data: UartRx<'static, Async>,
    stream_slice: &'static mut [u8; DSMR_BUFFER_SIZE],
) {
    loop {
        dsmr_data.read_async(stream_slice).await.unwrap();
        Timer::after_secs(1).await;
    }
}

pub struct DSMRReading {
    pub power_delivered: f64,
    pub power_received: f64,
}

pub fn parse_readout(
    readout: Option<Result<Readout, ReaderError<RxError>>>,
) -> Option<DSMRReading> {
    let readout = readout?.ok()?;
    info!("Got readout");

    let telegram = readout.to_telegram();
    let telegram = telegram.ok()?;
    info!("Parsed telegram");

    let power_received: f64 = (&telegram.objects().find_map(|i| match i {
        Ok(OBIS::PowerReceived(number)) => Some(number),
        _ => None,
    })?)
        .into();
    info!("Power received: {}", power_received);

    let power_delivered: f64 = (&telegram.objects().find_map(|i| match i {
        Ok(OBIS::PowerDelivered(number)) => Some(number),
        _ => None,
    })?)
        .into();
    info!("Power delivered: {}", power_delivered);

    Some(DSMRReading {
        power_delivered,
        power_received,
    })
}
