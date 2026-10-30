#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::globals,
}

const PAINT: u32 = 0u;
const STEEL: u32 = 1u;
const DARK: u32 = 2u;
const GLOW: u32 = 3u;
const GILT: u32 = 4u;
const BLADE: u32 = 5u;
const LENS: u32 = 6u;
const PULSE: u32 = 7u;

const SCRATCH_GRID: f32 = 0.035;

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
        + du * (vec3<f32>(u.y * u.z, u.z * u.x, u.x * u.y) * (-va + vb + vc - vd + ve - vf - vg + vh)
            + vec3<f32>(vb - va, vc - va, ve - va)
            + vec3<f32>(u.y, u.z, u.x) * vec3<f32>(va - vb - vc + vd, va - vc - ve + vg, va - vb - ve + vf)
            + vec3<f32>(u.z, u.x, u.y) * vec3<f32>(va - vb - ve + vf, va - vb - vc + vd, va - vc - ve + vg));
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

fn flat(v: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return v - n * dot(v, n);
}

struct Look {
    color: vec3<f32>,
    slope: vec3<f32>,
    rough: f32,
    metal: f32,
    glow: vec3<f32>,
}

fn scratches(p: vec3<f32>, n: vec3<f32>, footprint: f32, salt: u32) -> f32 {
    var total = 0.0;
    for (var layer = 0u; layer < 3u; layer++) {
        let shifted = p + vec3<f32>(f32(layer) * 0.0117, f32(layer) * 0.0213, f32(layer) * 0.0071);
        let cell = vec3<i32>(floor(shifted / SCRATCH_GRID));
        let pick = spread(cell, salt + layer * 7u);
        let along = normalize(flat(pick * 2.0 - 1.0 + vec3<f32>(0.001, 0.0, 0.0), n));
        let centre = (vec3<f32>(cell) + spread(cell, salt + layer * 7u + 3u)) * SCRATCH_GRID;
        let offset = shifted - centre;
        let reach = SCRATCH_GRID * (0.25 + pick.x * 0.5);
        let bend = (noise3(shifted * 900.0) - 0.5) * 0.0004;
        let away = abs(dot(offset, normalize(cross(along, n))) + bend);
        let width = 0.00012 + pick.y * 0.00018;
        let soft = max(width, footprint * 0.7);
        let line = fall(0.0, soft, away) * width / soft;
        let length_fade = fall(reach * 0.7, reach, abs(dot(offset, along)));
        let present = select(0.0, 1.0, chance(cell, salt + layer * 7u + 5u) < 0.55);
        total = max(total, line * length_fade * present);
    }
    return total;
}

struct Panel {
    border: f32,
    across: vec3<f32>,
    id: vec3<i32>,
    centre: vec3<f32>,
}

fn panels(p: vec3<f32>, n: vec3<f32>, size: vec3<f32>, salt: u32) -> Panel {
    let row = i32(floor(p.y / size.y));
    let stagger = spread(vec3<i32>(row, 0, 0), salt) * size;
    let q = (p + vec3<f32>(stagger.x, 0.0, stagger.z)) / size;
    let cell = vec3<i32>(floor(q));
    let f = fract(q);
    let near = min(f, 1.0 - f) * size;
    let distance = select(vec3<f32>(9.0), near, abs(n) < vec3<f32>(0.72));
    let toward = select(vec3<f32>(-1.0), vec3<f32>(1.0), f > vec3<f32>(0.5));
    var border = distance.x;
    var across = vec3<f32>(toward.x, 0.0, 0.0);
    if distance.y < border {
        border = distance.y;
        across = vec3<f32>(0.0, toward.y, 0.0);
    }
    if distance.z < border {
        border = distance.z;
        across = vec3<f32>(0.0, 0.0, toward.z);
    }
    let centre = (vec3<f32>(cell) + 0.5) * size - vec3<f32>(stagger.x, 0.0, stagger.z);
    return Panel(border, across, cell, centre);
}

struct Seam {
    groove: f32,
    bevel: f32,
    lip: f32,
    slope: vec3<f32>,
}

