use crate::DSMR_BUFFER_SIZE;
use embassy_time::Timer;
use esp_hal::{Async, uart::UartRx};

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
