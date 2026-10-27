#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::globals,
}

struct Carving {
    glow: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> carving: Carving;

const RUNES_FROM: f32 = 0.5;
const RUNE_STEP: f32 = 0.38;
const RUNE_SIZE: vec2<f32> = vec2<f32>(0.2, 0.28);

fn scramble(cell: vec3<i32>, salt: u32) -> u32 {
    var v = bitcast<vec3<u32>>(cell) * 1664525u + vec3<u32>(1013904223u + salt);
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v = v ^ (v >> vec3<u32>(16u));
    return v.x + v.y * v.z;
}

fn chance(cell: vec3<i32>, salt: u32) -> f32 {
    return f32(scramble(cell, salt) & 0xffffu) / 65535.0;
}

fn noise3(at: vec3<f32>) -> f32 {
    let base = floor(at);
    let f = at - base;
    let c = vec3<i32>(base);
    let u = f * f * (3.0 - 2.0 * f);
    let x00 = mix(chance(c, 0u), chance(c + vec3(1, 0, 0), 0u), u.x);
    let x10 = mix(chance(c + vec3(0, 1, 0), 0u), chance(c + vec3(1, 1, 0), 0u), u.x);
    let x01 = mix(chance(c + vec3(0, 0, 1), 0u), chance(c + vec3(1, 0, 1), 0u), u.x);
    let x11 = mix(chance(c + vec3(0, 1, 1), 0u), chance(c + vec3(1, 1, 1), 0u), u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

fn stroke(point: vec2<f32>, start: vec2<f32>, end: vec2<f32>) -> f32 {
    let line = end - start;
    let along = clamp(dot(point - start, line) / dot(line, line), 0.0, 1.0);
    return length(point - start - line * along);
}

fn rune(at: vec2<f32>, seed: vec3<i32>) -> f32 {
    var spots = array<vec2<f32>, 9>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0), vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, 2.0), vec2<f32>(1.0, 2.0), vec2<f32>(2.0, 2.0));
    var lines = array<vec2<u32>, 16>(
        vec2<u32>(0u, 1u), vec2<u32>(1u, 2u), vec2<u32>(3u, 4u), vec2<u32>(4u, 5u), vec2<u32>(6u, 7u), vec2<u32>(7u, 8u),
        vec2<u32>(0u, 4u), vec2<u32>(2u, 4u), vec2<u32>(4u, 6u), vec2<u32>(4u, 8u),
        vec2<u32>(0u, 3u), vec2<u32>(3u, 6u), vec2<u32>(2u, 5u), vec2<u32>(5u, 8u), vec2<u32>(1u, 3u), vec2<u32>(5u, 7u));
    let stem = floor(chance(seed, 60u) * 3.0);
    var near = stroke(at, vec2<f32>(stem, 0.0), vec2<f32>(stem, 2.0));
    for (var index = 0u; index < 16u; index++) {
        if chance(seed, 61u + index) < 0.22 {
            let pair = lines[index];
            near = min(near, stroke(at, spots[pair.x], spots[pair.y]));
        }
    }
    return near;
}

const OCTAVES: i32 = 10;
const LOWEST: f32 = 7.0;
const LACUNARITY: f32 = 2.13;
const PERSISTENCE: f32 = 0.56;
const RELIEF: f32 = 0.0035;

struct Relief {
    gradient: vec3<f32>,
    height: f32,
}

fn heading(index: i32) -> vec3<f32> {
    let k = f32(index) + 0.5;
    let rise = 1.0 - 2.0 * fract(k * 0.618034);
    let turn = k * 2.399963;
    let out = sqrt(1.0 - rise * rise);
    return vec3<f32>(cos(turn) * out, rise, sin(turn) * out);
}

