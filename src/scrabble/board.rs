use std::collections::HashSet;

use super::dictionary::TrieNode;

/// Effect that a square has on the placed word/letter
#[derive(Debug)]
enum SquareEffect {
    None,
    DoubleWord,
    DoubleLetter,
    TripleWord,
    TripleLetter,
}
#[derive(Debug, Clone, Copy)]
enum Direction {
    Across,
    Down,
}

/// Square on a scrabble board
#[derive(Debug)]
struct Square {
    /// The currently placed tile on the board
    tile: Option<char>,
    /// Effect of the square
    effect: SquareEffect,
    /// Set of letters that can form valid cross-words across
    across_words: HashSet<char>,
    /// Set of letters that can form valid cross-words downwards
    down_words: HashSet<char>,
}

#[derive(Debug)]
struct Placement {
    word: String,
    root: (usize, usize),
    direction: bool,
}

pub struct ScrabbleBoard {
    board_size: usize,
    squares: Vec<Square>,
}

impl ScrabbleBoard {
    fn get_square(&self, i: usize, j: usize) -> &Square {
        let idx = i * self.board_size + j;
        &self.squares[idx]
    }
    fn get_square_mut(&mut self, i: usize, j: usize) -> &mut Square {
        let idx = i * self.board_size + j;
        &mut self.squares[idx]
    }
    /// Gets all the placements where the given word can be placed
    pub fn get_valid_placements(&self, words: &[&str]) {}

    fn generate_internal(
        &self,
        anchor: (usize, usize),
        dir: Direction,
        pos: usize,
        word: &str,
        rack: &str,
        trie: &TrieNode,
        new_tiles: Vec<char>,
    ) {
        let (i, j) = self.compute_offset(anchor, dir, pos);
        let square = self.get_square(i, j);
        if let Some(tile) = square.tile {
            let next_tiles = new_tiles.clone();
            if let Some(next_node) = trie.next_node(tile) {}
        }
    }

    fn walk_next_tiles(
        &self,
        anchor: (usize, usize),
        dir: Direction,
        pos: usize,
        curr_trie: Option<&TrieNode>,
        prev_trie: &TrieNode,
        new_tiles: Vec<char>
    ) 
    {
        
        let left_pos = self.compute_offset(anchor, dir, pos - 1);
    }

    fn compute_offset(
        &self,
        coord: (usize, usize),
        dir: Direction,
        offset: usize,
    ) -> (usize, usize) {
        match dir {
            Direction::Across => (coord.0, coord.1 + offset),
            Direction::Down => (coord.0 + offset, coord.1),
        }
    }
}
