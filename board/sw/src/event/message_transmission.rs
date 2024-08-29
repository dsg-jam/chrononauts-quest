//! # Message Transmission Event
//!
//! This event is triggered when a message should be transmitted.
//! The message is then passed to the transportation layer for further processing.
//!

use esp_idf_svc::eventloop::{
    EspEvent, EspEventDeserializer, EspEventPostData, EspEventSerializer, EspEventSource,
};

use crate::{communication::ChrononautsMessage, consts};

#[allow(dead_code)]
#[derive(Clone, Debug, Copy)]
pub enum MessageTransmissionEvent {
    Message(ChrononautsMessage),
}

unsafe impl EspEventSource for MessageTransmissionEvent {
    fn source() -> Option<&'static core::ffi::CStr> {
        Some(consts::MESSAGE_TRANSMISSION_EVENT_BASE)
    }

    fn event_id() -> Option<i32> {
        Some(consts::MESSAGE_TRANSMISSION_EVENT_ID)
    }
}

impl EspEventSerializer for MessageTransmissionEvent {
    type Data<'a> = MessageTransmissionEvent;

    fn serialize<F, R>(event: &Self::Data<'_>, f: F) -> R
    where
        F: FnOnce(&EspEventPostData) -> R,
    {
        f(&unsafe { EspEventPostData::new(Self::source().unwrap(), Self::event_id(), event) })
    }
}

impl EspEventDeserializer for MessageTransmissionEvent {
    type Data<'a> = MessageTransmissionEvent;

    fn deserialize<'a>(data: &EspEvent<'a>) -> Self::Data<'a> {
        // Just as easy as serializing
        *unsafe { data.as_payload::<MessageTransmissionEvent>() }
    }
}
