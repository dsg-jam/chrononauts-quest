use std::time::Duration;

use backend_api::DeviceId;
use esp_idf_svc::timer::EspTimerService;

use crate::ChrononautsError;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum ChrononautsId {
    T = 0,
    L = 1,
}

impl ChrononautsId {
    pub fn other(&self) -> Self {
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

impl From<ChrononautsId> for DeviceId {
    fn from(val: ChrononautsId) -> Self {
        match val {
            ChrononautsId::L => DeviceId::Player1,
            ChrononautsId::T => DeviceId::Player2,
        }
    }
}

pub fn get_chrononauts_id() -> Result<ChrononautsId, ChrononautsError> {
    let chrononauts_id = env!("CHRONONAUTS_ID")
        .parse::<u8>()
        .expect("Chrononauts ID MUST be set");
    ChrononautsId::try_from(chrononauts_id)
}

pub struct DebounceButton {
    previous_level: bool,
    button_state: bool,
    last_change_time: Duration,
}

impl DebounceButton {
    pub fn new() -> Self {
        Self {
            previous_level: false,
            button_state: false,
            last_change_time: Duration::from_secs(0),
        }
    }

    pub fn debounce_button(&mut self, current_level: bool) -> Result<bool, ChrononautsError> {
        let system_time = EspTimerService::new()?;
        if system_time.now() - self.last_change_time > Duration::from_millis(50) {
            self.last_change_time = system_time.now();
            if current_level == self.previous_level {
                return Ok(false);
            }
            self.previous_level = current_level;
            // the reading is same for the last 50 ms
            // if the button state has changed:
            if current_level != self.button_state {
                self.button_state = current_level;
                if !self.button_state {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