fn seam(border: f32, across: vec3<f32>, n: vec3<f32>, width: f32, bevel: f32, depth: f32, footprint: f32) -> Seam {
    let soft = max(footprint * 0.8, 0.00015);
    let groove = fall(width - soft, width + soft, border);
    let ramp = smoothstep(width, width + bevel, border) * (1.0 - groove);
    let incline = select(0.0, depth / bevel, border > width && border < width + bevel);
    let lip = smoothstep(width + bevel * 0.7, width + bevel, border) * fall(width + bevel, width + bevel + soft * 2.0 + 0.0006, border);
    let fade = 1.0 - smoothstep(0.4, 1.0, footprint / max(width + bevel, 1e-5));
    return Seam(groove * fade, (1.0 - ramp) * fade, lip * fade, flat(-across, n) * incline * fade);
}

struct Detail {
    shade: f32,
    slope: vec3<f32>,
    rough: f32,
    ink: vec4<f32>,
    glow: vec3<f32>,
}

fn box_mask(u: f32, v: f32, half: vec2<f32>, soft: f32) -> f32 {
    return fall(half.x - soft, half.x + soft, abs(u)) * fall(half.y - soft, half.y + soft, abs(v));
}

fn outline(u: f32, v: f32, half: vec2<f32>, width: f32, soft: f32) -> f32 {
    let edge = max(abs(u) - half.x, abs(v) - half.y);
    return fall(width - soft, width + soft, abs(edge));
}

fn glyphs(u: f32, v: f32, id: vec3<i32>, count: f32, footprint: f32) -> f32 {
    let dot_size = 0.00055;
    let column = floor(u / dot_size);
    let row = floor(v / dot_size);
    let letter = floor(column / 4.0);
    let inside = column >= 0.0 && row >= 0.0 && row < 5.0 && letter < count && (column - letter * 4.0) < 3.0;
    let cell = vec3<i32>(id.x * 31 + i32(letter), id.y * 17 + i32(column - letter * 4.0), id.z * 13 + i32(row));
    let lit = select(0.0, 1.0, inside && chance(cell, 91u) < 0.55);
    return mix(lit, select(0.0, 0.3, inside), smoothstep(dot_size * 0.5, dot_size * 1.5, footprint));
}

fn screw(offset: vec2<f32>, along: vec3<f32>, side: vec3<f32>, footprint: f32, turn: f32) -> Detail {
    var detail = Detail(1.0, vec3<f32>(0.0), 0.0, vec4<f32>(0.0), vec3<f32>(0.0));
    let r = length(offset);
    let radius = 0.0017;
    let soft = max(footprint, 0.00012);
    let head = fall(radius - soft, radius + soft, r);
    let rim = smoothstep(radius - 0.0004, radius, r) * fall(radius, radius + 0.0004 + soft, r);
    let twisted = vec2<f32>(offset.x * cos(turn) - offset.y * sin(turn), offset.x * sin(turn) + offset.y * cos(turn));
    let slot = fall(0.00022, 0.00022 + soft, min(abs(twisted.x), abs(twisted.y))) * fall(radius * 0.7, radius * 0.7 + soft, r);
    let dome = (along * offset.x + side * offset.y) / radius;
    detail.slope = dome * head * 0.5 * (1.0 - slot) - (along * offset.x + side * offset.y) / max(r, 1e-5) * rim * 0.6;
    detail.ink = vec4<f32>(vec3<f32>(0.55, 0.56, 0.58) * (1.0 - slot * 0.85), head * (1.0 - smoothstep(0.0004, 0.0012, footprint) * 0.5));
    detail.rough = head * 0.1;
    return detail;
}

