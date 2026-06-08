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
    self, Interface, Interfaces, WifiController, scan::ScanConfig, sta::StationConfig,
};
use reqwless::{Certificate, TlsVersion, client::HttpClient, client::TlsConfig};

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");
const CERT_BYTES: &[u8] = include_bytes!(env!("CERT_FILE"));

pub async fn init_wifi(
    wifi: WIFI<'static>,
    stack_resources: &'static mut StackResources<3>,
    spawner: embassy_executor::Spawner,
) -> Result<ReqwlessClient, WifiError> {
    let (controller, interface) = esp_radio::wifi::new(wifi, wifi::ControllerConfig::default())?;

    let (stack, runner) = configure_controller(interface, stack_resources).await?;
    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());
    stack.wait_config_up().await;
    stack.wait_link_up().await;
    info!("Network stack: {:?}", stack.config_v4());
    Ok(make_reqwless_client(stack).await?)
}

async fn configure_controller(
    interface: Interfaces<'static>,
    stack_resources: &'static mut StackResources<3>,
) -> Result<(Stack<'static>, Runner<'static, Interface<'static>>), WifiError> {
    info!("Configuring wifi...");

    let dhcp_config = embassy_net::Config::dhcpv4(Default::default());

    let trng = Trng::try_new()?;

    let seed = (trng.random() as u64) << 32 | trng.random() as u64;

    let (stack, runner) = embassy_net::new(interface.station, dhcp_config, stack_resources, seed);

    info!("Wifi configured");
    Ok((stack, runner))
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    info!("Starting connection task...");

    loop {
        if controller.is_connected() {
            info!("Connected to wifi");
            controller.wait_for_disconnect_async().await.unwrap();
            Timer::after_secs(5).await;
        } else {
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
    let config = wifi::Config::Station(
        StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.into()),
    );
    info!("{:?}", &config);
    controller.set_config(&config)?;
    info!("Wifi started");
    Ok(())
}

async fn scan_wifi(controller: &mut WifiController<'static>) -> Result<(), WifiError> {
    info!("Scanning wifi networks...");
    let config = ScanConfig::default().with_max(10);
    let result = controller.scan_async(&config).await?;
    info!("Scanned wifi networks");
    info!("{:?}", result.as_slice());
    Ok(())
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}

pub type ReqwlessClient =
    HttpClient<'static, TcpClient<'static, 1, 4096, 4096>, DnsSocket<'static>>;
pub type ReqwlessConnection<'a> = TcpConnection<'a, 1, 4096, 4096>;

async fn make_reqwless_client(stack: Stack<'static>) -> Result<ReqwlessClient, WifiError> {
    info!("Building Reqwless client");
    let state = mk_static!(TcpClientState<1,4096,4096>, TcpClientState::<1, 4096, 4096>::new());
    let tcp_client = mk_static!(TcpClient<1,4096,4096>,TcpClient::new(stack, state));
    let trng = mk_static!(Trng, Trng::try_new()?);
    let mbedtls_instance = mk_static!(mbedtls_rs::Tls, mbedtls_rs::Tls::new(trng)?);
    mbedtls_instance.set_debug(6);
    let dns_socket = mk_static!(DnsSocket, DnsSocket::new(stack));

    let cert = Certificate::new_no_copy(CERT_BYTES)?;

    let tls_config = TlsConfig::new(TlsVersion::Tls1_2, cert, None, mbedtls_instance.reference());

    Ok(HttpClient::new_with_tls(tcp_client, dns_socket, tls_config))
}
