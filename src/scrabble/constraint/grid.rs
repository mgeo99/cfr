use fst::{IntoStreamer, Set, Streamer};

use super::letter_set::LetterSet;
use super::searcher::{ConstraintSearcher, ConstraintSearcherState};
use super::{Constraint, ConstraintQuery};
use crate::scrabble::board::{ScrabbleBoard, Tile};
use crate::scrabble::util::{Direction, Position};
use crate::scrabble::BOARD_SIZE;

#[derive(Debug, Clone, Copy)]
pub struct ConstraintIndex(pub Position, pub Direction);

impl ConstraintIndex {
    pub fn next(mut self) -> Self {
        self.0[self.1] = self.0[self.1].saturating_add(1);
        self
    }

    pub fn back(mut self) -> Self {
        self.0[self.1] = self.0[self.1].wrapping_sub(1);
        self
    }

    /// A placement at the same position, but different direction
    pub fn perp(self) -> Self {
        Self(self.0, self.1.flip())
    }
}

/// Constraint grid used for generating move candidates. Takes the current game state
/// and generates constraints along a certain direction
pub struct ConstraintGrid {
    /// Constraints for each cell in the grid
    state: Vec<Vec<Constraint>>,
    /// Direction the grid is associated with
    dir: Direction,
}

impl ConstraintGrid {
    pub fn build(
        curr_board: &ScrabbleBoard,
        dir: Direction,
        vocab: &Set<impl AsRef<[u8]> + Sync>,
    ) -> Self {
        let mut state = vec![vec![Constraint::Empty(LetterSet::empty()); BOARD_SIZE]; BOARD_SIZE];
        // Fill in the board state
        let mut pos = ConstraintIndex(Position { row: 0, col: 0 }, dir);
        for i in 0..BOARD_SIZE {
            let mut curr_pos = pos.clone();
            let mut tiles = [Tile::Empty; BOARD_SIZE];
            let mut constraints = [Constraint::Empty(LetterSet::empty()); BOARD_SIZE];

            // Copy the tiles from the current board
            for j in 0..BOARD_SIZE {
                tiles[j] = curr_board[curr_pos.0];
                curr_pos = curr_pos.next();
            }

            // Fill the constraints for the current row. Note:
            // these constraints are ONLY for the row and do not account for cross checks
            Self::fill_constraints(&tiles, &mut constraints, vocab);
            for j in 0..BOARD_SIZE {
                state[j][i] = constraints[j];
            }

            pos = pos.perp().next().perp();
        }

        Self { state, dir }
    }

    fn fill_constraints(
        line: &[Tile],
        constraints: &mut [Constraint],
        vocab: &Set<impl AsRef<[u8]> + Sync>,
    ) {
        // Avoid a bunch of allocations b/c they can become very expensive
        let mut prefix = Vec::with_capacity(BOARD_SIZE);
        let mut suffix = Vec::with_capacity(BOARD_SIZE);
        constraints
            .iter_mut()
            .enumerate()
            .for_each(|(i, constraint)| {
                *constraint = match line[i] {
                    Tile::Letter(l) => Constraint::Filled(l),
                    _ => {
                        // Find a prefix
                        prefix.clear();
                        for j in (0..i).rev() {
                            match line[j] {
                                Tile::Letter(l) => prefix.insert(0, l),
                                _ => break,
                            };
                        }
                        // Find the suffix
                        suffix.clear();
                        for j in (i + 1)..(line.len()) {
                            match line[j] {
                                Tile::Letter(l) => suffix.push(l),
                                _ => break,
                            };
                        }

                        let letters = if prefix.is_empty() && suffix.is_empty() {
                            LetterSet::any()
                        } else {
                            let automaton = ConstraintSearcher {
                                prefix: &prefix,
                                suffix: &suffix,
                            };
                            // Use the vocabulary to scan for valid letters. If we want to place a new word that lies
                            // directly adjacent to the current word. All those new words formed along the edge must be valid
                            let mut matches = vocab.search_with_state(automaton).into_stream();
                            let mut letter_set = LetterSet::empty();
                            while let Some((_, state)) = matches.next() {
                                if let Some(ConstraintSearcherState::Done(l)) = state {
                                    letter_set.insert(l);
                                } else {
                                    unreachable!("not in final state");
                                }
                            }

                            letter_set
                        };

                        Constraint::Empty(letters)
                    }
                }
            })
    }

