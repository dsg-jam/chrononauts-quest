use crate::captive::CaptivePortal;
use backend_api::Level;
use core::pin::pin;
use dns::*;
use esp_idf_svc::{
    eventloop::{Background, EspBackgroundEventLoop, EspEventLoop, EspSystemEventLoop, User},
    hal::{
        adc::{
            attenuation::DB_11,
            oneshot::{config::AdcChannelConfig, AdcChannelDriver, AdcDriver},
        },
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
use event::{
    GameLoopEvent, MainEvent, MessageTransmissionEvent, PacketReceptionEvent,
    PacketTransmissionEvent,
};
use radio::{ChrononautsMessage, ChrononautsPacket, MessagePayload, MessageSource};
use std::{
    sync::{mpsc, Arc, Condvar, Mutex},
    thread::{self, sleep},
    time::Duration,
};
use wifi::{WifiCreds, WifiRunner, IP_ADDRESS};
mod captive;
mod consts;
mod dns;
mod event;
mod radio;
mod server;
mod wifi;
mod ws;

use cc1101::Cc1101;

#[derive(Debug, thiserror::Error)]
pub enum ChrononautsError {
    #[error("Invalid Chrononauts ID")]
    InvalidChrononautsId,
    #[error(transparent)]
    EspError(#[from] EspError),
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

type ChrononautsEventLoop = EspEventLoop<User<Background>>;

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
    log::info!("Chrononauts ID: {:?}", chrononauts_id);

    // Get system event loop (used for wifi)
    let system_event_loop = EspSystemEventLoop::take()?;

    // Create an event loop for the main thread
    let chrononauts_event_loop = EspBackgroundEventLoop::new(&Default::default())?;

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
    let mut radio_transceiver = radio::ChrononautsTransceiver::new(cc1101);
    let mut radio_transport =
        radio::ChrononautsTransport::new(chrononauts_event_loop.clone(), chrononauts_id);

    let mut push_button = PinDriver::input(pins.gpio9)?;
    push_button.set_pull(Pull::Up)?;

    let mut led1 = PinDriver::output(pins.gpio3)?;
    let mut led2 = PinDriver::output(pins.gpio1)?;

    let poti_adc = AdcDriver::new(peripherals.adc1)?;

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

    log::info!("Starting radio components...");
    log::info!("[RADIO]: Initializing radio transceiver...");
    {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        thread::spawn(move || {
            radio_transceiver.init().expect("Radio init failed");
            let (packets_to_send_tx, packets_to_send_rx) = mpsc::channel::<ChrononautsPacket>();

            let _packet_transmission_sub = chrononauts_event_loop
                .subscribe::<PacketTransmissionEvent, _>(move |event| {
                    let PacketTransmissionEvent::Packet(packet) = event;
                    log::info!("Packet transmission event: {:?}", packet);
                    packets_to_send_tx.send(packet).unwrap();
                })
                .unwrap();

            loop {
                if let Ok(packet) = radio_transceiver.get_packet() {
                    if packet.matches_destination(chrononauts_id.into()) {
                        chrononauts_event_loop
                            .post::<PacketReceptionEvent>(
                                &PacketReceptionEvent::Packet(packet),
                                delay::BLOCK,
                            )
                            .unwrap();
                    }
                }

                if let Ok(packet) = packets_to_send_rx.try_recv() {
                    if radio_transceiver.send_packet(&packet).is_ok() {
                        log::info!("Packet sent");
                    }
                }

                sleep(Duration::from_millis(20));
            }
        });
    }

    log::info!("[RADIO]: Starting transport layer...");
    let _radio_transport_handler = {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        thread::spawn(move || {
            let (packets_to_process_tx, packets_to_process_rx) =
                mpsc::channel::<ChrononautsPacket>();
            let (messages_to_process_tx, messages_to_process_rx) =
                mpsc::channel::<ChrononautsMessage>();

            let _packet_reception_sub = chrononauts_event_loop
                .subscribe::<PacketReceptionEvent, _>(move |event| {
                    let PacketReceptionEvent::Packet(packet) = event;
                    log::info!("Packet reception event: {:?}", packet);
                    packets_to_process_tx.send(packet).unwrap();
                })
                .unwrap();

            let _message_transmission_sub = chrononauts_event_loop
                .subscribe::<MessageTransmissionEvent, _>(move |event| {
                    let MessageTransmissionEvent::Message(message) = event;
                    log::info!("Message transmission event: {:?}", message);
                    messages_to_process_tx.send(message).unwrap();
                })
                .unwrap();

            loop {
                if let Ok(packet) = packets_to_process_rx.try_recv() {
                    if let Ok(Some(message)) = radio_transport.handle_reception(packet) {
                        chrononauts_event_loop
                            .post::<MainEvent>(&MainEvent::MessageReceived(message), delay::BLOCK)
                            .unwrap();
                    };
                }

                if let Ok(message) = messages_to_process_rx.try_recv() {
                    radio_transport.enqueue_message(message).unwrap();
                }

                let _ = radio_transport.handle_send();

                sleep(Duration::from_millis(20));
            }
        })
    };

    log::info!("Radio components started");

    let _http_server_handler = if let ChrononautsId::L = chrononauts_id {
        let wifi_runner_tx = wifi_runner_tx.clone();
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

    let (ws_start_tx, ws_start_rx) = mpsc::channel::<()>();
    let _ws_client_handler = if let ChrononautsId::L = chrononauts_id {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        Some(thread::spawn(move || {
            ws_start_rx.recv().unwrap();
            log::info!("Starting WebSocket client...");
            ws::ChrononautsWebSocketClient::new(chrononauts_event_loop)
            //assert_eq!(rx.recv(), Ok(ChrononautsWsEvent::Connected));
            //assert!(ws_client.is_connected());
            //ws_client.send_message("Hello, World!");
        }))
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
                        .post_async::<MainEvent>(&MainEvent::ButtonChanged(false))
                        .await
                        .unwrap();
                    push_button.wait_for_high().await.unwrap();
                    chrononauts_event_loop
                        .post_async::<MainEvent>(&MainEvent::ButtonChanged(true))
                        .await
                        .unwrap();
                }
            }))
        })
    };

    // Notification for wifi connected
    let _wifi_subscription = {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        let wifi_runner_tx = wifi_runner_tx.clone();
        system_event_loop.subscribe::<WifiEvent, _>(move |event| match event {
            WifiEvent::StaConnected(_) => {
                chrononauts_event_loop
                    .post::<MainEvent>(&MainEvent::WifiConnected, delay::NON_BLOCK)
                    .unwrap();
                ws_start_tx.send(()).unwrap();
            }
            WifiEvent::StaDisconnected(_) => {
                log::info!("Wi-Fi disconnected - trying to reconnect");
                wifi_runner_tx.send(WifiRunner::ReconnectWifi).unwrap();
            }
            _ => {}
        })?
    };

    // Turning knob handler
    let _turning_knob_handler = {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        thread::spawn(move || {
            let game_level = Arc::new(Mutex::new(Level::L0));
            let config = AdcChannelConfig {
                attenuation: DB_11,
                calibration: true,
                ..Default::default()
            };
            let mut poti = AdcChannelDriver::new(&poti_adc, pins.gpio0, &config).unwrap();
            let _sub = {
                let game_level = game_level.clone();
                chrononauts_event_loop
                    .subscribe::<GameLoopEvent, _>(move |event| {
                        if let GameLoopEvent::GameLevelChanged(level) = event {
                            *game_level.lock().unwrap() = level;
                        }
                    })
                    .unwrap()
            };

            log::info!("[TURNING_KNOB]: Starting turning knob handler");
            loop {
                let game_level = *game_level.lock().unwrap();

                if let Level::L2 = game_level {
                    let poti_value = poti_adc.read(&mut poti).unwrap() + 100;
                    chrononauts_event_loop
                        .post::<GameLoopEvent>(
                            &GameLoopEvent::SetLedBlinkSpeed(poti_value as u64),
                            delay::BLOCK,
                        )
                        .unwrap();
                }
                sleep(Duration::from_millis(100));
            }
        })
    };

    // Led handler
    let _led_handler = {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        thread::spawn(move || {
            let game_level = Arc::new(Mutex::new(Level::L0));
            let blink_speed = Arc::new(Mutex::new(None));
            let _game_loop_sub = {
                let game_level = game_level.clone();
                let blink_speed = blink_speed.clone();
                chrononauts_event_loop
                    .subscribe::<GameLoopEvent, _>(move |event| match event {
                        GameLoopEvent::GameLevelChanged(level) => {
                            *game_level.lock().unwrap() = level;
                        }
                        GameLoopEvent::SetLedBlinkSpeed(speed) => {
                            *blink_speed.lock().unwrap() = Some(speed);
                        }
                        _ => {}
                    })
                    .unwrap()
            };

            log::info!("[LED_HANDLER]: Starting LED handler");
            loop {
                let game_level = *game_level.lock().unwrap();
                let blink_speed = *blink_speed.lock().unwrap();

                match game_level {
                    Level::L2 => {
                        let Some(speed) = blink_speed else {
                            led1.set_low().unwrap();
                            led2.set_low().unwrap();
                            sleep(Duration::from_millis(100));
                            continue;
                        };
                        led1.set_high().unwrap();
                        led2.set_low().unwrap();
                        sleep(Duration::from_millis(speed));
                        led1.set_low().unwrap();
                        led2.set_high().unwrap();
                        sleep(Duration::from_millis(speed));
                    }
                    _ => {
                        led1.set_low().unwrap();
                        led2.set_low().unwrap();
                        sleep(Duration::from_millis(100));
                    }
                }
            }
        })
    };

    // Game loop
    let game_level = Arc::new((Mutex::new(Level::L1), Condvar::new()));
    let _game_loop_handler = {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        let game_level = game_level.clone();
        thread::spawn(move || {
            let mut previous_level = *game_level.0.lock().unwrap();
            loop {
                let (lock, cvar) = &*(game_level.clone());
                let mut current_level = lock.lock().unwrap();
                while *current_level == previous_level {
                    // this will block this thread until game game_level is changed
                    current_level = cvar.wait(current_level).unwrap();
                }
                previous_level = *current_level;

                log::info!("[GAME_LOOP]: Game level changed to {:?}", previous_level);
                chrononauts_event_loop
                    .post::<GameLoopEvent>(
                        &GameLoopEvent::GameLevelChanged(*current_level),
                        delay::BLOCK,
                    )
                    .unwrap();
            }
        })
    };

    // Register handlers for the event loop
    block_on(pin!(async move {
        // Fetch posted events with an async subscription as well
        let mut subscription = chrononauts_event_loop.subscribe_async::<MainEvent>()?;
        log::info!("Subscribed to events");

        // Debounce button press helpers
        let mut previous_level = true;
        let mut button_state = true;
        let mut last_change_time = Duration::from_millis(0);

        let (lock, cvar) = &*game_level;

        loop {
            let event = subscription.recv().await?;
            match event {
                MainEvent::ButtonChanged(state) => {
                    if debounce_button(
                        state,
                        &mut previous_level,
                        &mut button_state,
                        &mut last_change_time,
                    )? {
                        log::info!("Button pressed");
                    }
                }
                MainEvent::WifiConnected => {
                    log::info!("Wifi connected");
                }
                MainEvent::MessageReceived(msg) => {
                    log::info!("Message received: {:?}", msg);
                    match msg.source() {
                        MessageSource::Backend => {
                            if let MessagePayload::SetGameLevel(level) = msg.payload() {
                                log::info!("[MAIN_EVENT]: Received SetGameLevel from Backend");
                                let mut game_level = lock.lock().unwrap();
                                *game_level = level;
                                cvar.notify_one();
                                chrononauts_event_loop
                                    .post::<MessageTransmissionEvent>(
                                        &MessageTransmissionEvent::Message(
                                            ChrononautsMessage::new_from_board(
                                                MessagePayload::SetGameLevel(level),
                                            ),
                                        ),
                                        delay::BLOCK,
                                    )
                                    .unwrap();
                            }
                        }
                        MessageSource::Board => {
                            if let MessagePayload::SetGameLevel(level) = msg.payload() {
                                log::info!("[MAIN_EVENT]: Received SetGameLevel from Wifi-Board");
                                let mut game_level = lock.lock().unwrap();
                                *game_level = level;
                                cvar.notify_one();
                            }
                        }
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
