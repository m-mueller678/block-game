use block_texture_loader::TextureLoader;
use block::BlockRegistry;
use world::Generator;

pub trait Module {
    fn init(&mut self,
            &mut TextureLoader,
            &mut BlockRegistry,
            &mut FnMut(Box<Generator>)
    );
}