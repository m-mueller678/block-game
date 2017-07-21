use glium::backend::Facade;
use glium::{Program,ProgramCreationError};

mod quad;
mod line;
mod chunk;
mod block;
mod world;
#[allow(dead_code)]
mod block_overlay;

pub use self::chunk::{RenderChunk, ChunkUniforms};
pub use self::block::{BlockTextureId, DrawType};
pub use self::world::WorldRender;
pub use self::line::Vertex as LineVertex;
pub use self::block_overlay::{BlockOverlay,OverlayDataSupplier,Overlay2d};

use self::quad::Vertex as QuadVertex;

pub struct Shader {
    pub quad:Program,
    pub overlay:Program,
    pub line:Program,
}

impl Shader {
    pub fn new<F:Facade>(facade:&F) -> Result<Self,ProgramCreationError> {
        Ok(Shader{
            quad: quad::load_quad_shader(facade)?,
            overlay: block_overlay::load_overlay_shader(facade)?,
            line: line::load_line_shader(facade)?,
        })
    }
}