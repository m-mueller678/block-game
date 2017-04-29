#![feature(integer_atomics)]
#![feature(conservative_impl_trait)]
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

use glium::texture::CompressedSrgbTexture2dArray;
use glium::DisplayBuild;
use std::fs::File;
use std::sync::Arc;
use world::*;
use block::{BlockId, BlockRegistry};
use time::SteadyTime;
use std::sync::mpsc::{channel, TryRecvError};
use std::thread;
use block::{Block, LightType};
use graphics::{DrawType, BlockTextureId};
use ui::Message;

mod window_util;
mod graphics;
mod block;
mod world;
mod geometry;
mod ui;

fn load_image(path: &str) -> glium::texture::RawImage2d<u8> {
    let file = std::io::BufReader::new(File::open(path).unwrap());
    let image = image::load(file, image::PNG).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions)
}

fn main() {
    let mut bgs = BlockRegistry::new();
    let block_light = bgs.add(Block::new(DrawType::FullOpaqueBlock([BlockTextureId::new(0);6]),LightType::Source(30)));
    let block_sand = bgs.add(Block::new(DrawType::FullOpaqueBlock([BlockTextureId::new(1); 6]), LightType::Opaque));
    let block_dirt = bgs.add(Block::new(DrawType::FullOpaqueBlock([BlockTextureId::new(2); 6]), LightType::Opaque));
    let block_stone = bgs.add(Block::new(DrawType::FullOpaqueBlock([BlockTextureId::new(3); 6]), LightType::Opaque));
    let seeder = world::WorldRngSeeder::new(1);
    let generator = world::Generator::new(&seeder, vec![
        WorldGenBlock::new(
            block_dirt,
            ParameterWeight::new(0., 1., 1., 1.),
            ParameterWeight::new(0.5, 1., 0.3, 1.),
            ParameterWeight::new(0., 3., 3., 1.),
        ),
        WorldGenBlock::new(
            block_sand,
            ParameterWeight::new(0., 1., 1., 1.),
            ParameterWeight::new(0., 0.2, 0.2, 1.),
            ParameterWeight::new(0., 3., 3., 1.),
        ), WorldGenBlock::new(
            block_stone,
            ParameterWeight::new(0., 1., 1., 1.),
            ParameterWeight::new(0., 1., 1., 1.),
            ParameterWeight::new(5., std::f32::INFINITY, 2., 1.),
        )
    ]);
    let world = Arc::new(World::new(Arc::new(bgs), generator));
    let (send, rec) = channel();
    let display = glium::glutin::WindowBuilder::new().with_depth_buffer(24).with_vsync().build_glium().unwrap();
    let texture = CompressedSrgbTexture2dArray::new(&display, vec![
        load_image("textures/debug.png"),
        load_image("textures/sand.png"),
        load_image("textures/dirt.png"),
        load_image("textures/stone.png"),
    ]).unwrap();
    display.get_window().unwrap().set_cursor_state(glium::glutin::CursorState::Hide).unwrap();
    let quad_shader = graphics::load_quad_shader(&display).expect("cannot load quad shader");
    let line_shader = graphics::load_line_shader(&display).unwrap();
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
    let mut ui = ui::Ui::new(display, quad_shader, line_shader, send, texture, w2);
    ui.run()
}