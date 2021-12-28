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
