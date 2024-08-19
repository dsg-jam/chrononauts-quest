#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct LabyrinthState {
    #[serde(default)]
    pub player1: Option<Position>,
    #[serde(default)]
    pub player2: Option<Position>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Position {
    x: u32,
    y: u32,
}
