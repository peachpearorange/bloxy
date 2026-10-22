#define_import_path bloxy::meadow

const WIND: vec2<f32> = vec2<f32>(0.8, 0.6);
const ROOT_LUSH: vec3<f32> = vec3<f32>(0.004, 0.03, 0.018);
const TIP_LUSH: vec3<f32> = vec3<f32>(0.11, 0.34, 0.025);
const ROOT_DRY: vec3<f32> = vec3<f32>(0.012, 0.04, 0.01);
const TIP_DRY: vec3<f32> = vec3<f32>(0.3, 0.38, 0.04);
const SHEEN: vec3<f32> = vec3<f32>(0.08, 0.12, 0.02);

const CELL: f32 = 0.0625;
const CELLS: i32 = 16;
const PER_CELL: u32 = 2u;
const SHORTEST: f32 = 0.15;
const TALLEST: f32 = 0.35;
const NEAR: f32 = 4.0;
const HANDOVER: f32 = 1.5;
const OUT: f32 = 46.0;
const THINNING: f32 = 12.0;
const TREAD_REACH: f32 = 0.85;
const TREAD_PUSH: f32 = 1.6;

fn scatter(cell: vec2<f32>) -> f32 {
    return fract(sin(dot(cell, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(at: vec2<f32>) -> f32 {
    let cell = floor(at);
    let inside = fract(at);
    let blend = inside * inside * (3.0 - 2.0 * inside);
    let low = mix(scatter(cell), scatter(cell + vec2<f32>(1.0, 0.0)), blend.x);
    let high = mix(scatter(cell + vec2<f32>(0.0, 1.0)), scatter(cell + vec2<f32>(1.0, 1.0)), blend.x);
    return mix(low, high, blend.y);
}

fn gust(at: vec2<f32>, time: f32) -> f32 {
    let drift = at * 0.045 - WIND * time * 0.55;
    let rolling = noise(drift) * 0.7 + noise(drift * 2.7 + vec2<f32>(13.0, 7.0)) * 0.3;
    return smoothstep(0.35, 0.85, rolling);
}

fn lush(at: vec2<f32>) -> f32 {
    let broad = noise(at / 46.0) * 0.65 + noise(at / 13.0 + vec2<f32>(5.0, 9.0)) * 0.35;
    return smoothstep(0.2, 0.6, broad);
}

fn blade_colour(lushness: f32, along: f32) -> vec3<f32> {
    let root = mix(ROOT_DRY, ROOT_LUSH, lushness);
    let tip = mix(TIP_DRY, TIP_LUSH, lushness);
    return mix(root, tip, smoothstep(0.0, 1.0, pow(along, 0.8)));
}

fn lawn(at: vec2<f32>, time: f32, away: f32) -> vec3<f32> {
    let tufts = noise(at * 1.7) * 0.6 + noise(at * 4.3 + vec2<f32>(3.0, 1.0)) * 0.4;
    let under = blade_colour(lush(at), 0.05 + 0.15 * tufts) * 0.8;
    let seen = blade_colour(lush(at), 0.35 + 0.3 * tufts) * 0.9 + SHEEN * gust(at, time) * 0.5;
    return mix(under, seen, smoothstep(OUT - THINNING, OUT, away));
}

fn mixed(seed: u32) -> u32 {
    let state = seed * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn unit(seed: u32) -> f32 {
    return f32(mixed(seed) & 0xffffffu) / 16777216.0;
}

struct Field {
    lean: vec2<f32>,
    lush: f32,
    gust: f32,
}

fn field(at: vec2<f32>, time: f32, wind: f32, tread: vec4<f32>) -> Field {
    let drift = (vec2<f32>(noise(at / 2.3), noise(at / 2.3 + vec2<f32>(19.0, 4.0))) - 0.5) * 0.7;
    let gusting = gust(at, time);
    let away = at - tread.xz;
    let trodden = tread.w * (1.0 - smoothstep(0.2, TREAD_REACH, length(away)));
    let lean = drift
        + WIND * max(wind, 0.3) * (0.1 + 0.8 * gusting)
        + away / max(length(away), 0.05) * trodden * TREAD_PUSH;
    return Field(lean, lush(at), gusting);
}

struct Blade {
    root: vec3<f32>,
    axis: vec3<f32>,
    across: vec3<f32>,
    width: f32,
    chance: f32,
    odd: f32,
}

fn blade(cell: vec2<i32>, slot: u32, ground: f32, tall: f32, around: Field) -> Blade {
    let seed = mixed(u32(cell.x) * 73856093u ^ u32(cell.y) * 19349663u ^ slot * 83492791u);
    let spot = (vec2<f32>(cell) + vec2<f32>(0.3, 0.3) + vec2<f32>(unit(seed), unit(seed + 1u)) * 0.4) * CELL;
    let height = mix(SHORTEST, TALLEST, around.lush * 0.55 + unit(seed + 2u) * 0.45)
        * (1.0 + tall * (0.5 + 0.8 * unit(seed + 3u)));
    let lean = around.lean + (vec2<f32>(unit(seed + 4u), unit(seed + 5u)) - 0.5) * 0.12;
    let axis = normalize(vec3<f32>(lean.x, 1.0, lean.y)) * height * (1.0 - 0.15 * min(length(lean), 1.5));
    let turn = unit(seed + 6u) * 3.14159;
    let facing = vec3<f32>(cos(turn), 0.0, sin(turn));
    let across = normalize(facing - normalize(axis) * dot(facing, normalize(axis)) + vec3<f32>(1e-4, 0.0, 0.0));
    return Blade(
        vec3<f32>(spot.x, ground - 0.03, spot.y),
        axis,
        across,
        mix(0.009, 0.018, unit(seed + 7u)),
        unit(seed + 8u),
        unit(seed + 9u)
    );
}

fn handed(blade_root: vec2<f32>, eye: vec2<f32>) -> f32 {
    return 1.0 - smoothstep(NEAR - HANDOVER, NEAR, distance(blade_root, eye));
}

fn thinned(blade_root: vec2<f32>, eye: vec2<f32>) -> f32 {
    return smoothstep(OUT - THINNING, OUT, distance(blade_root, eye));
}

fn shade(colour: vec3<f32>, along: f32, gusting: f32, odd: f32) -> vec3<f32> {
    return colour * (0.88 + 0.24 * odd) + SHEEN * gusting * along;
}
