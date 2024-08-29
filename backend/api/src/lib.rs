pub mod labyrinth;

/// Message sent to or from the Chrononauts board.
#[cfg(feature = "board")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "@type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoardMessage {
    /// Sent by backend immediately upon accepting a new connection and whenever the state changes.
    GameState(GameState),
    /// Sent by the board to indicate that a player is moving (or turning).
    ///
    /// Only accepted in [`Level::L4`].
    LabyrinthAction(labyrinth::Action),
    /// Sent by the backend when the action sent in [`LabyrinthAction`] is rejected.
    ///
    /// Only sent in [`Level::L4`].
    LabyrinthActionRejected,
    /// Sent by the board to indicate that the frequency has been tuned.
    ///
    /// Sending this message will complete the level.
    ///
    /// Only accepted in [`Level::L2`].
    FrequencyTuned,
    /// Sent by the board to store miscellaneous log entries in the backend.
    ///
    /// This is designed to be used for telemetry and debugging purposes.
    LogEntry(LogEntry),
    /// Sent by the board to indicate the connection status of the other board.
    ///
    /// This message is sent whenever the connection status changes.
    ConnectionStatus(ConnectionStatus),
}

/// Message sent to or from the website.
#[cfg(feature = "website")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "@type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebMessage {
    /// Sent by backend immediately upon accepting a new connection and whenever the state changes.
    GameState(GameState),
    /// Sent by the backend when the labyrinth state changes.
    ///
    /// Only sent in [`Level::L4`].
    LabyrinthState(labyrinth::FullState),
    /// Sent by the website to "guess" the encryption key.
    ///
    /// Only accepted in [`Level::L3`].
    EnterEncryptionKey { key: String },
    /// Sent by the backend when the encryption key sent in [`EnterEncryptionKey`] is rejected.
    ///
    /// Only sent in [`Level::L3`].
    EncryptionKeyRejected,
}

#[cfg(feature = "shared")]
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceId {
    Player1,
    Player2,
}

/// See [`BoardMessage::LogEntry`].
#[cfg(feature = "board")]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub device: DeviceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    // add more fields as needed, the backend will store them
}

#[cfg(feature = "board")]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
}

#[cfg(feature = "shared")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Level {
    /// Unlock the website using password from QR code.
    ///
    /// This value can only be observed by the website after the very first connection.
    /// Immediately afterwards the game will transition to [`Level::L1`].
    L0,
    /// Connecting the board to wifi.
    L1,
    /// Establish connection between the boards by tuning the frequency.
    L2,
    /// Decipher encryption key from morse code.
    L3,
    /// Labyrinth.
    L4,
    /// Game over.
    Finish,
}

#[cfg(feature = "shared")]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    pub level: Level,
}
