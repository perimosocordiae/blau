use blau_api::{GameAPI, PlayerInfo, Result};
use serde::{Deserialize, Serialize};

use crate::{
    agent::{Agent, create_agent},
    game_state::GameState,
    player_move,
};

/// Parameters for game initialization.
#[derive(Deserialize)]
struct GameParams {
    tutor_mode: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct MoveMessage {
    color: usize,
    factory: usize,
    working: usize,
}

impl MoveMessage {
    pub fn to_move(&self) -> Result<player_move::Move> {
        let color = self.color.try_into().map_err(|_| "Invalid color")?;
        Ok(player_move::Move {
            color,
            factory_idx: self.factory,
            working_row: self.working,
        })
    }
    pub fn from_move(mv: &player_move::Move) -> Self {
        Self {
            color: mv.color as usize,
            factory: mv.factory_idx,
            working: mv.working_row,
        }
    }
}

#[derive(Serialize)]
struct TakeTurnMessage {
    #[serde(flatten)]
    move_: MoveMessage,
    player_idx: usize,
    round_over: bool,
    winner: Option<String>,
    game_data: GameState,
}

#[derive(Serialize)]
#[serde(tag = "action")]
#[serde(rename_all = "lowercase")]
enum GameMessage {
    Start { game_id: i64 },
    Tutor(MoveMessage),
    Play(TakeTurnMessage),
    End(TakeTurnMessage),
}

pub struct BlauAPI {
    // Current game state
    state: GameState,
    // Player IDs in the same order as agents
    player_ids: Vec<String>,
    // None if human player
    agents: Vec<Option<Box<dyn Agent + Send>>>,
    // Indicates if the game is over
    game_over: bool,
    // If Some(i), agents[i] is the tutor.
    tutor_idx: Option<usize>,
}

impl BlauAPI {
    fn do_action<F: FnMut(&str, &str)>(
        &mut self,
        action: &MoveMessage,
        mut notice_cb: F,
    ) -> Result<()> {
        let player_idx = self.state.curr_player_idx;
        let round_over = self.state.take_turn(&action.to_move()?)?;
        let mut winner = None;
        if round_over {
            self.game_over = self.state.finish_round()?;
            if self.game_over {
                let max_idx = self
                    .player_scores()
                    .iter()
                    .enumerate()
                    .max_by_key(|&(_idx, score)| score)
                    .map(|(idx, _score)| idx)
                    .ok_or("No players to determine winner")?;
                winner = Some(self.player_ids[max_idx].clone());
            } else {
                self.state.start_round();
            }
        }
        // Notify all human players of the action.
        let turn_msg = TakeTurnMessage {
            move_: action.clone(),
            player_idx,
            round_over,
            winner,
            game_data: self.state.clone(),
        };
        let msg = if self.game_over {
            GameMessage::End(turn_msg)
        } else {
            GameMessage::Play(turn_msg)
        };
        let msg = serde_json::to_string(&msg)?;
        for idx in self.human_player_idxs() {
            notice_cb(self.player_ids[idx].as_str(), &msg);
        }
        Ok(())
    }
    fn human_player_idxs(&self) -> impl Iterator<Item = usize> + '_ {
        self.agents.iter().enumerate().filter_map(|(idx, agent)| {
            if agent.is_none() { Some(idx) } else { None }
        })
    }
    fn process_agents<F: FnMut(&str, &str)>(
        &mut self,
        mut notice_cb: F,
    ) -> Result<()> {
        while !self.game_over {
            if let Some(ai) = &self.agents[self.state.curr_player_idx] {
                let mv = ai.choose_action(&self.state);
                self.do_action(&MoveMessage::from_move(&mv), &mut notice_cb)?;
            } else if let Some(tutor_idx) = self.tutor_idx {
                let tutor = self.agents[tutor_idx]
                    .as_ref()
                    .expect("Tutor agent missing");
                let tutor_mv = tutor.choose_action(&self.state);
                // Send tutor move to human player.
                let msg = GameMessage::Tutor(MoveMessage::from_move(&tutor_mv));
                let msg = serde_json::to_string(&msg)?;
                notice_cb(self.current_player_id(), &msg);
                break;
            } else {
                // Next player is human.
                break;
            }
        }
        Ok(())
    }
}
impl GameAPI for BlauAPI {
    fn init(players: &[PlayerInfo], params: Option<&str>) -> Result<Self> {
        let params: GameParams = match params {
            Some(p) => serde_json::from_str(p)?,
            None => GameParams { tutor_mode: false },
        };
        let mut rng = rand::rng();
        let player_names: Vec<&str> =
            players.iter().map(|p| p.id.as_str()).collect();
        let state = GameState::new(&player_names, &mut rng);
        let player_ids = players.iter().map(|p| p.id.clone()).collect();
        let mut agents = players
            .iter()
            .map(|p| p.level.map(|lvl| create_agent(lvl as usize)))
            .collect::<Vec<_>>();
        let tutor_idx = if params.tutor_mode {
            agents.push(Some(create_agent(2))); // Tutor agent
            Some(agents.len() - 1)
        } else {
            None
        };
        Ok(Self {
            state,
            player_ids,
            agents,
            game_over: false,
            tutor_idx,
        })
    }

