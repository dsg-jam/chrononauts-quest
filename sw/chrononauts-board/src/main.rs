use crate::captive::CaptivePortal;
use dns::*;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::{InputMode, Pin, PinDriver, Pull},
        prelude::Peripherals,
        spi::{
            config::{Config, DriverConfig},
            Dma, SpiDeviceDriver, SpiDriver,
        },
    },
    http::server::Configuration,
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    sys::{self, EspError},
    wifi::{AccessPointInfo, WifiDriver},
};
use radio::{FromBytes, ToBytes};
use std::{
    sync::{mpsc, Arc, Condvar, Mutex},
    thread::{self, sleep},
    time::{Duration, SystemTime},
};
use wifi::{WifiCreds, WifiRunner, IP_ADDRESS};

mod captive;
mod dns;
mod radio;
mod server;
mod wifi;

use cc1101::Cc1101;

fn main() -> Result<(), EspError> {
    unsafe {
        sys::nvs_flash_init();
    }
    sys::link_patches();
    EspLogger::initialize_default();

    // Determine chrononauts-board id
    let chrononauts_id = env!("CHRONONAUTS_ID").parse::<isize>().unwrap();
    if chrononauts_id == -1 {
        log::info!("Chrononauts ID not set");
    } else {
        log::info!("Chrononauts ID: {}", chrononauts_id);
    }

    let event_loop = EspSystemEventLoop::take()?;
    let peripherals = Peripherals::take()?;
    let (wifi_runner_tx, wifi_runner_rx) = mpsc::channel::<WifiRunner>();

    let wifi_update_cond = Arc::new((Mutex::new(false), Condvar::new()));
    let pins = peripherals.pins;

    let wifi_nets_store = Arc::new(Mutex::new(Vec::<AccessPointInfo>::new()));

    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        pins.gpio6,
        pins.gpio7,
        Some(pins.gpio2),
        &DriverConfig::default().dma(Dma::Auto(4096)),
    )?;

    let spi_device_driver =
        SpiDeviceDriver::new(spi_driver, Some(pins.gpio10), &Config::default())?;

    let cc1101 = Cc1101::new(spi_device_driver).unwrap();
    let mut radio = radio::ChrononautsRadio::new(cc1101);

    let mut push_button = PinDriver::input(pins.gpio9)?;
    push_button.set_pull(Pull::Up)?;

    let mut led1 = PinDriver::output(pins.gpio3)?;
    let mut led2 = PinDriver::output(pins.gpio1)?;

    // Start the wifi driver
    {
        let wifi_driver = WifiDriver::new(
            peripherals.modem,
            event_loop.clone(),
            EspDefaultNvsPartition::take().ok(),
        )?;
        let wifi_nets_store = wifi_nets_store.clone();
        let wifi_update_cond = wifi_update_cond.clone();
        thread::spawn(move || {
            let mut chrononauts_wifi =
                wifi::ChrononautsWifi::new(wifi_driver, wifi_nets_store.clone(), wifi_runner_rx)
                    .unwrap();
            chrononauts_wifi.start(wifi_update_cond.clone()).unwrap();
        });
    }

    log::info!("Starting DNS server...");
    let mut dns = SimpleDns::try_new(IP_ADDRESS).expect("DNS server init failed");
    thread::spawn(move || loop {
        dns.poll().ok();
        sleep(Duration::from_millis(50));
    });
    log::info!("DNS server started");

    log::info!("Starting radio...");
    radio.init_radio().expect("Radio init failed");
    thread::spawn(move || {
        let mut previous_level = false;
        let mut button_state = false;
        let mut last_change_time = SystemTime::now();

        loop {
            if let Ok((payload, _len)) = radio.get_packet() {
                let packet = radio::ChrononautsPackage::from_bytes(&payload).unwrap();
                log::info!("Packet received: {:?}", packet);
                led1.toggle().unwrap();
            }

            if debounce_button(
                &push_button,
                &mut previous_level,
                &mut button_state,
                &mut last_change_time,
            )
            .unwrap()
            {
                let payload = radio::ChrononautsPayload::SyncRequest;
                let header = radio::ChrononautsHeader::new(0, 1, 8);
                let package = radio::ChrononautsPackage { header, payload };
                let mut packet = package.to_bytes();
                if radio.send_packet(&mut packet).is_ok() {
                    log::info!("Packet sent");
                    led2.toggle().unwrap();
                }
            };
            sleep(Duration::from_millis(20));
        }
    });

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

fn debounce_button<T, MODE>(
    push_button: &PinDriver<T, MODE>,
    previous_level: &mut bool,
    button_state: &mut bool,
    last_change_time: &mut SystemTime,
) -> Result<bool, EspError>
where
    MODE: InputMode,
    T: Pin,
{
    let current_level = push_button.is_high();
    // If the switch changed, due to noise or pressing:
    if current_level != *previous_level {
        *previous_level = current_level;
        *last_change_time = SystemTime::now();
    }

    if SystemTime::now()
        .duration_since(*last_change_time)
        .unwrap()
        .as_millis()
        > 50
    {
        // the reading is same for the last 50 ms
        // if the button state has changed:
        if current_level != *button_state {
            *button_state = current_level;
            if !(*button_state) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
