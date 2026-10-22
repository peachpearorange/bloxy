#define_import_path bloxy::meadow

const WIND: vec2<f32> = vec2<f32>(0.8, 0.6);
const ROOT_LUSH: vec3<f32> = vec3<f32>(0.004, 0.03, 0.018);
const TIP_LUSH: vec3<f32> = vec3<f32>(0.11, 0.34, 0.025);
const ROOT_DRY: vec3<f32> = vec3<f32>(0.012, 0.04, 0.01);
const TIP_DRY: vec3<f32> = vec3<f32>(0.3, 0.38, 0.04);
const SHEEN: vec3<f32> = vec3<f32>(0.08, 0.12, 0.02);

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

fn lawn(at: vec2<f32>, time: f32) -> vec3<f32> {
    let tufts = noise(at * 1.7) * 0.6 + noise(at * 4.3 + vec2<f32>(3.0, 1.0)) * 0.4;
    let seen = blade_colour(lush(at), 0.12 + 0.28 * tufts) * 0.85;
    return seen + SHEEN * gust(at, time) * 0.4;
}
