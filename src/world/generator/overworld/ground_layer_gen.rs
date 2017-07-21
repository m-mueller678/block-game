use noise::{Perlin, NoiseModule};
use world::CHUNK_SIZE;
use world::random::*;
use block::BlockId;
use std::cmp::max;

pub struct GroundGen {
    layers: Vec<(BlockId, Perlin, f32, f32)>,
    noise_iter: NoiseIterator,
}


impl GroundGen {
    pub fn new() -> Self {
        GroundGen {
            layers: vec![],
            noise_iter: WorldRngSeeder::new(0).noises(),
        }
    }
    pub fn reseed(&mut self, r: &WorldRngSeeder) {
        for (&mut (_, ref mut perlin, _, _), noise) in self.layers.iter_mut().zip(r.noises()) {
            *perlin = noise
        }
    }
    pub fn push_layer(mut self, block: BlockId, min_thickness: f32, max_depth: f32) -> Self {
        self.layers.push((block, self.noise_iter.next().unwrap(), min_thickness, max_depth));
        self
    }
    pub fn gen_column<F: FnMut(usize, BlockId)>(&self, gen_depth: i32, set_block: &mut F, x: i32, z: i32) -> usize {
        let mut layer_iter = self.layers.iter().skip_while(|&&(_, _, _, max)| max-0.01 <= gen_depth as f32);
        let mut i=0;
        if gen_depth<0{
            i=(-gen_depth) as usize;
        }
        if i>=CHUNK_SIZE{
            return CHUNK_SIZE;
        }
        let mut depth=max(0,gen_depth) as f32;
        while i<CHUNK_SIZE {
            match layer_iter.next() {
                None => { return i; }
                Some(&(block, ref perlin, min_thickness, max_depth)) => {
                    let max_thickness = (max_depth - depth) as f32;
                    assert!(max_thickness-min_thickness>-0.01);
                    let noise = (0.5 * perlin.get([x as f32 / 32., z as f32 / 32.])) + 0.5;
                    let thickness = min_thickness+ noise * (max_thickness-min_thickness);
                    depth += thickness;
                    let depth=depth.round() as i32;
                    while i<CHUNK_SIZE && (i as i32 +gen_depth as i32)<depth{
                        set_block(i, block);
                        i+=1;
                    }
                }
            }
        }
        i
    }
}