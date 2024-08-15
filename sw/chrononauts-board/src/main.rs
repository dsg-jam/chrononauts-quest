use crate::captive::CaptivePortal;
use core::pin::pin;
use dns::*;
use esp_idf_svc::{
    eventloop::{
        EspBackgroundEventLoop, EspEvent, EspEventDeserializer, EspEventPostData,
        EspEventSerializer, EspEventSource, EspSystemEventLoop,
    },
    hal::{
        delay,
        gpio::{PinDriver, Pull},
        prelude::Peripherals,
        spi::{
            config::{Config, DriverConfig},
            Dma, SpiDeviceDriver, SpiDriver,
        },
        task::block_on,
    },
    http::server::Configuration,
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    sys::{self, EspError},
    timer::EspTimerService,
    wifi::{AccessPointInfo, WifiDriver, WifiEvent},
};
use radio::ChrononautsPackage;
use std::{
    ffi::CStr,
    sync::{mpsc, Arc, Condvar, Mutex},
    thread::{self, sleep},
    time::Duration,
};
use wifi::{WifiCreds, WifiRunner, IP_ADDRESS};
mod captive;
mod dns;
mod radio;
mod server;
mod wifi;

use cc1101::Cc1101;

#[derive(Debug, thiserror::Error)]
pub enum ChrononautsError {
    #[error("Invalid Chrononauts ID")]
    InvalidChrononautsId,
    #[error(transparent)]
    EspError(#[from] EspError),
    #[error(transparent)]
    GameLoopSendError(#[from] mpsc::SendError<GameLoop>),
}

#[derive(Clone, Copy, Debug)]
pub enum GameLoop {
    Init,
    WifiSetup,
    SendSyncRequest,
    AwaitSyncRequest,
    SendSyncAck,
    AwaitSyncAck,
    SendLevel(u8),
    AwaitLevel,
    AwaitLevelAck,
    Level(u8),
    End,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum ChrononautsId {
    T = 0,
    L = 1,
}

impl ChrononautsId {
    fn other(&self) -> Self {
        match self {
            ChrononautsId::T => ChrononautsId::L,
            ChrononautsId::L => ChrononautsId::T,
        }
    }
}

impl From<ChrononautsId> for u8 {
    fn from(value: ChrononautsId) -> u8 {
        value as u8
    }
}

impl TryFrom<u8> for ChrononautsId {
    type Error = ChrononautsError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ChrononautsId::T),
            1 => Ok(ChrononautsId::L),
            _ => Err(ChrononautsError::InvalidChrononautsId),
        }
    }
}

// Chrononauts event loop shenanigans
#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
enum ChrononautsEvent {
    ButtonChanged(bool),
    RadioPacketReceived(ChrononautsPackage),
    WifiConnected,
}

unsafe impl EspEventSource for ChrononautsEvent {
    fn source() -> Option<&'static core::ffi::CStr> {
        Some(CStr::from_bytes_with_nul(b"CHRONONAUTS-SERVICE\0").unwrap())
    }
}

impl EspEventSerializer for ChrononautsEvent {
    type Data<'a> = ChrononautsEvent;

    fn serialize<F, R>(event: &Self::Data<'_>, f: F) -> R
    where
        F: FnOnce(&EspEventPostData) -> R,
    {
        f(&unsafe { EspEventPostData::new(Self::source().unwrap(), Self::event_id(), event) })
    }
}

impl EspEventDeserializer for ChrononautsEvent {
    type Data<'a> = ChrononautsEvent;

    fn deserialize<'a>(data: &EspEvent<'a>) -> Self::Data<'a> {
        // Just as easy as serializing
        *unsafe { data.as_payload::<ChrononautsEvent>() }
    }
}

fn main() -> Result<(), ChrononautsError> {
    unsafe {
        sys::nvs_flash_init();
    }
    sys::link_patches();
    EspLogger::initialize_default();

    run()?;

    Ok(())
}