fn greeble(p: vec3<f32>, n: vec3<f32>, panel: Panel, footprint: f32) -> Detail {
    var detail = Detail(1.0, vec3<f32>(0.0), 0.0, vec4<f32>(0.0), vec3<f32>(0.0));
    let kind = chance(panel.id, 5u);
    let along = normalize(select(flat(vec3<f32>(1.0, 0.0, 0.0), n), flat(vec3<f32>(0.0, 1.0, 0.0), n), abs(n.y) < 0.75) + vec3<f32>(0.0, 0.0, 1e-4));
    let side = normalize(cross(along, n));
    let local = p - panel.centre;
    let u = dot(local, along);
    let v = dot(local, side);
    let inside = smoothstep(0.005, 0.008, panel.border);
    let soft = max(footprint, 0.00015);
    if kind < 0.2 {
        let pitch = 0.0045;
        let f = fract(u / pitch) - 0.5;
        let area = box_mask(u, v, vec2<f32>(0.016, 0.009), soft) * inside;
        let slot = fall(0.18, 0.18 + soft / pitch, abs(f)) * area;
        let wall = smoothstep(0.18, 0.3, abs(f)) * fall(0.3, 0.42, abs(f)) * area;
        detail.shade = 1.0 - slot * 0.9 + wall * 0.12;
        detail.slope = along * sign(f) * wall * 0.6 + side * sign(v) * outline(u, v, vec2<f32>(0.017, 0.01), 0.0005, soft) * 0.5;
        detail.rough = slot * 0.4;
    } else if kind < 0.38 {
        let spacing = 0.016;
        let k = clamp(round(u / spacing), -1.0, 1.0);
        let r = length(vec2<f32>(u - k * spacing, v));
        let radius = 0.0042;
        let hole = fall(radius - soft, radius + soft, r) * inside;
        let rim = smoothstep(radius, radius + 0.0008, r) * fall(radius + 0.0008, radius + 0.0022, r) * inside;
        let toward = (along * (u - k * spacing) + side * v) / max(r, 1e-5);
        detail.shade = 1.0 - hole * 0.95 + rim * 0.1;
        detail.slope = -toward * rim * 0.7;
        detail.rough = hole * 0.5;
    } else if kind < 0.62 {
        let half = vec2<f32>(0.017, 0.011);
        let groove = outline(u, v, half, 0.00045, soft) * inside;
        let lid = box_mask(u, v, half, soft) * inside;
        let hinge_at = vec2<f32>(u, v + half.y);
        let knuckle = fall(0.0009, 0.0009 + soft, abs(hinge_at.y)) * select(0.0, 1.0, abs(fract(u / 0.008) - 0.5) < 0.3 && abs(u) < half.x * 0.8) * inside;
        let latch = box_mask(u - half.x + 0.002, v, vec2<f32>(0.0008, 0.0035), soft) * inside;
        detail.shade = 1.0 - groove * 0.8 + lid * 0.04 - latch * 0.5;
        detail.slope = side * sign(v) * groove * sign(max(abs(u) - half.x, abs(v) - half.y)) * 0.5 + side * knuckle * sign(hinge_at.y) * 0.6;
        detail.ink = vec4<f32>(vec3<f32>(0.5, 0.51, 0.53), knuckle);
    } else if kind < 0.74 {
        let line = abs(abs(v) - 0.006);
        let engraved = fall(0.0004, 0.0004 + soft, line) * inside * fall(0.026, 0.028, abs(u));
        detail.shade = 1.0 - engraved * 0.6;
        detail.slope = side * sign(v) * sign(abs(v) - 0.006) * engraved * 0.4;
    }
    let corner = vec2<f32>(0.026, 0.017) * (spread(panel.id, 19u).xy * 0.5 + 0.75);
    let stencil_at = vec2<f32>(u + corner.x, v + corner.y);
    if chance(panel.id, 21u) < 0.45 {
        let letters = 2.0 + floor(chance(panel.id, 22u) * 4.0);
        let ink = glyphs(stencil_at.x, stencil_at.y, panel.id, letters, footprint) * inside;
        let light = chance(panel.id, 23u) < 0.5;
        detail.ink = mix(detail.ink, vec4<f32>(select(vec3<f32>(0.03), vec3<f32>(0.85, 0.84, 0.78), light), 0.85), ink);
        let bar = box_mask(stencil_at.x - 0.009, stencil_at.y - 0.0042, vec2<f32>(0.009, 0.0003), soft) * inside;
        detail.ink = mix(detail.ink, vec4<f32>(select(vec3<f32>(0.03), vec3<f32>(0.85, 0.84, 0.78), light), 0.8), bar);
    }
    if chance(panel.id, 24u) < 0.18 {
        let at = vec2<f32>(u - corner.x + 0.008, v - corner.y + 0.003);
        let plate = box_mask(at.x, at.y, vec2<f32>(0.008, 0.0024), soft) * inside;
        let stripes = select(0.0, 1.0, fract((at.x + at.y) / 0.0024) < 0.5);
        detail.ink = mix(detail.ink, vec4<f32>(mix(vec3<f32>(0.9, 0.66, 0.08), vec3<f32>(0.03), stripes), 0.9), plate);
    }
    if chance(panel.id, 25u) < 0.14 {
        let at = vec2<f32>(u + corner.x - 0.004, v - corner.y + 0.004);
        let r = length(at);
        let pick = chance(panel.id, 26u);
        let hue = select(select(select(vec3<f32>(0.2, 1.0, 0.35), vec3<f32>(1.0, 0.65, 0.1), pick < 0.5), vec3<f32>(1.0, 0.15, 0.1), pick < 0.25), vec3<f32>(0.2, 0.8, 1.0), pick > 0.8);
        let blink = select(1.0, 0.25 + 0.75 * step(0.5, fract(globals.time * (0.4 + pick) + pick * 7.0)), chance(panel.id, 27u) < 0.5);
        let lamp = fall(0.0009 - soft * 0.5, 0.0009 + soft * 0.5, r) * inside;
        let bezel = smoothstep(0.0009, 0.0011, r) * fall(0.0016, 0.0016 + soft, r) * inside;
        detail.ink = mix(detail.ink, vec4<f32>(hue * 0.4, 1.0), lamp);
        detail.ink = mix(detail.ink, vec4<f32>(vec3<f32>(0.08), 1.0), bezel);
        detail.glow += hue * lamp * blink * 3.0;
        detail.slope -= (along * at.x + side * at.y) / max(r, 1e-5) * bezel * 0.5;
    }
    return detail;
}

