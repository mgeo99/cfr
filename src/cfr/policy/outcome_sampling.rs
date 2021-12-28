use std::collections::HashMap;
use std::marker::PhantomData;

use ndarray::{Array1, ArrayView1, NdFloat};
use ndarray_rand::rand_distr::num_traits::Zero;
use ndarray_rand::rand_distr::uniform::SampleUniform;
use ndarray_rand::rand_distr::{Distribution, Uniform, WeightedIndex};
use rand::prelude::SliceRandom;

use crate::cfr::node::StateNode;
use crate::cfr::state::GameState;

const EPSILON: f32 = 0.6;
const REACH_CLIP: f32 = 1e-12;

/// Training policy that uses the outcome sampling variant of CFR
/// Implementation is based off of https://github.com/bakanaouji/cpp-cfr
/// and https://github.com/deepmind/open_spiel/blob/master/open_spiel/algorithms/outcome_sampling_mccfr.cc
pub struct OutcomeSamplingPolicy<'a, S: GameState, A> {
    /// Mutable reference to the strategies in each game state
    strategies: &'a mut HashMap<S::Key, StateNode<A>>,
    /// Number of valid actions in the entire game
    num_actions: usize,
    _a: PhantomData<A>,
}

impl<'a, S: GameState, A> OutcomeSamplingPolicy<'a, S, A>
where
    A: NdFloat + Zero + SampleUniform + Default + PartialOrd + for<'b> std::ops::AddAssign<&'b A>,
{
    pub fn new(strategies: &'a mut HashMap<S::Key, StateNode<A>>, num_actions: usize) -> Self {
        Self {
            strategies,
            num_actions,
            _a: PhantomData,
        }
    }
    pub fn update_player_strategy(&mut self, initial_state: &S, player: usize) -> A {
        let utility = self.outcome_sampling_cfr(
            initial_state,
            player,
            A::from(1.0).unwrap(),
            A::from(1.0).unwrap(),
            A::from(1.0).unwrap(),
        );
        utility
    }

    pub fn seen_states(&self) -> usize {
        self.strategies.len()
    }

    pub fn strategies(&self) -> &HashMap<S::Key, StateNode<A>> {
        &self.strategies
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
        reach_player: A,
        reach_other: A,
        reach_chance: A,
    ) -> A {
        // Upon a terminal state, just return the reward for the current player
        if curr_state.is_terminal() {
            return A::from(curr_state.get_reward(player)).unwrap();
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

        // Sample a policy and take a randomly weighted action from that policy
        let mut rng = rand::thread_rng();
        let valid_actions = curr_state.valid_actions();
        //assert!(valid_actions.len() > 0, "Must have at least 1 valid action");
        let mut action_probs =
            self.sample_policy(curr_state, player, strategy.view(), &valid_actions);
        let selected_action;
        if let Ok(dist) = WeightedIndex::new(action_probs.iter()) {
            selected_action = dist.sample(&mut rng);
        } else {
            // Weird edge case in scrabble where the probabilities diverge to 0
            selected_action = *valid_actions.choose(&mut rng).unwrap();
            action_probs[selected_action] = A::one() / A::from(self.num_actions).unwrap();
        }
        // For the sampled action, recursively call the CFR method and update weights
        let next_state = curr_state.next_state(selected_action).unwrap();
        let new_reach_player = if player == curr_state.active_player() {
            reach_player * strategy[selected_action]
        } else {
            reach_player
        };
        let new_reach_other = if player == curr_state.active_player() {
            reach_other
        } else {
            reach_other * strategy[selected_action]
        };
        let new_reach_chance = reach_chance * action_probs[selected_action];
        let child_value = self.outcome_sampling_cfr(
            &next_state,
            player,
            new_reach_player,
            new_reach_other,
            new_reach_chance,
        );

        // Estimate the value of each child action
        let mut child_values = Array1::zeros(self.num_actions);
        for &a in valid_actions.iter() {
            child_values[a] =
                self.baseline_corrected_value(a, selected_action, child_value, action_probs[a]);
        }

        // Compute the value estimate for this node
        let mut value_estimate = A::zero();
        for &a in valid_actions.iter() {
            value_estimate += strategy[a] * child_values[a];
        }

        // Update regrets and average strategy for the player
        if curr_state.active_player() == player {
            // Recompute the strategy again using cumulative regrets from all downstream nodes
            let udpdated_policy = self
                .strategies
                .get_mut(&state_key)
                .unwrap()
                .compute_strategy()
                .to_owned();
            // Compute a counterfactual value using our current value, the reach of other players
            // and the chance that this node was actually sampled
            let cf_value = value_estimate * reach_other / reach_chance;

            // Since we are already returning utilities from downstream recursive calls as they are multiplied
            // by the chance of reaching that state, we dont need to deal with the tail call probability
            for &a in valid_actions.iter() {
                let cf_action_value = child_values[a] * reach_other / reach_chance;
                let curr_regret = self.strategies.get(&state_key).unwrap().get_regret_sum(a);
                self.strategies
                    .get_mut(&state_key)
                    .unwrap()
                    .update_regret_sum(a, curr_regret + (cf_action_value - cf_value));
            }

            // Now we need to update the cumulative (average) strategy for each valid action
            for &a in valid_actions.iter() {
                let curr_sum = self.strategies.get(&state_key).unwrap().get_strategy_sum(a);
                let amount = reach_player * udpdated_policy[a] / reach_chance;
                self.strategies
                    .get_mut(&state_key)
                    .unwrap()
                    .update_strategy_sum(a, amount + curr_sum);
            }
        }
        value_estimate
    }

    /// Samples a policy depending on on the current player and state
    fn sample_policy(
        &self,
        curr_state: &S,
        player: usize,
        strategy: ArrayView1<A>,
        valid_actions: &[usize],
    ) -> Array1<A> {
        let mut action_probs = Array1::zeros(self.num_actions);
        debug_assert!(valid_actions.len() > 0, "Must have at least 1 valid action");
        let eps = A::from(EPSILON).unwrap();
        let num_acts = A::from(valid_actions.len()).unwrap();
        if curr_state.active_player() == player {
            for &i in valid_actions.iter() {
                let prob = (eps / num_acts) + (A::one() - eps) * strategy[i];
                action_probs[i] = prob;
            }
        } else {
            for &i in valid_actions.iter() {
                let prob = strategy[i];
                action_probs[i] = prob;
            }
        }
        action_probs
    }

    fn baseline_corrected_value(
        &self,
        action_idx: usize,
        sampled_idx: usize,
        value: A,
        sample_prob: A,
    ) -> A {
        // Just default this to vanilla CFR as done in Deepmind's impl
        let baseline = A::zero();
        if action_idx == sampled_idx {
            return baseline + (value - baseline) / sample_prob;
        } else {
            return baseline;
        }
    }
}
