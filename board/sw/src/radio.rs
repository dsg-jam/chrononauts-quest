//! Radio module for the Chrononauts board
//!
//! This module contains the radio implementation for the Chrononauts board.
//! The radio module consists of all required parts to exchange messages between two boards reliably.
//!

mod message;
mod packet;
mod transceiver;
mod transport;

use cc1101::Error;
use esp_idf_svc::hal::spi::SpiError;
pub use message::{ChrononautsMessage, MessageError, MessagePayload, MessageSource};
pub use packet::ChrononautsPacket;
use packet::PacketError;
pub use transceiver::ChrononautsTransceiver;
pub use transport::ChrononautsTransport;
use transport::TransportError;

#[derive(Debug, thiserror::Error)]
pub enum RadioError {
    #[error("Empty payload")]
    EmptyPayload,
    #[error("Radio not found")]
    RadioNotFound,
    #[error(transparent)]
    TransportError(#[from] TransportError),
    #[error(transparent)]
    PacketError(#[from] PacketError),
    #[error(transparent)]
    SpiError(#[from] Error<SpiError>),
    #[error(transparent)]
    PostcardError(#[from] postcard::Error),
    #[error(transparent)]
    EspError(#[from] esp_idf_svc::sys::EspError),
}
