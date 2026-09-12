# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`libDQG` is a personal 2D/3D game framework in Rust, built on `wgpu` + `winit`. It's a hobby
successor to an earlier DirectX 9 framework, deliberately moving away from OOP/inheritance
toward data-driven design. It is not meant to be a general-purpose production engine — favor
simplicity and directness over extensibility when extending it.

Workspace layout (`Cargo.toml` at the root, resolver "3"):
- `core/` — the `libdqg` library crate (the framework itself).
- `demo/` — a `demo` binary crate exercising the framework; treat it as the framework's own
  integration test/sample game, not a real product.

## Common commands

```bash
cargo build                 # build everything
cargo build -p libdqg        # build only the core library
cargo run -p demo            # run the demo game
cargo test                  # run all tests (unit tests live inline in core/src, e.g. transform.rs)
cargo test -p libdqg transform::tests::rotation_holds_the_pivot_fixed   # run a single test
```

CI (`.github/workflows/builds.yml`) runs `cargo build --verbose` and `cargo test --verbose` on
Ubuntu for pushes/PRs to `master`. Keep changes buildable on that target in mind; the crate is
also expected to work on Windows (see `windows-sys` conditional dependency in `core/Cargo.toml`).

`demo/build.rs` calls `libdqg::copy_res_to_output_dir()`, which copies `demo/res/*` next to the
built executable so the demo can load assets via plain relative paths (`./res/...`) regardless of
the working directory `cargo run` was invoked from.

## Architecture

### Game loop: App → SceneManager → Scene

- [core/src/game.rs](core/src/game.rs) — `GameBuilder` configures window title/size/titlebar/clear
  color and builds a `Game`, which owns a `winit::EventLoop` and runs it against `App`.
- [core/src/app.rs](core/src/app.rs) — `App` implements `winit::ApplicationHandler`. It owns the
  `Window`, the `Renderer`, `InputState`, and the `SceneManager`, and drives the frame loop:
  `RedrawRequested` → `SceneManager::update` → `Renderer::render(clear_color, |pass| { scene.render(pass) })`.
  Also handles custom window-chrome logic (drag-resize borders, integrated titlebar hit-testing)
  when `integrated_titlebar` is enabled, since decorations are turned off in that mode.
- [core/src/scene.rs](core/src/scene.rs) — user code implements the `Scene` trait
  (`update` + `render`); `update` returns a `SceneTransition` (`Push`/`Pop`/`Replace`/`Next`/
  `Previous`/`Quit`/`None`) that `SceneManager` acts on. Scenes are a flat `Vec` with a current
  index, not a stack machine with separate push/pop semantics for render vs. update — read
  `SceneManager::update` before assuming transition behavior.
- Resource loading (textures, models) happens lazily inside `Scene::update`, gated on
  `renderer: Option<&mut Renderer>` being `Some` — see the `=== LOADING ===` pattern in
  [demo/src/main.rs](demo/src/main.rs). The renderer is only available once the window/GPU
  surface exists (after `resumed`), so scenes must tolerate `None` on early frames.

### Renderer: a thin wgpu wrapper + higher-level draw helpers

- [core/src/renderer/mod.rs](core/src/renderer/mod.rs) — `Renderer` owns the wgpu device/queue/
  surface, the camera uniform buffer, the depth buffer, and every built-in pipeline (immediate
  shapes, screen-space sprites, world-space sprites, models). `Renderer::render` begins the wgpu
  render pass (color + depth attachments) and hands a `DrawPass` into a caller-supplied closure —
  this is the only place a frame is actually submitted.
- `Renderer` also exposes a **generic pipeline-building API**
  (`create_shader_module`, `create_render_pipeline`, `create_buffer`, `create_buffer_init`) built
  on wrapper types in [core/src/renderer/types.rs](core/src/renderer/types.rs)
  (`VertexFormat`, `PrimitiveState`, `DepthStencilState`, etc.). These wrapper enums exist so
  downstream crates (like `demo`) can describe custom pipelines without depending on `wgpu`
  directly — every wrapper type has a `to_wgpu()`/`from_wgpu()` conversion. Any custom pipeline
  built this way that participates in the main pass must declare a depth-stencil state using
  `Renderer::DEPTH_FORMAT` (`Depth32Float`) or it falls back to a no-op overlay depth state.
- [core/src/renderer/draw_pass.rs](core/src/renderer/draw_pass.rs) — `DrawPass` wraps
  `wgpu::RenderPass` with the same wrapper-type philosophy (`set_index_buffer`, `set_bind_group`,
  etc. take the crate's own enums, not raw wgpu ones). Higher-level draw calls
  (`draw_rect`/`draw_ellipse`/`draw_line`/`draw_ui_sprite`/`draw_world_sprite`/`draw_model`) are
  added via `impl DrawPass` blocks split across [extras.rs](core/src/renderer/extras.rs) (immediate
  shapes + sprites) and [model.rs](core/src/renderer/model.rs) (OBJ models).
- Two coordinate spaces coexist per frame: **screen space** (pixel coordinates, e.g.
  `draw_rect`, `Sprite::draw`, the titlebar) uses depth-untested overlay rendering so draw order
  wins; **world space** (`draw_world_sprite`, `draw_model`, camera-relative) is depth-tested
  against the shared `Depth32Float` buffer. Mixing them means being deliberate about which `draw_*`
  method / pipeline you're using — it's not inferred from the data.
- Shaders are plain `.wgsl` files under [core/shaders/](core/shaders/), pulled in via
  `include_str!` in [extras.rs](core/src/renderer/extras.rs) — editing shader behavior means
  editing the `.wgsl` file, not Rust code.

### Transforms

[core/src/transform.rs](core/src/transform.rs) defines `Transformable`, a default-method trait
giving any type with a `glam::Mat4` (`Sprite`, `Model`) a fluent transform API
(`translate`/`rotate`/`scale`/`flip_x`/...). Rotations and scales compose about
`Transformable::pivot()` (defaults to local origin; `Sprite` overrides it to its quad center so
sprites spin in place). All operations post-multiply onto the existing matrix in local space —
chained calls read outside-in (e.g. `rotate(...).translate(...)` translates along the *rotated*
axis). `set_transform`/`reset_transform` bypass composition entirely when you want to drive the
matrix directly.

### Other core modules

- [core/src/camera.rs](core/src/camera.rs) — `Camera` (eye/target/up/fov) builds a right-handed
  view-projection matrix via `glam`; `CameraUniform` mirrors it into the GPU buffer each frame.
- [core/src/input.rs](core/src/input.rs) — `InputState` tracks per-frame key press/hold state from
  winit keyboard events; `App` clears frame-transient state after each `RedrawRequested`.
- [core/src/titlebar.rs](core/src/titlebar.rs) / [core/src/platform.rs](core/src/platform.rs) —
  custom-drawn titlebar and Windows-specific chrome (rounded corners via `windows-sys`) used when
  `integrated_titlebar(true)`.
- [core/src/util.rs](core/src/util.rs) — `resolve_resource_path` (exe-relative asset lookup
  fallback) and `slice_to_bytes` (raw byte view of a `&[T]` for uploading to GPU buffers).
- `libdqg` re-exports `glam` (`pub use glam;`) so downstream crates don't need to pin their own
  version to stay compatible with types like `Sprite::transform`.
