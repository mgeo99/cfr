use gdk::RGBA;
use glib::Cast;
//use gdk::pango::{AttrList, Attribute};
use gtk::prelude::{ContainerExt, GridExt, GtkWindowExt, LabelExt, WidgetExt};
use gtk::{Align, EventBox, GestureExt, Grid, Inhibit, Label, StateFlags, WindowType};
use relm::{connect, timeout, Relm, Update, Widget};
use relm_derive::Msg;

use crate::cfr::state::Game;
use crate::scrabble::agent::ScrabbleAgent;
use crate::scrabble::board::Tile;
use crate::scrabble::state::{ScrabbleGame, ScrabbleState};
use crate::scrabble::util::{Letter, Position, SquareEffect};

pub struct ScrabbleUI {
    /// Current state of the game
    state: ScrabbleState,
    /// UI Board used to update after a state change
    board: Grid,
    /// Agent the user will play against
    agent: ScrabbleAgent,

    // Internal variables to the view state
    relm_window: gtk::Window,
    relm: Relm<ScrabbleUI>,
    selected_cell: Option<(i32, i32)>,
}

#[derive(Msg)]
pub enum ScrabbleMsg {
    Tick,
    Quit,
    SelectTile((f64, f64)),
}

impl ScrabbleUI {
    pub fn new(
        initial_state: ScrabbleState,
        board: Grid,
        parent: gtk::Window,
        relm: Relm<ScrabbleUI>,
    ) -> Self {
        Self {
            agent: ScrabbleAgent::new(Default::default()),
            board,
            relm_window: parent,
            state: initial_state,
            selected_cell: None,
            relm,
        }
    }

    fn get_board_label(txt: &str, color: &str, score: i32) -> String {
        format!("<span face=\"sans\" color=\"{}\">{}</span><span color=\"{0}\" face=\"sans\"><sub>{}</sub></span>", color, txt, score)
    }

    fn update_board_label(label: &Label, tile: &Tile) {
        // Update the label text
        let lbl_text = match tile {
            Tile::Letter(Letter::Letter(l)) => l.to_string(),
            _ => " ".to_string(),
        };
        let lbl_text = Self::get_board_label(&lbl_text, "black", 12);
        label.set_markup(&lbl_text);
        let tile_color = Self::compute_tile_color(tile);
        label.override_background_color(StateFlags::empty(), Some(&tile_color));
    }

    fn compute_tile_color(tile: &Tile) -> RGBA {
        match tile {
            Tile::Empty => RGBA {
                alpha: 1.0,
                red: 1.0,
                green: 1.0,
                blue: 1.0,
            },
            Tile::Letter(_) => RGBA {
                alpha: 1.0,
                red: 1.0,
                green: 1.0,
                blue: 1.0,
            },
            Tile::Special(SquareEffect::Center) => RGBA {
                alpha: 0.8,
                red: 214.0,
                green: 86.0,
                blue: 252.0,
            },
            Tile::Special(SquareEffect::DoubleWord) => RGBA {
                red: 0.94,
                green: 0.73,
                blue: 0.73,
                alpha: 1.0,
            },
            Tile::Special(SquareEffect::TripleWord) => RGBA {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            },
            Tile::Special(SquareEffect::DoubleLetter) => RGBA {
                red: 0.48,
                green: 0.79,
                blue: 0.90,
                alpha: 1.0,
            },
            Tile::Special(SquareEffect::TripleLetter) => RGBA {
                red: 0.2,
                green: 0.38,
                blue: 0.92,
                alpha: 1.0,
            },
        }
    }

    fn get_label(&self, row: i32, col: i32) -> Label {
        self.board
            .get_child_at(col, row)
            .unwrap()
            .dynamic_cast::<Label>()
            .unwrap()
    }

