use std::rc::Rc;

use fst::Automaton;

use super::{constraints::ConstrainedTile, util::Letter};

/*
    Automaton implemented to assist with rapidly searching over all the available positions
    in the scrabble board using the FST crate.


*/

#[derive(Debug, Clone)]
pub struct TrayRemaining {
    letters: [u8; 256],
    n_blanks: u8,
    /// The total number of remaining letters+wildcards to play
    n_total: u32,
}

impl TrayRemaining {
    pub fn remove(&self, letter: char) -> Option<TrayRemaining> {
        if self.letters[letter as usize] > 0 {
            let mut tmp = self.clone();
            tmp.letters[letter as usize] -= 1;
            tmp.n_total -= 1;
            Some(tmp)
        } else {
            None
        }
    }
    pub fn remove_wildcard(&self) -> Option<TrayRemaining> {
        if self.n_blanks > 0 {
            let mut tmp = self.clone();
            tmp.n_blanks -= 1;
            tmp.n_total -= 1;
            Some(tmp)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlankAssignmentList {
    Empty,
    Elem(char, Rc<BlankAssignmentList>),
}

impl TrayRemaining {
    pub fn new(letters: [u8; 256], n_blanks: u8) -> TrayRemaining {
        let n_total = letters.iter().map(|&i| i as u32).sum::<u32>() + n_blanks as u32;
        TrayRemaining {
            letters,
            n_blanks,
            n_total,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WordSearcherState {
    /// Position on the line
    pub position: usize,
    /// Blank spaces that we have assigned
    pub blank_assignments: BlankAssignmentList,
    /// Remaning cards in the rack
    pub rack: TrayRemaining,
}

#[derive(Debug, Clone)]
pub struct WordSearcher<'a> {
    /// The line we are searching for words on with constraints
    pub line: &'a [ConstrainedTile],
    /// Letters we have remaining in our rack (separated from whitespace)
    pub rack: TrayRemaining,
    /// Minimum length for anything to be considered a word
    pub min_length: usize,
}

impl<'a> Automaton for WordSearcher<'a> {
    type State = Option<WordSearcherState>;

    fn start(&self) -> Self::State {
        Some(WordSearcherState {
            position: 0,
            rack: self.rack.clone(),
            blank_assignments: BlankAssignmentList::Empty,
        })
    }

    fn is_match(&self, state: &Self::State) -> bool {
        if let Some(state) = state {
            // The word we are currently on does not match whats in the line
            if let Some(ConstrainedTile::Filled(_)) = self.line.get(state.position) {
                false
            } else {
                // Cannot match if we haven't tested any tiles
                if self.rack.n_total == state.rack.n_total {
                    false
                } else {
                    // Must have a word at least as long as the min length
                    if state.position < self.min_length {
                        false
                    } else {
                        true
                    }
                }
            }
        } else {
            false
        }
    }

    fn accept(&self, curr_state: &Self::State, byte: u8) -> Self::State {
        let letter = byte as char;
        curr_state.as_ref().and_then(|state| {
            match self.line.get(state.position) {
                None => None,
                Some(tile) => match tile {
                    // Blanks can be used to match any letter
                    ConstrainedTile::Filled(Letter::Blank) => Some(WordSearcherState {
                        position: state.position + 1,
                        blank_assignments: state.blank_assignments.clone(),
                        rack: state.rack.clone(),
                    }),
                    // The letter must match the tile we accept
                    &ConstrainedTile::Filled(Letter::Letter(l)) => {
                        if l == letter {
                            Some(WordSearcherState {
                                position: state.position + 1,
                                blank_assignments: state.blank_assignments.clone(),
                                rack: state.rack.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    // Consume the letter from our tray or use a blank. If we cant do either then return None again
                    &ConstrainedTile::Letters(letter_set) => {
                        if letter_set.is_empty() {
                            None
                        } else {
                            let (new_rack, blank_assignment) = state
                                .rack
                                .remove(letter)
                                // We have the required letter
                                .map(|t| (Some(t), None))
                                .or_else(|| {
                                    // We use the blank as the missing letter
                                    state
                                        .rack
                                        .remove_wildcard()
                                        .map(|r| (Some(r), Some(letter)))
                                })
                                .unwrap_or((None, None));
                            new_rack.map(|rack| WordSearcherState {
                                position: state.position + 1,
                                blank_assignments: if let Some(assig) = blank_assignment {
                                    BlankAssignmentList::Elem(
                                        assig,
                                        Rc::new(state.blank_assignments.clone()),
                                    )
                                } else {
                                    state.blank_assignments.clone()
                                },
                                rack,
                            })
                        }
                    }
                },
            }
        })
    }

    fn can_match(&self, _state: &Self::State) -> bool {
        _state.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrabble::{word_search::WordSearcher, letter_set::LetterSet};

    #[test]
    fn test_simple_search() {

        let line = [
            ConstrainedTile::Letters("abdfghklmopqstx".chars().collect()),
            ConstrainedTile::Letters(
                "abdefghijklmnopqrstuwxyz"
                    .chars()
                    .collect(),
            ),
            ConstrainedTile::Letters("a".chars().collect()),
            ConstrainedTile::Letters(LetterSet::any()),
            ConstrainedTile::Letters(LetterSet::any()),
            ConstrainedTile::Letters(LetterSet::any()),
        ];

        let automaton = WordSearcher {
            line: &line[..],
            rack: TrayRemaining {
                letters: [1; 256],
                n_blanks: 1,
                n_total: 257,
            },
            min_length: 0
        };


        let mut build = fst::SetBuilder::memory();
        build.insert(b"tepa").unwrap();
        let dict = build.into_set();

        use fst::{IntoStreamer, Streamer};

        let mut x = dict.search_with_state(automaton).into_stream();

        let mut acc = vec![];

        while let Some(w) = x.next() {
            acc.push((
                std::str::from_utf8(w.0).unwrap().to_string(),
                w.1.expect("reached valid state"),
            ))
        }

        assert_eq!(acc.len(), 1);

        assert_eq!(acc[0].0, "tepa");
        assert_eq!(acc[0].1.position, 4);
    }
}
