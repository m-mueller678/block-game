#[macro_use]
extern crate glium;
extern crate cam;
extern crate vecmath;
extern crate image;

use std::time::SystemTime;
use chunk::{RenderChunk, ChunkUniforms, CHUNK_SIZE};
use chunk::block_graphics_supplier::*;
use glium::texture::CompressedSrgbTexture2dArray;
use glium::uniforms::SamplerWrapFunction;
use glium::DisplayBuild;
use std::io::Cursor;


struct BGS {}

impl BlockGraphicsSupplier for BGS {
    fn get_draw_type(&self, block_id: u32) -> DrawType {
        if block_id == 0 {
            DrawType::None
        } else {
            DrawType::FullOpaqueBlock([0; 6])
        }
    }

    fn texture_size(&self) -> u32 {
        16
    }

    fn is_opaque(&self, block_id: u32) -> bool {
        block_id != 0
    }
}

mod window_util;
mod chunk;

fn main() {
    let display = glium::glutin::WindowBuilder::new().with_depth_buffer(24).with_vsync().build_glium().unwrap();
    let bgs = BGS {};
    let texture = {
        let image = image::load(Cursor::new(&include_bytes!("../test.png")[..]),
                                image::PNG).unwrap().to_rgba();
        let image_dimensions = image.dimensions();
        let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions);
        CompressedSrgbTexture2dArray::new(&display, vec![image]).unwrap()
    };
    display.get_window().unwrap().set_cursor_state(glium::glutin::CursorState::Hide).unwrap();
    chunk::init_chunk_shader(&display).expect("cannot load chunk shader");
    let mut chunk1 = chunk::Chunk::new();
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                if ((x % 32 < 16) ^ (z % 32 < 16)) && y % 2 == 0 {
                    chunk1.set_block(&[x, y, z], 1);
                }
            }
        }
    }
    let mut rc = Vec::new();
    for x in -1..2 {
        for y in -1..2 {
            for z in -1..2 {
                rc.push(RenderChunk::new(&display, &chunk1, &bgs, [x as f32 * 32., y as f32 * 32., z as f32 * 32.]));
            }
        }
    }
    let (mut yaw, mut pitch) = (0., 0.);
    let mut camera = cam::Camera::new([0., 0., -0.]);
    let perspective = cam::CameraPerspective { fov: 90., near_clip: 0.05, far_clip: 1000., aspect_ratio: 1.0 };
    let mut loop_count = 0;
    let start_time = SystemTime::now();
    'main_loop: loop {
        rc[0].update(&chunk1, &bgs, [-64., -64., -64.]);
        use glium::Surface;
        loop_count += 1;
        let mut target = display.draw();
        target.clear_color_and_depth((0., 0., 1., 1.), 1.0);
        {
            let matrix: [[f32; 4]; 4] = vecmath::col_mat4_mul(perspective.projection(), camera.orthogonal());
            let light: [f32; 3] = [0., -2., 1.];
            let params = glium::DrawParameters {
                depth: glium::Depth {
                    test: glium::draw_parameters::DepthTest::IfLess,
                    write: true,
                    ..Default::default()
                },
                backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
                ..Default::default()
            };
            for chunk in rc.iter() {
                chunk.draw(&mut target, &ChunkUniforms { transform: matrix, light: light, sampler: texture.sampled().wrap_function(SamplerWrapFunction::Repeat) }, &params).unwrap();
            }
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