fn applied(look: Look, raw: Detail, footprint: f32) -> Look {
    let near = 1.0 - smoothstep(0.0005, 0.0018, footprint);
    let detail = Detail(mix(1.0, raw.shade, near), raw.slope * near, raw.rough * near, vec4<f32>(raw.ink.rgb, raw.ink.a * near), raw.glow * near);
    var out = look;
    out.color = mix(out.color * detail.shade, detail.ink.rgb, detail.ink.a);
    out.slope += detail.slope;
    out.rough += detail.rough;
    out.metal = mix(out.metal, 0.6, detail.ink.a * select(0.0, 1.0, detail.ink.r > 0.4 && detail.ink.b > 0.4));
    out.glow += detail.glow;
    return out;
}

fn seam_screws(p: vec3<f32>, n: vec3<f32>, panel: Panel, footprint: f32) -> Detail {
    let tangent = normalize(cross(panel.across, n) + 1e-4);
    let along_seam = dot(p, tangent);
    let spacing = 0.024;
    let offset = vec2<f32>((fract(along_seam / spacing) - 0.5) * spacing, panel.border - 0.0045);
    let index = vec3<i32>(panel.id.x, panel.id.y + i32(floor(along_seam / spacing)), panel.id.z);
    var detail = screw(offset, tangent, -panel.across, footprint, chance(index, 31u) * 3.14);
    let present = select(0.0, 1.0, chance(panel.id, 8u) < 0.55);
    detail.ink.a *= present;
    detail.slope *= present;
    detail.rough *= present;
    return detail;
}

fn paint(p: vec3<f32>, n: vec3<f32>, base: vec3<f32>, footprint: f32) -> Look {
    var look = Look(base, vec3<f32>(0.0), 0.3, 0.08, vec3<f32>(0.0));
    let panel = panels(p, n, vec3<f32>(0.1, 0.075, 0.1), 3u);
    look.color *= 0.94 + chance(panel.id, 4u) * 0.12;
    look.color *= 0.95 + noise3(p * 9.0) * 0.1;
    let peel = wave(p * 700.0, 5u);
    look.slope += peel.yzw * 0.006 * seen(700.0, footprint);
    let flakes = cells(p * 3200.0, 9u);
    let sparkle = fall(0.1, 0.3, flakes.nearest) * seen(3200.0, footprint);
    look.slope += normalize(spread(flakes.id, 13u) - 0.5) * sparkle * 0.1;
    look.color *= 1.0 + sparkle * 0.1;
    look = applied(look, greeble(p, n, panel, footprint), footprint);
    let cut = seam(panel.border, panel.across, n, 0.0006, 0.0018, 0.0014, footprint);
    look.color *= 1.0 - cut.groove * 0.85 - cut.bevel * 0.1;
    look.slope += cut.slope;
    look.color += vec3<f32>(cut.lip * 0.06);
    look = applied(look, seam_screws(p, n, panel, footprint), footprint);
    let edge = 1.0 - smoothstep(0.001, 0.005, panel.border);
    let wear = noise3(p * 160.0) * 0.55 + noise3(p * 700.0) * 0.25 + noise3(p * 30.0) * 0.2
        + edge * 0.12 + clamp(0.2 - p.y, 0.0, 0.2) * 0.4;
    let chipped = smoothstep(0.84, 0.845 + footprint * 25.0, wear) * (1.0 - cut.groove);
    look.color = mix(look.color, mix(vec3<f32>(0.2, 0.21, 0.22), vec3<f32>(0.6, 0.61, 0.64), smoothstep(0.86, 0.88, wear)), chipped);
    look.metal = mix(look.metal, 0.9, chipped * smoothstep(0.86, 0.88, wear));
    look.rough = mix(look.rough, 0.45, chipped);
    let scratched = scratches(p, n, footprint, 31u) * 0.4;
    look.color = mix(look.color, look.color * 1.3 + 0.03, scratched);
    let grime = clamp(0.2 - p.y, 0.0, 0.2) / 0.2 * (0.4 + noise3(p * 60.0) * 0.6) + cut.groove * 0.25;
    look.color = mix(look.color, vec3<f32>(0.16, 0.13, 0.1), clamp(grime, 0.0, 1.0) * 0.3);
    return look;
}

