# Astryx Rust — architecture

Astryx components and systems in Rust, drawn by the app itself, so one app
runs on desktop (macOS, Windows, Linux), on the web (WebAssembly), and in
headless tests. Any screen with OpenGL ES, WebGL2, or only a CPU can host it.
Device-specific work (handhelds, kiosks) lives outside this package and only
depends on it.

## Principles

1. **Astryx first.** Tokens, component names, props, states, and behavior come
   from Astryx's own sources: themes, `{Name}.doc.mjs`, component specs, and
   the native contracts in `packages/native`. When Astryx says nothing, ask the
   orchestrator. Do not invent a new shared API.
2. **Portable core.** `astryx-core` has no GPU, window, or OS dependency. It
   builds for `wasm32-unknown-unknown` and every desktop target.
3. **Pixels and layout agree.** Each renderer supplies the `TextMeasure` that
   layout uses.
4. **Accessible by construction.** Every frame produces an `A11yNode` tree.
   Shells bridge it to the platform: AccessKit on desktop, a DOM mirror on the web.
5. **Input is focus-first.** Keyboard and gamepad focus with spatial
   navigation is the baseline. Pointer and touch are added on top later.

## Tokens

`tokens/themes.json` holds every published Astryx theme, light and dark,
exported by `tools/export-tokens.py` from the generated native themes in
`packages/native/apple/Sources/Astryx/Theme/Generated/`. Shape:

```
{ "<theme>": { "light"|"dark": {
    "color":  { "<member>": {"rgba": [r, g, b, a]} },      // 0..1 sRGB
    "space":  { "s0".."s12": points },
    "radius": { "none","inner","element","container","page","chat","full": points },
    "size":   { "borderWidth","elementSm","elementMd","elementLg": points },
    "font":   { "body"|"label"|"heading1".."6"|"display1".."3"|"large"|"supporting"|"code":
                  {"size","weight","leading"}, "family": {...}, "size": {...} },
    "shadow": { "low"|"med"|"high": [ {"x","y","blur","spread"?,"isInset"?,"color"} ] } } } }
```

Themes: astryx (default), neutral, stone, butter, chocolate, gothic, matcha, y2k.
Never edit `themes.json` by hand. Member names match the Swift `Tokens` surface.

## Crates

```
crates/astryx-tokens        build.rs turns themes.json into typed Rust          (Lane A)
crates/astryx-core          contract.rs (shared, orchestrator-owned), theme,
                            layout (taffy), elements, focus, runtime, a11y tree,
                            components                                          (Lane A)
crates/astryx-motion        motion tokens (tokens/motion.json), easing, springs,
                            Animated, enter/exit, reduced motion, gestures      (Lane D)
crates/astryx-image         PNG/JPEG/WebP decoding and downscaling              (Lane D)
crates/astryx-raster        shared text engine, pixel snapping, shadow masks    (Lane B)
crates/astryx-render-gl     batched GL renderer on glow: OpenGL ES 2/3, WebGL2   (Lane B)
crates/astryx-render-soft   CPU renderer (tiny-skia) for headless and no-GPU    (Lane B)
crates/astryx-audio         WAV decoding and a small voice mixer for shells      (Lane B)
crates/astryx-desktop       shell: winit + glutin + accesskit_winit, cpal sound (Lane B)
crates/astryx-headless      shell: Surface -> PNG, scripted input, golden tests (Lane B)
crates/astryx-theme-pixel   retro pixel theme: 5 palettes, pixel grid, Galmuri,   
                            bitmap icons (tokens/pixel-*.json)                 (Lane E)
crates/astryx-web           shell: wasm-bindgen + WebGL2 + DOM a11y mirror      (Lane C)
examples/gallery            one app showing every component, run by every shell
assets/fonts                Roboto Regular/Medium/Bold (Apache 2.0)
tools/                      token export and helper scripts
```

## The contract

`crates/astryx-core/src/contract.rs` is the only coupling between lanes:

- `DisplayList` / `DrawCmd`: what components paint and renderers draw.
- `TextMeasure` and `Renderer`: renderers implement both.
- `Input`, `InputPhase`, `FrameInfo`, `Command`: what shells send and receive.
- `Surface`: what a shell drives each frame. The core runtime implements it
  for any `App`; shells and renderers never see app or element types.
- `A11yNode`, `Role`: the accessibility tree.
- Images: `DrawCmd::Image`, `Renderer::set_image`, and `Surface::take_image_updates`.
  A `Surface` wrapper must forward every `Surface` method, including the ones
  with defaults; a missed `take_image_updates` drops images without an error.

- Light, transitions, sound, and the person's state, below.

### Light

A light pass darkens a scene and lights it again. `DrawCmd::PushLighting(Lighting)`
opens it, and `DrawCmd::PopLighting` closes it. `DrawCmd::Light(Light)` adds a light
inside the pass. `DrawCmd::Emissive { rect, color }` draws after the light map, so
it is never darkened. Commands after `PopLighting`, such as the UI, are not lit.
Passes do not nest. Transforms move lights and emissive rects like any shape.

