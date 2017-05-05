#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockTextureId(u32);

impl BlockTextureId {
    pub fn new(i: u32) -> Self {
        BlockTextureId(i)
    }
    pub fn to_u32(&self)->u32{
        self.0
    }
}

#[derive(Clone)]
pub enum DrawType {
    FullOpaqueBlock([BlockTextureId; 6]),
    None
}