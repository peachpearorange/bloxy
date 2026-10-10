#import bevy_pbr::{forward_io::VertexOutput, mesh_view_bindings::view}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> toward_light: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> light: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> ambient: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> drift: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> haze: vec4<f32>;

const PI: f32 = 3.14159265;
const TURN: mat2x2<f32> = mat2x2<f32>(0.8, -0.6, 0.6, 0.8);

fn scramble(cell: vec2<i32>) -> vec2<u32> {
  var v = bitcast<vec2<u32>>(cell) * 1664525u + 1013904223u;
  v.x += v.y * 1664525u;
  v.y += v.x * 1664525u;
  v = v ^ (v >> vec2<u32>(16u));
  v.x += v.y * 1664525u;
  v.y += v.x * 1664525u;
  return v ^ (v >> vec2<u32>(16u));
}

fn jitter(cell: vec2<i32>) -> vec2<f32> {
  return vec2<f32>(scramble(cell) & vec2<u32>(0xffffu)) / 65535.0;
}

fn gradient(cell: vec2<i32>, offset: vec2<f32>) -> f32 {
  return dot(jitter(cell) * 2.0 - 1.0, offset);
}

fn perlin(at: vec2<f32>) -> f32 {
  let base = floor(at);
  let f = at - base;
  let c = vec2<i32>(base);
  let u = f * f * (3.0 - 2.0 * f);
  return mix(
    mix(gradient(c, f), gradient(c + vec2(1, 0), f - vec2(1.0, 0.0)), u.x),
    mix(gradient(c + vec2(0, 1), f - vec2(0.0, 1.0)), gradient(c + vec2(1, 1), f - vec2(1.0, 1.0)), u.x),
    u.y
  );
}

fn fbm(at: vec2<f32>, octaves: i32) -> f32 {
  var sum = 0.0;
  var amplitude = 0.5;
  var p = at;
  for (var i = 0; i < octaves; i++) {
    sum += amplitude * perlin(p);
    p = TURN * p * 2.03 + vec2(13.7, 5.1);
    amplitude *= 0.5;
  }
  return sum;
}

fn cells(at: vec2<f32>) -> f32 {
  let base = floor(at);
  let c = vec2<i32>(base);
  var nearest = 2.0;
  for (var y = -1; y <= 1; y++) {
    for (var x = -1; x <= 1; x++) {
      let neighbour = c + vec2(x, y);
      let point = vec2<f32>(neighbour) + jitter(neighbour + vec2(71, 29));
      nearest = min(nearest, length(point - at));
    }
  }
  return nearest;
}

const BASE: f32 = 170.0;
const THICK: f32 = 80.0;
const STEPS: i32 = 20;

fn hash3(cell: vec3<i32>) -> f32 {
  let a = scramble(cell.xy + vec2(cell.z * 31, cell.z * 17));
  return f32(a.x & 0xffffu) / 65535.0;
}

