use std;
use std::collections::btree_map::*;
use std::path::*;
use glium;
use image;
use image::Rgba;
use graphics::TextureId;
use logging::root_logger;

fn image_from_file(path: &Path) -> Result<glium::texture::RawImage2d<'static, u8>, Box<std::error::Error>> {
    let file = std::io::BufReader::new(std::fs::File::open(&path)?);
    let image = image::load(file, image::PNG)?;
    let image = image.to_rgba();
    let image_dimensions = image.dimensions();
    Ok(glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions))
}

///generates a randomly colored checkerboard texture from name
fn generate_missing_texture(name: &str) -> glium::texture::RawImage2d<'static, u8> {
    use std::hash::*;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    let h = hasher.finish();
    let hash_color = [h as u8, (h >> 8) as u8, (h >> 16) as u8];
    let colors = [
        Rgba { data: [hash_color[0], hash_color[1], hash_color[2], 255] },
        Rgba { data: [hash_color[0] ^ 128, !hash_color[1] ^ 128, !hash_color[2] ^ 128, 255] }
    ];
    let image = image::RgbaImage::from_fn(32, 32, |x, y| {
        colors[(((x ^ y) >> 3) & 1) as usize]
    });
    let image_dimensions = image.dimensions();
    glium::texture::RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions)
}

fn name_to_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from("textures");
    for element in name.split('/') {
        path.push(element);
    }
    path.set_extension("png");
    path
}

fn load_image(name: &str) -> glium::texture::RawImage2d<u8> {
    let path = name_to_path(name);
    match image_from_file(&path) {
        Ok(image) => image,
        Err(e) => {
            error! (root_logger(), "can not load texture {:?}: {}", path, e);
            generate_missing_texture(name)
        }
    }
}

pub struct TextureLoader {
    names: BTreeMap<String, TextureId>,
    count: u32,
}

impl TextureLoader {
    pub fn new() -> Self {
        TextureLoader {
            names: BTreeMap::new(),
            count: 0,
        }
    }

    pub fn get(&mut self, name: &str) -> TextureId {
        match self.names.entry(name.into()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let id = *e.insert(TextureId::new(self.count));
                self.count += 1;
                id
            }
        }
    }

    pub fn load<F>(self, facade: &F) -> glium::texture::CompressedSrgbTexture2dArray
        where
            F: glium::backend::Facade,
    {
        let mut names = vec!["".into(); self.count as usize];
        for (n, i) in self.names {
            names[i.to_u32() as usize] = n;
        }
        glium::texture::CompressedSrgbTexture2dArray::new(
            facade,
            names.iter().map(|name| load_image(name)).collect(),
        ).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn load() {
        image_from_file(&name_to_path("stone")).unwrap();
        assert!(image_from_file(&name_to_path("abcdef")).is_err());
        generate_missing_texture("stone");
    }
}