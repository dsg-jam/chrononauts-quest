use std::{
    pin::pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use backend_api::Level;
use esp_idf_svc::{
    hal::{
        adc::{
            attenuation::DB_11,
            oneshot::{config::AdcChannelConfig, AdcChannelDriver, AdcDriver},
        },
        delay::BLOCK,
        gpio::ADCPin,
        peripheral::Peripheral,
        task::block_on,
    },
    io::EspIOError,
    sys::EspError,
    timer::EspTaskTimerService,
};

use crate::{
    event::{GameLoopEvent, MainEvent},
    ChrononautsEventLoop,
};

#[derive(Debug, thiserror::Error)]
pub enum PotentiometerError {
    #[error(transparent)]
    EspIOError(#[from] EspIOError),
    #[error(transparent)]
    EspError(#[from] EspError),
}

pub struct ChrononautsPotentiometer<T>
where
    T: ADCPin + 'static,
{
    adc: AdcDriver<'static, T::Adc>,
    event_loop: ChrononautsEventLoop,
}

impl<T> ChrononautsPotentiometer<T>
where
    T: ADCPin,
{
    pub fn new(
        adc: AdcDriver<'static, T::Adc>,
        event_loop: ChrononautsEventLoop,
    ) -> Result<Self, PotentiometerError> {
        Ok(Self { adc, event_loop })
    }

    pub fn run(
        &mut self,
        adc_pin: impl Peripheral<P = T> + 'static,
    ) -> Result<(), PotentiometerError> {
        let game_level = Arc::new(Mutex::new(Level::L0));
        let _game_loop_sub = {
            let game_level = game_level.clone();
            self.event_loop
                .subscribe::<GameLoopEvent, _>(move |event| {
                    if let GameLoopEvent::GameLevelChanged(level) = event {
                        *game_level.lock().unwrap() = level;
                    }
                })
                .unwrap()
        };

        let timer_service = EspTaskTimerService::new()?;
        block_on(pin!(async move {
            let mut async_timer = timer_service.timer_async()?;
            let config = AdcChannelConfig {
                attenuation: DB_11,
                calibration: true,
                ..Default::default()
            };
            let mut poti = AdcChannelDriver::new(&self.adc, adc_pin, &config).unwrap();
            loop {
                let game_level = *game_level.lock().unwrap();
                if game_level == Level::L2 {
                    let poti_value = self.adc.read(&mut poti).unwrap().saturating_add(100);
                    self.event_loop
                        .post::<MainEvent>(&MainEvent::PotentiometerValueChanged(poti_value), BLOCK)
                        .unwrap();
                }
                async_timer.after(Duration::from_millis(100)).await.unwrap();
            }
        }))
    }
}
