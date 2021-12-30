use fst::{Automaton, IntoStreamer, Set, Streamer};
use rayon::prelude::*;

use crate::scrabble::util::Position;
use crate::scrabble::BOARD_SIZE;

use super::board::{ScrabbleBoard, Tile};
use super::letter_set::LetterSet;
use super::util::{Direction, Letter};

#[derive(Debug, Clone, Copy)]
pub enum ConstrainedTile {
    Filled(Letter),
    Letters(LetterSet),
}

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

/// Board used when applying an automaton to suggest valid words given an input board state
#[derive(Debug, Clone)]
pub struct ConstraintBoard {
    /// Direction used to construct the constraints
    dir: Direction,
    /// Table with tiles and their restrictions
    state: Vec<Vec<ConstrainedTile>>,
}

impl ConstraintBoard {
    /// Builds a constraint board using the existing board for a specific direction
    pub fn build(
        curr_board: &ScrabbleBoard,
        dir: Direction,
        vocab: &Set<impl AsRef<[u8]> + Sync>,
    ) -> Self {
        let mut pos = Position { row: 0, col: 0 };
        let mut state =
            vec![vec![ConstrainedTile::Letters(LetterSet::empty()); BOARD_SIZE]; BOARD_SIZE];
        for i in 0..BOARD_SIZE {
            let mut tile_buffer = [Tile::Empty; BOARD_SIZE];
            let mut curr_pos = pos.clone();

            for j in 0..BOARD_SIZE {
                tile_buffer[j] = curr_board[curr_pos].clone();
                curr_pos[dir] += 1;
            }

            let mut constraint_buffer = [ConstrainedTile::Letters(LetterSet::empty()); BOARD_SIZE];
            fill_constraints(&tile_buffer, &mut constraint_buffer, vocab);
            for j in 0..BOARD_SIZE {
                state[j][i] = constraint_buffer[j];
            }
            pos[dir.flip()] += 1;
        }

        Self { dir, state }
    }

    fn is_empty(&self) -> bool {
        for i in 0..BOARD_SIZE {
            for j in 0..BOARD_SIZE {
                if let ConstrainedTile::Filled(_) = self.state[i][j] {
                    return false;
                }
            }
        }
        true
    }

    /// Gets the ranges of tiles that should be searched over to find vocabulary words
    pub fn get_candidate_slices<'a>(
        &'a self,
    ) -> impl Iterator<Item = (ConstraintIndex, &[ConstrainedTile], usize)> + 'a {
        // Use some std iterator magic to make life easy here
        let mut line = ConstraintIndex(Position { row: 0, col: 0 }, self.dir.flip());
        let is_empty = self.is_empty();
        std::iter::from_fn(move || {
            if line.0[self.dir] >= BOARD_SIZE {
                return None;
            }
            let mut head = line.clone();
            let line_slice = &self.state[line.0[self.dir]][..];
            line = line.perp().next().perp();
            Some(std::iter::from_fn(move || {
                while head.0[self.dir.flip()] < BOARD_SIZE {
                    // skip the square just after a tile
                    match line_slice.get(head.back().0[self.dir.flip()]) {
                        None | Some(ConstrainedTile::Letters(_)) => break,
                        Some(ConstrainedTile::Filled(_)) => {
                            head = head.next();
                            continue;
                        }
                    }
                }

                if head.0[self.dir.flip()] >= BOARD_SIZE {
                    return None;
                }

                let sub_slice = &line_slice[head.0[self.dir.flip()]..];
                let place = head.clone();
                head = head.next();

                // find minimum length to be attached: first square that is filled or that have constraints (some perpendicular word)
                let mut end = place.clone();
                while end.0[self.dir.flip()] < BOARD_SIZE {
                    if is_empty
                        && end.0
                            == (Position {
                                row: BOARD_SIZE / 2,
                                col: BOARD_SIZE / 2,
                            })
                    {
                        break;
                    }
                    match line_slice[end.0[self.dir.flip()]] {
                        ConstrainedTile::Letters(letter_set) if letter_set.is_any() => {
                            end = end.next();
                            continue;
                        }
                        _ => break,
                    }
                }

                if end.0[self.dir.flip()] == BOARD_SIZE {
                    // The line is empty
                    return None;
                }

                Some((
                    place,
                    sub_slice,
                    (end.0[self.dir.flip()] - place.0[self.dir.flip()] + 1).max(2),
                ))
            }))
        })
        .flatten()
    }
}

