#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::globals,
}

const ROBE: u32 = 0u;
const MANTLE: u32 = 1u;
const LINING: u32 = 2u;
const FELT: u32 = 3u;
const BAND: u32 = 4u;
const LEATHER: u32 = 5u;
const GOLD: u32 = 6u;
const WOOD: u32 = 7u;
const GEM: u32 = 8u;
const SKIN: u32 = 9u;
const HAIR: u32 = 10u;
const SLIPPER: u32 = 11u;
const GLASS: u32 = 12u;
const CORK: u32 = 13u;

const BODY: u32 = 0u;
const BUCKLE: u32 = 1u;
const COLLAR: u32 = 2u;
const HEAD: u32 = 3u;
const STAFF: u32 = 4u;
const POUCH: u32 = 5u;
const SHOE: u32 = 6u;
const HAT: u32 = 7u;
const MOTE: u32 = 8u;
const HAND: u32 = 9u;

const TAU: f32 = 6.2831853;
const STAFF_AXIS: vec2<f32> = vec2<f32>(0.42, -0.26);
const GEM_AT: vec3<f32> = vec3<f32>(0.42, 1.93, -0.26);
const HAT_AT: vec3<f32> = vec3<f32>(0.0, 1.625, 0.0);
const HAT_TILT: f32 = 0.1;
const POUCH_AT: vec3<f32> = vec3<f32>(-0.17, 0.0, -0.13);
const POUCH_TURN: f32 = -0.75;
const EYE: vec3<f32> = vec3<f32>(0.041, 1.573, -0.103);
const NOSE_TIP: vec3<f32> = vec3<f32>(0.0, 1.532, -0.172);

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
    let o = f32(salt) * 17.1;
    let here = noise3(p + o);
    return vec3<f32>(
        noise3(p + vec3<f32>(e, 0.0, 0.0) + o) - here,
        noise3(p + vec3<f32>(0.0, e, 0.0) + o) - here,
        noise3(p + vec3<f32>(0.0, 0.0, e) + o) - here
    ) / e;
}

fn fall(low: f32, high: f32, x: f32) -> f32 {
    return 1.0 - smoothstep(low, high, x);
}

fn band(low: f32, high: f32, soft: f32, x: f32) -> f32 {
    return smoothstep(low - soft, low + soft, x) * fall(high - soft, high + soft, x);
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

fn turn2(q: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(q.x * c - q.y * s, q.x * s + q.y * c);
}

fn segment(q: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let ab = b - a;
    let t = clamp(dot(q - a, ab) / max(dot(ab, ab), 1e-8), 0.0, 1.0);
    return length(q - a - ab * t);
}

struct Look {
    color: vec3<f32>,
    slope: vec3<f32>,
    rough: f32,
    metal: f32,
    glow: vec3<f32>,
    reflect: f32,
    sheen: f32,
    strands: f32,
}

fn look(color: vec3<f32>, rough: f32) -> Look {
    return Look(color, vec3<f32>(0.0), rough, 0.0, vec3<f32>(0.0), 0.5, 0.0, 0.0);
}

struct Surface {
    p: vec3<f32>,
    n: vec3<f32>,
    footprint: f32,
    arc: f32,
    profile: vec3<f32>,
    part: u32,
    facing: f32,
    around: vec3<f32>,
    angle: f32,
    radius: f32,
}

struct Spot {
    q: vec2<f32>,
    size: f32,
    roll: f32,
    d: f32,
}

fn sprinkle(s: Surface, columns: f32, row: f32, salt: u32, odds: f32, small: f32, large: f32) -> Spot {
    let a = s.angle / TAU * columns;
    let ci = i32(floor(a));
    let ri = i32(floor(s.p.y / row));
    let count = i32(columns);
    var best = Spot(vec2<f32>(9.0), 1.0, 0.0, 9.0);
    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            let column = ((ci + dx) % count + count) % count;
            let cell = vec3<i32>(column, ri + dy, 7);
            if chance(cell, salt) < odds {
                let j = spread(cell, salt + 1u);
                let ca = f32(ci + dx) + 0.2 + 0.6 * j.x;
                let cy = (f32(ri + dy) + 0.2 + 0.6 * j.y) * row;
                let q = vec2<f32>((a - ca) / columns * TAU * s.radius, s.p.y - cy);
                let size = mix(small, large, j.z * j.z);
                let d = length(q) / size;
                if d < best.d {
                    best = Spot(q, size, chance(cell, salt + 2u), d);
                }
            }
        }
    }
    return best;
}

fn star(q: vec2<f32>, size: f32, points: f32, roll: f32, soft: f32) -> vec2<f32> {
    let t = turn2(q, roll * TAU);
    let a = atan2(t.x, t.y);
    let k = fract(a * points / TAU + 0.5);
    let tip = 1.0 - abs(k - 0.5) * 2.0;
    let reach = size * mix(0.4, 1.0, tip * tip);
    let r = length(t);
    return vec2<f32>(fall(reach - soft, reach + soft, r), r / reach);
}

fn crescent(q: vec2<f32>, size: f32, roll: f32, soft: f32) -> f32 {
    let bite = vec2<f32>(cos(roll * TAU), sin(roll * TAU)) * size * 0.42;
    return fall(size - soft, size + soft, length(q)) * smoothstep(size * 0.86 - soft, size * 0.86 + soft, length(q - bite));
}

