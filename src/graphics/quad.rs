use std::borrow::Cow;
use glium::*;
use glium::backend::Facade;

pub fn load_quad_shader<F: Facade>(facade: &F) -> Result<Program, ProgramCreationError> {
    Program::from_source(facade, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC, None)
}

pub fn get_triangle_indices(quad_count: usize) -> Vec<u32> {
    let mut ind = Vec::with_capacity(quad_count * 6);
    for i in 0..(quad_count as u32) {
        ind.push(i * 4);
        ind.push(i * 4 + 1);
        ind.push(i * 4 + 2);
        ind.push(i * 4);
        ind.push(i * 4 + 2);
        ind.push(i * 4 + 3);
    }
    ind
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub texture_id: f32,
    pub light_level: f32,
}

//workaround for bug in implement_vertex macro
//glium issue #1607
impl vertex::Vertex for Vertex {
    fn build_bindings() -> vertex::VertexFormat {
        static VERTEX_FORMAT: [(Cow<'static, str>, usize, vertex::AttributeType, bool); 5] = [
            (Cow::Borrowed("position"), 0, vertex::AttributeType::F32F32F32, false),
            (Cow::Borrowed("normal"), 3 * 4, vertex::AttributeType::F32F32F32, false),
            (Cow::Borrowed("tex_coords"), 6 * 4, vertex::AttributeType::F32F32, false),
            (Cow::Borrowed("texture_id"), 8 * 4, vertex::AttributeType::F32, false),
            (Cow::Borrowed("light_level"), 9 * 4, vertex::AttributeType::F32, false),
        ];
        Cow::Borrowed(&VERTEX_FORMAT)
    }
}

const VERTEX_SHADER_SRC: &str = r#"
    #version 140

    in vec3 normal;
    in vec3 position;
    in vec2 tex_coords;
    in float texture_id;
    in float light_level;

    out float brightness;
    out vec2 v_tex_coords;
    out float v_texture_id;

    uniform mat4 matrix;
    uniform vec3 light_direction;

    void main() {
        gl_Position = matrix*vec4(position, 1.0);
        brightness = mix(0.6,1.0,abs(dot(normalize(light_direction),normalize(normal))))*light_level;
        v_tex_coords=tex_coords;
        v_texture_id=texture_id;
    }
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
    #version 140

    in float brightness;
    in vec2 v_tex_coords;
    in float v_texture_id;

    out vec4 color;

    uniform sampler2DArray sampler;


    void main() {
        color=texture(sampler,vec3(v_tex_coords,floor(v_texture_id+0.5)))*brightness;
    }
"#;
