@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

// Only `highlight` is used here; `model` is declared to match the buffer's actual layout (see
// `ModelUniform` in model.rs) even though this stage doesn't read it.
struct ModelTransform {
    model: mat4x4<f32>,
    highlight: vec4<f32>,
}
@group(2) @binding(0) var<uniform> transform: ModelTransform;

const LIGHT_DIR: vec3<f32> = vec3<f32>(0.4, 0.8, 0.4);
const AMBIENT: f32 = 0.25;

@fragment
fn fs_main(
    @location(0) uv: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
) -> @location(0) vec4<f32> {
    let n = normalize(world_normal);
    let diffuse = max(dot(n, normalize(LIGHT_DIR)), 0.0);
    let light = min(AMBIENT + diffuse, 1.0);
    let tex_color = textureSample(t_diffuse, s_diffuse, uv);
    let shaded = tex_color.rgb * light;
    let final_color = mix(shaded, transform.highlight.rgb, transform.highlight.a);
    return vec4<f32>(final_color, tex_color.a);
}
