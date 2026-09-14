struct CameraUniform {
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) highlight: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    // Both already computed on the CPU per draw call (world_normal from the sprite's transform,
    // highlight from Sprite::highlight), so they're passed straight through rather than
    // transformed here.
    @location(3) normal: vec3<f32>,
    @location(4) highlight: vec4<f32>,
) -> VOut {
    var out: VOut;
    out.pos = camera.view_proj * vec4<f32>(pos, 1.0);
    out.uv = uv;
    out.color = color;
    out.world_normal = normal;
    out.highlight = highlight;
    return out;
}
