/// Message sent to or from the Chrononauts board.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "@type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoardMessage {
    /// Sent by backend immediately upon accepting a new connection.
    GameState(GameState),
    /// Sent by the board to indicate that a player is moving (or turning).
    ///
    /// Only accepted in [`Level::L4`].
    LabyrinthAction(LabyrinthAction),
}

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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    pub level: Level,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoardId {
    Player1,
    Player2,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LabyrinthAction {
    /// Which board is taking the action.
    pub board: BoardId,
    /// Direction the player is facing.
    pub direction: Direction,
    /// Whether the player is taking a step.
    #[serde(skip_serializing_if = "is_false")]
    pub step: bool,
}



const fn is_false(x: &bool) -> bool {
    return !*x;
}
