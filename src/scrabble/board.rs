use rayon::prelude::*;
use std::ops::RangeBounds;

use fst::{IntoStreamer, Set, Streamer};

use super::{
    bag::Bag,
    constraints::ConstraintBoard,
    util::{Direction, Letter, Move, Position, SquareEffect},
    word_search::{TrayRemaining, WordSearcher},
    BOARD_SIZE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tile {
    Empty,
    Special(SquareEffect),
    Letter(Letter),
}

pub struct ScrabbleBoard {
    /// Actual letters on the board
    state: Vec<Vec<Tile>>,
    /// Position of all placed blanks
    blanks: Vec<Position>,
    /// Bag with associated words
    bag: Bag,
}

impl ScrabbleBoard {
    pub fn empty() -> Self {
        let state = vec![vec![Tile::Empty; BOARD_SIZE]; BOARD_SIZE];
        let bag = Bag::default();
        Self {
            state,
            bag,
            blanks: Vec::new(),
        }
    }
    /// Calculates the cross score sums for the current board. In the scrabble rules,
    /// if you place something on a square that happens to touch a previously placed word
    /// (regardless of its orientation), you gain the points for that word
    pub fn calculate_cross_sums(&self, bag: &Bag) -> [[i32; BOARD_SIZE * BOARD_SIZE]; 2] {
        let mut cross_sums = [[0; BOARD_SIZE * BOARD_SIZE]; 2];
        for idx in 0..BOARD_SIZE * BOARD_SIZE {
            let row = idx / BOARD_SIZE;
            let col = idx % BOARD_SIZE;
            for (i, &dir) in Direction::iter().enumerate() {
                let mut score = 0;
                let mut found_letter = false;
                let mut pos = Some(Position { row, col });

                // Move forwards until we find a letter (or reach the end of the scan)
                while pos.is_some() && self.is_letter(pos.unwrap()) {
                    found_letter = true;

                    // If the position isn't blank then we add the value to the score
                    if let Tile::Letter(Letter::Letter(l)) = self.state[row][col] {
                        score += bag.score(l);
                    }
                    pos = pos.unwrap().next(dir);
                }

                // Move backwards doing the same as above
                pos = Some(Position { row, col });
                while pos.is_some() && self.is_letter(pos.unwrap()) {
                    found_letter = true;
                    if let Tile::Letter(Letter::Letter(l)) = self.state[row][col] {
                        score += bag.score(l);
                    }
                    pos = pos.unwrap().next(dir);
                }

                if found_letter {
                    cross_sums[i][idx] = score;
                } else {
                    cross_sums[i][idx] = -1;
                }
            }
        }
        cross_sums
    }

    /// Calculates all possible moves
    pub fn calculate_moves(
        &self,
        rack: &[Letter],
        vocab: &Set<impl AsRef<[u8]> + Sync>,
    ) -> Vec<Move> {
        let cross_sums = self.calculate_cross_sums(&self.bag);
        let cb_across = ConstraintBoard::build(self, Direction::Across, vocab);
        let cb_down = ConstraintBoard::build(self, Direction::Down, vocab);

        let across_cands = cb_across.get_candidate_slices();
        let down_cands = cb_down.get_candidate_slices();

        let all_cands = across_cands.chain(down_cands).collect::<Vec<_>>();
        let mut n_blanks = 0;
        let mut rack_hist = [0; 256];
        for l in rack {
            if let Letter::Blank = l {
                n_blanks += 1;
            } else if let Letter::Letter(l) = l {
                rack_hist[*l as usize] += 1;
            }
        }
        let rack = TrayRemaining::new(rack_hist, n_blanks);

        let mut moves = all_cands
            .into_iter()
            .map(|(pos, line, min_length)| {
                let searcher = WordSearcher {
                    line,
                    min_length,
                    rack: rack.clone(),
                };

                let mut matches = vocab.search_with_state(searcher).into_stream();
                let mut moves = Vec::new();
                while let Some((word, _)) = matches.next() {
                    //let state = state.unwrap();
                    let word = String::from_utf8(word.to_vec()).unwrap();
                    let mut new_move = Move {
                        dir: pos.1,
                        pos: pos.0,
                        word,
                        score: 0,
                    };
                    let cross_sums = match pos.1 {
                        Direction::Across => &cross_sums[0],
                        _ => &cross_sums[1]
                    };
                    new_move.score = self.score(&new_move, cross_sums);
                    moves.push(new_move);
                }
                moves
            })
            .flatten()
            .collect::<Vec<_>>();
        moves.par_sort_unstable_by_key(|x| std::cmp::Reverse(x.score));
        moves
    }

    pub fn score(&self, m: &Move, cross_sums: &[i32; 225]) -> i32 {
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
                },
                Tile::Special(SquareEffect::TripleWord) => {
                    true_mult *= 3;
                    cross_mult *= 3;
                },
                Tile::Special(SquareEffect::TripleLetter) => {
                    tile_mult *= 3;
                },
                Tile::Special(SquareEffect::DoubleLetter) => {
                    tile_mult *= 2;
                },
                Tile::Empty => {},
                _ => {
                    cross_mult = 0;
                    n_played += 1;
                } // char was already there, so don't score old words
            }

            let mut curr_score = 0;
            if !(i.is_lowercase() || self.blanks.contains(&curr_pos)) {
                curr_score = self.bag.score(i) * tile_mult;
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
