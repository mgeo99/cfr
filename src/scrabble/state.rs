use std::rc::Rc;

use fst::Set;
use rand::prelude::SliceRandom;

use crate::cfr::state::GameState;
use crate::scrabble::{util, BOARD_SIZE};

use super::bag::Bag;
use super::board::{ScrabbleBoard, Tile};
use super::rack::Rack;
use super::util::{Letter, Move, Position, SquareEffect};

/// Grid of all possible moves the player can make in the current state.
/// Once a player chooses an action, a random word is sampled from the list of available words
/// and applied to the board
pub struct MoveGrid {
    /// IDs of each move in the master move array
    /// 15x15x5 (maybe add an extra dimension to allow the choice of direction as well)
    move_ids: Vec<Vec<Vec<Vec<usize>>>>,
    /// Best move ID that will result in the optimal number of points
    best_move_id: usize,
    /// The actual data associated with the move
    moves: Vec<Move>,
}

impl MoveGrid {
    pub fn build(bag: &Bag, curr_board: &ScrabbleBoard, vocab: &Set<Vec<u8>>, rack: &Rack) -> Self {
        let mut move_ids = vec![vec![vec![Vec::new(); 5]; BOARD_SIZE]; BOARD_SIZE];
        let mut max_scores = vec![vec![vec![0; 5]; BOARD_SIZE]; BOARD_SIZE];
        let mut best_move_id = 0;
        let mut best_move_score = 0;
        let moves = curr_board.calculate_moves(rack, vocab, bag);
        for (i, m) in moves.iter().enumerate() {
            // Only store the best possible moves at each length
            if max_scores[m.pos.row][m.pos.col][m.word.len()] < m.score {
                max_scores[m.pos.row][m.pos.col][m.word.len()] = m.score;
                move_ids[m.pos.row][m.pos.col][m.word.len()].clear();
            }
            // Track the best overall move
            if m.score > best_move_score {
                best_move_score = m.score;
                best_move_id = i;
            }

            move_ids[m.pos.row][m.pos.col][m.word.len()].push(i);
        }

        Self {
            move_ids,
            moves,
            best_move_id,
        }
    }

    pub fn get_move(&self, action_id: usize) -> &Move {
        // If 0 is passed, then return the best move
        if action_id == 0 {
            return &self.moves[self.best_move_id];
        }
        let idx = action_id - 1;

        let coord = util::index_to_coord(idx, &[BOARD_SIZE, BOARD_SIZE, 5]);

        let valid_moves = &self.move_ids[coord[0]][coord[1]][coord[2]];
        // Pick a random move
        let mut rng = rand::thread_rng();
        let selected_move = *valid_moves.choose(&mut rng).unwrap();
        &self.moves[selected_move]
    }

    // TODO: Add an offset by 1 that basically means always take the highest scoring move
    pub fn get_valid_moves(&self) -> Vec<usize> {
        let mut result = Vec::new();
        let max_i = self.move_ids.len();
        for i in 0..self.move_ids.len() {
            let max_j = self.move_ids[i].len();
            for j in 0..self.move_ids[i].len() {
                for k in 0..self.move_ids[i][j].len() {
                    let idx = i * max_i + j * max_j + k;
                    if self.move_ids[i][j][k].len() > 0 {
                        // +1 because the best move is always an option
                        result.push(idx + 1);
                    }
                }
            }
        }
        result
    }
}

pub struct ScrabbleState {
    /// Current tile bag
    bag: Bag,
    /// Board state
    board: ScrabbleBoard,
    /// Current racks for each player
    player_racks: Vec<Rack>,
    /// Current scores for each player
    player_scores: Vec<i32>,
    /// Currently active player
    curr_player: usize,
    /// Current player active move grid,
    curr_move_grid: MoveGrid,
    /// Pointer to the vocabulary to avoid excessive and expensive copies
    vocab: Rc<Set<Vec<u8>>>,
}

impl GameState for ScrabbleState {
    type Key = String;

    fn active_player(&self) -> usize {
        self.curr_player
    }

    fn valid_actions(&self) -> Vec<usize> {
        self.curr_move_grid.get_valid_moves()
    }

    fn state_key(&self) -> Self::Key {
        let mut key = String::new();
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                let pos = Position { row, col };
                match self.board[pos] {
                    Tile::Empty => key.push(' '),
                    Tile::Letter(letter) => match letter {
                        Letter::Letter(l) => key.push(l),
                        Letter::Blank => key.push('-'),
                    },
                    Tile::Special(effect) => match effect {
                        SquareEffect::Center => key.push('C'),
                        SquareEffect::DoubleLetter => key.push_str("DL"),
                        SquareEffect::DoubleWord => key.push_str("DW"),
                        SquareEffect::TripleLetter => key.push_str("TL"),
                        SquareEffect::TripleWord => key.push_str("TW"),
                    },
                }
            }
        }
        key
    }

    fn next_state(&self, action: usize) -> Option<Self> {
        todo!()
    }

    fn is_terminal(&self) -> bool {
        // Check if any player can make a move
        let vocab_ref = self.vocab.as_ref();
        let any_player_has_move = self
            .player_racks
            .iter()
            .all(|x| self.board.is_move_possible(x, vocab_ref, &self.bag));
        if !any_player_has_move {
            return false;
        }
        if !self.bag.is_empty() {
            return false;
        }
        true
    }

    fn get_reward(&self, player: usize) -> f32 {
        self.player_scores[player] as f32
    }
}
