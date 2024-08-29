use backend_api::labyrinth::{Direction, PlayerState, Position};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
enum Tile {
    #[serde(rename = "#")]
    Wall,
    #[serde(rename = " ")]
    Empty,
}

#[derive(Debug, Clone)]
pub struct LabyrinthMap {
    tiles: Vec<Vec<Tile>>,
    pub player1_start_state: PlayerState,
    pub player2_start_state: PlayerState,
}

impl Serialize for LabyrinthMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        // only inlcude the tiles in the serialized output
        self.tiles
            .iter()
            .enumerate()
            .map(|(y, row)| {
                let mut line = row
                    .iter()
                    .enumerate()
                    .map(|(x, tile)| {
                        let mut tile = match tile {
                            Tile::Wall => '#',
                            Tile::Empty => ' ',
                        };
                        if self.player1_start_state.position.x == x as u8
                            && self.player1_start_state.position.y == y as u8
                        {
                            tile = '1';
                        }
                        if self.player2_start_state.position.x == x as u8
                            && self.player2_start_state.position.y == y as u8
                        {
                            tile = '2';
                        }
                        tile
                    })
                    .collect::<String>();
                line.push('\n');
                line
            })
            .collect::<String>()
            .serialize(serializer)
    }
}

impl<'a> Deserialize<'a> for LabyrinthMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(&s))
    }
}

impl Default for LabyrinthMap {
    fn default() -> Self {
        Self::new(SAMPLE_LABYRINTH)
    }
}

impl LabyrinthMap {
    pub fn new(input: &str) -> Self {
        let mut player1_start_state = PlayerState {
            position: Position { x: 0, y: 0 },
            direction: Direction::Up,
        };
        let mut player2_start_state = PlayerState {
            position: Position { x: 0, y: 0 },
            direction: Direction::Up,
        };
        let tiles = input
            .lines()
            .filter(|line| !line.trim().is_empty())
            .enumerate()
            .map(|(y, line)| {
                line.chars()
                    .enumerate()
                    .map(|(x, c)| match c {
                        ' ' => Tile::Empty,
                        '1' => {
                            player1_start_state.position = Position {
                                x: x as u8,
                                y: y as u8,
                            };
                            Tile::Empty
                        }
                        '2' => {
                            player2_start_state.position = Position {
                                x: x as u8,
                                y: y as u8,
                            };
                            Tile::Empty
                        }
                        _ => Tile::Wall,
                    })
                    .collect()
            })
            .collect();
        Self {
            tiles,
            player1_start_state,
            player2_start_state,
        }
    }

    pub fn try_move(&self, position: Position, direction: Direction) -> Option<Position> {
        let Position { mut x, mut y } = position;
        match direction {
            Direction::Up => {
                if y == 0 {
                    return None;
                }
                y -= 1;
            }
            Direction::Down => {
                if y == self.tiles.len() as u8 - 1 {
                    return None;
                }
                y += 1;
            }
            Direction::Left => {
                if x == 0 {
                    return None;
                }
                x -= 1;
            }
            Direction::Right => {
                if x == self.tiles[0].len() as u8 - 1 {
                    return None;
                }
                x += 1;
            }
        }
        match self.tiles[y as usize][x as usize] {
            Tile::Wall => None,
            Tile::Empty => Some(Position { x, y }),
        }
    }
}

const SAMPLE_LABYRINTH: &str = r#"
########################################
####1  ##########################    ###
###### ##        ##   ########### ## ###
###### ## ###### ## # ####        ## ###
##     ## ###### ## # #### ### ##### ###
## ### ##    ### ## ###    ### ##### ###
## ### ##### ###        ###### ##### ###
## ###       ### ############# ##### ###
################       #######2#########
########################################
"#;
