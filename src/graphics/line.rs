use std::borrow::Cow;
use glium::{Program, ProgramCreationError};
use glium::vertex;
use glium::backend::Facade;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}


//workaround for bug in implement_vertex macro
//glium issue #1607
impl vertex::Vertex for Vertex {
    fn build_bindings() -> vertex::VertexFormat {
        static VERTEX_FORMAT: [(Cow<'static, str>, usize, vertex::AttributeType, bool); 2] = [
            (
                Cow::Borrowed("pos"),
                0,
                vertex::AttributeType::F32F32F32,
                false,
            ),
            (
                Cow::Borrowed("color"),
                4 * 3,
                vertex::AttributeType::F32F32F32,
                false,
            ),
        ];
        Cow::Borrowed(&VERTEX_FORMAT)
    }
}

pub fn load_line_shader<F: Facade>(facade: &F) -> Result<Program, ProgramCreationError> {
    Program::from_source(facade, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC, None)
}

const VERTEX_SHADER_SRC: &str = r#"
#version 140

in vec3 pos;
in vec3 color;

out vec3 v_color;

uniform mat4 transform;

void main(){
    v_color=color;
    gl_Position=transform*vec4(pos,1.);
}
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
#version 140

in vec3 v_color;

out vec4 color;

void main(){
    color=vec4(v_color,1);
}
"#;
