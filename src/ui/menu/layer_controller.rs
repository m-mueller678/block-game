use super::*;
use super::super::ui_core::UiCore;

pub struct MenuLayerController {
    layers: Vec<Box<Menu>>,
}

impl MenuLayerController {
    pub fn new(menus: Vec<Box<Menu>>) -> Self {
        MenuLayerController{
            layers:menus,
        }
    }

    pub fn render_start(&self) -> usize {
        let mut i = self.layers.len();
        loop {
            if i == 0 { return 0; }
            i-=1;
            if !self.layers[i].transparent() {return i; }
        }
    }
}

impl Menu for MenuLayerController {
    fn transparent(&self) -> bool {
        self.layers.iter().all(|m| m.transparent())
    }

    fn process_event(&mut self, event: &WindowEvent, ui_core: &mut UiCore) -> EventResult {
        match self.layers.last_mut().expect("empty MenuLayerController received event").process_event(event, ui_core) {
            EventResult::MenuClosed => {
                self.layers.pop();
            }
            EventResult::NewMenu(m) => {
                self.layers.push(m);
            }
            EventResult::Processed => {}
        }
        if self.layers.is_empty() {
            EventResult::MenuClosed
        } else {
            EventResult::Processed
        }
    }

    fn render(&mut self, ui_core: &mut UiCore) {
        let render_start=self.render_start();
        for mut m in &mut self.layers[render_start..]{
            m.render(ui_core);
        }
    }
}