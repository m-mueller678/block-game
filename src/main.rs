#![feature(integer_atomics)]
#![feature(range_contains)]
#![feature(test)]
extern crate test;

#[macro_use]
extern crate glium;
extern crate cam;
extern crate vecmath;
extern crate image;
extern crate num;
extern crate time;
extern crate rand;
extern crate noise;
extern crate threadpool;
extern crate chashmap;
extern crate glium_text_rusttype;
extern crate font_loader;
extern crate owning_ref;
#[macro_use]
extern crate slog;
extern crate slog_term;
#[macro_use]
extern crate lazy_static;
extern crate rayon;

#[macro_use]
mod logging;
mod window_util;
mod graphics;
mod block;
mod block_texture_loader;
mod world;
mod geometry;
mod ui;
mod module;
mod physics;
mod player;
mod item;
mod debug;

use world::*;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;

mod base_module;

fn main() {
    let (game_data, textures) = module::start([base_module::module()].iter().map(|m| m.init()));
    let (send, rec) = channel();
    let (chunk_send, chunk_rec) = graphics::chunk_update_channel();
    let (display, mut events_loop) = window_util::create_window();
    let world = Arc::new(World::new(game_data, chunk_send));
    let w2 = Arc::clone(&world);
    let (player_pos_rec, player_pos_send) = ui::new_position_channel();
    let player = Arc::new(player::Player::new(player_pos_send, &world, rec));
    let p2 = Arc::clone(&player);
    world.on_tick(Box::new(move |w, t| {
        player.tick(t, w);
        TickFunctionResult::Keep
    }));
    thread::Builder::new()
        .name("logic".into())
        .spawn(move || {
            loop {
                world.flush_chunk();
                world.run_tick();
                world.time().next_tick();
            }
        })
        .expect("cannot create main logic thread");
    let mut ui = ui::Ui::new(display, textures, send, w2, chunk_rec, p2, player_pos_rec);
    ui.run(&mut events_loop);
}