#[derive(Clone, Debug)]
enum ConstraintBuilderState {
    Prefix(usize),
    Mid,
    Suffix(usize, char),
    Done(char),
}

struct ConstraintBuilder<'a> {
    prefix: &'a [Letter],
    suffix: &'a [Letter],
}

impl<'a> Automaton for ConstraintBuilder<'a> {
    type State = Option<ConstraintBuilderState>;

    fn start(&self) -> Self::State {
        if self.prefix.len() == 0 {
            Some(ConstraintBuilderState::Mid)
        } else {
            Some(ConstraintBuilderState::Prefix(0))
        }
    }

    fn is_match(&self, state: &Self::State) -> bool {
        match state {
            Some(ConstraintBuilderState::Done(_)) => true,
            _ => false,
        }
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        state.as_ref().and_then(|s| match s {
            &ConstraintBuilderState::Prefix(i) => {
                let ok = match self.prefix[i] {
                    Letter::Blank => true,
                    Letter::Letter(l) => l as u8 == byte,
                };
                if ok {
                    Some(if i + 1 == self.prefix.len() {
                        ConstraintBuilderState::Mid
                    } else {
                        ConstraintBuilderState::Prefix(i + 1)
                    })
                } else {
                    None
                }
            }
            ConstraintBuilderState::Mid => Some(if self.suffix.len() == 0 {
                ConstraintBuilderState::Done(byte as char)
            } else {
                ConstraintBuilderState::Suffix(0, byte as char)
            }),
            &ConstraintBuilderState::Suffix(i, letter) => {
                let ok = match self.suffix.get(i) {
                    None => false,
                    Some(Letter::Blank) => true,
                    Some(&Letter::Letter(l)) => l == letter,
                };
                if ok {
                    Some(if i + 1 == self.suffix.len() {
                        ConstraintBuilderState::Done(letter)
                    } else {
                        ConstraintBuilderState::Suffix(i + 1, letter)
                    })
                } else {
                    None
                }
            }
            &ConstraintBuilderState::Done(_) => None,
        })
    }

    fn can_match(&self, _state: &Self::State) -> bool {
        _state.is_some()
    }
}

pub fn fill_constraints(
    line: &[Tile],
    constraints: &mut [ConstrainedTile],
    vocab: &Set<impl AsRef<[u8]> + Sync>,
) {
    let mut prefix = Vec::with_capacity(BOARD_SIZE);
    let mut suffix = Vec::with_capacity(BOARD_SIZE);
    constraints
        .iter_mut()
        .enumerate()
        .for_each(|(i, constraint)| {
            *constraint = match line[i] {
                Tile::Letter(l) => ConstrainedTile::Filled(l),
                _ => {
                    /*
                        Even though the search routine upstream relies on these constraints,
                        do we actually need this to still have valid moves???

                        Tiles that are filled will still have the constraint in place that they must match
                        that letter EXACTLY. Other tiles (like what is being accounted for here) can essentially
                        be set to just anything because it is possible we will run into another tile with a letter
                        already there that MUST match. Since words are all or nothing in this context we can't just place
                        a random letter down. So implicitly we will find the right word even without a constraint
                        based on our vocabulary
                    */

                    // Otherwise scan for letters using prefix/suffix
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
                        let automaton = ConstraintBuilder {
                            prefix: &prefix,
                            suffix: &suffix,
                        };
                        // Use the vocabulary to scan for valid letters
                        let mut matches = vocab.search_with_state(automaton).into_stream();
                        let mut letter_set = LetterSet::empty();
                        while let Some((_, state)) = matches.next() {
                            if let Some(ConstraintBuilderState::Done(l)) = state {
                                letter_set.insert(l);
                            }
                        }

                        letter_set
                    };

                    ConstrainedTile::Letters(letters)
                }
            }
        })
}

// Test Cases? LOL u thought. I'll do these later
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrabble::board::ScrabbleBoard;
    use crate::scrabble::util::Direction;
    use fst::SetBuilder;
    #[test]
    fn test_build_board() {
        let mut words = vec!["lore", "love", "elle", "bles"];
        words.sort_unstable();
        let mut build = SetBuilder::memory();
        build.extend_iter(words).unwrap();
        let vocab = build.into_set();

        let board = ScrabbleBoard::empty();

        let constrained_board = ConstraintBoard::build(&board, Direction::Across, &vocab);
    }
}
