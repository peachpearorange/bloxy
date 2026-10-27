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

const RUNES_FROM: f32 = 0.42;
const RUNE_STEP: f32 = 0.4;
const RUNE_SIZE: vec2<f32> = vec2<f32>(0.15, 0.24);
const POST: f32 = 0.34;
const LEAN: f32 = 0.025;

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

const VESICLES: f32 = 16.0;
const JOINTS: f32 = 3.5;
const SCALLOPS: f32 = 6.0;
const GRAINS: f32 = 110.0;
const RELIEF: f32 = 2.0;

fn bumped(normal: vec3<f32>, at: vec3<f32>, height: f32) -> vec3<f32> {
    let dx = dpdx(at);
    let dy = dpdy(at);
    let r1 = cross(dy, normal);
    let r2 = cross(normal, dx);
    let det = dot(dx, r1);
    let grad = sign(det) * (dpdx(height) * r1 + dpdy(height) * r2);
    return normalize(abs(det) * normal - grad);
}

struct Rock {
    tone: vec3<f32>,
    height: f32,
    hole: f32,
    crack: f32,
    hollow: f32,
}

struct Cells {
    nearest: f32,
    gap: f32,
    id: vec3<i32>,
}

fn cells(p: vec3<f32>, salt: u32) -> Cells {
    let base = vec3<i32>(floor(p));
    var found = Cells(9.0, 9.0, base);
    var second = 9.0;
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let cell = base + vec3<i32>(x, y, z);
                let point = vec3<f32>(cell) + vec3<f32>(chance(cell, salt), chance(cell, salt + 1u), chance(cell, salt + 2u));
                let distance = length(p - point);
                if distance < found.nearest {
                    second = found.nearest;
                    found.nearest = distance;
                    found.id = cell;
                } else if distance < second {
                    second = distance;
                }
            }
        }
    }
    found.gap = second - found.nearest;
    return found;
}

fn basalt(at: vec3<f32>, normal: vec3<f32>, footprint: f32) -> Rock {
    let mottle = noise3(at * 1.4) * 0.6 + noise3(at * 5.0 + 3.0) * 0.4;
    var tone = vec3<f32>(0.07, 0.07, 0.076) * (0.75 + mottle * 0.5);
    let crust = smoothstep(0.55, 0.75, noise3(at * 2.2 + 11.0) + normal.y * 0.15);
    tone = mix(tone, vec3<f32>(0.15, 0.142, 0.13) * (0.85 + mottle * 0.3), crust * 0.6);
    let warp = vec3<f32>(noise3(at * 2.0), noise3(at * 2.0 + 9.0), noise3(at * 2.0 + 19.0)) - 0.5;
    let scallop = cells(at * SCALLOPS + warp * 0.9, 41u);
    let flakes = cells(at * SCALLOPS * 2.7 + warp * 1.7, 47u);
    let chips = -scallop.nearest * scallop.nearest * (0.012 + chance(scallop.id, 44u) * 0.03)
        - flakes.nearest * flakes.nearest * (0.002 + chance(flakes.id, 48u) * 0.006);
    let fine = 1.0 - smoothstep(0.25, 0.7, footprint * GRAINS);
    let grain = ((noise3(at * GRAINS) - 0.5) * 0.0024 + (noise3(at * GRAINS * 2.3 + 5.0) - 0.5) * 0.0012) * fine;
    let pale = smoothstep(0.66, 0.74, noise3(at * GRAINS * 0.8 + 13.0));
    let dark = smoothstep(0.64, 0.72, noise3(at * GRAINS * 0.6 + 29.0));
    let speck = mix(1.0, 1.0 + pale * 1.3 - dark * 0.45, fine);
    let bubbles = cells(at * VESICLES, 21u);
    let pixel = max(footprint * VESICLES, 0.01);
    let sharp = 1.0 - smoothstep(0.3, 0.8, pixel);
    let size = 0.12 + chance(bubbles.id, 24u) * 0.22;
    let hole = select(0.0, 1.0 - smoothstep(size - pixel, size + pixel, bubbles.nearest), chance(bubbles.id, 25u) < 0.3) * sharp;
    let joints = cells(at * JOINTS + vec3<f32>(noise3(at * 3.0), noise3(at * 3.0 + 7.0), 0.0) * 0.6, 31u);
    let width = max(footprint * JOINTS * 1.2, 0.012);
    let crack = select(0.0, 1.0 - smoothstep(0.0, width, joints.gap), chance(joints.id, 34u) < 0.35) * (1.0 - smoothstep(0.3, 0.8, footprint * JOINTS * 6.0));
    let lumps = (noise3(at * 13.0 + 5.0) - 0.5) * 0.004;
    let height = chips + lumps + grain - hole * 0.004 - crack * 0.003;
    let hollow = 1.0 - smoothstep(-0.024, -0.002, chips);
    return Rock(tone * speck, height, hole, crack, hollow);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let at = in.world_position.xyz;
    let smooth_normal = normalize(in.world_normal);
    let footprint = length(fwidth(at));
    let rock = basalt(at, smooth_normal, footprint);
    let normal = bumped(smooth_normal, at, rock.height * RELIEF);
    let height = in.uv.x;
    let finish = in.uv.y;
    let cell = vec3<i32>(floor(at - normal * 0.01));
    let home = vec3<i32>(i32(floor(at.x)), 0, i32(floor(at.z)));
    let pulse = 0.55 + 0.45 * sin(globals.time * 1.6 - height * 4.0 + chance(home, 9u) * 6.28);
    let grain = noise3(at * 7.0) * 0.5 + noise3(at * 21.0) * 0.3 + noise3(at * 2.3) * 0.2;
    var stone = rock.tone * (1.0 - rock.hole * 0.7) * (1.0 - rock.crack * 0.65);
    let moss = smoothstep(0.52, 0.7, noise3(at * 3.1 + 7.0)) * (1.0 - smoothstep(0.1, 0.7, height)) * smoothstep(-0.2, 0.6, normal.y + 0.3);
    stone = mix(stone, vec3<f32>(0.05, 0.11, 0.025) * (0.8 + grain * 0.4), moss);
    stone = stone * (1.0 - rock.hollow * 0.5);
    var glow = 0.0;
    if finish > 0.5 && finish < 1.5 {
        let across_axis = normalize(vec2<f32>(-smooth_normal.z, smooth_normal.x));
        let local = at.xz - (floor(at.xz) + 0.5);
        let side = select(-1.0, 1.0, local.x > 0.0);
        let across = dot(local - vec2<f32>(side * (POST - LEAN * height), 0.0), across_axis);
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
        glow = (0.15 + 0.35 * pulse) * smoothstep(0.1, 0.4, abs(dot(smooth_normal, vec3<f32>(0.0, 0.0, 1.0))));
    }
    pbr_input.material.base_color = vec4<f32>(stone, 1.0);
    pbr_input.N = normal;
    pbr_input.material.perceptual_roughness = clamp(0.82 + rock.hollow * 0.1 + rock.hole * 0.15 + rock.crack * 0.1, 0.5, 0.97) - glow * 0.4;
    pbr_input.material.emissive = vec4<f32>(carving.glow.rgb * glow * 2.5, 0.0);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
