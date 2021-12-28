use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use fst::SetBuilder;
use ndarray_rand::rand_distr::{Distribution, WeightedIndex};
use state::{Game, GameState};
#[macro_use]
extern crate text_io;
use crate::{
    cfr::CFRTrainer,
    scrabble::{board::ScrabbleBoard, dictionary::ScrabbleDictionary, util::Letter},
    tictactoe::TicTacToe,
};

mod cfr;
mod node;
mod scrabble;
mod state;
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

fn main() {
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

    let board = ScrabbleBoard::empty();

    let result = board.calculate_moves(&[
        Letter::Letter('C'),
        Letter::Letter('A'),
        Letter::Letter('B'),
        Letter::Letter('G'),
        Letter::Letter('E'),
        Letter::Letter('A'),
        Letter::Letter('T')
    ], &vocab);

    println!("Valid Moves: {}", result.len());

    for m in result.iter().take(25) {
        println!("{:?}", m);
    }
}

// 15x15x26
