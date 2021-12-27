use std::hash::Hash;

pub trait GameState: Sized {
    /// Key associated with the game state
    type Key: Hash + Eq;
    /// Gets the active player for the given game state
    fn active_player(&self) -> usize;
    /// Fetches all the available actions in the current game state
    fn valid_actions(&self) -> Vec<usize>;
    /// Gets the key associated with the game state
    fn state_key(&self) -> Self::Key;
    /// Returns the next state of the game by applying the provided action for the currently active player
    /// If we are in the terminal state then this returns None
    fn next_state(&self, action: usize) -> Option<Self>;
    /// Checks if the current state is terminal
    fn is_terminal(&self) -> bool;
    /// Gets the payout for the player at this state
    fn get_reward(&self, player: usize) -> f32;
}

pub trait Game {
    /// Associated state type for the game
    type State: GameState;
    /// Returns the number of players in the game state
    fn num_players(&self) -> usize;
    /// Returns the number of actions possible in any state in the game. This does not mean that
    /// all actions are valid, just that the game supports a finite number of actions
    fn num_actions(&self) -> usize;
    /// Starts the game and retrieves the initial state
    fn start(&self) -> Self::State;
    /// Resets the game to an initial state and clears all scores/actions of each player
    fn reset(&mut self);
}
