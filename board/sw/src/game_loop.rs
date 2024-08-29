use std::pin::pin;

use backend_api::{labyrinth::Direction, Level};
use esp_idf_svc::hal::{delay, task::block_on};

use crate::{
    communication::{ChrononautsMessage, MessagePayload, MessageSource},
    event::{GameLoopEvent, MainEvent, MessageTransmissionEvent, WsTransmissionEvent},
    utils::{ChrononautsId, DebounceButton},
    ChrononautsError, ChrononautsEventLoop,
};

pub struct GameLoop {
    chrononauts_event_loop: ChrononautsEventLoop,
    chrononauts_id: ChrononautsId,
    game_level: Level,
    labyrinth_dir: Direction,
    button: DebounceButton,
    l2_my_led_speed: u16,
    l2_other_led_speed: Option<u16>,
}

impl GameLoop {
    pub fn new(
        chrononauts_event_loop: ChrononautsEventLoop,
        chrononauts_id: ChrononautsId,
    ) -> Self {
        let button = DebounceButton::new();
        Self {
            chrononauts_event_loop,
            game_level: Level::L0,
            chrononauts_id,
            labyrinth_dir: Direction::Up,
            button,
            l2_my_led_speed: 0,
            l2_other_led_speed: None,
        }
    }

    fn handle_level_change(&mut self) -> Result<(), ChrononautsError> {
        self.chrononauts_event_loop.post::<GameLoopEvent>(
            &GameLoopEvent::GameLevelChanged(self.game_level),
            delay::BLOCK,
        )?;
        Ok(())
    }

    fn handle_backend_message(&mut self, msg: ChrononautsMessage) -> Result<(), ChrononautsError> {
        if let MessagePayload::SetGameLevel(level) = msg.payload() {
            self.game_level = level;
            self.send_to_board(msg)?;
            self.handle_level_change()?;
        }
        Ok(())
    }

    fn handle_board_message(&mut self, msg: ChrononautsMessage) -> Result<(), ChrononautsError> {
        match msg.payload() {
            MessagePayload::SetGameLevel(level) => {
                self.game_level = level;
                self.handle_level_change()?;
            }
            MessagePayload::LabyrinthAction(_action) => {
                if let Level::L4 = self.game_level {
                    self.send_to_backend(msg)?;
                }
            }
            MessagePayload::LedSpeed(speed) => {
                self.l2_other_led_speed = Some(speed);
            }
            MessagePayload::ShowEncryptionKey => {
                let Level::L3 = self.game_level else {
                    return Ok(());
                };
                self.chrononauts_event_loop
                    .post::<GameLoopEvent>(&GameLoopEvent::ShowEncryptionKey, delay::BLOCK)?;
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

    fn send_message(&mut self, msg: ChrononautsMessage) -> Result<(), ChrononautsError> {
        if let ChrononautsId::L = self.chrononauts_id {
            self.chrononauts_event_loop
                .post::<WsTransmissionEvent>(&WsTransmissionEvent::Send(msg), delay::BLOCK)?;
        } else {
            self.chrononauts_event_loop
                .post::<MessageTransmissionEvent>(
                    &MessageTransmissionEvent::Message(msg),
                    delay::BLOCK,
                )?;
        }
        Ok(())
    }

    fn l2_speeds_match(&self) -> bool {
        self.l2_my_led_speed == self.l2_other_led_speed.unwrap_or_default()
    }

    fn handle_button_press(&mut self) -> Result<(), ChrononautsError> {
        match self.game_level {
            Level::L2 => {
                if let ChrononautsId::L = self.chrononauts_id {
                    if self.l2_speeds_match() {
                        let msg =
                            ChrononautsMessage::new_from_board(MessagePayload::FrequencyTuned);
                        self.send_to_backend(msg)?;
                    } else {
                        log::info!("Speeds do not match");
                    }
                }
            }
            Level::L3 => {
                let msg = ChrononautsMessage::new_from_board(MessagePayload::ShowEncryptionKey);
                self.send_to_board(msg)?;
            }
            Level::L4 => {
                let message = ChrononautsMessage::new_from_board(MessagePayload::LabyrinthAction(
                    backend_api::labyrinth::Action {
                        device: self.chrononauts_id.into(),
                        direction: self.labyrinth_dir,
                        step: true,
                    },
                ));
                self.send_message(message)?;
            }
            _ => {}
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

        self.labyrinth_dir = direction;

        let message = ChrononautsMessage::new_from_board(MessagePayload::LabyrinthAction(
            backend_api::labyrinth::Action {
                device: self.chrononauts_id.into(),
                direction,
                step: false,
            },
        ));
        self.send_message(message)?;
        Ok(())
    }

    fn handle_event(&mut self, event: MainEvent) -> Result<(), ChrononautsError> {
        match event {
            MainEvent::ButtonChanged(state) => {
                if self.button.debounce_button(state)? {
                    self.handle_button_press()?;
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
                let led_speed = poti_to_led_speed(value);
                if led_speed == self.l2_my_led_speed {
                    return Ok(());
                }
                self.l2_my_led_speed = led_speed;
                self.chrononauts_event_loop
                    .post::<GameLoopEvent>(
                        &GameLoopEvent::SetLedBlinkSpeed(1, led_speed),
                        delay::BLOCK,
                    )
                    .unwrap();
                if let ChrononautsId::T = self.chrononauts_id {
                    let msg =
                        ChrononautsMessage::new_from_board(MessagePayload::LedSpeed(led_speed));
                    self.send_to_board(msg)?;
                }
            }
        }
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), ChrononautsError> {
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

fn poti_to_led_speed(value: u16) -> u16 {
    let val = match value {
        0..600 => 4000,
        600..1300 => 2000,
        1300..2000 => 1000,
        2000..2700 => 500,
        2700.. => 250,
    };
    log::info!("Poti value: {}", val);
    val
}

trait BoardTransmitter {
    fn send_to_board(&self, msg: ChrononautsMessage) -> Result<(), ChrononautsError>;
}

impl BoardTransmitter for GameLoop {
    fn send_to_board(&self, mut msg: ChrononautsMessage) -> Result<(), ChrononautsError> {
        msg.change_source(MessageSource::Board);
        self.chrononauts_event_loop
            .post::<MessageTransmissionEvent>(
                &MessageTransmissionEvent::Message(msg),
                delay::BLOCK,
            )?;
        Ok(())
    }
}

impl BackendTransmitter for GameLoop {
    fn send_to_backend(&self, msg: ChrononautsMessage) -> Result<(), ChrononautsError> {
        self.chrononauts_event_loop
            .post::<WsTransmissionEvent>(&WsTransmissionEvent::Send(msg), delay::BLOCK)?;
        Ok(())
    }
}

trait BackendTransmitter {
    fn send_to_backend(&self, msg: ChrononautsMessage) -> Result<(), ChrononautsError>;
}
