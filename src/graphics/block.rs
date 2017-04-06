#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockTextureId(u32);

impl BlockTextureId {
    pub fn new(i: u32) -> Self {
        BlockTextureId(i)
    }
    pub fn to_f32(&self) -> f32 {
        self.0 as f32
    }
}

#[derive(Clone)]
pub enum DrawType {
    FullOpaqueBlock([BlockTextureId; 6]),
    None
}