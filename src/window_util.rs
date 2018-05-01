use glium::backend::glutin::*;
use glium::glutin::*;

pub fn read_mouse_delta(win: &Display, new_pos: (f64, f64)) -> Result<(f64, f64), ()> {
    let win = win.gl_window();
    if let Some(size) = win.get_inner_size() {
        let dx = new_pos.0 - f64::from(size.0) / 2.;
        let dy = new_pos.1 - f64::from(size.1) / 2.;
        win.set_cursor_position(
            size.0 as i32 / 2,
            size.1 as i32 / 2,
        )?;
        Ok((dx, dy))
    } else {
        Err(())
    }
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