    fn restore(player_info: &[PlayerInfo], final_state: &str) -> Result<Self> {
        let fs: GameState = serde_json::from_str(final_state)?;
        Ok(Self {
            state: fs,
            player_ids: player_info.iter().map(|p| p.id.clone()).collect(),
            agents: vec![], // No agents in restored game.
            game_over: true,
            tutor_idx: None, // Tutor games are not stored.
        })
    }

    fn is_game_over(&self) -> bool {
        self.game_over
    }

    fn final_state(&self) -> Result<String> {
        if !self.game_over {
            return Err("Game is not finished".into());
        }
        Ok(serde_json::to_string(&self.state)?)
    }

    fn player_view(&self, _player_id: &str) -> Result<String> {
        Ok(serde_json::to_string(&self.state)?)
    }

    fn start<F: FnMut(&str, &str)>(
        &mut self,
        game_id: i64,
        mut notice_cb: F,
    ) -> Result<()> {
        self.state.start_round();
        let msg = GameMessage::Start { game_id };
        let msg = serde_json::to_string(&msg)?;
        for idx in self.human_player_idxs() {
            notice_cb(self.player_ids[idx].as_str(), &msg);
        }
        // Advance to wait for the next player action.
        self.process_agents(notice_cb)?;
        Ok(())
    }

    fn process_action<F: FnMut(&str, &str)>(
        &mut self,
        action: &str,
        mut notice_cb: F,
    ) -> Result<()> {
        if self.game_over {
            return Err("Game is over".into());
        }
        let action: MoveMessage = serde_json::from_str(action)?;
        self.do_action(&action, &mut notice_cb)?;
        // Advance to wait for the next player action.
        self.process_agents(&mut notice_cb)?;
        Ok(())
    }

    fn current_player_id(&self) -> &str {
        self.player_ids[self.state.curr_player_idx].as_str()
    }

    fn player_scores(&self) -> Vec<i32> {
        self.state.players.iter().map(|p| p.score()).collect()
    }

    fn should_persist(&self) -> bool {
        self.tutor_idx.is_none()
    }
}

#[test]
fn exercise_api() {
    let players = vec![
        PlayerInfo::human("foo".into()),
        PlayerInfo::ai("bot".into(), 1),
    ];
    let mut game: BlauAPI =
        GameAPI::init(&players, Some(r#"{"tutor_mode": true}"#)).unwrap();
    assert!(!game.should_persist());

    let mut num_notices = 0;
    game.start(1234, |id, msg| {
        assert_eq!(id, "foo");
        match num_notices {
            // First notice is the start message.
            0 => {
                assert_eq!(msg, r#"{"action":"start","game_id":1234}"#);
            }
            // Any others are tutor or bot moves.
            1..=2 => {
                assert!(
                    msg.starts_with(r#"{"action":"#),
                    "notice={num_notices} msg={msg}"
                );
            }
            _ => panic!("Too many notices: {num_notices}"),
        }
        num_notices += 1;
    })
    .unwrap();

    let view_json = game.player_view("foo").unwrap();
    assert!(view_json.starts_with("{"));

    num_notices = 0;
    let mv = game.state.valid_moves()[0];
    game.process_action(
        &serde_json::to_string(&MoveMessage::from_move(&mv)).unwrap(),
        |id, msg| {
            assert_eq!(id, "foo");
            assert!(msg.starts_with("{"));
            num_notices += 1;
        },
    )
    .unwrap();
    // One for us, one for the bot move, one for the tutor.
    assert_eq!(num_notices, 3);
}
