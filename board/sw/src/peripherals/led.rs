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
    timer::EspTaskTimerService,
};

use crate::{event::GameLoopEvent, ChrononautsEventLoop};

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
        Ok(Self {
            led,
            led_number,
            state,
            event_loop,
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

    pub fn run(&mut self) -> Result<(), LedError> {
        let game_level = Arc::new(Mutex::new(Level::L0));
        let blink_speed = Arc::new(Mutex::new(None));
        let expected_state = Arc::new(Mutex::new(false));
        let _game_loop_sub = {
            let game_level = game_level.clone();
            let blink_speed = blink_speed.clone();
            let expected_state = expected_state.clone();
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
                    _ => {}
                })
                .unwrap()
        };

        let timer_service = EspTaskTimerService::new()?;
        block_on(pin!(async move {
            let mut async_timer = timer_service.timer_async()?;
            loop {
                let blink_speed = *blink_speed.lock().unwrap();
                let game_level = *game_level.lock().unwrap();
                match (game_level, blink_speed) {
                    (Level::L2, Some(speed)) => {
                        async_timer
                            .after(Duration::from_millis(speed as u64))
                            .await
                            .unwrap();
                        self.toggle()?;
                        continue;
                    }
                    _ => {
                        let expected_state = *expected_state.lock().unwrap();
                        self.set_state(expected_state)?;
                    }
                }
                async_timer.after(Duration::from_millis(100)).await.unwrap();
            }
        }))
    }
}
