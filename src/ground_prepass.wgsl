#import bevy_pbr::{prepass_io, pbr_prepass_functions}
#import bevy_render::globals::Globals

struct Emerge {
    born: vec4<f32>,
    hidden: vec4<f32>,
}

@group(0) @binding(1) var<uniform> globals: Globals;
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> emerge: Emerge;

const EMERGING: f32 = 1.1;

fn vanish(at: vec3<f32>) {
    let inside = all(at >= emerge.hidden.xyz - 0.001) && all(at <= emerge.hidden.xyz + 1.001);
    let progress = (globals.time - emerge.born.x) / EMERGING;
    let speck = fract(sin(dot(floor(at * 16.0 + 0.001), vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
    if (emerge.hidden.w > 0.5 && inside) || progress < speck {
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
