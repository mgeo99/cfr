use std::collections::HashMap;
use std::path::Path;

use ndarray_rand::rand_distr::{Distribution, WeightedIndex};

use crate::cfr::node::StateNode;
use crate::cfr::state::GameState;
use crate::utils::serialization;

use super::state::ScrabbleState;

pub struct ScrabbleAgent {
    strategies: HashMap<String, StateNode<f32>>,
}

impl ScrabbleAgent {
    pub fn new(strategies: HashMap<String, StateNode<f32>>) -> Self {
        Self { strategies }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
        println!("Loading Agent Strategy");
        let strategies = serialization::load_from_disk(path);
        println!("Strategy Loaded");
        Self::new(strategies)
    }

    pub fn get_action(&self, state: &ScrabbleState) -> usize {
        let state_key = state.state_key();
        let valid_moves = state.valid_actions();
        if let Some(node) = self.strategies.get(&state_key) {
            let mut avg_strat = node.get_average_strategy();
            for i in 0..avg_strat.len() {
                if !valid_moves.contains(&i) {
                    avg_strat[i] = 0.0;
                }
            }
            let dist = WeightedIndex::new(avg_strat.view()).unwrap();
            let mut rng = rand::thread_rng();
            let selected_action = dist.sample(&mut rng);
            println!(
                "Selected Action: {} with probability {}",
                selected_action, avg_strat[selected_action]
            );
            return selected_action;
        }
        println!("Agent has not seen this state before. Deferring to highest scoring move");
        // Return the default action if we dont have any stored strategy node
        0
    }
}
