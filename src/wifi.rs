use crate::mk_static;
use core::ffi::CStr;
use defmt::info;
use embassy_net::{
    Runner, Stack, StackResources,
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
};
use embassy_time::Timer;
use esp_hal::{peripherals::WIFI, rng::Trng};
use esp_radio::wifi::{
    ClientConfig, Interfaces, ModeConfig, ScanConfig, WifiController, WifiDevice, WifiEvent,
};
use reqwless::{
    Certificate, TlsVersion,
    client::{HttpClient, TlsConfig},
};

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");
const CERT_BYTES: &[u8] = include_bytes!(env!("CERT_FILE"));

pub async fn init_wifi(
    wifi: WIFI<'static>,
    stack_resources: &'static mut StackResources<3>,
    spawner: embassy_executor::Spawner,
) -> ReqwlessClient {
    let device = mk_static!(esp_radio::Controller, esp_radio::init().unwrap());
    let (controller, interface) =
        esp_radio::wifi::new(device, wifi, esp_radio::wifi::Config::default()).unwrap();

    let (stack, runner) = configure_controller(interface, stack_resources).await;
    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();
    stack.wait_config_up().await;
    make_reqwless_client(stack).await
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
async fn connection(mut controller: WifiController<'static>) {
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
                    continue;
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

type ReqwlessClient = HttpClient<'static, TcpClient<'static, 1, 4096, 4096>, DnsSocket<'static>>;

async fn make_reqwless_client(stack: Stack<'static>) -> ReqwlessClient {
    let state = mk_static!(TcpClientState<1,4096,4096>, TcpClientState::<1, 4096, 4096>::new());
    let tcp_client = mk_static!(TcpClient<1,4096,4096>,TcpClient::new(stack, state));
    let trng = mk_static!(Trng, Trng::try_new().unwrap());
    let mbedtls_instance = mk_static!(mbedtls_rs::Tls, mbedtls_rs::Tls::new(trng).unwrap());
    let dns_socket = mk_static!(DnsSocket, DnsSocket::new(stack));

    let cstr_cert: &CStr = CStr::from_bytes_until_nul(CERT_BYTES).unwrap();

    let tls_config = TlsConfig::new(
        TlsVersion::Tls1_3,
        Certificate::new(reqwless::X509::PEM(cstr_cert)).unwrap(),
        None,
        mbedtls_instance.reference(),
    );

    HttpClient::new_with_tls(tcp_client, dns_socket, tls_config)
}
