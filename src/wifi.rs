use crate::mk_static;
use defmt::info;
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::Timer;
use esp_hal::{peripherals::WIFI, rng::Trng};
use esp_radio::wifi::{
    ClientConfig, Interfaces, ModeConfig, ScanConfig, WifiController, WifiDevice, WifiEvent,
};

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");

pub async fn init_wifi(
    wifi: WIFI<'static>,
    stack_resources: &'static mut StackResources<3>,
    spawner: embassy_executor::Spawner,
) {
    let device = mk_static!(esp_radio::Controller, esp_radio::init().unwrap());
    let (controller, interface) =
        esp_radio::wifi::new(device, wifi, esp_radio::wifi::Config::default()).unwrap();

    let (stack, runner) = configure_controller(interface, stack_resources).await;
    spawner.spawn(connection(controller, spawner)).ok();
    spawner.spawn(net_task(runner)).ok();
}

async fn configure_controller(
    interface: Interfaces<'static>,
    stack_resources: &'static mut StackResources<3>,
) -> (Stack<'static>, Runner<'static, WifiDevice<'static>>) {
    info!("Configuring wifi...");

    let dhcp_config = embassy_net::Config::dhcpv4(Default::default());

    let trng = Trng::try_new().unwrap();

    let seed = (trng.random() as u64) << 32 | trng.random() as u64;

    let (stack, runner) = embassy_net::new(interface.sta, dhcp_config, stack_resources, seed);

    info!("Wifi configured");
    (stack, runner)
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>, spawner: embassy_executor::Spawner) {
    info!("Starting connection task...");

    loop {
        if matches!(controller.is_connected(), Ok(true)) {
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after_secs(5).await;
        }

        if !matches!(controller.is_started(), Ok(true)) {
            start_controller(&mut controller).await;
            scan_wifi(&mut controller).await;

            match controller.connect_async().await {
                Ok(_) => info!("Wifi connected"),
                Err(e) => {
                    info!("Failed to connect to wifi: {}", e);
                    Timer::after_secs(5).await;
                }
            }
        }
    }
}

async fn start_controller(controller: &mut WifiController<'static>) {
    let config = ModeConfig::Client(
        ClientConfig::default()
            .with_ssid(SSID.into())
            .with_password(PASSWORD.into()),
    );
    controller.set_config(&config).unwrap();
    info!("Starting wifi...");
    controller.start_async().await.unwrap();
    info!("Wifi started");
}

async fn scan_wifi(controller: &mut WifiController<'static>) {
    info!("Scanning wifi networks...");
    let config = ScanConfig::default().with_ssid(SSID).with_max(10);
    let result = controller.scan_with_config_async(config).await.unwrap();
    info!("Scanned wifi networks");
    info!("{:?}", result[0]);
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
