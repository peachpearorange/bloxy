#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::globals,
}

struct Emerge {
    born: vec4<f32>,
    hidden: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> emerge: Emerge;

const EMERGING: f32 = 1.1;
const EDGE: f32 = 0.14;

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

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    let at = in.world_position.xyz;
    let normal = normalize(in.world_normal);
    let cell = floor(at - normal * 0.01);
    if emerge.hidden.w > 0.5 && all(cell == emerge.hidden.xyz) {
        discard;
    }
    let progress = (globals.time - emerge.born.x) / EMERGING;
    let edge = progress * (1.0 + EDGE) - threshold(at, normal);
    if edge < 0.0 {
        discard;
    }
    let rim = clamp(1.0 - edge / EDGE, 0.0, 1.0);
    pbr_input.material.emissive = pbr_input.material.emissive + vec4<f32>(0.55, 0.85, 1.0, 0.0) * rim * 5.0;
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