fn rune(g: vec2<f32>, seed: vec3<i32>, width: f32, soft: f32) -> f32 {
    let h = scramble(seed, 77u);
    var lattice = array<vec2<f32>, 9>(
        vec2<f32>(0.2, 0.12), vec2<f32>(0.5, 0.12), vec2<f32>(0.8, 0.12),
        vec2<f32>(0.2, 0.5), vec2<f32>(0.5, 0.5), vec2<f32>(0.8, 0.5),
        vec2<f32>(0.2, 0.88), vec2<f32>(0.5, 0.88), vec2<f32>(0.8, 0.88)
    );
    var d = segment(g, lattice[1], lattice[7]);
    for (var i = 0u; i < 3u; i++) {
        let a = lattice[(h >> (i * 8u)) % 9u];
        let b = lattice[(h >> (i * 8u + 4u)) % 9u];
        if any(a != b) {
            d = min(d, segment(g, a, b));
        }
    }
    return fall(width - soft, width + soft, d);
}

struct Stitch {
    cover: f32,
    slope: vec3<f32>,
}

fn embroidered(q: vec2<f32>, cover: f32, rim: f32, s: Surface, thread: f32) -> Stitch {
    let lines = 0.5 + 0.5 * cos(dot(q, vec2<f32>(0.6, 0.8)) / thread * TAU);
    let shown = seen(1.0 / thread, s.footprint);
    let dir = s.around * q.x + vec3<f32>(0.0, q.y, 0.0);
    let raised = flat(normalize(dir + s.n * 1e-6), s.n) * smoothstep(0.6, 1.0, rim) * 1.2 * seen(30.0, s.footprint);
    let across = flat(s.around * 0.6 + vec3<f32>(0.0, 0.8, 0.0), s.n) * sin(dot(q, vec2<f32>(0.6, 0.8)) / thread * TAU) * 0.35 * shown;
    return Stitch(cover * mix(1.0, 0.75 + 0.35 * lines, shown), (raised + across) * cover);
}

fn gilt(l: Look, cover: f32, slope: vec3<f32>, tone: vec3<f32>) -> Look {
    var g = l;
    g.color = mix(g.color, tone, cover);
    g.metal = mix(g.metal, 0.85, cover);
    g.rough = mix(g.rough, 0.32, cover);
    g.sheen = mix(g.sheen, 0.0, cover);
    g.slope += slope;
    return g;
}

const GOLD_THREAD: vec3<f32> = vec3<f32>(1.0, 0.72, 0.28);
const SILVER_THREAD: vec3<f32> = vec3<f32>(0.86, 0.88, 0.95);

fn woven(s: Surface, base: vec3<f32>, pitch: f32) -> Look {
    var l = look(base, 0.82);
    l.sheen = 0.6;
    let u = s.angle * s.radius;
    let twill = 0.5 + 0.5 * cos((u + s.p.y) / pitch * TAU);
    let weft = 0.5 + 0.5 * cos((u - s.p.y * 0.3) / (pitch * 0.5) * TAU);
    let shown = seen(1.0 / pitch, s.footprint);
    l.color *= mix(1.0, 0.82 + 0.3 * twill * (0.7 + 0.3 * weft), shown);
    l.color *= 0.88 + 0.24 * noise3(vec3<f32>(u * 6.0, s.p.y * 1.5, 0.3));
    l.color *= 0.92 + 0.16 * noise3(vec3<f32>(u * 400.0, s.p.y * 25.0, 1.7)) * seen(400.0, s.footprint);
    l.slope += ripple(vec3<f32>(s.p.x * 5.0, s.p.y * 1.2, s.p.z * 5.0), 3u) * 0.35;
    l.slope += ripple(s.p * 40.0, 4u) * 0.06;
    l.slope += flat(s.around, s.n) * sin((u + s.p.y) / pitch * TAU) * 0.08 * shown;
    return l;
}

fn dusty(l: Look, s: Surface) -> Look {
    var d = l;
    let grime = fall(0.0, 0.3, s.p.y) * (0.5 + 0.5 * noise3(s.p * 18.0));
    let splash = smoothstep(0.72, 0.8, noise3(s.p * 60.0)) * fall(0.05, 0.22, s.p.y);
    d.color = mix(d.color, vec3<f32>(0.24, 0.2, 0.15), clamp(grime * 0.55 + splash * 0.6, 0.0, 1.0));
    d.rough = mix(d.rough, 0.95, grime);
    d.sheen *= 1.0 - grime;
    return d;
}

