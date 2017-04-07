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
use graphics::{WorldRender, DrawType, BlockTextureId, LineVertex};

mod window_util;
mod graphics;
mod block;
mod world;
mod geometry;

enum Message {
    CamChanged { pos: [f32; 3], direction: [f32; 3] }
}

fn run_graphics(world: Arc<RwLock<World>>, cam_pos: Sender<Message>) {
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
    let view_line_indices = glium::IndexBuffer::<u32>::new(&display, glium::index::PrimitiveType::LinesList, &[0, 1, 2, 3, 4, 5]).unwrap();
    let view_line_vertices = glium::VertexBuffer::new(&display, &[
        LineVertex { pos: [0., 0., 0.], color: [1., 1., 0.] },
        LineVertex { pos: [0., 0., 0.], color: [1., 1., 0.] },
        LineVertex { pos: [0., 0., 0.], color: [1., 1., 0.] },
        LineVertex { pos: [0., 0., 0.], color: [1., 1., 0.] },
        LineVertex { pos: [0., 0., 0.], color: [1., 1., 0.] },
        LineVertex { pos: [0., 0., 0.], color: [1., 1., 0.] },
    ]).unwrap();

    let (mut yaw, mut pitch) = (0., 0.);
    let mut camera = cam::Camera::new([0., 100., -0.]);
    let mut world_render = WorldRender::new(&display);
    'main_loop: loop {
        {
            let center = vecmath::vec3_add(camera.position, vecmath::vec3_scale(camera.forward, 10.));
            view_line_vertices.write(&[
                LineVertex { pos: center, color: [1., 0., 0.] },
                LineVertex { pos: vecmath::vec3_add(center, [1., 0., 0.]), color: [1., 0., 0.] },
                LineVertex { pos: center, color: [0., 1., 0.] },
                LineVertex { pos: vecmath::vec3_add(center, [0., 1., 0.]), color: [0., 1., 0.] },
                LineVertex { pos: center, color: [0., 0., 1.] },
                LineVertex { pos: vecmath::vec3_add(center, [0., 0., 1.]), color: [0., 0., 1.] },
            ]);
        }
        let pos = [camera.position[0] as i32, camera.position[1] as i32, camera.position[2] as i32];
        world_render.update(&pos, &world.read().unwrap());
        let mut target = display.draw();
        target.clear_color_and_depth((0.5, 0.5, 0.5, 1.), 1.0);
        {
            let perspective = {
                let f = (0.5 as f32).tan();
                let aspect_ratio = 9. / 16.;
                let zfar = 1000.;
                let znear = 0.1;
                [
                    [f * aspect_ratio, 0.0, 0.0, 0.0],
                    [0.0, f, 0.0, 0.0],
                    [0.0, 0.0, (zfar + znear) / (zfar - znear), 1.0],
                    [0.0, 0.0, -(2.0 * zfar * znear) / (zfar - znear), 0.0],
                ]
            };
            let matrix = vecmath::col_mat4_mul(perspective, camera.orthogonal());
            let sampler = texture.sampled().wrap_function(SamplerWrapFunction::Repeat);
            world_render.draw(&mut target, matrix, sampler, &quad_shader).unwrap();
            target.draw(&view_line_vertices, &view_line_indices, &line_shader, &uniform! {transform:matrix}, &Default::default()).unwrap();
        }
        target.finish().unwrap();
        for ev in display.poll_events() {
            use vecmath::{vec3_add, vec3_scale, vec3_sub};
            match ev {
                glium::glutin::Event::Closed => break 'main_loop,
                glium::glutin::Event::MouseMoved(x, y) => {
                    if let Ok((x, y)) = window_util::read_mouse_delta(&display, (x, y)) {
                        use std::f32::consts::PI;
                        yaw += x as f32 / 300.;
                        pitch = (pitch - y as f32 / 300.).min(0.5 * PI).max(-0.5 * PI);
                        camera.set_yaw_pitch(yaw, pitch);
                    }
                    cam_pos.send(Message::CamChanged { pos: camera.position, direction: camera.forward }).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::W)) => {
                    camera.position = vec3_add(camera.position, vec3_scale(camera.forward, 0.5));
                    cam_pos.send(Message::CamChanged { pos: camera.position, direction: camera.forward }).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::S)) => {
                    camera.position = vec3_sub(camera.position, vec3_scale(camera.forward, 0.5));
                    cam_pos.send(Message::CamChanged { pos: camera.position, direction: camera.forward }).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::D)) => {
                    camera.position = vec3_add(camera.position, vec3_scale(camera.right, 0.5));
                    cam_pos.send(Message::CamChanged { pos: camera.position, direction: camera.forward }).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::A)) => {
                    camera.position = vec3_sub(camera.position, vec3_scale(camera.right, 0.5));
                    cam_pos.send(Message::CamChanged { pos: camera.position, direction: camera.forward }).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::E)) => {
                    camera.position = vec3_add(camera.position, vec3_scale(camera.up, 0.5));
                    cam_pos.send(Message::CamChanged { pos: camera.position, direction: camera.forward }).unwrap();
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::Q)) => {
                    camera.position = vec3_sub(camera.position, vec3_scale(camera.up, 0.5));
                    cam_pos.send(Message::CamChanged { pos: camera.position, direction: camera.forward }).unwrap();
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
    let mut cam_pos = [0.; 3];
    let mut cam_dir = [1.; 3];
    loop {
        loop {
            match rec.try_recv() {
                Ok(Message::CamChanged { pos, direction }) => {
                    cam_pos = pos;
                    cam_dir = direction;
                },
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => break,
            }
        }
        world.read().unwrap().gen_area(&[cam_pos[0] as i32, cam_pos[1] as i32, cam_pos[2] as i32], 3);
        if let Some(look_at) = world.read().unwrap().block_ray_trace(cam_pos, cam_dir, 5.) {
            world.read().unwrap().set_block(&look_at, block2).unwrap();
        }
        world.write().unwrap().flush_chunks(5, 150);
        thread::sleep(Duration::from_millis(20));
    }
}