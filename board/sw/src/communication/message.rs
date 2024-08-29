//! ChrononautsMessage
//!
//! This module contains the definition of the ChrononautsMessage enum, which is used to represent the messages that can be sent between the Chrononauts Boards
//! Typically a ChrononautsMessage is sent over the radio module and thus wrapped in a ChrononautsPacket.
//!
//! The messages get serialized and deserialized into/from a binary format (`PostCard`) using the serde library.
//!

use backend_api::BoardMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("Invalid board message from backend")]
    InvalidBoardMessageFromBackend,
    #[error("Invalid board message from board")]
    InvalidBoardMessageFromBoard,
}

/// The source of the message.
///
/// This is used to determine where the message originated from, as we combine messages from the backend and the board.
/// When a message arrives from the backend, the source MUST be set, when converting the message from a BoardMessage
/// to a ChrononautsMessage.
///
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Copy)]
pub enum MessageSource {
    Backend,
    Board,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Copy)]
pub enum MessagePayload {
    // This message is used to synchronize the game between two boards.
    // This message is ALWAYS sent by the board connected to WiFi.
    // The payload is the game level.
    SyncRequest(backend_api::Level),
    // This message is used to respond to a SyncRequest.
    // This message is ALWAYS sent by the board NOT connected to WiFi.
    // The payload is the game level (the same as from the request above).
    SyncResponse(u8),
    // If the board not connected to WiFi (accidentally) restarts, it will send this message to the other board.
    // This message triggers the board connected to WiFi to send a SyncRequest.
    RecoveryRequest,
    /// This message is sent EITHER by the board connected to WiFi OR the backend.
    ///
    /// The payload is the game level that the board should set.
    SetGameLevel(backend_api::Level),
    /// This message is sent EITHER by the board connected to WiFi OR the backend.
    ///
    /// The payload is the action to be performed in the labyrinth.
    /// This message is only sent in [`Level::L4`].
    LabyrinthAction(backend_api::labyrinth::Action),
    /// This message is ONLY sent from the non-WiFi board to the WiFi board.
    ///
    /// The payload is the value of the potentiometer.
    LedSpeed(u16),
    /// This message is ONLY sent from the WiFi board to the backend upon completion of `Level::L2`.
    FrequencyTuned,
    /// This message is ONLY sent between the boards in `Level::L3`.
    ///
    /// It triggers to show the encryption key on the LEDs on the opposite board
    ShowEncryptionKey,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Copy)]
pub struct ChrononautsMessage {
    source: MessageSource,
    payload: MessagePayload,
}

impl ChrononautsMessage {
    pub fn new(source: MessageSource, payload: MessagePayload) -> Self {
        Self { source, payload }
    }

    pub fn new_from_board(payload: MessagePayload) -> Self {
        Self::new(MessageSource::Board, payload)
    }

    pub fn change_source(&mut self, source: MessageSource) {
        self.source = source;
    }

    pub fn source(&self) -> MessageSource {
        self.source
    }

    pub fn payload(&self) -> MessagePayload {
        self.payload
    }
}

impl TryFrom<BoardMessage> for ChrononautsMessage {
    type Error = MessageError;

    fn try_from(board_msg: BoardMessage) -> Result<Self, Self::Error> {
        match board_msg {
            BoardMessage::GameState(game_state) => Ok(Self::new(
                MessageSource::Backend,
                MessagePayload::SetGameLevel(game_state.level),
            )),
            _ => Err(MessageError::InvalidBoardMessageFromBackend),
        }
    }
}

impl TryFrom<ChrononautsMessage> for BoardMessage {
    type Error = MessageError;

    fn try_from(chrononauts_msg: ChrononautsMessage) -> Result<Self, Self::Error> {
        match chrononauts_msg.payload {
            MessagePayload::LabyrinthAction(action) => Ok(BoardMessage::LabyrinthAction(action)),
            MessagePayload::FrequencyTuned => Ok(BoardMessage::FrequencyTuned),
            _ => Err(MessageError::InvalidBoardMessageFromBoard),
        }
    }
}
