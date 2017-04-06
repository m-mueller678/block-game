#![feature(integer_atomics)]

#[macro_use]
extern crate glium;
extern crate cam;
extern crate vecmath;
extern crate image;
extern crate num;
extern crate md5;

use glium::texture::CompressedSrgbTexture2dArray;
use glium::uniforms::SamplerWrapFunction;
use glium::DisplayBuild;
use glium::Surface;
use std::io::Cursor;
use std::sync::{RwLock, Arc};
use world::World;
use std::time::Duration;
use std::sync::mpsc::{Sender, channel, TryRecvError};
use std::thread;
use block::{Block, LightType};
use graphics::{WorldRender, DrawType, BlockTextureId};

mod window_util;
mod graphics;
mod block;
mod world;
mod geometry;

fn run_graphics(world: Arc<RwLock<World>>, cam_pos: Sender<[f32; 3]>) {
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

    let (mut yaw, mut pitch) = (0., 0.);
    let mut camera = cam::Camera::new([0., 100., -0.]);
    let perspective = cam::CameraPerspective { fov: 90., near_clip: 0.05, far_clip: 1000., aspect_ratio: 1.0 };
    let mut world_render = WorldRender::new(&display);
    'main_loop: loop {
        let pos = [camera.position[0] as i32, camera.position[1] as i32, camera.position[2] as i32];
        world_render.update(&pos, &world.read().unwrap());
        let mut target = display.draw();
        target.clear_color_and_depth((0., 0., 1., 1.), 1.0);
        {
            let matrix = vecmath::col_mat4_mul(
                perspective.projection(), camera.orthogonal());
            let sampler = texture.sampled().wrap_function(SamplerWrapFunction::Repeat);
            world_render.draw(&mut target, matrix, sampler, &quad_shader).unwrap();
        }
        target.finish().unwrap();
        for ev in display.poll_events() {
            match ev {
                glium::glutin::Event::Closed => break 'main_loop,
                glium::glutin::Event::MouseMoved(x, y) => {
                    if let Ok((x, y)) = window_util::read_mouse_delta(&display, (x, y)) {
                        use std::f32::consts::PI;
                        yaw -= x as f32 / 300.;
                        pitch = (pitch + y as f32 / 300.).min(0.5 * PI).max(-0.5 * PI);
                        camera.set_yaw_pitch(yaw, pitch);
                    }
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::W)) => {
                    camera.position = vecmath::vec3_sub(camera.position, camera.forward);
                    cam_pos.send(camera.position).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::S)) => {
                    camera.position = vecmath::vec3_add(camera.position, camera.forward);
                    cam_pos.send(camera.position).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::D)) => {
                    camera.position = vecmath::vec3_add(camera.position, camera.right);
                    cam_pos.send(camera.position).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::A)) => {
                    camera.position = vecmath::vec3_sub(camera.position, camera.right);
                    cam_pos.send(camera.position).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::E)) => {
                    camera.position = vecmath::vec3_add(camera.position, camera.up);
                    cam_pos.send(camera.position).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::Q)) => {
                    camera.position = vecmath::vec3_sub(camera.position, camera.up);
                    cam_pos.send(camera.position).unwrap();
                },
                _ => ()
            }
        }
    }
}

fn main() {
    let mut bgs = block::BlockRegistry::new();
    let block1 = bgs.add(Block::new(DrawType::FullOpaqueBlock([BlockTextureId::new(0); 6]), LightType::Opaque));
    let block2 = bgs.add(Block::new(DrawType::None, LightType::Source(15)));
    let world = Arc::new(RwLock::new(world::World::new(Arc::new(bgs), world::Generator::new(block1))));
    let (send, rec) = channel();
    {
        let world2 = world.clone();
        thread::spawn(|| { run_graphics(world2, send) });
    }
    loop {
        let mut cam_pos = [0.; 3];
        loop {
            match rec.try_recv() {
                Ok(pos) => cam_pos = pos,
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => break,
            }
        }
        let block_pos = [cam_pos[0] as i32, cam_pos[1] as i32, cam_pos[2] as i32];
        world.read().unwrap().gen_area(&[cam_pos[0] as i32, cam_pos[1] as i32, cam_pos[2] as i32], 3);
        world.write().unwrap().flush_chunks();
        world.read().unwrap().set_block(&block_pos, block2).unwrap();
        thread::sleep(Duration::from_millis(20));
    }
}