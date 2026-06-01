use crate::{error::led::RGBLEDError, mk_static};
use esp_hal::{
    gpio::OutputPin,
    peripherals::RMT,
    rmt::{PulseCode, Rmt},
    time::Rate,
};
use esp_hal_smartled::{SmartLedsAdapterAsync, smart_led_buffer};
use smart_leds::{RGB, SmartLedsWriteAsync};

#[derive(defmt::Format)]
pub struct RGBLED<'a> {
    adapter: esp_hal_smartled::SmartLedsAdapterAsync<'a, 25>,
}

impl<'a> RGBLED<'a> {
    pub async fn new<T: OutputPin + 'a>(gpio_pin: T, rmt: RMT<'a>) -> Result<Self, RGBLEDError> {
        let rmt = Rmt::new(rmt, Rate::from_mhz(80))?.into_async();

        let buffer = smart_led_buffer!(1);
        let buffer_ref = mk_static!([PulseCode; 25], buffer);

        let adapter = SmartLedsAdapterAsync::new_with_color(rmt.channel0, gpio_pin, buffer_ref);
        Ok(Self { adapter })
    }

    pub async fn set_color<T: Into<RGB<u8>>>(&mut self, color: T) -> Result<(), RGBLEDError> {
        self.adapter.write([color.into()]).await?;
        Ok(())
    }
}