    fn handle_tile_selected(&mut self, row: i32, col: i32) {
        let label = self.get_label(row, col);
        let pos = Position {
            row: row as usize,
            col: col as usize,
        };
        let tile = self.state.board[pos];

        if let Some((sr, sc)) = self.selected_cell {
            let color;
            if sr == row && sc == col {
                self.selected_cell = None;
                color = Self::compute_tile_color(&tile);
            } else {
                self.selected_cell = Some((row, col));
                color = RGBA {
                    alpha: 1.0,
                    blue: 0.0,
                    red: 0.5,
                    green: 0.5,
                };
                let old_label = self.get_label(sr, sc);
                let old_pos = Position {
                    row: sr as usize,
                    col: sc as usize,
                };
                let old_tile = self.state.board[old_pos];
                let old_color = Self::compute_tile_color(&old_tile);
                old_label.override_background_color(StateFlags::empty(), Some(&old_color));
            }
            label.override_background_color(StateFlags::empty(), Some(&color));
        } else {
            let color = RGBA {
                alpha: 1.0,
                blue: 0.5,
                red: 0.5,
                green: 0.5,
            };
            label.override_background_color(StateFlags::empty(), Some(&color));
            self.selected_cell = Some((row, col));
        }
    }
}

impl Update for ScrabbleUI {
    type Model = ScrabbleState;

    type ModelParam = ScrabbleGame;

    type Msg = ScrabbleMsg;

    fn model(relm: &relm::Relm<Self>, param: Self::ModelParam) -> Self::Model {
        param.start()
    }

    fn update(&mut self, event: Self::Msg) {
        match event {
            ScrabbleMsg::Tick => {
                self.relm_window.show_all();
                timeout(self.relm.stream(), 1, || ScrabbleMsg::Tick)
            }
            ScrabbleMsg::Quit => gtk::main_quit(),
            ScrabbleMsg::SelectTile((x, y)) => {
                // Do some math to figure out the grid row/column
                let width = self.board.get_allocated_width() as f64;
                let height = self.board.get_allocated_height() as f64;
                let col = (x / width * 15.0) as i32;
                let row = (y / height * 15.0) as i32;
                self.handle_tile_selected(row, col);

                self.update(ScrabbleMsg::Tick);
            }
        }
    }
}

impl Widget for ScrabbleUI {
    type Root = gtk::Window;

    fn root(&self) -> Self::Root {
        self.relm_window.clone()
    }

    fn view(relm: &relm::Relm<Self>, initial_state: Self::Model) -> Self {
        // GTK+ widgets are used normally within a `Widget`.
        let window = gtk::Window::new(WindowType::Toplevel);

        // Connect the signal `delete_event` to send the `Quit` message.
        connect!(
            relm,
            window,
            connect_delete_event(_, _),
            return (Some(ScrabbleMsg::Quit), Inhibit(false))
        );

        // Create a grid and an event box for that grid
        let board = Grid::new();
        board.set_row_homogeneous(true);
        board.set_column_homogeneous(true);
        board.set_row_spacing(2);
        board.set_column_spacing(2);
        board.set_border_width(1);

        let evt_box = EventBox::new();
        evt_box.add(&board);
        connect!(
            relm,
            evt_box,
            connect_button_press_event(_, e),
            return (
                Some(ScrabbleMsg::SelectTile(e.get_position())),
                Inhibit(false)
            )
        );

        for i in 0..15 {
            for j in 0..15 {
                let pos = Position { row: i, col: j };
                let label = Label::new(Some(" "));
                Self::update_board_label(&label, &initial_state.board[pos]);
                board.attach(&label, j as i32, i as i32, 1, 1);
            }
        }

        let grid = Grid::new();
        grid.set_hexpand(true);
        grid.set_vexpand(true);
        grid.set_row_homogeneous(true);
        grid.set_column_homogeneous(true);
        grid.set_halign(Align::Fill);
        grid.set_valign(Align::Fill);

        // attach: left, top, width, height
        grid.attach(&evt_box, 0, 0, 23, 1);
        //grid.attach(&event_box, 0, 1, 13, 15);

        window.add(&grid);
        window.set_default_size(800, 600);
        window.show_all();
        let game = Self::new(initial_state, board, window, relm.clone());
        game
    }
}
