use relm_derive::Msg;

use crate::scrabble::util::{Direction, Position};

#[derive(Msg, Debug)]
pub enum Msg {
    Tick,
    Quit,
    Click((f64, f64)),
    Type(u32),
    SetMove(usize),
    GenChoices,
    NewGame,
    ItemSelect,
}

pub struct ClickData {
    pub start_pos: Position,
    direction: Direction,
    curr_pos: Position,
    word: Vec<char>,
    _is_typing: bool,
}

impl ClickData {
    fn new() -> ClickData {
        ClickData {
            start_pos: Position { row: 0, col: 0 },
            direction: Direction::Across,
            curr_pos: Position { row: 0, col: 0 },
            word: vec![],
            _is_typing: false,
        }
    }

    pub fn is_typing(&self) -> bool {
        self._is_typing
    }

    pub fn start(&mut self, at: Position) {
        self.start_pos = at;
        self.direction = Direction::Across;
        self.curr_pos = at;
        self.word = vec![];
        self._is_typing = true;
    }

    pub fn dir_str(&self) -> String {
        match self.direction {
            Direction::Across => "Across".into(),
            Direction::Down => "Down".into(),
        }
    }

    pub fn is_at(&self, at: Position) -> bool {
        at == self.curr_pos
    }

    pub fn flip(&mut self) {
        self.direction = self.direction.flip()
    }

    pub fn tick(&mut self) -> bool {
        if let Some(next) = self.curr_pos.next(self.direction) {
            self.curr_pos = next;
            return true;
        }
        false
    }

    pub fn push(&mut self, c: char) {
        self.word.push(c);
    }
}
