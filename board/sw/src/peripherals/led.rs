use std::{
    pin::pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use backend_api::Level;
use esp_idf_svc::{
    hal::{
        gpio::{Output, Pin, PinDriver},
        task::block_on,
    },
    io::EspIOError,
    sys::EspError,
    timer::{EspAsyncTimer, EspTaskTimerService},
};

use crate::{
    consts::{self, LED_MORSE_UNIT_MS},
    event::GameLoopEvent,
    ChrononautsEventLoop,
};

#[derive(Debug, thiserror::Error)]
pub enum LedError {
    #[error(transparent)]
    EspIOError(#[from] EspIOError),
    #[error(transparent)]
    EspError(#[from] EspError),
}

type Led<T> = PinDriver<'static, T, Output>;

pub struct ChrononautsLed<T>
where
    T: Pin,
{
    led: Led<T>,
    led_number: u8,
    state: bool,
    event_loop: ChrononautsEventLoop,
    timer: EspAsyncTimer,
}

impl<T> ChrononautsLed<T>
where
    T: Pin,
{
    pub fn new(
        mut led: Led<T>,
        led_number: u8,
        event_loop: ChrononautsEventLoop,
    ) -> Result<Self, LedError> {
        led.set_low()?;
        let state = false;
        let timer_service = EspTaskTimerService::new()?;
        let timer = timer_service.timer_async()?;
        Ok(Self {
            led,
            led_number,
            state,
            event_loop,
            timer,
        })
    }

    fn set_low(&mut self) -> Result<(), LedError> {
        self.led.set_low()?;
        self.state = false;
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), LedError> {
        self.led.set_high()?;
        self.state = true;
        Ok(())
    }

    fn toggle(&mut self) -> Result<(), LedError> {
        if self.state {
            self.set_low()
        } else {
            self.set_high()
        }
    }

    fn set_state(&mut self, state: bool) -> Result<(), LedError> {
        if state {
            self.set_high()
        } else {
            self.set_low()
        }
    }

    /// Show a space between components (dot or dash)
    ///
    /// This is a 1 unit pause
    async fn show_inter_component_pause(&mut self) -> Result<(), LedError> {
        self.set_low()?;
        self.wait_for(LED_MORSE_UNIT_MS).await?;
        Ok(())
    }

    /// Show a space between letters
    ///
    /// This is a 3 units pause
    async fn show_inter_letter_pause(&mut self) -> Result<(), LedError> {
        self.set_low()?;
        self.wait_for(3 * LED_MORSE_UNIT_MS).await?;
        Ok(())
    }

    async fn show_time_pulse(&mut self) -> Result<(), LedError> {
        let fifths_of_period = LED_MORSE_UNIT_MS / 5;
        self.set_low()?;
        self.wait_for(2 * fifths_of_period).await?;
        self.set_high()?;
        self.wait_for(fifths_of_period).await?;
        self.set_low()?;
        self.wait_for(2 * fifths_of_period).await?;
        Ok(())
    }

    async fn show_dot(&mut self) -> Result<(), LedError> {
        self.set_high()?;
        self.wait_for(LED_MORSE_UNIT_MS).await?;
        Ok(())
    }

    async fn show_dash(&mut self) -> Result<(), LedError> {
        self.set_high()?;
        self.wait_for(3 * LED_MORSE_UNIT_MS).await?;
        Ok(())
    }

    async fn l3(&mut self) -> Result<(), LedError> {
        let mut additional_pause = false;
        if self.led_number == 1 {
            for c in consts::L3_ENCODED_KEY.chars() {
                match c {
                    '.' => {
                        if additional_pause {
                            self.show_inter_component_pause().await?;
                        }
                        self.show_dot().await?;
                        additional_pause = true;
                    }
                    '-' => {
                        if additional_pause {
                            self.show_inter_component_pause().await?;
                        }
                        self.show_dash().await?;
                        additional_pause = true;
                    }
                    _ => {
                        self.show_inter_letter_pause().await?;
                        additional_pause = false;
                    }
                }
            }
        } else {
            for _ in 0..morse_length() {
                self.show_time_pulse().await?;
            }
        }
        self.set_low()?;
        Ok(())
    }

    async fn wait_for(&mut self, millis: u64) -> Result<(), LedError> {
        self.timer.after(Duration::from_millis(millis)).await?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), LedError> {
        let game_level = Arc::new(Mutex::new(Level::L0));
        let blink_speed = Arc::new(Mutex::new(None));
        let expected_state = Arc::new(Mutex::new(false));
        let show_encryption_key = Arc::new(Mutex::new(false));
        let _game_loop_sub = {
            let game_level = game_level.clone();
            let blink_speed = blink_speed.clone();
            let expected_state = expected_state.clone();
            let show_encryption_key = show_encryption_key.clone();
            let self_led_number = self.led_number;
            self.event_loop
                .subscribe::<GameLoopEvent, _>(move |event| match event {
                    GameLoopEvent::GameLevelChanged(level) => {
                        *game_level.lock().unwrap() = level;
                    }
                    GameLoopEvent::SetLedBlinkSpeed(led_number, speed) => {
                        if self_led_number != led_number {
                            return;
                        }
                        *blink_speed.lock().unwrap() = Some(speed);
                    }
                    GameLoopEvent::SetLedState(led_number, state) => {
                        if self_led_number != led_number {
                            return;
                        }
                        *expected_state.lock().unwrap() = state;
                    }
                    GameLoopEvent::ShowEncryptionKey => {
                        *show_encryption_key.lock().unwrap() = true;
                    }
                    _ => {}
                })
                .unwrap()
        };

        block_on(pin!(async move {
            loop {
                let blink_speed = *blink_speed.lock().unwrap();
                let game_level = *game_level.lock().unwrap();
                let show_l3 = *show_encryption_key.lock().unwrap();
                let expected_state = *expected_state.lock().unwrap();
                match game_level {
                    Level::L2 if blink_speed.is_some() => {
                        self.wait_for(blink_speed.unwrap() as u64).await?;
                        self.toggle()?;
                        continue;
                    }
                    Level::L3 if show_l3 => {
                        self.l3().await?;
                        *show_encryption_key.lock().unwrap() = false;
                    }
                    _ => {}
                }
                self.set_state(expected_state)?;
                self.wait_for(100).await?;
            }
        }))
    }
}

fn morse_length() -> usize {
    let mut additional_pause = false;
    consts::L3_ENCODED_KEY
        .chars()
        .fold(0, |mut acc, x| match x {
            '.' => {
                if additional_pause {
                    acc += 1;
                }
                additional_pause = true;
                acc + 1
            }
            '-' => {
                if additional_pause {
                    acc += 1;
                }
                additional_pause = true;
                acc + 3
            }
            _ => {
                additional_pause = false;
                acc + 3
            }
        })
}
