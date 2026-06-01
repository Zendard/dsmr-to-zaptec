#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(test)]
#[embedded_test::tests(executor = esp_rtos::embassy::Executor::new())]
mod tests {
    use defmt::{error, info};
    use dsmr_to_zaptec::led::RGBLED;
    use embassy_net::StackResources;
    use embassy_time::Timer;
    use esp_hal::{clock::CpuClock, peripherals::WIFI, rng::TrngSource, timer::timg::TimerGroup};
    use smart_leds::colors;

    macro_rules! mk_static {
        ($t:ty) => {{
            static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
            STATIC_CELL.uninit()
        }};
        ($t:ty,$val:expr) => {{ mk_static!($t).write($val) }};
    }

    struct Init<'a> {
        wifi: WIFI<'a>,
        led: RGBLED<'a>,
        spawner: embassy_executor::Spawner,
        stack_resources: &'a mut StackResources<3>,
    }

    #[init]
    async fn init() -> Init<'static> {
        info!("Starting...");

        rtt_target::rtt_init_defmt!();

        let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        let peripherals = esp_hal::init(config);

        esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

        let timg0 = TimerGroup::new(peripherals.TIMG0);
        let sw_interrupt =
            esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

        let _trng_src = TrngSource::new(peripherals.RNG, peripherals.ADC1);

        let stack_resources = mk_static!(StackResources<3>, StackResources::new());

        let spawner = unsafe { embassy_executor::Spawner::for_current_executor() }.await;
        info!("Embassy initialized!");

        let led = RGBLED::new(peripherals.GPIO8, peripherals.RMT).await;
        if let Err(e) = &led {
            error!("RGBLED error: {}", e);
        }
        let led = led.unwrap();

        Init {
            wifi: peripherals.WIFI,
            led,
            stack_resources,
            spawner,
        }
    }

    #[test]
    async fn test_led(init: Init<'static>) {
        let mut led = init.led;
        led.set_color(colors::RED).await.unwrap();
        Timer::after_secs(2).await;
        led.set_color(colors::GREEN).await.unwrap();
        Timer::after_secs(2).await;
        led.set_color(colors::BLUE).await.unwrap();
        Timer::after_secs(2).await;
        led.set_color(colors::WHITE).await.unwrap();
        Timer::after_secs(2).await;
        led.set_color(colors::BLACK).await.unwrap();
    }
}
