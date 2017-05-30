use noise::{Perlin, NoiseModule};
use world::CHUNK_SIZE;
use world::random::*;
use block::BlockId;

pub struct GroundGen {
    layers: Vec<(BlockId, Perlin, f32, f32)>,
    noise_iter: NoiseIterator,
}

impl GroundGen {
    pub fn new(r:&WorldRngSeeder)->Self{
        GroundGen{
            layers:vec![],
            noise_iter:r.noises(),
        }
    }
    pub fn reseed(&mut self,r:&WorldRngSeeder){
        for (&mut(_,ref mut perlin,_,_),noise) in  self.layers.iter_mut().zip(r.noises()){
            *perlin=noise
        }
    }
    pub fn push_layer(mut self,block:BlockId,min_thickness:f32,max_depth:f32)->Self{
        self.layers.push((block,self.noise_iter.next().unwrap(),min_thickness,max_depth));
        self
    }
    pub fn gen_column<F: FnMut(usize, BlockId)>(&self, gen_depth: i32, set_block: &mut F, x: i32, z: i32)->usize {
        let mut iter = self.layers.iter().skip_while(|&&(_, _, _, max)| max < gen_depth as f32);
        let mut i = 0;
        if gen_depth <= 0 {
            i = (-gen_depth) as usize + 1;
        }
        let mut depth = 0.;
        while i < CHUNK_SIZE  {
            match iter.next() {
                None => { return i; }
                Some(&(block, ref perlin, min_thickness, max_depth)) => {
                    let max_thickness = (max_depth - depth) as f32;
                    let noise = (0.5 * perlin.get([x as f32 / 32., z as f32 / 32.])) + 1.;
                    let mut thickness = noise * max_thickness;
                    if thickness < min_thickness {
                        thickness = min_thickness
                    }
                    depth += thickness;
                    let depth = depth.round() as i32;
                    while i as i32<= depth && i < CHUNK_SIZE {
                        set_block(i, block);
                        i += 1;
                    }
                }
            }
        }
        i
    }
}