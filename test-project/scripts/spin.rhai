let spin_speed = 2.0;

// updates every frame
let on_update = |dt, input| {
    entity.rotate(0.0, spin_speed * dt, 0.0);
}