#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use dsmr_to_zaptec::dsmr::parse_readout;
use dsmr_to_zaptec::zaptec::ZaptecRequester;
use dsmr_to_zaptec::{DSMR_BUFFER_SIZE, calc_voltage};
use embassy_executor::Spawner;
use embassy_net::StackResources;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::rng::TrngSource;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart;
use panic_rtt_target as _;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

macro_rules! mk_static {
    ($t:ty) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        STATIC_CELL.uninit()
    }};
    ($t:ty,$val:expr) => {{ mk_static!($t).write($val) }};
}

const DSMR_BAUD_RATE: u32 = 115200;

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

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

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    let reqwless_client =
        dsmr_to_zaptec::wifi::init_wifi(peripherals.WIFI, stack_resources, spawner)
            .await
            .expect("Failed to create reqwless client");
    let zaptec_client = ZaptecRequester::new(reqwless_client)
        .await
        .expect("Failed to create zaptec requester");

    info!("Initializing DSMR stream...");
    // GPIO1: DSMR data request
    // GPIO2: DSMR data
    let mut dsmr_data_request_line =
        Output::new(peripherals.GPIO1, Level::Low, OutputConfig::default());

    let dsmr_data = uart::UartRx::new(
        peripherals.UART0,
        uart::Config::default().with_baudrate(DSMR_BAUD_RATE),
    )
    .unwrap()
    .with_rx(peripherals.GPIO2)
    .into_async();

    let dsmr_data_slice = [0u8; DSMR_BUFFER_SIZE];
    let dsmr_data_iter = dsmr_data_slice.iter().map(|i| Ok::<u8, uart::RxError>(*i));

    let mut dsmr_stream = dsmr5::Reader::new(dsmr_data_iter);

    let dsmr_data_slice_ref = mk_static!([u8; DSMR_BUFFER_SIZE], dsmr_data_slice);

    info!("UART initialized");

    spawner
        .spawn(dsmr_to_zaptec::dsmr::read_to_buffer(
            dsmr_data,
            dsmr_data_slice_ref,
        ))
        .unwrap();

    info!("Starting main loop...");
    loop {
        dsmr_data_request_line.set_high();
        let readout = dsmr_stream.next();
        dsmr_data_request_line.set_low();

        let dsmr_reading = parse_readout(readout);
        if dsmr_reading.is_none() {
            info!("Couldn't parse readout");
            continue;
        }
        let dsmr_reading = dsmr_reading.unwrap();

        let zaptec_settings = calc_voltage(dsmr_reading);
    }
}
