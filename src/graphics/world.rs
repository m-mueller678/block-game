use glium;
use super::{RenderChunk, ChunkUniforms};
use glium::texture::CompressedSrgbTexture2dArray;
use world::{CHUNK_SIZE, World};

pub struct WorldRender<'a, F: 'a + glium::backend::Facade> {
    render_dist: i32,
    render_chunks: Vec<([i32; 3], RenderChunk)>,
    facade: &'a F,
}

impl<'a, F: glium::backend::Facade> WorldRender<'a, F> {
    pub fn new(facade: &'a F) -> Self {
        WorldRender {
            render_dist: 4,
            render_chunks: Vec::new(),
            facade: facade,
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
        let chunk_iter = self.render_chunks.iter().filter(|&&(pos, _)| {
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
    pub fn update(&mut self, pos: &[i32; 3], world: &World) {
        let chunk_size = CHUNK_SIZE as i32;
        let chunk_index = [pos[0] / chunk_size, pos[1] / chunk_size, pos[2] / chunk_size];
        let render_dist = self.render_dist;
        self.render_chunks.retain(|&(pos, _)| {
            (pos[0] - chunk_index[0]).abs() <= render_dist
                && (pos[1] - chunk_index[1]).abs() <= render_dist
                && (pos[2] - chunk_index[2]).abs() <= render_dist
        });
        let range = -self.render_dist..(self.render_dist + 1);
        for x in range.clone() {
            for y in range.clone() {
                for z in range.clone() {
                    let chunk_pos = [x + chunk_index[0], y + chunk_index[1], z + chunk_index[2]];
                    if !self.render_chunks.iter().any(|&(pos, _)| pos == chunk_pos) {
                        if world.chunk_loaded(&chunk_pos) {
                            let render_chunk = RenderChunk::new(self.facade, world, chunk_pos);
                            self.render_chunks.push((chunk_pos, render_chunk));
                        }
                    }
                }
            }
        }
        for chunk in self.render_chunks.iter_mut() {
            if world.reset_chunk_updated(&chunk.0) {
                chunk.1.update(&world, chunk.0);
            }
        }
    }
}