fn noise3(at: vec3<f32>) -> f32 {
  let base = floor(at);
  let f = at - base;
  let c = vec3<i32>(base);
  let u = f * f * (3.0 - 2.0 * f);
  let x00 = mix(hash3(c), hash3(c + vec3(1, 0, 0)), u.x);
  let x10 = mix(hash3(c + vec3(0, 1, 0)), hash3(c + vec3(1, 1, 0)), u.x);
  let x01 = mix(hash3(c + vec3(0, 0, 1)), hash3(c + vec3(1, 0, 1)), u.x);
  let x11 = mix(hash3(c + vec3(0, 1, 1)), hash3(c + vec3(1, 1, 1)), u.x);
  return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

fn coverage(at: vec2<f32>) -> f32 {
  let q = at + drift.xy;
  let warp = vec2(fbm(q / 260.0, 2), fbm(q / 260.0 + vec2(17.0, 3.0), 2)) * 80.0;
  let broad = fbm(q / 1100.0 + vec2(4.0, 9.0), 2);
  let big = 1.0 - cells((vec2(q.x * 0.8, q.y) + warp) / 110.0);
  let small = 1.0 - cells((q + warp * 0.5) / 38.0 + vec2(5.3, 1.7));
  let clustered = smoothstep(-0.1, 0.25, fbm(q / 520.0 + vec2(2.0, 11.0), 2));
  let puffs = max(big, small * 0.8 * clustered);
  let fine = fbm((q + warp) / 50.0, 2);
  let shape = puffs * 0.62 + fine * 0.5 + broad * 0.9 + drift.z;
  return clamp((shape - 0.5) * 2.2, 0.0, 1.0);
}

fn density(p: vec3<f32>, detailed: bool) -> f32 {
  let h = (p.y - BASE) / THICK;
  let column = coverage(p.xz);
  let q = p + vec3(drift.x, 0.0, drift.y);
  let towering = 0.3 + 0.7 * clamp(fbm(q.xz / 700.0 + vec2(31.0, 7.0), 2) * 1.6 + 0.5, 0.0, 1.0);
  let dome = sqrt(column);
  var top = dome * towering;
  if (detailed) {
    top += (noise3(q / 24.0) - 0.5) * 0.3 * dome;
  }
  let bottom = 0.06 * (1.0 - column);
  let profile = clamp((h - bottom) * 12.0, 0.0, 1.0) * clamp((top - h) * 6.0, 0.0, 1.0);
  var d = profile * smoothstep(0.0, 0.35, column);
  if (detailed && d > 0.0) {
    let erosion = noise3(q / 9.0) * 0.6 + noise3(q / 3.5) * 0.4;
    d = clamp(d * 1.6 - erosion * (1.0 - d) * 0.9, 0.0, 1.0);
  }
  return d;
}

fn scatter(cosine: f32, g: f32) -> f32 {
  let g2 = g * g;
  return (1.0 - g2) / (4.0 * PI * pow(max(1.0 + g2 - 2.0 * g * cosine, 1e-4), 1.5));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
  let eye = view.world_position;
  let ray = normalize(in.world_position.xyz - eye);
  let exit_far = length(in.world_position.xyz - eye);
  let rise = select(ray.y, 1e-4, abs(ray.y) < 1e-4);
  let t_low = (BASE - eye.y) / rise;
  let t_high = (BASE + THICK - eye.y) / rise;
  let enter = max(min(t_low, t_high), 0.0);
  let leave = min(max(t_low, t_high), min(exit_far, 2600.0));
  if (leave <= enter) {
    discard;
  }
  let span = leave - enter;
  let stride = span / f32(STEPS);
  let offset = fract(sin(dot(in.position.xy, vec2(12.9898, 78.233))) * 43758.547);
  let cosine = dot(ray, toward_light.xyz);
  let phase = 0.08 + scatter(cosine, 0.6) * 0.8 + scatter(cosine, -0.2) * 0.4;
  let gloom = 1.0 - drift.w * 0.75;
  var light_sum = vec3(0.0);
  var transmit = 1.0;
  var depth_sum = 0.0;
  for (var i = 0; i < STEPS; i++) {
    let t = enter + (f32(i) + offset) * stride;
    let p = eye + ray * t;
    let d = density(p, true);
    if (d > 0.01) {
      let toward = normalize(toward_light.xyz + vec3(0.0, 0.15, 0.0));
      let shade = density(p + toward * 8.0, false) + density(p + toward * 22.0, false)
        + density(p + toward * 45.0, false);
      let height = clamp((p.y - BASE) / THICK, 0.0, 1.0);
      let sunlit = light.rgb * exp(-shade * 1.4) * phase * (1.0 - exp(-d * 6.0));
      let skylit = ambient.rgb * (0.35 + 0.65 * height);
      let extinct = d * stride * 0.09;
      let absorbed = 1.0 - exp(-extinct);
      light_sum += transmit * absorbed * (sunlit + skylit) * gloom;
      depth_sum += transmit * absorbed * t;
      transmit *= exp(-extinct);
      if (transmit < 0.03) {
        break;
      }
    }
  }
  let alpha = 1.0 - transmit;
  let seen = depth_sum / max(alpha, 1e-4);
  let far = smoothstep(300.0, 2200.0, seen);
  let colour = mix(light_sum, haze.rgb * alpha, far * 0.8) * (1.0 - smoothstep(1800.0, 2600.0, seen));
  let fade = alpha * (1.0 - smoothstep(1800.0, 2600.0, seen));
  return vec4(clamp(colour * view.exposure, vec3(0.0), vec3(64.0)), fade);
}
