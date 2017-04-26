#![feature(integer_atomics)]
#![feature(conservative_impl_trait)]

#[macro_use]
extern crate glium;
extern crate cam;
extern crate vecmath;
extern crate image;
extern crate num;
extern crate time;
extern crate rand;
extern crate noise;

use noise::NoiseModule;
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
use self::biome::*;

mod window_util;
mod graphics;
mod block;
mod world;
mod geometry;
mod ui;
mod biome;

fn make_surface_map_builder(dirt: BlockId,
                            sand: BlockId,
                            biome_reg: &mut BiomeRegistry,
                            seeder: WorldRngSeeder)
                            -> SurfaceMapBuilder {
    let desert_env_dat = EnvironmentData { moisture: -0.8, temperature: 0.8, elevation: 0., magic: -1. };
    let desert_biome_id = biome_reg.push(Biome::new("desert".into(), desert_env_dat.clone()));
    let dirt_env_dat = EnvironmentData { moisture: 0., temperature: 0., elevation: 0., magic: -1. };
    let dirt_biome_id = biome_reg.push(Biome::new("dirt biome".into(), dirt_env_dat.clone()));
    SurfaceMapBuilder::new(seeder)
        .push_biome(desert_biome_id,
                    desert_env_dat,
                    EnvironmentDataWeight { moisture: 0.1, temperature: 0.1, elevation: 0., magic: 0.1 },
                    Box::new(move |_, _, perlin, _| {
                        (perlin * 32.) as i32
                    }),
                    Box::new(move |_, _, y_rel, _| {
                        let mut ret = [BlockId::empty(); CHUNK_SIZE];
                        for i in 0..CHUNK_SIZE {
                            if i as i32 + y_rel < 0 {
                                ret[i] = sand;
                            }
                        }
                        ret
                    }))
        .push_biome(dirt_biome_id,
                    dirt_env_dat,
                    EnvironmentDataWeight { moisture: 0.4, temperature: 0.4, elevation: 0.1, magic: 0.1 },
                    Box::new(move |x, z, perlin, seeder| {
                        let local_noise = seeder.noises(1).next().unwrap().get([x as f32 / 128., z as f32 / 128.]);
                        (32. * perlin + 8. * local_noise) as i32
                    }),
                    Box::new(move |_, _, y_rel, _| {
                        let mut ret = [Default::default(); CHUNK_SIZE];
                        for i in 0..CHUNK_SIZE {
                            if i as i32 + y_rel < 0 {
                                ret[i] = dirt;
                            }
                        }
                        ret
                    }))
}

fn load_image(path: &str) -> glium::texture::RawImage2d<u8> {
    let file = std::io::BufReader::new(File::open(path).unwrap());
    let image = image::load(file, image::PNG).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions)
}

fn main() {
    let mut bgs = BlockRegistry::new();
    let block_sand = bgs.add(Block::new(DrawType::FullOpaqueBlock([BlockTextureId::new(0); 6]), LightType::Opaque));
    let block_dirt = bgs.add(Block::new(DrawType::FullOpaqueBlock([BlockTextureId::new(1); 6]), LightType::Opaque));
    let mut biomes = BiomeRegistry::new();
    let seeder = world::WorldRngSeeder::new(1);
    let surface_map = make_surface_map_builder(block_dirt, block_sand, &mut biomes, seeder);
    let biomes = Arc::new(biomes);
    let generator = world::Generator::new(seeder, surface_map);
    let (world_reader, mut world_writer) = new_world(Arc::new(bgs), biomes, generator);
    let (send, rec) = channel();
    let display = glium::glutin::WindowBuilder::new().with_depth_buffer(24).with_vsync().build_glium().unwrap();
    let texture = CompressedSrgbTexture2dArray::new(&display, vec![
        load_image("textures/sand.png"),
        load_image("textures/dirt.png")
    ]).unwrap();
    display.get_window().unwrap().set_cursor_state(glium::glutin::CursorState::Hide).unwrap();
    let quad_shader = graphics::load_quad_shader(&display).expect("cannot load quad shader");
    let line_shader = graphics::load_line_shader(&display).unwrap();
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
                        assert! (button < 2);
                        mouse_pressed_since[button] = Some(SteadyTime::now());
                        if button == 1 {
                            if let Some(ref block_target) = block_target {
                                world_writer.read().set_block(&block_target.block.facing(block_target.face), block_dirt).is_ok();
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
            if let Some(cam_pos) = cam_pos {
                world_writer.gen_area(&BlockPos([cam_pos[0] as i32, cam_pos[1] as i32, cam_pos[2] as i32]), 3);
            }
            if let Some(block_target) = block_target.clone() {
                let read_guard = world_writer.read();
                if let Some(pressed_since) = mouse_pressed_since[0] {
                    if (SteadyTime::now() - pressed_since).num_milliseconds() > 500 {
                        read_guard.set_block(&block_target.block, BlockId::empty()).is_ok();
                    }
                } else {
                    if let Some(pressed_since) = mouse_pressed_since[1] {
                        if (SteadyTime::now() - pressed_since).num_milliseconds() > 500 {
                            read_guard.set_block(&block_target.block.facing(block_target.face), block_dirt).is_ok();
                        }
                    }
                }
            }
            world_writer.flush_chunk();
            thread::sleep(std::time::Duration::from_millis(20));
        }
    });
    let mut ui = ui::Ui::new(display, quad_shader, line_shader, send, texture, world_reader);
    ui.run()
}