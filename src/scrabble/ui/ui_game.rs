use crate::cfr::state::{Game, GameState};
use crate::scrabble::bag::Bag;
use crate::scrabble::board::ScrabbleBoard;
use crate::scrabble::state::{ScrabbleGame, ScrabbleState};
use crate::scrabble::util::Move;

pub struct UIGame {
    initial_game: ScrabbleGame,
    game_state: ScrabbleState,
    pub current: usize,
    turn: u32,
    pub finished: bool,
    states: Vec<(S, Move, Vec<char>, f32)>,
    pub state: usize,
}

impl UIGame {
    pub fn new(game: ScrabbleGame) -> Self {
        let game_state = game.start();
        Self {
            initial_game: game,
            game_state,
            current: 0,
            turn: 1,
            finished: false,
            states: vec![],
            state: 1,
        }
    }

    pub fn do_move(&mut self, difficulty: usize, eff: bool) -> (Move, String, String, usize) {
        let r = self.get_current_player().rack.clone();
        let m = self.players[self.current].do_move(&mut self.board, difficulty, eff);
        self.states
            .push((self.board.save_state(), Move::of(&m.0), r, 0.0f32));
        self.tick();
        m
    }

    pub fn get_bag(&self) -> &Bag {
        &self.game_state.bag
    }

    pub fn tick(&mut self) {
        self.current = (self.current + 1) % 2;
        if self.current == 0 {
            self.turn += 1;
        }
        self.state += 1;
    }


    pub fn is_over(&self) -> bool {
        self.game_state.is_terminal()
    }

    pub fn get_board(&self) -> &ScrabbleBoard {
        &self.game_state.board
    }
    // pub fn get_board_mut(&mut self) -> &mut ScrabbleBoard {
    //     &mut self.board
    // }
    pub fn get_turn(&self) -> u32 {
        self.turn
    }

    /* 
    pub fn get_current_player(&self) -> &Player {
        &self.players[self.current]
    }

    pub fn get_player(&self, n: i32) -> &Player {
        &self.players[n as usize]
    }

    pub fn get_player_mut(&mut self, n: i32) -> &mut Player {
        &mut self.players[n as usize]
    }

    pub fn set_state(&mut self, to: usize) -> (Move, Vec<char>, f32) {
        let (s, m, r, skill) = &self.states[to];

        self.board.set_state(s);
        self.state = to;
        self.current = (to - 1) % 2;

        (Move::of(m), r.clone(), *skill)
    }

    pub fn get_rack(&self, n: usize) -> Vec<char> {
        self.states[n].2.clone()
    }

    pub fn get_last_state(&self) -> S {
        if self.state == 0 {
            return (
                STATE,
                vec![],
                [array_init(|_| Vec::new()), array_init(|_| Vec::new())],
                Bag::default().distribution,
                vec![],
            );
        }

        self.states[self.state - 1].0.clone()
    }*/

    pub fn reset(&mut self) {
        self.game_state = self.initial_game.start();
        self.current = 0;
        self.turn = 1;
        self.finished = false;
        self.states = Vec::new();
        self.state = 0;
    }

    pub fn states(&self) -> usize {
        self.states.len()
    }
}
