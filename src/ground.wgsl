#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::globals,
    mesh_functions,
    view_transformations::position_world_to_clip,
}

struct Emerge {
    born: vec4<f32>,
    hidden: vec4<f32>,
    weather: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> emerge: Emerge;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var atlas: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var atlas_sampler: sampler;

const EMERGING: f32 = 1.1;
const WIND: vec3<f32> = vec3<f32>(0.8, 0.0, 0.6);

fn sway(at: vec3<f32>, bend: vec2<f32>, wind: f32, time: f32) -> vec3<f32> {
    let phase = at.x * 0.35 + at.z * 0.27 + at.y * 0.2;
    let gust = 0.55 + 0.45 * sin(time * 0.37 + at.x * 0.021 + at.z * 0.013);
    let swing = sin(time * 1.3 + phase) * 0.6 + sin(time * 2.9 + phase * 1.7) * 0.3 + gust * 0.5;
    let across = vec3<f32>(-WIND.z, 0.0, WIND.x) * sin(time * 1.7 + phase * 1.3) * 0.4;
    let leaves = (WIND * swing + across + vec3<f32>(0.0, sin(time * 2.3 + phase) * 0.3, 0.0)) * 0.07 * bend.x;
    let plants = (WIND * swing + across) * 0.16 * max(bend.y, 0.0) * max(bend.y, 0.0);
    return (leaves + plants) * wind * gust;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
#ifdef VERTEX_UVS_B
    world = vec4<f32>(world.xyz + sway(world.xyz, vertex.uv_b, emerge.weather.x, globals.time), 1.0);
    out.uv_b = vertex.uv_b;
#endif
    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(vertex.instance_index, world_from_local[3]);
#endif
    return out;
}

fn bayer(at: vec2<f32>) -> f32 {
    let p = vec2<u32>(at - floor(at / 4.0) * 4.0);
    let table = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    return (table[p.y * 4u + p.x] + 0.5) / 16.0;
}

fn scatter(cell: vec3<f32>) -> f32 {
    return fract(sin(dot(cell, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

fn threshold(at: vec3<f32>, normal: vec3<f32>) -> f32 {
    let texel = floor(at * 16.0 - normal * 0.01);
    let across = select(select(texel.xy, texel.zy, abs(normal.x) > 0.5), texel.xz, abs(normal.y) > 0.5);
    let block = floor(at - normal * 0.01);
    return bayer(across) * 0.55 + scatter(block) * 0.45;
}

fn painted(origin: vec2<f32>, tile: vec2<f32>, at: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>) -> vec4<f32> {
    let uv = origin + (fract(at) * 0.996 + 0.002) * tile;
    return textureSampleGrad(atlas, atlas_sampler, uv, dx * tile, dy * tile);
}

fn projected(origin: vec2<f32>, at: vec3<f32>, normal: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>) -> vec4<f32> {
    let tile = 16.0 / vec2<f32>(textureDimensions(atlas, 0));
    let sharp = pow(abs(normal), vec3<f32>(8.0));
    let weights = sharp / (sharp.x + sharp.y + sharp.z);
    let east = painted(origin, tile, vec2<f32>(at.z, -at.y), vec2<f32>(dx.z, -dx.y), vec2<f32>(dy.z, -dy.y));
    let up = painted(origin, tile, at.xz, dx.xz, dy.xz);
    let south = painted(origin, tile, vec2<f32>(at.x, -at.y), vec2<f32>(dx.x, -dx.y), vec2<f32>(dy.x, -dy.y));
    return east * weights.x + up * weights.y + south * weights.z;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let at = in.world_position.xyz;
    let dx = dpdx(at);
    let dy = dpdy(at);
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    let normal = normalize(in.world_normal);
#ifdef VERTEX_UVS_B
#ifdef VERTEX_COLORS
    if in.uv_b.y < -0.5 {
        pbr_input.material.base_color = projected(in.uv, at, normal, dx, dy) * in.color;
    }
#endif
#endif
    let cell = floor(at - normal * 0.01);
    if emerge.hidden.w > 0.5 && all(cell == emerge.hidden.xyz) {
        discard;
    }
    let arriving = (globals.time - emerge.born.x) / EMERGING;
    let leaving = select(0.0, (globals.time - emerge.born.y) / EMERGING, emerge.born.z > 0.5);
    let level = threshold(at, normal);
    if level >= arriving || level < leaving {
        discard;
    }
    let soaked = emerge.weather.y * smoothstep(0.3, 0.9, normal.y);
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * (1.0 - 0.3 * soaked), pbr_input.material.base_color.a);
    pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 0.28, soaked);
    pbr_input.material.reflectance = mix(pbr_input.material.reflectance, vec3<f32>(0.5), soaked);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
