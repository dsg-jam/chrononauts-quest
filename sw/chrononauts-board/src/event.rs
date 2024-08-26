use std::ffi::CStr;

use esp_idf_svc::eventloop::{
    EspEvent, EspEventDeserializer, EspEventPostData, EspEventSerializer, EspEventSource,
};

mod game_loop;
mod message_transmission;
mod packet_reception;
mod packet_transmission;

pub use game_loop::GameLoopEvent;
pub use message_transmission::MessageTransmissionEvent;
pub use packet_reception::PacketReceptionEvent;
pub use packet_transmission::PacketTransmissionEvent;

use crate::{consts, radio::ChrononautsMessage};

// Main event
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum MainEvent {
    /// This event is issued by the button task when the button state changes
    ButtonChanged(bool),
    /// This event is issued by EITHER the radio task or the websocket task when a message has been received
    MessageReceived(ChrononautsMessage),
    /// This event is issued by the wifi task when the wifi is (re)connected
    WifiConnected,
}

unsafe impl EspEventSource for MainEvent {
    fn source() -> Option<&'static core::ffi::CStr> {
        Some(CStr::from_bytes_with_nul(consts::MAIN_EVENT_BASE).unwrap())
    }

    fn event_id() -> Option<i32> {
        Some(consts::MAIN_EVENT_ID)
    }
}

impl EspEventSerializer for MainEvent {
    type Data<'a> = MainEvent;

    fn serialize<F, R>(event: &Self::Data<'_>, f: F) -> R
    where
        F: FnOnce(&EspEventPostData) -> R,
    {
        f(&unsafe { EspEventPostData::new(Self::source().unwrap(), Self::event_id(), event) })
    }
}

impl EspEventDeserializer for MainEvent {
    type Data<'a> = MainEvent;

    fn deserialize<'a>(data: &EspEvent<'a>) -> Self::Data<'a> {
        // Just as easy as serializing
        *unsafe { data.as_payload::<MainEvent>() }
    }
}
