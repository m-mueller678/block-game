use glium::backend::glutin::*;
use glium::glutin::*;
use ui::UiCore;

pub fn read_mouse_delta(core: &UiCore, new_pos: (f64, f64)) -> (f64, f64) {
    let dx = new_pos.0 - f64::from(core.window_size.0) / 2.;
    let dy = new_pos.1 - f64::from(core.window_size.1) / 2.;
    core.display.gl_window().set_cursor_position(
        core.window_size.0 as i32 / 2,
        core.window_size.1 as i32 / 2,
    ).ok();
    (dx, dy)
}

pub fn create_window() -> (Display, EventsLoop) {
    let events_loop = EventsLoop::new();
    let w_builder = WindowBuilder::new();
    let c_builder = ContextBuilder::new().with_vsync(true).with_depth_buffer(24);
    (
        Display::new(w_builder, c_builder, &events_loop).expect("cannot open window"),
        events_loop,
    )
}
