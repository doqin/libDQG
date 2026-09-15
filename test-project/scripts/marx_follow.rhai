// Called once when this script starts (Play begins, or the script is attached mid-Play).
let on_start = || {
};

// Called every frame while playing.
// `dt` is the elapsed time in seconds since the last frame.
// `input` lets you check keys, e.g. input.is_held("KeyW") or input.is_pressed("Space").
let on_update = |dt, input| {
    let marx_entity = world.find("marx");
    marx_entity.x = entity.x;
    marx_entity.y = entity.y + 2;
    marx_entity.z = entity.z;
};
