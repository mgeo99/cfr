use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::ops::RangeBounds;
use std::path::Path;

use fst::{IntoStreamer, Set, Streamer};

use super::bag::Bag;
use super::constraints::ConstraintBoard;
use super::rack::Rack;
use super::util::{Direction, Letter, Move, Position, SquareEffect};
use super::word_search::{BlankAssignmentList, WordSearcher};
use super::BOARD_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tile {
    Empty,
    Special(SquareEffect),
    Letter(Letter),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub word: String,
    pub pos: Position,
    pub dir: Direction,
}

#[derive(Debug, Clone)]
pub struct ScrabbleBoard {
    /// Actual letters on the board
    state: Vec<Vec<Tile>>,
    /// Position of all placed blanks
    blanks: HashSet<Position>,
    /// Moves placed on the board
    pub placements: Vec<Placement>,
}

impl ScrabbleBoard {
    pub fn empty() -> Self {
        let state = vec![vec![Tile::Empty; BOARD_SIZE]; BOARD_SIZE];
        Self {
            state,
            blanks: HashSet::new(),
            placements: Vec::new(),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        let mut file = File::open(path.as_ref()).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        let raw_board: Vec<Vec<String>> = serde_json::from_str(&data).unwrap();
        let mut state = vec![vec![Tile::Empty; BOARD_SIZE]; BOARD_SIZE];

        for i in 0..BOARD_SIZE {
            for j in 0..BOARD_SIZE {
                state[i][j] = match raw_board[i][j].as_str() {
                    "" => Tile::Empty,
                    "?" => Tile::Letter(Letter::Blank),
                    "TL" => Tile::Special(SquareEffect::TripleLetter),
                    "TW" => Tile::Special(SquareEffect::TripleWord),
                    "DL" => Tile::Special(SquareEffect::DoubleLetter),
                    "DW" => Tile::Special(SquareEffect::TripleLetter),
                    "CN" => Tile::Special(SquareEffect::Center),
                    c => Tile::Letter(Letter::Letter(c.chars().nth(0).unwrap()))
                }
            }
        }

        Self {
            state,
            blanks: HashSet::new(),
            placements: vec![]
        }

    }

    /// Places the current word on the board. Assumes that the word is a valid placement.
    /// Returns the list of characters from the rack that were required to place the move
    /// TODO: Add more validation when implementing live play against a human.
    pub fn place_word(&mut self, word: &str, pos: Position, dir: Direction) -> Vec<Letter> {
        let mut curr_pos = pos;
        let mut placed_letters = vec![];
        for c in word.chars() {
            let uc = c.to_ascii_uppercase();
            let tile = Tile::Letter(Letter::Letter(uc));
            match self[curr_pos] {
                // Letter should have already existed so just make sure we dont change it
                Tile::Letter(Letter::Letter(l)) => {
                    assert!(l == uc, "Placement would overwrite the current letter")
                }
                _ => {
                    // We need to mark that we are using a letter
                    // from our rack (blank)
                    if c.is_ascii_lowercase() {
                        placed_letters.push(Letter::Blank);
                    } else {
                        placed_letters.push(Letter::Letter(uc));
                    }
                    self[curr_pos] = tile
                }
            }

            curr_pos[dir] += 1;
        }
        self.placements.push(Placement {
            dir,
            pos,
            word: word.to_string(),
        });
        placed_letters
    }

    /// Calculates the cross score sums for the current board. In the scrabble rules,
    /// if you place something on a square that happens to touch a previously placed word
    /// (regardless of its orientation), you gain the points for that word
    pub fn calculate_cross_sums(&self, bag: &Bag) -> [[i32; BOARD_SIZE * BOARD_SIZE]; 2] {
        let mut cross_sums = [[0; BOARD_SIZE * BOARD_SIZE]; 2];
        for row in 0..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                for (i, &dir) in Direction::iter().enumerate() {
                    let mut score = 0;
                    let mut found_letter = false;
                    let pos = Position { row, col };

                    // Scan forwards and backwards until we find a tile that is no longer a letter then stops the iterator
                    let next_iter = pos.iter_next(dir).take_while(|x| self.is_letter(*x));
                    let prev_iter = pos.iter_prev(dir).take_while(|x| self.is_letter(*x));
                    for p in next_iter.chain(prev_iter) {
                        found_letter = true;
                        // If the position isn't blank then we add the value to the score
                        if let Tile::Letter(l) = self[p] {
                            score += bag.score(l);
                        }
                    }

                    if found_letter {
                        cross_sums[i][pos.as_index()] = score;
                    } else {
                        cross_sums[i][pos.as_index()] = -1;
                    }
                }
            }
        }
        cross_sums
    }

    /// Checks if a move can be played at all given the current rack and board state
    pub fn is_move_possible(&self, rack: &Rack, vocab: &Set<impl AsRef<[u8]> + Sync>) -> bool {
        let cb_across = ConstraintBoard::build(self, Direction::Across, vocab);
        let cb_down = ConstraintBoard::build(self, Direction::Down, vocab);

        let across_cands = cb_across.get_candidate_slices();
        let down_cands = cb_down.get_candidate_slices();

        let all_cands = across_cands.chain(down_cands).collect::<Vec<_>>();
        if all_cands.len() == 0 {
            return false;
        }

        // Check if any candidate can result in a valid move
        let has_moves = all_cands.into_iter().any(|(_, line, min_length)| {
            let searcher = WordSearcher {
                line,
                min_length,
                rack: rack.clone(),
            };
            let mut matches = vocab.search_with_state(searcher).into_stream();
            if let Some(_) = matches.next() {
                return true;
            }
            false
        });

        has_moves
    }

    /// Calculates all possible moves. Moves are not returned in any particular order so when presenting
    /// possible candidates to a player, be sure to sort them
    pub fn calculate_moves(
        &self,
        rack: &Rack,
        vocab: &Set<impl AsRef<[u8]> + Sync>,
        bag: &Bag,
    ) -> Vec<Move> {
        let cross_sums = self.calculate_cross_sums(bag);
        let cb_across = ConstraintBoard::build(self, Direction::Across, vocab);
        let cb_down = ConstraintBoard::build(self, Direction::Down, vocab);
        let across_cands = cb_across.get_candidate_slices();
        let down_cands = cb_down.get_candidate_slices();

        let all_cands = across_cands.chain(down_cands).collect::<Vec<_>>();

        all_cands
            .into_par_iter()
            .flat_map(|(pos, line, min_length)| {
                let searcher = WordSearcher {
                    line,
                    min_length,
                    rack: rack.clone(),
                };

                let mut matches = vocab.search_with_state(searcher).into_stream();
                let mut moves = Vec::new();
                while let Some((word, state)) = matches.next() {
                    let state = state.unwrap();

                    // Make all usages of blanks lowercase so we can easily figure out after the fact what blanks are used
                    // and where
                    let mut blank_pos = Vec::<(char, usize)>::new();
                    let mut blank_node = state.blank_assignments;
                    let mut word_chars = word.iter().map(|x| *x as char).collect::<Vec<_>>();
                    while let BlankAssignmentList::Elem(blank, nxt) = blank_node {
                        blank_node = (*nxt).clone();
                        word_chars[blank.1] = word_chars[blank.1].to_ascii_lowercase();
                        blank_pos.push(blank);
                    }

                    let mut new_move = Move {
                        dir: pos.1,
                        pos: pos.0,
                        word: word_chars.into_iter().collect(),
                        score: 0,
                    };
                    let cross_sums = match pos.1 {
                        Direction::Across => &cross_sums[0],
                        _ => &cross_sums[1],
                    };
                    new_move.score = self.score(&new_move, cross_sums, bag);
                    moves.push(new_move);
                }
                moves
            })
            .collect::<Vec<_>>()
    }

    pub fn score(&self, m: &Move, cross_sums: &[i32; 225], bag: &Bag) -> i32 {
        let mut true_score = 0;
        let mut total_cross_score = 0;
        let mut true_mult = 1;
        let mut n_played = 0;
        for (curr_pos, i) in m.iter() {
            let mut cross_mult = 1;
            let mut tile_mult = 1;
            /*
            #: TWS
            ^: DWS
            +: TLS
            -: DLS
            *: center
            */

            match self[curr_pos] {
                Tile::Special(SquareEffect::DoubleWord | SquareEffect::Center) => {
                    true_mult *= 2;
                    cross_mult *= 2;
                }
                Tile::Special(SquareEffect::TripleWord) => {
                    true_mult *= 3;
                    cross_mult *= 3;
                }
                Tile::Special(SquareEffect::TripleLetter) => {
                    tile_mult *= 3;
                }
                Tile::Special(SquareEffect::DoubleLetter) => {
                    tile_mult *= 2;
                }
                Tile::Empty => {}
                _ => {
                    cross_mult = 0;
                    n_played += 1;
                } // char was already there, so don't score old words
            }

            let mut curr_score = 0;
            if !(i.is_lowercase() || self.blanks.contains(&curr_pos)) {
                curr_score = bag.score(Letter::Letter(i)) * tile_mult;
            }

            let cross_sum = cross_sums[curr_pos.as_index()];

            if cross_sum >= 0 {
                let cross_score = curr_score + cross_sum;
                total_cross_score += cross_mult * cross_score;
            }

            true_score += curr_score;
        }

        let mut score = true_mult * true_score + total_cross_score;

        if m.word.len() - n_played == 7 {
            score += 50;
        }

        score
    }

    /// Checks if a position can be used as an anchor position
    pub fn is_anchor(&self, pos: Position) -> bool {
        if self.is_letter(pos) {
            return false;
        }

        for n in pos.adjacent() {
            if self.is_letter(n) {
                return true;
            }
        }

        false
    }
    /// Checks if the tile at the given position holds a letter
    pub fn is_letter(&self, pos: Position) -> bool {
        match self.state[pos.row][pos.col] {
            Tile::Letter(_) => true,
            _ => false,
        }
    }
}

impl std::ops::Index<Position> for ScrabbleBoard {
    type Output = Tile;

    fn index(&self, index: Position) -> &Self::Output {
        &self.state[index.row][index.col]
    }
}

impl std::ops::IndexMut<Position> for ScrabbleBoard {
    fn index_mut(&mut self, index: Position) -> &mut Self::Output {
        &mut self.state[index.row][index.col]
    }
}
