use fst::automaton::{Str, Subsequence};
use fst::{Automaton, IntoStreamer, Set};

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::BOARD_SIZE;

static SEP: u8 = ',' as u8;
static SEP_STR: &str = ",";

fn read_word_file<P: AsRef<Path>>(path: P) -> Vec<String> {
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let mut words = Vec::new();
    for line in reader.lines() {
        let word = line.unwrap().to_uppercase();
        if word.chars().count() < 2 {
            continue;
        }
        words.push(word);
    }

    words.sort_unstable();
    words
}

/// Builds the scrabble dictionary into a GADDAG. The fst crate only constructs a
/// DAG, so we need to handle the case for suffix searches as well
/// Implementation adopted from https://amedee.me/2020/11/04/fst-gaddag/
fn build_entries_sorted<P: AsRef<Path>>(vocab_path: P) -> BTreeSet<Vec<u8>> {
    let words = read_word_file(vocab_path);

    let mut entries: BTreeSet<Vec<u8>> = BTreeSet::new();
    // Only need to allocate a max of BOARD_SIZE characters
    let mut new_word: Vec<u8> = Vec::with_capacity(BOARD_SIZE);

    //We're going to re-use these instead instead of allocating for every entry
    let mut before_sep: Vec<u8> = Vec::with_capacity(BOARD_SIZE);
    let mut after_sep: Vec<u8> = Vec::with_capacity(BOARD_SIZE);

    for word in words.into_iter() {
        //Insert the reversed variant without the separator
        let whole_word_rev = word.chars().rev().collect::<String>().as_bytes().to_vec();
        entries.insert(whole_word_rev);

        // Clear our intermediate state from the last input word
        after_sep.clear();
        before_sep.clear();

        // Start with eg SERA+C
        before_sep.extend(word.as_bytes());
        after_sep.push(before_sep.pop().unwrap());

        while before_sep.len() > 0 {
            new_word.clear();
            // We store before_sep backwards and use .rev() here so we can call .pop() on it later down
            new_word.extend(before_sep.iter().rev());
            new_word.push(SEP);
            new_word.extend(after_sep.iter().rev());
            after_sep.push(before_sep.pop().unwrap());

            entries.insert(new_word.iter().cloned().collect());
        }
    }
    entries
}

/// Implementation of GADDAG for efficiently scanning scrabble words
/// Adopted from https://github.com/amedeedaboville/fst-gaddag/blob/main/src/lib.rs
pub struct Gaddag {
    dict: Set<Vec<u8>>,
}

impl Gaddag {
    pub fn build_from_file<P: AsRef<Path>>(vocab_path: P) -> Self {
        let entries = build_entries_sorted(vocab_path);
        let dict = Set::from_iter(entries).unwrap();
        Self { dict }
    }

    /// Finds all the words that end with the provided suffix
    pub fn find_suffixes(&self, text: &str) -> Vec<String> {
        let search_val: String = text.chars().rev().collect();

        let matcher = Str::new(&search_val)
            .starts_with()
            .intersection(Subsequence::new(SEP_STR).complement());

        let stream = self.dict.search(matcher).into_stream();
        stream
            .into_strs()
            .unwrap()
            .iter()
            .map(|w| Self::demangle_item(w))
            .collect()
    }

    /// Finds all the words that start with the provided prefix
    pub fn find_prefixes(&self, input: &str) -> Vec<String> {
        let search_val: String = input
            .chars()
            .rev()
            .chain(std::iter::once(SEP as char))
            .collect();
        let matcher = Str::new(&search_val).starts_with();
        self.search_fst(matcher)
    }

    /// Searches the FST with the provided automaton
    pub fn search_fst<A: Automaton>(&self, matcher: A) -> Vec<String> {
        self.dict
            .search(matcher)
            .into_stream()
            .into_strs()
            .unwrap()
            .iter()
            .map(|w| Self::demangle_item(w))
            .collect()
    }

    ///Turns the GADDAG row for a word back into that word.
    ///For example GINT+BOA will demangle to BOATING.
    fn demangle_item(item: &str) -> String {
        if let Some(idx) = item.find(SEP as char) {
            item[0..idx]
                .chars()
                .rev()
                .chain(item[(idx + 1)..].chars())
                .collect()
        } else {
            item.chars().rev().collect()
        }
    }
}