fn run() -> Result<(), ChrononautsError> {
    // Determine chrononauts-board id
    let chrononauts_id = env!("CHRONONAUTS_ID")
        .parse::<u8>()
        .expect("Chrononauts ID MUST be set");
    let chrononauts_id = ChrononautsId::try_from(chrononauts_id)?;

    // Get system event loop (used for wifi)
    let system_event_loop = EspSystemEventLoop::take()?;

    // Create an event loop for the main thread
    let chrononauts_event_loop = EspBackgroundEventLoop::new(&Default::default())?;

    let peripherals = Peripherals::take()?;
    let (wifi_runner_tx, wifi_runner_rx) = mpsc::channel::<WifiRunner>();
    let wifi_update_cond = Arc::new((Mutex::new(false), Condvar::new()));

    let (packets_to_send_tx, packets_to_send_rx) = mpsc::channel::<ChrononautsPackage>();

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
    let _wifi_handler = if let ChrononautsId::L = chrononauts_id {
        let wifi_driver = WifiDriver::new(
            peripherals.modem,
            system_event_loop.clone(),
            EspDefaultNvsPartition::take().ok(),
        )?;
        let wifi_nets_store = wifi_nets_store.clone();
        let wifi_update_cond = wifi_update_cond.clone();
        Some(thread::spawn(move || {
            let mut chrononauts_wifi =
                wifi::ChrononautsWifi::new(wifi_driver, wifi_nets_store.clone(), wifi_runner_rx)
                    .unwrap();
            chrononauts_wifi.start(wifi_update_cond.clone()).unwrap();
        }))
    } else {
        None
    };

    let _dns_handler = if let ChrononautsId::L = chrononauts_id {
        log::info!("Starting DNS server...");
        let mut dns = SimpleDns::try_new(IP_ADDRESS).expect("DNS server init failed");
        Some(thread::spawn(move || loop {
            dns.poll().ok();
            sleep(Duration::from_millis(50));
        }))
    } else {
        None
    };

    log::info!("Starting radio...");
    radio.init_radio().expect("Radio init failed");
    {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        thread::spawn(move || loop {
            if let Ok(packet) = radio.get_packet() {
                if packet.header.destination == chrononauts_id.into() {
                    chrononauts_event_loop
                        .post::<ChrononautsEvent>(
                            &ChrononautsEvent::RadioPacketReceived(packet),
                            delay::BLOCK,
                        )
                        .unwrap();
                }
            }

            if let Ok(packet) = packets_to_send_rx.try_recv() {
                if radio.send_packet(packet).is_ok() {
                    log::info!("Packet sent");
                    led2.toggle().unwrap();
                }
            }
            sleep(Duration::from_millis(20));
        });
    }

    let _http_server_handler = if let ChrononautsId::L = chrononauts_id {
        log::info!("Starting HTTP server...");
        let config = Configuration::default();
        let mut server =
            server::setup_server(&config, wifi_runner_tx, wifi_update_cond, wifi_nets_store)?;
        log::info!("HTTP server started");

        log::info!("Attaching captive portal...");
        CaptivePortal::attach(&mut server, IP_ADDRESS).expect("Captive portal attach failed");
        Some(server)
    } else {
        None
    };

    // Notification for button press
    let _button_subscription_handler = {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        thread::spawn(|| {
            block_on(pin!(async move {
                loop {
                    push_button.wait_for_low().await.unwrap();
                    chrononauts_event_loop
                        .post_async::<ChrononautsEvent>(&ChrononautsEvent::ButtonChanged(false))
                        .await
                        .unwrap();
                    push_button.wait_for_high().await.unwrap();
                    chrononauts_event_loop
                        .post_async::<ChrononautsEvent>(&ChrononautsEvent::ButtonChanged(true))
                        .await
                        .unwrap();
                }
            }))
        })
    };

    // Notification for wifi connected
    let _wifi_subscription = {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        system_event_loop.subscribe::<WifiEvent, _>(move |event| {
            if let WifiEvent::StaConnected = event {
                chrononauts_event_loop
                    .post::<ChrononautsEvent>(&ChrononautsEvent::WifiConnected, delay::NON_BLOCK)
                    .unwrap();
            }
        })?
    };

    let mut game_state = GameLoop::Init;

    if let ChrononautsId::L = chrononauts_id {
        game_state = GameLoop::WifiSetup;
    } else {
        game_state = GameLoop::AwaitSyncRequest;
    }

    // Register handlers for the event loop
    block_on(pin!(async move {
        // Fetch posted events with an async subscription as well
        let mut subscription = chrononauts_event_loop.subscribe_async::<ChrononautsEvent>()?;
        log::info!("Subscribed to events");

        // Debounce button press helpers
        let mut previous_level = true;
        let mut button_state = true;
        let mut last_change_time = Duration::from_millis(0);

        loop {
            let event = subscription.recv().await?;
            match event {
                ChrononautsEvent::ButtonChanged(state) => {
                    if debounce_button(
                        state,
                        &mut previous_level,
                        &mut button_state,
                        &mut last_change_time,
                    )
                    .unwrap()
                    {
                        log::info!("Button pressed");
                        let header = radio::ChrononautsHeader::new(
                            chrononauts_id.into(),
                            chrononauts_id.other().into(),
                            0,
                        );
                        let payload = radio::ChrononautsPayload::SetLevel(1);
                        let packet = ChrononautsPackage::new(header, payload);
                        packets_to_send_tx.send(packet).unwrap();
                    }
                }
                ChrononautsEvent::RadioPacketReceived(packet) => {
                    if packet.header.destination == chrononauts_id.into() {
                        log::info!("Packet received: {:?}", packet);
                        led1.toggle().unwrap();
                        match packet.payload {
                            radio::ChrononautsPayload::SyncRequest => {
                                if let GameLoop::AwaitSyncRequest = game_state {
                                    log::info!("[GAME_LOOP]: Send SyncAck");
                                    let header = radio::ChrononautsHeader::new(
                                        chrononauts_id.into(),
                                        chrononauts_id.other().into(),
                                        0,
                                    );
                                    let payload = radio::ChrononautsPayload::Ack;
                                    let packet = ChrononautsPackage::new(header, payload);
                                    packets_to_send_tx.send(packet).unwrap();
                                    game_state = GameLoop::AwaitLevel;
                                }
                            }
                            radio::ChrononautsPayload::Ack => {
                                if let GameLoop::AwaitSyncAck = game_state {
                                    log::info!("[GAME_LOOP]: Send Level 1");
                                    let header = radio::ChrononautsHeader::new(
                                        chrononauts_id.into(),
                                        chrononauts_id.other().into(),
                                        0,
                                    );
                                    let payload = radio::ChrononautsPayload::SetLevel(1);
                                    let packet = ChrononautsPackage::new(header, payload);
                                    packets_to_send_tx.send(packet).unwrap();
                                    game_state = GameLoop::AwaitLevelAck;
                                }
                                if let GameLoop::AwaitLevelAck = game_state {
                                    log::info!("[GAME_LOOP]: Level 1 Acked");
                                    game_state = GameLoop::Level(1);
                                }
                            }
                            radio::ChrononautsPayload::SetLevel(level) => {
                                if let GameLoop::AwaitLevel = game_state {
                                    log::info!("[GAME_LOOP]: Level {}", level);
                                    let header = radio::ChrononautsHeader::new(
                                        chrononauts_id.into(),
                                        chrononauts_id.other().into(),
                                        0,
                                    );
                                    let payload = radio::ChrononautsPayload::Ack;
                                    let packet = ChrononautsPackage::new(header, payload);
                                    packets_to_send_tx.send(packet).unwrap();
                                    game_state = GameLoop::Level(level);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                ChrononautsEvent::WifiConnected => {
                    log::info!("Wifi connected");
                    if let GameLoop::WifiSetup = game_state {
                        log::info!("[GAME_LOOP]: Send SyncRequest");
                        let header = radio::ChrononautsHeader::new(
                            chrononauts_id.into(),
                            chrononauts_id.other().into(),
                            0,
                        );
                        let payload = radio::ChrononautsPayload::SyncRequest;
                        let packet = ChrononautsPackage::new(header, payload);
                        packets_to_send_tx.send(packet).unwrap();
                        game_state = GameLoop::AwaitSyncAck;
                    }
                }
            }
        }
    }))
}

fn debounce_button(
    current_level: bool,
    previous_level: &mut bool,
    button_state: &mut bool,
    last_change_time: &mut Duration,
) -> Result<bool, EspError> {
    let system_time = EspTimerService::new()?;
    if system_time.now() - *last_change_time > Duration::from_millis(50) {
        *last_change_time = system_time.now();
        if current_level == *previous_level {
            return Ok(false);
        }
        *previous_level = current_level;
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
