#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{globals, view},
    mesh_functions,
    view_transformations::position_world_to_clip,
}
#import bloxy::meadow::{WIND, SHEEN, gust, lush, blade_colour}

struct Blades {
    dug: vec4<f32>,
    weather: vec4<f32>,
    tread: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> blades: Blades;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var tops: texture_2d<f32>;

const HIDDEN: vec4<f32> = vec4<f32>(2.0, 2.0, 2.0, 1.0);
const SPAN: i32 = 256;
const ROUND: f32 = 0.1875;
const CLUMP: f32 = 0.7;
const TREAD_REACH: f32 = 0.85;
const TREAD_PUSH: f32 = 1.6;
const SHORTEST: f32 = 0.32;
const TALLEST: f32 = 0.72;

fn mixed(seed: u32) -> u32 {
    var state = seed * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn unit(seed: u32) -> f32 {
    return f32(mixed(seed) & 0xffffffu) / 16777216.0;
}

fn spot(block: vec2<i32>) -> u32 {
    return mixed(u32(block.x) * 73856093u ^ u32(block.y) * 19349663u);
}

fn surveyed(block: vec2<i32>) -> vec2<f32> {
    let cell = textureLoad(tops, vec2<i32>(block.x & (SPAN - 1), block.y & (SPAN - 1)), 0);
    return select(vec2<f32>(-1.0, 0.0), cell.ra, all(cell.gb == vec2<f32>(block)));
}

fn top(block: vec2<i32>) -> f32 {
    return surveyed(block).x;
}

fn dip(edge: f32) -> f32 {
    let inward = clamp(ROUND - edge, 0.0, ROUND);
    return ROUND - sqrt(max(ROUND * ROUND - inward * inward, 0.0));
}

fn sunk(block: vec2<i32>, height: f32, inside: vec2<f32>) -> f32 {
    let open = vec4<f32>(
        select(1.0, 0.0, top(block + vec2<i32>(-1, 0)) == height),
        select(1.0, 0.0, top(block + vec2<i32>(1, 0)) == height),
        select(1.0, 0.0, top(block + vec2<i32>(0, -1)) == height),
        select(1.0, 0.0, top(block + vec2<i32>(0, 1)) == height)
    );
    return max(
        max(open.x * dip(inside.x), open.y * dip(1.0 - inside.x)),
        max(open.z * dip(inside.y), open.w * dip(1.0 - inside.y))
    );
}

struct Clump {
    centre: vec2<f32>,
    seed: u32,
}

fn clump(at: vec2<f32>) -> Clump {
    let cell = floor(at / CLUMP);
    var best = Clump(at, 0u);
    var nearest = 9.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let around = cell + vec2<f32>(f32(x), f32(y));
            let seed = spot(vec2<i32>(around) + vec2<i32>(7919, 104729));
            let centre = (around + vec2<f32>(unit(seed), unit(seed + 1u))) * CLUMP;
            let far = distance(centre, at);
            if far < nearest {
                nearest = far;
                best = Clump(centre, seed);
            }
        }
    }
    return best;
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
    let ring = vertex.normal;
    let time = globals.time;
    let seed = spot(block) ^ (slot * 2654435761u);

    let scattered = vec2<f32>(block) + vec2<f32>(0.04, 0.04) + vec2<f32>(unit(seed), unit(seed + 1u)) * 0.92;
    let tuft = clump(scattered);
    let gathered = clamp(mix(scattered, tuft.centre, 0.3), vec2<f32>(block) + 0.03, vec2<f32>(block) + 0.97);
    let cell = surveyed(block);
    let height = cell.x;
    let eye = view.world_position;
    let away = distance(gathered, eye.xz);
    let inner = select(smoothstep(ring.x - 3.0, ring.x + 3.0, away), 1.0, ring.x <= 0.0);
    let present = inner * (1.0 - smoothstep(ring.y - 6.0, ring.y, away));
    let chance = unit(seed + 2u);
    let thinned = clamp((present - chance) / 0.2, 0.0, 1.0);

