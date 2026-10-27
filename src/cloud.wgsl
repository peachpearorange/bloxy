#import bevy_pbr::{forward_io::VertexOutput, mesh_view_bindings::{view, globals}}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> toward_light: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> light: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> ambient: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> drift: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> haze: vec4<f32>;

const PI: f32 = 3.14159265;
const GRAIN: f32 = 16.0;
const FINE: f32 = 6.5;

fn scramble(cell: vec3<i32>) -> u32 {
  var v = bitcast<vec3<u32>>(cell) * 1664525u + 1013904223u;
  v.x += v.y * v.z;
  v.y += v.z * v.x;
  v.z += v.x * v.y;
  v = v ^ (v >> vec3<u32>(16u));
  return v.x + v.y * v.z;
}

fn corner(cell: vec3<i32>) -> f32 {
  return f32(scramble(cell) & 0xffffu) / 65535.0;
}

fn noise3(at: vec3<f32>) -> f32 {
  let base = floor(at);
  let f = at - base;
  let c = vec3<i32>(base);
  let u = f * f * (3.0 - 2.0 * f);
  let x00 = mix(corner(c), corner(c + vec3(1, 0, 0)), u.x);
  let x10 = mix(corner(c + vec3(0, 1, 0)), corner(c + vec3(1, 1, 0)), u.x);
  let x01 = mix(corner(c + vec3(0, 0, 1)), corner(c + vec3(1, 0, 1)), u.x);
  let x11 = mix(corner(c + vec3(0, 1, 1)), corner(c + vec3(1, 1, 1)), u.x);
  return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

fn scatter(cosine: f32, g: f32) -> f32 {
  let g2 = g * g;
  return (1.0 - g2) / (4.0 * PI * pow(max(1.0 + g2 - 2.0 * g * cosine, 1e-4), 1.5));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
  let at = in.world_position.xyz;
  let eye = view.world_position;
  let toward_eye = normalize(eye - at);
  let local = at - vec3(drift.x, 0.0, drift.y) + vec3(0.0, globals.time * 0.4, 0.0);
  let fluff = noise3(local / GRAIN) * 0.65 + noise3(local / FINE) * 0.35;
  let geometric = normalize(in.world_normal);
  let slope = vec3(dpdx(fluff), dpdy(fluff), 0.0);
  let along_x = dpdx(at);
  let along_y = dpdy(at);
  let across = cross(along_y, geometric);
  let up = cross(geometric, along_x);
  let area = dot(along_x, across);
  let bend = sign(area) * (slope.x * across + slope.y * up) / max(abs(area), 1e-6);
  let normal = normalize(geometric - bend * 2.5);
  let sun = toward_light.xyz;
  let wrapped = clamp(dot(normal, sun) * 0.5 + 0.5, 0.0, 1.0);
  let facing = clamp(dot(geometric, toward_eye), 0.0, 1.0);
  let cosine = dot(-toward_eye, sun);
  let lining = scatter(cosine, 0.6) * pow(1.0 - facing, 2.0);
  let below = clamp(-normal.y, 0.0, 1.0);
  let gloom = 1.0 - drift.w * 0.6 - drift.z * 0.25;
  let sunlit = light.rgb * (wrapped * wrapped * 0.22 * (0.85 + 0.3 * fluff) + lining * 0.9) * (1.0 - below * 0.45);
  let skylit = ambient.rgb * (1.05 + 0.25 * normal.y) * (0.9 + 0.1 * fluff);
  let distance = length((eye - at).xz);
  let reach = haze.w;
  var colour = (sunlit + skylit) * gloom;
  colour = mix(colour, haze.rgb, smoothstep(300.0, reach, distance) * 0.8);
  let edge = smoothstep(0.05, 0.5, facing + (fluff - 0.5) * 0.35);
  let alpha = edge * (1.0 - smoothstep(reach * 0.7, reach, distance));
  return vec4(clamp(colour * view.exposure, vec3(0.0), vec3(64.0)) * alpha, alpha);
}
