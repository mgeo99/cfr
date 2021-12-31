use gdk::RGBA;
use glib::{Cast, Type};
//use gdk::pango::{AttrList, Attribute};
use gtk::prelude::{
    ContainerExt,
    GridExt,
    GtkListStoreExtManual,
    GtkWindowExt,
    LabelExt,
    WidgetExt,
};
use gtk::{
    Align,
    Button,
    ButtonExt,
    EventBox,
    GestureExt,
    Grid,
    GtkListStoreExt,
    Inhibit,
    Label,
    ListStore,
    Stack,
    StateFlags,
    StyleContextExt,
    TreeModelExt,
    TreeView,
    TreeViewColumn,
    TreeViewExt,
    WindowType,
};
use relm::{connect, timeout, Relm, Update, Widget};
use relm_derive::Msg;

use crate::cfr::state::{Game, GameState};
use crate::scrabble::agent::ScrabbleAgent;
use crate::scrabble::board::Tile;
use crate::scrabble::state::{ScrabbleGame, ScrabbleState};
use crate::scrabble::ui::util;
use crate::scrabble::util::{Letter, Move, Position, SquareEffect};

const GREY: RGBA = RGBA {
    red: 0.38,
    green: 0.38,
    blue: 0.38,
    alpha: 1.0,
};

pub struct ScrabbleUI {
    /// Current state of the game
    state: ScrabbleState,

    // UI Components
    board: Grid,
    rack: Grid,

    move_options: TreeView,
    move_store: ListStore,
    move_data: Vec<Move>,
    selected_move: Option<usize>,

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
    SelectMove(u32),
    GenerateMoves,
}

impl ScrabbleUI {
    pub fn new(
        initial_state: ScrabbleState,
        board: Grid,
        rack: Grid,
        parent: gtk::Window,
        relm: Relm<ScrabbleUI>,
        move_options: TreeView,
        move_store: ListStore,
    ) -> Self {
        Self {
            agent: ScrabbleAgent::new(Default::default()),//ScrabbleAgent::from_file("./strategies/scrabble.ckpt"),
            board,
            relm_window: parent,
            state: initial_state,
            selected_cell: None,
            relm,
            rack,
            move_options,
            move_store,
            move_data: Vec::new(),
            selected_move: None,
        }
    }

    fn get_board_label(txt: &str, color: &str, score: i32) -> String {
        format!("<span face=\"sans\" color=\"{}\">{}</span><span color=\"{0}\" face=\"sans\"><sub>{}</sub></span>", color, txt, score)
    }

