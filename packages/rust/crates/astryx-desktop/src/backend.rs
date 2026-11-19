//! Which renderer draws the window: the GPU renderer through OpenGL, or the
//! CPU renderer presented as plain pixels.
//!
//! Where OpenGL itself runs on the CPU (Mesa's llvmpipe on a machine with
//! no GPU, a VM or CI), the CPU renderer is faster to start, faster to draw
//! a change, and uses far less memory, so the shell uses it there. The
//! choice is made before any GL library loads.
//!
//! On an X server with a GPU, OpenGL takes about 150 ms to start (driver
//! loading, context creation, the first make-current), so the CPU renderer
//! draws the first frames while OpenGL starts on a thread of its own, which
//! then draws every frame (see `gl_thread`).

/// The renderer that draws the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Backend {
    /// OpenGL on the main thread from the first frame.
    Gl,
    /// The CPU renderer first, then OpenGL on its own thread (X11 only).
    CpuFirst,
    Cpu,
}

/// What the choice depends on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Facts {
    /// `ASTRYX_RENDERER`: `gl`, `cpu-first` or `cpu` chooses outright.
    pub choice: Option<String>,
    /// The window opens on an X server.
    pub on_x11: bool,
    /// `LIBGL_ALWAYS_SOFTWARE` is on: Mesa draws GL on the CPU.
    pub mesa_software: bool,
    /// On X11, what the server offers.
    pub x11: Option<X11>,
}

/// The X server's GPU extensions. An X server that draws with a GPU offers
/// DRI3 (Mesa drivers) or NV-GLX (the NVIDIA driver); one without either,
/// such as Xvfb or a VM's, has only software GL.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct X11 {
    pub dri3: bool,
    pub nv_glx: bool,
}

/// The renderer for `facts`.
pub(crate) fn choose(facts: &Facts) -> Backend {
    // With GL: the CPU renderer first where OpenGL can start on its own
    // thread.
    let gl = if facts.on_x11 {
        Backend::CpuFirst
    } else {
        Backend::Gl
    };
    match facts.choice.as_deref().map(str::trim) {
        Some(c) if c.eq_ignore_ascii_case("cpu") => return Backend::Cpu,
        Some(c) if c.eq_ignore_ascii_case("gl") => return Backend::Gl,
        Some(c) if c.eq_ignore_ascii_case("cpu-first") => return gl,
        Some("") | None => {}
        Some(other) => {
            eprintln!(
                "astryx-desktop: ASTRYX_RENDERER={other:?} is not gl, cpu-first or cpu; ignored"
            );
        }
    }
    if facts.mesa_software {
        return Backend::Cpu;
    }
    match facts.x11 {
        Some(x) if !x.dri3 && !x.nv_glx => Backend::Cpu,
        _ => gl,
    }
}

/// Whether `choice` chooses outright.
fn decides(choice: &str) -> bool {
    ["gl", "cpu", "cpu-first"]
        .iter()
        .any(|c| choice.trim().eq_ignore_ascii_case(c))
}

/// Whether an environment variable is set to a true value, as Mesa reads
/// `LIBGL_ALWAYS_SOFTWARE`.
fn env_on(value: Option<std::ffi::OsString>) -> bool {
    value.is_some_and(|v| {
        let v = v.to_string_lossy();
        let v = v.trim();
        !(v.is_empty()
            || v == "0"
            || v.eq_ignore_ascii_case("false")
            || v.eq_ignore_ascii_case("no"))
    })
}

/// The facts on this machine. `on_x11` says whether the window opens on an
/// X server; only then is the server asked.
pub(crate) fn facts(on_x11: bool) -> Facts {
    let choice = std::env::var("ASTRYX_RENDERER").ok();
    let mesa_software = env_on(std::env::var_os("LIBGL_ALWAYS_SOFTWARE"));
    // An explicit choice needs no server round trip.
    let decided = choice.as_deref().is_some_and(decides);
    let x11 = if on_x11 && !decided && !mesa_software {
        x11_extensions()
    } else {
        None
    };
    Facts {
        choice,
        on_x11,
        mesa_software,
        x11,
    }
}

/// Asks the X server named by `DISPLAY` for its GPU extensions: one
/// connection and two queries, well under a millisecond. `None` when it
/// cannot be asked, which leaves the choice to GL.
#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
fn x11_extensions() -> Option<X11> {
    use x11rb::connection::RequestConnection;
    let (conn, _) = x11rb::connect(None).ok()?;
    let has = |name: &'static str| conn.extension_information(name).ok().flatten().is_some();
    Some(X11 {
        dri3: has("DRI3"),
        nv_glx: has("NV-GLX"),
    })
}

#[cfg(not(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
)))]
fn x11_extensions() -> Option<X11> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn x11(dri3: bool, nv_glx: bool) -> Option<X11> {
        Some(X11 { dri3, nv_glx })
    }

    fn on_x11(x11: Option<X11>) -> Facts {
        Facts {
            on_x11: true,
            x11,
            ..Facts::default()
        }
    }

    #[test]
    fn software_only_x_servers_draw_on_the_cpu() {
        // Xvfb, a VM's X server: neither extension.
        assert_eq!(choose(&on_x11(x11(false, false))), Backend::Cpu);
        // Mesa on a GPU, and the NVIDIA driver, keep GL, started behind the
        // CPU renderer's first frames.
        for (dri3, nv_glx) in [(true, false), (false, true), (true, true)] {
            let gpu = on_x11(x11(dri3, nv_glx));
            assert_eq!(
                choose(&gpu),
                Backend::CpuFirst,
                "DRI3 {dri3}, NV-GLX {nv_glx}"
            );
        }
        // The server could not be asked: GL, still behind the CPU renderer.
        assert_eq!(choose(&on_x11(None)), Backend::CpuFirst);
        // Not on X11: GL from the first frame.
        assert_eq!(choose(&Facts::default()), Backend::Gl);
    }

    #[test]
    fn mesa_software_gl_draws_on_the_cpu() {
        let facts = Facts {
            mesa_software: true,
            ..on_x11(x11(true, false))
        };
        assert_eq!(choose(&facts), Backend::Cpu);
        assert!(env_on(Some("1".into())));
        assert!(env_on(Some("true".into())));
        assert!(!env_on(Some("0".into())));
        assert!(!env_on(Some("".into())));
        assert!(!env_on(Some("false".into())));
        assert!(!env_on(None));
    }

    #[test]
    fn astryx_renderer_chooses_outright() {
        let cpu = Facts {
            choice: Some("cpu".into()),
            ..on_x11(x11(true, true))
        };
        assert_eq!(choose(&cpu), Backend::Cpu);
        // gl: OpenGL on the main thread from the first frame, as before
        // the CPU renderer came first.
        let gl = Facts {
            choice: Some(" GL ".into()),
            mesa_software: true,
            ..on_x11(x11(false, false))
        };
        assert_eq!(choose(&gl), Backend::Gl);
        let first = Facts {
            choice: Some("cpu-first".into()),
            ..on_x11(x11(false, false))
        };
        assert_eq!(choose(&first), Backend::CpuFirst);
        // Not on X11 the CPU renderer cannot come first: GL.
        let first_elsewhere = Facts {
            choice: Some("cpu-first".into()),
            ..Facts::default()
        };
        assert_eq!(choose(&first_elsewhere), Backend::Gl);
        // Anything else is ignored.
        let other = Facts {
            choice: Some("vulkan".into()),
            ..on_x11(x11(false, false))
        };
        assert_eq!(choose(&other), Backend::Cpu);
    }
}
