use crate::captive::CaptivePortal;
use backend_api::{labyrinth::Direction, Level};
use consts::AP_IP_ADDRESS;
use core::pin::pin;
use dns::*;
use esp_idf_svc::{
    eventloop::{Background, EspBackgroundEventLoop, EspEventLoop, EspSystemEventLoop, User},
    hal::{
        adc::oneshot::AdcDriver,
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
use event::{GameLoopEvent, MainEvent, MessageTransmissionEvent, WsTransmissionEvent};
use http_server::ChrononautsHttpServer;
use peripherals::{
    ChrononautsAccelerometer, ChrononautsLed, ChrononautsPotentiometer, PeripheralError,
};
use radio::{ChrononautsMessage, MessagePayload, MessageSource};
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
    let pins = peripherals.pins;
    
    let (wifi_runner_tx, wifi_runner_rx) = mpsc::channel::<WifiRunner>();
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

    let mut push_button = PinDriver::input(pins.gpio9)?;
    push_button.set_pull(Pull::Up)?;

    let led1 = PinDriver::output(pins.gpio3)?;
    let led2 = PinDriver::output(pins.gpio1)?;

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
    let mut radio_transceiver =
        radio::ChrononautsTransceiver::new(cc1101, chrononauts_event_loop.clone());
    let _radio_transceiver_handler = thread::spawn(move || radio_transceiver.run(chrononauts_id));

    // ###########################
    // # Radio transport handler #
    // ###########################
    let mut radio_transport =
        radio::ChrononautsTransport::new(chrononauts_event_loop.clone(), chrononauts_id);
    let _radio_transport_handler = { thread::spawn(move || radio_transport.run()) };

    // #######################
    // # HTTP server handler #
    // #######################
    let _http_server_handler = if let ChrononautsId::L = chrononauts_id {
        let mut http_server =
            ChrononautsHttpServer::new(wifi_runner_tx.clone(), wifi_available_ssids);
        http_server.setup().expect("HTTP server setup failed");

        CaptivePortal::attach(&mut http_server, AP_IP_ADDRESS)
            .expect("Captive portal attach failed");
        Some(http_server)
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
    let mut potentiometer = ChrononautsPotentiometer::new(poti_adc, chrononauts_event_loop.clone())
        .expect("Potentiometer init failed");
    let _turning_knob_handler = { thread::spawn(move || potentiometer.run(pins.gpio0)) };

    // ################
    // # LEDs handler #
    // ################
    let mut led1 =
        ChrononautsLed::new(led1, 1, chrononauts_event_loop.clone()).expect("LED1 init failed");
    let _led1_handler = thread::spawn(move || led1.run());
    let mut led2 =
        ChrononautsLed::new(led2, 2, chrononauts_event_loop.clone()).expect("LED2 init failed");
    let _led2_handler = thread::spawn(move || led2.run());

    // #########################
    // # Accelerometer handler #
    // #########################
    let mut accelerometer =
        ChrononautsAccelerometer::new(i2c_driver, chrononauts_event_loop.clone());
    let _accelerometer_handler = thread::spawn(move || accelerometer.run());

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

    fn handle_backend_message(
        &mut self,
        mut msg: ChrononautsMessage,
    ) -> Result<(), ChrononautsError> {
        if let MessagePayload::SetGameLevel(level) = msg.payload() {
            log::info!("[MAIN_EVENT]: Received SetGameLevel({level:?}) from Backend");
            self.game_level = level;

            msg.change_source(MessageSource::Board);
            self.chrononauts_event_loop
                .post::<MessageTransmissionEvent>(
                    &MessageTransmissionEvent::Message(msg),
                    delay::BLOCK,
                )?;
        }
        Ok(())
    }

    fn handle_board_message(&mut self, msg: ChrononautsMessage) -> Result<(), ChrononautsError> {
        match msg.payload() {
            MessagePayload::SetGameLevel(level) => {
                log::info!("[MAIN_EVENT]: Received SetGameLevel from Wifi-Board");
                self.game_level = level;
            }
            MessagePayload::LabyrinthAction(_action) => {
                log::info!("[MAIN_EVENT]: Received LabyrinthAction from Wifi-Board");
                if let Level::L4 = self.game_level {
                    self.chrononauts_event_loop
                        .post::<WsTransmissionEvent>(&WsTransmissionEvent::Send(msg), delay::BLOCK)
                        .unwrap();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_message(&mut self, msg: ChrononautsMessage) -> Result<(), ChrononautsError> {
        match msg.source() {
            MessageSource::Backend => {
                self.handle_backend_message(msg)?;
            }
            MessageSource::Board => {
                self.handle_board_message(msg)?;
            }
        }
        Ok(())
    }

    fn handle_accelerometer_direction(
        &mut self,
        direction: Direction,
    ) -> Result<(), ChrononautsError> {
        let Level::L4 = self.game_level else {
            return Ok(());
        };

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

    fn handle_event(&mut self, event: MainEvent) -> Result<(), ChrononautsError> {
        match event {
            MainEvent::ButtonChanged(state) => {
                if self.button.debounce_button(state)? {
                    log::info!("Button pressed");
                }
            }
            MainEvent::WifiConnected => {
                self.chrononauts_event_loop
                    .post::<WsTransmissionEvent>(&WsTransmissionEvent::Connect, delay::BLOCK)?;
            }
            MainEvent::MessageReceived(msg) => self.handle_message(msg)?,
            MainEvent::AccelerometerDirectionChanged(direction) => {
                self.handle_accelerometer_direction(direction)?;
            }
            MainEvent::PotentiometerValueChanged(value) => {
                log::info!("Potentiometer value changed: {}", value);
                self.chrononauts_event_loop
                    .post::<GameLoopEvent>(&GameLoopEvent::SetLedBlinkSpeed(1, value), delay::BLOCK)
                    .unwrap();
            }
        }
        Ok(())
    }

    fn run(&mut self) -> Result<(), ChrononautsError> {
        block_on(pin!(async move {
            let mut subscription = self.chrononauts_event_loop.subscribe_async::<MainEvent>()?;

            while let Ok(event) = subscription.recv().await {
                self.handle_event(event)?;
            }

            // TODO: Handle error
            Ok(())
        }))
    }
}
