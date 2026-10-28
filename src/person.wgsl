#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

struct Living {
    parts: array<vec4<f32>, 19>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> living: Living;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var skin: texture_2d<f32>;

const FLESH: i32 = 0;
const HAIR: i32 = 1;
const CLOTH: i32 = 2;
const LEATHER: i32 = 3;

const PORES: f32 = 650.0;
const FURROWS: f32 = 380.0;
const THREADS: f32 = 520.0;
const GRAIN: f32 = 420.0;

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

fn fall(low: f32, high: f32, x: f32) -> f32 {
    return 1.0 - smoothstep(low, high, x);
}

fn seen(frequency: f32, footprint: f32) -> f32 {
    return 1.0 - smoothstep(0.12, 0.45, frequency * footprint);
}

fn tilted(normal: vec3<f32>, slope: vec3<f32>) -> vec3<f32> {
    return normalize(normal - (slope - normal * dot(slope, normal)));
}

struct Relief {
    height: f32,
    slope: vec3<f32>,
    lost: f32,
}

fn rough_surface(p: vec3<f32>, footprint: f32, first: f32, amplitude: f32, octaves: i32) -> Relief {
    var relief = Relief(0.0, vec3<f32>(0.0), 0.0);
    var frequency = first;
    var height = amplitude;
    for (var octave = 0; octave < octaves; octave++) {
        let fade = seen(frequency, footprint);
        let steep = height * frequency;
        relief.lost += steep * steep * (1.0 - fade);
        if fade > 0.0 {
            let n = wave(p * frequency + f32(octave) * 13.7, u32(octave) * 5u + 11u);
            relief.height += n.x * height * fade;
            relief.slope += n.yzw * steep * fade;
        }
        frequency *= 2.03;
        height *= 0.55;
    }
    return relief;
}

struct Look {
    color: vec3<f32>,
    slope: vec3<f32>,
    rough: f32,
    sheen: f32,
}

fn texel_of(part: u32, at: vec3<f32>) -> vec2<i32> {
    let a = living.parts[part * 3u];
    let b = living.parts[part * 3u + 1u];
    let c = living.parts[part * 3u + 2u];
    let size = vec3<f32>(a.w, b.w, c.z);
    let q = (at - a.xyz) / b.xyz;
    let m = abs(q);
    var rect = vec4<f32>(0.0);
    var st = vec2<f32>(0.0);
    let u = c.x;
    let v = c.y;
    if m.x >= m.y && m.x >= m.z {
        if q.x > 0.0 {
            rect = vec4<f32>(u, v + size.z, size.z, size.y);
            st = vec2<f32>(-q.z, -q.y);
        } else {
            rect = vec4<f32>(u + size.z + size.x, v + size.z, size.z, size.y);
            st = vec2<f32>(q.z, -q.y);
        }
    } else if m.y >= m.z {
        if q.y > 0.0 {
            rect = vec4<f32>(u + size.z, v, size.x, size.z);
            st = vec2<f32>(-q.x, -q.z);
        } else {
            rect = vec4<f32>(u + size.z + size.x, v, size.x, size.z);
            st = vec2<f32>(-q.x, q.z);
        }
    } else {
        if q.z < 0.0 {
            rect = vec4<f32>(u + size.z, v + size.z, size.x, size.y);
            st = vec2<f32>(-q.x, -q.y);
        } else {
            rect = vec4<f32>(u + 2.0 * size.z + size.x, v + size.z, size.x, size.y);
            st = vec2<f32>(q.x, -q.y);
        }
    }
    let inside = clamp(st * 0.5 + 0.5, vec2<f32>(0.0), vec2<f32>(0.999));
    return vec2<i32>(floor(rect.xy + inside * rect.zw));
}

fn flesh(p: vec3<f32>, base: vec3<f32>, footprint: f32, part: u32, n: vec3<f32>) -> Look {
    var look = Look(base, vec3<f32>(0.0), 0.5, 0.0);
    let mottle = noise3(p * 30.0) * 0.5 + noise3(p * 90.0 + 5.0) * 0.3 + noise3(p * 300.0 + 2.0) * 0.2;
    let blood = noise3(p * 12.0 + 21.0) * 0.7 + noise3(p * 45.0 + 4.0) * 0.3;
    var color = base * (0.86 + mottle * 0.28);
    var flush = smoothstep(0.4, 0.8, blood) * 0.45;
    var shadow = 0.0;
    if part == 0u {
        let cheek = vec2<f32>(abs(p.x) - 0.085, p.y - 0.17);
        flush += exp(-dot(cheek, cheek) / 0.0015) * fall(-0.1, 0.0, p.z) * 0.7;
        let nose = vec2<f32>(p.x, p.y - 0.18);
        flush += exp(-dot(nose, nose) / 0.0006) * fall(-0.22, -0.17, p.z) * 0.6;
        flush += smoothstep(0.14, 0.17, abs(p.x)) * 0.5;
        let socket = vec2<f32>(abs(p.x) - 0.06, p.y - 0.205);
        shadow = exp(-dot(socket * vec2<f32>(0.8, 1.4), socket * vec2<f32>(0.8, 1.4)) / 0.0012) * 0.3;
    }
    if part >= 2u && p.y < -0.6 {
        flush += fall(-0.74, -0.66, p.y) * 0.5;
    }
    color = mix(color, color * vec3<f32>(1.15, 0.78, 0.76), clamp(flush, 0.0, 1.0));
    color = mix(color, color * vec3<f32>(0.72, 0.62, 0.66), shadow);
    let freckle = cells(p * 140.0, 51u);
    let speck = (1.0 - smoothstep(0.1, 0.2, freckle.nearest)) * select(0.0, 1.0, chance(freckle.id, 52u) < 0.07) * seen(140.0, footprint);
    color = mix(color, color * vec3<f32>(0.65, 0.52, 0.45), speck * 0.55);

    let relief = rough_surface(p, footprint, 70.0, 0.0004, 7);
    var slope = relief.slope;
    var shade = 1.0;
    let warp = vec3<f32>(noise3(p * 300.0), noise3(p * 300.0 + 7.0), noise3(p * 300.0 + 13.0)) - 0.5;
    let furrows = cells(p * FURROWS + warp * 1.2, 53u);
    let groove_fade = seen(FURROWS * 1.6, footprint);
    if groove_fade > 0.0 {
        let width = 0.14;
        let groove = (1.0 - smoothstep(0.0, width, furrows.gap)) * groove_fade;
        shade *= 1.0 - groove * 0.22;
        let dome = 1.0 - smoothstep(0.0, 0.7, furrows.nearest);
        slope -= furrows.offset * dome * 0.4 * groove_fade;
        let fine = cells(p * FURROWS * 3.1 + warp * 2.0, 54u);
        let fine_fade = seen(FURROWS * 3.1 * 2.5, footprint);
        shade *= 1.0 - (1.0 - smoothstep(0.0, 0.12, fine.gap)) * 0.14 * fine_fade;
    }
    let pore_fade = seen(PORES * 1.1, footprint);
    if pore_fade > 0.0 {
        let pores = cells(p * PORES, 57u);
        let radius = 0.18 + chance(pores.id, 58u) * 0.16;
        let present = select(0.35, 1.0, chance(pores.id, 59u) < 0.55);
        let pit = (1.0 - smoothstep(radius * 0.4, radius, pores.nearest)) * pore_fade * present;
        let rim = smoothstep(radius, radius * 1.4, pores.nearest) * (1.0 - smoothstep(radius * 1.4, radius * 2.0, pores.nearest)) * pore_fade * present;
        shade *= 1.0 - pit * 0.42;
        shade *= 1.0 + rim * 0.04;
        slope += pores.offset / max(pores.nearest, 0.001) * (pit * 0.9 - rim * 0.25);
    }
    let creases = smoothstep(0.6, 0.8, noise3(vec3<f32>(p.x * 30.0, p.y * 240.0, p.z * 30.0))) * seen(240.0, footprint);
    shade *= 1.0 - creases * 0.12 * select(0.0, 1.0, part >= 2u);
    look.color = color * shade;
    look.slope = slope;
    var oil = 0.0;
    if part == 0u {
        oil = smoothstep(0.24, 0.3, p.y) * fall(-0.18, -0.12, p.z) + exp(-(p.x * p.x) / 0.001) * fall(-0.21, -0.16, p.z);
    }
    look.rough = clamp(0.5 - oil * 0.16 + (mottle - 0.5) * 0.12 + (1.0 - shade) * 0.4 + relief.lost * 0.2, 0.3, 0.8);
    look.sheen = 0.12;
    return look;
}

fn lips(p: vec3<f32>, n: vec3<f32>, look: Look, footprint: f32) -> Look {
    var out = look;
    let mouth = living.parts[18].xy;
    let d = p.xy - mouth;
    let half_width = 0.038;
    let across = clamp(d.x / half_width, -1.0, 1.0);
    let bow = 1.0 - across * across;
    let pixel = max(footprint, 0.0003);
    let parting = -0.001 + 0.0015 * across * across;
    let upper = parting + (0.0105 - 0.004 * exp(-pow(d.x / 0.007, 2.0))) * pow(bow, 0.6);
    let lower = parting - 0.013 * pow(bow, 0.75);
    let front = smoothstep(0.2, 0.5, -n.z);
    let inside = (1.0 - smoothstep(-pixel, pixel, d.y - upper)) * (1.0 - smoothstep(-pixel, pixel, lower - d.y)) * step(abs(d.x), half_width * 1.2) * front;
    let line = (1.0 - smoothstep(0.0004, 0.0012 + pixel, abs(d.y - parting))) * fall(0.7, 1.05, abs(across)) * front;
    let furrows = wave(vec3<f32>(d.x * 700.0, d.y * 50.0, 3.0), 61u);
    let fade = seen(700.0, footprint);
    let lower_lip = step(d.y, parting);
    let tint = mix(vec3<f32>(1.1, 0.68, 0.68), vec3<f32>(1.15, 0.72, 0.72), lower_lip);
    let lip_color = look.color * tint * (1.0 - smoothstep(-0.1, 0.5, -furrows.x) * 0.18 * fade)
        * (1.0 + lower_lip * fall(-0.008, 0.0, d.y - parting) * 0.15);
    out.color = mix(look.color, lip_color, inside);
    out.color = mix(out.color, look.color * vec3<f32>(0.25, 0.12, 0.12), line);
    out.rough = mix(look.rough, 0.3, inside);
    out.slope = mix(look.slope, vec3<f32>(furrows.y, 0.0, 0.0) * 0.004 * fade, inside);
    return out;
}

struct Strands {
    look: Look,
    flow: vec3<f32>,
}

fn hair(p: vec3<f32>, base: vec3<f32>, n: vec3<f32>) -> Strands {
    var look = Look(base, vec3<f32>(0.0), 0.42, 0.0);
    let centre = vec3<f32>(0.0, 0.25, 0.015);
    let axis = normalize(vec3<f32>(0.0, 0.22, 0.055));
    let ahead = normalize(vec3<f32>(0.0, 0.0, -1.0) - axis * dot(vec3<f32>(0.0, 0.0, -1.0), axis));
    let beside = cross(axis, ahead);
    let d = normalize(p - centre);
    let theta = acos(clamp(dot(d, axis), -1.0, 1.0));
    let whorl = 1.6 * exp(-theta * 3.0);
    let phi = atan2(dot(d, beside), dot(d, ahead)) + whorl;
    let meridian = normalize(d * dot(d, axis) - axis + beside * 0.0001);
    let flow = normalize(meridian - n * dot(meridian, n));
    let across = normalize(cross(n, flow));

    let wander = noise3(vec3<f32>(theta * 9.0, phi * 2.0, 1.0)) - 0.5;
    let clump_coord = phi * 45.0 + wander * 2.5 + noise3(vec3<f32>(theta * 30.0, phi * 20.0, 5.0)) * 0.6;
    let clump_fade = 1.0 - smoothstep(0.2, 0.5, fwidth(clump_coord));
    let clump_within = fract(clump_coord);
    let clump_id = floor(clump_coord);
    let clump_round = clump_within * 2.0 - 1.0;
    let clump_profile = sqrt(max(1.0 - clump_round * clump_round, 0.0));
    let clump_tone = 0.85 + chance(vec3<i32>(i32(clump_id), 3, 0), 75u) * 0.3;
    var shade = mix(1.0, (0.7 + 0.3 * clump_profile) * clump_tone, clump_fade);
    var slope = across * (-clump_round / max(clump_profile, 0.3)) * 0.12 * clump_fade;

    let mid_coord = phi * 170.0 + wander * 5.0 + clump_round * 1.2;
    let mid_fade = 1.0 - smoothstep(0.15, 0.4, fwidth(mid_coord));
    if mid_fade > 0.0 {
        let lane = floor(mid_coord);
        let round = (mid_coord - lane) * 2.0 - 1.0;
        let profile = sqrt(max(1.0 - round * round, 0.0));
        let tone = 0.7 + chance(vec3<i32>(i32(lane), i32(floor(theta * 5.0 + chance(vec3<i32>(i32(lane), 0, 2), 77u) * 4.0)), 2), 78u) * 0.6;
        shade *= mix(1.0, (0.5 + 0.5 * profile) * tone, mid_fade);
        slope += across * (-round / max(profile, 0.3)) * 0.18 * mid_fade;
    }
    let lock_coord = phi * 320.0 + wander * 8.0 + clump_round * 1.5;
    let lock_fade = 1.0 - smoothstep(0.15, 0.4, fwidth(lock_coord));
    if lock_fade > 0.0 {
        let lane = floor(lock_coord);
        let round = (lock_coord - lane) * 2.0 - 1.0;
        let profile = sqrt(max(1.0 - round * round, 0.0));
        let tone = 0.8 + chance(vec3<i32>(i32(lane), i32(floor(theta * 7.0)), 1), 76u) * 0.4;
        shade *= mix(1.0, (0.6 + 0.4 * profile) * tone, lock_fade);
        slope += across * (-round / max(profile, 0.3)) * 0.15 * lock_fade;
    }
    let strand_coord = phi * 1400.0 + wander * 30.0 + clump_round * 3.0;
    let strand_fade = 1.0 - smoothstep(0.15, 0.4, fwidth(strand_coord));
    if strand_fade > 0.0 {
        let lane = floor(strand_coord);
        let within = strand_coord - lane;
        let piece = floor(theta * 18.0 + chance(vec3<i32>(i32(lane), 0, 0), 72u) * 9.0);
        let id = vec3<i32>(i32(lane), i32(piece), 0);
        let tone = 0.7 + chance(id, 73u) * 0.55;
        let round = within * 2.0 - 1.0;
        let profile = sqrt(max(1.0 - round * round, 0.0));
        shade *= mix(1.0, (0.5 + 0.5 * profile) * tone, strand_fade);
        slope += across * (-round / max(profile, 0.3)) * 0.12 * strand_fade;
        let scales = fract(theta * 4000.0 + chance(id, 74u));
        shade *= mix(1.0, 0.93 + 0.07 * smoothstep(0.0, 0.2, scales), strand_fade * (1.0 - smoothstep(0.2, 0.5, fwidth(theta * 4000.0))));
    }
    let depth = smoothstep(0.0, 0.6, theta) * (1.0 - clump_profile * 0.5);
    look.color = base * shade * mix(1.0, 0.85, depth * clump_fade);
    look.slope = slope;
    look.rough = 0.4 + (1.0 - clump_profile) * 0.15 * clump_fade;
    look.sheen = 0.2;
    return Strands(look, flow);
}

fn around(p: vec3<f32>, part: u32, side: f32) -> vec2<f32> {
    let flat = vec2<f32>(p.x * side, p.z);
    let angle = atan2(flat.y, flat.x);
    let radius = select(length(flat), 0.08, part >= 2u);
    let girth = select(0.16, radius, part >= 2u);
    return vec2<f32>(angle * girth, p.y);
}

fn cloth(p: vec3<f32>, base: vec3<f32>, footprint: f32, part: u32, side: f32, n: vec3<f32>) -> Look {
    var look = Look(base, vec3<f32>(0.0), 0.86, 0.6);
    let uv = around(p, part, side);
    let flat = vec2<f32>(p.x * side, p.z);
    let r = max(length(flat), 0.001);
    let tangent_u = vec3<f32>(-flat.y / r * side, 0.0, flat.x / r);
    let tangent_v = vec3<f32>(0.0, 1.0, 0.0);

    let folds = wave(vec3<f32>(uv.x * 9.0, uv.y * 14.0, f32(part) * 3.0), 81u);
    let joint = select(-0.33, -0.37, part >= 4u);
    let bend = select(0.0, exp(-pow((p.y - joint) / 0.05, 2.0)), part >= 2u);
    let creases = wave(vec3<f32>(uv.x * 30.0, uv.y * 90.0, 7.0), 82u);
    var slope = (tangent_u * folds.y * 9.0 + tangent_v * folds.z * 14.0) * 0.0035
        + (tangent_u * creases.y * 30.0 + tangent_v * creases.z * 90.0) * 0.0006 * (0.4 + bend * 2.0);
    var shade = 0.94 + folds.x * 0.08 - smoothstep(0.1, 0.5, -creases.x) * 0.06 * (0.4 + bend * 2.0);

    let weave_fade = seen(THREADS, footprint);
    let fuzz = noise3(p * 900.0) * 0.6 + noise3(p * 2400.0) * 0.4;
    shade *= mix(1.0, 0.9 + fuzz * 0.2, seen(900.0, footprint));
    if weave_fade > 0.0 {
        let grid = uv * THREADS;
        let cell = floor(grid);
        let f = grid - cell;
        let over = ((i32(cell.x) + i32(cell.y)) % 4 + 4) % 4 < 2;
        let thread_tone = 0.93 + chance(vec3<i32>(select(i32(cell.y), i32(cell.x), over), select(1, 0, over), i32(part)), 83u) * 0.14;
        let wp = 1.0 - pow(2.0 * f.x - 1.0, 2.0);
        let wf = 1.0 - pow(2.0 * f.y - 1.0, 2.0);
        let arch_warp = 0.55 + 0.45 * sin(3.14159 * f.y);
        let arch_weft = 0.55 + 0.45 * sin(3.14159 * f.x);
        var height = 0.0;
        var grad = vec2<f32>(0.0);
        if over {
            height = sqrt(max(wp, 0.0)) * arch_warp;
            grad = vec2<f32>(-(2.0 * f.x - 1.0) * 2.0 / max(sqrt(max(wp, 0.0)), 0.2), 0.45 * 3.14159 * cos(3.14159 * f.y));
        } else {
            height = sqrt(max(wf, 0.0)) * arch_weft;
            grad = vec2<f32>(0.45 * 3.14159 * cos(3.14159 * f.x), -(2.0 * f.y - 1.0) * 2.0 / max(sqrt(max(wf, 0.0)), 0.2));
        }
        let fibre = wave(vec3<f32>(select(grid.x * 6.0, grid.x * 0.6, over), select(grid.y * 0.6, grid.y * 6.0, over), 2.0), 85u);
        shade *= mix(1.0, (0.6 + 0.4 * height) * (0.9 + fibre.x * 0.2) * thread_tone * select(0.94, 1.05, over), weave_fade);
        slope += (tangent_u * grad.x + tangent_v * grad.y) * 0.18 * weave_fade;
    }
    let angle = abs(atan2(flat.y, flat.x));
    let seam_dist = min(3.14159 - angle, select(9.0, angle, part == 1u)) * r;
    let seam_width = 0.012;
    let cut = 1.0 - smoothstep(seam_width * 0.1, seam_width * 0.35, seam_dist);
    shade *= 1.0 - cut * 0.22;
    let stitch_row = fract(p.y * 220.0);
    let stitched = (1.0 - smoothstep(0.0012, 0.002, abs(seam_dist - 0.006))) * step(stitch_row, 0.6) * seen(440.0, footprint);
    shade *= 1.0 - stitched * 0.3;
    look.color = base * shade;
    look.slope = slope;
    look.rough = 0.88;
    return look;
}

fn leather(p: vec3<f32>, base: vec3<f32>, footprint: f32, part: u32) -> Look {
    var look = Look(base, vec3<f32>(0.0), 0.55, 0.1);
    let grain = cells(p * GRAIN, 91u);
    let fade = seen(GRAIN * 1.1, footprint);
    let dome = (1.0 - smoothstep(0.0, 0.7, grain.nearest));
    var slope = -grain.offset * dome * 0.7 * fade;
    var shade = (1.0 - (1.0 - smoothstep(0.0, 0.14, grain.gap)) * 0.28 * fade) * (0.94 + dome * 0.08 * fade);
    let ankle = select(0.0, exp(-pow((p.y + 0.6) / 0.05, 2.0)), part >= 4u);
    let wrinkle = wave(vec3<f32>(p.x * 25.0, p.y * 160.0, p.z * 25.0), 92u);
    let sparse = smoothstep(0.55, 0.75, noise3(vec3<f32>(p.x * 30.0, p.y * 12.0, p.z * 30.0)));
    let fold = (1.0 - smoothstep(0.0, 0.08, abs(wrinkle.x))) * max(ankle, sparse * 0.5);
    shade *= 1.0 - fold * 0.3 * seen(160.0, footprint);
    slope += wrinkle.yzw * 0.0025 * sign(wrinkle.x) * fold;
    let scuff = smoothstep(0.62, 0.8, noise3(p * 45.0));
    var color = mix(base * shade, base * 1.35 + 0.02, scuff * 0.4);
    var rough = 0.5 + scuff * 0.3 + noise3(p * 80.0) * 0.1;
    if part >= 4u {
        let sole = fall(-0.702, -0.698, p.y);
        color = mix(color, vec3<f32>(0.02, 0.018, 0.016), sole);
        rough = mix(rough, 0.85, sole);
        let welt = 1.0 - smoothstep(0.0008, 0.0016, abs(p.y + 0.693));
        let dashes = step(fract((p.x + p.z * 1.3) * 160.0), 0.55);
        color *= 1.0 - welt * dashes * 0.5 * seen(320.0, footprint);
        color = mix(color, color * 0.6, (1.0 - smoothstep(0.0, 0.004, abs(p.y + 0.6))) * 0.5);
    }
    look.color = color;
    look.slope = slope;
    look.rough = rough;
    return look;
}

fn eyeball(n: vec3<f32>, p: vec3<f32>, iris: vec3<f32>, lid: vec3<f32>, footprint: f32) -> Look {
    var look = Look(vec3<f32>(0.0), vec3<f32>(0.0), 0.04, 0.0);
    let toward = -n.z;
    let angle = acos(clamp(toward, -1.0, 1.0));
    let turn = atan2(n.y, n.x);
    let iris_edge = 0.44;
    let pupil = 0.15;
    let pixel = max(footprint * 50.0, 0.01);
    let veins = cells(n * 30.0, 101u);
    let vein = (1.0 - smoothstep(0.0, 0.04, veins.gap)) * smoothstep(0.6, 1.2, angle) * select(0.0, 1.0, chance(veins.id, 102u) < 0.4);
    var sclera = vec3<f32>(0.74, 0.7, 0.66) * (0.95 + noise3(n * 20.0) * 0.08);
    sclera = mix(sclera, vec3<f32>(0.6, 0.28, 0.26), vein * 0.3 + smoothstep(0.7, 1.2, angle) * 0.2);
    let fibres = wave(vec3<f32>(turn * 24.0, angle * 6.0, 1.0), 103u);
    let crypts = wave(vec3<f32>(turn * 8.0, angle * 14.0, 4.0), 104u);
    let collarette = 1.0 - smoothstep(0.0, 0.05, abs(angle - 0.27));
    var ring = iris * (0.7 + fibres.x * 0.6) * (1.0 - smoothstep(0.0, 0.3, -crypts.x) * 0.4);
    ring = mix(ring, iris * 1.7 + vec3<f32>(0.06, 0.05, 0.02), collarette * 0.4);
    ring *= 1.0 - smoothstep(iris_edge - 0.12, iris_edge, angle) * 0.75;
    let in_iris = 1.0 - smoothstep(iris_edge - pixel, iris_edge + pixel, angle);
    let in_pupil = 1.0 - smoothstep(pupil - pixel, pupil + pixel, angle);
    var color = mix(mix(sclera, ring, in_iris), vec3<f32>(0.006), in_pupil);

    let inner = -sign(p.x) * n.x;
    let spread_x = clamp(n.x / 0.86, -1.0, 1.0);
    let bow = 1.0 - spread_x * spread_x;
    let upper = 0.5 * bow - 0.04 + inner * 0.04;
    let lower = -0.5 * bow - 0.02;
    let above = n.y - upper;
    let below = lower - n.y;
    let lidded = max(smoothstep(-pixel, pixel, above), smoothstep(-pixel, pixel, below));
    let shadow = smoothstep(-0.25, 0.0, above) * 0.65 + smoothstep(-0.15, 0.0, below) * 0.3;
    color *= 1.0 - shadow * (1.0 - lidded);
    let caruncle = smoothstep(0.55, 0.8, inner) * (1.0 - lidded);
    color = mix(color, vec3<f32>(0.55, 0.22, 0.2), caruncle * 0.7);
    let lash_line = 1.0 - smoothstep(0.0, 0.05 + pixel, abs(above - 0.02));
    let lashes = lash_line * (0.7 + 0.3 * step(0.5, fract(turn * 60.0)));
    let lower_lash = (1.0 - smoothstep(0.0, 0.03 + pixel, abs(below - 0.015))) * 0.5;
    let skin_tone = lid * 0.9 * vec3<f32>(1.05, 0.9, 0.88) * (0.9 + noise3(n * 40.0) * 0.15) * mix(1.0, 0.8, fall(0.0, 0.1, above) * step(0.0, above));
    let crease = (1.0 - smoothstep(0.0, 0.04, abs(above - 0.22))) * 0.3;
    var lid_color = skin_tone * (1.0 - crease);
    lid_color = mix(lid_color, vec3<f32>(0.02, 0.015, 0.012), max(lashes, lower_lash));
    look.color = mix(color, lid_color, lidded);
    look.rough = mix(0.04, 0.5, lidded);
    look.sheen = 0.0;
    return look;
}

fn unpacked(tint: u32) -> vec3<f32> {
    return pow(vec3<f32>(f32(tint & 31u), f32((tint >> 5u) & 31u), f32((tint >> 10u) & 31u)) / 31.0, vec3<f32>(2.2));
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = vec3<f32>(in.uv.x, in.uv.y, in.uv_b.x);
    let local_normal = normalize(in.color.xyz);
    let code = u32(round(in.color.w));
    let part = code & 7u;
    let finish = (code >> 3u) & 3u;
    let tint = code >> 5u;
    let footprint = length(fwidth(p));
    let side = living.parts[part * 3u + 2u].w;

    let up = normalize(in.world_normal);
    let along = normalize(in.world_tangent.xyz - up * dot(in.world_tangent.xyz, up));
    let turn = mat3x3<f32>(along, up, cross(along, up));

    var look: Look;
    var flow = vec3<f32>(0.0);
    var anisotropy = 0.0;
    if finish == 1u {
        let iris = unpacked(tint);
        let lid = unpacked(u32(round(in.uv_b.y)));
        look = eyeball(local_normal, p, iris, lid, footprint);
    } else {
        let warp = vec3<f32>(noise3(p * 55.0), noise3(p * 55.0 + 9.0), noise3(p * 55.0 + 17.0)) - 0.5
            + (vec3<f32>(noise3(p * 260.0), noise3(p * 260.0 + 4.0), noise3(p * 260.0 + 8.0)) - 0.5) * 0.35;
        let ragged = textureLoad(skin, texel_of(part, p + warp * 0.012), 0);
        let neat = textureLoad(skin, texel_of(part, p + warp * 0.003), 0);
        let texel = select(neat, ragged, i32(round(ragged.a * 255.0 / 40.0)) == HAIR || i32(round(neat.a * 255.0 / 40.0)) == HAIR);
        let kind = i32(round(texel.a * 255.0 / 40.0));
        let base = texel.rgb;
        if kind == HAIR {
            let strands = hair(p, base, local_normal);
            look = strands.look;
            flow = strands.flow;
            anisotropy = 0.6;
        } else if kind == CLOTH {
            look = cloth(p, base, footprint, part, side, local_normal);
        } else if kind == LEATHER {
            look = leather(p, base, footprint, part);
        } else {
            look = flesh(p, base, footprint, part, local_normal);
            if part == 0u {
                look = lips(p, local_normal, look, footprint);
            }
        }
    }
    let normal = turn * tilted(local_normal, look.slope);
    let view = pbr_input.V;
    let grazing = pow(1.0 - clamp(dot(normal, view), 0.0, 1.0), 4.0);
    pbr_input.material.base_color = vec4<f32>(look.color + look.color * look.sheen * grazing, 1.0);
    pbr_input.N = normal;
    pbr_input.world_normal = turn * local_normal;
    pbr_input.material.perceptual_roughness = look.rough;
    let streak = normalize(turn * flow - normal * dot(turn * flow, normal) + along * 0.0001);
    pbr_input.anisotropy_strength = anisotropy;
    pbr_input.anisotropy_T = streak;
    pbr_input.anisotropy_B = normalize(cross(normal, streak));
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
