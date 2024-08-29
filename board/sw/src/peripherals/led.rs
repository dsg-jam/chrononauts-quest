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

use crate::{consts, event::GameLoopEvent, ChrononautsEventLoop};

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

    /// Show a space between components (dot or dash)
    ///
    /// This is a 1 unit pause
    async fn show_inter_component_pause(
        &mut self,
        async_timer: &mut EspAsyncTimer,
    ) -> Result<(), LedError> {
        self.set_low()?;
        async_timer.after(Duration::from_millis(1000)).await?;
        Ok(())
    }

    /// Show a space between letters
    ///
    /// This is a 3 units pause
    async fn show_inter_letter_pause(
        &mut self,
        async_timer: &mut EspAsyncTimer,
    ) -> Result<(), LedError> {
        self.set_low()?;
        async_timer.after(Duration::from_millis(2000)).await?;
        Ok(())
    }

    async fn show_time_pulse(&mut self, async_timer: &mut EspAsyncTimer) -> Result<(), LedError> {
        self.set_low()?;
        async_timer.after(Duration::from_millis(400)).await?;
        self.set_high()?;
        async_timer.after(Duration::from_millis(200)).await?;
        self.set_low()?;
        async_timer.after(Duration::from_millis(400)).await?;
        Ok(())
    }

    async fn show_dot(&mut self, async_timer: &mut EspAsyncTimer) -> Result<(), LedError> {
        self.set_high()?;
        async_timer.after(Duration::from_millis(1000)).await?;
        Ok(())
    }

    async fn show_dash(&mut self, async_timer: &mut EspAsyncTimer) -> Result<(), LedError> {
        self.set_high()?;
        async_timer.after(Duration::from_millis(3000)).await?;
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

        let timer_service = EspTaskTimerService::new()?;
        block_on(pin!(async move {
            let mut async_timer = timer_service.timer_async()?;
            loop {
                let blink_speed = *blink_speed.lock().unwrap();
                let game_level = *game_level.lock().unwrap();
                let show_l3 = *show_encryption_key.lock().unwrap();
                *show_encryption_key.lock().unwrap() = false;
                match game_level {
                    Level::L2 => {
                        if let Some(speed) = blink_speed {
                            async_timer
                                .after(Duration::from_millis(speed as u64))
                                .await?;
                            self.toggle()?;
                            continue;
                        }
                    }
                    Level::L3 => {
                        if show_l3 {
                            for c in consts::L3_ENCODED_KEY.chars() {
                                if self.led_number == 1 {
                                    match c {
                                        '.' => {
                                            self.show_dot(&mut async_timer).await?;
                                            self.show_inter_component_pause(&mut async_timer)
                                                .await?;
                                        }
                                        '-' => {
                                            self.show_dash(&mut async_timer).await?;
                                            self.show_inter_component_pause(&mut async_timer)
                                                .await?;
                                        }
                                        _ => {
                                            self.show_inter_letter_pause(&mut async_timer).await?;
                                        }
                                    }
                                } else {
                                    match c {
                                        '.' => {
                                            self.show_time_pulse(&mut async_timer).await?;
                                            self.show_time_pulse(&mut async_timer).await?;
                                        }
                                        '-' => {
                                            self.show_time_pulse(&mut async_timer).await?;
                                            self.show_time_pulse(&mut async_timer).await?;
                                            self.show_time_pulse(&mut async_timer).await?;
                                            self.show_time_pulse(&mut async_timer).await?;
                                        }
                                        _ => {
                                            self.show_time_pulse(&mut async_timer).await?;
                                            self.show_time_pulse(&mut async_timer).await?;
                                        }
                                    }
                                }
                            }
                            self.set_low()?;
                            continue;
                        }
                    }
                    _ => {}
                }
                let expected_state = *expected_state.lock().unwrap();
                self.set_state(expected_state)?;
                async_timer.after(Duration::from_millis(100)).await.unwrap();
            }
        }))
    }
}
