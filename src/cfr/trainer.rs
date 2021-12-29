use std::collections::HashMap;

use ndarray::NdFloat;
use ndarray_rand::rand_distr::num_traits::Zero;
use ndarray_rand::rand_distr::uniform::SampleUniform;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::cfr::solvers::OutcomeSamplingSolver;
use crate::utils::{self, serialization};

use super::node::StateNode;
use super::state::{Game, GameState};

pub struct CFRTrainer<G, A>
where
    G: Game,
    A: NdFloat + Zero + SampleUniform + Default + PartialOrd + for<'b> std::ops::AddAssign<&'b A>,
{
    /// The game to train on
    game: G,
    /// Strategies for each player in the game
    strategies: HashMap<<G::State as GameState>::Key, StateNode<A>>,
}

impl<G, A> CFRTrainer<G, A>
where
    G: Game,
    A: NdFloat + Zero + SampleUniform + Default + PartialOrd + for<'b> std::ops::AddAssign<&'b A> + Serialize + DeserializeOwned,
    <G::State as GameState>::Key: Serialize + DeserializeOwned
{
    pub fn new(game: G) -> Self {
        Self {
            game,
            strategies: HashMap::new(),
        }
    }

    pub fn get_strategies(&self) -> &HashMap<<G::State as GameState>::Key, StateNode<A>> {
        &self.strategies
    }
    pub fn train(&mut self, rounds: usize, print_steps: usize, ckpt_steps: usize) {
        println!("Starting CFR Trainer for {} rounds", rounds);
        let mut cumulative_utility = Vec::new();
        cumulative_utility.resize(self.game.num_players(), A::zero());

        let mut policy = OutcomeSamplingSolver::<G::State, A>::new(
            &mut self.strategies,
            self.game.num_actions(),
        );

        for i in 0..rounds {
            let initial_state = self.game.start();
            for p in 0..self.game.num_players() {
                // let util = Self::vanilla_cfr(
                //     &self.game,
                //     &initial_state,
                //     p,
                //     1.0,
                //     1.0,
                //     &mut self.strategies,
                // );
                let util = policy.update_player_strategy(&initial_state, p);
                cumulative_utility[p] += util;
            }
            if (i + 1) % print_steps == 0 {
                println!("Round: {}", i + 1);
                println!("\tUtility (Cumulative): {:?}", cumulative_utility);
                println!("\tVisited States: {}", policy.seen_states());
            }

            if (i + 1) % ckpt_steps == 0 {
                println!("Saving Current Strategy");
                //let path = format!("./strategies/scrabble_{}.ckpt", i + 1);
                let path = "./strategies/scrabble.ckpt";
                serialization::save_to_disk(policy.strategies(), path)
            }
        }
        println!("CFR Training Complete");
    }

    
}
