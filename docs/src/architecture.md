# Architecture

This page describes how libDQG's pieces fit together. For the full API surface, see the
[API reference](api/index.html); this page is the map, not the territory.

## Workspace layout

The workspace root's `Cargo.toml` defines three crates:

- `core/` — the `libdqg` library crate (the framework itself).
- `demo/` — a `demo` binary crate exercising the framework; more an integration test and sample
  game than a real product.
- `editor/` — an `editor` binary crate: a visual scene editor built on `libdqg` (an
  [`egui`](https://github.com/emilk/egui)-based UI, its own project file format, mesh-accurate
  picking, and a `.rhai` scripting system).

`libdqg` re-exports [`glam`](https://docs.rs/glam) (`pub use glam;`) so downstream crates don't
need to pin their own version to stay compatible with types like `Sprite::transform`.

## Game loop: App → SceneManager → Scene

- `game.rs` — `GameBuilder` configures window title/size/titlebar/clear color and builds a `Game`,
  which owns a `winit::EventLoop` and runs it against `App`.
- `app.rs` — `App` implements `winit::ApplicationHandler`. It owns the `Window`, the `Renderer`,
  `InputState`, and the `SceneManager`, and drives the frame loop: `RedrawRequested` →
  `SceneManager::update` → `Renderer::render(clear_color, |pass| { scene.render(pass) })`. It also
  handles custom window-chrome logic (drag-resize borders, integrated titlebar hit-testing) when
  `integrated_titlebar` is enabled, since window decorations are turned off in that mode.
- `scene.rs` — user code implements the `Scene` trait (`update` + `render`); `update` returns a
  `SceneTransition` (`Push`/`Pop`/`Replace`/`Next`/`Previous`/`Quit`/`None`) that `SceneManager`
  acts on. Scenes are a flat `Vec` with a current index, not a stack machine with separate
  push/pop semantics for render vs. update.
- Resource loading (textures, models) happens lazily inside `Scene::update`, gated on
  `renderer: Option<&mut Renderer>` being `Some`. The renderer is only available once the
  window/GPU surface exists (after `resumed`), so scenes must tolerate `None` on early frames.

## Renderer: a thin wgpu wrapper + higher-level draw helpers

- `renderer/mod.rs` — `Renderer` owns the wgpu device/queue/surface, the camera uniform buffer,
  the depth buffer, and every built-in pipeline (immediate shapes, screen-space sprites,
  world-space sprites, models). `Renderer::render` begins the wgpu render pass (color + depth
  attachments) and hands a `DrawPass` into a caller-supplied closure — this is the only place a
  frame is actually submitted.
- `Renderer` also exposes a **generic pipeline-building API** (`create_shader_module`,
  `create_render_pipeline`, `create_buffer`, `create_buffer_init`) built on wrapper types in
  `renderer/types.rs` (`VertexFormat`, `PrimitiveState`, `DepthStencilState`, etc.). These wrapper
  enums exist so downstream crates (like `demo`) can describe custom pipelines without depending
  on `wgpu` directly — every wrapper type has a `to_wgpu()`/`from_wgpu()` conversion. Any custom
  pipeline built this way that participates in the main pass must declare a depth-stencil state
  using `Renderer::DEPTH_FORMAT` (`Depth32Float`) or it falls back to a no-op overlay depth state.
- `renderer/draw_pass.rs` — `DrawPass` wraps `wgpu::RenderPass` with the same wrapper-type
  philosophy. Higher-level draw calls (`draw_rect`/`draw_ellipse`/`draw_line`/`draw_ui_sprite`/
  `draw_world_sprite`/`draw_model`) live in `extras.rs` (immediate shapes + sprites) and
  `model.rs` (OBJ models).
- Two coordinate spaces coexist per frame: **screen space** (pixel coordinates, e.g. `draw_rect`,
  `Sprite::draw`, the titlebar) uses depth-untested overlay rendering so draw order wins; **world
  space** (`draw_world_sprite`, `draw_model`, camera-relative) is depth-tested against the shared
  `Depth32Float` buffer.
- Shaders are plain `.wgsl` files under `core/shaders/`, pulled in via `include_str!`.

### Camera

`Camera` (`camera.rs`) is an FPS-style camera: `position` plus `yaw`/`pitch` (up is fixed to world
`+Y`, so there's no roll), rather than an eye/target look-at pair. `Camera::forward()` derives the
look direction from yaw/pitch, and `Camera::look_at(target)` is a convenience for call sites that
think in terms of a point to look at rather than angles. `Renderer` owns exactly one `Camera`;
`Renderer::render` re-derives the GPU view-projection uniform from it every frame, so swapping
`*renderer.camera_mut() = ...` before calling `render` is the whole mechanism for changing what's
displayed — there's no separate multi-camera concept in the renderer itself. (The editor builds a
camera-*entity* concept in the ECS on top of this — see [The editor](#the-editor) below.)

## Transforms

`transform.rs` defines `Transformable`, a default-method trait giving any type with a `glam::Mat4`
(`Sprite`, `Model`) a fluent transform API (`translate`/`rotate`/`scale`/`flip_x`/...). Rotations
and scales compose about `Transformable::pivot()` (defaults to local origin; `Sprite` overrides it
to its quad center so sprites spin in place). All operations post-multiply onto the existing
matrix in local space — chained calls read outside-in (e.g. `rotate(...).translate(...)`
translates along the *rotated* axis). `set_transform`/`reset_transform` bypass composition
entirely when you want to drive the matrix directly.

## ECS: World, Entity, ComponentStore

`core/src/ecs/` is a small hand-rolled ECS, not a library like `hecs`/`bevy_ecs`. `Entity` (index +
generation) is allocated/recycled by `EntityAllocator`; `ComponentStore<T>` is a sparse
`Vec<Option<(generation, T)>>` keyed by entity index, generation-checked on every access so a
stale `Entity` never reads/writes a recycled slot. There's no query/archetype abstraction —
components are just named `ComponentStore<T>` fields on `World`.

`world.rs`'s `World` holds `transforms`, `names`, `renderables`, `scripts`, and `cameras`, each its
own `ComponentStore`. `World::sync_transforms()` copies each entity's `Transform` into its
`Renderable`'s GPU-facing matrix once per frame. A `CameraComponent` (fov/near/far/`active`)
attached to an entity, combined with its `Transform`, derives a renderable `Camera` via
`CameraComponent::derive_camera` — this is what lets the editor place a camera *in* a scene (see
below) rather than only having the one built into `Renderer`.

## The editor

The editor (`editor/`) is a scene editor built on `libdqg`. `editor/src/main.rs` is the entry
point; it builds a `Game` starting at a menu scene (New/Open/Recent project), which hands off to
`EditorScene` once a project is chosen. The UI is all `egui`, drawn across menu bar, hierarchy
panel, assets panel, and inspector.

- **Project format** — a `Project` is a folder on disk (a manifest, `assets/{textures,models}/`,
  `scenes/main.ron`, `scripts/`). Entities serialize as an `EntityRecord`
  (name/transform/renderable/camera/scripts); assets and scripts are referenced by
  project-root-relative path, never by an ID/handle.
- **Picking** — mesh-accurate ray/triangle picking for models, ray/sphere for sprites and camera
  entities, driving hover/selection highlight in the viewport.
- **Camera entities** — an entity with a `CameraComponent` renders as a frustum gizmo in Edit mode
  and can be marked "active"; entering Play mode swaps the renderer's camera to whichever entity
  is active, in place of the editor's own fly camera.
- **Scripting** — entities carry a stack of `.rhai` script attachments
  (`ScriptList`/`ScriptAttachment` in `core/src/scripting.rs`, a `World` component like any
  other), created from the Assets panel and attached/reordered/removed from the Inspector. See
  [Scripting](./scripting.md) for the full API scripts can use. `ScriptRuntime` compiles/caches one
  Rhai `AST` per script path and runs each attachment's `on_start()`/`on_update(dt, input)`
  against a minimal `entity`/`world`/`input` API — deliberately small; a script mostly only
  touches its own entity, plus limited cross-entity access via `world.find(name)`.
  `EditorScene`'s Play/Stop toggle snapshots the whole `World` before running scripts and restores
  it on Stop, dropping the `ScriptRuntime` (and all script state) with it. A script that errors
  repeatedly auto-disables itself, and errors surface in a small overlay rather than crashing the
  editor.

## Other core modules

- `camera.rs` — `Camera`, described above.
- `input.rs` — `InputState`/`MouseState` track per-frame key/button press/hold state and mouse
  position/wheel motion from winit events; `App` clears frame-transient state after each
  `RedrawRequested`.
- `titlebar.rs` / `platform.rs` — custom-drawn titlebar and Windows-specific chrome (rounded
  corners via `windows-sys`) used when `integrated_titlebar(true)`.
- `util.rs` — `resolve_resource_path` (exe-relative asset lookup fallback) and `slice_to_bytes`
  (raw byte view of a `&[T]` for uploading to GPU buffers).
