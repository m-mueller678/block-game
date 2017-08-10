#![feature(integer_atomics)]
#![feature(conservative_impl_trait)]
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

use glium::glutin::{MouseButton, ElementState};
use num::Integer;
use world::*;
use block::BlockId;
use time::{SteadyTime,Duration};
use std::sync::mpsc::{channel, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use ui::Message;


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

mod base_module;

const TICK_TIME: f64 = 1. / 16.;

fn main() {
    let (game_data,textures) = module::start([base_module::module()].iter().map(|m| m.init()));
    let block_light = game_data.blocks().by_name("debug_light").unwrap();

    let (send, rec) = channel();
    let (display, mut events_loop) = window_util::create_window();
    display.gl_window().set_cursor_state(glium::glutin::CursorState::Hide).unwrap();
    let world = Arc::new(World::new(game_data));
    let w2 = world.clone();
    let player = Arc::new(Mutex::new(player::Player::new()));
    let p2 = player.clone();
    thread::Builder::new().name("logic".into()).spawn(move || {
        #[allow(unused_variables)]
        let mut chunk_load_guard;
        let mut chunk_pos = ChunkPos([2_000_000_000; 3]);
        let mut mouse_pressed_since = [None; 2];
        let mut block_target = None;
        let mut tick_start_time=SteadyTime::now();
        loop {
            let pos = player.lock().unwrap().position();
            let block_pos = BlockPos([
                (pos[0].floor() as i32),
                (pos[1].floor() as i32),
                (pos[2].floor() as i32),
            ]);
            let new_chunk_pos = ChunkPos([
                block_pos[0].div_floor(&(CHUNK_SIZE as i32)),
                block_pos[1].div_floor(&(CHUNK_SIZE as i32)),
                block_pos[2].div_floor(&(CHUNK_SIZE as i32)),
            ]);
            if new_chunk_pos != chunk_pos {
                chunk_pos = new_chunk_pos;
                #[allow(unused_assignments)]{
                    chunk_load_guard = world.load_cube(&chunk_at(&block_pos), 2);
                }
            }
            loop {
                match rec.try_recv() {
                    Ok(Message::BlockTargetChanged { target }) => {
                        for p in mouse_pressed_since.iter_mut() {
                            *p = p.map(|_| SteadyTime::now());
                        }
                        block_target = target;
                    }
                    Ok(Message::MouseInput { state: ElementState::Pressed, button }) => {
                        mouse_pressed_since[match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 1,
                            _ => continue,
                        }] = Some(SteadyTime::now());
                        if button == MouseButton::Right {
                            if let Some(ref block_target) = block_target {
                                world.read().set_block(&block_target.block.facing(block_target.face), block_light).is_ok();
                            }
                        }
                    }
                    Ok(Message::MouseInput { state: ElementState::Released, button }) => {
                        mouse_pressed_since[match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 1,
                            _ => continue,
                        }] = None;
                    }
                    Err(TryRecvError::Disconnected) => return,
                    Err(TryRecvError::Empty) => break,
                }
            }
            if let Some(block_target) = block_target.clone() {
                let read_guard = world.read();
                if let Some(pressed_since) = mouse_pressed_since[0] {
                    if (SteadyTime::now() - pressed_since).num_milliseconds() > 500 {
                        read_guard.set_block(&block_target.block, BlockId::empty()).is_ok();
                    }
                } else {
                    if let Some(pressed_since) = mouse_pressed_since[1] {
                        if (SteadyTime::now() - pressed_since).num_milliseconds() > 500 {
                            read_guard.set_block(&block_target.block.facing(block_target.face), block_light).is_ok();
                        }
                    }
                }
            }
            world.flush_chunk();
            player.lock().unwrap().tick(&world.read());
            let tick_end_time=SteadyTime::now();
            let real_tick_duration=tick_end_time-tick_start_time;
            let planned_tick_duration=Duration::nanoseconds((TICK_TIME*1e9) as i64);
            if real_tick_duration<planned_tick_duration{
                thread::sleep((planned_tick_duration-real_tick_duration).to_std().unwrap());
                tick_start_time=tick_start_time+planned_tick_duration;
            }else{
                tick_start_time=tick_end_time;
            }
            world.time().next_tick();
        }
    }).expect("cannot create main logic thread");
    let mut ui = ui::Ui::new(display,textures,send,w2,p2);
    ui.run(&mut events_loop);
}