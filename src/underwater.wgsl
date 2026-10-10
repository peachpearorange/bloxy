#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;

struct Submerged {
    tint: vec4<f32>,
    time: f32,
    strength: f32,
    _pad: vec2<f32>,
}

@group(0) @binding(2) var<uniform> submerged: Submerged;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let t = submerged.time;
    let wobble = vec2<f32>(
        sin(in.uv.y * 26.0 + t * 2.1) + sin(in.uv.y * 11.0 - t * 1.3) * 0.6,
        cos(in.uv.x * 21.0 + t * 1.7) + cos(in.uv.x * 9.0 + t * 0.9) * 0.6
    ) * 0.0025 * submerged.strength;
    let uv = clamp(in.uv + wobble, vec2<f32>(0.001), vec2<f32>(0.999));
    let seen = textureSample(screen, screen_sampler, uv).rgb;
    let rim = in.uv - vec2<f32>(0.5);
    let vignette = 1.0 - dot(rim, rim) * 1.1 * submerged.strength;
    let caustic = 1.0 + submerged.tint.a * submerged.strength
        * sin(in.uv.x * 40.0 + t * 1.9 + sin(in.uv.y * 31.0 - t * 1.4) * 2.0)
        * sin(in.uv.y * 37.0 - t * 1.6);
    let tinted = mix(seen, seen * submerged.tint.rgb, submerged.strength);
    return vec4<f32>(tinted * vignette * caustic, 1.0);
}
