#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::globals,
}

const CHROME: u32 = 0u;
const SATIN: u32 = 1u;
const ANODISED: u32 = 2u;
const RUBBER: u32 = 3u;
const BRASS: u32 = 4u;
const GLASS: u32 = 5u;
const LAMP: u32 = 6u;

const LIMB: u32 = 0u;
const HEAD: u32 = 1u;
const CHEST: u32 = 2u;
const DIAL: u32 = 3u;

const HEAD_BASE: f32 = 1.244;
const EYES: f32 = 1.346;
const DIAL_CENTRE: vec2<f32> = vec2<f32>(0.0, 1.0);
const TAU: f32 = 6.2831853;

fn scramble(cell: vec3<i32>, salt: u32) -> u32 {
    var v = bitcast<vec3<u32>>(cell) * 1664525u + vec3<u32>(1013904223u + salt * 2654435761u);
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

fn ripple(p: vec3<f32>, salt: u32) -> vec3<f32> {
    let e = 0.37;
    let here = noise3(p + f32(salt) * 17.1);
    return vec3<f32>(
        noise3(p + vec3<f32>(e, 0.0, 0.0) + f32(salt) * 17.1) - here,
        noise3(p + vec3<f32>(0.0, e, 0.0) + f32(salt) * 17.1) - here,
        noise3(p + vec3<f32>(0.0, 0.0, e) + f32(salt) * 17.1) - here
    ) / e;
}

fn fall(low: f32, high: f32, x: f32) -> f32 {
    return 1.0 - smoothstep(low, high, x);
}

fn seen(frequency: f32, footprint: f32) -> f32 {
    return 1.0 - smoothstep(0.15, 0.5, frequency * footprint);
}

fn flat(v: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return v - n * dot(v, n);
}

fn tilted(normal: vec3<f32>, slope: vec3<f32>) -> vec3<f32> {
    return normalize(normal - flat(slope, normal));
}

struct Look {
    color: vec3<f32>,
    slope: vec3<f32>,
    rough: f32,
    metal: f32,
    glow: vec3<f32>,
    reflect: f32,
}

struct Surface {
    p: vec3<f32>,
    n: vec3<f32>,
    footprint: f32,
    arc: f32,
    arc_footprint: f32,
    profile: vec3<f32>,
    part: u32,
    facing: f32,
}

fn hairlines(p: vec3<f32>, n: vec3<f32>, footprint: f32, salt: u32) -> f32 {
    let pitch = 0.004;
    let base = floor(p / pitch);
    var most = 0.0;
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let cell = vec3<i32>(base) + vec3<i32>(x, y, z);
                if chance(cell, salt) < 0.3 {
                    let centre = (vec3<f32>(cell) + spread(cell, salt + 1u)) * pitch;
                    let dir = normalize(flat(spread(cell, salt + 2u) * 2.0 - 1.0, n) + n.yzx * 0.001);
                    let r = p - centre;
                    let along = dot(r, dir);
                    let off = dot(r, cross(n, dir));
                    let reach = pitch * (0.3 + chance(cell, salt + 3u) * 1.5);
                    let width = 0.000012 + chance(cell, salt + 4u) * 0.00003;
                    let blur = max(footprint, width);
                    let line = width / blur * fall(blur * 0.5, blur, abs(off)) * fall(reach * 0.6, reach, abs(along));
                    most = max(most, line * (0.35 + 0.65 * chance(cell, salt + 5u)));
                }
            }
        }
    }
    return most;
}

