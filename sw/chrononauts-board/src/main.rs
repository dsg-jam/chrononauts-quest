use crate::captive::CaptivePortal;
use dns::*;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::prelude::Peripherals,
    http::server::Configuration,
    ipv4::{self, Mask, RouterConfiguration, Subnet},
    log::EspLogger,
    netif::{EspNetif, NetifConfiguration, NetifStack},
    nvs::EspDefaultNvsPartition,
    sys::{self, EspError},
    wifi::{
        self, AccessPointConfiguration, AccessPointInfo, ClientConfiguration, EspWifi, WifiDriver,
    },
};
use std::{
    net::Ipv4Addr,
    sync::{mpsc, Arc, Condvar, Mutex},
    thread::{self, sleep},
    time::Duration,
};

mod captive;
mod dns;
mod server;

pub const IP_ADDRESS: Ipv4Addr = Ipv4Addr::new(10, 9, 1, 1);

#[derive(Debug)]
enum WifiRunner {
    ChangeWifi(WifiCreds),
    GetWifi,
}

#[derive(Debug)]
struct WifiCreds {
    ssid: String,
    wpa2: String,
}

fn main() -> Result<(), EspError> {
    unsafe {
        sys::nvs_flash_init();
    }
    sys::link_patches();
    EspLogger::initialize_default();

    let event_loop = EspSystemEventLoop::take()?;
    let peripherals = Peripherals::take()?;
    let (wifi_runner_tx, wifi_runner_rx) = mpsc::channel::<WifiRunner>();

    let wifi_update_cond = Arc::new((Mutex::new(false), Condvar::new()));

    let wifi_nets_store = Arc::new(Mutex::new(Vec::<AccessPointInfo>::new()));

    log::info!("Starting Wi-Fi...");
    let wifi_driver = WifiDriver::new(
        peripherals.modem,
        event_loop.clone(),
        EspDefaultNvsPartition::take().ok(),
    )?;
    let mut wifi = EspWifi::wrap_all(
        wifi_driver,
        EspNetif::new(NetifStack::Sta)?,
        EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: ipv4::Configuration::Router(RouterConfiguration {
                subnet: Subnet {
                    gateway: IP_ADDRESS,
                    mask: Mask(24),
                },
                dhcp_enabled: true,
                dns: Some(IP_ADDRESS),
                secondary_dns: Some(IP_ADDRESS),
            }),
            ..NetifConfiguration::wifi_default_router()
        })?,
    )
    .expect("WiFi init failed");

    wifi.set_configuration(&wifi::Configuration::Mixed(
        ClientConfiguration {
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: env!("SSID").into(),
            password: env!("SSID_PASSWORD").into(),
            auth_method: wifi::AuthMethod::WPA2Personal,
            ..Default::default()
        },
    ))?;
    wifi.start()?;
    let wifi_nets = wifi_nets_store.clone();
    *wifi_nets.lock().unwrap() = scan_for_available_ssids(&mut wifi);
    log::info!("Wi-Fi started");

    // Thread to handle Wi-Fi IPC messages
    let wifi_nets = wifi_nets_store.clone();
    let wifi_update_pair = Arc::clone(&wifi_update_cond);
    thread::spawn(move || {
        while let Ok(msg) = wifi_runner_rx.recv() {
            log::info!("{msg:?}");
            match msg {
                WifiRunner::ChangeWifi(creds) => {
                    wifi.set_configuration(&wifi::Configuration::Mixed(
                        ClientConfiguration {
                            ssid: creds.ssid.as_str().into(),
                            password: creds.wpa2.as_str().into(),
                            ..Default::default()
                        },
                        AccessPointConfiguration {
                            ssid: env!("SSID").into(),
                            password: env!("SSID_PASSWORD").into(),
                            auth_method: wifi::AuthMethod::WPA2Personal,
                            ..Default::default()
                        },
                    ))
                    .unwrap();
                    wifi.connect().unwrap();
                }
                WifiRunner::GetWifi => {
                    let (lock, cvar) = &*wifi_update_pair;
                    let mut wifi_update = lock.lock().unwrap();
                    *wifi_nets.lock().unwrap() = scan_for_available_ssids(&mut wifi);
                    *wifi_update = true;
                    cvar.notify_one();
                }
            }
        }
    });

    log::info!("Starting DNS server...");
    let mut dns = SimpleDns::try_new(IP_ADDRESS).expect("DNS server init failed");
    thread::spawn(move || loop {
        dns.poll().ok();
        sleep(Duration::from_millis(50));
    });
    log::info!("DNS server started");

    log::info!("Starting HTTP server...");
    let config = Configuration::default();
    let mut server =
        server::setup_server(&config, wifi_runner_tx, wifi_update_cond, wifi_nets_store)?;
    log::info!("HTTP server started");

    log::info!("Attaching captive portal...");
    CaptivePortal::attach(&mut server, IP_ADDRESS).expect("Captive portal attach failed");

    loop {
        sleep(Duration::from_millis(100));
    }
}

fn scan_for_available_ssids(wifi: &mut EspWifi) -> Vec<AccessPointInfo> {
    let mut available_ssids = wifi.scan().unwrap();
    available_ssids.sort_by(|a, b| a.ssid.cmp(&b.ssid));
    available_ssids.dedup_by(|a, b| a.ssid == b.ssid);
    available_ssids.sort_by(|a, b| a.signal_strength.cmp(&b.signal_strength).reverse());
    available_ssids
}
