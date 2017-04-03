#[macro_use]
extern crate glium;
extern crate cam;
extern crate vecmath;
extern crate image;
extern crate num;
extern crate md5;

use std::time::SystemTime;
use chunk::block_graphics_supplier::*;
use glium::texture::CompressedSrgbTexture2dArray;
use glium::uniforms::SamplerWrapFunction;
use glium::DisplayBuild;
use std::io::Cursor;

mod window_util;
mod chunk;
mod block;
mod world;
mod geometry;

fn main() {
    let mut bgs = block::BlockRegistry::new();
    let block1 = bgs.add(block::Block::new(DrawType::FullOpaqueBlock([BlockTextureId::new(0); 6])));

    let display = glium::glutin::WindowBuilder::new().with_depth_buffer(24).with_vsync().build_glium().unwrap();
    let texture = {
        let image = image::load(Cursor::new(&include_bytes!("../test.png")[..]),
                                image::PNG).unwrap().to_rgba();
        let image_dimensions = image.dimensions();
        let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions);
        CompressedSrgbTexture2dArray::new(&display, vec![image]).unwrap()
    };
    display.get_window().unwrap().set_cursor_state(glium::glutin::CursorState::Hide).unwrap();
    chunk::init_chunk_shader(&display).expect("cannot load chunk shader");

    let mut world = world::World::new(&bgs, world::Generator::new(block1));
    let mut world_render = world::WorldRender::new(&display);
    world_render.update(&[0, 0, 0], &world);

    let (mut yaw, mut pitch) = (0., 0.);
    let mut camera = cam::Camera::new([0., 300., -0.]);
    let perspective = cam::CameraPerspective { fov: 90., near_clip: 0.05, far_clip: 1000., aspect_ratio: 1.0 };
    let mut loop_count = 0;

    let start_time = SystemTime::now();
    'main_loop: loop {
        let cam_pos = camera.position;
        let cam_pos = [cam_pos[0] as i32, cam_pos[1] as i32, cam_pos[2] as i32];
        world.gen_area(&cam_pos, 4);
        world_render.update(&cam_pos, &world);
        use glium::Surface;
        loop_count += 1;
        let mut target = display.draw();
        target.clear_color_and_depth((0., 0., 1., 1.), 1.0);
        {
            let matrix = vecmath::col_mat4_mul(
                perspective.projection(), camera.orthogonal());
            let sampler = texture.sampled().wrap_function(SamplerWrapFunction::Repeat);
            world_render.draw(&mut target, matrix, sampler).unwrap();
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
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::S)) => {
                    camera.position = vecmath::vec3_add(camera.position, camera.forward);
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::D)) => {
                    camera.position = vecmath::vec3_add(camera.position, camera.right);
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::A)) => {
                    camera.position = vecmath::vec3_sub(camera.position, camera.right);
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::E)) => {
                    camera.position = vecmath::vec3_add(camera.position, camera.up);
                },
                glium::glutin::Event::KeyboardInput(glium::glutin::ElementState::Pressed, _, Some(glium::glutin::VirtualKeyCode::Q)) => {
                    camera.position = vecmath::vec3_sub(camera.position, camera.up);
                },
                _ => ()
            }
        }
    }
    println!("{}", SystemTime::now().duration_since(start_time).unwrap().as_secs() as f32 / loop_count as f32 * 1000.);
}