    let lushness = lush(gathered);
    let tall = mix(SHORTEST, TALLEST, lushness * 0.7 + unit(tuft.seed + 2u) * 0.3)
        * (0.75 + 0.45 * unit(seed + 3u))
        * (1.0 + cell.y * (0.5 + 0.8 * unit(seed + 11u)))
        * thinned;
    let wide = mix(0.055, 0.1, unit(seed + 4u)) * ring.z;
    let root = vec3<f32>(gathered.x, height - sunk(block, height, gathered - vec2<f32>(block)) - 0.04, gathered.y);

    let wind = max(blades.weather.x, 0.3);
    let gusting = gust(root.xz, time);
    let flutter = sin(time * (2.2 + unit(seed + 5u) * 1.6) + unit(seed + 6u) * 40.0 + root.x * 0.9 + root.z * 0.6);
    let lean_turn = unit(tuft.seed + 3u) * 6.2831853;
    let lean = vec3<f32>(cos(lean_turn), 0.0, sin(lean_turn)) * (0.15 + 0.35 * unit(tuft.seed + 4u))
        + vec3<f32>(unit(seed + 7u) - 0.5, 0.0, unit(seed + 8u) - 0.5) * 0.35;
    var push = vec3<f32>(WIND.x, 0.0, WIND.y) * wind * (0.12 + 0.95 * gusting)
        + vec3<f32>(WIND.y, 0.0, -WIND.x) * flutter * 0.07 * wind;
    let off = root.xz - blades.tread.xz;
    let trodden = blades.tread.w
        * (1.0 - smoothstep(0.2, TREAD_REACH, length(off)))
        * (1.0 - smoothstep(0.4, 1.8, abs(root.y - blades.tread.y)));
    push += vec3<f32>(off.x, 0.0, off.y) / max(length(off), 0.05) * trodden * TREAD_PUSH;

    let up = vec3<f32>(0.0, 1.0, 0.0);
    let reach = up + lean + push;
    let bent = length(reach - up);
    let tip = root + normalize(reach) * tall * (1.0 - 0.25 * min(bent, 1.6));
    let control = root + up * tall * 0.6;
    let rest = 1.0 - along;
    let at = rest * rest * root + 2.0 * rest * along * control + along * along * tip;
    let tangent = normalize(2.0 * rest * (control - root) + 2.0 * along * (tip - control) + vec3<f32>(0.0, 1e-4, 0.0));
    let facing_turn = unit(tuft.seed + 5u) * 3.14159 + (unit(seed + 9u) - 0.5) * 1.6;
    let facing = vec3<f32>(cos(facing_turn), 0.0, sin(facing_turn));
    let across = normalize(facing - tangent * dot(facing, tangent) + vec3<f32>(1e-4, 0.0, 0.0));
    let width = wide * pow(rest, 0.6);
    let placed = at + across * side * width * 0.5;

    var flat = normalize(cross(across, tangent));
    if dot(flat, eye - placed) < 0.0 {
        flat = -flat;
    }
    let curved = normalize(flat + across * side * 0.7);
    let normal = normalize(mix(curved, up, 0.55));

    let dug = blades.dug.w > 0.5 && all(vec2<f32>(block) == blades.dug.xz) && height == blades.dug.y + 1.0;
    out.world_position = vec4<f32>(placed, 1.0);
    out.position = select(position_world_to_clip(placed), HIDDEN, height < 0.0 || tall < 0.01 || dug);
    out.world_normal = normal;
    out.uv = vec2<f32>(along, side);
    out.uv_b = vec2<f32>(gusting, lushness);
    out.color = vec4<f32>(1.0, 1.0, 1.0, unit(seed + 10u));
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
    return out;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let along = in.uv.x;
    let gusting = in.uv_b.x;
    let odd = in.color.a;
    let lushness = clamp(in.uv_b.y - select(0.0, 0.5, odd > 0.92), 0.0, 1.0);
    let colour = blade_colour(lushness, along) * (0.88 + 0.24 * odd) + SHEEN * gusting * along;
    let rib = 1.0 + 0.08 * (1.0 - abs(in.uv.y));
    pbr_input.material.base_color = vec4<f32>(colour * rib, 1.0);
    pbr_input.diffuse_occlusion = vec3<f32>(mix(0.3, 1.0, smoothstep(0.0, 0.7, along)));
    pbr_input.material.perceptual_roughness = mix(0.7, 0.35, blades.weather.y);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
