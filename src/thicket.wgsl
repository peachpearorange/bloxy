#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{Vertex, VertexOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{globals, view},
    mesh_functions,
    view_transformations::position_world_to_clip,
}
#import bloxy::meadow::{CELL, PER_CELL, BAND, OUT, spread, Field, Blade, field, blade, blade_colour, handed, thinned, shade}

struct Blades {
    dug: vec4<f32>,
    weather: vec4<f32>,
    tread: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> blades: Blades;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var tops: texture_2d<f32>;

const HIDDEN: vec4<f32> = vec4<f32>(2.0, 2.0, 2.0, 1.0);
const SPAN: i32 = 256;
const STEPS: i32 = 72;
const FAR: f32 = 1e6;

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


const SIDES: array<vec2<i32>, 5> = array<vec2<i32>, 5>(
    vec2<i32>(0, 0), vec2<i32>(-1, 0), vec2<i32>(1, 0), vec2<i32>(0, -1), vec2<i32>(0, 1)
);

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    let origin = world_from_local[3].xyz;
    let block = vec2<i32>(round(origin.xz + vertex.uv.xy));
    let face = i32(round(vertex.uv_b.x));
    let surface = surveyed(block);
    let ground = surface.x;
    let corner = vertex.position;
    let world = vec3<f32>(
        f32(block.x) + corner.x,
        select(ground - 0.3, ground + BAND, corner.y > 0.5),
        f32(block.y) + corner.z
    );
    let beside = surveyed(block + SIDES[face]);
    let top = ground + BAND;
    let covered = beside.x == ground || beside.x >= top;
    let inner = face > 0 && covered;
    let away = distance(vec2<f32>(block) + 0.5, view.world_position.xz);
    out.world_position = vec4<f32>(world, 1.0);
    out.position = select(position_world_to_clip(world), HIDDEN, ground < 0.0 || inner || away > OUT + 2.0);
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);
    out.uv = vec2<f32>(0.0, 0.0);
    out.uv_b = vec2<f32>(0.0, 0.0);
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
    return out;
}

struct Painted {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

fn crossing(start: vec2<f32>, toward: vec2<f32>, size: f32) -> f32 {
    let edge = (floor(start / size) + select(vec2<f32>(0.0), vec2<f32>(1.0), toward > vec2<f32>(0.0))) * size;
    let span = select(vec2<f32>(FAR), (edge - start) / toward, abs(toward) > vec2<f32>(1e-6));
    return max(min(span.x, span.y), 0.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> Painted {
    let eye = view.world_position;
    let entry = in.world_position.xyz;
    let toward = normalize(entry - eye);
    let around = field(entry.xz, globals.time, blades.weather.x, blades.tread);
    let shear = around.lean;
    let tall = tallness(entry.xz);
    let pixel = 2.0 / (view.clip_from_view[1][1] * view.viewport.w);

    var t = max(distance(entry, eye) - 0.01, 0.0);
    var best = FAR;
    var struck = Blade(vec3<f32>(0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), 0.0, 0.0, 0.0);
    var ended = false;
    for (var step = 0; step < STEPS; step++) {
        if best < FAR || ended {
            break;
        }
        let at = eye + toward * t;
        let block = vec2<i32>(floor(at.xz));
        let surface = surveyed(block);
        let ground = surface.x;
        let grassy = ground >= 0.0;
        let top = ground + BAND;
        let buried = grassy && at.y < ground - 0.02;
        let risen = (!grassy || at.y > top) && toward.y >= 0.0;
        ended = buried || risen;
        let height = select(0.0, at.y - ground, grassy);
        let sheared = at.xz - shear * height;
        let drift = toward.xz - shear * toward.y;
        let span = min(crossing(sheared, drift, CELL), crossing(at.xz, toward.xz, 1.0)) + 1e-4;
        if grassy && !ended {
            let cell = vec2<i32>(floor(sheared / CELL));
            for (var slot = 0u; slot < PER_CELL; slot++) {
                let grown = blade(cell, slot, ground, tall, around);
                let kept = grown.chance >= handed(grown.root.xz, eye.xz) && grown.chance >= thinned(grown.root.xz, eye.xz);
                let facing = normalize(cross(grown.axis, grown.across));
                let towards = dot(toward, facing);
                if kept && abs(towards) > 1e-5 {
                    let hit = dot(grown.root - eye, facing) / towards;
                    let offset = eye + toward * hit - grown.root;
                    let along = dot(offset, grown.axis) / dot(grown.axis, grown.axis);
                    let wide = max(grown.width, pixel * hit * 1.2) * pow(max(1.0 - along, 0.0), 0.6) * 0.5;
                    if hit >= t - 0.05 && hit < min(best, t + span + 2.0 * CELL) && along >= 0.0 && along <= 1.0 && abs(dot(offset, grown.across)) < wide {
                        best = hit;
                        struck = grown;
                    }
                }
            }
        }
        t += span;
    }
    if best >= FAR {
        discard;
    }

    let point = eye + toward * best;
    let offset = point - struck.root;
    let along = clamp(dot(offset, struck.axis) / dot(struck.axis, struck.axis), 0.0, 1.0);
    let edge = clamp(dot(offset, struck.across) / max(struck.width * 0.5, 1e-4), -1.0, 1.0);
    var flat = normalize(cross(struck.axis, struck.across));
    if dot(flat, -toward) < 0.0 {
        flat = -flat;
    }
    let normal = normalize(mix(normalize(flat + struck.across * edge * 0.7), vec3<f32>(0.0, 1.0, 0.0), 0.5));

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.world_position = vec4<f32>(point, 1.0);
    pbr_input.world_normal = normal;
    pbr_input.N = normal;
    pbr_input.V = -toward;
    let colour = shade(blade_colour(around.lush, along), along, around.gust, struck.odd);
    pbr_input.material.base_color = vec4<f32>(colour, 1.0);
    pbr_input.diffuse_occlusion = vec3<f32>(mix(0.3, 1.0, smoothstep(0.0, 0.7, along)));
    pbr_input.material.perceptual_roughness = mix(0.7, 0.35, blades.weather.y);
    var lit = apply_pbr_lighting(pbr_input);
    lit = main_pass_post_lighting_processing(pbr_input, lit);
    let clip = position_world_to_clip(point);
    return Painted(lit, clip.z / clip.w);
}
