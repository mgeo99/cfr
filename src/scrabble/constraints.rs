use fst::{Automaton, IntoStreamer, Set, Streamer};
use rayon::prelude::*;

use crate::scrabble::util::Position;
use crate::scrabble::BOARD_SIZE;

use super::board::{ScrabbleBoard, Tile};
use super::constraint::letter_set::LetterSet;
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

            // Copy the tiles on the board at the current position
            // Note: This may be transposed if we are going in the opposite direction
            for j in 0..BOARD_SIZE {
                tile_buffer[j] = curr_board[curr_pos].clone();
                curr_pos[dir] += 1;
            }
            let mut constraint_buffer = [ConstrainedTile::Letters(LetterSet::empty()); BOARD_SIZE];

            // Fill the constraints using the current tiles on the board
            fill_constraints(&tile_buffer, &mut constraint_buffer, vocab);
            
            for j in 0..BOARD_SIZE {
                state[j][i] = constraint_buffer[j];
            }

            // Go to the next row
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

    /// Generates all of the constraint sequences that can should be used as a search query
    /// in the given direction.
    /// We need to generate several queries for our fst to find matches for. The queries we generate should
    /// be analagous to the types of queries in the original paper: https://www.cs.cmu.edu/afs/cs/academic/class/15451-s06/www/lectures/scrabble.pdf
    /// We first need to generate queries that imitate expand-left (aka find all possible left words)
    /// and then do the same for expand right. The slices we emit follow the following format:
    /// (Pos, Slice, min_length)
    ///
    /// Lets say we have a row with the following constraints:
    ///     ? - Anything can go there
    ///     <c> - That character MUST be present
    ///         [?, ?, H, E, L, L, ?, ?, C, ?, T]
    ///         [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    ///     Just to get your imagination going lets pretend that our vocab has HELLO and CAT as words. Lets also assume that
    ///     the cross checks that formed these constraints are correct
    /// First, we need to find the anchor positions for the row.
    /// An anchor position is blank with one with an adjacent tile that we can use to start a new word. The rules of scrabble say all new words
    /// (except the first one) must be built off another character
    /// For this particular row, the anchors are: 1, 6, 7 and 9.
    /// Now we need to find the MINIMUM length for any words find off of the anchor. We do this by scanning until we reach
    /// another blank character
    ///  This is so that when we go to do an automaton search, we
    /// can mimic the extend-right behavior of the original paper all the way to the end of the line
    /// So here are the minimum lengths of any words starting at the anchors in the example:
    ///     1: 5 -> Go to the end of HELL
    ///     6: 1 -> The next blank is immediately to the right
    ///     7: 2 -> Also go to the blank before C
    ///     9: 2 -> Go to the T and reach the end of the board
    /// To simplify the candidate generation and leverage fst's automatons, once we compute these minimum lengths, all that's
    /// left is to iteratively expand out to the left and just use the line starting at that new position as a candidate
    /// The procedure for going to the left is as follows:
    ///     1. Find the next blank space / beginning of the word we are anchored to
    ///     2. Expand left until we hit the previous anchor position adding a slice at each position
    ///     all the way to the end of the line as a candidate. Must also update the min-length to account for
    ///     the length of this new word because our automaton impl treats the # of successful transitions as length meaning we are iteratively finding candidate words
    ///         2a. Add this partial expansion to the list of candidates
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
                    if is_empty && end.0.is_center() {
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
    // Avoid a bunch of allocations b/c they can become very expensive
    let mut prefix = Vec::with_capacity(BOARD_SIZE);
    let mut suffix = Vec::with_capacity(BOARD_SIZE);
    constraints
        .iter_mut()
        .enumerate()
        .for_each(|(i, constraint)| {
            *constraint = match line[i] {
                Tile::Letter(l) => ConstrainedTile::Filled(l),
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
                        let automaton = ConstraintBuilder {
                            prefix: &prefix,
                            suffix: &suffix,
                        };
                        // Use the vocabulary to scan for valid letters. If we want to place a new word that lies
                        // directly adjacent to the current word. All those new words formed along the edge must be valid
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
