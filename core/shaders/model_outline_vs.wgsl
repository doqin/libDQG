struct CameraUniform {
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct ModelTransform {
    model: mat4x4<f32>,
}
@group(2) @binding(0) var<uniform> transform: ModelTransform;

// World-space distance each vertex is pushed outward along its normal, forming the outline
// silhouette rendered by model_outline_fs.wgsl.
const OUTLINE_WIDTH: f32 = 0.02;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
) -> @builtin(position) vec4<f32> {
    let len = length(normal);
    let n = select(vec3<f32>(0.0), normal / max(len, 1e-6), len > 1e-6);
    let expanded = position + n * OUTLINE_WIDTH;
    let world_pos = transform.model * vec4<f32>(expanded, 1.0);
    return camera.view_proj * world_pos;
}
