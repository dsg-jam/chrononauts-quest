use std::sync::mpsc::Receiver;

use esp_idf_svc::{
    ipv4::{self, Mask, RouterConfiguration, Subnet},
    netif::{EspNetif, NetifConfiguration, NetifStack},
    sys::EspError,
    wifi::{
        self, config::ScanConfig, AccessPointConfiguration, AccessPointInfo, ClientConfiguration,
        EspWifi, WifiDriver,
    },
};

use crate::{consts::AP_IP_ADDRESS, ChrononautsSSIDs};

#[derive(Debug)]
pub enum WifiRunner {
    ChangeWifi(WifiCreds),
    GetWifi,
    ReconnectWifi,
    ScanFinished,
}

#[derive(Debug)]
pub struct WifiCreds {
    pub ssid: heapless::String<32>,
    pub wpa2: heapless::String<64>,
}

pub struct ChrononautsWifi<'a> {
    wifi: EspWifi<'a>,
    runner_rx: Receiver<WifiRunner>,
    ssids: ChrononautsSSIDs,
}

impl<'a> ChrononautsWifi<'a> {
    pub fn new(
        wifi_driver: WifiDriver<'a>,
        runner_rx: Receiver<WifiRunner>,
        ssids: ChrononautsSSIDs,
    ) -> Result<Self, EspError> {
        let wifi = EspWifi::wrap_all(
            wifi_driver,
            EspNetif::new(NetifStack::Sta)?,
            EspNetif::new_with_conf(&NetifConfiguration {
                ip_configuration: ipv4::Configuration::Router(RouterConfiguration {
                    subnet: Subnet {
                        gateway: AP_IP_ADDRESS,
                        mask: Mask(24),
                    },
                    dhcp_enabled: true,
                    dns: Some(AP_IP_ADDRESS),
                    secondary_dns: Some(AP_IP_ADDRESS),
                }),
                ..NetifConfiguration::wifi_default_router()
            })?,
        )?;
        Ok(ChrononautsWifi {
            wifi,
            runner_rx,
            ssids,
        })
    }

    /// Configure Wi-Fi in mixed mode
    fn configure(&mut self) -> Result<(), EspError> {
        self.wifi.set_configuration(&wifi::Configuration::Mixed(
            ClientConfiguration {
                ..Default::default()
            },
            get_ap_config(),
        ))?;
        Ok(())
    }

    /// Check if we have saved Wi-Fi credentials in NVS
    ///
    /// Returns true if we have saved credentials, false otherwise
    fn has_saved_credentials(&self) -> bool {
        let Ok(conf) = self.wifi.get_configuration() else {
            return false;
        };
        let Some(client_conf) = conf.as_client_conf_ref() else {
            return false;
        };

        if client_conf.ssid.len() > 0 && client_conf.password.len() > 0 {
            return true;
        }

        false
    }

    /// Start the Wi-Fi stack
    ///
    /// This function will start the Wi-Fi stack and attempt to connect to a Wi-Fi network if we have saved credentials.
    /// If we do not have saved credentials, it will configure the Wi-Fi stack in mixed mode.
    fn start(&mut self) -> Result<(), EspError> {
        self.wifi.start()?;

        // Let's (attempt to) connect if we have saved credentials
        if self.has_saved_credentials() {
            self.wifi.connect()?;
        } else {
            self.configure()?;
        }

        log::info!("Wi-Fi started");
        Ok(())
    }

    fn get_scanned_ssids(&mut self) -> Result<Vec<AccessPointInfo>, EspError> {
        let mut ssids = self.wifi.get_scan_result()?;
        ssids.sort_by(|a, b| a.ssid.cmp(&b.ssid));
        ssids.dedup_by(|a, b| a.ssid == b.ssid);
        ssids.sort_by(|a, b| a.signal_strength.cmp(&b.signal_strength).reverse());
        Ok(ssids)
    }

    pub fn run(&mut self) -> Result<(), EspError> {
        self.start()?;
        while let Ok(event) = self.runner_rx.recv() {
            match event {
                WifiRunner::ChangeWifi(creds) => {
                    log::info!("Changing Wi-Fi to {:?}", creds);
                    self.change_wifi(creds)?;
                }
                WifiRunner::GetWifi => {
                    self.scan_for_available_ssids()?;
                }
                WifiRunner::ReconnectWifi => {
                    self.wifi.connect()?;
                }
                WifiRunner::ScanFinished => {
                    let scanned_ssids = self.get_scanned_ssids()?;
                    let mut ssids = self.ssids.lock().unwrap();
                    *ssids = scanned_ssids;
                }
            }
        }

        Ok(())
    }

    fn change_wifi(&mut self, creds: WifiCreds) -> Result<(), EspError> {
        self.wifi.set_configuration(&wifi::Configuration::Mixed(
            ClientConfiguration {
                ssid: creds.ssid,
                password: creds.wpa2,
                ..Default::default()
            },
            get_ap_config(),
        ))?;
        self.wifi.connect()?;
        Ok(())
    }

    fn scan_for_available_ssids(&mut self) -> Result<(), EspError> {
        let scan_config = ScanConfig::new();
        self.wifi.start_scan(&scan_config, false)?;
        Ok(())
    }
}

fn get_ap_config() -> AccessPointConfiguration {
    AccessPointConfiguration {
        ssid: env!("SSID").parse().unwrap(),
        password: env!("SSID_PASSWORD").parse().unwrap(),
        auth_method: wifi::AuthMethod::WPA2Personal,
        ..Default::default()
    }
}
