let movement_speed = 5.0;

// Called once when this script starts (Play begins, or the script is attached mid-Play).
let on_start = || {
};

// Called every frame while playing.
// `dt` is the elapsed time in seconds since the last frame.
// `input` lets you check keys, e.g. input.is_held("KeyW") or input.is_pressed("Space").
let on_update = |dt, input| {
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