fn robe(s: Surface) -> Look {
    var l = woven(s, vec3<f32>(0.05, 0.06, 0.2), 0.0016);
    let soft = max(s.footprint, 0.0004);

    let field = sprinkle(s, 26.0, 0.15, 11u, 0.75, 0.016, 0.034);
    let shape = star(field.q, field.size, 5.0, field.roll * 0.2, soft);
    let moon = crescent(field.q, field.size * 0.9, field.roll, soft) * select(0.0, 1.0, field.roll > 0.82);
    let gold = select(shape.x, moon, field.roll > 0.82);
    let stitch = embroidered(field.q, gold, shape.y, s, 0.0011);
    l = gilt(l, stitch.cover, stitch.slope, GOLD_THREAD);

    let dust = sprinkle(s, 70.0, 0.05, 23u, 0.5, 0.003, 0.006);
    let twinkle = star(dust.q, dust.size, 4.0, dust.roll, soft);
    l = gilt(l, twinkle.x * (1.0 - gold), vec3<f32>(0.0), SILVER_THREAD);

    let y = s.p.y;
    let u = s.angle * 0.37;
    let hem = band(0.038, 0.128, soft, y);
    let cords = band(0.036, 0.046, soft, y) + band(0.12, 0.13, soft, y);
    let cell = floor(u / 0.05);
    let g = vec2<f32>(fract(u / 0.05), (y - 0.052) / 0.064);
    let inside = band(0.0, 1.0, 0.01, g.y);
    let glyph = select(
        rune(g, vec3<i32>(i32(cell), 3, 1), 0.07, soft / 0.05),
        star((g - 0.5) * vec2<f32>(0.05, 0.064), 0.02, 4.0, 0.0, soft).x,
        i32(cell) % 2 == 0
    ) * inside;
    l.color = mix(l.color, vec3<f32>(0.3, 0.035, 0.06), hem);
    l.sheen = mix(l.sheen, 0.8, hem);
    let trim = clamp(cords + glyph, 0.0, 1.0) * hem;
    l = gilt(l, trim, flat(vec3<f32>(0.0, 1.0, 0.0), s.n) * (band(0.036, 0.041, soft, y) - band(0.041, 0.046, soft, y)) * 0.8, GOLD_THREAD);

    let front = s.p.z < 0.0 && ((y > 0.13 && y < 0.88) || (y > 0.97 && y < 1.34));
    let strip = band(-0.03, 0.03, soft, s.p.x) * select(0.0, 1.0, front);
    let edges = (band(-0.032, -0.024, soft, s.p.x) + band(0.024, 0.032, soft, s.p.x)) * strip;
    let link = abs(fract(y / 0.045) - 0.5) * 0.045;
    let lozenge = fall(0.0145 - soft, 0.0145 + soft, abs(s.p.x) * 1.15 + link) * smoothstep(0.009 - soft, 0.009 + soft, abs(s.p.x) * 1.15 + link);
    let jewel = fall(0.004 - soft, 0.004 + soft, length(vec2<f32>(s.p.x, link)));
    l.color = mix(l.color, vec3<f32>(0.3, 0.035, 0.06), strip);
    l = gilt(l, clamp(edges + lozenge * strip, 0.0, 1.0), vec3<f32>(0.0), GOLD_THREAD);
    l.color = mix(l.color, vec3<f32>(0.1, 0.5, 0.35), jewel * strip);
    l.glow += vec3<f32>(0.05, 0.4, 0.25) * jewel * strip * 0.3;
    l.rough = mix(l.rough, 0.1, jewel * strip);

    let patch_q = turn2(vec2<f32>((s.angle - 2.0) * s.radius, y - 0.45), 0.12);
    let patch_ = fall(0.0, soft, max(abs(patch_q.x) - 0.042, abs(patch_q.y) - 0.036));
    let ring_ = abs(max(abs(patch_q.x) - 0.036, abs(patch_q.y) - 0.03));
    let dashes = select(step(0.45, fract(patch_q.x / 0.006)), step(0.45, fract(patch_q.y / 0.006)), abs(patch_q.x) > 0.034);
    let stitches = fall(0.0008, 0.0008 + soft, ring_) * dashes * patch_;
    l.color = mix(l.color, vec3<f32>(0.19, 0.12, 0.2) * (0.9 + 0.2 * noise3(s.p * 300.0)), patch_ * (1.0 - trim));
    l.color = mix(l.color, vec3<f32>(0.75, 0.68, 0.5), stitches);
    l.sheen *= 1.0 - patch_ * 0.6;

    return dusty(l, s);
}

fn mantle(s: Surface) -> Look {
    var l = woven(s, vec3<f32>(0.33, 0.035, 0.055), 0.0012);
    l.sheen = 1.0;
    let soft = max(s.footprint, 0.0004);
    let field = sprinkle(s, 34.0, 0.07, 41u, 0.55, 0.007, 0.013);
    let shape = star(field.q, field.size, 4.0, 0.0, soft);
    let stitch = embroidered(field.q, shape.x, shape.y, s, 0.0008);
    l = gilt(l, stitch.cover, stitch.slope, SILVER_THREAD);
    let y = s.p.y;
    if s.part == COLLAR {
        let top = band(0.155, 0.2, soft, y - 1.4);
        let rays = 0.5 + 0.5 * cos(s.angle * 40.0);
        l = gilt(l, top, vec3<f32>(0.0), GOLD_THREAD);
        l = gilt(l, fall(0.6, 0.7, rays) * band(0.06, 0.15, soft, y - 1.4) * 0.8, vec3<f32>(0.0), GOLD_THREAD);
    } else {
        let u = s.angle * 0.28;
        let zig = abs(fract(u / 0.016) - 0.5) * 0.016;
        let braid = band(1.084, 1.104, soft, y) * fall(0.0016, 0.0016 + soft, abs(y - 1.0875 - zig));
        let cord = band(1.08, 1.085, soft, y) + band(1.103, 1.11, soft, y);
        l = gilt(l, clamp(braid + cord, 0.0, 1.0), vec3<f32>(0.0), GOLD_THREAD);
        let clasp = fall(0.0, soft, length(vec2<f32>(abs(s.p.x) - 0.13, y - 1.33)) - 0.016) * select(0.0, 1.0, s.p.z < 0.0);
        let boss = fall(0.0, soft, length(vec2<f32>(abs(s.p.x) - 0.13, y - 1.33)) - 0.007);
        l = gilt(l, clasp, vec3<f32>(0.0), vec3<f32>(1.0, 0.78, 0.36));
        l.color = mix(l.color, vec3<f32>(0.5, 0.02, 0.04), boss * clasp);
        l.glow += vec3<f32>(0.6, 0.02, 0.05) * boss * clasp * 0.15;
        l.metal = mix(l.metal, 0.0, boss * clasp);
        l.rough = mix(l.rough, 0.08, boss * clasp);
    }
    return l;
}

fn lining(s: Surface) -> Look {
    var l = look(vec3<f32>(0.3, 0.03, 0.05), 0.3);
    l.color *= 0.85 + 0.25 * noise3(s.p * vec3<f32>(30.0, 8.0, 30.0));
    l.slope += ripple(s.p * vec3<f32>(20.0, 6.0, 20.0), 9u) * 0.2;
    l.strands = 0.5;
    return dusty(l, s);
}

