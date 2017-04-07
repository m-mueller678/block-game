use glium::{Program, ProgramCreationError};
use glium::backend::Facade;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}

implement_vertex!(Vertex,pos,color);

pub fn load_line_shader<F: Facade>(facade: &F) -> Result<Program, ProgramCreationError> {
    Program::from_source(facade, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC, None)
}

const VERTEX_SHADER_SRC: &'static str = r#"
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

const FRAGMENT_SHADER_SRC: &'static str = r#"
#version 140

in vec3 v_color;

out vec4 color;

void main(){
    color=vec4(v_color,1);
}
"#;