fn prints(p: vec3<f32>, n: vec3<f32>, footprint: f32) -> vec2<f32> {
    let pitch = 0.06;
    let base = floor(p / pitch - 0.5);
    var mark = vec2<f32>(0.0);
    for (var x = 0; x <= 1; x++) {
        for (var y = 0; y <= 1; y++) {
            for (var z = 0; z <= 1; z++) {
                let cell = vec3<i32>(base) + vec3<i32>(x, y, z);
                if chance(cell, 71u) < 0.22 {
                    let centre = (vec3<f32>(cell) + 0.25 + spread(cell, 72u) * 0.5) * pitch;
                    let dir = normalize(flat(spread(cell, 73u) * 2.0 - 1.0, n) + n.zxy * 0.001);
                    let r = p - centre;
                    let q = vec2<f32>(dot(r, dir), dot(r, cross(n, dir))) / vec2<f32>(0.0085, 0.0065);
                    let d = length(q);
                    let print = fall(0.55, 1.0, d) * smoothstep(0.25, 0.6, noise3(p * 900.0) * 0.5 + noise3(p * 160.0) * 0.5 + 0.2) * fall(0.0, 0.004, abs(dot(r, n)));
                    let whorl = d * 0.0075 + noise3(vec3<f32>(q * 3.0, f32(cell.x))) * 0.0009;
                    let ridges = 0.5 + 0.5 * cos(whorl / 0.00048 * TAU);
                    let shown = seen(1.0 / 0.00048, footprint);
                    mark = max(mark, vec2<f32>(print, print * mix(0.5, ridges, shown)));
                }
            }
        }
    }
    return mark;
}

fn specks(p: vec3<f32>, footprint: f32) -> f32 {
    let pitch = 0.0009;
    let cell = vec3<i32>(floor(p / pitch));
    let centre = (vec3<f32>(cell) + 0.2 + spread(cell, 91u) * 0.6) * pitch;
    let radius = pitch * (0.03 + chance(cell, 92u) * 0.07);
    let blur = max(footprint * 0.7, radius);
    let speck = fall(blur * 0.6, blur, length(p - centre)) * (radius / blur) * (radius / blur);
    return select(0.0, speck, chance(cell, 93u) < 0.07);
}

struct Cut {
    inside: f32,
    slope: vec3<f32>,
}

fn groove(d: f32, across: vec3<f32>, n: vec3<f32>, half: f32, bevel: f32, footprint: f32) -> Cut {
    let a = abs(d);
    let soft = max(footprint, 0.00002);
    let inside = fall(half - soft * 0.5, half + soft * 0.5, a) * min(1.0, half * 2.0 / soft);
    let t = clamp((a - half) / bevel, 0.0, 1.0);
    let shoulder = smoothstep(half - soft * 0.5, half + soft * 0.5, a) * (1.0 - t);
    let slope = flat(across, n) * sign(d) * shoulder * 0.7 * seen(1.0 / bevel, footprint);
    return Cut(inside, slope);
}

struct Seam {
    d: f32,
    across: vec3<f32>,
}

fn nearer(a: Seam, d: f32, across: vec3<f32>, open: bool) -> Seam {
    if open && abs(d) < abs(a.d) {
        return Seam(d, across);
    }
    return a;
}

fn seams(p: vec3<f32>, part: u32) -> Seam {
    var s = Seam(1.0, vec3<f32>(0.0, 1.0, 0.0));
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let side = vec3<f32>(1.0, 0.0, 0.0);
    if part == HEAD {
        s = nearer(s, p.x, side, p.y > HEAD_BASE + 0.142);
        s = nearer(s, p.y - (HEAD_BASE + 0.05), up, p.y < HEAD_BASE + 0.07);
        s = nearer(s, p.y - (HEAD_BASE + 0.19), up, p.z > 0.0);
    } else if part == CHEST {
        s = nearer(s, p.y - 1.128, up, true);
        s = nearer(s, p.y - 0.905, up, true);
        let panel = p.y > 0.905 && p.y < 1.128;
        s = nearer(s, abs(p.x) - 0.088, side * sign(p.x), panel && p.z < 0.0);
        s = nearer(s, p.x, side, panel && p.z > 0.0);
    }
    return s;
}

struct Screw {
    cover: f32,
    slope: vec3<f32>,
    socket: f32,
}

fn screw_at(u: f32, v: f32, du: vec3<f32>, dv: vec3<f32>, n: vec3<f32>, turn: f32, footprint: f32) -> Screw {
    let radius = 0.0028;
    let r = length(vec2<f32>(u, v));
    let soft = max(footprint, 0.00002);
    let cover = fall(radius - soft * 0.5, radius + soft * 0.5, r);
    let dome = clamp(r / radius, 0.0, 0.97);
    let radial = (du * u + dv * v) / max(r, 0.00001);
    let rim = smoothstep(0.75, 0.97, dome);
    let slope = flat(radial, n) * (dome * 0.5 + rim * 1.2) * cover * seen(1.0 / radius, footprint * 0.5);
    let c = cos(turn);
    let s = sin(turn);
    let h = abs(vec2<f32>(u * c - v * s, u * s + v * c));
    let hex = max(h.x * 0.866 + h.y * 0.5, h.y);
    let socket = fall(0.0011 - soft * 0.5, 0.0011 + soft * 0.5, hex) * min(1.0, 0.0022 / soft);
    let ring = smoothstep(radius * 0.92, radius * 1.02, r) * fall(radius * 1.02, radius * 1.25, r) * seen(1.0 / radius, footprint);
    return Screw(cover, slope - flat(radial, n) * ring * 0.8, socket);
}

