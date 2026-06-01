use crate::{error::wifi::WifiError, mk_static};
use defmt::info;
use embassy_net::{
    Runner, Stack, StackResources,
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState, TcpConnection},
};
use embassy_time::Timer;
use esp_hal::{peripherals::WIFI, rng::Trng};
use esp_radio::wifi::{
    ClientConfig, Interfaces, ModeConfig, ScanConfig, WifiController, WifiDevice, WifiEvent,
};
use reqwless::{
    // Certificate, TlsVersion,
    // client::TlsConfig,
    client::HttpClient,
};

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");
// const CERT_BYTES: &[u8] = concat!(include_str!(env!("CERT_FILE")), "\0").as_bytes();

pub async fn init_wifi(
    wifi: WIFI<'static>,
    stack_resources: &'static mut StackResources<3>,
    spawner: embassy_executor::Spawner,
) -> Result<ReqwlessClient, WifiError> {
    let device = mk_static!(esp_radio::Controller, esp_radio::init()?);
    let (controller, interface) =
        esp_radio::wifi::new(device, wifi, esp_radio::wifi::Config::default())?;

    let (stack, runner) = configure_controller(interface, stack_resources).await?;
    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();
    stack.wait_config_up().await;
    stack.wait_link_up().await;
    info!("Network stack: {:?}", stack.config_v4());
    Ok(make_reqwless_client(stack).await)
}

async fn configure_controller(
    interface: Interfaces<'static>,
    stack_resources: &'static mut StackResources<3>,
) -> Result<(Stack<'static>, Runner<'static, WifiDevice<'static>>), WifiError> {
    info!("Configuring wifi...");

    let dhcp_config = embassy_net::Config::dhcpv4(Default::default());

    let trng = Trng::try_new()?;

    let seed = (trng.random() as u64) << 32 | trng.random() as u64;

    let (stack, runner) = embassy_net::new(interface.sta, dhcp_config, stack_resources, seed);

    info!("Wifi configured");
    Ok((stack, runner))
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    info!("Starting connection task...");

    loop {
        if matches!(controller.is_connected(), Ok(true)) {
            info!("Connected to wifi");
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after_secs(5).await;
        }

        if !matches!(controller.is_started(), Ok(true)) {
            start_controller(&mut controller)
                .await
                .expect("Failed to start controller");
            scan_wifi(&mut controller)
                .await
                .expect("Failed to scan wifi");

            info!("Connecting");
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

async fn start_controller(controller: &mut WifiController<'static>) -> Result<(), WifiError> {
    let config = ModeConfig::Client(
        ClientConfig::default()
            .with_ssid(SSID.into())
            .with_password(PASSWORD.into()),
    );
    info!("{:?}", &config);
    controller.set_config(&config)?;
    info!("Starting wifi...");
    controller.start_async().await?;
    info!("Wifi started");
    Ok(())
}

async fn scan_wifi(controller: &mut WifiController<'static>) -> Result<(), WifiError> {
    info!("Scanning wifi networks...");
    let config = ScanConfig::default().with_max(10);
    let result = controller.scan_with_config_async(config).await?;
    info!("Scanned wifi networks");
    info!("{:?}", result.as_slice());
    Ok(())
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

pub type ReqwlessClient =
    HttpClient<'static, TcpClient<'static, 1, 4096, 4096>, DnsSocket<'static>>;
pub type ReqwlessConnection<'a> = TcpConnection<'a, 1, 4096, 4096>;

async fn make_reqwless_client(stack: Stack<'static>) -> ReqwlessClient {
    info!("Building Reqwless client");
    let state = mk_static!(TcpClientState<1,4096,4096>, TcpClientState::<1, 4096, 4096>::new());
    let tcp_client = mk_static!(TcpClient<1,4096,4096>,TcpClient::new(stack, state));
    // let trng = mk_static!(Trng, Trng::try_new()?);
    // let mbedtls_instance = mk_static!(mbedtls_rs::Tls, mbedtls_rs::Tls::new(trng)?);
    let dns_socket = mk_static!(DnsSocket, DnsSocket::new(stack));

    // let cstr_cert: &CStr = CStr::from_bytes_with_nul(CERT_BYTES)?;

    // let tls_config = TlsConfig::new(
    //     TlsVersion::Tls1_3,
    //     Certificate::new(reqwless::X509::PEM(cstr_cert))?,
    //     None,
    //     mbedtls_instance.reference(),
    // );
    //
    // HttpClient::new_with_tls(tcp_client, dns_socket, tls_config)

    HttpClient::new(tcp_client, dns_socket)
}
