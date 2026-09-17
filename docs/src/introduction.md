# libDQG

`libDQG` is a personal 2D/3D game framework in Rust, built on [`wgpu`](https://wgpu.rs/) and
[`winit`](https://github.com/rust-windowing/winit). It's a hobby successor to an earlier DirectX 9
framework, moving away from OOP/inheritance toward data-driven design. It isn't meant to be a
general-purpose production engine — it favors simplicity and directness over extensibility.

The workspace has three crates:

- **`core`** — the `libdqg` library: the framework itself (renderer, scene/ECS, input, scripting
  runtime, and so on).
- **`demo`** — a small game exercising the framework, effectively the framework's own integration
  test/sample.
- **`editor`** — a visual scene editor built on `libdqg`, with its own project file format,
  mesh-accurate picking, and a Rhai-based scripting system for entity behavior.

## Where to go from here

- **[Architecture](./architecture.md)** — how the pieces fit together: the game loop, the
  renderer, the ECS, transforms, and the editor.
- **[Scripting](./scripting.md)** — the API available to `.rhai` scripts attached to entities in
  the editor.
- **[API reference](api/index.html)** — generated documentation for every public type and
  function in the `libdqg` and `editor` crates.

## Source

The source code lives at [github.com/doqin/libDQG](https://github.com/doqin/libDQG).
