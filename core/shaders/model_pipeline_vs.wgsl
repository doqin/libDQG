struct CameraUniform {
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct ModelTransform {
    model: mat4x4<f32>,
}
@group(2) @binding(0) var<uniform> transform: ModelTransform;

struct VOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
) -> VOut {
    var out: VOut;
    let world_pos = transform.model * vec4<f32>(position, 1.0);
    out.clip_pos = camera.view_proj * world_pos;
    out.uv = tex_coords;
    out.world_normal = normalize((transform.model * vec4<f32>(normal, 0.0)).xyz);
    return out;
}
