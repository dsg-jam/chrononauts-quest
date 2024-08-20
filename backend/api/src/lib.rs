pub mod labyrinth;

/// Message sent to or from the Chrononauts board.
#[cfg(feature = "board")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "@type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoardMessage {
    /// Sent by backend immediately upon accepting a new connection.
    GameState(GameState),
    /// Sent by the board to indicate that a player is moving (or turning).
    ///
    /// Only accepted in [`Level::L4`].
    LabyrinthAction(labyrinth::Action),
}

/// Message sent to or from the website.
#[cfg(feature = "website")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "@type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebMessage {
    /// Sent by backend immediately upon accepting a new connection.
    GameState(GameState),
    /// Sent by the backend when the labyrinth state changes.
    ///
    /// Only sent in [`Level::L4`].
    LabyrinthState(labyrinth::FullState),
}

#[cfg(feature = "shared")]
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Level {
    /// Unlock the website using password from QR code.
    L0,
    /// Connecting the board to wifi.
    L1,
    /// Establish connection between the boards by tuning the frequency.
    L2,
    /// Decipher encryption key from morse code.
    L3,
    /// Labyrinth
    L4,
}

#[cfg(feature = "shared")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    pub level: Level,
}
