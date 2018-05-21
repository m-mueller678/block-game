use glium::glutin::*;
use glium::backend::glutin::Display;
use logging::PerformanceMonitor;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use world::World;
use geometry::*;
use player::Player;
use block_texture_loader::TextureLoader;
use graphics::ChunkUpdateReceiver;
use self::game_ui::GameUi;
use self::keyboard_state::KeyboardState;
use self::menu::{Menu, MenuLayerController, EventResult};
use self::player_controller::PlayerController;

pub use self::ui_core::UiCore;
pub use self::position_interpolator::{PositionUpdateSender, PositionInterpolator, new as new_position_channel};

mod player_controller;
mod keyboard_state;
mod game_ui;
mod ui_core;
mod menu;
mod position_interpolator;

pub enum UiState {
    Swapped,
    Closing,
    InGame,
    Menu(Box<Menu>),
}

pub enum Message {
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
    BlockTargetChanged { target: Option<ray::BlockIntersection> },
}

pub struct Ui {
    state: UiState,
    core: UiCore,
    in_game: GameUi,
    perf: PerformanceMonitor,
}

impl Ui {
    pub fn new(
        display: Display,
        textures: TextureLoader,
        event_sender: Sender<Message>,
        world: Arc<World>,
        chunk_update_receiver: ChunkUpdateReceiver,
        player: Arc<Player>,
        player_pos: PositionInterpolator,
    ) -> Self {
        let mut core = UiCore::new(display, textures);
        core.disable_cursor();
        Ui {
            in_game: GameUi::new(
                event_sender,
                world,
                chunk_update_receiver,
                PlayerController::new(player, player_pos),
                &core,
            ),
            core: core,
            state: UiState::InGame,
            perf: PerformanceMonitor::new("ui loop".into(), 40000000, 16700000, &[
                ("events", 3000000, 500000),
                ("decide draw game", 100000, 50000),
                ("game update", 10000000, 1000000),
                ("frame creation", 40000000, 40000000),
                ("game render", 3000000, 1000000),
                ("menu render", 1000000, 500000),
                ("frame finish", 40000000, 40000000),
            ]),
        }
    }

    pub fn run(&mut self, events: &mut EventsLoop) {
        use glium::Surface;
        loop {
            self.perf.start_run();
            events.poll_events(|e| self.process_event(e));
            self.core.update();
            self.perf.action_complete();
            let draw_game = match self.state {
                UiState::Closing => {
                    break;
                }
                UiState::InGame => true,
                UiState::Menu(ref m) => m.transparent(),
                UiState::Swapped => unreachable!(),
            };
            self.perf.action_complete();
            if draw_game {
                self.in_game.update(&self.core, &self.state);
            }
            self.perf.action_complete();
            let mut target = self.core.display.draw();
            target.clear_color_and_depth((0.5, 0.5, 0.5, 1.), 1.0);
            self.perf.action_complete();
            if draw_game {
                self.in_game.render(
                    &self.core,
                    &mut target,
                );
            }
            self.perf.action_complete();
            match self.state {
                UiState::Closing | UiState::Swapped => unreachable!(),
                UiState::InGame => {}
                UiState::Menu(ref mut menu) => {
                    menu.render(&self.core, &mut target);
                }
            }
            self.perf.action_complete();
            target.finish().unwrap();
            self.perf.action_complete();
            self.perf.end_run();
        }
    }

    fn process_event(&mut self, e: Event) {
        if let UiState::Closing = self.state {
            return;
        }
        let id = self.core.display.gl_window().id();
        match e {
            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == id => {
                match *event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        self.core.key_state.update(&input);
                    }
                    WindowEvent::Focused(b) => {
                        if b {
                            if let UiState::InGame = self.state {
                                self.core.disable_cursor();
                            }
                        } else {
                            self.core.enable_cursor();
                        }
                    }
                    WindowEvent::Resized(x, y) => {
                        self.core.window_size = (x, y);
                    }
                    WindowEvent::CursorMoved { position: (x, y), .. } => {
                        let size = self.core.window_size;
                        self.core.mouse_position =
                            [x as f32 / size.0 as f32, y as f32 / size.1 as f32];
                    }
                    WindowEvent::Closed => {
                        self.state = UiState::Closing;
                        return;
                    }
                    _ => {}
                }
                use std::mem::replace;
                self.state = match replace(&mut self.state, UiState::Swapped) {
                    UiState::Menu(mut m) => {
                        match m.process_event(event, &mut self.core) {
                            EventResult::Processed => UiState::Menu(m),
                            EventResult::MenuClosed => {
                                self.core.disable_cursor();
                                UiState::InGame
                            }
                            EventResult::NewMenu(pushed) => {
                                eprintln!("ui received EventResult::NewMenu");
                                UiState::Menu(Box::new(MenuLayerController::new(vec![m, pushed])))
                            }
                        }
                    }
                    UiState::InGame => {
                        let mut new_state = UiState::InGame;
                        self.in_game.process_window_event(
                            event,
                            &mut self.core,
                            &mut new_state,
                        );
                        if let UiState::Menu(_) = new_state {
                            self.core.enable_cursor()
                        }
                        new_state
                    }
                    UiState::Closing | UiState::Swapped => unreachable!(),
                };
            }
            _ => {}
        }
    }
}
