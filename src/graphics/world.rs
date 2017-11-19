use std::collections::hash_map::*;
use std::sync::mpsc::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use vecmath::*;
use geometry::CORNER_OFFSET;
use glium;
use super::{RenderChunk, RenderChunkUpdate, ChunkUniforms};
use glium::texture::CompressedSrgbTexture2dArray;
use world::{CHUNK_SIZE, BlockPos, ChunkPos, World};

#[derive(PartialEq, Eq, Clone, Copy)]
struct RenderArea {
    min: [i32; 3],
    size: [i32; 3],
}

impl RenderArea {
    fn new() -> RenderArea {
        RenderArea {
            min: [0; 3],
            size: [0; 3],
        }
    }

    fn contains(&self, pos: ChunkPos) -> bool {
        pos.iter()
            .enumerate()
            .all(|(i, p)| {
                     let dif = p - self.min[i];
                     dif >= 0 && dif < self.size[i]
                 })
    }


    fn iter(&self) -> RenderAreaIter {
        RenderAreaIter {
            min: self.min,
            max: [self.min[0] + self.size[0],
                  self.min[1] + self.size[1],
                  self.min[2] + self.size[2]],
            pos: [self.min[0] - 1, self.min[1], self.min[2]],
        }
    }
}

struct RenderAreaIter {
    min: [i32; 3],
    max: [i32; 3],
    pos: [i32; 3],
}

impl Iterator for RenderAreaIter {
    type Item = ChunkPos;

    fn next(&mut self) -> Option<Self::Item> {
        self.pos[0] += 1;
        if self.pos[0] == self.max[0] {
            self.pos[0] = self.min[0];
            self.pos[1] += 1;
            if self.pos[1] == self.max[1] {
                self.pos[1] = self.min[1];
                self.pos[2] += 1;
                if self.pos[2] == self.max[2] {
                    return None;
                }
            }
        }
        Some(ChunkPos(self.pos))
    }
}

type UpdateMessage = (usize, Option<(ChunkPos, RenderChunkUpdate)>);

pub struct WorldRender {
    render_dist: i32,
    player_chunk: ChunkPos,
    shared_area: Arc<Mutex<RenderArea>>,
    update_receiver: Receiver<UpdateMessage>,
    render_chunks: Vec<Option<(ChunkPos, RenderChunk)>>,
}

const DEFAULT_RENDER_CHUNK_COUNT: usize = 625;

impl WorldRender {
    pub fn new(world: Arc<World>) -> Self {
        let player_pos_init = ChunkPos([i32::min_value(); 3]);
        let (send, rec) = channel();
        let shared = Arc::new(Mutex::new(RenderArea::new()));
        let shared2 = Arc::clone(&shared);
        thread::spawn(move || {
                          let updater = RenderChunkUpdater::new(shared2, world, send);
                          updater.run();
                      });
        WorldRender {
            render_dist: 4,
            shared_area: shared,
            player_chunk: player_pos_init,
            update_receiver: rec,
            render_chunks: (0..DEFAULT_RENDER_CHUNK_COUNT).map(|_| None).collect(),
        }
    }
    pub fn draw<S: glium::Surface>(
        &self,
        surface: &mut S,
        transform: [[f32; 4]; 4],
        sampler: glium::uniforms::Sampler<CompressedSrgbTexture2dArray>,
        quad_shader: &glium::Program,
) -> Result<(), glium::DrawError>{
        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        };
        let uniforms = ChunkUniforms {
            transform: transform,
            light: [0., -2., 1.],
            sampler: sampler,
        };
        let chunk_iter = self.render_chunks
            .iter()
            .filter_map(|opt| {
                opt.as_ref()
                    .and_then(|&(pos, ref chunk)| {
                        let corners: Vec<[f32; 3]> = CORNER_OFFSET
                            .iter()
                            .map(|c| {
                                     [c[0] + pos[0] as f32,
                                      c[1] + pos[1] as f32,
                                      c[2] + pos[2] as f32]
                                 })
                            .map(|c| vec3_scale(c, CHUNK_SIZE as f32))
                            .map(|c| col_mat4_transform(transform, [c[0], c[1], c[2], 1.]))
                            .map(|c| [c[0] / c[3], c[1] / c[3], c[2] / c[3]])
                            .collect();
                        let visible = corners.iter().any(|c| c[2] < 1.) &&
                                      corners.iter().any(|c| c[0] < 1.) &&
                                      corners.iter().any(|c| c[0] > -1.) &&
                                      corners.iter().any(|c| c[1] < 1.) &&
                                      corners.iter().any(|c| c[1] > -1.);
                        if visible { Some(chunk) } else { None }
                    })
            });
        for chunk in chunk_iter {
            chunk.draw(surface, &uniforms, &params, quad_shader)?;
        }
        Ok(())
    }
    pub fn update<F: glium::backend::Facade>(&mut self, player_pos: BlockPos, facade: &F) {
        let chunk_pos = player_pos.containing_chunk();
        if chunk_pos != self.player_chunk {
            self.player_chunk = chunk_pos;
            *self.shared_area.lock().unwrap() = Self::render_area(self.render_dist, chunk_pos);
        }
        loop {
            match self.update_receiver.try_recv() {
                Ok((index, update)) => {
                    if self.render_chunks.len() <= index {
                        self.render_chunks.resize_default(index + 1)
                    }
                    if let Some(update) = update {
                        if update.1.is_empty() {
                            self.render_chunks[index] = None;
                        } else {
                            match &mut self.render_chunks[index] {
                                &mut Some((ref mut pos, ref mut chunk)) => {
                                    *pos = update.0;
                                    chunk.apply_update(update.1);
                                }
                                none => {
                                    *none = Some((update.0,
                                                  RenderChunk::from_update(facade, update.1)));
                                }
                            }
                        }
                    } else {
                        self.render_chunks[index] = None;
                    }
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    panic!("connection to Render chunk updater lost");
                }
            }
        }
    }

    fn render_area(dist: i32, center: ChunkPos) -> RenderArea {
        RenderArea {
            min: vec3_sub(*center, [dist; 3]),
            size: [dist * 2 + 1; 3],
        }
    }
}

