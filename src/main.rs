use std::{fs::File, io::{BufReader, BufRead}};

use ndarray_rand::rand_distr::{Distribution, WeightedIndex};
use state::{Game, GameState};
#[macro_use]
extern crate text_io;
use crate::{cfr::CFRTrainer, tictactoe::TicTacToe, scrabble::dictionary::ScrabbleDictionary};

mod cfr;
mod node;
mod state;
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

fn main() {
    let file = File::open("words.txt").unwrap();
    let reader = BufReader::new(file);
    let mut words = Vec::new();
    for line in reader.lines() {
        words.push(line.unwrap());
    }

    println!("Number of Words: {}", words.len());

    let dictionary = ScrabbleDictionary::new(words);

    loop {
        println!("Enter a Word:");
        let letters: String = read!("{}\n");
        let valid_words = dictionary.find_valid_words(letters.as_str());
        
        for word in valid_words.iter() {
            println!("\t{}", word);
        }
        println!("Found {} words:", valid_words.len());
    }
}

// 15x15x26