fn hat_local(p: vec3<f32>) -> vec3<f32> {
    let d = p - HAT_AT;
    let c = cos(HAT_TILT);
    let s = sin(HAT_TILT);
    return vec3<f32>(d.x, d.y * c + d.z * s, -d.y * s + d.z * c);
}

fn felt(s: Surface) -> Look {
    var l = look(vec3<f32>(0.075, 0.07, 0.25), 0.88);
    let h = hat_local(s.p);
    let soft = max(s.footprint, 0.0004);
    l.color *= 0.85 + 0.3 * noise3(s.p * 25.0);
    l.color *= 0.9 + 0.2 * noise3(s.p * 2500.0) * seen(2500.0, s.footprint);
    l.slope += ripple(s.p * 2500.0, 12u) * 0.25 * seen(2500.0, s.footprint);
    l.slope += ripple(s.p * vec3<f32>(9.0, 4.0, 9.0), 13u) * 0.5;
    l.sheen = 0.35;
    let r = length(h.xz);
    if r > 0.142 {
        let under = select(0.0, 1.0, s.n.y < -0.2);
        l.color *= 1.0 - under * 0.35;
        let binding = smoothstep(0.302 - soft, 0.302 + soft, r);
        l.color = mix(l.color, vec3<f32>(0.03, 0.02, 0.07), binding);
        l.rough = mix(l.rough, 0.45, binding);
        let dash = step(0.4, fract(atan2(h.x, -h.z) * r / 0.007));
        l.color = mix(l.color, vec3<f32>(0.7, 0.55, 0.3), fall(0.0007, 0.0007 + soft, abs(r - 0.285)) * dash * (1.0 - under));
        let mended = fall(0.0, soft, length(vec2<f32>(h.x - 0.19, h.z + 0.12)) - 0.03) * (1.0 - under);
        let cross_ = fall(0.0006, 0.0006 + soft, min(abs(fract((h.x + h.z) / 0.008) - 0.5), abs(fract((h.x - h.z) / 0.008) - 0.5)) * 0.008);
        l.color = mix(l.color, vec3<f32>(0.18, 0.13, 0.08), mended * 0.8);
        l.color = mix(l.color, vec3<f32>(0.6, 0.5, 0.35), mended * cross_ * select(0.0, 1.0, abs(length(vec2<f32>(h.x - 0.19, h.z + 0.12)) - 0.026) < 0.003));
    } else {
        let local = Surface(h, s.n, s.footprint, s.arc, s.profile, s.part, s.facing, s.around, atan2(h.x, -h.z), max(length(h.xz), 0.02));
        let field = sprinkle(local, 9.0, 0.11, 61u, 0.8, 0.018, 0.03);
        let shape = star(field.q, field.size, 5.0, field.roll * 0.3, soft);
        let front = vec2<f32>(local.angle * local.radius, h.y - 0.2);
        let moon = crescent(front, 0.05, 0.08, soft);
        let free = smoothstep(1.4, 1.6, length(front) / 0.05);
        let gold = max(shape.x * free * select(1.0, 0.0, h.y < 0.09), moon);
        l = gilt(l, gold, vec3<f32>(0.0), GOLD_THREAD);
        l.color *= 1.0 - fall(0.0, 0.02, h.y - 0.076) * 0.3;
    }
    return l;
}

fn ribbon(s: Surface) -> Look {
    var l = look(vec3<f32>(0.08, 0.02, 0.06), 0.3);
    let h = hat_local(s.p);
    let soft = max(s.footprint, 0.0003);
    let a = atan2(h.x, -h.z);
    let k = round(a / TAU * 14.0);
    let q = vec2<f32>((a - k * TAU / 14.0) * 0.137, h.y - 0.053);
    let phase = (k + 7.0) / 14.0 * 2.0 - 1.0;
    let disc = fall(0.0095 - soft, 0.0095 + soft, length(q));
    let lit = disc * smoothstep(0.0095 - soft, 0.0095 + soft, length(q - vec2<f32>(phase * 0.021, 0.0)));
    let rim = disc * fall(0.0008, 0.0008 + soft, abs(length(q) - 0.009));
    l.strands = 0.8;
    l.color *= 0.9 + 0.2 * (0.5 + 0.5 * cos(h.y / 0.0006 * TAU)) * seen(1.0 / 0.0006, s.footprint);
    l = gilt(l, max(lit, rim * 0.7), vec3<f32>(0.0), mix(SILVER_THREAD, GOLD_THREAD, lit));
    let edges = band(0.031, 0.034, soft, h.y) + band(0.072, 0.075, soft, h.y);
    l = gilt(l, edges, vec3<f32>(0.0), GOLD_THREAD);
    return l;
}

