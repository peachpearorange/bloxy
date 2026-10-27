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

const FIRST: f32 = 2.7;
const LACUNARITY: f32 = 1.93;
const HURST: f32 = 0.72;
const ROUGH: f32 = 0.06;
const OCTAVES: i32 = 16;
const GROUNDMASS: f32 = 2200.0;
const PHENOCRYSTS: f32 = 70.0;
const VESICLES: f32 = 38.0;
const JOINTS: f32 = 3.5;
const SCALLOPS: f32 = 6.0;
const FLECKS: f32 = 55.0;
const BASALT: vec3<f32> = vec3<f32>(0.036, 0.045, 0.07);

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

fn spread(cell: vec3<i32>, salt: u32) -> vec3<f32> {
    let h = scramble(cell, salt);
    return vec3<f32>(f32(h & 1023u), f32((h >> 10u) & 1023u), f32((h >> 20u) & 1023u)) / 1023.0;
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

fn wave(p: vec3<f32>, salt: u32) -> vec4<f32> {
    let i = floor(p);
    let f = p - i;
    let c = vec3<i32>(i);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
    let ga = spread(c, salt) * 2.0 - 1.0;
    let gb = spread(c + vec3(1, 0, 0), salt) * 2.0 - 1.0;
    let gc = spread(c + vec3(0, 1, 0), salt) * 2.0 - 1.0;
    let gd = spread(c + vec3(1, 1, 0), salt) * 2.0 - 1.0;
    let ge = spread(c + vec3(0, 0, 1), salt) * 2.0 - 1.0;
    let gf = spread(c + vec3(1, 0, 1), salt) * 2.0 - 1.0;
    let gg = spread(c + vec3(0, 1, 1), salt) * 2.0 - 1.0;
    let gh = spread(c + vec3(1, 1, 1), salt) * 2.0 - 1.0;
    let va = dot(ga, f);
    let vb = dot(gb, f - vec3(1.0, 0.0, 0.0));
    let vc = dot(gc, f - vec3(0.0, 1.0, 0.0));
    let vd = dot(gd, f - vec3(1.0, 1.0, 0.0));
    let ve = dot(ge, f - vec3(0.0, 0.0, 1.0));
    let vf = dot(gf, f - vec3(1.0, 0.0, 1.0));
    let vg = dot(gg, f - vec3(0.0, 1.0, 1.0));
    let vh = dot(gh, f - vec3(1.0, 1.0, 1.0));
    let value = va + u.x * (vb - va) + u.y * (vc - va) + u.z * (ve - va)
        + u.x * u.y * (va - vb - vc + vd) + u.y * u.z * (va - vc - ve + vg)
        + u.z * u.x * (va - vb - ve + vf) + u.x * u.y * u.z * (-va + vb + vc - vd + ve - vf - vg + vh);
    let slope = ga + u.x * (gb - ga) + u.y * (gc - ga) + u.z * (ge - ga)
        + u.x * u.y * (ga - gb - gc + gd) + u.y * u.z * (ga - gc - ge + gg)
        + u.z * u.x * (ga - gb - ge + gf) + u.x * u.y * u.z * (-ga + gb + gc - gd + ge - gf - gg + gh)
        + du * (vec3<f32>(vb - va, vc - va, ve - va)
            + u.yzx * vec3<f32>(va - vb - vc + vd, va - vc - ve + vg, va - vb - ve + vf)
            + u.zxy * vec3<f32>(va - vb - ve + vf, va - vb - vc + vd, va - vc - ve + vg)
            + u.yzx * u.zxy * (-va + vb + vc - vd + ve - vf - vg + vh));
    return vec4<f32>(value, slope);
}

const TURN: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>(0.00, 0.80, 0.60),
    vec3<f32>(-0.80, 0.36, -0.48),
    vec3<f32>(-0.60, -0.48, 0.64));

struct Relief {
    height: f32,
    slope: vec3<f32>,
    speckle: f32,
    fine: f32,
    seen: f32,
    lost: f32,
}

