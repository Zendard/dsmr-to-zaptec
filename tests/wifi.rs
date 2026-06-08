#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(test)]
#[embedded_test::tests(executor=esp_rtos::embassy::Executor::new())]
mod tests {
    use defmt::{error, info};
    use dsmr_to_zaptec::*;
    use embassy_net::StackResources;
    use esp_hal::{clock::CpuClock, peripherals::WIFI, rng::TrngSource, timer::timg::TimerGroup};
    use mbedtls_rs::sys::hook::backend::esp::EspAccel;

    macro_rules! mk_static {
        ($t:ty) => {{
            static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
            STATIC_CELL.uninit()
        }};
        ($t:ty,$val:expr) => {{ mk_static!($t).write($val) }};
    }

    struct Init<'a> {
        wifi: WIFI<'a>,
        spawner: embassy_executor::Spawner,
        stack_resources: &'a mut StackResources<3>,
        accel: EspAccel<'a>,
    }

    #[init]
    async fn init() -> Init<'static> {
        let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        let peripherals = esp_hal::init(config);

        esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: HEAP_SIZE);
        esp_alloc::heap_allocator!( size: 256*1024 - HEAP_SIZE);

        let timg0 = TimerGroup::new(peripherals.TIMG0);
        let sw_interrupt =
            esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

        let _trng_src = TrngSource::new(peripherals.RNG, peripherals.ADC1);

        let accel = EspAccel::new(peripherals.SHA, peripherals.RSA);

        let stack_resources = mk_static!(StackResources<3>, StackResources::new());

        let spawner = unsafe { embassy_executor::Spawner::for_current_executor() }.await;
        rtt_target::rtt_init_defmt!();
        info!("Embassy initialized!");
        info!("Remaining heap memory: {}", esp_alloc::HEAP.stats());

        Init {
            wifi: peripherals.WIFI,
            spawner,
            stack_resources,
            accel,
        }
    }

    #[test]
    async fn init_wifi(init: Init<'static>) {
        wifi::init_wifi(init.wifi, init.stack_resources, init.spawner)
            .await
            .unwrap();
    }

    #[test]
    async fn http_request(mut init: Init<'static>) {
        let https_client = wifi::init_wifi(init.wifi, init.stack_resources, init.spawner).await;
        if let Err(ref e) = https_client {
            error!("Error while creating client: {}", e)
        }
        let mut https_client = https_client.unwrap();

        let _accel_queue = init.accel.start();

        let mut rx_buf = [0u8; 1024];

        info!("Remaining heap memory: {}", esp_alloc::HEAP.stats());
        let request = https_client
            .request(
                reqwless::request::Method::GET,
                "https://api.zaptec.com/oauth/token",
            )
            .await;
        info!("Remaining heap memory: {}", esp_alloc::HEAP.stats());

        if let Err(e) = &request {
            defmt::error!("Error while creating request: {}", e);
        }
        let mut request = request.unwrap();

        let response = request.send(&mut rx_buf).await;
        if let Err(ref e) = response {
            error!("Error while sending request: {}", e)
        }
        let response = response.unwrap();

        info!("Status code: {}", response.status);
    }
}
