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

use glium::DisplayBuild;
use std::sync::Arc;
use world::*;
use block::{BlockId, BlockRegistry};
use time::SteadyTime;
use std::sync::mpsc::{channel, TryRecvError};
use std::thread;
use graphics::DrawType;
use ui::Message;

mod window_util;
mod graphics;
mod block;
mod block_texture_loader;
mod world;
mod geometry;
mod ui;
mod module;

mod base_module;

fn main() {
    let mut block_registry = BlockRegistry::new();
    let mut texture_loader = block_texture_loader::TextureLoader::new();
    let mut world_gen_blocks = Vec::new();
    let mut structures= Vec::new();
    base_module::module().init(&mut texture_loader,
                               &mut block_registry,
                               &mut |block| world_gen_blocks.push(block),
                               &mut |structure| structures.push(structure)
    );
    let block_light = block_registry.by_name("debug_light").unwrap();
    let seeder = world::WorldRngSeeder::new(1);
    let generator = world::Generator::new(&seeder, world_gen_blocks, structures);
    let world = Arc::new(World::new(Arc::new(block_registry), generator));
    let (send, rec) = channel();
    let display = glium::glutin::WindowBuilder::new().with_depth_buffer(24).with_vsync().build_glium().unwrap();
    display.get_window().unwrap().set_cursor_state(glium::glutin::CursorState::Hide).unwrap();
    let shader = graphics::Shader::new(&display).unwrap();
    let w2 = world.clone();
    thread::spawn(move || {
        let mut chunk_load_guard = None;
        let mut mouse_pressed_since = [None; 2];
        let mut block_target = None;
        loop {
            chunk_load_guard.is_some();//prevent unused assignment warning as required attribute is experimental
            loop {
                match rec.try_recv() {
                    Ok(Message::CamChanged { pos, .. }) => {
                        let block_pos = BlockPos([pos[0] as i32, pos[1] as i32, pos[2] as i32]);
                        chunk_load_guard = Some(world.load_cube(&chunk_at(&block_pos), 2));
                    }
                    Ok(Message::BlockTargetChanged { target }) => {
                        for p in mouse_pressed_since.iter_mut() {
                            *p = p.map(|_| SteadyTime::now());
                        }
                        block_target = target;
                    }
                    Ok(Message::MousePressed { button }) => {
                        assert! (button < 2);
                        mouse_pressed_since[button] = Some(SteadyTime::now());
                        if button == 1 {
                            if let Some(ref block_target) = block_target {
                                world.read().set_block(&block_target.block.facing(block_target.face), block_light).is_ok();
                            }
                        }
                    }
                    Ok(Message::MouseReleased { button }) => {
                        assert! (button < 2);
                        mouse_pressed_since[button] = None;
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
            thread::sleep(std::time::Duration::from_millis(20));
        }
    });
    let texture = texture_loader.load(&display);
    let mut ui = ui::Ui::new(display, shader, send, texture, w2);
    ui.run()
}