#![allow(clippy::all)]
use crate::agent::{Agent, create_agent};
use crate::game_state;
use crate::player_move;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::convert::TryInto;
use std::sync::Mutex;

#[pyclass]
struct BlauState {
    gs: Mutex<game_state::GameState>,
}

#[pymethods]
impl BlauState {
    #[new]
    fn new(names: Vec<String>) -> Self {
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut rng = rand::rng();
        let wrapped = game_state::GameState::new(&name_refs, &mut rng);
        Self {
            gs: Mutex::new(wrapped),
        }
    }

    fn start_round(&self) -> PyResult<()> {
        self.gs
            .lock()
            .map_err(|_| PyRuntimeError::new_err("game state lock poisoned"))?
            .start_round();
        Ok(())
    }

    fn do_move(&self, m: PyRef<'_, BlauMove>) -> PyResult<bool> {
        self.gs
            .lock()
            .map_err(|_| PyRuntimeError::new_err("game state lock poisoned"))?
            .take_turn(&m.pm)
            .map_err(PyValueError::new_err)
    }

    fn finish_round(&self) -> PyResult<bool> {
        self.gs
            .lock()
            .map_err(|_| PyRuntimeError::new_err("game state lock poisoned"))?
            .finish_round()
            .map_err(PyValueError::new_err)
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!(
            "{:?}",
            self.gs.lock().map_err(|_| PyRuntimeError::new_err(
                "game state lock poisoned"
            ))?
        ))
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(
            &*self.gs.lock().map_err(|_| {
                PyRuntimeError::new_err("game state lock poisoned")
            })?,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    #[getter]
    fn curr_player_idx(&self) -> PyResult<usize> {
        Ok(self
            .gs
            .lock()
            .map_err(|_| PyRuntimeError::new_err("game state lock poisoned"))?
            .curr_player_idx)
    }

    fn players(&self) -> PyResult<Vec<(String, i32)>> {
        Ok(self
            .gs
            .lock()
            .map_err(|_| PyRuntimeError::new_err("game state lock poisoned"))?
            .players
            .iter()
            .map(|p| (p.display_name.clone(), p.score()))
            .collect())
    }

    fn is_finished(&self) -> PyResult<bool> {
        Ok(self
            .gs
            .lock()
            .map_err(|_| PyRuntimeError::new_err("game state lock poisoned"))?
            .is_finished())
    }
}

#[pyclass]
struct BlauMove {
    pm: player_move::Move,
}

#[pymethods]
impl BlauMove {
    #[new]
    fn new(
        factory_idx: usize,
        cidx: usize,
        working_row: usize,
    ) -> PyResult<Self> {
        let color = cidx
            .try_into()
            .map_err(|_| PyValueError::new_err("Invalid color"))?;
        let wrapped = player_move::Move {
            factory_idx,
            color,
            working_row,
        };
        Ok(Self { pm: wrapped })
    }

    #[getter]
    fn factory_idx(&self) -> usize {
        self.pm.factory_idx
    }

    #[getter]
    fn color(&self) -> usize {
        self.pm.color as usize
    }

    #[getter]
    fn working_row(&self) -> usize {
        self.pm.working_row
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.pm)
    }
}

#[pyclass]
struct BlauAgent {
    ga: Mutex<Box<dyn Agent + Send>>,
}

#[pymethods]
impl BlauAgent {
    #[new]
    fn new(difficulty: usize) -> Self {
        Self {
            ga: Mutex::new(create_agent(difficulty)),
        }
    }

    fn choose_action(&self, game: PyRef<'_, BlauState>) -> PyResult<BlauMove> {
        let game = game
            .gs
            .lock()
            .map_err(|_| PyRuntimeError::new_err("game state lock poisoned"))?;
        let m = self
            .ga
            .lock()
            .map_err(|_| PyRuntimeError::new_err("agent lock poisoned"))?
            .choose_action(&game);
        Ok(BlauMove { pm: m })
    }
}

#[pymodule]
fn blau(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__doc__", "Blau's core game logic.")?;
    m.add_class::<BlauMove>()?;
    m.add_class::<BlauState>()?;
    m.add_class::<BlauAgent>()?;
    Ok(())
}