fn screws(p: vec3<f32>, n: vec3<f32>, part: u32, footprint: f32) -> Screw {
    var found = Screw(0.0, vec3<f32>(0.0), 0.0);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let ringed = normalize(cross(up, n) + vec3<f32>(0.0001, 0.0, 0.0));
    let ang = atan2(p.x, -p.z);
    let r = length(p.xz);
    if part == HEAD && abs(p.y - (HEAD_BASE + 0.05)) < 0.005 {
        let step = TAU / 9.0;
        let k = round((ang - step * 0.5) / step);
        let nearest = k * step + step * 0.5;
        found = screw_at((ang - nearest) * r, p.y - (HEAD_BASE + 0.05), ringed, up, n, k * 1.3, footprint);
    } else if part == HEAD && p.y > HEAD_BASE + 0.15 && abs(p.x) < 0.005 {
        let k = round(p.z / 0.032);
        let along = normalize(flat(vec3<f32>(0.0, 0.0, 1.0), n) + 0.0001);
        found = screw_at(p.x, dot(vec3<f32>(0.0, p.y, p.z - k * 0.032), along) - dot(vec3<f32>(0.0, p.y, 0.0), along), vec3<f32>(1.0, 0.0, 0.0), along, n, k * 2.1, footprint);
    } else if part == CHEST && p.z < 0.0 && abs(abs(p.x) - 0.072) < 0.005 {
        let cy = select(0.919, 1.114, p.y > 1.0);
        found = screw_at(abs(p.x) - 0.072, p.y - cy, vec3<f32>(sign(p.x), 0.0, 0.0), normalize(flat(up, n)), n, p.x * 40.0 + cy * 9.0, footprint);
    } else if part == CHEST && abs(p.y - 1.128) < 0.005 && abs(ang) > 0.9 {
        let step = TAU / 14.0;
        let k = round(ang / step);
        found = screw_at((ang - k * step) * r, p.y - 1.128, ringed, up, n, k * 0.9, footprint);
    }
    return found;
}

fn louvres(p: vec3<f32>, n: vec3<f32>, footprint: f32) -> Cut {
    let pitch = 0.0055;
    let low = 0.922;
    let row = floor((p.y - low) / pitch);
    let d = p.y - (low + (row + 0.5) * pitch);
    let reach = 0.048 - abs(row - 3.0) * 0.004;
    let inside = row >= 0.0 && row < 7.0 && p.z < 0.0;
    let ends = fall(reach - 0.0015, reach, abs(p.x));
    var cut = groove(d, vec3<f32>(0.0, 1.0, 0.0), n, 0.0011, 0.0009, footprint);
    cut.inside *= ends * select(0.0, 1.0, inside);
    cut.slope *= ends * select(0.0, 1.0, inside);
    return cut;
}

fn turned(s: Surface, pitch: f32) -> vec2<f32> {
    let marks = noise3(vec3<f32>(s.arc / pitch, 0.5, 0.5) + s.p * 6.0);
    let fine = noise3(vec3<f32>(s.arc / (pitch * 0.23), 3.5, 1.5) + s.p * 11.0);
    return vec2<f32>(marks * seen(1.0 / pitch, s.arc_footprint), fine * seen(1.0 / (pitch * 0.23), s.arc_footprint));
}

