use glium::{
    implement_vertex,
    index::{NoIndices, PrimitiveType},
    uniform, Display, DrawError, Frame, Program, Surface, VertexBuffer,
};
use itertools::Itertools;
use tap::Pipe;

#[derive(Copy, Clone)]
struct Vertex2 {
    position: [f32; 2],
}

implement_vertex!(Vertex2, position);

const VERTEX_SHADER: &'static str = r#"
#version 140

in vec2 position;

uniform vec2 scale_factor;

void main() {
    gl_Position = vec4(position * scale_factor, 0.0, 1.0);
}
"#;

const FRAGMENT_SHADER: &'static str = r#"
#version 140

out vec4 color;

void main() {
    color = vec4(1.0, 1.0, 1.0, 1.0);
}
"#;

pub struct DrawProgram {
    program: Program,
    grid: VertexBuffer<Vertex2>,
}

impl DrawProgram {
    pub fn new(display: &Display) -> Self {
        let vbuffer = (0..10)
            .cartesian_product(0..20)
            .flat_map(|(x, y)| {
                [
                    (x, y),
                    (x + 1, y),
                    (x + 1, y),
                    (x + 1, y + 1),
                    (x + 1, y + 1),
                    (x, y + 1),
                    (x, y + 1),
                    (x, y),
                ]
            })
            .map(|(x, y)| Vertex2 {
                position: [x as f32, y as f32],
            })
            .collect::<Vec<_>>();

        Self {
            program: Program::from_source(display, VERTEX_SHADER, FRAGMENT_SHADER, None).unwrap(),
            grid: VertexBuffer::immutable(display, &vbuffer).unwrap(),
        }
    }
    pub fn draw_grid(&self, frame: &mut Frame) -> Result<(), DrawError> {
        let (win_x, win_y) = frame.get_dimensions().pipe(|(x, y)| (x as f32, y as f32));
        let rect_ratio = win_x / win_y;
        let screen_ratio = 50. / win_x;

        frame.draw(
            &self.grid,
            &NoIndices(PrimitiveType::LinesList),
            &self.program,
            &uniform! {scale_factor: [screen_ratio, screen_ratio * rect_ratio]},
            &Default::default(),
        )
    }
}