fn weierstrass(p: vec3<f32>, footprint: f32) -> Relief {
    var relief = Relief(0.0, vec3<f32>(0.0), 0.0, 0.0, 0.0, 0.0);
    var frequency = FIRST;
    var amplitude = ROUGH * pow(FIRST, -HURST);
    let fall = pow(LACUNARITY, -HURST);
    var turn = mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0));
    var weight = 1.0;
    var total = 0.0;
    var fine_total = 0.0;
    for (var octave = 0; octave < OCTAVES; octave++) {
        let fade = 1.0 - smoothstep(0.15, 0.45, frequency * footprint);
        let steep = amplitude * frequency;
        relief.lost += steep * steep * (1.0 - fade);
        total += weight;
        let fine = smoothstep(80.0, 400.0, frequency);
        fine_total += weight * fine;
        if fade > 0.0 {
            let n = wave(turn * p * frequency + f32(octave) * 17.31, u32(octave) * 7u + 3u);
            let crisp = smoothstep(60.0, 300.0, frequency) * 0.8;
            let fold = mix(1.0, -sign(n.x), crisp);
            let value = mix(n.x, 0.3 - abs(n.x), crisp);
            relief.height += value * amplitude * fade;
            relief.slope += transpose(turn) * n.yzw * fold * steep * fade;
            relief.speckle += value * weight * fade;
            relief.fine += value * weight * fine * fade;
            relief.seen += weight * fine * fade;
        }
        frequency *= LACUNARITY;
        amplitude *= fall;
        weight *= 0.88;
        turn = TURN * turn;
    }
    relief.speckle /= total * 0.35;
    relief.seen /= max(fine_total, 0.001);
    relief.fine /= max(fine_total, 0.001) * 0.35;
    return relief;
}

struct Cells {
    nearest: f32,
    gap: f32,
    id: vec3<i32>,
    offset: vec3<f32>,
}

fn cells(p: vec3<f32>, salt: u32) -> Cells {
    let base = vec3<i32>(floor(p));
    var found = Cells(9.0, 9.0, base, vec3<f32>(0.0));
    var second = 9.0;
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let cell = base + vec3<i32>(x, y, z);
                let offset = p - vec3<f32>(cell) - spread(cell, salt);
                let distance = length(offset);
                if distance < found.nearest {
                    second = found.nearest;
                    found.nearest = distance;
                    found.id = cell;
                    found.offset = offset;
                } else if distance < second {
                    second = distance;
                }
            }
        }
    }
    found.gap = second - found.nearest;
    return found;
}

struct Mineral {
    tone: vec3<f32>,
    rough: f32,
    slope: vec3<f32>,
    pale: f32,
    phase: f32,
}

fn groundmass(p: vec3<f32>, footprint: f32) -> Mineral {
    let seen = 1.0 - smoothstep(0.12, 0.4, footprint * GROUNDMASS);
    var mineral = Mineral(BASALT, 0.72, vec3<f32>(0.0), 0.0, 0.0);
    if seen > 0.0 {
        let grain = cells(p * GROUNDMASS, 71u);
        let roll = chance(grain.id, 72u);
        let shade = chance(grain.id, 73u);
        var tone = BASALT * (0.88 + shade * 0.2);
        var rough = 0.78;
        var pale = 0.0;
        var tilt = (spread(grain.id, 74u) * 2.0 - 1.0) * 0.12;
        if roll > 0.96 {
            tone = select(vec3<f32>(0.06, 0.066, 0.04), vec3<f32>(0.075, 0.05, 0.035), shade > 0.6);
            rough = 0.45;
        } else if roll > 0.86 {
            tone = BASALT * (0.62 + shade * 0.15);
            rough = 0.6;
        } else if roll > 0.68 {
            tone = BASALT * (1.25 + shade * 0.3);
            rough = 0.32;
            tilt *= 3.0;
            pale = select(0.0, 1.0, shade > 0.8);
        }
        let edge = smoothstep(0.0, 0.25, grain.gap);
        tone *= mix(0.8, 1.0, edge);
        mineral = Mineral(
            mix(BASALT, tone, seen),
            mix(0.72, rough, seen),
            tilt * seen * edge,
            pale * seen,
            chance(grain.id, 75u));
    }
    return mineral;
}

