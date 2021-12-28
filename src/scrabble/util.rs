use std::{collections::HashSet, slice::Iter};

use super::BOARD_SIZE;



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Across,
    Down,
}

impl Direction {
    pub fn iter() -> Iter<'static, Direction> {
        static DIRS: [Direction; 2] = [Direction::Across, Direction::Down];
        DIRS.iter()
    }

    pub fn flip(&self) -> Self {
        match self {
            Self::Across => Self::Down,
            Self::Down => Self::Across,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    /// Returns the next position in the given direction
    pub fn next(&self, dir: Direction) -> Option<Position> {
        match dir {
            Direction::Across => {
                if self.col < BOARD_SIZE {
                    Some(Position {
                        row: self.row,
                        col: self.col + 1,
                    })
                } else {
                    None
                }
            }
            Direction::Down => {
                if self.row < BOARD_SIZE {
                    Some(Position {
                        row: self.row + 1,
                        col: self.col,
                    })
                } else {
                    None
                }
            }
        }
    }
    /// Returns the previous position in the given direction
    pub fn prev(&self, dir: Direction) -> Option<Position> {
        match dir {
            Direction::Across => {
                if self.col != 0 {
                    Some(Position {
                        row: self.row,
                        col: self.col - 1,
                    })
                } else {
                    None
                }
            }
            Direction::Down => {
                if self.row != 0 {
                    Some(Position {
                        row: self.row - 1,
                        col: self.col,
                    })
                } else {
                    None
                }
            }
        }
    }
    /// Returns all the valid adjacent positions to this position
    pub fn adjacent(&self) -> Vec<Position> {
        let mut result = Vec::new();
        for d in Direction::iter() {
            if let Some(pos) = self.next(*d) {
                result.push(pos);
            }
            if let Some(pos) = self.prev(*d) {
                result.push(pos);
            }
        }
        result
    }

    /// Converts the row/col to a 1d index
    pub fn as_index(&self) -> usize {
        self.row * BOARD_SIZE + self.col
    }

    /// Moves the position forward a fixed number of steps
    pub fn step_n(&self, n: usize, dir: Direction) -> Option<Position> {
        let mut p = *self;
        for _ in 0..n {
            if let Some(next) = p.next(dir) {
                p = next;
            }
            else {
                return None;
            }
        }
        Some(p)
    }
}

impl std::ops::Index<Direction> for Position {
    type Output = usize;

    fn index(&self, index: Direction) -> &Self::Output {
        match index {
            Direction::Across => &self.col,
            Direction::Down => &self.row
        }
    }
}


impl std::ops::IndexMut<Direction> for Position {

    fn index_mut(&mut self, index: Direction) -> &mut Self::Output {
        match index {
            Direction::Across => &mut self.col,
            Direction::Down => &mut self.row
        }
    }
}



#[derive(Debug, Clone)]
pub struct Move {
    pub word: String,
    pub pos: Position,
    pub dir: Direction,
    pub score: i32
}

impl Move {
    pub fn iter(&self) -> IterMove {
        IterMove {
            _m: self,
            _curr: 0
        }
    }
}

pub struct IterMove<'a> {
    _m: &'a Move,
    _curr: usize,
}

impl<'a> Iterator for IterMove<'a> {
    type Item = (Position, char);

    fn next(&mut self) -> Option<Self::Item> {
        match self._m.pos.step_n(self._curr, self._m.dir) {
            Some(p) => match self._m.word.chars().nth(self._curr as usize) {
                Some(c) => {
                    self._curr += 1;
                    Some((p, c))
                }
                None => None,
            },
            None => None,
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Letter {
    Blank,
    Letter(char)
}

impl Letter {
    pub fn as_index(&self) -> usize {
        match self {
            Self::Blank => 27,
            Self::Letter(l) => *l as usize - 'A' as usize
        }
    }
}




#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SquareEffect {
    DoubleWord,
    DoubleLetter,
    TripleWord,
    TripleLetter,
    Center
}
