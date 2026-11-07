//! The one shader program, compiled for whichever GL dialect is current.

use glow::HasContext;

/// The GL dialect the context speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    /// OpenGL ES 2 or WebGL 1: GLSL ES 1.00, no vertex arrays, LUMINANCE masks.
    Es2,
    /// OpenGL ES 3 or WebGL 2: GLSL ES 3.00.
    Es3,
    /// Desktop OpenGL 3.3 core: GLSL 3.30 and a required vertex array.
    Core,
}

impl Dialect {
    pub fn detect(gl: &glow::Context) -> Dialect {
        let v = gl.version();
        match (v.is_embedded, v.major) {
            (true, 2) | (true, 0..=1) => Dialect::Es2,
            (true, _) => Dialect::Es3,
            (false, 0..=2) => Dialect::Es2,
            (false, _) => Dialect::Core,
        }
    }

    fn header(self, fragment: bool) -> &'static str {
        match (self, fragment) {
            (Dialect::Es2, false) => "#version 100\n#define ATTR attribute\n#define VARY varying\n",
            (Dialect::Es2, true) => concat!(
                "#version 100\n#ifdef GL_FRAGMENT_PRECISION_HIGH\nprecision highp float;\n#else\nprecision mediump float;\n#endif\n",
                "#define VARY varying\n#define TEX texture2D\n#define FRAG gl_FragColor\n"
            ),
            (Dialect::Es3, false) => "#version 300 es\n#define ATTR in\n#define VARY out\n",
            (Dialect::Es3, true) => concat!(
                "#version 300 es\nprecision highp float;\n",
                "#define VARY in\n#define TEX texture\nout vec4 frag_color;\n#define FRAG frag_color\n"
            ),
            (Dialect::Core, false) => "#version 330 core\n#define ATTR in\n#define VARY out\n",
            (Dialect::Core, true) => {
                "#version 330 core\n#define VARY in\n#define TEX texture\nout vec4 frag_color;\n#define FRAG frag_color\n"
            }
        }
    }
}

/// Attribute names in binding order. Each is one vec4 slot.
pub const ATTRIBUTES: [&str; 7] = [
    "a_pos_local",
    "a_half_uv",
    "a_radii",
    "a_color",
    "a_params",
    "a_clip",
    "a_clip_radii",
];

const VERTEX: &str = r#"
ATTR vec4 a_pos_local;
ATTR vec4 a_half_uv;
ATTR vec4 a_radii;
ATTR vec4 a_color;
ATTR vec4 a_params;
ATTR vec4 a_clip;
ATTR vec4 a_clip_radii;
uniform vec2 u_size;
uniform vec2 u_origin;
VARY vec2 v_pos;
VARY vec2 v_local;
VARY vec2 v_half;
VARY vec2 v_uv;
VARY vec4 v_radii;
VARY vec4 v_color;
VARY vec4 v_params;
VARY vec4 v_clip;
VARY vec4 v_clip_radii;
void main() {
    vec2 p = a_pos_local.xy - u_origin;
    gl_Position = vec4(p.x / u_size.x * 2.0 - 1.0, 1.0 - p.y / u_size.y * 2.0, 0.0, 1.0);
    v_pos = a_pos_local.xy;
    v_local = a_pos_local.zw;
    v_half = a_half_uv.xy;
    v_uv = a_half_uv.zw;
    v_radii = a_radii;
    v_color = a_color;
    v_params = a_params;
    v_clip = a_clip;
    v_clip_radii = a_clip_radii;
}
"#;

// Coverage is 0.5 minus the signed distance at the pixel center, so edges
// on pixel boundaries come out exactly 0 or 1, as in the CPU renderer.
const FRAGMENT: &str = r#"
uniform sampler2D u_tex;
VARY vec2 v_pos;
VARY vec2 v_local;
VARY vec2 v_half;
VARY vec2 v_uv;
VARY vec4 v_radii;
VARY vec4 v_color;
VARY vec4 v_params;
VARY vec4 v_clip;
VARY vec4 v_clip_radii;
float sd(vec2 p, vec2 b, vec4 r) {
    float rad = p.x > 0.0 ? (p.y > 0.0 ? r.z : r.y) : (p.y > 0.0 ? r.w : r.x);
    vec2 q = abs(p) - b + rad;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - rad;
}
void main() {
    float mode = v_params.x;
    vec4 color = v_color;
    float cov = 1.0;
    if (mode < 0.5) {
        cov = clamp(0.5 - sd(v_local, v_half, v_radii), 0.0, 1.0);
    } else if (mode < 1.5) {
        float w = v_params.y;
        float outer = sd(v_local, v_half, v_radii);
        float inner = sd(v_local, v_half - w, max(v_radii - w, 0.0));
        cov = clamp(0.5 - outer, 0.0, 1.0) * clamp(0.5 + inner, 0.0, 1.0);
    } else if (mode < 2.5) {
        cov = TEX(u_tex, v_uv).r;
    } else {
        color = TEX(u_tex, v_uv) * v_color.a;
    }
    float clip = clamp(0.5 - sd(v_pos - v_clip.xy, v_clip.zw, v_clip_radii), 0.0, 1.0);
    FRAG = color * (cov * clip);
}
"#;

pub struct Program {
    pub program: glow::Program,
    pub size: Option<glow::UniformLocation>,
    pub origin: Option<glow::UniformLocation>,
    pub tex: Option<glow::UniformLocation>,
}

/// Compiles and links the program.
///
/// # Safety
/// `gl` must be current.
pub unsafe fn compile(gl: &glow::Context, dialect: Dialect) -> Result<Program, String> {
    let program = gl.create_program()?;
    let mut shaders = Vec::new();
    for (kind, body, fragment) in [
        (glow::VERTEX_SHADER, VERTEX, false),
        (glow::FRAGMENT_SHADER, FRAGMENT, true),
    ] {
        let shader = gl.create_shader(kind)?;
        gl.shader_source(shader, &format!("{}{}", dialect.header(fragment), body));
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            return Err(format!("shader: {}", gl.get_shader_info_log(shader)));
        }
        gl.attach_shader(program, shader);
        shaders.push(shader);
    }
    for (i, name) in ATTRIBUTES.iter().enumerate() {
        gl.bind_attrib_location(program, i as u32, name);
    }
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        return Err(format!("program: {}", gl.get_program_info_log(program)));
    }
    for shader in shaders {
        gl.detach_shader(program, shader);
        gl.delete_shader(shader);
    }
    Ok(Program {
        size: gl.get_uniform_location(program, "u_size"),
        origin: gl.get_uniform_location(program, "u_origin"),
        tex: gl.get_uniform_location(program, "u_tex"),
        program,
    })
}
