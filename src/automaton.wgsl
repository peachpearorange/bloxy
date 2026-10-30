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

fn greeble(p: vec3<f32>, n: vec3<f32>, panel: Panel, footprint: f32) -> Look {
    var look = Look(vec3<f32>(1.0), vec3<f32>(0.0), 0.0, 0.0, vec3<f32>(0.0));
    let kind = chance(panel.id, 5u);
    let along = normalize(select(flat(vec3<f32>(1.0, 0.0, 0.0), n), flat(vec3<f32>(0.0, 1.0, 0.0), n), abs(n.y) < 0.75) + vec3<f32>(0.0, 0.0, 1e-4));
    let side = normalize(cross(along, n));
    let local = p - panel.centre;
    let u = dot(local, along);
    let v = dot(local, side);
    let inside = smoothstep(0.006, 0.01, panel.border);
    let soft = max(footprint, 0.0002);
    if kind < 0.22 {
        let pitch = 0.0065;
        let f = fract(u / pitch) - 0.5;
        let slot = fall(0.16, 0.16 + soft / pitch, abs(f)) * fall(0.016, 0.016 + soft, abs(v)) * fall(0.022, 0.022 + soft, abs(u)) * inside;
        let wall = smoothstep(0.16, 0.3, abs(f)) * fall(0.3, 0.42, abs(f)) * fall(0.016, 0.016 + soft, abs(v)) * fall(0.022, 0.022 + soft, abs(u)) * inside;
        look.color = vec3<f32>(1.0 - slot * 0.92 + wall * 0.15);
        look.slope = along * sign(f) * wall * 0.6;
        look.rough = slot * 0.4;
    } else if kind < 0.42 {
        let spacing = 0.021;
        let k = clamp(round(u / spacing), -2.0, 2.0);
        let r = length(vec2<f32>(u - k * spacing, v));
        let radius = 0.0052;
        let hole = fall(radius - soft, radius + soft, r) * inside;
        let rim = smoothstep(radius, radius + 0.0012, r) * fall(radius + 0.0012, radius + 0.0032, r) * inside;
        let toward = (along * (u - k * spacing) + side * v) / max(r, 1e-5);
        look.color = vec3<f32>(1.0 - hole * 0.97 + rim * 0.12);
        look.slope = -toward * rim * 0.7;
        look.rough = hole * 0.5;
    } else if kind < 0.56 {
        let line = abs(abs(v) - 0.007);
        let engraved = fall(0.0005, 0.0005 + soft, line) * inside * fall(0.03, 0.032, abs(u));
        look.color = vec3<f32>(1.0 - engraved * 0.6);
        look.slope = side * sign(v) * sign(abs(v) - 0.007) * engraved * 0.4;
    } else {
        look.color = vec3<f32>(1.0);
    }
    return look;
}

