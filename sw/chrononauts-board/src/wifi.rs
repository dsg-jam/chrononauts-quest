use std::{
    net::Ipv4Addr,
    sync::{mpsc::Receiver, Arc, Condvar, Mutex},
};

use esp_idf_svc::{
    ipv4::{self, Mask, RouterConfiguration, Subnet},
    netif::{EspNetif, NetifConfiguration, NetifStack},
    sys::EspError,
    wifi::{
        self, AccessPointConfiguration, AccessPointInfo, ClientConfiguration, EspWifi, WifiDriver,
    },
};

pub const IP_ADDRESS: Ipv4Addr = Ipv4Addr::new(10, 9, 1, 1);

#[derive(Debug)]
pub enum WifiRunner {
    ChangeWifi(WifiCreds),
    GetWifi,
}

#[derive(Debug)]
pub struct WifiCreds {
    pub ssid: heapless::String<32>,
    pub wpa2: heapless::String<64>,
}

pub struct ChrononautsWifi<'a> {
    wifi: EspWifi<'a>,
    ssids: Arc<Mutex<Vec<AccessPointInfo>>>,
    runner_rx: Receiver<WifiRunner>,
}

impl<'a> ChrononautsWifi<'a> {
    pub fn new(
        wifi_driver: WifiDriver<'a>,
        ssids: Arc<Mutex<Vec<AccessPointInfo>>>,
        runner_rx: Receiver<WifiRunner>,
    ) -> Result<Self, EspError> {
        let wifi = EspWifi::wrap_all(
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
        )?;
        Ok(ChrononautsWifi {
            wifi,
            ssids,
            runner_rx,
        })
    }

    fn configure(&mut self) -> Result<(), EspError> {
        self.wifi.set_configuration(&wifi::Configuration::Mixed(
            ClientConfiguration {
                ..Default::default()
            },
            AccessPointConfiguration {
                ssid: env!("SSID").parse().unwrap(),
                password: env!("SSID_PASSWORD").parse().unwrap(),
                auth_method: wifi::AuthMethod::WPA2Personal,
                ..Default::default()
            },
        ))?;
        Ok(())
    }

    pub fn start(&mut self, update_pair: Arc<(Mutex<bool>, Condvar)>) -> Result<(), EspError> {
        self.configure()?;
        self.wifi.start()?;
        self.scan_for_available_ssids()?;
        log::info!("Wi-Fi started");

        while let Ok(msg) = self.runner_rx.recv() {
            log::info!("{msg:?}");
            match msg {
                WifiRunner::ChangeWifi(creds) => {
                    self.wifi
                        .set_configuration(&wifi::Configuration::Mixed(
                            ClientConfiguration {
                                ssid: creds.ssid,
                                password: creds.wpa2,
                                ..Default::default()
                            },
                            AccessPointConfiguration {
                                ssid: env!("SSID").parse().unwrap(),
                                password: env!("SSID_PASSWORD").parse().unwrap(),
                                auth_method: wifi::AuthMethod::WPA2Personal,
                                ..Default::default()
                            },
                        ))
                        .unwrap();
                    self.wifi.connect()?;
                }
                WifiRunner::GetWifi => {
                    let (lock, cvar) = &*update_pair;
                    let mut wifi_update = lock.lock().unwrap();
                    self.scan_for_available_ssids()?;
                    *wifi_update = true;
                    cvar.notify_one();
                }
            }
        }
        Ok(())
    }

    fn scan_for_available_ssids(&mut self) -> Result<(), EspError> {
        let mut available_ssids = self.wifi.scan()?;
        available_ssids.sort_by(|a, b| a.ssid.cmp(&b.ssid));
        available_ssids.dedup_by(|a, b| a.ssid == b.ssid);
        available_ssids.sort_by(|a, b| a.signal_strength.cmp(&b.signal_strength).reverse());
        *self.ssids.lock().unwrap() = available_ssids;
        Ok(())
    }
}
