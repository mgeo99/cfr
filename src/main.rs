use std::fs::File;
use std::io::{BufRead, BufReader};
use std::rc::Rc;

use fst::SetBuilder;
use ndarray_rand::rand_distr::{Distribution, WeightedIndex};
#[macro_use]
extern crate text_io;
use crate::cfr::state::{Game, GameState};
use crate::cfr::CFRTrainer;
use crate::scrabble::bag::Bag;
use crate::scrabble::board::ScrabbleBoard;
use crate::scrabble::state::ScrabbleGame;
use crate::scrabble::util::Letter;
use crate::tictactoe::TicTacToe;

mod cfr;
mod scrabble;
mod tictactoe;

fn play_tictactoe() {
    let game = TicTacToe::new(3);
    let mut trainer = CFRTrainer::new(game);
    trainer.train(1000000, 10000);

    let strat = trainer.get_strategies();
    println!("Number of Strategies: {}", strat.len());
    loop {
        println!("============ New Game ============");
        let game = TicTacToe::new(3);
        let mut state = game.start();
        while !state.is_terminal() {
            state.display_board();
            println!("Valid Actions: {:?}", state.valid_actions());
            let option: usize = read!("{}\n");
            state = state.next_state(option).unwrap();

            if !state.is_terminal() {
                let key = state.state_key();
                let node = strat.get(&key).unwrap();
                let mut avg_strat = node.get_average_strategy();
                let valid_actions = state.valid_actions();
                for i in 0..avg_strat.len() {
                    if !valid_actions.contains(&i) {
                        avg_strat[i] = 0.0;
                    }
                }
                println!("Strategy: {:?}", avg_strat.iter().collect::<Vec<_>>());
                let dist = WeightedIndex::new(avg_strat).unwrap();
                let mut rng = rand::thread_rng();
                let selected_action = dist.sample(&mut rng);

                state = state.next_state(selected_action).unwrap();
            }
        }

        let reward = state.get_reward(0);
        if reward < 0.0 {
            println!("You Lost!");
        } else if reward > 0.0 {
            println!("You Won!");
        } else {
            println!("Draw");
        }
    }
}

fn play_scrabble() {
    let file = File::open("words.txt").unwrap();
    let reader = BufReader::new(file);
    let mut words = Vec::new();
    for line in reader.lines() {
        words.push(line.unwrap());
    }

    words.sort_unstable();

    println!("Number of Words: {}", words.len());

    let mut build = SetBuilder::memory();
    build.extend_iter(words).unwrap();
    let vocab = build.into_set();

    let game = ScrabbleGame::new(2, Rc::new(vocab));
    let mut trainer = CFRTrainer::<_, f32>::new(game);
    trainer.train(1000000, 1);
}

fn main() {
    //play_tictactoe();
    play_scrabble();
}

// 15x15x26
