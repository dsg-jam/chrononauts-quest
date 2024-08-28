//! # Ws Event
//!
//! This event is triggered when a message should be sent to the server.
//!

use esp_idf_svc::eventloop::{
    EspEvent, EspEventDeserializer, EspEventPostData, EspEventSerializer, EspEventSource,
};

use crate::{consts, radio::ChrononautsMessage};

#[allow(dead_code)]
#[derive(Clone, Debug, Copy)]
pub enum WsTransmissionEvent {
    Send(ChrononautsMessage),
    Connect,
}

unsafe impl EspEventSource for WsTransmissionEvent {
    fn source() -> Option<&'static core::ffi::CStr> {
        Some(consts::WS_TRANSMISSION_EVENT_BASE)
    }

    fn event_id() -> Option<i32> {
        Some(consts::WS_TRANSMISSION_EVENT_ID)
    }
}

impl EspEventSerializer for WsTransmissionEvent {
    type Data<'a> = WsTransmissionEvent;

    fn serialize<F, R>(event: &Self::Data<'_>, f: F) -> R
    where
        F: FnOnce(&EspEventPostData) -> R,
    {
        f(&unsafe { EspEventPostData::new(Self::source().unwrap(), Self::event_id(), event) })
    }
}

impl EspEventDeserializer for WsTransmissionEvent {
    type Data<'a> = WsTransmissionEvent;

    fn deserialize<'a>(data: &EspEvent<'a>) -> Self::Data<'a> {
        // Just as easy as serializing
        *unsafe { data.as_payload::<WsTransmissionEvent>() }
    }
}
