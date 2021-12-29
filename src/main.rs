use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::rc::Rc;

use cfr::node::StateNode;
use fst::SetBuilder;
use ndarray_rand::rand_distr::{Distribution, WeightedIndex};
use scrabble::util::{Direction, Position};
use utils::serialization;
#[macro_use]
extern crate text_io;
use crate::cfr::state::{Game, GameState};
use crate::cfr::CFRTrainer;
use crate::scrabble::bag::Bag;
use crate::scrabble::board::ScrabbleBoard;
use crate::scrabble::rack::Rack;
use crate::scrabble::state::{MoveGrid, ScrabbleGame, ScrabbleState};
use crate::scrabble::util::Letter;
use crate::tictactoe::TicTacToe;

mod cfr;
mod scrabble;
mod tictactoe;
mod utils;

fn play_tictactoe() {
    let game = TicTacToe::new(3);
    let mut trainer = CFRTrainer::new(game);
    trainer.train(1000000, 10000, 100);

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

fn read_vocabulary() -> Vec<String> {
    let file = File::open("words_small.txt").unwrap();
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

fn train_scrabble() {
    let words = read_vocabulary();

    println!("Number of Words: {}", words.len());

    let mut build = SetBuilder::memory();
    build.extend_iter(words).unwrap();
    let vocab = build.into_set();

    let game = ScrabbleGame::new(2, Rc::new(vocab));
    let mut trainer = CFRTrainer::<_, f32>::new(game);
    trainer.train(10000, 10, 1000);
}

fn play_scrabble() {
    let file = File::open("words_small.txt").unwrap();
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

    let mut build = SetBuilder::memory();
    build.extend_iter(words).unwrap();
    let vocab = build.into_set();
    let vocab = Rc::new(vocab);
    let strategy: HashMap<String, StateNode<f32>> =
        serialization::load_from_disk("./strategies/scrabble_5000.ckpt");

    loop {
        let bag = Bag::default();
        let board = ScrabbleBoard::from_file("board.json");
        println!("Enter Tray:");
        let tray: String = read!("{}\n");
        let rack = Rack::from_iter(tray.chars());
        let move_grid = MoveGrid::build(&bag, &board, vocab.as_ref(), &rack);
        let state = ScrabbleState {
            bag,
            board,
            curr_move_grid: move_grid,
            curr_player: 0,
            player_active: vec![],
            player_racks: vec![],
            player_scores: vec![],
            vocab: vocab.clone(),
        };
        let valid_moves = state.valid_actions();
        if let Some(node) = strategy.get(&state.state_key()) {
            let mut avg_strat = node.get_average_strategy();
            for i in 0..avg_strat.len() {
                if !valid_moves.contains(&i) {
                    avg_strat[i] = 0.0;
                }
            }
            let dist = WeightedIndex::new(avg_strat).unwrap();
            let mut rng = rand::thread_rng();
            let selected_action = dist.sample(&mut rng);
            let mv = state.curr_move_grid.get_move(selected_action);
            println!("{:?}", mv);
        }
    }
}

fn main() {
    //play_tictactoe();
    train_scrabble();
    //play_scrabble();
}

// 15x15x26