fn leather(s: Surface) -> Look {
    var l = look(vec3<f32>(0.2, 0.11, 0.05), 0.55);
    let soft = max(s.footprint, 0.0003);
    let pebble = noise3(s.p * 900.0);
    l.color *= mix(1.0, 0.8 + 0.35 * pebble, seen(900.0, s.footprint));
    l.slope += ripple(s.p * 900.0, 21u) * 0.3 * seen(900.0, s.footprint);
    l.color *= 0.8 + 0.4 * noise3(s.p * 40.0);
    let scuff = smoothstep(0.6, 0.85, noise3(s.p * 120.0));
    l.color = mix(l.color, vec3<f32>(0.35, 0.24, 0.15), scuff * 0.5);
    l.rough = mix(l.rough, 0.85, scuff);
    if s.part == POUCH {
        let d = s.p - POUCH_AT;
        let c = cos(-POUCH_TURN);
        let n = sin(-POUCH_TURN);
        let pp = vec3<f32>(d.x * c + d.z * n, d.y, -d.x * n + d.z * c);
        let dash = step(0.45, fract(atan2(pp.x, -pp.z) * 0.06 / 0.005));
        let seam = fall(0.0008, 0.0008 + soft, abs(pp.y - 0.849)) * dash;
        l.color = mix(l.color, vec3<f32>(0.75, 0.65, 0.45), seam);
        let tooled = star(vec2<f32>(pp.x, pp.y - 0.873), 0.016, 5.0, 0.0, soft);
        let front = select(0.0, 1.0, pp.z < 0.0);
        l.color *= 1.0 - (tooled.x * 0.45 * front);
        l.slope += flat(normalize(vec3<f32>(pp.x, pp.y - 0.873, 0.0) + 1e-6), s.n) * fall(0.8, 1.0, tooled.y) * smoothstep(0.6, 1.0, tooled.y) * front * 1.5;
        let snap = fall(0.0, soft, length(vec2<f32>(pp.x, pp.y - 0.847)) - 0.008) * front;
        l = gilt(l, snap, vec3<f32>(0.0), vec3<f32>(0.9, 0.7, 0.4));
        l.metal = mix(l.metal, 1.0, snap);
        l.rough = mix(l.rough, 0.25, snap);
    } else {
        let u = s.angle * s.radius;
        let dash = step(0.45, fract(u / 0.006));
        let seam = (fall(0.0007, 0.0007 + soft, abs(s.p.y - 0.902)) + fall(0.0007, 0.0007 + soft, abs(s.p.y - 0.958))) * dash;
        l.color = mix(l.color, vec3<f32>(0.78, 0.68, 0.48), seam);
        let edge = fall(0.0, 0.006, min(s.p.y - 0.895, 0.965 - s.p.y));
        l.color *= 1.0 - edge * 0.4;
        let k = round(s.p.x / 0.026);
        let hole = fall(0.0026, 0.0026 + soft, length(vec2<f32>(s.p.x - k * 0.026, s.p.y - 0.93))) * select(0.0, 1.0, s.p.z < 0.0 && s.p.x < -0.04 && s.p.x > -0.15);
        let worn = fall(0.0026, 0.007, length(vec2<f32>(s.p.x - k * 0.026, s.p.y - 0.93))) * select(0.0, 1.0, s.p.z < 0.0 && s.p.x < -0.04 && s.p.x > -0.15);
        l.color = mix(l.color, vec3<f32>(0.32, 0.2, 0.1), worn * 0.6);
        l.color *= 1.0 - hole * 0.95;
    }
    return l;
}

fn gold(s: Surface) -> Look {
    var l = look(vec3<f32>(1.0, 0.75, 0.34), 0.22);
    l.metal = 1.0;
    let soft = max(s.footprint, 0.0002);
    l.slope += ripple(s.p * 260.0, 31u) * 0.12 * seen(260.0, s.footprint);
    let tarnish = smoothstep(0.55, 0.85, noise3(s.p * 90.0));
    l.color = mix(l.color, vec3<f32>(0.45, 0.32, 0.14), tarnish * 0.5);
    l.rough = mix(l.rough, 0.45, tarnish);
    if s.part == BUCKLE {
        let q = vec2<f32>(s.p.x, s.p.y - 0.93);
        let box_ = max(abs(q.x) / 0.025, abs(q.y) / 0.018);
        let opening = fall(1.0 - soft / 0.02, 1.0 + soft / 0.02, box_) * select(0.0, 1.0, s.p.z < -0.18);
        let prong = fall(0.0018, 0.0018 + soft, abs(q.x)) * band(-0.018, 0.006, soft, q.y);
        let strap = opening * (1.0 - prong);
        l.color = mix(l.color, vec3<f32>(0.18, 0.1, 0.045), strap);
        l.metal = mix(l.metal, 0.0, strap);
        l.rough = mix(l.rough, 0.6, strap);
        let beads = fall(0.0014, 0.0014 + soft, length(fract(q / 0.006) * 0.006 - 0.003)) * smoothstep(1.12, 1.18, box_) * fall(1.45, 1.55, box_);
        l.color *= 1.0 - beads * 0.5;
        let garnet = fall(0.0, soft, length(q - vec2<f32>(0.0, 0.026)) - 0.0045);
        l.color = mix(l.color, vec3<f32>(0.5, 0.02, 0.05), garnet);
        l.metal = mix(l.metal, 0.0, garnet);
        l.rough = mix(l.rough, 0.05, garnet);
        l.glow += vec3<f32>(0.5, 0.02, 0.05) * garnet * 0.2;
    } else {
        let spiral = 0.5 + 0.5 * cos((atan2(s.p.x - STAFF_AXIS.x, s.p.z - STAFF_AXIS.y) + s.p.y * 300.0));
        let engraved = fall(0.08, 0.2, spiral) * seen(50.0, s.footprint) * select(0.0, 1.0, s.part == STAFF);
        l.color *= 1.0 - engraved * 0.6;
    }
    return l;
}