- `Lighting { ambient, strength, time, dapple: Option<Dapple> }`. The light map
  starts at white mixed toward `ambient` by `strength`; 0 skips the pass.
- `Dapple { cell, velocity, bands, depth }`: leaf shade, a tiling noise pattern
  in 2 or 3 flat bands, scrolled by `velocity * time`.
- `Light { center, radius_x, radius_y, color, intensity, flicker: Flicker, pulse: Pulse }`:
  an ellipse whose falloff is `(1 - d)^2` in four flat bands.
  `Flicker { amplitude, hz }`, `Pulse { period, min }`, and `Light::intensity_at(time)`.
- The map is a quarter of the target's resolution, upsampled nearest-neighbor
  and multiplied over the scene. The math lives in `astryx-raster::light`, so
  the GL and CPU renderers match.
- Elements, experimental, in `astryx_core::world`: `Lit::new(lighting)` lights its
  children and takes its time from the runtime, which holds still under
  reduced motion. `LightSource::new(light)` is a zero-size light that layout
  places, so a `Board`, the camera, and glides carry it with its sprite.
  `Emissive::new(color).size(w, h)` is a glowing rect. Lights out of view are
  not painted.

### Day cycle

- `Preferences.day_phase: DayPhase { Day, Dusk, Night }` is for coarse
  choices, such as a dark scheme or lanterns on.
- `Preferences.day_time: Option<f32>` is minutes after local midnight.
  Shells with a clock set it every minute; the web leaves `None`.
- `Lighting::at(minutes, time)` blends night, dawn, day, golden hour, sunset,
  and dusk from Terrace palette tints, in eased steps too small to see.
  `None` means daylight.
- `Lighting::lamp_level(minutes)` fades lamps in through dusk and out
  through dawn.
- `Lighting::for_phase(phase, time)` is the older three-step version.

### Transitions

- `Command::Transition(Transition { kind, duration_ms, easing, sound })` shows
  the change to the next frame as a transition from the frame on screen.
- `TransitionKind`: `Fade`, `Wipe(Edge)`, `Door`, `Iris(Point)`,
  `Slide(Point)`, `PixelDissolve(cell)`, `Swirl`, `Encounter`.
- `Easing`: `Linear`, `EaseIn`, `EaseOut`, `Steps(n)`, `Overshoot`, with
  `Easing::apply(t)`.
- Shells keep the last `DisplayList` and call
  `Renderer::begin_transition(&t, &old, frame)`. They keep drawing while
  `Renderer::transitioning()`.
- The GL renderer composites both frames in one shader. It keeps two
  frame-sized textures ready (`GlRenderer::prepare_transitions`). The CPU
  renderer does every kind with the same math (`astryx-raster::transition`).
- Under reduced motion, shells show the new frame at once or a quick fade.

### Sound

- `Command::PlaySound(SoundId)` plays a sound once.
- `Runtime::register_sound(id, wav)` registers one, and shells drain
  `Surface::take_sound_updates()`.
- `Preferences.sound_off` silences everything.
- `astryx-audio` decodes WAV files (8- or 16-bit PCM, 32-bit float) and mixes
  up to four voices. A `Controller` on the UI thread feeds a `Mixer` that the
  audio thread pulls from.
- Desktop plays through cpal (default feature `audio`). The web uses WebAudio
  (feature `audio`, off by default). The Brick shell uses SDL.

### Gamepad face buttons

Every shell names face buttons by their printed label: `Input::A` is the
button printed A, wherever it sits. On an Xbox-style pad A is at the
bottom, B right, X left, and Y top; on a Nintendo-style pad, such as the
TrimUI Brick, A is right, B bottom, X top, and Y left. The Brick shell reads
the labels from its input table. The desktop shell gets places from gilrs
and swaps A with B, and X with Y, for Nintendo pads (USB vendor 0x057e); the
web shell does the same for pads whose id names Nintendo. Each shell also
sets `Preferences::face_layout`: the Brick always `Nintendo`, desktop and
web from the pad that last sent input (`Unknown` until one does).
Components that draw buttons by place, such as the on-screen keyboard's
diamonds, read it, and treat `Unknown` as `Nintendo`.

### Idle

- `Preferences.idle: Idle { Active, Dim, Asleep }` tells apps how long the
  person has been away.
- Shells that save power set it; the web and desktop leave `Active`.
- The Brick shell dims to about 8 frames a second after 30 seconds without
  input. It stops drawing after 2 minutes, and any input wakes it at once.

App model (Lane A owns it): Elm style. `view(&self) -> Element<Msg>`,
`update(&mut self, msg) -> Command`. An app is a library crate; each shell
runs it with one call, with no `cfg` in app code.

## Rules for every lane

- Work in your own git worktree and branch (`lane/<letter>-<name>`). Commit
  early and often. Touch only the paths your lane owns.
- Changes to `contract.rs` go through the orchestrator: send the exact change
  and the reason, keep working with a local shim until it lands.
- Rust stable (1.98 or newer); dependencies from crates.io only.
- This is a public repository: no company-internal links, hostnames, or ids.
- Comments only for non-obvious reasons: short, plain, present-tense sentences.
- `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` pass before
  every handoff.