fn plane(p: vec3<f32>, n: vec3<f32>) -> vec2<f32> {
    let a = abs(n);
    if a.x > a.y && a.x > a.z {
        return p.zy;
    } else if a.y > a.z {
        return p.xz;
    }
    return p.xy;
}

fn circuitry(p: vec3<f32>, n: vec3<f32>, footprint: f32) -> Detail {
    var detail = Detail(1.0, vec3<f32>(0.0), 0.0, vec4<f32>(0.0), vec3<f32>(0.0));
    let q = plane(p, n);
    let pitch = 0.0032;
    let cell = floor(q / pitch);
    let f = q / pitch - cell - 0.5;
    let id = vec3<i32>(i32(cell.x), i32(cell.y), 7);
    let soft = max(footprint / pitch, 0.04);
    let across = chance(id, 41u) < 0.45;
    let down = chance(id, 42u) < 0.35;
    let trace = max(select(0.0, fall(0.09, 0.09 + soft, abs(f.y)), across), select(0.0, fall(0.09, 0.09 + soft, abs(f.x)), down));
    let pad = select(0.0, fall(0.22, 0.22 + soft, length(f)), chance(id, 43u) < 0.18);
    let cluster = smoothstep(0.55, 0.62, noise3(p * 18.0 + 3.0));
    let copper = max(trace, pad) * seen(1.0 / pitch, footprint * 0.6) * cluster;
    detail.ink = vec4<f32>(vec3<f32>(0.62, 0.44, 0.16), copper * 0.85);
    detail.glow = vec3<f32>(0.2, 0.7, 1.0) * pad * cluster * select(0.0, 0.6 + 0.4 * sin(globals.time * 3.0 + f32(id.x) * 1.7), chance(id, 44u) < 0.1);
    let part_pitch = 0.012;
    let part_cell = floor(q / part_pitch);
    let pf = q / part_pitch - part_cell - 0.5;
    let pid = vec3<i32>(i32(part_cell.x), i32(part_cell.y), 11);
    let wide = chance(pid, 45u) < 0.5;
    let half = select(vec2<f32>(0.14, 0.3), vec2<f32>(0.3, 0.14), wide);
    let soft_part = max(footprint / part_pitch, 0.02);
    let body = box_mask(pf.x, pf.y, half, soft_part) * select(0.0, 1.0, chance(pid, 46u) < 0.35) * cluster;
    let caps = body * select(smoothstep(half.y * 0.6, half.y * 0.6 + soft_part, abs(pf.y)), smoothstep(half.x * 0.6, half.x * 0.6 + soft_part, abs(pf.x)), wide);
    detail.ink = mix(detail.ink, vec4<f32>(vec3<f32>(0.035, 0.035, 0.04), 1.0), body);
    detail.ink = mix(detail.ink, vec4<f32>(vec3<f32>(0.7, 0.7, 0.72), 1.0), caps);
    return detail;
}

