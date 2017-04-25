use glium;
use super::{RenderChunk, ChunkUniforms};
use glium::texture::CompressedSrgbTexture2dArray;
use world::{CHUNK_SIZE, BlockPos, ChunkPos, WorldReadGuard, chunk_at};

pub struct WorldRender {
    render_dist: i32,
    render_chunks: Vec<(ChunkPos, RenderChunk)>,
}

impl WorldRender {
    pub fn new() -> Self {
        WorldRender {
            render_dist: 4,
            render_chunks: Vec::new(),
        }
    }
    pub fn draw<S: glium::Surface>(&self, surface: &mut S,
                                   transform: [[f32; 4]; 4],
                                   sampler: glium::uniforms::Sampler<CompressedSrgbTexture2dArray>,
                                   quad_shader: &glium::Program)
                                   -> Result<(), glium::DrawError> {
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
        let chunk_iter = self.render_chunks.iter().filter(|&&(ref pos, _)| {
            let corners: Vec<[f32; 3]> = CORNER_OFFSET.iter().
                map(|c| [c[0] + pos[0] as f32, c[1] + pos[1] as f32, c[2] + pos[2] as f32])
                .map(|c| vec3_scale(c, CHUNK_SIZE as f32))
                .map(|c| col_mat4_transform(transform, [c[0], c[1], c[2], 1.]))
                .map(|c| [c[0] / c[3], c[1] / c[3], c[2] / c[3]]).collect();
            corners.iter().any(|c| c[2] < 1.)
                & &corners.iter().any(|c| c[0] < 1.)
                & &corners.iter().any(|c| c[0] > -1.)
                & &corners.iter().any(|c| c[1] < 1.)
                & &corners.iter().any(|c| c[1] > -1.)
        });
        for chunk in chunk_iter {
            chunk.1.draw(surface, &uniforms, &params, quad_shader)?;
        }
        Ok(())
    }
    pub fn update<F: glium::backend::Facade>(&mut self, player_pos: &BlockPos, world: &WorldReadGuard, facade: &F) {
        let chunk_pos = chunk_at(player_pos);
        let render_dist = self.render_dist;
        self.render_chunks.retain(|&(ref pos, _)| {
            (pos[0] - chunk_pos[0]).abs() <= render_dist
                && (pos[1] - chunk_pos[1]).abs() <= render_dist
                && (pos[2] - chunk_pos[2]).abs() <= render_dist
        });
        let range = -self.render_dist..(self.render_dist + 1);
        for x in range.clone() {
            for y in range.clone() {
                for z in range.clone() {
                    let chunk_pos = ChunkPos([x + chunk_pos[0], y + chunk_pos[1], z + chunk_pos[2]]);
                    if !self.render_chunks.iter().any(|&(ref pos, _)| *pos == chunk_pos) {
                        if world.chunk_loaded(&chunk_pos) {
                            let render_chunk = RenderChunk::new(facade, world, &chunk_pos);
                            self.render_chunks.push((chunk_pos, render_chunk));
                        }
                    }
                }
            }
        }
        for chunk in self.render_chunks.iter_mut() {
            if world.reset_chunk_updated(&chunk.0) {
                chunk.1.update(&world, &chunk.0);
            }
        }
    }
}