let spawned_entity_names = [];
let spin_speed = 2.0;

// Called once when this script starts (Play begins, or the script is attached mid-Play).
let on_start = || {
};

// Called every frame while playing.
// `dt` is the elapsed time in seconds since the last frame.
// `input` lets you check keys, e.g. input.is_held("KeyW") or input.is_pressed("Space").
let on_update = |dt, input| {
    if input.is_pressed("KeyF") {
        let kako_cube = world.spawn_entity("spawned_kako_cube_" + spawned_entity_names.len().to_string(), entity.x, entity.y, entity.z);
        kako_cube.set_persistent(true);
        kako_cube.set_model("assets/models/cube.obj");
        kako_cube.attach_script("scripts/spin.rhai");
        spawned_entity_names.push(kako_cube.name());
    }
};