fn steel(p: vec3<f32>, n: vec3<f32>, base: vec3<f32>, footprint: f32, polish: f32) -> Look {
    var look = Look(base, vec3<f32>(0.0), mix(0.34, 0.14, polish), 1.0, vec3<f32>(0.0));
    let brushed = noise3(vec3<f32>(p.x * 2600.0, p.y * 24.0, p.z * 2600.0));
    let fibres = seen(2600.0, footprint);
    look.rough += (brushed - 0.5) * 0.18 * fibres;
    look.color *= 0.92 + brushed * 0.16 * fibres + noise3(p * 14.0) * 0.08;
    let pitch = 260.0;
    let turned = sin(p.y * pitch * 6.2831853);
    look.slope += flat(vec3<f32>(0.0, cos(p.y * pitch * 6.2831853) * 0.05, 0.0), n) * seen(pitch, footprint) * (1.0 - polish);
    look.color *= 1.0 + turned * 0.03 * seen(pitch, footprint);
    let scratched = scratches(p, n, footprint, 47u);
    look.color = mix(look.color, look.color * 1.25 + 0.04, scratched * 0.35);
    look.rough = mix(look.rough, 0.2, scratched);
    let tarnish = smoothstep(0.55, 0.85, noise3(p * 30.0) * 0.7 + noise3(p * 140.0) * 0.3);
    look.color = mix(look.color, look.color * vec3<f32>(0.7, 0.7, 0.72), tarnish * 0.3 * (1.0 - polish));
    look.rough = mix(look.rough, 0.6, tarnish * 0.5 * (1.0 - polish));
    let band = fract(p.y * 22.0);
    let rib = fall(0.06, 0.06 + footprint * 22.0, abs(band - 0.5)) * (1.0 - polish);
    look.slope += flat(vec3<f32>(0.0, sign(band - 0.5) * 0.5, 0.0), n) * smoothstep(0.03, 0.06, abs(band - 0.5)) * fall(0.06, 0.09, abs(band - 0.5)) * (1.0 - polish) * seen(22.0, footprint * 0.1);
    look.color *= 1.0 - rib * 0.5;
    let knurl = abs(fract((p.x + p.z) * 900.0 + p.y * 900.0) - 0.5) + abs(fract((p.x + p.z) * 900.0 - p.y * 900.0) - 0.5);
    let knurled = select(0.0, 1.0, band > 0.12 && band < 0.3) * (1.0 - polish) * seen(900.0, footprint);
    look.slope += flat(vec3<f32>(0.0, knurl - 0.5, 0.0), n) * knurled * 0.5;
    look.color *= 1.0 - knurled * knurl * 0.25;
    let ticks = fract(p.y / 0.004);
    let graduated = select(0.0, 1.0, n.x > 0.55 && polish < 0.5) * smoothstep(0.44, 0.44 + footprint / 0.004 + 0.02, abs(ticks - 0.5)) * seen(250.0, footprint * 0.5);
    look.color *= 1.0 - graduated * 0.6;
    let rivets = cells(p * 60.0, 17u);
    let dome = fall(0.09, 0.11, rivets.nearest) * select(0.0, 1.0, chance(rivets.id, 18u) < 0.12) * seen(60.0, footprint * 0.2);
    look.slope += flat(rivets.offset, n) * dome * 8.0;
    look.color *= 1.0 + dome * 0.15;
    return look;
}

fn dark(p: vec3<f32>, n: vec3<f32>, base: vec3<f32>, footprint: f32) -> Look {
    var look = Look(base, vec3<f32>(0.0), 0.5, 0.55, vec3<f32>(0.0));
    let blasted = wave(p * 1800.0, 23u);
    look.slope += blasted.yzw * 0.025 * seen(1800.0, footprint);
    let grain = cells(p * 4200.0, 29u);
    look.color *= 0.9 + fall(0.0, 0.5, grain.nearest) * 0.2 * seen(4200.0, footprint);
    look.color *= 0.85 + noise3(p * 25.0) * 0.3;
    let worn = smoothstep(0.62, 0.8, noise3(p * 70.0) * 0.7 + noise3(p * 300.0) * 0.3);
    look.color = mix(look.color, look.color * 1.8 + 0.03, worn * 0.25);
    look.rough = mix(look.rough, 0.28, worn);
    let scratched = scratches(p, n, footprint, 59u);
    look.color = mix(look.color, look.color * 2.0 + 0.06, scratched * 0.3);
    let panel = panels(p, n, vec3<f32>(0.04, 0.03, 0.04), 61u);
    let cut = seam(panel.border, panel.across, n, 0.0005, 0.0012, 0.0008, footprint);
    look.color *= 1.0 - cut.groove * 0.7 + cut.lip * 0.6;
    look.slope += cut.slope;
    look = applied(look, circuitry(p, n, footprint), footprint);
    look = applied(look, seam_screws(p, n, panel, footprint), footprint);
    return look;
}