struct Crystal {
    tone: vec3<f32>,
    cover: f32,
    rough: f32,
    slope: vec3<f32>,
    phase: f32,
}

fn phenocryst(p: vec3<f32>, footprint: f32) -> Crystal {
    let pixel = max(footprint * PHENOCRYSTS, 0.002);
    var crystal = Crystal(vec3<f32>(0.0), 0.0, 0.5, vec3<f32>(0.0), 0.0);
    if pixel < 0.25 {
        let cell = cells(p * PHENOCRYSTS, 81u);
        let axis = normalize(spread(cell.id, 82u) * 2.0 - 1.0);
        let squeezed = cell.offset - axis * dot(cell.offset, axis) * 0.72;
        let size = 0.06 + chance(cell.id, 83u) * 0.12;
        let present = select(0.0, 1.0, chance(cell.id, 84u) < 0.12);
        let inside = 1.0 - smoothstep(size - pixel, size + pixel, length(squeezed));
        let olivine = chance(cell.id, 85u) < 0.3;
        let tone = select(vec3<f32>(0.15, 0.155, 0.155), vec3<f32>(0.075, 0.09, 0.04), olivine)
            * (0.8 + chance(cell.id, 86u) * 0.4);
        crystal = Crystal(
            tone,
            inside * present * (1.0 - smoothstep(0.1, 0.25, pixel)),
            select(0.3, 0.2, olivine),
            (spread(cell.id, 87u) * 2.0 - 1.0) * 0.15,
            chance(cell.id, 88u));
    }
    return crystal;
}

struct Pit {
    cover: f32,
    slope: vec3<f32>,
    depth: f32,
}

