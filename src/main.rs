#![feature(integer_atomics)]

#[macro_use]
extern crate glium;
extern crate cam;
extern crate vecmath;
extern crate image;
extern crate num;
extern crate md5;
extern crate time;

use glium::texture::CompressedSrgbTexture2dArray;
use glium::DisplayBuild;
use std::io::Cursor;
use std::sync::{RwLock, Arc};
use world::{BlockPos, World};
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

fn main() {
    let mut bgs = BlockRegistry::new();
    let block1 = bgs.add(Block::new(DrawType::FullOpaqueBlock([BlockTextureId::new(0); 6]), LightType::Opaque));
    let world = Arc::new(RwLock::new(World::new(Arc::new(bgs), world::Generator::new(block1))));
    let (send, rec) = channel();
    let display = glium::glutin::WindowBuilder::new().with_depth_buffer(24).with_vsync().build_glium().unwrap();
    let texture = {
        let image = image::load(Cursor::new(&include_bytes!("../test.png")[..]),
                                image::PNG).unwrap().to_rgba();
        let image_dimensions = image.dimensions();
        let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions);
        CompressedSrgbTexture2dArray::new(&display, vec![image]).unwrap()
    };
    display.get_window().unwrap().set_cursor_state(glium::glutin::CursorState::Hide).unwrap();
    let quad_shader = graphics::load_quad_shader(&display).expect("cannot load quad shader");
    let line_shader = graphics::load_line_shader(&display).unwrap();
    let world2 = world.clone();
    thread::spawn(move || {
        let mut cam_pos = None;
        let mut mouse_pressed_since = [None; 2];
        let mut block_target = None;
        loop {
            loop {
                match rec.try_recv() {
                    Ok(Message::CamChanged { pos, .. }) => {
                        cam_pos = Some(pos);
                    },
                    Ok(Message::BlockTargetChanged { target }) => {
                        for p in mouse_pressed_since.iter_mut() {
                            *p = p.map(|_| SteadyTime::now());
                        }
                        block_target = target;
                    },
                    Ok(Message::MousePressed { button }) => {
                        assert!(button < 2);
                        mouse_pressed_since[button] = Some(SteadyTime::now());
                        if button == 1 {
                            if let Some(ref block_target) = block_target {
                                world.read().unwrap().set_block(&block_target.block.facing(block_target.face), block1).is_ok();
                            }
                        }
                    }
                    Ok(Message::MouseReleased { button }) => {
                        assert!(button < 2);
                        mouse_pressed_since[button] = None;
                    }
                    Err(TryRecvError::Disconnected) => return,
                    Err(TryRecvError::Empty) => break,
                }
            }
            let world_read = world.read().unwrap();
            if let Some(cam_pos) = cam_pos {
                world.read().unwrap().gen_area(&BlockPos([cam_pos[0] as i32, cam_pos[1] as i32, cam_pos[2] as i32]), 3);
            }
            if let Some(block_target) = block_target.clone() {
                if let Some(pressed_since) = mouse_pressed_since[0] {
                    if (SteadyTime::now() - pressed_since).num_milliseconds() > 500 {
                        world_read.set_block(&block_target.block, BlockId::empty()).is_ok();
                    }
                } else {
                    if let Some(pressed_since) = mouse_pressed_since[1] {
                        if (SteadyTime::now() - pressed_since).num_milliseconds() > 500 {
                            world_read.set_block(&block_target.block.facing(block_target.face), block1).is_ok();
                        }
                    }
                }
            }
            std::mem::drop(world_read);
            world.write().unwrap().flush_chunk();
            thread::sleep(std::time::Duration::from_millis(20));
        }
    });
    let mut ui = ui::Ui::new(display, quad_shader, line_shader, send, texture, world2);
    ui.run()
}