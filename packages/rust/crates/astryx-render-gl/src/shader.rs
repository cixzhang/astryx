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
    // Fragments well inside a shape, away from its corners, skip the
    // distance math: most of a large box is interior.
    float rmax = max(max(v_radii.x, v_radii.y), max(v_radii.z, v_radii.w));
    vec2 depth = v_half - abs(v_local) - rmax;
    bool interior = min(depth.x, depth.y) > 0.5;
    if (mode < 0.5) {
        if (!interior) {
            cov = clamp(0.5 - sd(v_local, v_half, v_radii), 0.0, 1.0);
        }
    } else if (mode < 1.5) {
        float w = v_params.y;
        if (interior && min(depth.x, depth.y) > w + 0.5) {
            cov = 0.0;
        } else {
            float outer = sd(v_local, v_half, v_radii);
            float inner = sd(v_local, v_half - w, max(v_radii - w, 0.0));
            cov = clamp(0.5 - outer, 0.0, 1.0) * clamp(0.5 + inner, 0.0, 1.0);
        }
    } else if (mode < 2.5) {
        cov = TEX(u_tex, v_uv).r;
    } else if (mode < 3.5) {
        color = TEX(u_tex, v_uv) * v_color.a;
    } else {
        color = TEX(u_tex, v_uv) * v_color.a;
        cov = clamp(0.5 - sd(v_local, v_half, v_radii), 0.0, 1.0);
    }
    float clip = 1.0;
    if (v_params.z > 0.5) {
        clip = clamp(0.5 - sd(v_pos - v_clip.xy, v_clip.zw, v_clip_radii), 0.0, 1.0);
    }
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
    let program = link(gl, dialect, VERTEX, FRAGMENT, &ATTRIBUTES)?;
    Ok(Program {
        size: gl.get_uniform_location(program, "u_size"),
        origin: gl.get_uniform_location(program, "u_origin"),
        tex: gl.get_uniform_location(program, "u_tex"),
        program,
    })
}

/// Attributes of the light map program, each one vec4 slot.
pub const LIGHT_ATTRIBUTES: [&str; 3] = ["a_pos_center", "a_radii_kind", "a_color"];

const LIGHT_VERTEX: &str = r#"
ATTR vec4 a_pos_center;
ATTR vec4 a_radii_kind;
ATTR vec4 a_color;
uniform vec2 u_size;
VARY vec2 v_pos;
VARY vec2 v_center;
VARY vec3 v_radii_kind;
VARY vec3 v_color;
void main() {
    vec2 p = a_pos_center.xy;
    gl_Position = vec4(p.x / u_size.x * 2.0 - 1.0, 1.0 - p.y / u_size.y * 2.0, 0.0, 1.0);
    v_pos = p;
    v_center = a_pos_center.zw;
    v_radii_kind = a_radii_kind.xyz;
    v_color = a_color.rgb;
}
"#;

// Matches astryx_raster::light: the base mixed with the banded dapple, or
// a light's color in four bands of (1 - d)^2. Positions are target pixels.
const LIGHT_FRAGMENT: &str = r#"
VARY vec2 v_pos;
VARY vec2 v_center;
VARY vec3 v_radii_kind;
VARY vec3 v_color;
uniform sampler2D u_lattice;
uniform float u_divisor;
// Inverse cell size in target pixels, offset in cells (x, y), bands.
uniform vec4 u_dapple;
// Depth times strength; 0 without dapple.
uniform float u_dapple_depth;
float lattice(vec2 cell) {
    return TEX(u_lattice, (cell + 0.5) / 16.0).r;
}
void main() {
    vec2 p = v_pos * u_divisor;
    if (v_radii_kind.z < 0.5) {
        float shade = 1.0;
        if (u_dapple_depth > 0.0) {
            vec2 uv = p * u_dapple.x + u_dapple.yz;
            vec2 i = floor(uv);
            vec2 f = uv - i;
            vec2 s = f * f * (3.0 - 2.0 * f);
            float top = mix(lattice(i), lattice(i + vec2(1.0, 0.0)), s.x);
            float bottom = mix(lattice(i + vec2(0.0, 1.0)), lattice(i + vec2(1.0, 1.0)), s.x);
            float n = mix(top, bottom, s.y);
            float bands = u_dapple.w;
            float band = min(floor(n * bands), bands - 1.0) / (bands - 1.0);
            shade = 1.0 - u_dapple_depth * (1.0 - band);
        }
        FRAG = vec4(v_color * shade, 1.0);
    } else {
        vec2 d = (p - v_center) / v_radii_kind.xy;
        float f = max(1.0 - length(d), 0.0);
        f = f * f;
        float q = f > 0.0 ? ceil(f * 4.0) / 4.0 : 0.0;
        FRAG = vec4(v_color * q, 0.0);
    }
}
"#;

pub struct LightProgram {
    pub program: glow::Program,
    pub size: Option<glow::UniformLocation>,
    pub lattice: Option<glow::UniformLocation>,
    pub divisor: Option<glow::UniformLocation>,
    pub dapple: Option<glow::UniformLocation>,
    pub dapple_depth: Option<glow::UniformLocation>,
}

/// Compiles and links the light map program.
///
/// # Safety
/// `gl` must be current.
pub unsafe fn compile_light(gl: &glow::Context, dialect: Dialect) -> Result<LightProgram, String> {
    let program = link(gl, dialect, LIGHT_VERTEX, LIGHT_FRAGMENT, &LIGHT_ATTRIBUTES)?;
    Ok(LightProgram {
        size: gl.get_uniform_location(program, "u_size"),
        lattice: gl.get_uniform_location(program, "u_lattice"),
        divisor: gl.get_uniform_location(program, "u_divisor"),
        dapple: gl.get_uniform_location(program, "u_dapple"),
        dapple_depth: gl.get_uniform_location(program, "u_dapple_depth"),
        program,
    })
}

