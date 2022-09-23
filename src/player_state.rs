use crate::colors::Color;
use serde::{Deserialize, Serialize};

type PlayGrid = [[bool; 5]; 5];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub display_name: String,
    played_tiles: PlayGrid,
    working_count: [usize; 5],
    working_color: [Color; 5],
    trashed_tiles: Vec<Color>,
    scores: Vec<i32>,
}

static PENALTIES: [i32; 7] = [-1, -1, -2, -2, -2, -3, -3];
static ROW_BONUS: i32 = 2;
static COL_BONUS: i32 = 7;
static KIND_BONUS: i32 = 10;

pub fn played_column(row: usize, color: Color) -> usize {
    (color as usize + row) % 5
}

fn played_color(row: usize, column: usize) -> usize {
    (column + 5 - row) % 5
}

fn score_tile(grid: &PlayGrid, row: usize, col: usize) -> i32 {
    let line = grid[row];
    let horiz = 1
        + line[(col + 1)..].iter().take_while(|&x| *x).count()
        + line[..col].iter().rev().take_while(|&x| *x).count();
    let vert = 1
        + grid[(row + 1)..].iter().take_while(|x| x[col]).count()
        + grid[..row].iter().rev().take_while(|x| x[col]).count();
    let res = (horiz + vert) as i32;
    if horiz == 1 || vert == 1 {
        res - 1
    } else {
        res
    }
}

#[test]
fn scoring_a_tile() {
    let mut grid = [[false; 5]; 5];
    grid[2][2] = true;
    assert_eq!(score_tile(&grid, 2, 2), 1);
    grid[2][4] = true;
    assert_eq!(score_tile(&grid, 2, 4), 1);
    grid[2][3] = true;
    assert_eq!(score_tile(&grid, 2, 3), 3);
    grid[1][3] = true;
    assert_eq!(score_tile(&grid, 1, 3), 2);
    grid[3][3] = true;
    assert_eq!(score_tile(&grid, 3, 3), 3);
    assert_eq!(score_tile(&grid, 2, 3), 6);
}

impl PlayerState {
    pub fn new(name: &str) -> PlayerState {
        PlayerState {
            display_name: name.to_string(),
            played_tiles: [[false; 5]; 5],
            working_count: [0; 5],
            working_color: [Color::Blank; 5],
            trashed_tiles: vec![],
            scores: vec![],
        }
    }

    pub fn score(&self) -> i32 {
        self.scores.iter().sum()
    }

    pub fn is_new_working_row(&self, row: usize) -> bool {
        row < self.working_count.len() && self.working_count[row] == 0
    }

    pub fn send_to_trash(&mut self, c: Color, num_tiles: usize) {
        for _i in 0..num_tiles {
            self.trashed_tiles.push(c);
        }
    }

    fn is_played(&self, row: usize, c: Color) -> bool {
        self.played_tiles[row][played_column(row, c)]
    }

    pub fn add_tiles(
        &mut self,
        row: usize,
        color: Color,
        num_tiles: usize,
    ) -> Result<(), String> {
        if row == 5 {
            self.send_to_trash(color, num_tiles);
            return Ok(());
        }
        if row > 5 {
            return Err("invalid row".to_string());
        }
        if color == Color::Start {
            return Err("the start tile can only be trashed".to_string());
        }
        let row_number = 1 + row;
        if self.is_played(row, color) {
            return Err(format!(
                "color {:?} has already been played in row {}",
                color, row_number
            ));
        }
        let w_count = self.working_count[row];
        if w_count >= row_number {
            return Err(format!("no room left in row {}", row_number));
        }
        let w_color = &mut self.working_color[row];
        if *w_color != color {
            if *w_color != Color::Blank {
                return Err(format!(
                    "working row {} is locked to color {:?}",
                    row_number, *w_color
                ));
            }
            *w_color = color;
        }
        self.working_count[row] += if num_tiles + w_count > row_number {
            let num_trashed = num_tiles + w_count - row_number;
            self.send_to_trash(color, num_trashed);
            num_tiles - num_trashed
        } else {
            num_tiles
        };
        Ok(())
    }

    pub fn score_round(&mut self) -> Result<Vec<Color>, String> {
        let mut round_score: i32 = 0;
        let mut returned_tiles = vec![];
        // move completed working rows into the play grid
        for row in 0..5 {
            let count = self.working_count[row];
            if count <= row {
                continue;
            }
            let color = self.working_color[row];
            for _i in 1..count {
                returned_tiles.push(color);
            }
            let column = played_column(row, color);
            let is_played = &mut self.played_tiles[row][column];
            if *is_played {
                return Err(format!(
                    "color {:?} has already been played in row {}",
                    color,
                    row + 1
                ));
            }
            *is_played = true;
            // score the newly-played tile
            round_score += score_tile(&self.played_tiles, row, column);
            self.working_color[row] = Color::Blank;
            self.working_count[row] = 0;
        }
        // process trashed tiles
        for (idx, color) in self.trashed_tiles.iter().enumerate() {
            if idx < PENALTIES.len() {
                round_score += PENALTIES[idx];
            }
            if *color != Color::Start {
                returned_tiles.push(*color);
            }
        }
        self.trashed_tiles.clear();
        let prev_score = self.score();
        if prev_score + round_score < 0 {
            round_score = -prev_score;
        }
        self.scores.push(round_score);
        Ok(returned_tiles)
    }

