#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{globals, view},
    mesh_functions,
    view_transformations::position_world_to_clip,
}

struct Blades {
    dug: vec4<f32>,
    weather: vec4<f32>,
    tread: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> blades: Blades;

const HIDDEN: vec4<f32> = vec4<f32>(2.0, 2.0, 2.0, 1.0);
const WIND: vec2<f32> = vec2<f32>(0.8, 0.6);
const ROOT: vec3<f32> = vec3<f32>(0.6, 0.68, 0.5);
const TIP: vec3<f32> = vec3<f32>(1.35, 1.3, 0.92);
const DRY: vec3<f32> = vec3<f32>(1.55, 1.3, 0.62);
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
    let drift = at * 0.05 - WIND * time * 0.45;
    return noise(drift) * 0.65 + noise(drift * 2.9 + vec2<f32>(13.0, 7.0)) * 0.35;
}

fn dug(code: f32, origin: vec3<f32>) -> bool {
    let whole = round(code);
    let index = floor(whole / 2.0);
    let cell = round(origin) + vec3<f32>(index - floor(index / 32.0) * 32.0, floor(index / 1024.0), floor(index / 32.0) - floor(index / 1024.0) * 32.0);
    return whole >= 0.0 && blades.dug.w > 0.5 && all(cell == blades.dug.xyz);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let root = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0)).xyz;
    let ground = normalize(mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index));
    let t = vertex.uv.x;
    let side = vertex.uv.y;
    let seed = vertex.uv_b.y;
    let shape = vertex.tangent;
    let time = globals.time;
    let tall = vertex.color.a * (1.0 - smoothstep(blades.weather.z, blades.weather.w, distance(root, view.world_position)));

    let wind = blades.weather.x;
    let gusting = gust(root.xz, time);
    let flutter = sin(time * (2.1 + seed * 1.9) + seed * 40.0 + root.x * 0.9 + root.z * 0.6);
    var push = vec3<f32>(WIND.x, 0.0, WIND.y) * wind * (0.1 + 0.9 * gusting * gusting)
        + vec3<f32>(WIND.y, 0.0, -WIND.x) * flutter * 0.06 * (0.3 + wind);
    let away = root.xz - blades.tread.xz;
    let trodden = blades.tread.w
        * (1.0 - smoothstep(0.2, TREAD_REACH, length(away)))
        * (1.0 - smoothstep(0.4, 1.8, abs(root.y - blades.tread.y)));
    push += vec3<f32>(away.x, 0.0, away.y) / max(length(away), 0.05) * trodden * TREAD_PUSH;

    let up = normalize(ground * 0.35 + vec3<f32>(0.0, 0.65, 0.0));
    let reach = up + vec3<f32>(shape.x, 0.0, shape.y) + push;
    let bent = length(reach - up);
    let tip = root + normalize(reach) * tall * (1.0 - 0.22 * min(bent, 1.6));
    let control = root + up * tall * 0.55;
    let rest = 1.0 - t;
    let at = rest * rest * root + 2.0 * rest * t * control + t * t * tip;
    let along = normalize(2.0 * rest * (control - root) + 2.0 * t * (tip - control) + vec3<f32>(0.0, 1e-4, 0.0));
    let facing = vec3<f32>(cos(shape.z), 0.0, sin(shape.z));
    let across = normalize(facing - along * dot(facing, along) + vec3<f32>(1e-4, 0.0, 0.0));
    let width = shape.w * pow(rest, 0.7) * (0.35 + 0.65 * smoothstep(0.0, 0.15, tall));
    let placed = at + across * side * width * 0.5;

    var flat = normalize(cross(across, along));
    if dot(flat, view.world_position - placed) < 0.0 {
        flat = -flat;
    }
    let curved = normalize(flat + across * side * 0.6);
    let normal = normalize(mix(curved, ground, 0.3));

    out.world_position = vec4<f32>(placed, 1.0);
    out.position = position_world_to_clip(placed);
    if dug(vertex.uv_b.x, world_from_local[3].xyz) {
        out.position = HIDDEN;
    }
    out.world_normal = normal;
    out.uv = vec2<f32>(t, side);
    out.uv_b = vec2<f32>(seed, 0.0);
    out.color = vec4<f32>(vertex.color.rgb, 1.0);
#ifdef VERTEX_TANGENTS
    out.world_tangent = vec4<f32>(across, 1.0);
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(vertex.instance_index, world_from_local[3]);
#endif
    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let t = in.uv.x;
    let seed = in.uv_b.x;
    let withered = smoothstep(0.6, 1.0, seed);
    let tip = mix(TIP, DRY, withered * 0.85);
    let shade = mix(ROOT, tip, pow(t, 0.75));
    let rib = 1.0 + 0.1 * (1.0 - abs(in.uv.y)) * smoothstep(0.1, 0.5, t);
    pbr_input.material.base_color = vec4<f32>(in.color.rgb * shade * rib, 1.0);
    pbr_input.diffuse_occlusion = vec3<f32>(mix(0.6, 1.0, smoothstep(0.0, 0.5, t)));
    let soaked = blades.weather.y;
    pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 0.3, soaked);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
