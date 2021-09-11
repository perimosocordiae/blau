use crate::game_state::GameState;
use crate::player_move::Move;
use crate::player_state::{played_column, PlayerState};
use rand::seq::SliceRandom;

pub trait Agent {
    fn choose_action(&self, game: &GameState) -> Move;
}

pub struct RandomAgent {}
impl Agent for RandomAgent {
    fn choose_action(&self, game: &GameState) -> Move {
        let moves = game.valid_moves();
        let mut rng = rand::thread_rng();
        if let Some(m) = moves.choose(&mut rng) {
            *m
        } else {
            panic!("No moves to choose from! GameState: {:?}", game);
        }
    }
}

#[derive(Debug)]
pub struct GreedyAgent {
    // Value to place on taking the "1st" token, aside from the trash penalty.
    first_player_bias: i32,
    // Value to place on adding tiles to a fresh working row.
    new_row_bias: i32,
    // Value to place on fresh working row size.
    new_row_bias_factor: i32,
    // Value to place on taking from the center, rather than a factory.
    center_bias: i32,
    // Value to place on the number of tiles taken.
    num_tiles_bias: i32,
    // Value to place on playing colors in the middle of the board.
    middle_bias: i32,
}

impl Agent for GreedyAgent {
    fn choose_action(&self, game: &GameState) -> Move {
        *game
            .valid_moves()
            .iter()
            .max_by_key(|m| self.score_move(&game, m))
            .unwrap_or_else(|| {
                panic!("No moves to choose from! GameState: {:?}", game)
            })
    }
}

impl GreedyAgent {
    pub fn new() -> Self {
        Self {
            first_player_bias: 0,
            new_row_bias: -50,
            new_row_bias_factor: -1,
            center_bias: 10,
            num_tiles_bias: 10,
            middle_bias: 0,
        }
    }

    fn score_move(&self, game: &GameState, m: &Move) -> (i32, i32) {
        let mut bias: i32 = 0;
        let mut player = game.current_player().clone();

        if m.is_from_center() {
            bias += self.center_bias;
            if game.is_start_token_available() {
                bias += self.first_player_bias;
                player.send_to_trash(m.color, 1);
            }
        }
        if player.is_new_working_row(m.working_row) {
            bias += self.new_row_bias
                + m.working_row as i32 * self.new_row_bias_factor;
        }
        let num_tiles = game.num_tiles_taken(m).expect("Cannot score move");
        bias += self.num_tiles_bias * num_tiles as i32;
        let column = played_column(m.working_row, m.color);
        bias += self.middle_bias * (2 - column as i32);
        player
            .add_tiles(m.working_row, m.color, num_tiles)
            .expect("Cannot add tiles");
        let score = player_score(&mut player);
        (score, bias)
    }
}

pub struct RoundPlanningAgent {
    num_branches: usize,
    recurse: bool,
    greedy: GreedyAgent,
}

impl Agent for RoundPlanningAgent {
    fn choose_action(&self, game: &GameState) -> Move {
        let mut moves = game.valid_moves();
        moves.sort_by_cached_key(|m| self.greedy.score_move(game, m));
        let start = if moves.len() >= self.num_branches {
            moves.len() - self.num_branches
        } else {
            0
        };
        let my_idx = game.curr_player_idx;
        *moves[start..]
            .iter()
            .max_by_key(|m| {
                let mut ng = game.clone();
                if ng.take_turn(m).unwrap() {
                    player_score(&mut ng.players[my_idx])
                } else {
                    self.rollout(my_idx, &mut ng)
                }
            })
            .unwrap()
    }
}

impl RoundPlanningAgent {
    pub fn new(recurse: bool) -> Self {
        Self {
            num_branches: 5,
            recurse,
            greedy: GreedyAgent::new(),
        }
    }
    fn rollout(&self, idx: usize, game: &mut GameState) -> i32 {
        loop {
            let m = if self.recurse && game.curr_player_idx == idx {
                self.choose_action(game)
            } else {
                self.greedy.choose_action(&game)
            };
            if game.take_turn(&m).unwrap() {
                return player_score(&mut game.players[idx]);
            }
        }
    }
}

fn player_score(p: &mut PlayerState) -> i32 {
    p.score_round().expect("Cannot score round");
    p.score_bonuses();
    p.score()
}
