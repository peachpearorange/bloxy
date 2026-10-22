#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{globals, view},
    mesh_functions,
    view_transformations::position_world_to_clip,
}
#import bloxy::meadow::lawn

struct Surface {
    dug: vec4<f32>,
    aimed: vec4<f32>,
    weather: vec4<f32>,
    turf: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> surface: Surface;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var atlas: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var atlas_sampler: sampler;

const HIDDEN: vec4<f32> = vec4<f32>(2.0, 2.0, 2.0, 1.0);
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

fn ownership(code: f32, origin: vec3<f32>) -> vec2<f32> {
    let whole = round(code);
    if whole < 0.0 {
        return vec2<f32>(0.0, 0.0);
    }
    let leafy = whole - floor(whole / 2.0) * 2.0;
    let index = floor(whole / 2.0);
    let cell = round(origin) + vec3<f32>(index - floor(index / 32.0) * 32.0, floor(index / 1024.0), floor(index / 32.0) - floor(index / 1024.0) * 32.0);
    var mark = 0.0;
    if surface.dug.w > 0.5 && all(cell == surface.dug.xyz) {
        mark = 1.0;
    } else if surface.aimed.w > 0.5 && all(cell == surface.aimed.xyz) {
        mark = 2.0;
    }
    return vec2<f32>(mark, leafy);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
#ifdef VERTEX_UVS_B
    let owned = ownership(vertex.uv_b.x, world_from_local[3].xyz);
    world = vec4<f32>(world.xyz + sway(world.xyz, vec2<f32>(owned.y, vertex.uv_b.y), surface.weather.x, globals.time), 1.0);
    out.uv_b = vec2<f32>(owned.x, vertex.uv_b.y);
#endif
    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);
#ifdef VERTEX_UVS_B
    if owned.x > 0.5 && owned.x < 1.5 {
        out.position = HIDDEN;
    }
#endif
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

fn scatter(cell: vec3<f32>) -> f32 {
    return fract(sin(dot(cell, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

fn lumpy(at: vec3<f32>) -> f32 {
    let cell = floor(at);
    let inside = fract(at);
    let blend = inside * inside * (3.0 - 2.0 * inside);
    let low = mix(mix(scatter(cell), scatter(cell + vec3<f32>(1.0, 0.0, 0.0)), blend.x),
                  mix(scatter(cell + vec3<f32>(0.0, 1.0, 0.0)), scatter(cell + vec3<f32>(1.0, 1.0, 0.0)), blend.x), blend.y);
    let high = mix(mix(scatter(cell + vec3<f32>(0.0, 0.0, 1.0)), scatter(cell + vec3<f32>(1.0, 0.0, 1.0)), blend.x),
                   mix(scatter(cell + vec3<f32>(0.0, 1.0, 1.0)), scatter(cell + vec3<f32>(1.0, 1.0, 1.0)), blend.x), blend.y);
    return mix(low, high, blend.z);
}

fn painted(origin: vec2<f32>, tile: vec2<f32>, at: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>) -> vec4<f32> {
    let uv = origin + (fract(at) * 0.996 + 0.002) * tile;
    return textureSampleGrad(atlas, atlas_sampler, uv, dx * tile, dy * tile);
}

fn origin_of(index: f32, tile: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(index - floor(index / 8.0) * 8.0, floor(index / 8.0)) * tile;
}

fn projected(top: vec2<f32>, code: f32, at: vec3<f32>, normal: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>) -> vec4<f32> {
    let tile = 16.0 / vec2<f32>(textureDimensions(atlas, 0));
    let packed = round(-code - 1.0);
    let flank = origin_of(floor(packed / 256.0), tile);
    let bottom = origin_of(packed - floor(packed / 256.0) * 256.0, tile);
    let sharp = pow(abs(normal), vec3<f32>(8.0));
    let weights = sharp / (sharp.x + sharp.y + sharp.z);
    let east = painted(flank, tile, vec2<f32>(at.z, -at.y), vec2<f32>(dx.z, -dx.y), vec2<f32>(dy.z, -dy.y));
    let south = painted(flank, tile, vec2<f32>(at.x, -at.y), vec2<f32>(dx.x, -dx.y), vec2<f32>(dy.x, -dy.y));
    let same = all(abs(top - flank) < tile * 0.5);
    let level = select(bottom, top, normal.y > 0.0 && same);
    let up = painted(level, tile, at.xz, dx.xz, dy.xz);
    let mixed = east * weights.x + up * weights.y + south * weights.z;
    if same {
        return mixed;
    }
    let ragged = (lumpy(at * 2.3) - 0.5) * 0.3 + (lumpy(at * 7.0) - 0.5) * 0.12;
    let covered = smoothstep(0.34, 0.5, normal.y + ragged);
    var cover = painted(top, tile, at.xz, dx.xz, dy.xz);
    if all(abs(top - surface.turf.xy) < tile * 0.5) {
        cover = vec4<f32>(lawn(at.xz, globals.time, distance(at.xz, view.world_position.xz)), 1.0);
    }
    return mix(mixed, cover, covered);
}

fn glow(at: vec3<f32>, time: f32) -> f32 {
    let pulse = 0.5 + 0.5 * sin(time * 4.5);
    let sweep = fract(dot(at, vec3<f32>(0.42, 0.75, 0.31)) * 0.9 - time * 0.8);
    let band = smoothstep(0.0, 0.06, sweep) * (1.0 - smoothstep(0.06, 0.32, sweep));
    return 0.25 + 0.35 * pulse + band;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let at = in.world_position.xyz;
    let dx = dpdx(at);
    let dy = dpdy(at);
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    let normal = normalize(in.world_normal);
    var mark = 0.0;
#ifdef VERTEX_UVS_B
    mark = in.uv_b.x;
#ifdef VERTEX_COLORS
    if in.uv_b.y < -0.5 {
        pbr_input.material.base_color = projected(in.uv, in.uv_b.y, at, normal, dx, dy) * in.color;
    }
#endif
#endif
    let soaked = surface.weather.y * smoothstep(0.3, 0.9, normal.y);
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * (1.0 - 0.3 * soaked), pbr_input.material.base_color.a);
    pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 0.28, soaked);
    pbr_input.material.reflectance = mix(pbr_input.material.reflectance, vec3<f32>(0.5), soaked);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    if mark > 1.5 {
        let shine = glow(at, globals.time);
        let base = pbr_input.material.base_color.rgb;
        out.color = vec4<f32>(out.color.rgb * (1.0 + 1.2 * shine) + (base * 0.5 + vec3<f32>(0.1, 0.12, 0.14)) * shine, out.color.a);
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
