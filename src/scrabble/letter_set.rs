

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct LetterSet {
    // bit is one if letter is in it
    accepted: [u128; 2],
}

impl LetterSet {
    pub fn empty() -> Self {
        Self { accepted: [0; 2] }
    }
    pub fn any() -> Self {
        Self { accepted: [u128::MAX; 2] }
    }
    pub fn contains(&self, letter: char) -> bool {
        let i = letter as usize;
        (self.accepted[i / 128]  & (1 << (i%128))) != 0
    }
    pub fn insert(&mut self, letter: char) {
        let i = letter as usize;
        self.accepted[i / 128]  |= 1 << (i%128)
    }
    pub fn from_many(iter: impl Iterator<Item=char>) -> Self {
        let mut tmp = Self::empty();
        iter.for_each(|l| tmp.insert(l));
        tmp
    }
    pub fn is_empty(&self) -> bool {
        self.accepted.iter().all(|&l| l == 0)
    }
    pub fn is_any(&self) -> bool {
        self.accepted.iter().all(|&l| l == u128::MAX)
    }
    
}

impl Default for LetterSet {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::iter::FromIterator<char> for LetterSet {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = char>,
    {
        let mut tmp = Self::default();
        iter.into_iter().for_each(|l| tmp.insert(l));
        tmp
    }
}

use std::fmt;

impl fmt::Debug for LetterSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_any() {
            write!(f, ".")
        } else {
            write!(f, "[")?;
            for l in 'A'..='Z' {
                if self.contains(l) {
                    write!(f, "{}", l)?;
                }
            }
            write!(f, "]")
        }
    }
}