fn vesicle(p: vec3<f32>, footprint: f32) -> Pit {
    let pixel = max(footprint * VESICLES, 0.002);
    var pit = Pit(0.0, vec3<f32>(0.0), 1.0);
    if pixel < 0.3 {
        let cell = cells(p * VESICLES, 21u);
        let radius = 0.07 + chance(cell.id, 24u) * 0.2;
        let present = select(0.0, 1.0, chance(cell.id, 25u) < 0.22) * (1.0 - smoothstep(0.08, 0.3, pixel));
        let reach = cell.nearest / radius;
        let cover = (1.0 - smoothstep(1.0 - pixel / radius, 1.0 + pixel / radius, reach)) * present;
        let rim = smoothstep(1.0, 1.25, reach) * (1.0 - smoothstep(1.25, 1.6, reach)) * present;
        pit = Pit(cover, cell.offset / radius * 1.3 * cover - cell.offset / max(cell.nearest, 0.0001) * rim * 0.25, reach);
    }
    return pit;
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

fn bumped(normal: vec3<f32>, at: vec3<f32>, height: f32) -> vec3<f32> {
    let dx = dpdx(at);
    let dy = dpdy(at);
    let r1 = cross(dy, normal);
    let r2 = cross(normal, dx);
    let det = dot(dx, r1);
    let grad = sign(det) * (dpdx(height) * r1 + dpdy(height) * r2);
    return normalize(abs(det) * normal - grad);
}

fn tilted(normal: vec3<f32>, slope: vec3<f32>) -> vec3<f32> {
    return normalize(normal - (slope - normal * dot(slope, normal)));
}

fn twinkle(phase: f32, time: f32) -> f32 {
    return pow(max(0.0, sin(time * (1.7 + phase * 2.3) + phase * 40.0)), 24.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let at = in.world_position.xyz;
    let local = vec3<f32>(in.uv_b.x, in.uv.x, in.uv_b.y);
    let home = vec3<i32>(i32(floor(at.x)), 0, i32(floor(at.z)));
    let p = local + spread(home, 5u) * 9.0;
    let footprint = length(fwidth(local));
    let smooth_normal = normalize(in.world_normal);
    let height = in.uv.x;
    let finish = in.uv.y;
    let time = globals.time;

    let relief = weierstrass(p, footprint);
    let warp = vec3<f32>(noise3(p * 2.0), noise3(p * 2.0 + 9.0), noise3(p * 2.0 + 19.0)) - 0.5;
    let scallop = cells(p * SCALLOPS + warp * 0.9, 41u);
    let chip_depth = 0.01 + chance(scallop.id, 44u) * 0.025;
    let chip = -scallop.nearest * scallop.nearest * chip_depth;
    let chip_slope = -2.0 * scallop.offset * chip_depth * SCALLOPS;
    let hollow = 1.0 - smoothstep(-0.02, -0.002, chip);

    let mineral = groundmass(p, footprint);
    let crystal = phenocryst(p, footprint);
    let pit = vesicle(p, footprint);

    let joints = cells(p * JOINTS + vec3<f32>(noise3(p * 3.0), noise3(p * 3.0 + 7.0), 0.0) * 0.6, 31u);
    let width = max(footprint * JOINTS * 1.2, 0.002);
    let cracked = chance(joints.id, 34u) < 0.35;
    let crack = select(0.0, 1.0 - smoothstep(0.0, width, joints.gap), cracked)
        * (1.0 - smoothstep(0.3, 0.8, footprint * JOINTS * 6.0));
    let seep = select(0.0, 1.0 - smoothstep(0.0, width * 5.0, joints.gap), cracked);
    let veined = cracked && chance(joints.id, 35u) < 0.3;

    var groove = 0.0;
    var glow = 0.0;
    let pulse = 0.55 + 0.45 * sin(time * 1.6 - height * 4.0 + chance(home, 9u) * 6.28);
    if finish > 0.5 && finish < 1.5 {
        let across_axis = normalize(vec2<f32>(-smooth_normal.z, smooth_normal.x));
        let side = select(-1.0, 1.0, local.x > 0.0);
        let across = dot(local.xz - vec2<f32>(side * (POST - LEAN * height), 0.0), across_axis);
        let slot = floor((height - RUNES_FROM + RUNE_SIZE.y * 0.3) / RUNE_STEP);
        let within = vec2<f32>(across + RUNE_SIZE.x * 0.5, height - RUNES_FROM - slot * RUNE_STEP) / RUNE_SIZE;
        if slot >= 0.0 && slot < 3.0 && all(within > vec2<f32>(-0.3)) && all(within < vec2<f32>(1.3)) {
            let face = i32(round(smooth_normal.x * 2.0 + smooth_normal.z * 3.0));
            let near = rune(within * 2.0, home + vec3<i32>(face * 7, i32(slot), face)) + (relief.fine * 0.02 + warp.x * 0.05);
            groove = 1.0 - smoothstep(0.13, 0.2, near);
            let core = 1.0 - smoothstep(0.0, 0.05, near);
            let flow = noise3(vec3<f32>(within * vec2<f32>(5.0, 3.0) - vec2<f32>(0.0, time * 0.9), time * 0.35));
            let halo = exp(-near * 7.0) * (0.3 + 0.7 * smoothstep(-0.6, 0.8, -relief.fine));
            let pores = mix(0.3, smoothstep(0.35, 1.3, -relief.fine), relief.seen);
            glow = (groove * (0.06 + 1.2 * pores * flow * flow) + core * (0.2 + 0.6 * flow) + halo * 0.1 * pores) * pulse;
        }
        let ring = abs(fract((height - RUNES_FROM + 0.05) / RUNE_STEP) - 0.5);
        let band = smoothstep(0.47, 0.49, ring);
        let rising = 0.5 + 0.5 * sin(height * 6.0 - time * 2.4 + atan2(local.z, local.x - side * POST) * 2.0);
        groove = max(groove, band * 0.7);
        glow = max(glow, band * (0.25 + 0.5 * rising) * pulse);
    } else if finish > 1.5 {
        let swirl = noise3(vec3<f32>(local.x * 14.0, local.y * 14.0 - time * 0.8, local.z * 14.0 + time * 0.3));
        glow = (0.2 + 0.4 * pulse) * (0.6 + 0.8 * swirl) * smoothstep(0.1, 0.4, abs(smooth_normal.z));
    }
    let packet = pow(0.5 + 0.5 * sin(height * 7.0 - time * 1.9 + chance(joints.id, 36u) * 6.28), 8.0);
    let vein = select(0.0, crack * (0.08 + 0.8 * packet) + seep * packet * 0.15 * smoothstep(0.0, 1.0, -relief.fine), veined && finish < 1.5);
    let magic = select(0.25, 0.7, finish > 0.5 && finish < 1.5);
    let sparks = (mineral.pale * twinkle(mineral.phase, time) * 0.5
        + crystal.cover * twinkle(crystal.phase, time) * 0.4) * magic;
    let fleck_pixel = max(footprint * FLECKS, 0.002);
    var fleck = 0.0;
    if fleck_pixel < 0.3 {
        let fleck_cell = cells(p * FLECKS, 91u);
        fleck = (1.0 - smoothstep(0.05, 0.05 + fleck_pixel * 1.5, fleck_cell.nearest))
            * select(0.0, 1.0, chance(fleck_cell.id, 92u) < 0.06)
            * twinkle(chance(fleck_cell.id, 93u), time * 0.6) * magic * (1.0 - smoothstep(0.05, 0.3, fleck_pixel));
    }
    glow = max(glow, vein) + sparks + fleck;

    let mottle = noise3(p * 1.4) * 0.6 + noise3(p * 5.0 + 3.0) * 0.4;
    let crust = smoothstep(0.5, 0.75, noise3(p * 2.2 + 11.0) + smooth_normal.y * 0.15 - hollow * 0.2);
    var stone = mineral.tone * (0.85 + mottle * 0.3);
    stone = mix(stone, vec3<f32>(0.078, 0.085, 0.097) * (0.85 + mottle * 0.3), crust * 0.5);
    stone *= mix(0.5, 1.2, smoothstep(-1.3, 1.1, relief.speckle));
    stone = mix(stone, crystal.tone, crystal.cover * (1.0 - crust * 0.6));
    stone *= mix(0.55, 1.1, smoothstep(-1.2, 1.0, relief.fine));
    stone *= mix(1.0, 0.25 + 0.5 * pit.depth * pit.depth, pit.cover);
    stone *= 1.0 - crack * 0.75;
    stone *= 1.0 - hollow * 0.35;
    let lichen_at = cells(p * 7.0, 101u);
    let lichen = (1.0 - smoothstep(0.25, 0.32 + noise3(p * 60.0) * 0.1, lichen_at.nearest))
        * select(0.0, 1.0, chance(lichen_at.id, 102u) < 0.18) * crust * (1.0 - groove);
    stone = mix(stone, vec3<f32>(0.15, 0.16, 0.15) * (0.75 + noise3(p * 140.0) * 0.5), lichen * 0.7);
    let moss = smoothstep(0.52, 0.7, noise3(p * 3.1 + 7.0)) * (1.0 - smoothstep(0.1, 0.7, height))
        * smoothstep(-0.2, 0.6, smooth_normal.y + 0.3);
    stone = mix(stone, vec3<f32>(0.04, 0.09, 0.02) * (0.7 + relief.speckle * 0.25 + mottle * 0.3), moss);
    stone *= 1.0 - groove * 0.6;

    let grit = 1.0 - groove * 0.7;
    let slope = (relief.slope + chip_slope) * grit
        + mineral.slope * (1.0 - crystal.cover) * (1.0 - crust * 0.7) * grit
        + crystal.slope * crystal.cover
        + pit.slope;
    let normal = bumped(tilted(smooth_normal, slope), local, (-crack * 0.0015 - groove * 0.004));

    let rough = mix(mix(mineral.rough, crystal.rough, crystal.cover), 0.88, max(crust, lichen));
    pbr_input.material.base_color = vec4<f32>(stone, 1.0);
    pbr_input.N = normal;
    pbr_input.material.perceptual_roughness = clamp(sqrt(rough * rough + relief.lost * 0.4) + pit.cover * 0.15 + crack * 0.1, 0.25, 0.97) - glow * 0.3;
    pbr_input.material.emissive = vec4<f32>(carving.glow.rgb * glow * 2.0, 0.0);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
