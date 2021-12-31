use fst::Automaton;

use crate::scrabble::util::Letter;


#[derive(Clone, Debug)]
pub enum ConstraintSearcherState {
    Prefix(usize),
    Mid,
    Suffix(usize, char),
    Done(char),
}

pub struct ConstraintSearcher<'a> {
    pub prefix: &'a [Letter],
    pub suffix: &'a [Letter],
}

impl<'a> Automaton for ConstraintSearcher<'a> {
    type State = Option<ConstraintSearcherState>;

    fn start(&self) -> Self::State {
        if self.prefix.len() == 0 {
            Some(ConstraintSearcherState::Mid)
        } else {
            Some(ConstraintSearcherState::Prefix(0))
        }
    }

    fn is_match(&self, state: &Self::State) -> bool {
        match state {
            Some(ConstraintSearcherState::Done(_)) => true,
            _ => false,
        }
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        let byte_char = char::from_u32(byte as u32).unwrap();
        state.as_ref().and_then(|s| match s {
            &ConstraintSearcherState::Prefix(i) => {
                let ok = match self.prefix[i] {
                    Letter::Blank => true,
                    Letter::Letter(l) => l == byte_char,
                };
                if ok {
                    Some(if i + 1 == self.prefix.len() {
                        ConstraintSearcherState::Mid
                    } else {
                        ConstraintSearcherState::Prefix(i + 1)
                    })
                } else {
                    None
                }
            }
            ConstraintSearcherState::Mid => Some(if self.suffix.len() == 0 {
                ConstraintSearcherState::Done(byte_char)
            } else {
                ConstraintSearcherState::Suffix(0, byte_char)
            }),
            &ConstraintSearcherState::Suffix(i, letter) => {
                let ok = match self.suffix.get(i) {
                    None => false,
                    Some(Letter::Blank) => true,
                    Some(&Letter::Letter(letter)) => byte_char == letter,
                };
                if ok {
                    Some(if i + 1 == self.suffix.len() {
                        ConstraintSearcherState::Done(letter)
                    } else {
                        ConstraintSearcherState::Suffix(i + 1, letter)
                    })
                } else {
                    None
                }
            }
            &ConstraintSearcherState::Done(_) => None,
        })
    }

    fn can_match(&self, _state: &Self::State) -> bool {
        _state.is_some()
    }
}