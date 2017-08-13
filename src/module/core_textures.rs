use block_texture_loader::TextureLoader;
use graphics::TextureId;

pub struct CoreTextureMap{
    pub ui_item_slot:TextureId,
}

impl CoreTextureMap{
    pub fn new(loader:&mut TextureLoader)->CoreTextureMap{
        CoreTextureMap{
            ui_item_slot:loader.get("ui/item_slot")
        }
    }
}