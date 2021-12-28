use std::collections::HashMap;

use rand::prelude::SliceRandom;

use super::util::Letter;

#[derive(Debug, Clone)]
pub struct Bag {
    /// Alphabetic characters in the bag
    alph: [Letter; 27],
    /// Score associated with each
    amts: [usize; 27],
    values: [i32; 27],
    scores: HashMap<Letter, i32>,
    random: bool,
    pub distribution: Vec<Letter>,
}

impl Bag {
    pub fn default() -> Bag {
        let mut bag = Bag {
            alph: [
                'A'.into(),
                'B'.into(),
                'C'.into(),
                'D'.into(),
                'E'.into(),
                'F'.into(),
                'G'.into(),
                'H'.into(),
                'I'.into(),
                'J'.into(),
                'K'.into(),
                'L'.into(),
                'M'.into(),
                'N'.into(),
                'O'.into(),
                'P'.into(),
                'Q'.into(),
                'R'.into(),
                'S'.into(),
                'T'.into(),
                'U'.into(),
                'V'.into(),
                'W'.into(),
                'X'.into(),
                'Y'.into(),
                'Z'.into(),
                Letter::Blank,
            ],
            amts: [
                9, 2, 2, 4, 12, 2, 3, 2, 9, 1, 1, 4, 2, 6, 8, 2, 1, 6, 4, 6, 4, 2, 2, 1, 2, 1, 2,
            ],
            values: [
                1, 3, 3, 2, 1, 4, 2, 4, 1, 8, 5, 1, 3, 1, 1, 3, 10, 1, 1, 1, 1, 4, 4, 8, 4, 10, 0,
            ],
            scores: HashMap::new(),
            distribution: Vec::new(),
            random: true,
        };

        for (i, &c) in bag.alph.iter().enumerate() {
            bag.scores.insert(c, bag.values[i]);
        }

        for (i, &c) in bag.alph.iter().enumerate() {
            for _ in 0..bag.amts[i] {
                bag.distribution.push(c);
            }
        }

        bag.distribution.shuffle(&mut rand::thread_rng());

        bag
    }

    pub fn new_with_order(order: &Vec<Letter>) -> Bag {
        let mut b = Bag::default();
        b.distribution = order.to_vec();
        b.random = false;
        b
    }

    pub fn is_empty(&self) -> bool {
        self.distribution.is_empty()
    }

    pub fn score(&self, c: Letter) -> i32 {
        match self.scores.get(&c) {
            Some(i) => *i,
            None => 0,
        }
    }

    pub fn draw_tiles(&mut self, n: usize) -> Vec<Letter> {
        let tiles: Vec<Letter>;
        if self.random {
            tiles = self
                .distribution
                .choose_multiple(&mut rand::thread_rng(), n)
                .cloned()
                .collect();
        } else {
            tiles = self.distribution.iter().take(n).cloned().collect();
        }
        for i in tiles.iter() {
            if let Some(pos) = self.distribution.iter().position(|&x| *i == x) {
                self.distribution.remove(pos);
            }
        }
        tiles
    }
}
