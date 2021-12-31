use self::letter_set::LetterSet;

use super::util::{Letter, Direction, Position};

pub mod letter_set;
pub mod grid;

pub mod searcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Constraint {
    Filled(Letter),
    Empty(LetterSet),
}

#[derive(Debug)]
pub struct ConstraintQuery<'a> {
    pub constraints: &'a [Constraint],
    pub dir: Direction,
    pub pos: Position,
    pub min_length: usize,
}