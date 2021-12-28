use std::collections::HashSet;
use std::slice::Iter;

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
            } else {
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
            Direction::Down => &self.row,
        }
    }
}

impl std::ops::IndexMut<Direction> for Position {
    fn index_mut(&mut self, index: Direction) -> &mut Self::Output {
        match index {
            Direction::Across => &mut self.col,
            Direction::Down => &mut self.row,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Move {
    pub word: String,
    pub pos: Position,
    pub dir: Direction,
    pub score: i32,
}

impl Move {
    pub fn iter(&self) -> IterMove {
        IterMove { _m: self, _curr: 0 }
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
    Letter(char),
}

impl Letter {
    pub fn as_index(&self) -> usize {
        match self {
            Self::Blank => 27,
            Self::Letter(l) => *l as usize - 'A' as usize,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SquareEffect {
    DoubleWord,
    DoubleLetter,
    TripleWord,
    TripleLetter,
    Center,
}

/// Takes array dimensions and computes a stride
pub fn dim_to_stride(dim: &[usize]) -> Vec<usize> {
    let mut stride = Vec::new();
    stride.push(1);
    for i in 0..dim.len() - 1 {
        stride.push(stride[i] * dim[i]);
    }
    stride.reverse();
    stride
}

/// Utility function to convert a N-d index into a 1d index given a stride
/// that represents the bounds of the parent grid
pub fn coord_to_index(coord: &[usize], dim: &[usize]) -> usize {
    let mut idx = 0;
    let stride = dim_to_stride(dim);
    
    for i in 0..coord.len() {
        idx += stride[i] * coord[i];
    }
    idx
}

/// Reverse of coord to index
pub fn index_to_coord(idx: usize, dim: &[usize]) -> Vec<usize> {
    let mut coord = vec![];
    let stride = dim_to_stride(dim);
    for i in 0..dim.len() {
        let partial_idx = (idx / stride[i]) % dim[i];
        coord.push(partial_idx);
        
    }
    coord
}

#[cfg(test)]
mod tests {
    use crate::scrabble::util::index_to_coord;

    use super::coord_to_index;

    #[test]
    fn test_coord_to_index() {
        let coords: Vec<&[usize]> = vec![
            &[1, 5], // random position in the middle
            &[2, 2], // Last index
            &[0, 0, 1]
        ];
        let stride: Vec<&[usize]> = vec![
            &[15, 15],   // 15x15 grid
            &[3, 3],     // 9x9 grid
            &[3, 3, 3]
        ];

        let expected = vec![
            20,
            8,
            1
        ];

        for i in 0..coords.len() {
            let actual = coord_to_index(coords[i], stride[i]);
            assert_eq!(actual, expected[i]);
        }
    }

    #[test]
    fn test_coord_index_invertible() {
        // Assumes that the above test case passes.
        // Just makes sure th
        let coords = vec![
            vec![0, 1, 3],
            vec![1, 5], // random position in the middle
            vec![2, 2], // Last index
            
        ];
        let dims: Vec<&[usize]> = vec![
            &[15, 15, 5],
            &[15, 15],   // 15x15 grid
            &[3, 3],     // 9x9 grid
            
        ];

        for i in 0..coords.len() {
            let idx = coord_to_index(&coords[i], dims[i]);
            let actual = index_to_coord(idx, dims[i]);
            assert_eq!(actual, coords[i]);
        }
    }
}