fn chrome(s: Surface) -> Look {
    var look = Look(vec3<f32>(0.94, 0.945, 0.96), vec3<f32>(0.0), 0.04, 1.0, vec3<f32>(0.0), 0.5);
    look.slope += ripple(s.p * 40.0, 1u) * 0.012;
    look.slope += ripple(s.p * 700.0, 2u) * 0.006 * seen(700.0, s.footprint);
    let smear = smoothstep(0.45, 0.85, noise3(s.p * 18.0) * 0.6 + noise3(s.p * 70.0) * 0.4);
    look.rough += smear * 0.05;
    let print = prints(s.p, s.n, s.footprint);
    look.rough = mix(look.rough, 0.42, print.y * 0.85);
    look.color *= 1.0 - print.x * 0.08;
    let lines = hairlines(s.p, s.n, s.footprint, 11u);
    look.rough = mix(look.rough, 0.22, lines);
    look.color *= 1.0 - lines * 0.12;
    let speck = specks(s.p, s.footprint);
    look.color = mix(look.color, vec3<f32>(0.42, 0.4, 0.37), speck);
    look.metal = mix(look.metal, 0.0, speck);
    look.rough = mix(look.rough, 0.9, speck);
    return look;
}

fn satin(s: Surface) -> Look {
    var look = Look(vec3<f32>(0.74, 0.75, 0.77), vec3<f32>(0.0), 0.2, 1.0, vec3<f32>(0.0), 0.5);
    let marks = turned(s, 0.0009);
    look.color *= 0.93 + marks.x * 0.1 + marks.y * 0.05;
    look.rough += (marks.x - 0.5) * 0.08 + (marks.y - 0.5) * 0.06;
    look.slope += s.profile * cos(s.arc / 0.00016 * TAU) * 0.035 * seen(1.0 / 0.00016, s.arc_footprint);
    look.color *= 0.96 + noise3(s.p * 20.0) * 0.08;
    let lines = hairlines(s.p, s.n, s.footprint, 21u);
    look.color *= 1.0 + lines * 0.2;
    return look;
}

fn anodised(s: Surface) -> Look {
    var look = Look(vec3<f32>(0.05, 0.052, 0.06), vec3<f32>(0.0), 0.34, 1.0, vec3<f32>(0.0), 0.5);
    look.slope += ripple(s.p * 6000.0, 3u) * 0.05 * seen(6000.0, s.footprint);
    look.slope += ripple(s.p * 1500.0, 4u) * 0.02 * seen(1500.0, s.footprint);
    look.color *= 0.85 + noise3(s.p * 30.0) * 0.3;
    let lines = hairlines(s.p, s.n, s.footprint, 31u);
    look.color = mix(look.color, vec3<f32>(0.35, 0.36, 0.38), lines * 0.6);
    look.rough = mix(look.rough, 0.2, lines);
    return look;
}

fn rubber(s: Surface) -> Look {
    var look = Look(vec3<f32>(0.022, 0.022, 0.024), vec3<f32>(0.0), 0.62, 0.0, vec3<f32>(0.0), 0.4);
    look.slope += ripple(s.p * 3000.0, 5u) * 0.08 * seen(3000.0, s.footprint);
    look.color *= 0.85 + noise3(s.p * 400.0) * 0.3;
    let speck = specks(s.p, s.footprint) * 1.5;
    look.color = mix(look.color, vec3<f32>(0.3, 0.29, 0.27), clamp(speck, 0.0, 1.0));
    return look;
}

fn brass(s: Surface) -> Look {
    var look = Look(vec3<f32>(0.96, 0.74, 0.4), vec3<f32>(0.0), 0.13, 1.0, vec3<f32>(0.0), 0.5);
    let marks = turned(s, 0.0006);
    look.color *= 0.95 + marks.x * 0.07 + marks.y * 0.04;
    look.rough += (marks.y - 0.5) * 0.06;
    look.slope += s.profile * cos(s.arc / 0.00012 * TAU) * 0.02 * seen(1.0 / 0.00012, s.arc_footprint);
    let tarnish = smoothstep(0.55, 0.85, noise3(s.p * 45.0) * 0.65 + noise3(s.p * 260.0) * 0.35);
    look.color = mix(look.color, vec3<f32>(0.55, 0.38, 0.2), tarnish * 0.45);
    look.rough = mix(look.rough, 0.32, tarnish * 0.6);
    let speck = specks(s.p, s.footprint);
    look.color = mix(look.color, vec3<f32>(0.42, 0.4, 0.37), speck);
    look.metal = mix(look.metal, 0.0, speck);
    return look;
}

