// Orbit distance and angle around the player, in spherical coordinates (yaw around the player's
// Y axis, pitch up/down). Captured once at start from wherever this camera starts relative to
// the player, so Play doesn't snap the camera to a different spot the moment it begins.
let distance = 0.0;
let yaw = 0.0;
let pitch = 0.0;

let mouse_sensitivity = 0.005;
// Keeps pitch shy of straight up/down, so the camera can't flip past the pole and orbit
// direction doesn't suddenly reverse.
let min_pitch = -1.5;
let max_pitch = 1.5;

// Called once when this script starts (Play begins, or the script is attached mid-Play).
let on_start = || {
    let player_entity = world.find("player");
    let dx = entity.x - player_entity.x;
    let dy = entity.y - player_entity.y;
    let dz = entity.z - player_entity.z;

    distance = sqrt(dx * dx + dy * dy + dz * dz);
    if distance > 0.0 {
        yaw = atan(dx, dz);
        pitch = asin(dy / distance);
    }
};

// Called every frame while playing.
// `dt` is the elapsed time in seconds since the last frame.
// `input` lets you check keys/mouse, e.g. input.is_held("KeyW") or input.mouse_dx().
let on_update = |dt, input| {
    let player_entity = world.find("player");

    if input.is_mouse_held("Middle") {
        // Mouse movement orbits the camera around the player.
        yaw -= input.mouse_dx() * mouse_sensitivity;
        pitch += input.mouse_dy() * mouse_sensitivity;
        pitch = max(min_pitch, min(max_pitch, pitch));
    }

    // Spherical offset from the player, at the fixed distance captured in on_start.
    let offset_x = distance * cos(pitch) * sin(yaw);
    let offset_y = distance * sin(pitch);
    let offset_z = distance * cos(pitch) * cos(yaw);

    entity.x = player_entity.x + offset_x;
    entity.y = player_entity.y + offset_y;
    entity.z = player_entity.z + offset_z;

    // Always keep facing the player, wherever it just moved.
    entity.look_at(player_entity.x, player_entity.y, player_entity.z);
};
