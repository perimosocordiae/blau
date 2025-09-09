use crate::colors::Color;
use crate::player_move::Move;
use crate::player_state::PlayerState;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    #[serde(skip_deserializing)]
    #[serde(serialize_with = "as_vec_len")]
    tile_bag: Vec<Color>,
    #[serde(skip_deserializing)]
    #[serde(serialize_with = "as_vec_len")]
    box_lid: Vec<Color>,
    factories: Vec<Vec<Color>>,
    center: HashMap<Color, usize>,
    pub players: Vec<PlayerState>,
    start_player_idx: usize,
    pub curr_player_idx: usize,
    round_number: usize,
}

fn as_vec_len<S>(vec: &Vec<Color>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i64(vec.len() as i64)
}

const ALL_COLORS: [Color; 5] = [
    Color::Blue,
    Color::Orange,
    Color::Green,
    Color::Red,
    Color::Purple,
];

impl GameState {
    pub fn new(player_names: &[&str], rng: &mut impl rand::Rng) -> GameState {
        let mut tile_bag: Vec<Color> =
            (0..20).flat_map(|_x| ALL_COLORS.iter()).cloned().collect();
        tile_bag.shuffle(rng);
        let players: Vec<PlayerState> =
            player_names.iter().map(|&p| PlayerState::new(p)).collect();
        let mut center = HashMap::new();
        center.insert(Color::Start, 1);
        let num_factories = player_names.len() * 2 + 1;
        GameState {
            tile_bag,
            box_lid: vec![],
            factories: vec![vec![]; num_factories],
            center,
            players,
            start_player_idx: rng.random_range(0..player_names.len()),
            curr_player_idx: 0,
            round_number: 0,
        }
    }

    pub fn current_player(&self) -> &PlayerState {
        &self.players[self.curr_player_idx]
    }

    fn current_player_mut(&mut self) -> &mut PlayerState {
        &mut self.players[self.curr_player_idx]
    }

    pub fn is_start_token_available(&self) -> bool {
        self.start_player_idx == self.players.len()
    }

    pub fn start_round(&mut self) {
        self.curr_player_idx = self.start_player_idx;
        self.start_player_idx = self.players.len();
        // replace the start token in the center
        self.center.insert(Color::Start, 1);
        // fill factories from the tile bag
        for factory in &mut self.factories {
            for _i in 0..4 {
                if let Some(tile) = self.tile_bag.pop() {
                    factory.push(tile);
                } else {
                    // tile bag is empty, this can happen in later rounds
                    return;
                }
            }
        }
        self.round_number += 1;
    }

    fn is_round_over(&self) -> bool {
        self.center.is_empty() && self.factories.iter().all(|f| f.is_empty())
    }

    pub fn num_tiles_taken(&self, m: &Move) -> Result<usize, String> {
        if m.is_from_center() {
            if let Some(n) = self.center.get(&m.color) {
                Ok(*n)
            } else {
                Err(format!("Color {:?} is not in the center.", m.color))
            }
        } else {
            Ok(self.factories[m.factory_idx - 1]
                .iter()
                .filter(|&t| *t == m.color)
                .count())
        }
    }

    pub fn take_turn(&mut self, m: &Move) -> Result<bool, String> {
        // println!("player {:?}: {:?}", self.current_player().display_name, m);
        if let Some(err_msg) = m.check_validity() {
            return Err(err_msg);
        }
        let taking_start_token =
            m.is_from_center() && self.is_start_token_available();
        let num_tiles = self.num_tiles_taken(m)?;
        if num_tiles == 0 {
            return Err(format!(
                "Color {:?} is not in factory #{}.",
                m.color, m.factory_idx
            ));
        }
        self.current_player_mut().add_tiles(
            m.working_row,
            m.color,
            num_tiles,
        )?;
        // Now safe to do mutations to game state.
        if m.is_from_center() {
            self.center.remove(&m.color);
        } else {
            let factory = &mut self.factories[m.factory_idx - 1];
            for t in factory.iter() {
                if *t != m.color {
                    *self.center.entry(*t).or_insert(0) += 1;
                }
            }
            factory.clear();
        }
        if taking_start_token {
            self.start_player_idx = self.curr_player_idx;
            self.current_player_mut().send_to_trash(Color::Start, 1);
            assert_eq!(self.center.remove(&Color::Start), Some(1));
        }
        // Check if the round is over.
        if self.is_round_over() {
            return Ok(true);
        }
        // Set up for the next player.
        self.curr_player_idx += 1;
        self.curr_player_idx %= self.players.len();
        Ok(false)
    }

    pub fn finish_round(&mut self) -> Result<bool, String> {
        if !self.is_round_over() {
            return Err("round isn't over".to_string());
        }
        // Move and score completed working rows.
        for player in &mut self.players {
            for t in player.score_round()? {
                self.box_lid.push(t);
            }
        }
        // Check for the end of the game.
        if self.is_finished() {
            for player in &mut self.players {
                player.score_bonuses();
            }
            return Ok(true);
        }
        // Prep the tile bag for the next round, if necessary.
        if self.tile_bag.len() < 4 * self.factories.len() {
            self.tile_bag.append(&mut self.box_lid);
            // TODO: store the RNG from initialization and reuse it here.
            let mut rng = rand::rng();
            self.tile_bag.shuffle(&mut rng);
        }
        Ok(false)
    }

    pub fn is_finished(&self) -> bool {
        self.players.iter().any(|p| p.num_full_rows() > 0)
    }

    pub fn valid_moves(&self) -> Vec<Move> {
        let player = self.current_player();
        let color_moves: Vec<Vec<usize>> =
            ALL_COLORS.iter().map(|c| player.valid_moves(*c)).collect();
        let mut result = vec![];
        // Consider all colors from each factory.
        for (fidx, factory) in self.factories.iter().enumerate() {
            for (cidx, c) in ALL_COLORS.iter().enumerate() {
                if factory.iter().any(|t| t == c) {
                    for row in color_moves[cidx].iter() {
                        result.push(Move {
                            factory_idx: fidx + 1,
                            color: *c,
                            working_row: *row,
                        });
                    }
                }
            }
        }
        // Consider all colors from the center.
        for c in self.center.keys() {
            if *c == Color::Start {
                continue;
            }
            for row in color_moves[*c as usize].iter() {
                result.push(Move {
                    factory_idx: 0,
                    color: *c,
                    working_row: *row,
                });
            }
        }
        result
    }
}