fn weierstrass(at: vec3<f32>, footprint: f32) -> Relief {
    let warp = vec3<f32>(noise3(at * 3.0), noise3(at * 3.0 + 17.0), noise3(at * 3.0 + 31.0)) - 0.5;
    let p = at + warp * 0.12;
    var relief = Relief(vec3<f32>(0.0), 0.0);
    var amplitude = 1.0;
    var frequency = LOWEST;
    for (var octave = 0; octave < OCTAVES; octave++) {
        let seen = 1.0 - smoothstep(0.2, 0.45, frequency * footprint);
        for (var wave = 0; wave < 3; wave++) {
            let toward = heading(octave * 3 + wave);
            let phase = dot(toward, p) * frequency + f32(octave * 3 + wave) * 2.1;
            relief.height += amplitude * seen * cos(phase) / 3.0;
            relief.gradient -= toward * (amplitude * frequency * seen * sin(phase) / 3.0);
        }
        amplitude *= PERSISTENCE;
        frequency *= LACUNARITY;
    }
    return relief;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let at = in.world_position.xyz;
    let smooth_normal = normalize(in.world_normal);
    let footprint = length(fwidth(at));
    let micro = weierstrass(at, footprint);
    let slope = micro.gradient * RELIEF;
    let normal = normalize(smooth_normal - (slope - smooth_normal * dot(slope, smooth_normal)));
    let height = in.uv.x;
    let finish = in.uv.y;
    let cell = vec3<i32>(floor(at - normal * 0.01));
    let home = vec3<i32>(i32(floor(at.x)), 0, i32(floor(at.z)));
    let pulse = 0.55 + 0.45 * sin(globals.time * 1.6 - height * 4.0 + chance(home, 9u) * 6.28);
    let grain = noise3(at * 7.0) * 0.5 + noise3(at * 21.0) * 0.3 + noise3(at * 2.3) * 0.2;
    var stone = mix(vec3<f32>(0.08, 0.085, 0.1), vec3<f32>(0.24, 0.24, 0.26), grain);
    let moss = smoothstep(0.52, 0.7, noise3(at * 3.1 + 7.0)) * (1.0 - smoothstep(0.1, 0.7, height)) * smoothstep(-0.2, 0.6, normal.y + 0.3);
    stone = mix(stone, vec3<f32>(0.05, 0.11, 0.025) * (0.8 + grain * 0.4), moss);
    stone = stone * (0.88 + 0.12 * clamp(micro.height * 0.8 + 0.5, 0.0, 1.0));
    var glow = 0.0;
    if finish > 0.5 && finish < 1.5 {
        let across_axis = normalize(vec2<f32>(-smooth_normal.z, smooth_normal.x));
        let across = dot(at.xz - (floor(at.xz) + 0.5), across_axis);
        let slot = floor((height - RUNES_FROM) / RUNE_STEP);
        let within = vec2<f32>(across + RUNE_SIZE.x * 0.5, height - RUNES_FROM - slot * RUNE_STEP) / RUNE_SIZE;
        if slot >= 0.0 && slot < 3.0 && all(within > vec2<f32>(0.0)) && all(within < vec2<f32>(1.0)) {
            let face = i32(round(smooth_normal.x * 2.0 + smooth_normal.z * 3.0));
            let near = rune(within * 2.0, home + vec3<i32>(face * 7, i32(slot), face));
            let groove = 1.0 - smoothstep(0.14, 0.24, near);
            stone = stone * (1.0 - groove * 0.55);
            glow = groove * pulse;
        }
        let band = smoothstep(0.475, 0.49, abs(fract((height - RUNES_FROM + 0.05) / RUNE_STEP) - 0.5));
        glow = max(glow, band * 0.35 * pulse);
    } else if finish > 1.5 {
        glow = smoothstep(1.86, 2.08, height) * (0.5 + 0.5 * pulse);
    }
    pbr_input.material.base_color = vec4<f32>(stone, 1.0);
    pbr_input.N = normal;
    pbr_input.material.perceptual_roughness = clamp(0.82 - micro.height * 0.06, 0.6, 0.95) - glow * 0.4;
    pbr_input.material.emissive = vec4<f32>(carving.glow.rgb * glow * 2.5, 0.0);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
