@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

@fragment
fn fs_main(
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>
) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, uv) * color;
}
