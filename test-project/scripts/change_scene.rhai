// Called once when this script starts (Play begins, or the script is attached mid-Play).
let on_start = || {
};

// Called every frame while playing.
// `dt` is the elapsed time in seconds since the last frame.
// `input` lets you check keys, e.g. input.is_held("KeyW") or input.is_pressed("Space").
let on_update = |dt, input| {
    if input.is_pressed("Digit1") {
        scene.change("scenes/main.ron");
    }
    if input.is_pressed("Digit2") {
        scene.change("scenes/secondary.ron");
    }
};
