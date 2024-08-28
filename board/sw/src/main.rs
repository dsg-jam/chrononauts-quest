use crate::captive::CaptivePortal;
use backend_api::{labyrinth::Direction, Level};
use consts::AP_IP_ADDRESS;
use core::pin::pin;
use dns::*;
use esp_idf_svc::{
    eventloop::{Background, EspBackgroundEventLoop, EspEventLoop, EspSystemEventLoop, User},
    hal::{
        adc::{
            attenuation::DB_11,
            oneshot::{config::AdcChannelConfig, AdcChannelDriver, AdcDriver},
        },
        delay::{self},
        gpio::{PinDriver, Pull},
        i2c::{self, I2cDriver},
        prelude::Peripherals,
        spi::{
            config::{Config, DriverConfig},
            Dma, SpiDeviceDriver, SpiDriver,
        },
        task::block_on,
    },
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    sys::{self, EspError},
    wifi::{AccessPointInfo, WifiDriver, WifiEvent},
};
use event::{
    GameLoopEvent, MainEvent, MessageTransmissionEvent, PacketReceptionEvent,
    PacketTransmissionEvent, WsTransmissionEvent,
};
use http_server::ChrononautsHttpServer;
use peripherals::{ChrononautsAccelerometer, PeripheralError};
use radio::{ChrononautsMessage, ChrononautsPacket, MessagePayload, MessageSource};
use std::{
    sync::{mpsc, Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};
use utils::{ChrononautsId, DebounceButton};
use wifi::{WifiCreds, WifiRunner};
use ws::ChrononautsWebSocketClient;

mod captive;
mod consts;
mod dns;
mod event;
mod http_server;
mod peripherals;
mod radio;
mod utils;
mod wifi;
mod ws;

use cc1101::Cc1101;

#[derive(Debug, thiserror::Error)]
pub enum ChrononautsError {
    #[error("Invalid Chrononauts ID")]
    InvalidChrononautsId,
    #[error(transparent)]
    EspError(#[from] EspError),
    #[error(transparent)]
    PeripheralError(#[from] PeripheralError),
    #[error(transparent)]
    WsError(#[from] ws::WsError),
}

type ChrononautsEventLoop = EspEventLoop<User<Background>>;
type ChrononautsSSIDs = Arc<Mutex<Vec<AccessPointInfo>>>;

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
    // ########
    // # Init #
    // ########
    let chrononauts_id = utils::get_chrononauts_id()?;

    let system_event_loop = EspSystemEventLoop::take()?;
    let chrononauts_event_loop = EspBackgroundEventLoop::new(&Default::default())?;

    let peripherals = Peripherals::take()?;
    let (wifi_runner_tx, wifi_runner_rx) = mpsc::channel::<WifiRunner>();

    let pins = peripherals.pins;

    let wifi_available_ssids: ChrononautsSSIDs =
        Arc::new(Mutex::new(Vec::<AccessPointInfo>::new()));

    let spi_driver = SpiDriver::new(
        peripherals.spi2,
        pins.gpio6,
        pins.gpio7,
        Some(pins.gpio2),
        &DriverConfig::default().dma(Dma::Auto(4096)),
    )?;

    let i2c_config = i2c::config::Config {
        sda_pullup_enabled: false,
        scl_pullup_enabled: false,
        ..Default::default()
    };
    let i2c_driver = I2cDriver::new(peripherals.i2c0, pins.gpio21, pins.gpio20, &i2c_config)?;

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

    // ############################
    // # Wi-Fi connection handler #
    // ############################
    let _wifi_handler = if let ChrononautsId::L = chrononauts_id {
        let wifi_driver = WifiDriver::new(
            peripherals.modem,
            system_event_loop.clone(),
            EspDefaultNvsPartition::take().ok(),
        )?;
        let mut chrononauts_wifi =
            wifi::ChrononautsWifi::new(wifi_driver, wifi_runner_rx, wifi_available_ssids.clone())?;
        Some(thread::spawn(move || {
            chrononauts_wifi.run().unwrap();
        }))
    } else {
        None
    };

    // ######################
    // # DNS server handler #
    // ######################
    let _dns_handler = if let ChrononautsId::L = chrononauts_id {
        let mut dns = SimpleDns::try_new(AP_IP_ADDRESS).expect("DNS server init failed");
        Some(thread::spawn(move || loop {
            dns.poll().ok();
            sleep(Duration::from_millis(50));
        }))
    } else {
        None
    };

    // #############################
    // # Radio transceiver handler #
    // #############################
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

    // ###########################
    // # Radio transport handler #
    // ###########################
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

    // #######################
    // # HTTP server handler #
    // #######################
    let _http_server_handler = if let ChrononautsId::L = chrononauts_id {
        let wifi_runner_tx = wifi_runner_tx.clone();
        let mut server = ChrononautsHttpServer::new(wifi_runner_tx, wifi_available_ssids);
        server.setup().expect("HTTP server setup failed");

        CaptivePortal::attach(&mut server, AP_IP_ADDRESS).expect("Captive portal attach failed");
        Some(server)
    } else {
        None
    };

    // ############################
    // # WebSocket client handler #
    // ############################
    let _ws_client_handler = if let ChrononautsId::L = chrononauts_id {
        let mut ws_client = ChrononautsWebSocketClient::new(chrononauts_event_loop.clone());
        Some(thread::spawn(move || ws_client.run()))
    } else {
        None
    };

    // ########################
    // # Button Press Handler #
    // ########################
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

    // #############################
    // # Wi-Fi connection listener #
    // #############################
    let _wifi_connection_listener = {
        let chrononauts_event_loop = chrononauts_event_loop.clone();
        let wifi_runner_tx = wifi_runner_tx.clone();
        system_event_loop.subscribe::<WifiEvent, _>(move |event| match event {
            WifiEvent::StaConnected(_) => {
                chrononauts_event_loop
                    .post::<MainEvent>(&MainEvent::WifiConnected, delay::NON_BLOCK)
                    .unwrap();
            }
            WifiEvent::StaDisconnected(_) => {
                log::debug!("Wi-Fi disconnected - trying to reconnect");
                wifi_runner_tx.send(WifiRunner::ReconnectWifi).unwrap();
            }
            WifiEvent::ScanDone(status) => {
                if status.is_successful() {
                    wifi_runner_tx.send(WifiRunner::ScanFinished).unwrap();
                }
            }
            _ => {}
        })?
    };

    // #########################
    // # Potentiometer handler #
    // #########################
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

    // ################
    // # LEDs handler #
    // ################
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

    // #########################
    // # Accelerometer handler #
    // #########################
    let mut accelerometer =
        ChrononautsAccelerometer::new(i2c_driver, chrononauts_event_loop.clone());
    thread::spawn(move || accelerometer.run());

    // ###########################
    // # Main event loop handler #
    // ###########################
    let mut main_event_handler = MainEventLoop::new(chrononauts_event_loop, chrononauts_id);
    main_event_handler.run()
}

struct MainEventLoop {
    chrononauts_event_loop: ChrononautsEventLoop,
    chrononauts_id: ChrononautsId,
    game_level: Level,
    button: DebounceButton,
}

impl MainEventLoop {
    fn new(chrononauts_event_loop: ChrononautsEventLoop, chrononauts_id: ChrononautsId) -> Self {
        let button = DebounceButton::new();
        Self {
            chrononauts_event_loop,
            game_level: Level::L0,
            chrononauts_id,
            button,
        }
    }

    fn handle_message(&mut self, mut msg: ChrononautsMessage) {
        log::info!("Message received: {:?}", msg);
        match msg.source() {
            MessageSource::Backend => {
                if let MessagePayload::SetGameLevel(level) = msg.payload() {
                    log::info!("[MAIN_EVENT]: Received SetGameLevel({level:?}) from Backend");
                    self.game_level = level;

                    msg.change_source(MessageSource::Board);
                    self.chrononauts_event_loop
                        .post::<MessageTransmissionEvent>(
                            &MessageTransmissionEvent::Message(msg),
                            delay::BLOCK,
                        )
                        .unwrap();
                }
            }
            MessageSource::Board => {
                if let MessagePayload::SetGameLevel(level) = msg.payload() {
                    log::info!("[MAIN_EVENT]: Received SetGameLevel from Wifi-Board");
                    self.game_level = level;
                }
                if let MessagePayload::LabyrinthAction(_) = msg.payload() {
                    log::info!("[MAIN_EVENT]: Received LabyrinthAction from Wifi-Board");
                    if let Level::L4 = self.game_level {
                        self.chrononauts_event_loop
                            .post::<WsTransmissionEvent>(
                                &WsTransmissionEvent::Send(msg),
                                delay::BLOCK,
                            )
                            .unwrap();
                    }
                }
            }
        }
    }

    fn handle_accelerometer_direction(
        &mut self,
        direction: Direction,
    ) -> Result<(), ChrononautsError> {
        let Level::L4 = self.game_level else {
            return Ok(());
        };

        // Handle the accelerometer direction change
        log::info!("Accelerometer direction changed: {:?}", direction);
        let message = ChrononautsMessage::new_from_board(MessagePayload::LabyrinthAction(
            backend_api::labyrinth::Action {
                device: self.chrononauts_id.into(),
                direction,
                step: false,
            },
        ));
        if let ChrononautsId::L = self.chrononauts_id {
            self.chrononauts_event_loop
                .post::<WsTransmissionEvent>(&WsTransmissionEvent::Send(message), delay::BLOCK)?;
        } else {
            self.chrononauts_event_loop
                .post::<MessageTransmissionEvent>(
                    &MessageTransmissionEvent::Message(message),
                    delay::BLOCK,
                )
                .unwrap();
        }
        Ok(())
    }

    fn run(&mut self) -> Result<(), ChrononautsError> {
        block_on(pin!(async move {
            let mut subscription = self.chrononauts_event_loop.subscribe_async::<MainEvent>()?;

            loop {
                let event = subscription.recv().await?;
                match event {
                    MainEvent::ButtonChanged(state) => {
                        if self.button.debounce_button(state)? {
                            log::info!("Button pressed");
                        }
                    }
                    MainEvent::WifiConnected => {
                        self.chrononauts_event_loop.post::<WsTransmissionEvent>(
                            &WsTransmissionEvent::Connect,
                            delay::BLOCK,
                        )?;
                    }
                    MainEvent::MessageReceived(msg) => self.handle_message(msg),
                    MainEvent::AccelerometerDirectionChanged(direction) => {
                        self.handle_accelerometer_direction(direction)?;
                    }
                }
            }
        }))
    }
}