fn wood(s: Surface) -> Look {
    var l = look(vec3<f32>(0.3, 0.17, 0.08), 0.6);
    let soft = max(s.footprint, 0.0003);
    let d = s.p.xz - STAFF_AXIS;
    let a = atan2(d.x, d.y);
    let r = length(d);
    let grain = noise3(vec3<f32>(d.x * 600.0, s.p.y * 6.0, d.y * 600.0));
    let figure = noise3(vec3<f32>(d.x * 90.0, s.p.y * 2.0, d.y * 90.0));
    l.color *= 0.7 + 0.4 * grain * seen(600.0, s.footprint) + 0.25 * figure;
    l.slope += flat(normalize(vec3<f32>(d.x, 0.0, d.y) + 1e-6), s.n) * (grain - 0.5) * 0.3 * seen(600.0, s.footprint);
    let knot = fall(0.0, 0.02, length(vec2<f32>(s.p.y - 0.5, (a - 2.2) * r)));
    l.color = mix(l.color, vec3<f32>(0.1, 0.05, 0.02), knot * (0.5 + 0.5 * cos(length(vec2<f32>(s.p.y - 0.5, (a - 2.2) * r)) / 0.003 * TAU)));
    let shaft = s.p.y > 0.08 && s.p.y < 1.66 && s.part == STAFF && r < 0.03;
    let wind = fract(a / TAU + s.p.y * 1.6);
    let inlay = fall(0.0018, 0.0018 + soft, abs(wind - 0.5) * TAU * r) * select(0.0, 1.0, shaft);
    l.color = mix(l.color, vec3<f32>(0.85, 0.87, 0.92), inlay);
    l.metal = mix(l.metal, 1.0, inlay);
    l.rough = mix(l.rough, 0.25, inlay);
    let wrap = select(0.0, 1.0, s.p.y > 0.88 && s.p.y < 1.13 && s.part == STAFF);
    let crossed = min(abs(fract(a / TAU * 3.0 + s.p.y * 22.0) - 0.5), abs(fract(a / TAU * 3.0 - s.p.y * 22.0) - 0.5));
    l.color = mix(l.color, vec3<f32>(0.14, 0.08, 0.04) * (0.8 + 0.4 * smoothstep(0.0, 0.25, crossed)), wrap);
    l.slope += flat(vec3<f32>(0.0, 1.0, 0.0), s.n) * sin(s.p.y * 22.0 * TAU) * 0.3 * wrap;
    l.rough = mix(l.rough, 0.7, wrap);
    let beat = 0.6 + 0.4 * sin(globals.time * 1.7);
    let strip = select(0.0, 1.0, s.p.y > 1.4 && s.p.y < 1.7 && shaft == false && s.part == STAFF && r < 0.03);
    let u = (fract(a / TAU * 3.0) - 0.5);
    let cell = floor((s.p.y - 1.4) / 0.045);
    let g = vec2<f32>(u * 3.0 + 0.5, fract((s.p.y - 1.4) / 0.045));
    let glyph = rune(g, vec3<i32>(i32(cell), i32(floor(a / TAU * 3.0)), 5), 0.08, soft / 0.03) * strip * select(0.0, 1.0, abs(u) < 0.3);
    l.color *= 1.0 - glyph * 0.6;
    l.glow += vec3<f32>(0.2, 0.7, 1.0) * glyph * beat * 0.8;
    return l;
}

fn gem(s: Surface) -> Look {
    var l = look(vec3<f32>(0.02, 0.06, 0.12), 0.04);
    l.reflect = 0.8;
    let t = globals.time;
    let beat = 0.7 + 0.3 * sin(t * 1.7);
    let core = pow(s.facing, 2.0);
    var centre = GEM_AT;
    var hue = vec3<f32>(0.2, 0.65, 1.0);
    if s.part == MOTE {
        centre = vec3<f32>(-0.28, 1.17, -0.35);
        hue = vec3<f32>(1.0, 0.6, 0.18);
    }
    let flicker = 0.75 + 0.25 * sin(t * 9.0 + sin(t * 3.7) * 2.0);
    let q = (s.p - centre) / 0.006 + vec3<f32>(0.0, t * 0.6, 0.0);
    let cell = vec3<i32>(floor(q));
    let c = vec3<f32>(cell) + 0.2 + spread(cell, 51u) * 0.6;
    let sparkle = fall(0.05, 0.3, length(q - c)) * pow(0.5 + 0.5 * sin(t * (2.0 + chance(cell, 52u) * 5.0) + chance(cell, 53u) * TAU), 6.0) * select(0.0, 1.0, chance(cell, 54u) < 0.35);
    let lines = fall(0.0, 0.08, abs(fract(dot(s.p - centre, vec3<f32>(0.3, 1.0, 0.2)) / 0.018) - 0.5) - 0.42);
    l.glow = hue * (0.4 + core * 2.5 * beat) * select(beat, flicker, s.part == MOTE) + vec3<f32>(0.9, 0.95, 1.0) * sparkle * 4.0 + hue * lines * 0.3;
    l.color = hue * 0.15;
    return l;
}