fn matrix(u: f32, v: f32, footprint: f32) -> vec2<f32> {
    let pitch = 0.0011;
    let f = fract(vec2<f32>(u, v) / pitch) - 0.5;
    let soft = footprint / pitch;
    let dot_ = fall(0.3 - soft * 0.5, 0.3 + soft * 0.5, length(f));
    let lit = mix(dot_, 0.28, smoothstep(0.3, 0.8, soft));
    return vec2<f32>(lit, floor(u / pitch));
}

fn rounded(u: f32, v: f32, half: vec2<f32>, radius: f32, soft: f32) -> f32 {
    let q = abs(vec2<f32>(u, v)) - half + radius;
    let d = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
    return fall(-soft, soft, d);
}

fn glass(s: Surface) -> Look {
    var look = Look(vec3<f32>(0.006, 0.007, 0.009), vec3<f32>(0.0), 0.03, 0.0, vec3<f32>(0.0), 0.75);
    let lines = hairlines(s.p, s.n, s.footprint, 41u);
    look.rough = mix(look.rough, 0.25, lines * 0.7);
    let speck = specks(s.p, s.footprint);
    look.color = mix(look.color, vec3<f32>(0.4, 0.39, 0.37), speck);
    look.rough = mix(look.rough, 0.9, speck);
    let t = globals.time;
    var glow = vec3<f32>(0.0);
    if s.part == DIAL {
        let q = s.p.xy - DIAL_CENTRE;
        let r = length(q);
        let dots = matrix(q.x, q.y, s.footprint);
        let ang = fract(atan2(q.x, q.y) / TAU + 0.5);
        let segment = floor(ang * 32.0);
        let gap = abs(fract(ang * 32.0) - 0.5);
        let level = 16.0 + sin(t * 0.8) * 9.0 + sin(t * 3.1) * 2.0;
        let band = smoothstep(0.0185, 0.0195, r) * fall(0.029, 0.03, r) * fall(0.38, 0.42, gap);
        let lit = select(0.04, 1.0, segment < level);
        let hue = mix(vec3<f32>(1.0, 0.5, 0.1), vec3<f32>(1.0, 0.12, 0.05), smoothstep(22.0, 28.0, segment));
        let core = fall(0.0035, 0.0045, r) * (0.6 + 0.4 * sin(t * 2.0));
        let ticks = fall(0.2, 0.25, abs(fract(ang * 8.0) - 0.5) * 8.0) * smoothstep(0.0125, 0.013, r) * fall(0.0158, 0.0165, r);
        glow = hue * (band * lit * dots.x * 5.0 + core * 2.0) + vec3<f32>(1.0, 0.6, 0.25) * ticks * 0.5;
        look.color += vec3<f32>(0.02) * dots.x * band;
    } else {
        let u = atan2(s.p.x, -s.p.z) * 0.105;
        let v = s.p.y - EYES;
        let blinking = fract(t / 5.3);
        let blink = 1.0 - smoothstep(0.97, 0.985, blinking) * fall(0.993, 1.0, blinking);
        let soft = max(s.footprint, 0.0002);
        let tall = 0.0072 * max(blink, 0.08);
        let eyes = rounded(abs(u) - 0.037, v + abs(u) * 0.06, vec2<f32>(0.019, tall), 0.003, soft);
        let dots = matrix(u, v, s.footprint);
        let scan = 0.75 + 0.25 * sin(dots.y * 0.9 - t * 6.0);
        let shown = dots.x * scan;
        glow = vec3<f32>(1.0, 0.52, 0.12) * eyes * shown * 5.0 + vec3<f32>(1.0, 0.35, 0.05) * eyes * 0.15;
        look.color += vec3<f32>(0.015, 0.014, 0.013) * dots.x * select(0.0, 1.0, s.p.z < 0.0);
    }
    look.glow = glow * (1.0 - speck);
    return look;
}