fn paint(p: vec3<f32>, n: vec3<f32>, base: vec3<f32>, footprint: f32) -> Look {
    var look = Look(base, vec3<f32>(0.0), 0.3, 0.08, vec3<f32>(0.0));
    let panel = panels(p, n, vec3<f32>(0.1, 0.075, 0.1), 3u);
    look.color *= 0.93 + chance(panel.id, 4u) * 0.14;
    look.color *= 0.94 + noise3(p * 9.0) * 0.12;
    let peel = wave(p * 700.0, 5u);
    look.slope += peel.yzw * 0.008 * seen(700.0, footprint);
    let flakes = cells(p * 3200.0, 9u);
    let sparkle = fall(0.1, 0.3, flakes.nearest) * seen(3200.0, footprint);
    look.slope += normalize(spread(flakes.id, 13u) - 0.5) * sparkle * 0.12;
    look.color *= 1.0 + sparkle * 0.12;
    let detail = greeble(p, n, panel, footprint);
    look.color *= detail.color;
    look.slope += detail.slope;
    look.rough += detail.rough;
    let cut = seam(panel.border, panel.across, n, 0.0007, 0.0022, 0.0016, footprint);
    look.color *= 1.0 - cut.groove * 0.85 - cut.bevel * 0.12;
    look.slope += cut.slope;
    let along_seam = dot(p, normalize(cross(panel.across, n) + 1e-4));
    let rivet_at = vec2<f32>((fract(along_seam / 0.018) - 0.5) * 0.018, panel.border - 0.0055);
    let rivet = fall(0.0011, 0.0011 + max(footprint, 0.0002), length(rivet_at)) * select(0.0, 1.0, chance(panel.id, 8u) < 0.45);
    look.slope += flat(normalize(rivet_at.x * cross(panel.across, n) - rivet_at.y * panel.across + 1e-6), n) * rivet * 0.8;
    look.color *= 1.0 + rivet * 0.25;
    let edge = 1.0 - smoothstep(0.001, 0.007, panel.border);
    let wear = noise3(p * 160.0) * 0.55 + noise3(p * 700.0) * 0.25 + noise3(p * 30.0) * 0.2
        + edge * 0.16 + clamp(0.25 - p.y, 0.0, 0.25) * 0.5;
    let chipped = smoothstep(0.79, 0.795 + footprint * 25.0, wear) * (1.0 - cut.groove);
    let rimmed = smoothstep(0.77, 0.788, wear) * (1.0 - chipped);
    look.color = mix(look.color, look.color * 0.7, rimmed);
    look.color = mix(look.color, mix(vec3<f32>(0.2, 0.21, 0.22), vec3<f32>(0.6, 0.61, 0.64), smoothstep(0.81, 0.84, wear)), chipped);
    look.metal = mix(look.metal, 0.9, chipped * smoothstep(0.81, 0.84, wear));
    look.rough = mix(look.rough, 0.45, chipped);
    look.color += vec3<f32>(cut.lip * 0.05);
    let scratched = scratches(p, n, footprint, 31u);
    look.color = mix(look.color, look.color * 1.35 + 0.04, scratched * 0.7);
    look.rough = mix(look.rough, 0.5, scratched);
    let grime = clamp(0.3 - p.y, 0.0, 0.3) / 0.3 * (0.4 + noise3(p * 60.0) * 0.6) + cut.groove * 0.3;
    look.color = mix(look.color, vec3<f32>(0.16, 0.13, 0.1), clamp(grime, 0.0, 1.0) * 0.4);
    look.rough = mix(look.rough, 0.8, clamp(grime, 0.0, 1.0) * 0.5);
    return look;
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
    look.color = mix(look.color, look.color * 1.25 + 0.05, scratched * 0.7);
    look.rough = mix(look.rough, 0.2, scratched);
    let tarnish = smoothstep(0.55, 0.85, noise3(p * 30.0) * 0.7 + noise3(p * 140.0) * 0.3);
    look.color = mix(look.color, look.color * vec3<f32>(0.6, 0.6, 0.62), tarnish * 0.5 * (1.0 - polish));
    look.rough = mix(look.rough, 0.6, tarnish * 0.5 * (1.0 - polish));
    let band = fract(p.y * 22.0);
    let rib = fall(0.06, 0.06 + footprint * 22.0, abs(band - 0.5)) * (1.0 - polish);
    look.slope += flat(vec3<f32>(0.0, sign(band - 0.5) * 0.5, 0.0), n) * smoothstep(0.03, 0.06, abs(band - 0.5)) * fall(0.06, 0.09, abs(band - 0.5)) * (1.0 - polish) * seen(22.0, footprint * 0.1);
    look.color *= 1.0 - rib * 0.5;
    let knurl = abs(fract((p.x + p.z) * 900.0 + p.y * 900.0) - 0.5) + abs(fract((p.x + p.z) * 900.0 - p.y * 900.0) - 0.5);
    let knurled = select(0.0, 1.0, band > 0.12 && band < 0.3) * (1.0 - polish) * seen(900.0, footprint);
    look.slope += flat(vec3<f32>(0.0, knurl - 0.5, 0.0), n) * knurled * 0.5;
    look.color *= 1.0 - knurled * knurl * 0.25;
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
    look.color = mix(look.color, look.color * 2.2 + 0.05, worn * 0.5);
    look.rough = mix(look.rough, 0.28, worn);
    let scratched = scratches(p, n, footprint, 59u);
    look.color = mix(look.color, look.color * 2.0 + 0.08, scratched * 0.6);
    let panel = panels(p, n, vec3<f32>(0.04, 0.03, 0.04), 61u);
    let cut = seam(panel.border, panel.across, n, 0.0005, 0.0012, 0.0008, footprint);
    look.color *= 1.0 - cut.groove * 0.7 + cut.lip * 0.6;
    look.slope += cut.slope;
    return look;
}

fn gilt(p: vec3<f32>, n: vec3<f32>, base: vec3<f32>, footprint: f32) -> Look {
    var look = Look(base, vec3<f32>(0.0), 0.2, 1.0, vec3<f32>(0.0));
    let hammered = cells(p * 140.0, 37u);
    look.slope += flat(hammered.offset, n) * 0.35 * seen(140.0, footprint * 0.5);
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
