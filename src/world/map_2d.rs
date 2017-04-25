use std::collections::HashMap;
use super::{CHUNK_SIZE, chunk_xz_index};
use num::Integer;

pub trait Map2d<T: Copy + Default> where Self: Send {
    fn get(&mut self, i32, i32) -> &T;
}

struct Map2dStruct<T: Copy + Default + Send, F: FnMut(i32, i32) -> T + Send> {
    chunks: HashMap<[i32; 2], [T; CHUNK_SIZE * CHUNK_SIZE]>,
    generate: F,
}

impl<T: Copy + Default + Send, F: FnMut(i32, i32) -> T + Send> Map2d<T> for Map2dStruct<T, F> {
    fn get(&mut self, x: i32, z: i32) -> &T {
        let cs = CHUNK_SIZE as i32;
        let x_div = x.div_floor(&cs);
        let z_div = z.div_floor(&cs);
        let key = [x_div, z_div];
        let generate = &mut self.generate;
        let chunk = self.chunks.entry(key).or_insert_with(|| {
            let mut ret = [Default::default(); CHUNK_SIZE * CHUNK_SIZE];
            let x_base = x_div * CHUNK_SIZE as i32;
            let z_base = z_div * CHUNK_SIZE as i32;
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    ret[chunk_xz_index(x, z)] = generate(x as i32 + x_base, z as i32 + z_base);
                }
            }
            ret
        });
        &chunk[chunk_xz_index(x.mod_floor(&cs) as usize, z.mod_floor(&cs) as usize)]
    }
}

pub fn new_map_2d<T: Copy + Default + Send, F: FnMut(i32, i32) -> T + Send>(gen: F) -> impl Map2d<T> {
    Map2dStruct {
        chunks: HashMap::new(),
        generate: gen,
    }
}