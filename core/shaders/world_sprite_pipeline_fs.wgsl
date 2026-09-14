@group(0) @binding(0) var t_diffuse: texture_2d<f32>;
@group(0) @binding(1) var s_diffuse: sampler;

// Matches model_pipeline_fs.wgsl's lighting so world sprites and models are lit consistently.
const LIGHT_DIR: vec3<f32> = vec3<f32>(0.4, 0.8, 0.4);
const AMBIENT: f32 = 0.25;

@fragment
fn fs_main(
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) highlight: vec4<f32>,
) -> @location(0) vec4<f32> {
    // world_normal can be the zero vector for a degenerate (e.g. zero-scale) transform; guard
    // against normalizing it into NaN.
    let len = length(world_normal);
    let n = select(vec3<f32>(0.0, 0.0, 1.0), world_normal / max(len, 1e-6), len > 1e-6);
    let diffuse = max(dot(n, normalize(LIGHT_DIR)), 0.0);
    let light = min(AMBIENT + diffuse, 1.0);
    let base = textureSample(t_diffuse, s_diffuse, uv) * color;
    let shaded = base.rgb * light;
    let final_color = mix(shaded, highlight.rgb, highlight.a);
    return vec4<f32>(final_color, base.a);
}
