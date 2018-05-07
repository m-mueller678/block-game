use glium;
use graphics::chunk::{RenderChunk, ChunkUniforms, RenderChunkData};
use graphics::ChunkUpdateReceiver;
use glium::texture::CompressedSrgbTexture2dArray;
use std::sync::mpsc::*;
use std::collections::{HashMap, HashSet};
use world::{CHUNK_SIZE, BlockPos, ChunkPos, chunk_at, Chunk};
use rayon;
use module::GameData;

pub struct WorldRender {
    render_dist: i32,
    render_chunks: HashMap<ChunkPos, RenderChunk>,
    need_update: Vec<ChunkPos>,
    updating: HashSet<ChunkPos>,
    render_chunk_receiver: Receiver<(ChunkPos, RenderChunkData)>,
    render_chunk_sender: Sender<(ChunkPos, RenderChunkData)>,
    player_chunk: ChunkPos,
    chunk_update_receiver: ChunkUpdateReceiver,
    game_data: GameData,
}

impl WorldRender {
    pub fn new(game_data: GameData, chunk_update_receiver: ChunkUpdateReceiver) -> Self {
        let (s, r) = channel();
        WorldRender {
            render_dist: 4,
            render_chunks: Default::default(),
            need_update: Default::default(),
            updating: Default::default(),
            render_chunk_receiver: r,
            render_chunk_sender: s,
            player_chunk: ChunkPos([0, 0, 0]),
            chunk_update_receiver,
            game_data,
        }
    }
    pub fn draw<S: glium::Surface>(
        &self,
        surface: &mut S,
        transform: [[f32; 4]; 4],
        sampler: glium::uniforms::Sampler<CompressedSrgbTexture2dArray>,
        quad_shader: &glium::Program,
    ) -> Result<(), glium::DrawError> {
        use geometry::CORNER_OFFSET;
        use vecmath::*;
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
        let chunk_iter = self.render_chunks.iter().filter(|&(ref pos, _)| {
            let corners: Vec<[f32; 3]> = CORNER_OFFSET
                .iter()
                .map(|c| {
                    [
                        c[0] + pos[0] as f32,
                        c[1] + pos[1] as f32,
                        c[2] + pos[2] as f32,
                    ]
                })
                .map(|c| vec3_scale(c, CHUNK_SIZE as f32))
                .map(|c| col_mat4_transform(transform, [c[0], c[1], c[2], 1.]))
                .map(|c| [c[0] / c[3], c[1] / c[3], c[2] / c[3]])
                .collect();
            corners.iter().any(|c| c[2] < 1.) && corners.iter().any(|c| c[0] < 1.) &&
                corners.iter().any(|c| c[0] > -1.) &&
                corners.iter().any(|c| c[1] < 1.) && corners.iter().any(|c| c[1] > -1.)
        });
        for chunk in chunk_iter {
            chunk.1.draw(surface, &uniforms, &params, quad_shader)?;
        }
        Ok(())
    }

    pub fn update<F: glium::backend::Facade>(
        &mut self,
        player_pos: BlockPos,
        facade: &F,
    ) {
        let player_pos = chunk_at(player_pos);
        //poll updates
        while let Some(pos) = self.chunk_update_receiver.poll_chunk_update() {
            if pos.square_distance(player_pos) <= self.render_dist * self.render_dist {
                self.queue_chunk_update(pos);
            }
        }
        if player_pos != self.player_chunk {
            self.change_player_pos(player_pos)
        }
        self.spawn_chunk_workers();
        let square_render_dist = self.render_dist * self.render_dist;
        self.render_chunks.retain(|&ref pos, _| {
            pos.square_distance(player_pos) <= square_render_dist
        });
        self.receive_finished_chunks(facade);
    }

    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunk_update_receiver.get_chunk(pos).map(|r| &*r.center)
    }

    fn change_player_pos(&mut self, player_pos: ChunkPos) {
        self.player_chunk = player_pos;
        let range = -self.render_dist..(self.render_dist + 1);
        for x in range.clone() {
            for y in range.clone() {
                for z in range.clone() {
                    let pos = ChunkPos([x, y, z]);
                    if pos.square_distance(player_pos) > self.render_dist * self.render_dist {
                        continue;
                    }
                    if !self.render_chunks.contains_key(&pos) && !self.updating.contains(&pos) {
                        self.queue_chunk_update(pos);
                    }
                }
            }
        }
    }

    fn spawn_chunk_workers(&mut self) {
        let player_chunk = self.player_chunk;
        self.need_update.sort_unstable_by_key(|pos| pos.square_distance(player_chunk));
        {
            let mut i = 0;
            while i < self.need_update.len() {
                let pos = self.need_update[i];
                if self.updating.contains(&pos) {
                    i += 1;
                } else {
                    if let Some(region) = self.chunk_update_receiver.get_chunk(pos).cloned() {
                        self.updating.insert(pos);
                        let sender = self.render_chunk_sender.clone();
                        let game_data = self.game_data.clone();
                        rayon::spawn(move || {
                            let render_chunk = RenderChunkData::new(&region, game_data.blocks(), pos);
                            sender.send((pos, render_chunk)).unwrap();
                        });
                    }
                    self.need_update.remove(i);
                }
            }
        }
        self.need_update.sort_unstable();
    }

    fn receive_finished_chunks<F: glium::backend::Facade>(&mut self, facade: &F) {
        const MAX_CHUNKS_PER_TICK: u32 = 1;
        for _ in 0..MAX_CHUNKS_PER_TICK {
            match self.render_chunk_receiver.try_recv() {
                Ok((pos, chunk_data)) => {
                    self.updating.remove(&pos);
                    self.render_chunks.insert(pos, RenderChunk::new(chunk_data, facade));
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    unreachable!()
                }
            }
        }
    }

    fn queue_chunk_update(&mut self, pos: ChunkPos) {
        match self.need_update.binary_search(&pos) {
            Ok(_) => {}
            Err(i) => { self.need_update.insert(i, pos); }
        }
    }
}
