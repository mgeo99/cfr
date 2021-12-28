use ndarray::prelude::*;
use ndarray_stats::QuantileExt;

pub struct StateNode {
    /// Number of available actions
    num_actions: usize,
    /// Sum of all the regrets for each action
    regret_sum: Array1<f32>,
    /// Strategy to be played in this state
    strategy: Array1<f32>,
    /// Sum of all the strategy logits for this node
    strategy_sum: Array1<f32>,
}

impl StateNode {
    pub fn new(num_actions: usize) -> Self {
        Self {
            num_actions,
            regret_sum: Array1::zeros(num_actions),
            strategy: Array1::zeros(num_actions),
            strategy_sum: Array1::zeros(num_actions),
        }
    }

    pub fn compute_strategy(&mut self) -> ArrayView1<f32> {
        let mut normalizing_sum = 0.0f32;
        for i in 0..self.num_actions {
            self.strategy[i] = if self.regret_sum[i] > 0.0 {
                self.regret_sum[i]
            } else {
                0.0
            };
            normalizing_sum += self.strategy[i];
        }

        for i in 0..self.num_actions {
            if normalizing_sum > 0.0 {
                self.strategy[i] /= normalizing_sum;
            } else {
                self.strategy[i] = 1.0 / self.num_actions as f32;
            }
        }
        self.strategy.view()
    }

    pub fn update_regret_sum(&mut self, action: usize, value: f32) {
        self.regret_sum[action] = value;
    }

    pub fn get_regret_sum(&self, action: usize) -> f32 {
        self.regret_sum[action]
    }

    /// Updates the current strategy weighted by the probabilitiy of reaching
    /// this state as well as all available actions in this state
    pub fn update_strategy_sum(&mut self, prev_strategy: ArrayView1<f32>, realization_weight: f32) {
        for a in 0..self.num_actions {
            self.strategy_sum[a] += realization_weight * prev_strategy[a];
        }
    }

    /// Updates the regret sums using the reach probability of this node
    pub fn update_regrets(
        &mut self,
        reach_weight: f32,
        state_utility: f32,
        utility: Array1<f32>,
        available_actions: &[usize],
    ) {
        for &a in available_actions {
            let regret = utility[a] - state_utility;
            self.regret_sum[a] += regret * reach_weight;
        }
    }

    /// Returns the strategy score for the provided action
    pub fn get_strategy_for_action(&self, action: usize) -> f32 {
        self.strategy[action]
    }

    /// Greedily samples the most likely action given the state
    pub fn sample_action_greedy(&self) -> usize {
        let strat = self.strategy.argmax().unwrap();
        strat
    }

    /// Gets the average strategy to be played in this state
    pub fn get_average_strategy(&self) -> Array1<f32> {
        let mut avg_strategy = Array1::zeros(self.num_actions);
        let normalizing_sum = self.strategy_sum.sum();
        for i in 0..self.num_actions {
            if normalizing_sum > 0.0 {
                avg_strategy[i] = self.strategy_sum[i] / normalizing_sum;
            } else {
                // The average strategy is split uniformly across all actions in this case
                avg_strategy[i] = 1.0 / self.num_actions as f32;
            }
        }
        avg_strategy
    }
}