fn skin(s: Surface) -> Look {
    var l = look(vec3<f32>(0.78, 0.53, 0.42), 0.55);
    let soft = max(s.footprint, 0.0002);
    l.color *= 0.92 + 0.12 * noise3(s.p * 70.0);
    l.color *= mix(1.0, 0.9 + 0.12 * noise3(s.p * 1800.0), seen(1800.0, s.footprint));
    l.slope += ripple(s.p * 1800.0, 61u) * 0.08 * seen(1800.0, s.footprint);
    let spots = smoothstep(0.78, 0.85, noise3(s.p * 260.0));
    l.color = mix(l.color, vec3<f32>(0.5, 0.32, 0.22), spots * 0.4);
    let rosy = fall(0.0, 0.05, length(s.p - NOSE_TIP)) + fall(0.0, 0.03, length(vec3<f32>(abs(s.p.x) - 0.07, s.p.y - 1.545, s.p.z + 0.088))) * 0.6;
    l.color = mix(l.color, vec3<f32>(0.85, 0.36, 0.32), clamp(rosy, 0.0, 1.0) * 0.6);
    if s.part == HEAD && s.p.z < -0.06 {
        let e = vec2<f32>(abs(s.p.x) - EYE.x, s.p.y - EYE.y);
        let lid = e.y - 0.0042 + e.x * e.x * 30.0;
        let low = -e.y - 0.0032 + e.x * e.x * 22.0;
        let sclera = fall(0.0, soft, max(lid, low)) * fall(0.0125, 0.0125 + soft, abs(e.x));
        let look_ = e - vec2<f32>(-0.0015, -0.0006);
        let iris = fall(0.0045, 0.0045 + soft, length(look_)) * sclera;
        let pupil = fall(0.002, 0.002 + soft, length(look_)) * sclera;
        let glint = fall(0.0008, 0.0008 + soft, length(look_ - vec2<f32>(0.0013, 0.0015))) * sclera;
        let fibres = 0.75 + 0.25 * cos(atan2(look_.y, look_.x) * 24.0);
        l.color = mix(l.color, vec3<f32>(0.85, 0.82, 0.76), sclera);
        l.color = mix(l.color, vec3<f32>(0.12, 0.38, 0.62) * fibres, iris);
        l.color = mix(l.color, vec3<f32>(0.01), pupil);
        l.glow += vec3<f32>(0.4, 0.6, 1.0) * iris * (1.0 - pupil) * 0.12 + vec3<f32>(1.0) * glint * 0.8;
        l.rough = mix(l.rough, 0.08, sclera);
        let lash = fall(0.0, 0.0012 + soft, abs(lid)) * fall(0.014, 0.016, abs(e.x));
        l.color = mix(l.color, vec3<f32>(0.25, 0.13, 0.1), lash);
        let socket = fall(0.004, 0.012, length(e * vec2<f32>(0.8, 1.4))) * (1.0 - sclera);
        l.color *= 1.0 - socket * 0.3;
        let corner = vec2<f32>(abs(s.p.x) - EYE.x - 0.015, s.p.y - EYE.y);
        let crow = fall(0.0, 0.018, length(corner)) * fall(0.0003, 0.0003 + soft, abs(fract(atan2(corner.y, corner.x) / 0.35) - 0.5) * 0.35 * length(corner)) * select(0.0, 1.0, corner.x > 0.0);
        l.color *= 1.0 - crow * 0.35;
        let brow_lines = fall(0.0004, 0.0004 + soft, abs(fract((s.p.y - 1.597 + sin(s.p.x * 60.0) * 0.001) / 0.0055) - 0.5) * 0.0055) * band(1.598, 1.625, 0.002, s.p.y);
        l.color *= 1.0 - brow_lines * 0.3;
        l.slope += vec3<f32>(0.0, 1.0, 0.0) * brow_lines * 0.3;
    }
    if s.part == HAND {
        let rel = s.p.xz - STAFF_AXIS;
        let gripping = length(rel) < 0.06;
        let k = round((s.p.y - 0.98) / 0.016);
        let knuckle = fall(0.0006, 0.0006 + soft, abs(s.p.y - 0.98 - k * 0.016)) * select(0.0, 1.0, gripping && k >= 0.0 && k <= 3.0 && rel.x > -0.01);
        let palm = vec3<f32>(-0.28, 1.06, -0.35);
        let fingers = select(0.0, 1.0, !gripping && s.p.z < palm.z - 0.012 && s.p.y > palm.y);
        let j = round((s.p.x - palm.x) / 0.0105);
        let gap = fall(0.0006, 0.0006 + soft, abs(s.p.x - palm.x - j * 0.0105)) * fingers;
        l.color *= 1.0 - (knuckle + gap) * 0.45;
        let ring_ = select(0.0, 1.0, !gripping && abs(s.p.x - palm.x - 0.0157) < 0.005 && abs(s.p.z - palm.z + 0.03) < 0.003);
        l = gilt(l, ring_, vec3<f32>(0.0), vec3<f32>(1.0, 0.75, 0.34));
        l.metal = mix(l.metal, 1.0, ring_);
    }
    return l;
}

fn hair(s: Surface) -> Look {
    var l = look(vec3<f32>(0.9, 0.89, 0.86), 0.5);
    let along = dot(s.p, s.profile);
    let across = s.p - s.profile * along;
    let lock = noise3(across * 90.0 + along * 0.8);
    let fibre = noise3(across * 380.0 + along * 2.0);
    let fine = noise3(across * 1500.0 + along * 6.0);
    l.color *= 0.5 + 0.5 * smoothstep(0.2, 0.75, lock);
    l.color *= mix(1.0, 0.6 + 0.55 * fibre, seen(380.0, s.footprint));
    l.color *= mix(1.0, 0.8 + 0.3 * fine, seen(1500.0, s.footprint));
    l.color = mix(l.color, vec3<f32>(0.6, 0.6, 0.62), smoothstep(0.6, 0.8, noise3(across * 160.0 + along * 0.5)) * 0.45);
    l.color = mix(l.color, vec3<f32>(0.78, 0.7, 0.5), fall(1.47, 1.52, s.p.y) * smoothstep(1.42, 1.48, s.p.y) * fall(0.03, 0.06, abs(s.p.x)) * 0.35 * select(0.0, 1.0, s.part == HEAD));
    let side = normalize(cross(s.profile, s.n) + 1e-6);
    l.slope += side * (lock - 0.5) * 1.4 + side * (fibre - 0.5) * 0.6 * seen(380.0, s.footprint);
    l.rough = mix(0.35, 0.6, fibre);
    l.strands = 1.0;
    return l;
}

