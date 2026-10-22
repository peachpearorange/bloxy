#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{globals, view},
    mesh_functions,
    view_transformations::position_world_to_clip,
}
#import bloxy::meadow::{CELLS, PER_CELL, Field, field, blade, blade_colour, handed, shade, spread}

struct Blades {
    dug: vec4<f32>,
    weather: vec4<f32>,
    tread: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> blades: Blades;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var tops: texture_2d<f32>;

const HIDDEN: vec4<f32> = vec4<f32>(2.0, 2.0, 2.0, 1.0);
const SPAN: i32 = 256;

fn surveyed(block: vec2<i32>) -> vec2<f32> {
    let cell = textureLoad(tops, vec2<i32>(block.x & (SPAN - 1), block.y & (SPAN - 1)), 0);
    let dug = blades.dug.w > 0.5 && all(vec2<f32>(block) == blades.dug.xz) && cell.r == blades.dug.y + 1.0;
    return select(vec2<f32>(-1.0, 0.0), cell.ra, all(cell.gb == vec2<f32>(block)) && !dug);
}

fn tallness(at: vec2<f32>) -> f32 {
    let around = spread(at);
    let low = mix(surveyed(around.base).y, surveyed(around.base + vec2<i32>(1, 0)).y, around.blend.x);
    let high = mix(surveyed(around.base + vec2<i32>(0, 1)).y, surveyed(around.base + vec2<i32>(1, 1)).y, around.blend.x);
    return mix(low, high, around.blend.y);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let origin = world_from_local[3].xyz;
    let block = vec2<i32>(round(origin.xz + vertex.position.xz));
    let along = vertex.position.y;
    let slot = u32(vertex.uv.x);
    let side = vertex.uv.y;
    let eye = view.world_position;
    let place = i32(slot / PER_CELL);
    let cell = block * CELLS + vec2<i32>(place % CELLS, place / CELLS);
    let surface = surveyed(block);
    let centre = (vec2<f32>(cell) + 0.5) / f32(CELLS);
    let around = field(centre, globals.time, blades.weather.x, blades.tread);
    let grown = blade(cell, slot % PER_CELL, surface.x, tallness(centre), around);
    let root = grown.root;
    let kept = grown.chance < handed(root.xz, eye.xz);

    let tip = root + grown.axis;
    let height = length(grown.axis);
    let control = root + vec3<f32>(0.0, height * 0.6, 0.0);
    let rest = 1.0 - along;
    let at = rest * rest * root + 2.0 * rest * along * control + along * along * tip;
    let tangent = normalize(2.0 * rest * (control - root) + 2.0 * along * (tip - control) + vec3<f32>(0.0, 1e-4, 0.0));
    let across = normalize(grown.across - tangent * dot(grown.across, tangent) + vec3<f32>(1e-4, 0.0, 0.0));
    let placed = at + across * side * grown.width * pow(rest, 0.6) * 0.5;

    var flat = normalize(cross(across, tangent));
    if dot(flat, eye - placed) < 0.0 {
        flat = -flat;
    }
    let normal = normalize(mix(normalize(flat + across * side * 0.7), vec3<f32>(0.0, 1.0, 0.0), 0.5));

    out.world_position = vec4<f32>(placed, 1.0);
    out.position = select(HIDDEN, position_world_to_clip(placed), surface.x >= 0.0 && kept);
    out.world_normal = normal;
    out.uv = vec2<f32>(along, side);
    out.uv_b = vec2<f32>(around.gust, around.lush);
    out.color = vec4<f32>(1.0, 1.0, 1.0, grown.odd);
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let along = in.uv.x;
    let colour = shade(blade_colour(in.uv_b.y, along), along, in.uv_b.x, in.color.a);
    pbr_input.material.base_color = vec4<f32>(colour, 1.0);
    pbr_input.diffuse_occlusion = vec3<f32>(mix(0.3, 1.0, smoothstep(0.0, 0.7, along)));
    pbr_input.material.perceptual_roughness = mix(0.7, 0.35, blades.weather.y);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
