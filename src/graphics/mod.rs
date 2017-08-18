use glium::backend::Facade;
use glium::{Program, ProgramCreationError};

mod quad;
mod line;
mod chunk;
mod block;
mod world;
mod virtual_display;
#[allow(dead_code)]
mod block_overlay;
mod text;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextureId(u32);

impl TextureId {
    pub fn new(i: u32) -> Self {
        TextureId(i)
    }
    pub fn to_u32(&self) -> u32 {
        self.0
    }
}

pub use self::chunk::{RenderChunk, ChunkUniforms};
pub use self::block::DrawType;
pub use self::world::WorldRender;
pub use self::line::Vertex as LineVertex;
pub use self::block_overlay::{BlockOverlay, OverlayDataSupplier, Overlay2d};
pub use self::virtual_display::{RenderBuffer2d, VirtualDisplay, TransformedDisplay};
pub use self::text::FontTextureHandle;

use self::quad::Vertex as QuadVertex;

pub struct Shader {
    pub quad: Program,
    pub overlay: Program,
    pub line: Program,
    pub tri_2d: Program,
}

impl Shader {
    pub fn new<F: Facade>(facade: &F) -> Result<Self, ProgramCreationError> {
        Ok(Shader {
            quad: quad::load_quad_shader(facade)?,
            overlay: block_overlay::load_overlay_shader(facade)?,
            line: line::load_line_shader(facade)?,
            tri_2d: virtual_display::load_2d_shader(facade)?,
        })
    }
}