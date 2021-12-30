use super::util::Letter;

#[derive(Debug, Clone)]
pub struct Rack {
    /// Histogram count of each letter in the rack
    pub letters: [u8; 256],
    /// Number of blanks in the rack
    pub n_blanks: u8,
    /// The total number of remaining letters+wildcards to play
    pub n_total: u32,
}

impl Rack {
    pub fn new(letters: [u8; 256], n_blanks: u8) -> Self {
        let n_total = letters.iter().map(|&i| i as u32).sum::<u32>() + n_blanks as u32;
        Self {
            letters,
            n_blanks,
            n_total,
        }
    }

    pub fn empty() -> Self {
        Self::new([0; 256], 0)
    }

    /// Adds an additional letter in-place. Should only be used when handling state transitions
    pub fn add_inplace(&mut self, letter: Letter) {
        match letter {
            Letter::Blank => self.n_blanks += 1,
            Letter::Letter(l) => self.letters[l as usize] += 1,
        };
        self.n_total += 1;
    }

    /// Does an in-place removal of the provided letter without any checking.
    /// Should only be used when handling state transitions in the game itself
    pub fn remove_inplace(&mut self, letter: Letter) {
        match letter {
            Letter::Blank => self.n_blanks -= 1,
            Letter::Letter(l) => self.letters[l as usize] -= 1,
        };
        self.n_total -= 1;
    }

    /// Used for automaton state searching
    pub fn remove(&self, letter: char) -> Option<Self> {
        if self.letters[letter as usize] > 0 {
            let mut tmp = self.clone();
            tmp.letters[letter as usize] -= 1;
            tmp.n_total -= 1;
            Some(tmp)
        } else {
            None
        }
    }

    /// Used for automaton state searching
    pub fn remove_wildcard(&self) -> Option<Self> {
        if self.n_blanks > 0 {
            let mut tmp = self.clone();
            tmp.n_blanks -= 1;
            tmp.n_total -= 1;
            Some(tmp)
        } else {
            None
        }
    }

    pub fn get_letters(&self) -> Vec<Letter> {
        let mut letters = Vec::new();
        for _ in 0..self.n_blanks {
            letters.push(Letter::Blank);
        }
        for i in 0..256 {
            for _ in 0..self.letters[i] {
                letters.push(Letter::Letter(char::from_u32(i as u32).unwrap()));
            }
        }
        letters
    }
}

impl std::iter::FromIterator<char> for Rack {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut chars = [0; 256];
        let mut n_blanks = 0;
        iter.into_iter().for_each(|x| {
            if x != '-' {
                chars[x as usize] += 1
            } else {
                n_blanks += 1;
            }
        });
        Self::new(chars, n_blanks)
    }
}
