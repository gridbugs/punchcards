extern crate direction;
extern crate prototty;
extern crate prototty_common;
extern crate punchcards;
extern crate rand;

use std::fmt::Write;
use std::time::Duration;
use rand::{Rng, SeedableRng, StdRng};
use direction::CardinalDirection;
use prototty::*;
use prototty::Input as ProtottyInput;
use prototty::inputs as prototty_inputs;
use prototty_common::*;

use punchcards::{ExternalEvent as GameEvent, GameState, Input as GameInput, SaveState};
use punchcards::tile::*;

const SAVE_PERIOD_MS: u64 = 10000;
const SAVE_FILE: &'static str = "save";

const GAME_OVER_MS: u64 = 1000;
const GAME_HEIGHT: u32 = 10;
const GAME_WIDTH: u32 = 10;
const DECK_WIDTH: u32 = 8;
const DECK_HEIGHT: u32 = 1;
const GAME_PADDING_BOTTOM: u32 = 1;
const GAME_PADDING_RIGHT: u32 = 1;

const TITLE_WIDTH: u32 = 16;
const TITLE_HEIGHT: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frontend {
    Unix,
    Glutin,
    Wasm,
}

impl Frontend {
    fn supports_saving(self) -> bool {
        match self {
            Frontend::Wasm => false,
            _ => true,
        }
    }
}

const INITIAL_INPUT_BUFFER_SIZE: usize = 16;

#[derive(Debug, Clone, Copy)]
enum AppState {
    Game,
    GameOver,
    MainMenu,
}

pub enum ControlFlow {
    Quit,
}

enum InputType {
    Game(GameInput),
    ControlFlow(ControlFlow),
}

#[derive(Debug, Clone, Copy)]
enum MainMenuChoice {
    NewGame,
    Continue,
    SaveAndQuit,
    Save,
    Quit,
    ClearData,
}

struct TitleScreenView {
    title_view: RichStringView,
    main_menu_view: DefaultMenuInstanceView,
}

impl TitleScreenView {
    fn new() -> Self {
        Self {
            title_view: RichStringView::with_info(TextInfo::default().bold().underline()),
            main_menu_view: DefaultMenuInstanceView::new(),
        }
    }
}

pub struct AppView {
    title_screen_view: Decorated<TitleScreenView, Align>,
}

impl View<MenuInstance<MainMenuChoice>> for TitleScreenView {
    fn view<G: ViewGrid>(
        &mut self,
        menu: &MenuInstance<MainMenuChoice>,
        offset: Coord,
        depth: i32,
        grid: &mut G,
    ) {
        self.title_view.view("Punchcards", offset, depth, grid);
        self.main_menu_view
            .view(menu, offset + Coord::new(0, 2), depth, grid);
    }
}
impl ViewSize<MenuInstance<MainMenuChoice>> for TitleScreenView {
    fn size(&mut self, _menu: &MenuInstance<MainMenuChoice>) -> Size {
        Size::new(TITLE_WIDTH, TITLE_HEIGHT)
    }
}

impl AppView {
    pub fn new(size: Size) -> Self {
        let align = Align::new(size, Alignment::Centre, Alignment::Centre);
        Self {
            title_screen_view: Decorated::new(TitleScreenView::new(), align),
        }
    }
    pub fn set_size(&mut self, size: Size) {
        self.title_screen_view.decorator.size = size;
    }
}

pub struct App<S: Storage> {
    main_menu: MenuInstance<MainMenuChoice>,
    app_state: AppState,
    state: GameState,
    in_progress: bool,
    input_buffer: Vec<GameInput>,
    game_over_duration: Duration,
    rng: StdRng,
    storage: S,
    frontend: Frontend,
    save_remaining: Duration,
}

impl<S: Storage> View<App<S>> for AppView {
    fn view<G: ViewGrid>(&mut self, app: &App<S>, offset: Coord, depth: i32, grid: &mut G) {
        match app.app_state {
            AppState::MainMenu => {
                self.title_screen_view
                    .view(&app.main_menu, offset, depth, grid);
            }
            AppState::Game => for (coord, tile_info) in app.state.render_iter() {
                let ch = match tile_info.typ {
                    TileType::Player => '@',
                    TileType::Wall => '#',
                    TileType::Floor => '.',
                };

                if let Some(cell) = grid.get_mut(offset + coord, depth + tile_info.depth as i32) {
                    cell.set_character(ch);
                }
            },
            AppState::GameOver => {
                StringView.view(&"Game Over", offset, depth, grid);
            }
        }
    }
}