    pub fn num_full_rows(&self) -> i32 {
        self.played_tiles
            .iter()
            .map(|row| row.iter().all(|p| *p) as i32)
            .sum()
    }

    fn num_full_columns(&self) -> i32 {
        let mut count = 0;
        for col in 0..5 {
            if (0..5).all(|i| self.played_tiles[i][col]) {
                count += 1;
            }
        }
        count
    }

    fn num_full_colors(&self) -> i32 {
        let mut bincount = [0usize; 5];
        for (i, row) in self.played_tiles.iter().enumerate() {
            for (j, is_played) in row.iter().enumerate() {
                if *is_played {
                    bincount[played_color(i, j)] += 1;
                }
            }
        }
        bincount.iter().filter(|&c| *c == 5).count() as i32
    }

    pub fn score_bonuses(&mut self) {
        self.scores.push(ROW_BONUS * self.num_full_rows());
        self.scores.push(COL_BONUS * self.num_full_columns());
        self.scores.push(KIND_BONUS * self.num_full_colors());
    }

    pub fn valid_moves(&self, c: Color) -> Vec<usize> {
        let mut result = vec![5]; // trashing is always valid
        for row in 0..5 {
            let w_color = self.working_color[row];
            if (w_color == c || w_color == Color::Blank)
                && !self.is_played(row, c)
                && self.working_count[row] <= row
            {
                result.push(row);
            }
        }
        result
    }
}

#[test]
fn scoring_kind_bonuses() {
    let mut p = PlayerState::new("jim");
    for j in 0..5 {
        p.played_tiles[j][j] = true;
    }
    p.played_tiles[0][1] = true;
    assert_eq!(p.num_full_colors(), 1);
    p.score_bonuses();
    assert_eq!(p.scores, vec![0, 0, KIND_BONUS]);

    p.played_tiles[0][0] = false;
    assert_eq!(p.num_full_colors(), 0);
}

#[test]
fn scoring_column_bonuses() {
    let mut p = PlayerState::new("fred");
    for j in 0..5 {
        p.played_tiles[j][3] = true;
    }
    p.played_tiles[0][0] = true;
    assert_eq!(p.num_full_columns(), 1);
    p.score_bonuses();
    assert_eq!(p.scores, vec![0, COL_BONUS, 0]);
}

#[test]
fn scoring_row_bonuses() {
    let mut p = PlayerState::new("harry");
    p.score_bonuses();
    assert_eq!(p.scores, vec![0, 0, 0]);

    p.scores.clear();
    for j in 0..5 {
        p.played_tiles[1][j] = true;
    }
    p.played_tiles[0][0] = true;
    assert_eq!(p.num_full_rows(), 1);
    p.score_bonuses();
    assert_eq!(p.scores, vec![ROW_BONUS, 0, 0]);
}

#[test]
fn scoring_round() {
    let mut p = PlayerState::new("tom");
    assert_eq!(p.score_round(), Ok(vec![]));
    assert_eq!(p.scores, vec![0]);

    assert_eq!(p.add_tiles(0, Color::Blue, 1), Ok(()));
    assert_eq!(p.score_round(), Ok(vec![]));
    assert_eq!(p.scores, vec![0, 1]);

    assert_eq!(p.add_tiles(1, Color::Purple, 2), Ok(()));
    assert_eq!(p.score_round(), Ok(vec![Color::Purple]));
    assert_eq!(p.scores, vec![0, 1, 2]);
    assert_eq!(p.score(), 3);

    assert_eq!(p.add_tiles(0, Color::Red, 5), Ok(()));
    assert_eq!(
        p.score_round(),
        Ok(vec![Color::Red, Color::Red, Color::Red, Color::Red])
    );
    assert_eq!(p.scores, vec![0, 1, 2, -3]);
    assert_eq!(p.score(), 0);
}

#[test]
fn adding_tiles() {
    let mut p = PlayerState::new("dave");

    assert_eq!(p.add_tiles(5, Color::Red, 1), Ok(()));
    assert_eq!(p.trashed_tiles, vec![Color::Red]);
    assert_eq!(p.score(), 0);

    assert_eq!(
        p.add_tiles(6, Color::Blue, 3),
        Err("invalid row".to_string())
    );
    assert_eq!(
        p.add_tiles(0, Color::Start, 1),
        Err("the start tile can only be trashed".to_string())
    );

    assert_eq!(p.add_tiles(0, Color::Orange, 2), Ok(()));
    assert_eq!(p.trashed_tiles, vec![Color::Red, Color::Orange]);
    assert_eq!(p.working_count, [1, 0, 0, 0, 0]);
    assert_eq!(p.working_color[0], Color::Orange);

    assert_eq!(
        p.add_tiles(0, Color::Orange, 1),
        Err("no room left in row 1".to_string())
    );

    assert_eq!(p.add_tiles(4, Color::Green, 3), Ok(()));
    assert_eq!(p.trashed_tiles, vec![Color::Red, Color::Orange]);
    assert_eq!(p.working_count, [1, 0, 0, 0, 3]);
    assert_eq!(p.working_color[4], Color::Green);

    assert_eq!(
        p.add_tiles(4, Color::Purple, 1),
        Err("working row 5 is locked to color Green".to_string())
    );
    p.played_tiles[1][played_column(1, Color::Orange)] = true;
    assert_eq!(
        p.add_tiles(1, Color::Orange, 2),
        Err("color Orange has already been played in row 2".to_string())
    );
}
