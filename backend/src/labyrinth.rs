use backend_api::labyrinth::{Direction, PlayerState, Position};

pub const PLAYER1_START_STATE: PlayerState = PlayerState {
    position: Position { x: 3, y: 8 },
    direction: Direction::Right,
};

pub const PLAYER2_START_STATE: PlayerState = PlayerState {
    position: Position { x: 9, y: 9 },
    direction: Direction::Down,
};
