#![allow(clippy::all)]
use crate::agent::{create_agent, Agent};
use crate::game_state;
use crate::player_move;
use cpython::exc::ValueError;
use cpython::{py_class, py_module_initializer, PyErr, PyResult};
use std::cell::RefCell;
use std::convert::TryInto;

py_class!(class BlauState |py| {
    data gs: RefCell<game_state::GameState>;
    def __new__(_cls, names: Vec<String>) -> PyResult<BlauState> {
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut rng = rand::thread_rng();
        let wrapped = game_state::GameState::new(&name_refs, &mut rng);
        BlauState::create_instance(py, RefCell::new(wrapped))
    }
    def start_round(&self) -> PyResult<Option<i32>> {
        self.gs(py).borrow_mut().start_round();
        Ok(None)
    }
    def do_move(&self, m: &BlauMove) -> PyResult<bool> {
        self.gs(py).borrow_mut()
            .take_turn(&m.pm(py))
            .map_err(|msg| PyErr::new::<ValueError, _>(py, msg))
    }
    def finish_round(&self) -> PyResult<bool> {
        self.gs(py).borrow_mut()
            .finish_round()
            .map_err(|msg| PyErr::new::<ValueError, _>(py, msg))
    }
    def __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.gs(py).borrow()))
    }
    def to_json(&self) -> PyResult<String> {
        Ok(self.gs(py).borrow().to_json())
    }
    @property def curr_player_idx(&self) -> PyResult<usize> {
        Ok(self.gs(py).borrow().curr_player_idx)
    }
    def players(&self) -> PyResult<Vec<(String, i32)>> {
        Ok(self.gs(py).borrow().players.iter()
               .map(|p| (p.display_name.clone(), p.score()))
               .collect())
    }
    def is_finished(&self) ->PyResult<bool> {
        Ok(self.gs(py).borrow().is_finished())
    }
});

py_class!(class BlauMove |py| {
    data pm: player_move::Move;
    def __new__(_cls, factory_idx: usize,
                cidx: usize, working_row: usize) -> PyResult<BlauMove> {
        let color = cidx.try_into().map_err(
            |_| PyErr::new::<ValueError, _>(py, "Invalid color")
        )?;
        let wrapped = player_move::Move { factory_idx, color, working_row };
        BlauMove::create_instance(py, wrapped)
    }
    @property def factory_idx(&self) -> PyResult<usize> {
        Ok(self.pm(py).factory_idx)
    }
    @property def color(&self) -> PyResult<usize> {
        Ok(self.pm(py).color as usize)
    }
    @property def working_row(&self) -> PyResult<usize> {
        Ok(self.pm(py).working_row)
    }
    def __str__(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.pm(py)))
    }
});

py_class!(class BlauAgent |py| {
    data ga: Box<dyn Agent + Send>;
    def __new__(_cls, difficulty: usize) -> PyResult<BlauAgent> {
        BlauAgent::create_instance(py, create_agent(difficulty))
    }
    def choose_action(&self, game: BlauState) -> PyResult<BlauMove> {
        let m = self.ga(py).choose_action(&game.gs(py).borrow());
        BlauMove::create_instance(py, m)
    }
});

// add bindings to the generated python module
py_module_initializer!(blau, |py, m| {
    m.add(py, "__doc__", "Blau's core game logic.")?;
    m.add(py, "BlauMove", py.get_type::<BlauMove>())?;
    m.add(py, "BlauState", py.get_type::<BlauState>())?;
    m.add(py, "BlauAgent", py.get_type::<BlauAgent>())?;
    Ok(())
});