    fn update_board_label(label: &Label, tile: &Tile, score: i32) {
        // Update the label text
        let lbl_text = match tile {
            Tile::Letter(Letter::Letter(l)) => l.to_string(),
            _ => " ".to_string(),
        };
        let lbl_text = Self::get_board_label(&lbl_text, "black", score);
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
                alpha: 0.6,
                red: 0.5,
                green: 0.5,
                blue: 0.5,
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

    fn get_rack_label(&self, i: i32) -> Label {
        self.rack
            .get_child_at(i, 0)
            .unwrap()
            .dynamic_cast::<Label>()
            .unwrap()
    }

    fn render_player_rack(&self) {
        // Clear the current rack
        for i in 0..7 {
            let l = self.get_rack_label(i);
            l.set_text(" ");
        }

        // Get the new rack
        for (i, letter) in self.state.player_racks[0]
            .get_letters()
            .into_iter()
            .enumerate()
        {
            let label = self.get_rack_label(i as i32);
            match letter {
                Letter::Blank => label.set_text("?"),
                Letter::Letter(l) => label.set_text(&l.to_string()),
            }
        }
    }

    fn render_scrabble_board(&self) {
        for i in 0..15 {
            for j in 0..15 {
                let pos = Position { row: i, col: j };
                let label = self.get_label(i as i32, j as i32);
                let score = if let Tile::Letter(l) = self.state.board[pos] {
                    self.state.bag.score(l)
                } else {
                    0
                };
                Self::update_board_label(&label, &self.state.board[pos], score);
            }
        }
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

    fn handle_generate_moves(&mut self) {
        self.move_store.clear();
        self.move_data.clear();
        let mut available_moves = self.state.curr_move_grid.moves().to_vec();
        available_moves.sort_unstable_by_key(|x| std::cmp::Reverse(x.score));

        for (pos, m) in available_moves.into_iter().enumerate() {
            let pos_str = format!("({}, {})", m.pos.row, m.pos.col);
            let dir_str = match m.dir {
                crate::scrabble::util::Direction::Across => "across",
                _ => "down",
            };
            self.move_store.insert_with_values(
                Some(pos as u32),
                &[0, 1, 2, 3, 4],
                &[&(pos as u32), &pos_str, &dir_str, &m.word, &m.score],
            );
            self.move_data.push(m);
        }
    }

    fn handle_move_selected(&mut self, move_id: u32) {
        // Run the player's move
        let target_move = &self.move_data[move_id as usize];

        let next_state = self.state.next_state_with_move(Some(target_move));

        // Now run the AI's move
        println!("Getting agent move");
        let ai_move = self.agent.get_action(&next_state);
        let next_state = next_state.next_state(ai_move).unwrap();
        self.state = next_state;

        self.render_scrabble_board();

        println!("Scores: {:?}", self.state.player_scores);
    }
}

impl Update for ScrabbleUI {
    type Model = ScrabbleState;

    type ModelParam = ScrabbleGame;

    type Msg = ScrabbleMsg;

    fn model(_: &relm::Relm<Self>, game: Self::ModelParam) -> Self::Model {
        game.start()
    }

    fn update(&mut self, event: Self::Msg) {
        match event {
            ScrabbleMsg::Tick => {
                self.render_player_rack();

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
            ScrabbleMsg::SelectMove(move_id) =>  {
                self.handle_move_selected(move_id);
                self.state.board.print_board();
            },
            ScrabbleMsg::GenerateMoves => {
                self.handle_generate_moves();
                timeout(self.relm.stream(), 1, || ScrabbleMsg::Tick)
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
        for i in 0..15 {
            for j in 0..15 {
                let pos = Position { row: i, col: j };
                let label = Label::new(Some(" "));
                Self::update_board_label(&label, &initial_state.board[pos], 0);
                board.attach(&label, j as i32, i as i32, 1, 1);
            }
        }

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

        // Create the rack to show for the player
        let rack = Grid::new();
        rack.set_hexpand(true); // todo make fn to generate grid
        rack.set_vexpand(true);
        rack.set_row_homogeneous(true);
        rack.set_column_homogeneous(true);
        rack.set_halign(Align::Fill);
        rack.set_border_width(5);
        for i in 0..7 {
            let l = Label::new(Some(" "));
            l.override_background_color(StateFlags::empty(), Some(&RGBA::white()));
            rack.attach(&l, i, 0, 1, 1);
        }

        // Create the space for a moves list
        let tree_model = ListStore::new(&[
            Type::U32,    // Index
            Type::String, // Pos
            Type::String, // Dir
            Type::String, // Move
            Type::U8,     // Score
        ]);

        let options = TreeView::with_model(&tree_model);
        options.get_style_context().add_class("monospace");
        connect!(relm, options, connect_row_activated(tree, path, _col), {
            let model = tree.get_model().unwrap();
            let iter = model.get_iter(path).unwrap();
            let move_id = model.get_value(&iter, 0).get::<u32>().unwrap();
            ScrabbleMsg::SelectMove(move_id.unwrap())
        });

        let options_container = util::create_scroll_window(&options);

        let mut columns: Vec<TreeViewColumn> = Vec::new();
        util::append_column("#", &mut columns, &options, None);
        util::append_column("Position", &mut columns, &options, None);
        util::append_column("Direction", &mut columns, &options, None);
        util::append_column("Word", &mut columns, &options, None);
        util::append_column("Score", &mut columns, &options, None);

        /*connect!(
            relm,
            options,
            connect_cursor_changed(_),
            ScrabbleMsg::SelectMove
        );*/

        // Set up button box
        let button_box = Grid::new();
        button_box.set_hexpand(true); // todo make fn to generate grid
        button_box.set_vexpand(true);
        button_box.set_row_homogeneous(true);
        button_box.set_column_homogeneous(true);

        let choices_btn = Button::new();
        choices_btn.add(&Label::new(Some("Generate Choices")));
        connect!(
            relm,
            choices_btn,
            connect_clicked(_),
            ScrabbleMsg::GenerateMoves
        );
        button_box.attach(&choices_btn, 0, 0, 1, 1);

        // Final layout grid
        let grid = Grid::new();
        grid.set_hexpand(true);
        grid.set_vexpand(true);
        grid.set_row_homogeneous(true);
        grid.set_column_homogeneous(true);
        grid.set_halign(Align::Fill);
        grid.set_valign(Align::Fill);

        grid.attach(&evt_box, 0, 0, 23, 16);
        grid.attach(&rack, 25, 0, 7, 1);
        grid.attach(&options_container, 25, 2, 15, 14);
        grid.attach(&button_box, 25, 1, 15, 1);

        window.add(&grid);
        window.set_default_size(1280, 600);
        window.show_all();
        let game = Self::new(
            initial_state,
            board,
            rack,
            window,
            relm.clone(),
            options,
            tree_model,
        );
        game
    }
}
