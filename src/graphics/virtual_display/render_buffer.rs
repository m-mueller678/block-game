use std::borrow::Cow;
use std::rc::Rc;
use glium::*;
use glium::backend::glutin::Display;
use glium::texture::CompressedSrgbTexture2dArray;
use glium::backend::Facade;
use graphics::TextureId;
use glium::backend::Context;
use super::VirtualDisplay;

pub struct RenderBuffer2d {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    x_y_ratio: f32,
    width: f32,
    height: f32,
    context: Rc<Context>,
}

pub fn load_2d_shader<F: Facade>(facade: &F) -> Result<Program, ProgramCreationError> {
    Program::from_source(facade, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC, None)
}

impl VirtualDisplay for RenderBuffer2d {
    fn textured_triangle(&mut self, position: [[f32; 2]; 3], tex_coords: [[f32; 2]; 3], texture_id: TextureId, brightness: f32) {
        for i in 0..3 {
            self.indices.push(self.vertices.len() as u16);
            self.vertices.push(Vertex {
                position: Self::map_to_gl(position[i]),
                tex_coords: tex_coords[i],
                texture_id: texture_id.to_u32() as f32,
                brightness
            });
        }
    }
    fn textured_quad(&mut self, position: [[f32; 2]; 4], tex_coords: [[f32; 2]; 4], texture_id: TextureId, brightness: f32) {
        let first_index = self.vertices.len() as u16;
        for i in 0..4 {
            self.vertices.push(Vertex {
                position: Self::map_to_gl(position[i]),
                tex_coords: tex_coords[i],
                texture_id: texture_id.to_u32() as f32,
                brightness
            });
        }
        self.indices.push(first_index + 0);
        self.indices.push(first_index + 1);
        self.indices.push(first_index + 2);
        self.indices.push(first_index + 2);
        self.indices.push(first_index + 3);
        self.indices.push(first_index + 0);
    }
    fn x_y_ratio(&self) -> f32 {
        self.x_y_ratio
    }
    fn ui_size_x(&self) -> f32 {
        self.width
    }
    fn ui_size_y(&self) -> f32 {
        self.height
    }
}

impl RenderBuffer2d {
    pub fn new(display: &Display) -> Self {
        let size = display.gl_window().get_inner_size_pixels();
        RenderBuffer2d {
            vertices: Vec::new(),
            indices: Vec::new(),
            x_y_ratio: size.map(|(x, y)| x as f32 / y as f32).unwrap_or(1.),
            width: size.map(|(x, _)| x as f32/50.).unwrap_or(20.),
            height: size.map(|(_, y)| y as f32/50.).unwrap_or(20.),
            context: display.get_context().clone(),

        }
    }
    pub fn render<S: Surface>(&self, surface: &mut S, tri_shader: &Program, sampler: uniforms::Sampler<CompressedSrgbTexture2dArray>) {
        let v_buf = VertexBuffer::new(&self.context, &self.vertices).unwrap();
        let i_buf = IndexBuffer::new(&self.context, index::PrimitiveType::TrianglesList, &self.indices).unwrap();
        surface.draw(&v_buf, &i_buf, tri_shader, &uniform! {sampler:sampler}, &Default::default()).unwrap();
    }
    fn map_to_gl(p: [f32; 2]) -> [f32; 2] {
        [p[0].mul_add(2., -1.), p[1].mul_add(-2., 1.)]
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    texture_id: f32,
    brightness: f32,
}

//workaround for bug in implement_vertex macro
//glium issue #1607
impl vertex::Vertex for Vertex {
    fn build_bindings() -> vertex::VertexFormat {
        static VERTEX_FORMAT: [(Cow<'static, str>, usize, vertex::AttributeType, bool); 4] = [
            (Cow::Borrowed("position"), 0, vertex::AttributeType::F32F32, false),
            (Cow::Borrowed("tex_coords"), 2 * 4, vertex::AttributeType::F32F32, false),
            (Cow::Borrowed("texture_id"), 4 * 4, vertex::AttributeType::F32, false),
            (Cow::Borrowed("brightness"), 5 * 4, vertex::AttributeType::F32, false),
        ];
        Cow::Borrowed(&VERTEX_FORMAT)
    }
}

const VERTEX_SHADER_SRC: &'static str = r#"
    #version 140

    in vec2 position;
    in vec2 tex_coords;
    in float texture_id;
    in float brightness;

    out vec2 v_tex_coords;
    out float v_texture_id;
    out float v_brightness;

    void main() {
        gl_Position = vec4(position,1.0, 1.0);
        v_tex_coords=tex_coords;
        v_texture_id=texture_id;
        v_brightness=brightness;
    }
"#;

const FRAGMENT_SHADER_SRC: &'static str = r#"
    #version 140

    in vec2 v_tex_coords;
    in float v_texture_id;
    in float v_brightness;

    out vec4 color;

    uniform sampler2DArray sampler;

    void main() {
        color=texture(sampler,vec3(v_tex_coords,floor(v_texture_id+0.5)))*v_brightness;
    }
"#;