fn gilt(p: vec3<f32>, n: vec3<f32>, base: vec3<f32>, footprint: f32) -> Look {
    var look = Look(base, vec3<f32>(0.0), 0.2, 1.0, vec3<f32>(0.0));
    let hammered = cells(p * 420.0, 37u);
    look.slope += flat(hammered.offset, n) * 0.12 * seen(420.0, footprint * 0.5);
    let patina = smoothstep(0.5, 0.9, noise3(p * 40.0) * 0.6 + noise3(p * 220.0) * 0.4);
    look.color = mix(look.color, look.color * 0.45, patina * 0.6);
    look.rough = mix(look.rough, 0.5, patina * 0.6);
    let scratched = scratches(p, n, footprint, 83u);
    look.color = mix(look.color, look.color * 1.3, scratched);
    let fine = wave(p * 2400.0, 41u);
    look.slope += fine.yzw * 0.01 * seen(2400.0, footprint);
    return look;
}

fn honeycomb(p: vec3<f32>, footprint: f32) -> f32 {
    let comb = cells(p * 420.0, 43u);
    return mix(1.0, 0.35 + smoothstep(0.0, 0.12, comb.gap) * 0.65, seen(420.0, footprint * 0.5));
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = vec3<f32>(in.uv.x, in.uv.y, in.uv_b.x);
    let footprint = length(fwidth(p));
    let local_normal = normalize(in.color.xyz);
    let code = u32(round(in.color.w));
    let finish = code & 7u;
    let phase = f32(code >> 3u) * 0.37;
    let packed = u32(round(in.uv_b.y));
    let tint = pow(vec3<f32>(f32((packed >> 16u) & 255u), f32((packed >> 8u) & 255u), f32(packed & 255u)) / 255.0, vec3<f32>(2.2));

    let up = normalize(in.world_normal);
    let along = normalize(in.world_tangent.xyz - up * dot(in.world_tangent.xyz, up));
    let turn = mat3x3<f32>(along, up, cross(along, up));
    let facing = clamp(dot(turn * local_normal, pbr_input.V), 0.0, 1.0);

    var look: Look;
    if finish == PAINT {
        look = paint(p, local_normal, tint, footprint);
    } else if finish == STEEL {
        look = steel(p, local_normal, tint, footprint, 0.0);
    } else if finish == BLADE {
        look = steel(p, local_normal, tint, footprint, 1.0);
    } else if finish == DARK {
        look = dark(p, local_normal, tint, footprint);
    } else if finish == GILT {
        look = gilt(p, local_normal, tint, footprint);
    } else if finish == LENS {
        let gleam = noise3(p * 300.0);
        look = Look(tint * 0.06, vec3<f32>(0.0), 0.05, 0.0, tint * (0.35 + gleam * 0.1) * honeycomb(p, footprint));
    } else {
        let beat = select(1.0, 0.72 + 0.28 * sin(globals.time * 2.6 + phase), finish == PULSE);
        let core = pow(facing, 6.0);
        let rings = 0.75 + 0.25 * sin(facing * 38.0);
        let hot = mix(tint, vec3<f32>(1.0, 0.95, 0.85), core * 0.6);
        look = Look(tint * 0.15, vec3<f32>(0.0), 0.04, 0.0, hot * (0.35 + facing * 0.9 + core * 1.6) * rings * beat * honeycomb(p, footprint));
    }
    let normal = turn * tilted(local_normal, look.slope);
    pbr_input.material.base_color = vec4<f32>(look.color, 1.0);
    pbr_input.N = normal;
    pbr_input.world_normal = turn * local_normal;
    pbr_input.material.perceptual_roughness = clamp(look.rough, 0.04, 1.0);
    pbr_input.material.metallic = look.metal;
    pbr_input.material.emissive = vec4<f32>(look.glow, 0.0);
    let brushed = normalize(turn * vec3<f32>(0.0, 1.0, 0.0) - normal * dot(turn * vec3<f32>(0.0, 1.0, 0.0), normal) + along * 0.0001);
    pbr_input.anisotropy_strength = select(0.0, 0.45, finish == STEEL);
    pbr_input.anisotropy_T = brushed;
    pbr_input.anisotropy_B = normalize(cross(normal, brushed));
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