fn slipper(s: Surface) -> Look {
    let side = sign(s.p.x);
    let x = s.p.x - side * 0.105;
    var l = look(vec3<f32>(0.24, 0.05, 0.22), 0.7);
    l.sheen = 1.0;
    let soft = max(s.footprint, 0.0003);
    l.color *= 0.85 + 0.3 * noise3(s.p * 50.0);
    l.slope += ripple(s.p * 30.0, 81u) * 0.3;
    let piping = fall(0.008, 0.008 + soft, s.p.y);
    l = gilt(l, piping, vec3<f32>(0.0), GOLD_THREAD);
    let top = select(0.0, 1.0, s.n.y > 0.3);
    let emblem = star(vec2<f32>(x, -(s.p.z + 0.28)), 0.018, 5.0, 0.0, soft);
    let stitch = embroidered(vec2<f32>(x, s.p.z + 0.28), emblem.x * top, emblem.y, s, 0.0009);
    l = gilt(l, stitch.cover, stitch.slope * 0.5, GOLD_THREAD);
    let seam = fall(0.0006, 0.0006 + soft, abs(x)) * step(0.45, fract(s.p.z / 0.005)) * select(0.0, 1.0, s.p.z > -0.3 && s.p.z < -0.17);
    l.color = mix(l.color, vec3<f32>(0.85, 0.7, 0.4), seam);
    let curl = select(0.0, 1.0, s.p.z < -0.42);
    let spiral = 0.5 + 0.5 * cos(atan2(s.p.y - 0.06, x) * 2.0 + s.p.z * 400.0);
    l = gilt(l, curl * fall(0.1, 0.25, spiral), vec3<f32>(0.0), GOLD_THREAD);
    return dusty(l, s);
}

fn glass(s: Surface) -> Look {
    var l = look(vec3<f32>(0.01, 0.03, 0.02), 0.04);
    l.reflect = 0.8;
    let t = globals.time;
    let soft = max(s.footprint, 0.0003);
    let level = 0.772 + sin(t * 1.3) * 0.002;
    let liquid = fall(level - soft, level + soft, s.p.y) * select(0.0, 1.0, s.p.y < 0.79);
    let q = (s.p - vec3<f32>(0.2, 0.0, 0.11)) / 0.004 - vec3<f32>(0.0, t * 1.5, 0.0);
    let cell = vec3<i32>(floor(q));
    let c = vec3<f32>(cell) + 0.3 + spread(cell, 91u) * 0.4;
    let bubble = fall(0.12, 0.25, length(q - c)) * select(0.0, 1.0, chance(cell, 92u) < 0.25);
    l.glow = vec3<f32>(0.15, 1.0, 0.35) * liquid * (0.35 + 0.4 * pow(s.facing, 2.0) + bubble * 1.5);
    l.color = mix(l.color, vec3<f32>(0.05, 0.4, 0.12), liquid);
    let d = s.p.xz - vec2<f32>(0.2, 0.11);
    let label = band(0.735, 0.765, soft, s.p.y) * fall(0.5, 0.55, abs(atan2(d.x, d.y) - 2.6));
    let ink = fall(0.0004, 0.0004 + soft, abs(fract((s.p.y - 0.737) / 0.006) - 0.5) * 0.006) * step(0.3, noise3(s.p * 900.0)) * band(0.739, 0.761, 0.0005, s.p.y);
    l.color = mix(l.color, vec3<f32>(0.75, 0.66, 0.46) * (1.0 - ink * 0.8), label);
    l.glow *= 1.0 - label;
    l.rough = mix(l.rough, 0.9, label);
    return l;
}

fn cork(s: Surface) -> Look {
    var l = look(vec3<f32>(0.55, 0.38, 0.22), 0.9);
    let pores = smoothstep(0.65, 0.75, noise3(s.p * 1600.0)) * seen(1600.0, s.footprint);
    l.color *= (0.8 + 0.3 * noise3(s.p * 300.0)) * (1.0 - pores * 0.5);
    return l;
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
    let finish = code & 15u;
    let part = code >> 4u;

    let up = normalize(in.world_normal);
    let along = normalize(in.world_tangent.xyz - up * dot(in.world_tangent.xyz, up));
    let turn = mat3x3<f32>(along, up, cross(along, up));
    let facing = clamp(dot(turn * n, pbr_input.V), 0.0, 1.0);
    let around = normalize(vec3<f32>(-p.z, 0.0, p.x) + vec3<f32>(1e-6, 0.0, 0.0));
    let s = Surface(p, n, footprint, arc, profile, part, facing, around, atan2(p.x, -p.z), max(length(p.xz), 0.02));

    var l: Look;
    if finish == ROBE {
        l = robe(s);
    } else if finish == MANTLE {
        l = mantle(s);
    } else if finish == LINING {
        l = lining(s);
    } else if finish == FELT {
        l = felt(s);
    } else if finish == BAND {
        l = ribbon(s);
    } else if finish == LEATHER {
        l = leather(s);
    } else if finish == GOLD {
        l = gold(s);
    } else if finish == WOOD {
        l = wood(s);
    } else if finish == GEM {
        l = gem(s);
    } else if finish == SKIN {
        l = skin(s);
    } else if finish == HAIR {
        l = hair(s);
    } else if finish == SLIPPER {
        l = slipper(s);
    } else if finish == GLASS {
        l = glass(s);
    } else {
        l = cork(s);
    }

    let sheen = pow(1.0 - facing, 3.0) * l.sheen;
    l.color = l.color * (1.0 - l.sheen * 0.25) + mix(l.color, vec3<f32>(1.0), 0.25) * sheen * 1.4;
    let normal = turn * tilted(n, l.slope);
    let mirrored = reflect(-pbr_input.V, normal);
    let ground = fall(-0.08, 0.004, mirrored.y);
    l.color *= mix(vec3<f32>(1.0), vec3<f32>(0.55), ground * l.metal);
    pbr_input.material.base_color = vec4<f32>(l.color, 1.0);
    pbr_input.N = normal;
    pbr_input.world_normal = turn * n;
    pbr_input.material.perceptual_roughness = clamp(l.rough, 0.04, 1.0);
    pbr_input.material.metallic = l.metal;
    pbr_input.material.reflectance = vec3<f32>(l.reflect);
    pbr_input.material.emissive = vec4<f32>(l.glow, 0.0);
    let strandwise = normalize(turn * profile);
    pbr_input.anisotropy_strength = l.strands * 0.7;
    pbr_input.anisotropy_T = strandwise;
    pbr_input.anisotropy_B = normalize(cross(normal, strandwise));
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