fn make_main_menu(in_progress: bool, frontend: Frontend) -> MenuInstance<MainMenuChoice> {
    let menu_items = if in_progress {
        vec![
            Some(("Continue", MainMenuChoice::Continue)),
            if frontend.supports_saving() {
                Some(("Save and Quit", MainMenuChoice::SaveAndQuit))
            } else {
                Some(("Save", MainMenuChoice::Save))
            },
            Some(("New Game", MainMenuChoice::NewGame)),
            Some(("Clear Data", MainMenuChoice::ClearData)),
        ].into_iter()
            .filter_map(|x| x)
            .collect()
    } else {
        vec![
            ("New Game", MainMenuChoice::NewGame),
            ("Quit", MainMenuChoice::Quit),
        ]
    };
    let main_menu = Menu::smallest(menu_items);
    MenuInstance::new(main_menu).unwrap()
}

impl<S: Storage> App<S> {
    pub fn new(frontend: Frontend, storage: S, seed: usize) -> Self {
        let mut rng = StdRng::from_seed(&[seed]);

        let existing_state: Option<SaveState> = storage.load(SAVE_FILE).ok();

        let (in_progress, state) = if let Some(state) = existing_state {
            (true, GameState::from_save_state(state))
        } else {
            (false, GameState::from_rng_seed(rng.gen()))
        };

        let main_menu = make_main_menu(in_progress, frontend);

        let app_state = AppState::MainMenu;
        let input_buffer = Vec::with_capacity(INITIAL_INPUT_BUFFER_SIZE);
        let game_over_duration = Duration::default();

        let save_remaining = Duration::from_millis(SAVE_PERIOD_MS);

        Self {
            main_menu,
            state,
            app_state,
            in_progress,
            input_buffer,
            game_over_duration,
            storage,
            rng,
            frontend,
            save_remaining,
        }
    }

    pub fn store(&mut self) {
        if self.in_progress {
            self.storage
                .store(SAVE_FILE, &self.state.save(self.rng.gen()))
                .expect("Failed to save");
        } else {
            match self.storage.remove_raw(SAVE_FILE) {
                Err(LoadError::IoError) => eprintln!("Failed to delete game data"),
                _ => (),
            }
        }
    }

    pub fn tick<I>(&mut self, inputs: I, period: Duration, view: &AppView) -> Option<ControlFlow>
    where
        I: IntoIterator<Item = ProtottyInput>,
    {
        if period < self.save_remaining {
            self.save_remaining -= period;
        } else {
            self.save_remaining = Duration::from_millis(SAVE_PERIOD_MS);
            self.store();
        }

        match self.app_state {
            AppState::MainMenu => {
                if let Some(menu_output) = self.main_menu
                    .tick_with_mouse(inputs, &view.title_screen_view.view.main_menu_view)
                {
                    match menu_output {
                        MenuOutput::Quit => Some(ControlFlow::Quit),
                        MenuOutput::Cancel => {
                            if self.in_progress {
                                self.app_state = AppState::Game;
                            }
                            None
                        }
                        MenuOutput::Finalise(selection) => match selection {
                            MainMenuChoice::Quit => Some(ControlFlow::Quit),
                            MainMenuChoice::Save => {
                                self.store();
                                None
                            }
                            MainMenuChoice::SaveAndQuit => {
                                self.store();
                                Some(ControlFlow::Quit)
                            }
                            MainMenuChoice::Continue => {
                                self.app_state = AppState::Game;
                                self.in_progress = true;
                                None
                            }
                            MainMenuChoice::NewGame => {
                                self.state = GameState::from_rng_seed(self.rng.gen());
                                self.app_state = AppState::Game;
                                self.in_progress = true;
                                self.main_menu = make_main_menu(true, self.frontend);
                                self.store();
                                None
                            }
                            MainMenuChoice::ClearData => {
                                self.state = GameState::from_rng_seed(self.rng.gen());
                                self.in_progress = false;
                                self.main_menu = make_main_menu(false, self.frontend);
                                self.store();
                                None
                            }
                        },
                    }
                } else {
                    None
                }
            }
            AppState::Game => {
                for input in inputs {
                    let input_type = match input {
                        prototty_inputs::ETX => InputType::ControlFlow(ControlFlow::Quit),
                        prototty_inputs::ESCAPE => {
                            self.app_state = AppState::MainMenu;
                            break;
                        }
                        _ => continue,
                    };
                    match input_type {
                        InputType::Game(input) => self.input_buffer.push(input),
                        InputType::ControlFlow(control_flow) => {
                            return Some(control_flow);
                        }
                    }
                }

                if let Some(meta) = self.state.tick(self.input_buffer.drain(..), period) {
                    match meta {
                        GameEvent::GameOver => {
                            self.app_state = AppState::GameOver;
                            self.game_over_duration = Duration::from_millis(GAME_OVER_MS);
                        }
                    }
                }

                None
            }
            AppState::GameOver => {
                if let Some(remaining) = self.game_over_duration.checked_sub(period) {
                    self.game_over_duration = remaining;
                } else {
                    self.in_progress = false;
                    self.main_menu = make_main_menu(false, self.frontend);
                    self.app_state = AppState::MainMenu;
                    self.state = GameState::from_rng_seed(self.rng.gen());
                }
                None
            }
        }
    }
}
