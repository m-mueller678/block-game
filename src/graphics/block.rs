use super::TextureId;

#[derive(Clone)]
pub enum DrawType {
    FullOpaqueBlock([TextureId; 6]),
    None,
}
