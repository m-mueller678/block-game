use std;
use std::collections::btree_map::*;
use std::path::PathBuf;
use glium;
use image;
use graphics::BlockTextureId;

fn load_image(name: &str) -> glium::texture::RawImage2d<u8> {
    let mut path=PathBuf::from("textures");
    path.push(name);
    path.set_extension("png");
    let file = std::io::BufReader::new(std::fs::File::open(path).unwrap());
    let image = image::load(file, image::PNG).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions)
}

pub struct TextureLoader {
    names: BTreeMap<String, BlockTextureId>,
    count: u32,
}

impl TextureLoader {
    pub fn new() -> Self {
        TextureLoader {
            names: BTreeMap::new(),
            count: 0,
        }
    }

    pub fn get(&mut self, name: &str) -> BlockTextureId {
        match self.names.entry(name.into()) {
            Entry::Occupied(e) => {
                *e.get()
            }
            Entry::Vacant(e) => {
                let id = *e.insert(BlockTextureId::new(self.count));
                self.count += 1;
                id
            }
        }
    }

    pub fn load<F>(self, facade: &F) -> glium::texture::CompressedSrgbTexture2dArray
        where F: glium::backend::Facade {
        let mut names= vec!["".into(); self.count as usize];
        for (n, i) in self.names {
            names[i.to_u32() as usize] = n;
        }
        glium::texture::CompressedSrgbTexture2dArray::new
            (facade, names.iter().map(|name| load_image(&name)).collect()).unwrap()
    }
}