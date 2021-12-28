use crate::cfr::state::{Game, GameState};

#[derive(Debug)]
pub struct TicTacToeState {
    board: Vec<Vec<usize>>,
    curr_player: usize,
}

impl TicTacToeState {
    pub fn display_board(&self) {
        println!("===== Board / Guide =====");
        for i in 0..self.board.len() {
            for j in 0..self.board.len() {
                let txt = match self.board[i][j] {
                    1 => "X",
                    2 => "O",
                    _ => "-",
                };

                print!("{} ", txt);
            }
            print!("\t");
            for j in 0..self.board.len() {
                print!("{} ", i * self.board.len() + j);
            }
            print!("\n");
        }
    }

    /// Determines if the board has been completely filled
    fn is_full(&self) -> bool {
        let mut filled_cells = 0;
        let num_cells = self.board.len() * self.board.len();
        for i in 0..self.board.len() {
            for j in 0..self.board[i].len() {
                if self.board[i][j] != 0 {
                    filled_cells += 1;
                }
            }
        }
        filled_cells == num_cells
    }

    /// Gets the winner of the current board. If there is no winner then None is returned
    fn get_winner(&self) -> Option<usize> {
        // Check rows
        for i in 0..self.board.len() {
            if self.board[i][0] == 0 {
                continue;
            }
            let player = self.board[i][0];
            if self.all_same((i, 0), (0, 1)) {
                return Some(player - 1);
            }
        }
        // Check columns
        for i in 0..self.board.len() {
            if self.board[0][i] == 0 {
                continue;
            }
            let player = self.board[0][i];
            if self.all_same((0, i), (1, 0)) {
                return Some(player - 1);
            }
        }

        // Diagonals
        let player = self.board[0][0];
        if player != 0 && self.all_same((0, 0), (1, 1)) {
            return Some(player - 1);
        }
        let player = self.board[self.board.len() - 1][0];
        if player != 0 && self.all_same((self.board.len() - 1, 0), (-1, 1)) {
            return Some(player - 1);
        }
        None
    }

    /// Checks if all the moves along a direction at a specific starting point are the same
    fn all_same(&self, start: (usize, usize), dir: (i32, i32)) -> bool {
        let start_value = self.board[start.0][start.1];

        let mut i = start.0 as i32;
        let mut j = start.1 as i32;

        while i < self.board.len() as i32 && i >= 0 && j < self.board.len() as i32 && j >= 0 {
            if self.board[i as usize][j as usize] != start_value {
                return false;
            }

            i += dir.0;
            j += dir.1;
        }

        true
    }
}

impl GameState for TicTacToeState {
    type Key = String;

    fn active_player(&self) -> usize {
        self.curr_player
    }

    fn valid_actions(&self) -> Vec<usize> {
        let mut valid_actions = Vec::new();
        for i in 0..self.board.len() {
            for j in 0..self.board[i].len() {
                if self.board[i][j] != 0 {
                    continue;
                }
                let idx = i * self.board.len() + j;
                valid_actions.push(idx);
            }
        }
        valid_actions
    }

    fn state_key(&self) -> Self::Key {
        let mut key = String::new();
        key.push_str(format!("{}", self.curr_player).as_str());
        for i in 0..self.board.len() {
            for j in 0..self.board[i].len() {
                key.push_str(match self.board[i][j] {
                    1 => "X",
                    2 => "O",
                    _ => "-",
                });
            }
        }
        key
    }

    fn next_state(&self, action: usize) -> Option<Self> {
        let i = action / self.board.len();
        let j = action % self.board.len();

        let next_player = match self.curr_player {
            0 => 1,
            1 => 0,
            _ => unreachable!(),
        };
        let mut next_board = self.board.clone();

        debug_assert!(
            next_board[i][j] == 0,
            "The provided action would overwrite a board spot"
        );
        next_board[i][j] = self.curr_player + 1;

        Some(TicTacToeState {
            board: next_board,
            curr_player: next_player,
        })
    }

    fn is_terminal(&self) -> bool {
        if let Some(_) = self.get_winner() {
            return true;
        }
        self.is_full()
    }

    fn get_reward(&self, player: usize) -> f32 {
        if let Some(winner) = self.get_winner() {
            if winner == player {
                return 1.0;
            } else {
                return -1.0;
            }
        }
        0.0
    }
}

pub struct TicTacToe {
    board_dim: usize,
}

impl TicTacToe {
    pub fn new(board_dim: usize) -> Self {
        Self { board_dim }
    }
}

impl Game for TicTacToe {
    type State = TicTacToeState;

    fn num_players(&self) -> usize {
        2
    }

    fn num_actions(&self) -> usize {
        self.board_dim * self.board_dim
    }

    fn start(&self) -> Self::State {
        let board = (0..self.board_dim)
            .map(|_| {
                let mut row = Vec::with_capacity(self.board_dim);
                row.resize(self.board_dim, 0);
                row
            })
            .collect();
        TicTacToeState {
            board,
            curr_player: 0,
        }
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {

    use crate::cfr::state::GameState;

    use super::TicTacToeState;

    #[test]
    fn test_not_terminal() {
        let board = vec![vec![1, 1, 0], vec![2, 0, 0], vec![2, 0, 0]];
        let state = TicTacToeState {
            curr_player: 0,
            board: board,
        };

        assert!(!state.is_terminal());
    }

    #[test]
    fn test_terminal() {
        let board = vec![vec![1, 1, 1], vec![2, 0, 0], vec![2, 0, 0]];
        let state = TicTacToeState {
            curr_player: 1,
            board: board,
        };

        assert!(state.is_terminal());
        assert!(!state.is_full());
    }

    #[test]
    fn test_is_full() {
        let board = vec![vec![2, 2, 1], vec![1, 1, 2], vec![2, 1, 1]];
        let state = TicTacToeState {
            curr_player: 1,
            board: board,
        };

        assert!(state.is_full());
    }

    #[test]
    fn test_reward() {
        let board = vec![vec![1, 1, 1], vec![2, 0, 0], vec![2, 0, 0]];
        let state = TicTacToeState {
            curr_player: 0,
            board: board.clone(),
        };
        assert_eq!(state.get_reward(0), 1.0);
        assert_eq!(state.get_reward(1), -1.0);
    }

    #[test]
    fn test_tie_reward() {
        let board = vec![vec![2, 2, 1], vec![1, 1, 2], vec![2, 1, 1]];
        let state = TicTacToeState {
            curr_player: 0,
            board: board.clone(),
        };
        assert_eq!(state.get_reward(0), 0.0);
        assert_eq!(state.get_reward(1), 0.0);
    }

    #[test]
    fn test_transition_midgame() {
        let board = vec![vec![1, 1, 0], vec![2, 0, 0], vec![2, 0, 0]];
        let state = TicTacToeState {
            curr_player: 0,
            board: board.clone(),
        };
        let valid_actions = state.valid_actions();
        let next_state = state.next_state(valid_actions[0]).unwrap();
        let next_valid_actions = next_state.valid_actions();

        assert_eq!(next_state.active_player(), 1);
        assert_ne!(valid_actions.len(), next_valid_actions.len());
        assert_ne!(state.state_key(), next_state.state_key());
    }
}
