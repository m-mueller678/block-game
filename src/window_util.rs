use glium::backend::glutin_backend::GlutinFacade;

pub fn read_mouse_delta(win: &GlutinFacade, new_pos: (i32, i32)) -> Result<(i32, i32), ()> {
    let win = win.get_window().unwrap();
    if let Some(size) = win.get_inner_size_pixels() {
        let dx = new_pos.0 - size.0 as i32 / 2;
        let dy = new_pos.1 - size.1 as i32 / 2;
        win.set_cursor_position(size.0 as i32 / 2, size.1 as i32 / 2)?;
        Ok((dx, dy))
    } else {
        Err(())
    }
}