/// Attributes of the transition program: one vec4 slot.
pub const TRANSITION_ATTRIBUTES: [&str; 1] = ["a_pos"];

const TRANSITION_VERTEX: &str = r#"
ATTR vec4 a_pos;
uniform vec2 u_size;
VARY vec2 v_pos;
void main() {
    gl_Position = vec4(a_pos.x / u_size.x * 2.0 - 1.0, 1.0 - a_pos.y / u_size.y * 2.0, 0.0, 1.0);
    v_pos = a_pos.xy;
}
"#;

// Matches astryx_raster::transition::Moment::at. The renderer works out
// everything that is the same for every pixel; this compares and samples.
const TRANSITION_FRAGMENT: &str = r#"
VARY vec2 v_pos;
uniform sampler2D u_old;
uniform sampler2D u_new;
uniform sampler2D u_lattice;
uniform vec2 u_size;
// Target size over texture size.
uniform vec2 u_extent;
uniform float u_kind;
uniform float u_p;
// Per kind: wipe (edge, limit); door (x0, y0, x1, y1); iris (cx, cy, r);
// slide (dx, dy); dissolve (cell); swirl (cx, cy, far); encounter
// (white, bars).
uniform vec4 u_param;
vec4 read(sampler2D t, vec2 p) {
    vec2 cell = clamp(floor(p), vec2(0.0), u_size - 1.0) + 0.5;
    return TEX(t, vec2(cell.x / u_size.x * u_extent.x, (u_size.y - cell.y) / u_size.y * u_extent.y));
}
void main() {
    vec2 p = v_pos;
    float n = 0.0;
    vec2 off = vec2(0.0);
    vec2 q = p;
    float white = 0.0;
    float black = 0.0;
    if (u_kind < 0.5) {
        n = u_p;
    } else if (u_kind < 1.5) {
        float e = u_param.x;
        float l = u_param.y;
        bool inside = e < 0.5 ? p.x < l : e < 1.5 ? p.x > l : e < 2.5 ? p.y < l : p.y > l;
        n = inside ? 1.0 : 0.0;
    } else if (u_kind < 2.5) {
        n = (p.x > u_param.x && p.x < u_param.z && p.y > u_param.y && p.y < u_param.w) ? 1.0 : 0.0;
    } else if (u_kind < 3.5) {
        n = length(p - u_param.xy) < u_param.z ? 1.0 : 0.0;
    } else if (u_kind < 4.5) {
        off = -u_param.xy;
        vec2 s = p - u_param.xy;
        n = (s.x >= 0.0 && s.y >= 0.0 && s.x < u_size.x && s.y < u_size.y) ? u_p : 0.0;
    } else if (u_kind < 5.5) {
        vec2 i = floor(p / u_param.x);
        float threshold = (TEX(u_lattice, (i + 0.5) / 16.0).r * 255.0 + 0.5) / 256.0;
        n = threshold < u_p ? 1.0 : 0.0;
    } else if (u_kind < 6.5) {
        vec2 c = u_param.xy;
        float a = u_p * 3.14159265 * max(1.0 - length(p - c) / u_param.z, 0.0);
        vec2 d = p - c;
        q = c + vec2(d.x * cos(a) - d.y * sin(a), d.x * sin(a) + d.y * cos(a));
        n = u_p;
    } else {
        white = u_param.x;
        black = (p.y < u_param.y || p.y > u_size.y - u_param.y) ? 1.0 : 0.0;
    }
    vec4 a = read(u_old, q);
    vec4 b = n > 0.0 ? read(u_new, p + off) : vec4(0.0);
    vec4 c = mix(a, b, n);
    c = mix(c, vec4(1.0), white);
    c = mix(c, vec4(0.0, 0.0, 0.0, 1.0), black);
    FRAG = c;
}
"#;

pub struct TransitionProgram {
    pub program: glow::Program,
    pub size: Option<glow::UniformLocation>,
    pub extent: Option<glow::UniformLocation>,
    pub old: Option<glow::UniformLocation>,
    pub new: Option<glow::UniformLocation>,
    pub lattice: Option<glow::UniformLocation>,
    pub kind: Option<glow::UniformLocation>,
    pub p: Option<glow::UniformLocation>,
    pub param: Option<glow::UniformLocation>,
}

/// Compiles and links the transition program.
///
/// # Safety
/// `gl` must be current.
pub unsafe fn compile_transition(
    gl: &glow::Context,
    dialect: Dialect,
) -> Result<TransitionProgram, String> {
    let program = link(
        gl,
        dialect,
        TRANSITION_VERTEX,
        TRANSITION_FRAGMENT,
        &TRANSITION_ATTRIBUTES,
    )?;
    let at = |name: &str| gl.get_uniform_location(program, name);
    Ok(TransitionProgram {
        size: at("u_size"),
        extent: at("u_extent"),
        old: at("u_old"),
        new: at("u_new"),
        lattice: at("u_lattice"),
        kind: at("u_kind"),
        p: at("u_p"),
        param: at("u_param"),
        program,
    })
}

unsafe fn link(
    gl: &glow::Context,
    dialect: Dialect,
    vertex: &str,
    fragment: &str,
    attributes: &[&str],
) -> Result<glow::Program, String> {
    let program = gl.create_program()?;
    let mut shaders = Vec::new();
    for (kind, body, fragment) in [
        (glow::VERTEX_SHADER, vertex, false),
        (glow::FRAGMENT_SHADER, fragment, true),
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
    for (i, name) in attributes.iter().enumerate() {
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
    Ok(program)
}
