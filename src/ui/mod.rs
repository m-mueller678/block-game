use glium::glutin::*;
use glium::backend::glutin::Display;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use world::World;
use geometry::*;
use player::Player;
use module::StartComplete;
use self::game_ui::GameUi;
use self::keyboard_state::KeyboardState;
use self::ui_core::{UiState,UiCore};


mod keyboard_state;
mod game_ui;
mod ui_core;

pub enum Message {
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
    BlockTargetChanged { target: Option<ray::BlockIntersection> },
}

pub struct Ui {
    core: UiCore,
    in_game: GameUi,
}

impl Ui {
    pub fn new(
        display:Display,
        game_data:StartComplete,
        event_sender: Sender<Message>,
        world: Arc<World>,
        player: Arc<Mutex<Player>>,
    ) -> Self {
        let core=UiCore::new(display,game_data);
        Ui {
            in_game: GameUi::new(event_sender, world, player, &core),
            core: core
        }
    }

    pub fn run(&mut self, events: &mut EventsLoop) {
        loop {
            events.poll_events(|e| self.process_event(e));
            if let UiState::Closing = self.core.state {
                break;
            }
            self.in_game.work(&self.core);
        }
    }

    fn process_event(&mut self, e: Event) {
        let id = self.core.display.gl_window().id();
        match e {
            Event::WindowEvent { window_id, ref event }if window_id == id => {
                match *event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        self.core.key_state.update(&input);
                    }
                    WindowEvent::Closed => {
                        self.core.state = UiState::Closing;
                        return;
                    }
                    _ => {}
                }
                self.in_game.process_window_event(event, &self.core);
            }
            _ => {}
        }
    }
}
