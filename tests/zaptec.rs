#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

#[cfg(test)]
#[embedded_test::tests(executor=esp_rtos::embassy::Executor::new())]
mod tests {
    use defmt::{error, info};
    use dsmr_to_zaptec::{
        HEAP_SIZE, RECLAIMED_RAM,
        wifi::{ReqwlessClient, init_wifi},
        zaptec::ZaptecClient,
    };
    use embassy_net::StackResources;
    use esp_hal::{clock::CpuClock, rng::TrngSource, timer::timg::TimerGroup};
    use mbedtls_rs::sys::hook::backend::esp::EspAccel;

    macro_rules! mk_static {
        ($t:ty) => {{
            static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
            STATIC_CELL.uninit()
        }};
        ($t:ty,$val:expr) => {{ mk_static!($t).write($val) }};
    }

    struct Init<'a> {
        spawner: embassy_executor::Spawner,
        accel: EspAccel<'a>,
        https_requester: ReqwlessClient,
    }

    #[init]
    async fn init() -> Init<'static> {
        let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
        let peripherals = esp_hal::init(config);

        esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: RECLAIMED_RAM);
        esp_alloc::heap_allocator!( size: HEAP_SIZE - RECLAIMED_RAM);

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

        let https_requester = init_wifi(peripherals.WIFI, stack_resources, spawner)
            .await
            .unwrap();

        Init {
            spawner,
            accel,
            https_requester,
        }
    }

    #[test]
    async fn fetch_token(mut init: Init<'static>) {
        let _accel_queue = init.accel.start();

        let client = ZaptecClient::new(init.https_requester).await;
        if let Err(ref e) = client {
            error!("Error making zaptec client: {}", e);
        }
        let client = client.unwrap();

        info!("Got token: {}", client.token())
    }
}