struct RenderChunkUpdater {
    shared_area: Arc<Mutex<RenderArea>>,
    is_used: Vec<bool>,
    chunk_indices: HashMap<ChunkPos, usize>,
    render_area: RenderArea,
    world: Arc<World>,
    update_sender: Sender<UpdateMessage>,
}

impl RenderChunkUpdater {
    fn new(area: Arc<Mutex<RenderArea>>,
           world: Arc<World>,
           update_sender: Sender<UpdateMessage>)
           -> Self {
        RenderChunkUpdater {
            shared_area: area,
            is_used: vec![false; DEFAULT_RENDER_CHUNK_COUNT],
            chunk_indices: HashMap::new(),
            render_area: RenderArea::new(),
            world,
            update_sender: update_sender,
        }
    }
    fn run(mut self) {
        loop {
            if Arc::strong_count(&self.shared_area) == 1 {
                return;
            }
            let new_render_area = {
                *self.shared_area.lock().unwrap()
            };
            if new_render_area != self.render_area {
                self.update_area(new_render_area)
            }
            let world = self.world.read();
            if let Some(pos) = world.poll_chunk_update() {
                if let Some(&index) = self.chunk_indices.get(&pos) {
                    let update = RenderChunkUpdate::new(&*world, pos);
                    self.update_sender
                        .send((index, Some((pos, update))))
                        .unwrap();
                }
            } else {
                thread::sleep(Duration::from_millis(5));
            }
        }
    }
    fn update_area(&mut self, new_area: RenderArea) {
        let &mut RenderChunkUpdater {
                     ref mut is_used,
                     ref mut chunk_indices,
                     ref mut render_area,
                     ref mut world,
                     ref mut update_sender,
                     ..
                 } = self;
        *render_area = new_area;
        {
            chunk_indices.retain(|&pos, &mut index| if new_area.contains(pos) {
                                     true
                                 } else {
                                     update_sender.send((index, None)).unwrap();
                                     is_used[index] = false;
                                     false
                                 });
        }
        let world = world.read();
        for pos in new_area.iter() {
            if let Entry::Vacant(v) = chunk_indices.entry(pos) {
                if let Some(empty) = (0..is_used.len()).find(|&i| !is_used[i]) {
                    is_used[empty] = true;
                    v.insert(empty);
                    update_sender
                        .send((empty, Some((pos, RenderChunkUpdate::new(&*world, pos)))))
                        .unwrap();
                } else {
                    let index = is_used.len();
                    is_used.push(false);
                    v.insert(index);
                    update_sender
                        .send((index, Some((pos, RenderChunkUpdate::new(&*world, pos)))))
                        .unwrap();
                    return;
                }
            };
        }
    }
}
