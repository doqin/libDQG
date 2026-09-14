struct CameraUniform {
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0) var<uniform> camera: CameraUniform;

// The enlarged outline quad is already in world space (baked on the CPU by
// DrawPass::draw_world_sprite_outline), so the vertex shader just projects it.
@vertex
fn vs_main(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return camera.view_proj * vec4<f32>(pos, 1.0);
}
