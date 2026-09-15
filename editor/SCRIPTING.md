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
attached to. There's no way for a script to reach any *other* entity.

| Member | Description |
|---|---|
| `entity.x`, `entity.y`, `entity.z` | Get or set the entity's position directly, one axis at a time. |
| `entity.translate(x, y, z)` | Moves the entity by this amount (adds to its current position). |
| `entity.rotate(x, y, z)` | Rotates the entity by this amount, in **radians**, around each axis. This is a *delta* — it turns the entity further from wherever it currently is, it doesn't set an absolute angle. |
| `entity.scale(x, y, z)` | Multiplies the entity's current scale by this amount. `entity.scale(2.0, 2.0, 2.0)` doubles its size; `entity.scale(1.0, 1.0, 1.0)` leaves it unchanged. |
| `entity.name()` | Returns the entity's name (the one shown in the Hierarchy panel), as text. |

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

When you click **Stop**:
- Every entity's position, rotation, and scale snap back to whatever they were the moment you
  clicked Play — so play-testing never permanently changes your scene.
- All script state (including things like `elapsed` above) is discarded.

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

- Reach or affect any entity other than the one they're attached to.
- Spawn or delete entities.
- Change an entity's name, its attached model/sprite, or which scripts are attached to it.
- Read or affect anything outside of Play mode (scripts don't run in Edit mode at all).

There's also one rough edge worth knowing about rather than being surprised by: attaching a
script (or re-enabling a disabled one) *while already in Play mode* doesn't make it start running
— only scripts that were enabled and attached at the moment you clicked Play actually run that
session. Click Stop and Play again to pick up the change.

If you need one of these, it's worth raising — the API is deliberately minimal for now, not
permanently limited.