    fn is_empty(&self) -> bool {
        for i in 0..BOARD_SIZE {
            for j in 0..BOARD_SIZE {
                let pos = Position { row: i, col: j };
                if self.is_filled(pos) {
                    return false;
                }
            }
        }
        true
    }

    /// Computes the valid queries given constraints. Finds all anchor points and
    /// computes a minimum length that a word must match as well as a subslice of the overall constraint array
    /// that is used for searching
    pub fn compute_queries(&self) -> Vec<ConstraintQuery> {
        let mut line = ConstraintIndex(Position { row: 0, col: 0 }, self.dir.flip());
        let is_empty = self.is_empty();
        std::iter::from_fn(move || {
            if line.0[self.dir] >= 15 {
                return None;
            }
            let mut head = line.clone();
            let line_slice = &self.state[line.0[self.dir]][..];
            line = line.perp().next().perp();
            Some(std::iter::from_fn(move || {
                while head.0[self.dir.flip()] < 15 {
                    // skip the Tile just after a tile
                    match line_slice.get(head.back().0[self.dir.flip()]) {
                        None | Some(Constraint::Empty(_)) => break,
                        Some(Constraint::Filled(_)) => {
                            head = head.next();
                            continue;
                        }
                    }
                }

                if head.0[self.dir.flip()] >= 15 {
                    return None;
                }

                let sub_slice = &line_slice[head.0[self.dir.flip()]..];
                let place = head.clone();
                head = head.next();

                // find minimum length to be attached: first Tile that is filled or that have constraints (some perpendicular word)
                let mut end = place.clone();
                while end.0[self.dir.flip()] < 15 {
                    if is_empty && end.0 == (Position { row: 7, col: 7 }) {
                        break;
                    }
                    match line_slice[end.0[self.dir.flip()]] {
                        Constraint::Empty(letter_set) if letter_set.is_any() => {
                            end = end.next();
                            continue;
                        }
                        _ => break,
                    }
                }

                if end.0[self.dir.flip()] == 15 {
                    // The line is empty
                    return None;
                }
                let query = ConstraintQuery {
                    constraints: sub_slice,
                    dir: place.1,
                    pos: place.0,
                    min_length: (end.0[self.dir.flip()] - place.0[self.dir.flip()] + 1).max(2),
                };
                Some(query)
            }))
        })
        .flatten()
        .collect()
    }

    fn is_filled(&self, pos: Position) -> bool {
        match self.state[pos.row][pos.col] {
            Constraint::Filled(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::scrabble::board::Tile;
    use crate::scrabble::constraint::grid::ConstraintGrid;
    use crate::scrabble::constraint::letter_set::LetterSet;
    use crate::scrabble::constraint::Constraint;
    use crate::scrabble::util::Letter;

    #[test]
    fn test() {
        use fst::SetBuilder;
        use std::iter::FromIterator;

        let mut words = vec!["lore", "love", "elle", "bles"];

        words.sort_unstable();

        let mut build = SetBuilder::memory();
        build.extend_iter(words).unwrap();
        let dict = build.into_set();

        let line = [
            Tile::Letter(Letter::Blank),
            Tile::Empty,
            Tile::Empty,
            Tile::Letter(Letter::Blank),
            Tile::Letter(Letter::Letter('l')),
            Tile::Letter(Letter::Letter('e')),
            Tile::Empty,
            Tile::Empty,
            Tile::Empty,
            Tile::Letter(Letter::Letter('l')),
            Tile::Letter(Letter::Letter('o')),
            Tile::Empty,
            Tile::Letter(Letter::Letter('e')),
        ];

        let mut restr = [
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
            Constraint::Empty(LetterSet::empty()),
        ];

        ConstraintGrid::fill_constraints(&line, &mut restr, &dict);

        dbg!(&restr);
        assert_eq!(
            restr,
            [
                Constraint::Filled(Letter::Blank),
                Constraint::Empty(LetterSet::empty()),
                Constraint::Empty(LetterSet::from_iter("e".chars())),
                Constraint::Filled(Letter::Blank),
                Constraint::Filled(Letter::Letter('l')),
                Constraint::Filled(Letter::Letter('e')),
                Constraint::Empty(LetterSet::from_iter("s".chars())),
                Constraint::Empty(LetterSet::any()),
                Constraint::Empty(LetterSet::empty()),
                Constraint::Filled(Letter::Letter('l')),
                Constraint::Filled(Letter::Letter('o')),
                Constraint::Empty(LetterSet::from_iter("vr".chars())),
                Constraint::Filled(Letter::Letter('e')),
            ]
        );
    }
}
