#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{globals, view},
}

struct Swell {
    strength: f32,
    scale: f32,
    speed: f32,
    glint: f32,
    born: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> swell: Swell;

fn wave(at: vec2<f32>, toward: vec2<f32>, frequency: f32, speed: f32, time: f32) -> vec3<f32> {
    let phase = dot(at, toward) * frequency + time * speed;
    return vec3<f32>(sin(phase), cos(phase) * toward * frequency);
}

fn hash(cell: vec2<f32>) -> f32 {
    return fract(sin(dot(cell, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn ripple(at: vec2<f32>, time: f32) -> vec3<f32> {
    let cell = floor(at);
    let inside = fract(at);
    let blend = inside * inside * (3.0 - 2.0 * inside);
    let slope = 6.0 * inside * (1.0 - inside);
    let a = hash(cell);
    let b = hash(cell + vec2<f32>(1.0, 0.0));
    let c = hash(cell + vec2<f32>(0.0, 1.0));
    let d = hash(cell + vec2<f32>(1.0, 1.0));
    let height = mix(mix(a, b, blend.x), mix(c, d, blend.x), blend.y);
    let dx = slope.x * mix(b - a, d - c, blend.y);
    let dz = slope.y * mix(c - a, d - b, blend.x);
    return vec3<f32>(height, dx, dz);
}

fn surface(at: vec2<f32>, time: f32) -> vec3<f32> {
    let p = at * swell.scale;
    let t = time * swell.speed;
    var sum = wave(p, normalize(vec2<f32>(1.0, 0.35)), 0.9, 1.3, t) * 0.5;
    sum += wave(p, normalize(vec2<f32>(-0.6, 1.0)), 1.6, 1.9, t) * 0.3;
    sum += wave(p, normalize(vec2<f32>(0.2, -1.0)), 2.7, 2.6, t) * 0.17;
    sum += wave(p, normalize(vec2<f32>(-1.0, -0.4)), 4.1, 3.3, t) * 0.1;
    let drift = vec2<f32>(t * 0.35, -t * 0.22);
    sum += (ripple(p * 2.3 + drift, t) - vec3<f32>(0.5, 0.0, 0.0)) * 0.22;
    sum += (ripple(p * 4.7 - drift * 1.6, t) - vec3<f32>(0.5, 0.0, 0.0)) * 0.12;
    return sum;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let at = in.world_position.xyz;
    let texel = floor(at.xz * 16.0);
    let p = vec2<u32>(texel - floor(texel / 4.0) * 4.0);
    let table = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    let speck = fract(sin(dot(floor(at - in.world_normal * 0.01), vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
    if (globals.time - swell.born.x) / 1.1 < (table[p.y * 4u + p.x] + 0.5) / 16.0 * 0.55 + speck * 0.45 {
        discard;
    }
    let distance = length(view.world_position.xyz - at);
    let fade = swell.strength / (1.0 + distance * 0.06);
    let shape = surface(at.xz, globals.time);
    let level = in.world_normal.y > 0.5;
    if level {
        let slope = shape.yz * swell.scale * fade;
        pbr_input.N = normalize(vec3<f32>(-slope.x, 1.0, -slope.y));
        let crest = clamp(shape.x * 0.5 + 0.5, 0.0, 1.0);
        let tint = 0.88 + crest * swell.glint;
        pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * tint, 1.0);
    }
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
