use std::collections::HashMap;

use ndarray::Array1;
use ndarray_rand::rand_distr::{Distribution, WeightedIndex};

use crate::cfr::node::StateNode;
use crate::cfr::state::GameState;

/// Training policy that uses the outcome sampling variant of CFR
/// Implementation is based off of https://github.com/bakanaouji/cpp-cfr
pub struct OutcomeSamplingPolicy<'a, S: GameState> {
    /// Mutable reference to the strategies in each game state
    strategies: &'a mut HashMap<S::Key, StateNode>,
    /// Number of valid actions in the entire game
    num_actions: usize,
}

impl<'a, S: GameState> OutcomeSamplingPolicy<'a, S> {
    pub fn new(strategies: &'a mut HashMap<S::Key, StateNode>, num_actions: usize) -> Self {
        Self {
            strategies,
            num_actions,
        }
    }
    pub fn update_player_strategy(&mut self, initial_state: &S, player: usize) -> f32 {
        let (utility, _) = self.outcome_sampling_cfr(initial_state, player, 1.0, 1.0, 1.0);
        utility
    }

    /// Chance Sampling Monte-Carlo CFR
    /// Params:
    ///     game: Reference to the current game object
    ///     state: Current game state
    ///     player: The index of the player to update a strategy for
    ///     reach_player: The probability of reaching the current state if the player always selected actions leading to this node
    ///     reach_other: The proabbility of reaching the current state if all other players except our target player selected actions leading to this node
    ///     reach_chance: Probability of reaching state if both other players and chance nodes choses actions leading to the terminal node
    /// Returns the expected payoff of the current player and the probability of actually reaching this node due to other upstream chance nodes
    fn outcome_sampling_cfr(
        &mut self,
        curr_state: &S,
        player: usize,
        reach_player: f32,
        reach_other: f32,
        reach_chance: f32,
    ) -> (f32, f32) {
        // Upon a terminal state, just return the reward for the current player
        if curr_state.is_terminal() {
            let reward = curr_state.get_reward(player);
            println!("Terminal Reward: {}", reward);
            return (reward / reach_chance, 1.0);
        }

        let state_key = curr_state.state_key();
        // If necessary, create an entry for the current state node
        if !self.strategies.contains_key(&state_key) {
            let node = StateNode::new(self.num_actions);
            self.strategies.insert(curr_state.state_key(), node);
        }

        // Compute the strategy for the current node
        let strategy = self
            .strategies
            .get_mut(&state_key)
            .unwrap()
            .compute_strategy()
            .to_owned();

        // If the currently active player matches the player we want to update, then
        // sample using an epsilon-on-policy, otherwise just directly sample from the strategy
        let mut action_probs = Array1::zeros(self.num_actions);
        let eps = 0.6f32;
        let valid_actions = curr_state.valid_actions();
        if curr_state.active_player() == player {
            for &i in valid_actions.iter() {
                action_probs[i] = (eps / valid_actions.len() as f32) + (1.0 - eps) * strategy[i];
            }
        } else {
            for &i in valid_actions.iter() {
                action_probs[i] = strategy[i];
            }
        }

        let dist = WeightedIndex::new(action_probs.iter()).unwrap();
        let mut rng = rand::thread_rng();
        let selected_action = dist.sample(&mut rng);

        // For the sampled action, recursively call the CFR method and update weights
        let next_state = curr_state.next_state(selected_action).unwrap();
        let new_reach_player = if player == curr_state.active_player() {
            reach_player * strategy[selected_action]
        } else {
            1.0
        };
        let new_reach_other = if player == curr_state.active_player() {
            1.0
        } else {
            reach_other * strategy[selected_action]
        };

        let (state_util, tail_prob) = self.outcome_sampling_cfr(
            &next_state,
            player,
            new_reach_player,
            new_reach_other,
            reach_chance * action_probs[selected_action],
        );

        if curr_state.active_player() == player {
            // Accumulate and compute counterfactual regret
            let weight = state_util * reach_other;
            for &a in valid_actions.iter() {
                let regret = if a == selected_action {
                    weight * (1.0 - strategy[selected_action]) * tail_prob
                } else {
                    -weight * tail_prob * strategy[selected_action]
                };
                let curr_regret_sum = self.strategies.get(&state_key).unwrap().get_regret_sum(a);
                let regret_sum = curr_regret_sum + regret;
                self.strategies
                    .get_mut(&state_key)
                    .unwrap()
                    .update_regret_sum(a, regret_sum);
            }
        } else {
            // Otherwise update the average strategy
            self.strategies
                .get_mut(&state_key)
                .unwrap()
                .update_strategy_sum(strategy.view(), reach_other / reach_chance);
        }
        (state_util, strategy[selected_action] * tail_prob)
    }
}