fn lamp(s: Surface) -> Look {
    let beat = pow(0.5 + 0.5 * sin(globals.time * 2.4), 4.0);
    let core = pow(s.facing, 3.0);
    let hot = mix(vec3<f32>(1.0, 0.12, 0.04), vec3<f32>(1.0, 0.7, 0.5), core * beat * 0.7);
    let facets = 0.8 + 0.2 * smoothstep(0.3, 0.7, fract(s.arc / 0.0025));
    return Look(vec3<f32>(0.5, 0.06, 0.04), vec3<f32>(0.0), 0.08, 0.0, hot * (0.3 + beat * (1.2 + core * 4.0)) * facets, 0.6);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = vec3<f32>(in.uv.x, in.uv.y, in.uv_b.x);
    let arc = in.uv_b.y;
    let n = normalize(in.color.xyz);
    let dpx = dpdx(p);
    let dpy = dpdy(p);
    let dax = dpdx(arc);
    let day = dpdy(arc);
    let footprint = max(length(dpx), length(dpy));
    let across = dot(dpx, cross(dpy, n));
    let gradient = (dax * cross(dpy, n) + day * cross(n, dpx)) / select(across, 1e-12, abs(across) < 1e-14);
    let profile = normalize(flat(gradient, n) + flat(vec3<f32>(0.0, 1.0, 0.0), n) * 1e-6 + n.yzx * 1e-9);
    let code = u32(round(in.color.w));
    let finish = code & 7u;
    let part = code >> 3u;

    let up = normalize(in.world_normal);
    let along = normalize(in.world_tangent.xyz - up * dot(in.world_tangent.xyz, up));
    let turn = mat3x3<f32>(along, up, cross(along, up));
    let facing = clamp(dot(turn * n, pbr_input.V), 0.0, 1.0);
    let s = Surface(p, n, footprint, arc, max(abs(dax), abs(day)), profile, part, facing);

    var look: Look;
    if finish == CHROME {
        look = chrome(s);
    } else if finish == SATIN {
        look = satin(s);
    } else if finish == ANODISED {
        look = anodised(s);
    } else if finish == RUBBER {
        look = rubber(s);
    } else if finish == BRASS {
        look = brass(s);
    } else if finish == GLASS {
        look = glass(s);
    } else {
        look = lamp(s);
    }

    if finish == CHROME || finish == SATIN {
        let seam = seams(p, part);
        let cut = groove(seam.d, seam.across, n, 0.00028, 0.0007, footprint);
        let chested = select(0.0, 1.0, part == CHEST);
        var vent = louvres(p, n, footprint);
        vent.inside *= chested;
        vent.slope *= chested;
        let inside = max(cut.inside, vent.inside);
        look.slope += cut.slope + vent.slope;
        look.color *= 1.0 - inside * 0.9;
        look.rough = mix(look.rough, 0.6, inside);
        let screw = screws(p, n, part, footprint);
        look.color = mix(look.color, vec3<f32>(0.62, 0.63, 0.66) * (1.0 - screw.socket * 0.9), screw.cover);
        look.rough = mix(look.rough, mix(0.18, 0.55, screw.socket), screw.cover);
        look.slope += screw.slope;
    }

    let normal = turn * tilted(n, look.slope);
    let mirrored = reflect(-pbr_input.V, normal);
    let ground = fall(-0.08, 0.004, mirrored.y);
    let horizon = fall(0.0, 0.05, abs(mirrored.y - 0.03)) * (1.0 - look.rough);
    let shown = mix(vec3<f32>(1.0), vec3<f32>(0.45), ground) * (1.0 + horizon * 0.25);
    look.color *= mix(vec3<f32>(1.0), shown, look.metal);
    look.reflect *= mix(1.0, 0.3, ground * (1.0 - look.metal));
    pbr_input.material.base_color = vec4<f32>(look.color, 1.0);
    pbr_input.N = normal;
    pbr_input.world_normal = turn * n;
    pbr_input.material.perceptual_roughness = clamp(look.rough, 0.035, 1.0);
    pbr_input.material.metallic = look.metal;
    pbr_input.material.reflectance = vec3<f32>(look.reflect);
    pbr_input.material.emissive = vec4<f32>(look.glow, 0.0);
    let ringwise = normalize(turn * cross(n, profile));
    pbr_input.anisotropy_strength = select(0.0, select(0.35, 0.6, finish == SATIN), finish == SATIN || finish == BRASS);
    pbr_input.anisotropy_T = ringwise;
    pbr_input.anisotropy_B = normalize(cross(normal, ringwise));
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    let grey = dot(out.color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    out.color = vec4<f32>(mix(out.color.rgb, grey * look.color / max(max(look.color.r, look.color.g), max(look.color.b, 0.001)) * vec3<f32>(0.92, 0.97, 1.06), ground * look.metal * 0.9), out.color.a);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
