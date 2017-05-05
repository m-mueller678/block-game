use block_texture_loader::TextureLoader;
use block::BlockRegistry;
use world::WorldGenBlock;
use world::structure::StructureFinder;

pub trait Module {
    fn init(&mut self,
            &mut TextureLoader,
            &mut BlockRegistry,
            &mut FnMut(WorldGenBlock),
            &mut FnMut(Box<StructureFinder>)
    );
}