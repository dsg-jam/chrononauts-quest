//! # Game Loop Event
//!
//! This event is triggered when the game level changes.
//!

use backend_api::Level;
use esp_idf_svc::eventloop::{
    EspEvent, EspEventDeserializer, EspEventPostData, EspEventSerializer, EspEventSource,
};

use crate::consts;

#[allow(dead_code)]
#[derive(Clone, Debug, Copy)]
pub enum GameLoopEvent {
    GameLevelChanged(Level),
    SetLedBlinkSpeed(u8, u16),
    SetLedState(u8, bool),
    ButtonPressed,
    ShowEncryptionKey,
}

unsafe impl EspEventSource for GameLoopEvent {
    fn source() -> Option<&'static core::ffi::CStr> {
        Some(consts::GAME_LOOP_EVENT_BASE)
    }

    fn event_id() -> Option<i32> {
        Some(consts::GAME_LOOP_EVENT_ID)
    }
}

impl EspEventSerializer for GameLoopEvent {
    type Data<'a> = GameLoopEvent;

    fn serialize<F, R>(event: &Self::Data<'_>, f: F) -> R
    where
        F: FnOnce(&EspEventPostData) -> R,
    {
        f(&unsafe { EspEventPostData::new(Self::source().unwrap(), Self::event_id(), event) })
    }
}

impl EspEventDeserializer for GameLoopEvent {
    type Data<'a> = GameLoopEvent;

    fn deserialize<'a>(data: &EspEvent<'a>) -> Self::Data<'a> {
        // Just as easy as serializing
        *unsafe { data.as_payload::<GameLoopEvent>() }
    }
}
