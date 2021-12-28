use std::rc::Rc;

use fst::Set;
use rand::prelude::SliceRandom;

use crate::cfr::state::{Game, GameState};
use crate::scrabble::{util, BOARD_SIZE};

use super::bag::Bag;
use super::board::{ScrabbleBoard, Tile};
use super::rack::Rack;
use super::util::{Letter, Move, Position, SquareEffect};

/// Grid of all possible moves the player can make in the current state.
/// Once a player chooses an action, a random word is sampled from the list of available words
/// and applied to the board
///

const MAX_LENGTH: usize = 7;

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
        let mut move_ids = vec![vec![vec![Vec::new(); MAX_LENGTH]; BOARD_SIZE]; BOARD_SIZE];
        let mut max_scores = vec![vec![vec![0; MAX_LENGTH]; BOARD_SIZE]; BOARD_SIZE];
        let mut best_move_id = 0;
        let mut best_move_score = 0;
        let moves = curr_board.calculate_moves(rack, vocab, bag);
        for (i, m) in moves.iter().enumerate() {
            // Only store the best possible moves at each length
            let len_idx = m.word.len() - 2;
            if len_idx >= MAX_LENGTH {
                continue;
            }

            if max_scores[m.pos.row][m.pos.col][len_idx] < m.score {
                max_scores[m.pos.row][m.pos.col][len_idx] = m.score;
                move_ids[m.pos.row][m.pos.col][len_idx].clear();
            }
            // Track the best overall move
            if m.score > best_move_score {
                best_move_score = m.score;
                best_move_id = i;
            }

            move_ids[m.pos.row][m.pos.col][len_idx].push(i);
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

        let coord = util::index_to_coord(idx, &[BOARD_SIZE, BOARD_SIZE, MAX_LENGTH]);

        let valid_moves = &self.move_ids[coord[0]][coord[1]][coord[2]];
        // Pick a random move
        let mut rng = rand::thread_rng();
        let selected_move = valid_moves.choose(&mut rng).unwrap();
        &self.moves[*selected_move]
    }

    // TODO: Add an offset by 1 that basically means always take the highest scoring move
    pub fn get_valid_moves(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for i in 0..self.move_ids.len() {
            for j in 0..self.move_ids[i].len() {
                for k in 0..self.move_ids[i][j].len() {
                    if self.move_ids[i][j][k].len() > 0 {
                        // +1 because the best move is always an option
                        let idx =
                            util::coord_to_index(&[i, j, k], &[BOARD_SIZE, BOARD_SIZE, MAX_LENGTH]);
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
    /// Flag to show whether or not each player is active
    player_active: Vec<bool>,
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
        let mut valid = self.curr_move_grid.get_valid_moves();
        valid.push(0);
        valid
    }

    fn state_key(&self) -> Self::Key {
        let mut key = String::new();
        key.push_str(format!("{}", self.curr_player).as_str());
       /*let mut placed_words = self.board.placements.iter().map(|x| x.word.as_str()).collect::<Vec<_>>();
        placed_words.sort_unstable();
        for p in placed_words {
            key.push_str(p);
        }*/
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
        let mut next_bag = self.bag.clone();
        let mut next_board = self.board.clone();
        let mut next_racks = self.player_racks.clone();
        let mut next_scores = self.player_scores.clone();
        let mut next_player_active = self.player_active.clone();

        // We can only do this if we have available moves
        if self.curr_move_grid.moves.len() > 0 {
            let selected_move = self.curr_move_grid.get_move(action);
            // Place the word
            let used_letters =
                next_board.place_word(&selected_move.word, selected_move.pos, selected_move.dir);

            // Remove all the letters used to place the previous word
            for l in used_letters.iter() {
                next_racks[self.curr_player].remove_inplace(*l);
            }
            // Replenish that same player's rack with new letters
            for l in next_bag.draw_tiles(used_letters.len()) {
                next_racks[self.curr_player].add_inplace(l);
            }
            // Add to the current player's score
            next_scores[self.curr_player] += selected_move.score;
        }

        // Find the next available player until we run out of space
        let mut next_player = (self.curr_player + 1) % self.player_racks.len();
        let mut next_movegrid;
        let mut inactive_players = next_player_active.iter().filter(|&x| !x == false).count();
        loop {
            next_movegrid = MoveGrid::build(
                &next_bag,
                &next_board,
                self.vocab.as_ref(),
                &next_racks[next_player],
            );

            if next_movegrid.moves.is_empty() {
                next_player_active[next_player] = false;
                inactive_players += 1;
                next_player = (next_player + 1) % self.player_racks.len();
            } else {
                break;
            }
            if inactive_players >= next_player_active.len() {
                break;
            }
        }

        next_player_active[next_player] = next_movegrid.moves.len() > 0;

        Some(ScrabbleState {
            bag: next_bag,
            board: next_board,
            curr_move_grid: next_movegrid,
            curr_player: next_player,
            player_racks: next_racks,
            player_scores: next_scores,
            player_active: next_player_active,
            vocab: self.vocab.clone(),
        })
    }

    fn is_terminal(&self) -> bool {
        // If nobody is active then we are in a terminal state
        if !self.player_active.iter().all(|x| *x) {
            return true;
        }
        // Check if any player can make a move
        /*let vocab_ref = self.vocab.as_ref();
        let any_player_has_move = self
            .player_racks
            .iter()
            .any(|x| self.board.is_move_possible(x, vocab_ref));

        // If there are any players that can make a move then it's not a terminal state
        if any_player_has_move {
            return false;
        }*/
        // TODO: DO we need to check if the bag is empty??
        false
    }

    fn get_reward(&self, player: usize) -> f32 {
        self.player_scores[player] as f32
    }
}

pub struct ScrabbleGame {
    /// Number of players in the game
    n_players: usize,
    /// Number of valid actions
    n_actions: usize,
    /// Vocabulary tied to the game
    vocab: Rc<Set<Vec<u8>>>,
}

impl ScrabbleGame {
    pub fn new(n_players: usize, vocab: Rc<Set<Vec<u8>>>) -> Self {
        Self {
            n_actions: BOARD_SIZE * BOARD_SIZE * MAX_LENGTH + 1,
            n_players,
            vocab,
        }
    }
}

impl Game for ScrabbleGame {
    type State = ScrabbleState;

    fn num_players(&self) -> usize {
        self.n_players
    }

    fn num_actions(&self) -> usize {
        self.n_actions
    }

    fn start(&self) -> Self::State {
        let mut racks = vec![];
        let scores = vec![0; self.n_players];
        let mut bag = Bag::default();

        for _ in 0..self.n_players {
            let mut rack = Rack::empty();
            for l in bag.draw_tiles(7) {
                rack.add_inplace(l);
            }
            racks.push(rack);
        }
        // Compute the initial move grid for the first player
        let board = ScrabbleBoard::empty();
        let move_grid = MoveGrid::build(&bag, &board, self.vocab.as_ref(), &racks[0]);

        ScrabbleState {
            bag,
            curr_move_grid: move_grid,
            curr_player: 0,
            player_racks: racks,
            player_scores: scores,
            player_active: vec![true; self.n_players],
            board,
            vocab: self.vocab.clone(),
        }
    }

    fn reset(&mut self) {}
}
