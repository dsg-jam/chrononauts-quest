//! Packet module
//!
//! This module contains the definition of the ChrononautsPacket struct, which is used to represent the packets that can be sent between the Chrononauts Boards
//!
//! # Packet Structure
//!
//! The package length is limited to 61 bytes, as the maximum packet size is 64 bytes,
//! with 3 bytes reserved for the length and RSSI/LQI.
//!
//! The package is structured as follows:
//! Header (3 bytes) | Payload (max 58 bytes)
//!

use serde::{Deserialize, Serialize};

use super::ChrononautsMessage;

#[derive(Debug, thiserror::Error)]
pub enum PacketError {}

/// ChrononautsPacket struct
/// Represents a Chrononauts message, which is composed of a header and a payload.
///
/// The package length is limited to 61 bytes, as the maximum packet size is 64 bytes,
/// with 3 bytes reserved for the length and RSSI/LQI.
///
/// The package is structured as follows:
/// Header (3 bytes) | Payload (max 58 bytes)
///
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Copy)]
pub struct ChrononautsPacket {
    pub header: ChrononautsHeader,
    pub payload: Option<ChrononautsMessage>,
}

impl ChrononautsPacket {
    pub fn from_message(
        source: u8,
        destination: u8,
        sequence_number: u8,
        message: ChrononautsMessage,
    ) -> Self {
        ChrononautsPacket {
            header: ChrononautsHeader::new(source, destination, sequence_number, false),
            payload: Some(message),
        }
    }

    pub fn new_ack_from(received_package: &ChrononautsPacket) -> Self {
        ChrononautsPacket {
            header: ChrononautsHeader {
                source: received_package.header.destination,
                destination: received_package.header.source,
                sequence_number: received_package.header.sequence_number,
                is_ack: true,
            },
            payload: None,
        }
    }

    pub fn get_payload(&self) -> Option<ChrononautsMessage> {
        self.payload
    }

    pub fn is_ack(&self) -> bool {
        self.header.is_ack
    }

    pub fn matches_sequence(&self, rhs: &ChrononautsPacket) -> bool {
        self.header.sequence_number == rhs.header.sequence_number
    }

    pub fn get_sequence(&self) -> u8 {
        self.header.sequence_number
    }

    pub fn matches_destination(&self, expected: u8) -> bool {
        self.header.destination == expected
    }
}

/// ChrononautsHeader struct
/// Represents the header of a Chrononauts message.
///
/// The header is composed of 3 bytes:
/// - Source (1 byte)
/// - Destination (1 byte)
/// - Sequence number (1 byte)
/// - Payload length (1 byte)
///
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Copy)]
pub struct ChrononautsHeader {
    source: u8,
    destination: u8,
    sequence_number: u8,
    is_ack: bool,
}

impl ChrononautsHeader {
    pub fn new(source: u8, destination: u8, sequence_number: u8, is_ack: bool) -> Self {
        ChrononautsHeader {
            source,
            destination,
            sequence_number,
            is_ack,
        }
    }
}
