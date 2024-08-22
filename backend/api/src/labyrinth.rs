//! Labyrinth level types.

use crate::DeviceId;

#[cfg(feature = "shared")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// See [`BoardMessage::LabyrinthAction`](crate::BoardMessage::LabyrinthAction).
#[cfg(feature = "board")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Action {
    /// Which device is taking the action.
    pub device: DeviceId,
    /// Direction the player is facing.
    pub direction: Direction,
    /// Whether the player is taking a step.
    pub step: bool,
}

/// See [`WebMessage::LabyrinthState`](crate::WebMessage::LabyrinthState).
#[cfg(feature = "website")]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FullState {
    pub player1: PlayerState,
    pub player2: PlayerState,
}

#[cfg(feature = "website")]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlayerState {
    /// The player's position on the board.
    pub position: Position,
    /// The direction the player is facing.
    pub direction: Direction,
}

#[cfg(feature = "website")]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}
