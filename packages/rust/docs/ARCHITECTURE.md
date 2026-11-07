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
crates/astryx-render-gl     femtovg on glow: OpenGL ES 2/3 and WebGL2           (Lane B)
crates/astryx-render-soft   CPU renderer (tiny-skia) for headless and no-GPU    (Lane B)
crates/astryx-desktop       shell: winit + glutin + accesskit_winit             (Lane B)
crates/astryx-headless      shell: Surface -> PNG, scripted input, golden tests (Lane B)
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
