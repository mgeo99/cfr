pub mod chance_sampling;

use std::collections::{hash_map::Entry, HashMap};

use ndarray::Array1;
use ndarray_rand::rand_distr::{Distribution, WeightedIndex};

use crate::node::StateNode;

use super::state::*;

#[derive(Debug, Clone, Copy)]
pub struct HistoryRecord(usize, usize, f32);

impl HistoryRecord {
    pub fn new(player: usize, action: usize, reach_prob: f32) -> Self {
        Self(player, action, reach_prob)
    }
    pub fn player_id(&self) -> usize {
        self.0
    }

    pub fn action_id(&self) -> usize {
        self.1
    }

    pub fn reach_prob(&self) -> f32 {
        self.2
    }
}

pub struct CFRTrainer<G>
where
    G: Game,
{
    /// The game to train on
    game: G,
    /// Strategies for each player in the game
    strategies: HashMap<<G::State as GameState>::Key, StateNode>,
}

impl<G> CFRTrainer<G>
where
    G: Game,
{
    pub fn new(game: G) -> Self {
        Self {
            game,
            strategies: HashMap::new(),
        }
    }

    pub fn get_strategies(&self) -> &HashMap<<G::State as GameState>::Key, StateNode> {
        &self.strategies
    }
    pub fn train(&mut self, rounds: usize, print_steps: usize) {
        println!("Starting CFR Trainer for {} rounds", rounds);
        let mut cumulative_utility = Vec::new();
        cumulative_utility.resize(self.game.num_players(), 0.0);

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

                let (util, _) = Self::outcome_sampling_cfr(
                    &self.game,
                    &initial_state,
                    p,
                    1.0,
                    1.0,
                    1.0,
                    &mut self.strategies,
                );
                cumulative_utility[p] += util;
            }
            if (i + 1) % print_steps == 0 {
                println!("Round: {}", i + 1);
                println!("\tUtility (Cumulative): {:?}", cumulative_utility);
            }
        }
        println!("CFR Training Complete");
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
        game: &G,
        curr_state: &G::State,
        player: usize,
        reach_player: f32,
        reach_other: f32,
        reach_chance: f32,
        strategies: &mut HashMap<<G::State as GameState>::Key, StateNode>,
    ) -> (f32, f32) {
        // Upon a terminal state, just return the reward for the current player
        if curr_state.is_terminal() {
            return (curr_state.get_reward(player) / reach_chance, 1.0);
        }

        let state_key = curr_state.state_key();
        // If necessary, create an entry for the current state node
        if !strategies.contains_key(&state_key) {
            let node = StateNode::new(game.num_actions());
            strategies.insert(curr_state.state_key(), node);
        }

        // Compute the strategy for the current node
        let strategy = strategies
            .get_mut(&state_key)
            .unwrap()
            .compute_strategy()
            .to_owned();

        // If the currently active player matches the player we want to update, then
        // sample using an epsilon-on-policy, otherwise just directly sample from the strategy
        let mut action_probs = Array1::zeros(game.num_actions());
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

        let (state_util, tail_prob) = Self::outcome_sampling_cfr(
            game,
            &next_state,
            player,
            new_reach_player,
            new_reach_other,
            reach_chance * action_probs[selected_action],
            strategies,
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
                let curr_regret_sum = strategies
                    .get(&state_key)
                    .unwrap()
                    .get_regret_sum(a);
                let regret_sum = curr_regret_sum + regret;
                strategies
                    .get_mut(&state_key)
                    .unwrap()
                    .update_regret_sum(a, regret_sum);
            }
        } else {
            // Otherwise update the average strategy
            strategies
                .get_mut(&state_key)
                .unwrap()
                .update_strategy_sum(strategy.view(), reach_other / reach_chance);
        }
        (state_util, strategy[selected_action] * tail_prob)
    }

    fn vanilla_cfr(
        game: &G,
        curr_state: &G::State,
        player: usize,
        reach_player: f32,
        reach_other: f32,
        strategies: &mut HashMap<<G::State as GameState>::Key, StateNode>,
    ) -> f32 {
        if curr_state.is_terminal() {
            return curr_state.get_reward(player);
        }

        let state_key = curr_state.state_key();
        // If necessary, create an entry for the current state node
        if !strategies.contains_key(&state_key) {
            let node = StateNode::new(game.num_actions());
            strategies.insert(curr_state.state_key(), node);
        }

        // Compute the strategy for the current node
        let strategy = strategies
            .get_mut(&state_key)
            .unwrap()
            .compute_strategy()
            .to_owned();

        let mut utility = Array1::<f32>::zeros(game.num_actions());
        let mut state_utility = 0.0f32;

        // Try all the possible actions in the given game tree
        let valid_actions = curr_state.valid_actions();
        for &a in valid_actions.iter() {
            let next_state = curr_state.next_state(a).unwrap();
            if player == curr_state.active_player() {
                utility[a] = Self::vanilla_cfr(
                    game,
                    &next_state,
                    player,
                    reach_player * strategy[a],
                    reach_other,
                    strategies,
                );
            } else {
                utility[a] = Self::vanilla_cfr(
                    game,
                    &next_state,
                    player,
                    reach_player,
                    reach_other * strategy[a],
                    strategies,
                );
            }
            state_utility += strategy[a] * utility[a];
        }

        if player == curr_state.active_player() {
            // Accumulate and compute counterfactual regret
            for &a in valid_actions.iter() {
                let regret = utility[a] - state_utility;
                let curr_regret_sum = strategies
                    .get(&state_key)
                    .unwrap()
                    .get_regret_sum(a);
                let regret_sum = curr_regret_sum + regret * reach_other;
                strategies
                    .get_mut(&state_key)
                    .unwrap()
                    .update_regret_sum(a, regret_sum);
            }

            strategies
                .get_mut(&state_key)
                .unwrap()
                .update_strategy_sum(strategy.view(), reach_player);
        }

        state_utility
    }
    /*
    fn run_cfr(
        game: &G,
        state: G::State,
        history: Vec<HistoryRecord>,
        strategies: &mut Vec<HashMap<<G::State as GameState>::Key, StateNode>>,
    ) -> f32 {
        let player = state.active_player();
        if state.is_terminal() {
            return state.get_reward(player);
        }

        let state_key = state.state_key();
        let actions = state.valid_actions();

        // If necessary, create an entry for the current state node
        if !strategies[player].contains_key(&state_key) {
            let node = StateNode::new(game.num_actions());
            strategies[player].insert(state.state_key(), node);
        }

        // Find the reachability prob by looking back at the state history. If no history entry is found,
        // then we just assume it is certain that the player will be in this state because it is likely the initial state
        let reach_prob = match history.iter().rev().find(|x| x.player_id() == player) {
            Some(x) => x.reach_prob(),
            None => 1.0,
        };

        // Update the strategy using valid actions
        strategies[player]
            .get_mut(&state_key)
            .unwrap()
            .update_strategy(reach_prob, &actions);

        // Track the "utility" of the current state as well as a total utility for the node
        let mut utility: Array1<f32> = Array1::zeros(game.num_actions());
        let mut state_utility: f32 = 0.0;

        // This is where we have the option of splitting between different variants of CFR. In vanilla CFR (this impl)
        // we iterate over all possible actions available to us and compute the cumulative regret by finishing the sub-game
        // rooted at this action
        for &a in actions.iter() {
            // Pretend that the currently active player takes this action, then recursively play out the rest of
            // the game to compute utility if we aren't in a terminal state
            let action_strat = strategies[player]
                .get(&state_key)
                .unwrap()
                .get_strategy_for_action(a);
            if let Some(next_state) = state.next_state(a) {
                let mut new_hist = history.clone();
                new_hist.push(HistoryRecord::new(player, a, reach_prob * action_strat));

                // Since we are playing against 1 or more players, we must negate the sign of the utility computed
                // on the next level of the game tree. However, here we will handle the case where the next state's active player
                // is still the current player
                let mult = if next_state.active_player() == player {
                    1.0
                } else {
                    -1.0
                };
                let next_util = Self::run_cfr(game, next_state, new_hist, strategies);
                utility[a] = mult * next_util;
                // The strategy for this action may have changed in lower levels so we need to fetch it again
                let new_strat_action = strategies[player]
                    .get(&state_key)
                    .unwrap()
                    .get_strategy_for_action(a);

                state_utility += new_strat_action * utility[a];
            }
        }

        // Now compute + accumulate the counterfactual regret for each action in this node
        // Depending on how we got to this state, we need to multiply the reach of the previous
        // player into this state. The node in the previous call to this function (one layer up) is what ended up
        // making the current state reachable thus we must account for this when updating our regret
        let prev_reach = match history.last() {
            Some(x) => x.reach_prob(),
            None => 1.0,
        };
        strategies[player]
            .get_mut(&state_key)
            .unwrap()
            .update_regrets(prev_reach, state_utility, utility, &actions);

        state_utility
    }
    */
}
