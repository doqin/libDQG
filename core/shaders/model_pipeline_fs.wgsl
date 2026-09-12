@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

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
    return vec4<f32>(tex_color.rgb * light, tex_color.a);
}
