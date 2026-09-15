# Scripting

Entities in the editor can have behavior attached to them by writing small scripts in
[Rhai](https://rhai.rs/), a lightweight scripting language embedded directly in the editor. This
page documents the API available to those scripts. It doesn't cover the Rhai language itself
(variables, `if`/`else`, loops, arrays, ...) — for that, see the
[Rhai Language Reference](https://rhai.rs/book/language/). Everything on this page is specific to
this editor.

## Getting started

1. Open the **Assets** panel at the bottom of the editor and find the **Scripts** section.
2. Click **New Script** to create a script file (`Script.rhai`, `Script2.rhai`, ...) pre-filled
   with a starting template.
3. Select an entity, open the **Inspector** panel on the right, and under **Scripts** click
   **Attach Script**, then pick your new script.
4. Click **Play** in the top menu bar. Your script now runs every frame until you click **Stop**.

You can attach more than one script to the same entity — they run top to bottom in the order
shown in the Inspector, which you can change with the **Up**/**Down** buttons there. Each has its
own checkbox to enable/disable it without removing it.

To edit a script's contents, click its tile in the Assets panel. The first time you do this,
you'll be asked to pick a text editor (any program that can open a plain text file); it's
remembered after that, so later clicks just open straight away. Right-click a script tile for
**Rename** or to change your remembered editor.

## Script structure

A script is a plain text file defining up to two functions:

```rhai
let on_start = || {
    // Runs once, right when Play starts.
};

let on_update = |dt, input| {
    // Runs every frame while playing.
};
```

Both are **optional** — a script with neither doesn't do anything (harmless), and a script with
only one of them just skips the other.

**Important:** they must be written exactly like this — `let name = |args| { ... };` — not as
`fn on_update(dt, input) { ... }`. This isn't a style preference: only a closure assigned to a
`let` variable can keep its own private state between frames (see
[Keeping state between frames](#keeping-state-between-frames) below). A plain `fn` can't see
outside variables at all in Rhai, so it wouldn't be able to remember anything from one frame to
the next.

`on_update`'s parameters:
- `dt` — the time in seconds since the last frame (a small number, typically around `0.016` at 60
  FPS). Multiply speeds by `dt` so movement is frame-rate independent.
- `input` — the current keyboard state. See [The `input` object](#the-input-object).

## The `entity` object

Every script has automatic access to a variable called `entity` — the entity the script is
attached to. This is the same kind of value `world.find(...)` hands back for *other* entities
(see [The `world` object](#the-world-object) below) — there's only one entity type with one set
of members, whether it's your own entity or one you looked up.

| Member | Description |
|---|---|
| `entity.x`, `entity.y`, `entity.z` | Get or set the entity's position directly, one axis at a time. |
| `entity.translate(x, y, z)` | Moves the entity by this amount (adds to its current position). |
| `entity.rotate(x, y, z)` | Rotates the entity by this amount, in **radians**, around each axis. This is a *delta* — it turns the entity further from wherever it currently is, it doesn't set an absolute angle. |
| `entity.scale(x, y, z)` | Multiplies the entity's current scale by this amount. `entity.scale(2.0, 2.0, 2.0)` doubles its size; `entity.scale(1.0, 1.0, 1.0)` leaves it unchanged. |
| `entity.name()` | Returns the entity's name (the one shown in the Hierarchy panel), as text. |
| `entity.set_name(name)` | Renames the entity. |
| `entity.despawn()` | Removes the entity from the scene. If a script despawns its own entity, none of that entity's other scripts run for the rest of that frame. |
| `entity.attach_script(path)` | Attaches another script to this entity, by its project-relative path (the same form shown in the Inspector, e.g. `"scripts/Move.rhai"`) — and, unlike attaching one from the Inspector's Assets panel, it starts running immediately, the same session, not just after the next Stop/Play. |
| `entity.set_script_enabled(index, enabled)` | Enables or disables one of the entity's attached scripts by its position in the Inspector's Scripts list (`0` is the first one). |
| `entity.set_sprite(path)` | Attaches (or swaps to) a sprite, loading the texture at `path` (project-relative, e.g. `"assets/textures/player.png"`) — same idea as the Inspector's **Attach Sprite** picker. Replaces whatever renderable the entity already had, if any. |
| `entity.set_model(path)` | The model counterpart to `set_sprite` — loads an `.obj` at `path` (e.g. `"assets/models/crate.obj"`). |
| `entity.detach_renderable()` | Removes whatever sprite/model the entity currently has, if any. Does nothing (not an error) if it didn't have one. |

```rhai
let on_update = |dt, input| {
    entity.translate(0.0, 0.0, -1.0 * dt);   // moves 1 unit/second along -Z
    entity.rotate(0.0, 1.5 * dt, 0.0);       // spins around Y at 1.5 radians/second
};
```

A note on rotation: `rotate` composes with whatever rotation the entity already has, the same way
turning a steering wheel further turns the car further, rather than snapping it to face a
specific direction. If you want to face a specific direction, you'll need to work that out
yourself from repeated `rotate` calls (there's currently no "set absolute rotation" — this may be
added later if it turns out to be needed).

A note on `set_sprite`/`set_model`: unlike the other `entity` methods, these read a file from disk
and upload it to the GPU — the same cost as clicking **Attach Sprite**/**Attach Model** in the
Inspector, just triggered from a script instead of a click. Call them when something actually
changes (an `on_start`, or in response to an event), not unconditionally every frame from
`on_update` — that would reload and re-upload the same asset 60 times a second for no reason.

```rhai
let on_start = || {
    entity.set_sprite("assets/textures/idle.png");
};
```

## The `world` object

Every script also has automatic access to a variable called `world`, for reaching entities other
than your own:

| Member | Description |
|---|---|
| `world.find(name)` | Looks up an entity by its name (the one shown in the Hierarchy panel). Returns an entity value — the same kind `entity` is — if one is found, or `()` (Rhai's "nothing" value) if not. If more than one entity shares that name, you get whichever was created first. |
| `world.spawn_entity(name, x, y, z)` | Creates a new entity at the given position and returns it, ready to use right away (`world.spawn_entity("Bullet", entity.x, entity.y, entity.z).translate(0.0, 0.0, -1.0)` works in the same line). |

```rhai
let on_update = |dt, input| {
    let target = world.find("Player");
    if target != () {
        entity.x = target.x;   // follow the entity named "Player" on the x axis
    }
};
```

A couple of things worth knowing about `world.find(...)`:
- It reflects that entity's position/name as of the **start of the current frame** — if another
  script already moved or renamed that entity earlier this same frame, `find` won't see that
  change until next frame. Your own `entity`, by contrast, is always fully up to date, including
  changes your own script just made a moment earlier in the same call. This only matters if
  you're chaining cross-entity logic within a single frame; for most scripts (following another
  entity, checking its position, etc.) it's not something you'll notice.
- Anything you *do* to an entity reached via `find` (`.translate(...)`, `.despawn()`, ...) takes
  effect right away, same as it does for `entity` — the frame-start staleness only applies to
  what `find` hands you, not to writes made through it.

## The `input` object

`input` (`on_update`'s second parameter) tells you what's happening with the keyboard **this
frame**:

| Method | True when... |
|---|---|
| `input.is_held(name)` | the key is currently held down (true for every frame it's held, including the first). |
| `input.is_pressed(name)` | the key was *just* pressed down this frame (true for exactly one frame per press, even if held afterward). |

`name` is a piece of text identifying a physical key. The common ones:

| Category | Names |
|---|---|
| Letters | `"KeyA"` through `"KeyZ"` |
| Digits | `"Digit0"` through `"Digit9"` |
| Arrows | `"ArrowUp"`, `"ArrowDown"`, `"ArrowLeft"`, `"ArrowRight"` |
| Modifiers | `"ShiftLeft"`, `"ShiftRight"`, `"ControlLeft"`, `"ControlRight"`, `"AltLeft"`, `"AltRight"` |
| Whitespace/editing | `"Space"`, `"Enter"`, `"Tab"`, `"Backspace"`, `"Escape"` |
| Function keys | `"F1"` through `"F12"` (and beyond, rarely needed) |

Any physical key on the keyboard works, not just the ones above — the name always matches the
key's own physical position (e.g. `"BracketLeft"` for `[`), not what's printed on it, so it's
consistent across keyboard layouts. This mostly (not always — the Windows/Cmd key is
`"SuperLeft"`/`"SuperRight"` here rather than the web's `"MetaLeft"`/`"MetaRight"`) resembles a web
browser's `KeyboardEvent.code` naming, if you want a mental model — for the exact, definitive list
of every supported name, see the `KeyCode` enum in
[`core/src/types.rs`](../core/src/types.rs). A misspelled or unrecognized name is simply never
held/pressed, it won't cause an error.

```rhai
let on_update = |dt, input| {
    if input.is_held("KeyW") {
        entity.translate(0.0, 0.0, -5.0 * dt);
    }
    if input.is_pressed("Space") {
        entity.translate(0.0, 1.0, 0.0);   // hop, once per press
    }
};
```

## Keeping state between frames

Since scripts are `let`-bound closures, any variable declared alongside them in the same script
is automatically remembered from one call to the next — this is how you build up state like a
timer, a counter, or a toggle:

```rhai
let elapsed = 0.0;

let on_update = |dt, input| {
    elapsed += dt;
    entity.y = 0.5 + 0.25 * sin(elapsed * 3.0);   // gentle bob up and down over time
};
```

`elapsed` isn't reset every frame — it keeps accumulating for as long as Play is running. It does
reset when you click Stop (and Play again), since every script's state starts fresh each time
Play begins.

## Play and Stop

Scripts only run while the editor is in Play mode. When you click **Play**:
- Every enabled script attached to every entity has its `on_start` called once (if it has one).
- Every entity's Transform panel in the Inspector becomes read-only for the duration — a script
  moving the entity every frame would otherwise fight with you dragging the same fields.
- Saving is disabled for the duration too — see why under Stop, below.

When you click **Stop**, the whole scene snaps back to exactly how it was the moment you clicked
Play — not just position/rotation/scale, but names, which scripts are attached to what, what
entities look like, and which entities exist at all:
- Every entity's position, rotation, and scale snap back to what they were before Play.
- Any entity a script renamed goes back to its old name; any script it attached (via
  `entity.attach_script(...)`) or enabled/disabled (via `entity.set_script_enabled(...)`) reverts
  too.
- Any entity whose sprite/model a script changed (`set_sprite`/`set_model`/`detach_renderable`)
  goes back to whatever it was showing before Play.
- Any entity a script spawned (`world.spawn_entity(...)`) disappears; any entity a script
  despawned (`entity.despawn()`) comes back.
- All script state (including things like `elapsed` above) is discarded.

Play-testing never permanently changes your scene — that's also why Save is disabled while
Playing: without that, saving mid-Play would bake all of the above into the scene file instead of
letting Stop discard it.

## Errors

If a script fails to compile, or throws an error while running (e.g. calling
`entity.rotate("a", "b", "c")` with text instead of numbers), you'll see a message in the
bottom-left corner of the screen rather than the editor crashing. A script that keeps failing
every frame for five frames in a row disables itself automatically (unchecking its box in the
Inspector's Scripts list) so it doesn't spam the screen with errors forever. It stays disabled —
including across Stop and Play again — until you manually re-check its box.

## A complete example

A script that spins an entity continuously and lets WASD move it around:

```rhai
let spin_speed = 2.0;
let movement_speed = 5.0;

let on_update = |dt, input| {
    entity.rotate(0.0, spin_speed * dt, 0.0);

    if input.is_held("KeyW") {
        entity.translate(0.0, 0.0, -movement_speed * dt);
    }
    if input.is_held("KeyS") {
        entity.translate(0.0, 0.0, movement_speed * dt);
    }
    if input.is_held("KeyD") {
        entity.translate(movement_speed * dt, 0.0, 0.0);
    }
    if input.is_held("KeyA") {
        entity.translate(-movement_speed * dt, 0.0, 0.0);
    }
};
```

## Current limitations

This is an early version of scripting, kept intentionally small. Things scripts **can't** do yet:

- Truly remove/detach a *script* from an entity (attaching and enabling/disabling are supported;
  removal isn't yet — this is specifically about the Scripts list, not about `detach_renderable`,
  which is fully supported).
- Read or affect anything outside of Play mode (scripts don't run in Edit mode at all).

There's also one rough edge worth knowing about rather than being surprised by: attaching a
script from the Inspector's Assets panel, or re-enabling a disabled one there by hand, *while
already in Play mode* doesn't make it start running — only scripts that were enabled and attached
at the moment you clicked Play, plus anything attached since via `entity.attach_script(...)`,
actually run that session. Click Stop and Play again to pick up a change made through the
Inspector.

If you need one of these, it's worth raising — the API is deliberately minimal for now, not
permanently limited.
