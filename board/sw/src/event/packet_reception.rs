//! # Packet Reception Event
//!
//! This event is triggered when a packet is received by the CC1101 radio module.
//! The packet is then passed to the transportation layer for further processing.
//!

use esp_idf_svc::eventloop::{
    EspEvent, EspEventDeserializer, EspEventPostData, EspEventSerializer, EspEventSource,
};

use crate::{communication::ChrononautsPacket, consts};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum PacketReceptionEvent {
    Packet(ChrononautsPacket),
}

unsafe impl EspEventSource for PacketReceptionEvent {
    fn source() -> Option<&'static core::ffi::CStr> {
        Some(consts::PACKET_RECEPTION_EVENT_BASE)
    }

    fn event_id() -> Option<i32> {
        Some(consts::PACKET_RECEPTION_EVENT_ID)
    }
}

impl EspEventSerializer for PacketReceptionEvent {
    type Data<'a> = PacketReceptionEvent;

    fn serialize<F, R>(event: &Self::Data<'_>, f: F) -> R
    where
        F: FnOnce(&EspEventPostData) -> R,
    {
        f(&unsafe { EspEventPostData::new(Self::source().unwrap(), Self::event_id(), event) })
    }
}

impl EspEventDeserializer for PacketReceptionEvent {
    type Data<'a> = PacketReceptionEvent;

    fn deserialize<'a>(data: &EspEvent<'a>) -> Self::Data<'a> {
        // Just as easy as serializing
        *unsafe { data.as_payload::<PacketReceptionEvent>() }
    }
}
