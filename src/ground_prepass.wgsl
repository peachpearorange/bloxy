#import bevy_pbr::{prepass_io, pbr_prepass_functions, mesh_functions, view_transformations::position_world_to_clip}
#import bevy_render::globals::Globals

struct Emerge {
    born: vec4<f32>,
    hidden: vec4<f32>,
    weather: vec4<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> emerge: Emerge;

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
fn vertex(vertex: prepass_io::Vertex) -> prepass_io::VertexOutput {
    var out: prepass_io::VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
#ifdef VERTEX_UVS_B
    world = vec4<f32>(world.xyz + sway(world.xyz, vertex.uv_b, emerge.weather.x, globals.time), 1.0);
    out.uv_b = vertex.uv_b;
#endif
    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex.instance_index);
#endif
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef MOTION_VECTOR_PREPASS
    out.previous_world_position = world;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(vertex.instance_index, world_from_local[3]);
#endif
    return out;
}

fn vanish(at: vec3<f32>) {
    let inside = all(at >= emerge.hidden.xyz - 0.001) && all(at <= emerge.hidden.xyz + 1.001);
    let arriving = (globals.time - emerge.born.x) / EMERGING;
    let leaving = select(0.0, (globals.time - emerge.born.y) / EMERGING, emerge.born.z > 0.5);
    let speck = fract(sin(dot(floor(at * 16.0 + 0.001), vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
    if (emerge.hidden.w > 0.5 && inside) || speck >= arriving || speck < leaving {
        discard;
    }
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(in: prepass_io::VertexOutput, @builtin(front_facing) is_front: bool) -> prepass_io::FragmentOutput {
    pbr_prepass_functions::prepass_alpha_discard(in);
    vanish(in.world_position.xyz);
    var out: prepass_io::FragmentOutput;
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.frag_depth = in.unclipped_depth;
#endif
#ifdef NORMAL_PREPASS
    out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
#endif
#ifdef MOTION_VECTOR_PREPASS
    out.motion_vector = pbr_prepass_functions::calculate_motion_vector(in.world_position, in.previous_world_position);
#endif
    return out;
}
#else
@fragment
fn fragment(in: prepass_io::VertexOutput) {
    pbr_prepass_functions::prepass_alpha_discard(in);
    vanish(in.world_position.xyz);
}
#endif
