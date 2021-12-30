pub mod bag;
pub mod board;
mod constraints;
mod letter_set;
pub mod rack;
pub mod state;
pub mod util;
mod word_search;
mod ui;
mod agent;
const BOARD_SIZE: usize = 15;


pub use self::ui::ScrabbleUI;