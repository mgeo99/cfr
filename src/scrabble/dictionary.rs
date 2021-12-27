use std::collections::HashMap;

/// Prefix tree node
#[derive(Debug)]
pub struct TrieNode {
    letter: char,
    next: HashMap<char, TrieNode>,
    terminal: bool,
}

impl TrieNode {
    /// Adds a word to the trie node
    pub fn add_word(&mut self, word: &str) {
        if word.is_empty() {
            return;
        }
        self.add_word_internal(word, 0);
    }

    pub fn next_node(&self, c: char) -> Option<&TrieNode> {
        self.next.get(&c)
    }

    fn add_word_internal(&mut self, word: &str, pos: usize) {
        let terminal = pos == word.chars().count() - 1;
        let curr_char = word.chars().nth(pos).unwrap();
        let node = self.next.entry(curr_char).or_insert(TrieNode {
            letter: curr_char,
            next: HashMap::new(),
            terminal,
        });

        if !terminal {
            node.add_word_internal(word, pos + 1);
        }
    }
}

pub struct ScrabbleDictionary {
    // Original word list
    words: Vec<String>,
    // Words where each letter is in sorted order
    words_cannon: Vec<Vec<char>>,
}

impl ScrabbleDictionary {
    pub fn new(words: Vec<String>) -> Self {
        // Cannonicalize all words up front since we will be hammering
        // this dictionary later with queries
        let words_cannon = words
            .iter()
            .map(|x| {
                let mut chars = x.chars().collect::<Vec<_>>();
                chars.sort_unstable();
                chars
            })
            .collect();
        Self {
            words,
            words_cannon,
        }
    }
    /// Algorithm to find valid scrabble words using the dictionary of words.
    /// Blank tiles that can be used as any word should be marked with an "_"
    pub fn find_valid_words(&self, letters: &str) -> Vec<&str> {
        let mut valid_words = Vec::new();
        // Count the number of free spaces we have that we can use for anything
        let free_count = letters.chars().filter(|x| *x == '_').count();

        // Cannonicalize the letters
        let mut letters = letters.replace("_", "").chars().collect::<Vec<_>>();
        letters.sort_unstable();

        // O(L * W * w). See if we can make this faster using a trie or maybe that aho-corasick algorithm
        for i in 0..self.words_cannon.len() {
            let lcs = self.longest_common_subsequence(&letters, &self.words_cannon[i]);

            if lcs + free_count >= self.words[i].len() {
                valid_words.push(self.words[i].as_str());
            }
        }
        valid_words
    }
    /// Given a list of sorted letters and a word (also sorted), determines
    /// the number of blank tiles that must be used to convert letters into another word
    fn longest_common_subsequence(&self, letters: &[char], word: &[char]) -> usize {
        let mut lcs_table = vec![vec![0; word.len() + 1]; letters.len() + 1];
        for i in 0..=letters.len() {
            for j in 0..=word.len() {
                if i == 0 || j == 0 {
                    lcs_table[i][j] = 0;
                    continue;
                }
                if letters[i - 1] == word[j - 1] {
                    lcs_table[i][j] = lcs_table[i - 1][j - 1] + 1;
                }
                else {
                    lcs_table[i][j] = std::cmp::max(lcs_table[i - 1][j], lcs_table[i][j - 1]);
                }
            }
        }

        lcs_table[letters.len()][word.len()]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{ScrabbleDictionary, TrieNode};
    #[test]
    fn test_dictionary_lookup() {
        let vocab = vec![
            "cat".into(),
            "dog".into(),
            "mouse".into(),
            "moose".into(),
            "laptop".into()
        ];
        let dict = ScrabbleDictionary::new(vocab);

        let valid = dict.find_valid_words("mo_se");
        println!("{:?}", valid);
        assert_eq!(valid.len(), 2);
    }

    #[test]
    fn test_trie() {
        let vocab: Vec<String> = vec![
            "cat".into(),
            "dog".into(),
            "mouse".into(),
            "moose".into(),
            "laptop".into()
        ];

        let mut root = TrieNode {
            letter: ' ',
            next: HashMap::new(),
            terminal: false
        };

        for w in vocab.iter() {
            root.add_word(w);
        }

    }
}

// DIMMAYBELOFTYOXHEYFRETFEETBENTOOTILE 