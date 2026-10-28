#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{globals, view},
    mesh_functions,
    view_transformations::position_world_to_clip,
}
#import bloxy::meadow::{lawn, fuzz, lush, blade_colour}

struct Surface {
    dug: vec4<f32>,
    aimed: vec4<f32>,
    weather: vec4<f32>,
    turf: vec4<f32>,
    shone: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> surface: Surface;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var atlas: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var atlas_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var shine: texture_3d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var shine_sampler: sampler;

const HIDDEN: vec4<f32> = vec4<f32>(2.0, 2.0, 2.0, 1.0);
const SEA_TOP: f32 = 62.88;
const ABSORBED: vec3<f32> = vec3<f32>(0.22, 0.09, 0.06);
const WIND: vec3<f32> = vec3<f32>(0.8, 0.0, 0.6);
const SHINE_SPAN: vec3<f32> = vec3<f32>(128.0, 64.0, 128.0);
const BRIGHTEST: f32 = 6.0;
const SHINE_FADE: f32 = 8.0;
const CAVE_DARK: f32 = 0.05;

fn shined(at: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    if surface.shone.w < 0.5 {
        return vec3<f32>(0.0);
    }
    let inside = at + normal * 0.5 - surface.shone.xyz;
    let margin = min(inside, SHINE_SPAN - inside);
    let fade = clamp(min(margin.x, min(margin.y, margin.z)) / SHINE_FADE, 0.0, 1.0);
    let stored = textureSampleLevel(shine, shine_sampler, inside / SHINE_SPAN, 0.0).rgb;
    return stored * stored * BRIGHTEST * fade;
}

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

struct Varying {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
    @location(8) near: vec3<f32>,
}

const ANCHOR: f32 = 16.0;

fn anchor() -> vec3<f32> {
    return floor(view.world_position / ANCHOR) * ANCHOR;
}

@vertex
fn vertex(vertex: Vertex) -> Varying {
    var out: Varying;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    var near = mat3x3<f32>(world_from_local[0].xyz, world_from_local[1].xyz, world_from_local[2].xyz) * vertex.position
        + (world_from_local[3].xyz - anchor());
#ifdef VERTEX_UVS_B
    let owned = ownership(vertex.uv_b.x, world_from_local[3].xyz);
    let swayed = sway(world.xyz, vec2<f32>(owned.y, vertex.uv_b.y), surface.weather.x, globals.time);
    world = vec4<f32>(world.xyz + swayed, 1.0);
    near += swayed;
    out.uv_b = vec2<f32>(owned.x, vertex.uv_b.y);
#endif
    out.world_position = world;
    out.near = near;
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

fn plain(passed: Varying) -> VertexOutput {
    var out: VertexOutput;
    out.position = passed.position;
    out.world_position = passed.world_position;
    out.world_normal = passed.world_normal;
#ifdef VERTEX_UVS_A
    out.uv = passed.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = passed.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = passed.world_tangent;
#endif
#ifdef VERTEX_COLORS
    out.color = passed.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = passed.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = passed.visibility_range_dither;
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
    let soil = vec4<f32>(rocky(23.0, at, normal, sqrt(length(dx) * length(dy)) * 16.0).color, 1.0);
    let ragged = (lumpy(at * 2.3) - 0.5) * 0.3 + (lumpy(at * 7.0) - 0.5) * 0.12;
    let covered = smoothstep(0.34, 0.5, normal.y + ragged);
    var cover = painted(top, tile, at.xz, dx.xz, dy.xz);
    if all(abs(top - surface.turf.xy) < tile * 0.5) {
        let size = sqrt(length(dx.xz) * length(dy.xz)) * 16.0;
        cover = vec4<f32>(select(lawn(at.xz, globals.time, distance(at.xz, view.world_position.xz)), fuzz(at.xz, globals.time, size), surface.turf.z > 0.5), 1.0);
    }
    return mix(soil, cover, covered);
}


fn hashed(cell: vec3<f32>) -> vec3<f32> {
    let p = vec3<f32>(dot(cell, vec3<f32>(127.1, 311.7, 74.7)), dot(cell, vec3<f32>(269.5, 183.3, 246.1)), dot(cell, vec3<f32>(113.5, 271.9, 124.6)));
    return fract(sin(p) * 43758.5453);
}

fn layered(at: vec3<f32>) -> f32 {
    return lumpy(at) * 0.5 + lumpy(at * 2.03 + 17.0) * 0.3 + lumpy(at * 4.1 + 31.0) * 0.2;
}

struct Cells {
    near: f32,
    edge: f32,
    id: vec3<f32>,
}

fn cellular(at: vec3<f32>) -> Cells {
    let base = floor(at);
    var first = 9.0;
    var second = 9.0;
    var id = vec3<f32>(0.0);
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let cell = base + vec3<f32>(f32(x), f32(y), f32(z));
                let seed = cell + hashed(cell) * 0.85;
                let gap = distance(at, seed);
                if gap < first {
                    second = first;
                    first = gap;
                    id = cell;
                } else if gap < second {
                    second = gap;
                }
            }
        }
    }
    return Cells(first, second - first, id);
}

struct Finish {
    color: vec3<f32>,
    height: f32,
    glow: vec3<f32>,
    rough: f32,
    metal: f32,
    slope: vec3<f32>,
}

fn bit(mask: f32, index: f32) -> bool {
    return floor(mask / exp2(index)) % 2.0 > 0.5;
}

fn border(mask: f32, along: vec2<f32>) -> f32 {
    var gap = 9.0;
    if !bit(mask, 0.0) { gap = min(gap, 1.0 - along.x); }
    if !bit(mask, 1.0) { gap = min(gap, along.x); }
    if !bit(mask, 2.0) { gap = min(gap, 1.0 - along.y); }
    if !bit(mask, 3.0) { gap = min(gap, along.y); }
    if bit(mask, 0.0) && bit(mask, 2.0) && !bit(mask, 4.0) { gap = min(gap, length(vec2<f32>(1.0) - along)); }
    if bit(mask, 1.0) && bit(mask, 2.0) && !bit(mask, 5.0) { gap = min(gap, length(vec2<f32>(along.x, 1.0 - along.y))); }
    if bit(mask, 0.0) && bit(mask, 3.0) && !bit(mask, 6.0) { gap = min(gap, length(vec2<f32>(1.0 - along.x, along.y))); }
    if bit(mask, 1.0) && bit(mask, 3.0) && !bit(mask, 7.0) { gap = min(gap, length(along)); }
    return gap;
}

const AETHER: vec3<f32> = vec3<f32>(0.25, 0.95, 1.0);

fn traces(plane: vec2<f32>, dense: f32, time: f32) -> f32 {
    let grid = plane * 4.0;
    let cell = floor(grid);
    let inside = fract(grid) - 0.5;
    let pick = hashed(vec3<f32>(cell, 7.0));
    let cluster = lumpy(vec3<f32>(plane * 0.35, 3.0));
    if cluster < 0.55 - dense * 0.4 {
        return 0.0;
    }
    var line = 9.0;
    if pick.x < 0.45 { line = min(line, abs(inside.y)); }
    if pick.y < 0.45 { line = min(line, abs(inside.x)); }
    if pick.z < 0.2 { line = min(line, abs(inside.x - inside.y) * 0.7071); }
    let node = select(9.0, length(inside), pick.x < 0.45 && pick.y < 0.45);
    let pulse = 0.55 + 0.45 * sin(time * 2.2 - (plane.x + plane.y) * 2.5 + pick.z * 6.0);
    let lit = 1.0 - smoothstep(0.03, 0.07, line);
    let knot = 1.0 - smoothstep(0.1, 0.16, node);
    return max(lit * pulse, knot);
}

fn texel_hash(cell: vec3<i32>, salt: u32) -> f32 {
    var state = u32(cell.x) * 73856093u ^ u32(cell.y) * 19349663u ^ u32(cell.z) * 83492791u ^ salt * 2654435761u;
    state = state * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return f32(((word >> 22u) ^ word) & 0xffffffu) / 16777216.0;
}

fn lattice_hash(cell: vec3<i32>) -> u32 {
    var state = u32(cell.x) * 73856093u ^ u32(cell.y) * 19349663u ^ u32(cell.z) * 83492791u;
    state = state * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn slope(cell: vec3<i32>, offset: vec3<f32>) -> f32 {
    let pick = lattice_hash(cell) & 15u;
    let u = select(offset.y, offset.x, pick < 8u);
    let v = select(select(offset.z, offset.x, pick == 12u || pick == 14u), offset.y, pick < 4u);
    return select(-u, u, (pick & 1u) == 0u) + select(-v, v, (pick & 2u) == 0u);
}

fn supple(at: vec3<f32>) -> f32 {
    let base = floor(at);
    let cell = vec3<i32>(base);
    let f = at - base;
    let fade = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let x00 = mix(slope(cell, f), slope(cell + vec3<i32>(1, 0, 0), f - vec3<f32>(1.0, 0.0, 0.0)), fade.x);
    let x10 = mix(slope(cell + vec3<i32>(0, 1, 0), f - vec3<f32>(0.0, 1.0, 0.0)), slope(cell + vec3<i32>(1, 1, 0), f - vec3<f32>(1.0, 1.0, 0.0)), fade.x);
    let x01 = mix(slope(cell + vec3<i32>(0, 0, 1), f - vec3<f32>(0.0, 0.0, 1.0)), slope(cell + vec3<i32>(1, 0, 1), f - vec3<f32>(1.0, 0.0, 1.0)), fade.x);
    let x11 = mix(slope(cell + vec3<i32>(0, 1, 1), f - vec3<f32>(0.0, 1.0, 1.0)), slope(cell + vec3<i32>(1, 1, 1), f - vec3<f32>(1.0, 1.0, 1.0)), fade.x);
    let value = mix(mix(x00, x10, fade.y), mix(x01, x11, fade.y), fade.z);
    return clamp(value * 0.5 + 0.5, 0.0, 1.0);
}

fn ramped(palette: array<vec3<f32>, 4>, along: f32) -> vec3<f32> {
    var points = array<vec3<f32>, 6>(palette[0], palette[0], palette[1], palette[2], palette[3], palette[3]);
    let reach = (along - 0.5) * 2.0;
    let eased = reach / sqrt(sqrt(1.0 + reach * reach * reach * reach));
    let position = (0.5 + 0.5 * eased) * 3.999;
    let low = i32(floor(position));
    let f = position - f32(low);
    return points[low] * (0.5 * (1.0 - f) * (1.0 - f)) + points[low + 1] * (0.5 + f - f * f) + points[low + 2] * (0.5 * f * f);
}

fn shades(base: vec3<f32>) -> array<vec3<f32>, 4> {
    return array<vec3<f32>, 4>(base * 0.62, base * 0.83, base, base * 1.17);
}

const GREY: array<vec3<f32>, 4> = array<vec3<f32>, 4>(vec3<f32>(0.3, 0.3, 0.32), vec3<f32>(0.4, 0.4, 0.42), vec3<f32>(0.48, 0.48, 0.5), vec3<f32>(0.56, 0.56, 0.58));

fn rock_palette(kind: f32) -> array<vec3<f32>, 4> {
    let rock = select(kind, kind - 19.0, kind > 28.5);
    if rock > 9.5 && rock < 10.5 {
        return shades(vec3<f32>(0.6, 0.42, 0.37));
    } else if rock > 10.5 && rock < 11.5 {
        return shades(vec3<f32>(0.72, 0.71, 0.67));
    } else if rock > 11.5 && rock < 12.5 {
        return shades(vec3<f32>(0.4, 0.44, 0.42));
    } else if rock > 12.5 && rock < 13.5 {
        return shades(vec3<f32>(0.74, 0.68, 0.53));
    } else if rock > 13.5 && rock < 14.5 {
        return shades(vec3<f32>(0.25, 0.27, 0.33));
    } else if kind > 14.5 && kind < 15.5 {
        return array<vec3<f32>, 4>(vec3<f32>(0.26, 0.25, 0.24), vec3<f32>(0.4, 0.39, 0.37), vec3<f32>(0.52, 0.51, 0.49), vec3<f32>(0.64, 0.63, 0.61));
    } else if kind > 22.5 && kind < 23.5 {
        return array<vec3<f32>, 4>(vec3<f32>(0.25, 0.17, 0.11), vec3<f32>(0.36, 0.25, 0.16), vec3<f32>(0.44, 0.31, 0.2), vec3<f32>(0.53, 0.39, 0.26));
    } else if kind > 23.5 && kind < 24.5 {
        return array<vec3<f32>, 4>(vec3<f32>(0.7, 0.62, 0.42), vec3<f32>(0.8, 0.72, 0.5), vec3<f32>(0.86, 0.79, 0.56), vec3<f32>(0.92, 0.87, 0.66));
    } else if kind > 24.5 && kind < 25.5 {
        return array<vec3<f32>, 4>(vec3<f32>(0.5, 0.53, 0.6), vec3<f32>(0.56, 0.59, 0.66), vec3<f32>(0.61, 0.64, 0.71), vec3<f32>(0.68, 0.71, 0.78));
    } else if kind > 25.5 && kind < 26.5 {
        return shades(vec3<f32>(0.22, 0.22, 0.24));
    } else if kind > 26.5 && kind < 27.5 {
        return array<vec3<f32>, 4>(vec3<f32>(0.08, 0.08, 0.09), vec3<f32>(0.18, 0.18, 0.19), vec3<f32>(0.3, 0.3, 0.31), vec3<f32>(0.42, 0.42, 0.43));
    }
    return GREY;
}

struct Vein {
    dark: vec3<f32>,
    base: vec3<f32>,
    light: vec3<f32>,
    glow: f32,
}

fn vein(kind: f32) -> Vein {
    if kind < 16.5 {
        return Vein(vec3<f32>(0.05, 0.05, 0.06), vec3<f32>(0.12, 0.12, 0.13), vec3<f32>(0.22, 0.22, 0.24), 0.0);
    } else if kind < 17.5 {
        return Vein(vec3<f32>(0.55, 0.4, 0.32), vec3<f32>(0.76, 0.6, 0.5), vec3<f32>(0.88, 0.75, 0.65), 0.0);
    } else if kind < 18.5 {
        return Vein(vec3<f32>(0.55, 0.28, 0.12), vec3<f32>(0.82, 0.47, 0.22), vec3<f32>(0.95, 0.65, 0.38), 0.0);
    } else if kind < 19.5 {
        return Vein(vec3<f32>(0.6, 0.62, 0.66), vec3<f32>(0.8, 0.82, 0.85), vec3<f32>(0.93, 0.94, 0.96), 0.0);
    } else if kind < 20.5 {
        return Vein(vec3<f32>(0.7, 0.52, 0.1), vec3<f32>(0.96, 0.8, 0.24), vec3<f32>(1.0, 0.93, 0.55), 0.35);
    } else if kind < 21.5 {
        return Vein(vec3<f32>(0.18, 0.6, 0.65), vec3<f32>(0.45, 0.92, 0.95), vec3<f32>(0.8, 1.0, 1.0), 1.0);
    }
    return Vein(vec3<f32>(0.6, 0.62, 0.66), vec3<f32>(0.84, 0.86, 0.9), vec3<f32>(1.0, 1.0, 1.0), 0.15);
}

struct Pebble {
    near: f32,
    edge: f32,
    id: vec3<i32>,
    centre: vec3<f32>,
}

fn pebbles(at: vec3<f32>, flat: bool) -> Pebble {
    let base = floor(at - 0.5);
    var first = 9.0;
    var second = 9.0;
    var id = vec3<i32>(0);
    var centre = vec3<f32>(0.0);
    for (var i = 0; i < 8; i++) {
        let step = vec3<f32>(f32(i & 1), f32((i >> 1u) & 1), f32((i >> 2u) & 1));
        let cell = base + select(step, vec3<f32>(step.x, 0.0, step.z), flat);
        let key = vec3<i32>(cell);
        let seed = cell + vec3<f32>(texel_hash(key, 3u), select(texel_hash(key, 4u), 0.5, flat), texel_hash(key, 5u)) * 0.5 + 0.25;
        let gap = select(distance(at, seed), distance(at.xz, seed.xz), flat) * (0.8 + texel_hash(key, 6u) * 0.4);
        if gap < first {
            second = first;
            first = gap;
            id = key;
            centre = seed;
        } else if gap < second {
            second = gap;
        }
    }
    return Pebble(first, second - first, id, centre);
}

fn below(value: f32, edge: f32) -> f32 {
    return 1.0 - smoothstep(edge - 0.08, edge + 0.08, value);
}

struct Spot {
    near: vec3<f32>,
    anchor: vec3<i32>,
}

struct Lattice {
    cell: vec3<i32>,
    inside: vec3<f32>,
}

fn lattice(spot: Spot, frequency: i32, shift: vec3<f32>) -> Lattice {
    let local = spot.near * f32(frequency) + shift;
    let base = floor(local);
    return Lattice(spot.anchor * frequency + vec3<i32>(base), local - base);
}

fn mingled(cell: vec3<i32>, salt: u32) -> u32 {
    var v = bitcast<vec3<u32>>(cell) * 1664525u + vec3<u32>(1013904223u + salt);
    v.x += v.y * v.z;
    v.y += v.z * v.x;
    v.z += v.x * v.y;
    v = v ^ (v >> vec3<u32>(16u));
    return v.x + v.y * v.z;
}

fn roll(cell: vec3<i32>, salt: u32) -> f32 {
    return f32(mingled(cell, salt) & 0xffffu) / 65535.0;
}

fn jitter(cell: vec3<i32>, salt: u32) -> vec3<f32> {
    let h = mingled(cell, salt);
    return vec3<f32>(f32(h & 1023u), f32((h >> 10u) & 1023u), f32((h >> 20u) & 1023u)) / 1023.0;
}

fn wave(at: Lattice, salt: u32) -> vec4<f32> {
    let f = at.inside;
    let c = at.cell;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
    let ga = jitter(c, salt) * 2.0 - 1.0;
    let gb = jitter(c + vec3(1, 0, 0), salt) * 2.0 - 1.0;
    let gc = jitter(c + vec3(0, 1, 0), salt) * 2.0 - 1.0;
    let gd = jitter(c + vec3(1, 1, 0), salt) * 2.0 - 1.0;
    let ge = jitter(c + vec3(0, 0, 1), salt) * 2.0 - 1.0;
    let gf = jitter(c + vec3(1, 0, 1), salt) * 2.0 - 1.0;
    let gg = jitter(c + vec3(0, 1, 1), salt) * 2.0 - 1.0;
    let gh = jitter(c + vec3(1, 1, 1), salt) * 2.0 - 1.0;
    let va = dot(ga, f);
    let vb = dot(gb, f - vec3(1.0, 0.0, 0.0));
    let vc = dot(gc, f - vec3(0.0, 1.0, 0.0));
    let vd = dot(gd, f - vec3(1.0, 1.0, 0.0));
    let ve = dot(ge, f - vec3(0.0, 0.0, 1.0));
    let vf = dot(gf, f - vec3(1.0, 0.0, 1.0));
    let vg = dot(gg, f - vec3(0.0, 1.0, 1.0));
    let vh = dot(gh, f - vec3(1.0, 1.0, 1.0));
    let value = va + u.x * (vb - va) + u.y * (vc - va) + u.z * (ve - va)
        + u.x * u.y * (va - vb - vc + vd) + u.y * u.z * (va - vc - ve + vg)
        + u.z * u.x * (va - vb - ve + vf) + u.x * u.y * u.z * (-va + vb + vc - vd + ve - vf - vg + vh);
    let slope = ga + u.x * (gb - ga) + u.y * (gc - ga) + u.z * (ge - ga)
        + u.x * u.y * (ga - gb - gc + gd) + u.y * u.z * (ga - gc - ge + gg)
        + u.z * u.x * (ga - gb - ge + gf) + u.x * u.y * u.z * (-ga + gb + gc - gd + ge - gf - gg + gh)
        + du * (vec3<f32>(vb - va, vc - va, ve - va)
            + u.yzx * vec3<f32>(va - vb - vc + vd, va - vc - ve + vg, va - vb - ve + vf)
            + u.zxy * vec3<f32>(va - vb - ve + vf, va - vb - vc + vd, va - vc - ve + vg)
            + u.yzx * u.zxy * (-va + vb + vc - vd + ve - vf - vg + vh));
    return vec4<f32>(value, slope);
}

struct Relief {
    height: f32,
    slope: vec3<f32>,
    lost: f32,
}

fn weierstrass(spot: Spot, first: i32, octaves: i32, hurst: f32, rough: f32, footprint: f32, salt: u32) -> Relief {
    var relief = Relief(0.0, vec3<f32>(0.0), 0.0);
    var frequency = first;
    for (var octave = 0; octave < octaves; octave++) {
        let wavelength = 1.0 / f32(frequency);
        let amplitude = rough * pow(wavelength, hurst);
        let fade = 1.0 - smoothstep(0.15, 0.45, f32(frequency) * footprint);
        let steep = amplitude * f32(frequency);
        relief.lost += steep * steep * (1.0 - fade) * 0.25;
        if fade > 0.0 {
            let n = wave(lattice(spot, frequency, vec3<f32>(f32(octave) * 0.37)), salt + u32(octave) * 7u);
            relief.height += n.x * amplitude * fade;
            relief.slope += n.yzw * steep * fade;
        }
        frequency *= 2;
    }
    return relief;
}

struct Grain {
    nearest: f32,
    gap: f32,
    id: vec3<i32>,
    offset: vec3<f32>,
}

fn grains(spot: Spot, frequency: i32, shift: vec3<f32>, salt: u32) -> Grain {
    let at = lattice(spot, frequency, shift);
    var found = Grain(9.0, 9.0, at.cell, vec3<f32>(0.0));
    var second = 9.0;
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let step = vec3<i32>(x, y, z);
                let offset = at.inside - vec3<f32>(step) - jitter(at.cell + step, salt);
                let distance = length(offset);
                if distance < found.nearest {
                    second = found.nearest;
                    found.nearest = distance;
                    found.id = at.cell + step;
                    found.offset = offset;
                } else if distance < second {
                    second = distance;
                }
            }
        }
    }
    found.gap = second - found.nearest;
    return found;
}

fn tangential(slope: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    return slope - normal * dot(slope, normal);
}

fn tilted(normal: vec3<f32>, slope: vec3<f32>) -> vec3<f32> {
    return normalize(normal - tangential(slope, normal));
}

fn rocky(kind: f32, at: vec3<f32>, normal: vec3<f32>, size: f32) -> Finish {
    let fine = at * 16.0 - normal * 0.02;
    let gravel = kind > 14.5 && kind < 15.5;
    let cobbled = kind > 27.5;
    let cell = vec3<i32>(floor(fine));
    let point = fine;
    let centre = point / 16.0;
    let palette = rock_palette(kind);
    let speckle = smoothstep(0.0, 1.0, (supple(fine * 0.9 + 13.0) - 0.5) * 2.4 + 0.5);
    let pick = speckle;
    let cloud = supple(centre * 3.0) * 0.55 + supple(centre * 1.1 + 7.0) * 0.45;
    var along = 0.5 + (cloud - 0.5) * 1.5 + (pick - 0.5) * 0.12;
    var color = vec3<f32>(0.0);
    var glow = 0.0;
    var rough = 0.9;
    var relief = 0.0;
    let clump = smoothstep(0.0, 1.0, (supple(fine * 0.45 + 29.0) - 0.5) * 2.2 + 0.5);
    let bedded = (kind > 8.5 && kind < 14.5) || (kind > 15.5 && kind < 22.5) || (kind > 25.5 && kind < 27.5);
    let sharp = 1.0 - smoothstep(0.3, 1.0, size);
    let grain = (supple(fine * 0.3 + 3.0) - 0.5) * 0.55 + ((supple(fine * 1.1 + 41.0) - 0.5) * 0.4 + (supple(fine * 2.7 + 59.0) - 0.5) * 0.3) * sharp;
    var seam = 0.0;
    var worn = 0.0;
    var facet = 0.0;
    var stain = 0.0;
    var tint = vec3<f32>(1.0);
    if bedded {
        let blocky = select(1.7, 1.15, kind > 9.5 && kind < 11.5);
        let warp = vec3<f32>(supple(centre * 2.1 + 3.0), supple(centre * 2.1 + 17.0), supple(centre * 2.1 + 29.0)) - 0.5;
        let split = cellular(centre * blocky + warp * 0.9);
        let wide = max(0.045, size * 0.3);
        let cracked = smoothstep(0.44, 0.5, supple(centre * 1.2 + 9.0 + kind));
        seam = (1.0 - smoothstep(0.0, wide, split.edge)) * min(1.0, 0.07 / wide) * cracked;
        worn = (1.0 - smoothstep(0.03, 0.12, split.edge)) * cracked;
        facet = (hashed(split.id + kind).x - 0.5) * 0.16;
        stain = smoothstep(0.66, 0.85, supple(vec3<f32>(centre.x * 1.8, centre.y * 0.5, centre.z * 1.8) + 31.0 + kind));
        tint = mix(vec3<f32>(0.97, 0.99, 1.03), vec3<f32>(1.04, 1.0, 0.95), supple(centre * 0.6 + 5.0 + kind));
    }
    if cobbled || gravel {
        let scale = select(3.0, 5.0, cobbled);
        let stone = pebbles(point / scale, false);
        let tone = texel_hash(stone.id, 9u);
        let lit = dot(normalize(point / scale - stone.centre + vec3<f32>(0.0, 0.001, 0.0)), normalize(vec3<f32>(-0.4, 0.8, -0.4)));
        let rim = select(0.42, 0.9, cobbled);
        let body = 1.0 - smoothstep(rim - 0.08, rim, stone.near);
        let mortar = smoothstep(0.02, 0.08, stone.edge);
        let warm = select(vec3<f32>(1.0), vec3<f32>(1.1, 0.98, 0.86), texel_hash(stone.id, 16u) < 0.3);
        let grit = palette[0] * (0.42 + clump * 0.12);
        along = 0.25 + tone * 0.5 + lit * 0.22 * smoothstep(0.0, 0.35, stone.near) + (pick - 0.5) * 0.08;
        color = mix(grit, ramped(palette, along) * warm, body * mortar);
        relief = body * mortar * (0.5 + lit * 0.3) + grain * 0.2;
    } else if kind > 9.5 && kind < 11.5 {
        let granite = kind < 10.5;
        let crystal = pebbles(fine * 2.6 + 3.1, false);
        let mineral = texel_hash(crystal.id, 25u + u32(kind));
        let shade = texel_hash(crystal.id, 26u) - 0.5;
        let boundary = (1.0 - smoothstep(0.0, 0.08, crystal.edge)) * (1.0 - smoothstep(0.05, 0.15, size));
        var grains = ramped(palette, 0.5 + shade * 0.4);
        if granite {
            grains = select(select(select(vec3<f32>(0.74, 0.6, 0.55), vec3<f32>(0.58, 0.58, 0.6), mineral > 0.42), vec3<f32>(0.1, 0.09, 0.09), mineral > 0.8), vec3<f32>(0.86, 0.84, 0.8), mineral > 0.92) * (1.0 + shade * 0.25);
        } else {
            grains = select(select(vec3<f32>(0.8, 0.8, 0.77), vec3<f32>(0.62, 0.62, 0.6), mineral > 0.55), vec3<f32>(0.13, 0.13, 0.14), mineral > 0.78) * (1.0 + shade * 0.2);
        }
        let mosaic = grains * (1.0 - boundary * 0.12);
        let far_tone = ramped(palette, along * 0.6 + 0.2);
        color = mix(mosaic, mix(far_tone, select(vec3<f32>(0.6, 0.52, 0.5), vec3<f32>(0.62, 0.62, 0.6), kind > 10.5), 0.6), smoothstep(0.12, 0.45, size)) * (1.0 + (cloud - 0.5) * 0.25 + grain * 0.3);
        relief = grain * 0.25 + (shade * 0.04 - boundary * 0.03) * (1.0 - smoothstep(0.05, 0.2, size));
    } else if kind > 11.5 && kind < 12.5 {
        color = ramped(palette, 0.5 + (cloud - 0.5) * 0.8 + (pick - 0.5) * 0.3 + grain * 0.7);
        relief = grain * 0.5 + (pick - 0.5) * 0.15;
    } else if kind > 12.5 && kind < 13.5 {
        let bed = supple(vec3<f32>(centre.x * 0.7, centre.y * 5.0, centre.z * 0.7));
        color = ramped(palette, 0.2 + bed * 0.6 + (cloud - 0.5) * 0.4);
        let pit = smoothstep(0.74, 0.8, supple(fine * 1.4 + 11.0)) * sharp;
        color = mix(color, palette[0] * 0.9, pit * 0.45);
        color = color * (1.0 + grain * 0.4);
        relief = bed * 0.4 + grain * 0.3 - pit * 0.06;
    } else if kind > 13.5 && kind < 14.5 {
        let row = point.y + supple(centre * 1.3) * 3.0;
        let cleave = smoothstep(0.33, 0.43, abs(fract(row / 3.0) - 0.5));
        color = ramped(palette, along - 0.25 * cleave - 0.05 + grain * 0.3);
        relief = -cleave * 0.8 + grain * 0.2;
        rough = 0.62;
    } else if kind > 22.5 && kind < 23.5 {
        let warped = point / 1.8 + (vec3<f32>(supple(point * 0.35), supple(point * 0.35 + 19.0), supple(point * 0.35 + 37.0)) - 0.5) * 1.6;
        let clod = pebbles(warped, false);
        let lit = dot(normalize(warped - clod.centre + vec3<f32>(0.0, 0.001, 0.0)), normalize(vec3<f32>(-0.4, 0.8, -0.4)));
        let crumb = 1.0 - smoothstep(0.1, 0.6, clod.near);
        let crevice = (1.0 - smoothstep(0.0, 0.14, clod.edge)) * smoothstep(0.4, 0.7, supple(point * 0.5 + 71.0));
        let crumbling = supple(point * 1.7 + 61.0);
        let crumbs = smoothstep(0.66, 0.78, supple(point * 0.8 + 83.0));
        let flecks = smoothstep(0.7, 0.8, supple(point * 1.2 + 97.0));
        let broad = supple(centre * 1.6 + 23.0) * 0.6 + supple(centre * 4.5 + 3.0) * 0.4;
        let soil = 0.45 + (broad - 0.5) * 0.7 + (texel_hash(clod.id, 18u) - 0.5) * 0.12 + lit * 0.1 * crumb + (crumbling - 0.5) * 0.25 - crevice * 0.14 - crumbs * 0.22 + flecks * 0.14;
        color = ramped(palette, soil);
        let stone = pebbles(point / 5.0 + 7.3, false);
        let present = texel_hash(stone.id, 19u) < 0.14;
        let shine = dot(normalize(point / 5.0 + 7.3 - stone.centre + vec3<f32>(0.0, 0.001, 0.0)), normalize(vec3<f32>(-0.4, 0.8, -0.4)));
        let pebble = select(0.0, 1.0 - smoothstep(0.2, 0.32, stone.near), present);
        color = mix(color, vec3<f32>(0.4, 0.36, 0.31) * (0.8 + 0.35 * shine), pebble);
        rough = 0.95;
    } else if kind > 23.5 && kind < 24.5 {
        let ripple = supple(vec3<f32>(centre.x * 2.0 + centre.z * 0.5, centre.y * 6.0, centre.z * 2.0 - centre.x * 0.5));
        color = ramped(palette, 0.45 + (ripple - 0.5) * 0.5 + (pick - 0.5) * 0.3);
        rough = 1.0;
    } else if kind > 24.5 && kind < 25.5 {
        let band = supple(vec3<f32>(centre.x * 0.5, centre.y * 4.0, centre.z * 0.5));
        color = ramped(palette, 0.35 + band * 0.4 + (pick - 0.5) * 0.1);
        rough = 0.85;
    } else if kind > 25.5 && kind < 26.5 {
        let column = pebbles(point / 6.0, true);
        let joint = 1.0 - smoothstep(0.03, 0.1, column.edge);
        let crack = 1.0 - below(abs(fract((point.y + texel_hash(column.id, 7u) * 16.0) / 16.0) - 0.5), 0.47);
        color = ramped(palette, 0.35 + texel_hash(column.id, 8u) * 0.3 + (pick - 0.5) * 0.15) * (1.0 - joint * 0.6) * (1.0 - 0.4 * crack);
        relief = grain * 0.3 - joint * 0.9 - crack * 0.5;
        rough = 0.75;
    } else if kind > 26.5 && kind < 27.5 {
        let crack = pebbles(point / 4.0, false);
        color = ramped(palette, along * 0.8 + 0.1) * (0.5 + 0.5 * smoothstep(0.02, 0.1, crack.edge));
        relief = grain * 0.5 - (1.0 - smoothstep(0.02, 0.1, crack.edge)) * 0.8;
    } else {
        let dark = smoothstep(0.68, 0.78, supple(fine * 1.5 + 13.0)) * sharp;
        let light = smoothstep(0.7, 0.8, supple(fine * 1.8 + 77.0)) * sharp;
        color = ramped(palette, 0.5 + (cloud - 0.5) * 0.8 + grain - dark * 0.3 + light * 0.22);
        relief = grain * 0.45 - dark * 0.1;
        rough = 0.82 + grain * 0.2;
    }
    if bedded {
        let weathered = vec3<f32>(0.5, 0.42, 0.34) * (0.85 + grain);
        color = mix(color * tint * (1.0 + facet * 0.9 + worn * 0.1), weathered, stain * 0.22);
        color = mix(color, palette[0] * 0.45, seam);
        relief = relief + (cloud - 0.5) * 0.5 + facet * 1.5 - seam * 0.25;
    }
    if kind > 15.5 && kind < 22.5 {
        let ore = vein(kind);
        let blob = floor(point / 5.0);
        let present = texel_hash(vec3<i32>(blob), 11u) < 0.7;
        let middle = blob * 5.0 + 1.0 + vec3<f32>(texel_hash(vec3<i32>(blob), 12u), texel_hash(vec3<i32>(blob), 13u), texel_hash(vec3<i32>(blob), 14u)) * 3.0;
        let toward = point - middle;
        let inside = select(0.0, below(length(toward), 1.3 + texel_hash(vec3<i32>(blob), 15u) * 0.8), present);
        let lean = dot(toward, vec3<f32>(-0.5, 0.7, -0.5));
        let stained = mix(mix(ore.dark, ore.base, smoothstep(-0.8, -0.2, lean)), ore.light, smoothstep(0.2, 0.7, lean));
        color = mix(color, stained, inside);
        glow = ore.glow * inside;
        rough = mix(rough, 0.5, inside);
    }
    let broad = supple(centre * 1.1 + 7.0) - 0.5;
    let flat = ramped(palette, select(0.5, 0.4, cobbled || gravel) + broad * 0.7) * select(1.0, 0.85, cobbled || gravel);
    let coarse = 0.35 + 0.65 * smoothstep(0.35, 0.7, supple(centre * 0.45 + 53.0));
    let near = 1.0 - smoothstep(0.08, 0.3, size);
    let nearest = 1.0 - smoothstep(0.03, 0.12, size);
    let fine_grain = (supple(fine * 5.3 + 7.0) - 0.5) * near * coarse;
    let finer_grain = (supple(fine * 13.7 + 19.0) - 0.5) * nearest * coarse;
    let grit = smoothstep(0.62, 0.7, supple(fine * 9.1 + 37.0)) * nearest * coarse;
    let micro = fine_grain * 0.07 + finer_grain * 0.025 - grit * 0.02;
    let detailed = color * (1.0 + fine_grain * 0.18 + finer_grain * 0.12 - grit * 0.12);
    let shown = mix(detailed, flat, smoothstep(0.8, 2.0, size) * (1.0 - smoothstep(0.0, 0.05, glow)));
    return Finish(shown, relief * (1.0 - smoothstep(0.5, 2.0, size)) + micro, shown * glow * 0.6, rough + fine_grain * 0.1, 0.0, vec3<f32>(0.0));
}

const SAND: array<vec3<f32>, 4> = array<vec3<f32>, 4>(vec3<f32>(0.7, 0.62, 0.42), vec3<f32>(0.8, 0.72, 0.5), vec3<f32>(0.86, 0.79, 0.56), vec3<f32>(0.92, 0.87, 0.66));
const RIPPLE: f32 = 0.17;
const RIPPLE_HEIGHT: f32 = 0.01;
const STOSS: f32 = 0.72;
const SAND_GRAINS: i32 = 700;
const SAND_FINES: i32 = 1600;
const SAND_MEAN: f32 = 0.74;
const SHELLS: i32 = 46;
const PEBBLES: i32 = 18;

fn edged(grain: Grain) -> f32 {
    return grain.nearest / max(grain.nearest + grain.gap * 0.5, 0.0001);
}

fn domed(grain: Grain, normal: vec3<f32>, steep: f32) -> vec3<f32> {
    let toward = tangential(grain.offset, normal);
    let edge = edged(grain);
    return -toward / max(length(toward), 0.0001) * edge * edge * steep;
}

fn sandy(at: vec3<f32>, spot: Spot, normal: vec3<f32>, size: f32) -> Finish {
    let footprint = size / 16.0;
    let broad = supple(at * 0.35 + 7.0) * 0.6 + supple(at * 1.3 + 3.0) * 0.4;
    let up = smoothstep(0.55, 0.9, normal.y);
    let turn = (supple(at * 0.12 + 41.0) - 0.5) * 1.6;
    let toward = vec2<f32>(cos(0.64 + turn), sin(0.64 + turn));
    let phase = dot(at.xz, toward) / RIPPLE + (supple(at * 0.8 + 5.0) - 0.5) * 3.0 + (supple(at * 3.1 + 9.0) - 0.5) * 0.5;
    let crest = fract(phase);
    let rise = select((1.0 - crest) / (1.0 - STOSS), crest / STOSS, crest < STOSS);
    let profile = smoothstep(0.0, 1.0, rise);
    let pitch = select(-1.0 / (1.0 - STOSS), 1.0 / STOSS, crest < STOSS) * 6.0 * rise * (1.0 - rise);
    let rippled = up * smoothstep(0.35, 0.6, supple(at * 0.5 + 19.0)) * smoothstep(2.5, 7.0, RIPPLE / footprint);
    let trough = (1.0 - profile) * rippled;
    var slope = vec3<f32>(toward.x, 0.0, toward.y) * pitch * RIPPLE_HEIGHT / RIPPLE * rippled;
    let laminae = (1.0 - up) * (supple(vec3<f32>(at.x * 0.6, at.y * 26.0, at.z * 0.6) + 3.0) - 0.5) * smoothstep(2.0, 6.0, 0.04 / footprint);
    var color = ramped(SAND, 0.5 + (broad - 0.5) * 0.6 + (profile - 0.5) * 0.25 * rippled + laminae * 0.5);
    var rough = 0.92;

    let grain_pixel = footprint * f32(SAND_GRAINS);
    let seen = 1.0 - smoothstep(0.18, 0.55, grain_pixel);
    if seen > 0.0 {
        let grain = grains(spot, SAND_GRAINS, vec3<f32>(0.0), 11u);
        let pick = roll(grain.id, 12u) - trough * 0.1;
        let shade = roll(grain.id, 13u);
        var tint = mix(vec3<f32>(1.12, 1.09, 1.02), vec3<f32>(0.98, 0.88, 0.72), roll(grain.id, 18u)) * (0.76 + shade * 0.44);
        var grit = 0.28;
        var glassy = 1.0;
        if pick < 0.025 {
            tint = vec3<f32>(0.2, 0.19, 0.2) * (0.8 + shade * 0.4);
            grit = 0.22;
            glassy = 0.0;
        } else if pick < 0.13 {
            tint = vec3<f32>(0.6, 0.57, 0.55) * (0.75 + shade * 0.4);
            grit = 0.65;
            glassy = 0.0;
        } else if pick < 0.17 {
            tint = vec3<f32>(0.95, 0.52, 0.4) * (0.85 + shade * 0.3);
            grit = 0.4;
            glassy = 0.5;
        } else if pick < 0.31 {
            tint = vec3<f32>(1.12, 0.86, 0.7) * (0.9 + shade * 0.2);
            grit = 0.55;
            glassy = 0.2;
        } else if pick < 0.36 {
            tint = vec3<f32>(1.2, 1.17, 1.1);
            grit = 0.6;
            glassy = 0.0;
        }
        let edge = edged(grain);
        let sunk = roll(grain.id, 14u) < 0.07;
        let seam = 1.0 - smoothstep(0.0, 0.06 + grain_pixel, grain.gap);
        tint = tint * (1.0 + glassy * 0.3 * (1.0 - edge) * (1.0 - edge)) * (1.0 - 0.5 * pow(edge, 2.5));
        let fine_pixel = footprint * f32(SAND_FINES);
        let fine_seen = 1.0 - smoothstep(0.18, 0.55, fine_pixel);
        var pore = vec3<f32>(0.4);
        var pore_slope = vec3<f32>(0.0);
        if fine_seen > 0.0 && (sunk || seam > 0.01) {
            let fine = grains(spot, SAND_FINES, vec3<f32>(0.0), 15u);
            let fine_seam = 1.0 - smoothstep(0.0, 0.08 + fine_pixel, fine.gap);
            pore = vec3<f32>(mix(0.4, (0.3 + roll(fine.id, 16u) * 0.35) * (1.0 - 0.5 * fine_seam) * (1.0 - 0.3 * pow(edged(fine), 3.0)), fine_seen));
            pore_slope = domed(fine, normal, 1.2) * fine_seen;
        }
        let low = select(seam, 1.0, sunk);
        let grain_slope = domed(grain, normal, 1.5) + (jitter(grain.id, 17u) - 0.5) * 0.6 * glassy;
        slope += mix(grain_slope * (1.0 - seam), pore_slope, low) * seen;
        let factor = mix(tint, pore, low) / SAND_MEAN;
        color = color * mix(vec3<f32>(1.0), factor, seen);
        rough = mix(rough, mix(grit, 0.95, low), seen);
    }

    let shell_pixel = footprint * f32(SHELLS);
    if shell_pixel < 0.4 {
        let shown = 1.0 - smoothstep(0.2, 0.4, shell_pixel);
        let shard = grains(spot, SHELLS, vec3<f32>(supple(at * 70.0), supple(at * 70.0 + 9.0), supple(at * 70.0 + 17.0)) * 0.5, 21u);
        let present = select(0.0, 1.0, roll(shard.id, 22u) < 0.012 + up * 0.012);
        let inside = smoothstep(0.03, 0.03 + shell_pixel * 2.0, shard.gap) * (1.0 - smoothstep(0.3, 0.3 + shell_pixel, shard.nearest)) * present * shown;
        let rim = (1.0 - smoothstep(0.3, 0.42, shard.nearest)) * present * shown * (1.0 - inside);
        let axis = normalize(jitter(shard.id, 24u) - 0.5);
        let ribs = sin(dot(shard.offset, axis) * 70.0);
        let shell = mix(vec3<f32>(0.97, 0.94, 0.88), vec3<f32>(0.93, 0.72, 0.64), roll(shard.id, 23u) * roll(shard.id, 29u)) * (0.86 + ribs * 0.1 + (1.0 - edged(shard)) * 0.1);
        color = mix(color * (1.0 - rim * 0.45), shell, inside);
        slope += tangential(axis, normal) * cos(dot(shard.offset, axis) * 70.0) * 0.35 * inside;
        rough = mix(rough, 0.45, inside);
    }
    let pebble_pixel = footprint * f32(PEBBLES);
    if pebble_pixel < 0.4 {
        let shown = 1.0 - smoothstep(0.2, 0.4, pebble_pixel);
        let stone = grains(spot, PEBBLES, vec3<f32>(0.0), 25u);
        let present = select(0.0, 1.0, roll(stone.id, 26u) < 0.025);
        let radius = 0.18 + roll(stone.id, 27u) * 0.12;
        let inside = (1.0 - smoothstep(radius - pebble_pixel, radius + pebble_pixel, stone.nearest)) * present * shown;
        let toward = tangential(stone.offset, normal);
        let lean = clamp(length(toward) / radius, 0.0, 1.0);
        let pebble = mix(vec3<f32>(0.32, 0.3, 0.28), vec3<f32>(0.55, 0.45, 0.36), roll(stone.id, 28u)) * (0.9 + (supple(at * 300.0) - 0.5) * 0.3);
        color = mix(color * (1.0 - 0.35 * (1.0 - smoothstep(radius, radius * 1.4, stone.nearest)) * present * shown), pebble, inside);
        slope += -toward / max(length(toward), 0.0001) * lean * lean * 1.6 * inside;
        rough = mix(rough, 0.5, inside);
    }

    let relief = weierstrass(spot, 4, 9, 0.8, 0.02, footprint, 31u);
    let damp = 1.0 - smoothstep(SEA_TOP + 0.3, SEA_TOP + 1.6, at.y);
    color = mix(color, color * vec3<f32>(0.6, 0.58, 0.54), damp);
    rough = mix(rough, rough * 0.6, damp);
    return Finish(color, 0.0, vec3<f32>(0.0), clamp(sqrt(rough * rough + relief.lost), 0.0, 1.0), 0.0, slope + relief.slope);
}

const CRUMBS: i32 = 120;
const CRUMB_MEAN: f32 = 0.86;
const SILT: i32 = 1300;
const ROOTLETS: i32 = 30;
const HUMUS: i32 = 90;

fn earthen(at: vec3<f32>, spot: Spot, normal: vec3<f32>, size: f32) -> Finish {
    let base = rocky(23.0, at, normal, size);
    let footprint = size / 16.0;
    var color = base.color;
    var slope = vec3<f32>(0.0);
    var rough = base.rough;

    let crumb_pixel = footprint * f32(CRUMBS);
    let crumb_seen = 1.0 - smoothstep(0.15, 0.45, crumb_pixel);
    if crumb_seen > 0.0 {
        let warp = (vec3<f32>(supple(at * 40.0), supple(at * 40.0 + 13.0), supple(at * 40.0 + 29.0)) - 0.5) * 1.6;
        let crumb = grains(spot, CRUMBS, warp, 31u);
        let radius = 0.45 + roll(crumb.id, 32u) * 0.3;
        let lump = 1.0 - smoothstep(radius * 0.7, radius + crumb_pixel, crumb.nearest);
        let parting = 1.0 - smoothstep(0.0, 0.07 + crumb_pixel, crumb.gap);
        let kind = roll(crumb.id, 33u);
        var tint = vec3<f32>(0.86 + roll(crumb.id, 34u) * 0.24);
        if kind < 0.1 {
            tint *= vec3<f32>(1.15, 1.1, 1.04);
        } else if kind < 0.3 {
            tint *= vec3<f32>(0.6, 0.56, 0.54);
        } else if kind < 0.35 {
            tint *= vec3<f32>(1.15, 0.86, 0.66);
        }
        let round = clamp(crumb.nearest / radius, 0.0, 1.0);
        tint *= 1.0 - 0.35 * round * round;
        let small_pixel = crumb_pixel * 2.7;
        let small = grains(spot, CRUMBS * 27 / 10, warp * 1.3, 39u);
        let small_radius = 0.4 + roll(small.id, 40u) * 0.3;
        let small_lump = (1.0 - smoothstep(small_radius * 0.6, small_radius + small_pixel, small.nearest)) * (1.0 - smoothstep(0.2, 0.5, small_pixel));
        let small_round = clamp(small.nearest / small_radius, 0.0, 1.0);
        let crevice = vec3<f32>(mix(0.3, (0.55 + roll(small.id, 42u) * 0.25) * (1.0 - 0.35 * small_round * small_round), small_lump));
        let body = lump * (1.0 - parting * 0.4);
        let shown = mix(crevice, tint, body);
        let toward = tangential(crumb.offset, normal);
        let small_toward = tangential(small.offset, normal);
        slope += (-toward / radius * 1.5 * body - small_toward / small_radius * 1.2 * small_lump * (1.0 - body)) * crumb_seen;
        color = color * mix(vec3<f32>(1.0), shown / CRUMB_MEAN, crumb_seen);
    }

    let silt_pixel = footprint * f32(SILT);
    let silt_seen = 1.0 - smoothstep(0.18, 0.55, silt_pixel);
    if silt_seen > 0.0 {
        let grain = grains(spot, SILT, vec3<f32>(0.0), 35u);
        let pick = roll(grain.id, 36u);
        let mica = pick < 0.015;
        let quartz = pick > 0.97;
        var tint = 0.93 + roll(grain.id, 37u) * 0.14;
        tint = select(tint, 1.15, quartz);
        tint = select(tint, 1.5, mica);
        let seam = 1.0 - smoothstep(0.0, 0.08 + silt_pixel, grain.gap);
        tint = mix(tint * (1.0 - 0.12 * pow(edged(grain), 3.0)), 0.82, seam);
        color = color * mix(1.0, tint, silt_seen);
        slope += (domed(grain, normal, 0.5) * (1.0 - seam) + select(vec3<f32>(0.0), (jitter(grain.id, 38u) - 0.5) * 0.8, mica)) * silt_seen;
        rough = mix(rough, select(rough, 0.18, mica), silt_seen);
    }

    let root = wave(lattice(spot, ROOTLETS, (vec3<f32>(supple(at * 90.0), supple(at * 90.0 + 7.0), supple(at * 90.0 + 19.0)) - 0.5) * 0.5), 41u);
    let reach = abs(root.x) / max(length(tangential(root.yzw, normal)), 0.25) / f32(ROOTLETS);
    let width = 0.00018 + supple(at * 23.0) * 0.00025;
    let rooted = smoothstep(0.55, 0.7, supple(at * 2.3 + 5.0)) * smoothstep(0.4, 0.6, supple(at * 35.0 + 11.0)) * smoothstep(0.4, 1.2, width / footprint);
    let strand = (1.0 - smoothstep(width - footprint * 0.5, width + footprint * 0.5, reach)) * rooted;
    let shadow = (1.0 - smoothstep(width, width * 2.5 + footprint, reach)) * rooted;
    color = mix(color * (1.0 - shadow * 0.35), base.color * vec3<f32>(1.4, 1.25, 1.0) * (0.85 + 0.3 * supple(at * 400.0)), strand);
    slope += tangential(root.yzw, normal) * sign(root.x) * -0.4 * strand / max(length(root.yzw), 0.1);

    let humus_pixel = footprint * f32(HUMUS);
    if humus_pixel < 0.4 {
        let bit = grains(spot, HUMUS, vec3<f32>(0.0), 45u);
        let present = select(0.0, 1.0, roll(bit.id, 46u) < 0.06);
        let inside = (1.0 - smoothstep(0.22 - humus_pixel, 0.22 + humus_pixel, bit.nearest)) * present * (1.0 - smoothstep(0.2, 0.4, humus_pixel));
        color = mix(color, vec3<f32>(0.07, 0.05, 0.035), inside);
    }

    let relief = weierstrass(spot, 8, 9, 0.7, 0.03, footprint, 51u);
    return Finish(color, base.height, base.glow, clamp(sqrt(rough * rough + relief.lost), 0.0, 1.0), 0.0, slope + relief.slope);
}

const THATCH: i32 = 40;
const LAYERS: i32 = 5;
const SLOTS: i32 = 3;
const STRAW: vec3<f32> = vec3<f32>(0.34, 0.28, 0.15);

struct Thatch {
    color: vec3<f32>,
    slope: vec2<f32>,
    rough: f32,
}

fn thatch(near: vec2<f32>, anchor: vec2<i32>, at: vec2<f32>, comb: f32, spread: f32, footprint: f32, ground: vec3<f32>) -> Thatch {
    let pixel = footprint * f32(THATCH);
    var hit = -1;
    var best = -1.0;
    var lengthwise = 0.0;
    var across = 0.0;
    var heading = vec2<f32>(1.0, 0.0);
    var blade = vec3<i32>(0);
    for (var layer = 0; layer < LAYERS; layer++) {
        if hit < 0 {
            let local = near * f32(THATCH) + vec2<f32>(f32(layer) * 0.37, f32(layer) * 0.61);
            let base = floor(local);
            let cell = anchor * THATCH + vec2<i32>(base);
            let inside = local - base;
            for (var x = -1; x <= 1; x++) {
                for (var y = -1; y <= 1; y++) {
                    for (var slot = 0; slot < SLOTS; slot++) {
                        let id = vec3<i32>(cell + vec2<i32>(x, y), layer * SLOTS + slot);
                        let r = jitter(id, 61u);
                        let more = jitter(id, 62u);
                        let angle = comb + (r.z - 0.5) * spread;
                        let toward = vec2<f32>(cos(angle), sin(angle));
                        let reach = 0.6 + more.x * 0.4;
                        let q = inside - vec2<f32>(f32(x), f32(y)) - r.xy;
                        let t = dot(q, toward) / reach;
                        let side = dot(q, vec2<f32>(-toward.y, toward.x));
                        let half = (0.06 + more.y * 0.045) * pow(min((1.0 - t) * 3.5, 1.0), 0.7);
                        let covered = t > 0.0 && t < 1.0 && abs(side) < half;
                        if covered && more.z > best {
                            best = more.z;
                            hit = layer;
                            lengthwise = t;
                            across = side / max(half, 0.0001);
                            heading = toward;
                            blade = id;
                        }
                    }
                }
            }
        }
    }
    var result = Thatch(ground, vec2<f32>(0.0), 0.95);
    if hit >= 0 {
        let lushness = lush(at);
        let depth = pow(0.74, f32(hit));
        var color = blade_colour(lushness, 0.3 + 0.6 * lengthwise) * (0.85 + 0.3 * roll(blade, 64u));
        color = mix(color, STRAW * (0.9 + 0.3 * roll(blade, 66u)), smoothstep(0.8, 1.0, lengthwise) * select(0.0, 0.6, roll(blade, 65u) < 0.15));
        color = select(color, STRAW * (0.5 + 0.4 * roll(blade, 67u)), roll(blade, 63u) < 0.03);
        color *= 1.0 + 0.16 * (1.0 - smoothstep(0.0, 0.14, abs(across)));
        color *= 1.0 + 0.06 * sin(across * 17.0);
        color *= 1.0 - 0.3 * smoothstep(0.6, 1.0, abs(across));
        let perp = vec2<f32>(-heading.y, heading.x);
        result = Thatch(color * depth, -perp * across * 0.8 + heading * (roll(blade, 68u) - 0.3) * 0.9, 0.55);
    }
    return result;
}

const TURF: f32 = 34.0;
const LEAVES: f32 = 37.0;

fn turfed(kind: f32, at: vec3<f32>, spot: Spot, normal: vec3<f32>, size: f32, time: f32) -> Finish {
    let soil = earthen(at, spot, normal, size);
    let ragged = (supple(at * 2.3) - 0.5) * 0.3 + (supple(at * 7.0) - 0.5) * 0.12;
    let inside = at - normal * 0.02;
    let block = floor(inside);
    let dangle = supple(vec3<f32>(inside.x * 5.0, block.y * 7.0, inside.z * 5.0) + 41.0) * 0.6 + supple(vec3<f32>(inside.x * 13.0, block.y, inside.z * 13.0)) * 0.4;
    let hang = (2.0 + dangle * 4.0) / 16.0;
    let fringe = abs(normal.y) < 0.6 && block.y + 1.0 - at.y < hang;
    let covered = max(smoothstep(0.34, 0.5, normal.y + ragged), select(0.0, 1.0, fringe));
    let pick = smoothstep(0.0, 1.0, (supple(inside * 14.0 + 3.0) - 0.5) * 2.6 + 0.5);
    let cloud = supple(inside * 3.0) * 0.6 + supple(at * 0.9 + 5.0) * 0.4;
    var cover = vec3<f32>(0.0);
    var slope = vec3<f32>(0.0);
    var rough = 0.95;
    if kind < TURF + 0.5 && surface.turf.z > 0.5 {
        let footprint = size / 16.0;
        let seen = 1.0 - smoothstep(0.06, 0.16, footprint * f32(THATCH));
        var tuft = 0.0;
        var hue = 0.0;
        var frequency = THATCH;
        for (var octave = 0; octave < 4; octave++) {
            let n = wave(lattice(spot, frequency, vec3<f32>(f32(octave) * 0.31)), 71u + u32(octave)).x;
            let fade = 1.0 - smoothstep(0.25, 0.7, footprint * f32(frequency));
            tuft += n * fade * mix(1.0, 0.75, f32(octave) / 3.0);
            hue += n * fade * f32(octave) / 3.0;
            frequency = max(frequency / 3, 1);
        }
        let meadow = fuzz(at.xz, time, 9.0);
        let shifted = mix(meadow, meadow * vec3<f32>(1.25, 1.0, 0.7), clamp(hue * 0.8, -0.5, 0.5));
        cover = shifted * (1.0 + tuft * 0.5) * select(1.0, 0.82, fringe && normal.y < 0.6);
        if seen > 0.0 {
            let ground = mix(vec3<f32>(0.03, 0.035, 0.018), STRAW * 0.35, smoothstep(0.55, 0.75, supple(at * 60.0)));
            let top = normal.y > 0.5;
            let east = abs(normal.x) > abs(normal.z);
            let near = select(vec2<f32>(select(spot.near.x, spot.near.z, east), -spot.near.y), spot.near.xz, top);
            let anchor = select(vec2<i32>(select(spot.anchor.x, spot.anchor.z, east), -spot.anchor.y), spot.anchor.xz, top);
            let comb = select(1.5708, supple(vec3<f32>(at.x * 0.4, 0.0, at.z * 0.4)) * 12.0, top);
            let mat = thatch(near, anchor, at.xz, comb, select(0.8, 2.6, top), footprint, ground);
            let flat = select(select(vec3<f32>(mat.slope.x, -mat.slope.y, 0.0), vec3<f32>(0.0, -mat.slope.y, mat.slope.x), east), vec3<f32>(mat.slope.x, 0.0, mat.slope.y), top);
            cover = mix(cover, mat.color, seen);
            slope = flat * seen;
            rough = mix(rough, mat.rough, seen);
        }
    } else if kind < TURF + 0.5 {
        cover = lawn(at.xz, time, distance(at.xz, view.world_position.xz));
        cover = cover * select(1.0, 0.82, fringe && normal.y < 0.6);
    } else if kind < TURF + 1.5 {
        let snow = array<vec3<f32>, 4>(vec3<f32>(0.7, 0.75, 0.82), vec3<f32>(0.82, 0.86, 0.92), vec3<f32>(0.92, 0.95, 0.99), vec3<f32>(1.0, 1.0, 1.0));
        cover = ramped(snow, 0.55 + (cloud - 0.5) * 0.6 + (pick - 0.5) * 0.1);
    } else {
        let spores = array<vec3<f32>, 4>(vec3<f32>(0.32, 0.26, 0.35), vec3<f32>(0.4, 0.33, 0.43), vec3<f32>(0.47, 0.39, 0.49), vec3<f32>(0.58, 0.5, 0.58));
        cover = ramped(spores, 0.45 + (cloud - 0.5) * 0.9 + (pick - 0.5) * 0.3);
        cover = mix(cover, vec3<f32>(0.72, 0.66, 0.7), (1.0 - below(pick, 0.94)) * (1.0 - smoothstep(0.8, 1.2, size)));
    }
    return Finish(mix(soil.color, cover, covered), soil.height * (1.0 - covered), vec3<f32>(0.0), mix(soil.rough, rough, covered), 0.0, mix(soil.slope, slope, covered));
}

fn leafy(kind: f32, at: vec3<f32>, normal: vec3<f32>, size: f32) -> vec4<f32> {
    var palette = array<vec3<f32>, 4>(vec3<f32>(0.11, 0.27, 0.09), vec3<f32>(0.16, 0.36, 0.12), vec3<f32>(0.22, 0.45, 0.16), vec3<f32>(0.3, 0.55, 0.21));
    var stretch = vec3<f32>(1.0);
    if kind > LEAVES + 0.5 && kind < LEAVES + 1.5 {
        palette = array<vec3<f32>, 4>(vec3<f32>(0.24, 0.38, 0.12), vec3<f32>(0.33, 0.49, 0.17), vec3<f32>(0.42, 0.6, 0.22), vec3<f32>(0.52, 0.69, 0.29));
    } else if kind > LEAVES + 1.5 && kind < LEAVES + 2.5 {
        palette = array<vec3<f32>, 4>(vec3<f32>(0.06, 0.17, 0.12), vec3<f32>(0.09, 0.23, 0.16), vec3<f32>(0.12, 0.29, 0.2), vec3<f32>(0.17, 0.36, 0.25));
        stretch = vec3<f32>(1.8, 0.6, 1.8);
    } else if kind > LEAVES + 2.5 {
        palette = array<vec3<f32>, 4>(vec3<f32>(0.15, 0.36, 0.08), vec3<f32>(0.22, 0.47, 0.11), vec3<f32>(0.3, 0.58, 0.15), vec3<f32>(0.4, 0.68, 0.22));
        stretch = vec3<f32>(0.5, 1.0, 0.5);
    }
    let fine = at * 16.0 - normal * 0.02;
    let point = fine;
    let leaf = pebbles(point / 3.0 * stretch, false);
    let clump = supple(point / 16.0 * 2.5);
    let lit = dot(normalize(point / 3.0 * stretch - leaf.centre + vec3<f32>(0.0, 0.001, 0.0)), normalize(vec3<f32>(-0.4, 0.8, -0.4)));
    let tone = texel_hash(leaf.id, 17u);
    let body = leaf.near < 0.4 && leaf.edge > 0.06;
    let open = !body && clump < 0.66 && size < 1.2;
    let color = ramped(palette, 0.05 + tone * 0.4 + lit * 0.22 * smoothstep(0.0, 0.35, leaf.near) + (clump - 0.5) * 0.3);
    let filled = smoothstep(0.42, 0.36, leaf.near) * smoothstep(0.04, 0.08, leaf.edge);
    let shown = mix(palette[0] * 0.55, color, filled);
    return vec4<f32>(mix(shown, ramped(palette, 0.45), smoothstep(0.8, 2.0, size)), select(1.0, 0.0, open));
}

const MASONRY: f32 = 41.0;
const CARVED: f32 = 45.0;
const PLANKS: f32 = 46.0;
const BARK: f32 = 47.0;
const MOSS: array<vec3<f32>, 4> = array<vec3<f32>, 4>(vec3<f32>(0.1, 0.2, 0.08), vec3<f32>(0.17, 0.3, 0.11), vec3<f32>(0.24, 0.4, 0.14), vec3<f32>(0.33, 0.5, 0.2));
const RUNE: vec3<f32> = vec3<f32>(0.35, 0.95, 0.8);

fn facing(at: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let axis = abs(normal);
    if axis.y > axis.x && axis.y > axis.z {
        return vec3<f32>(at.x, at.z, at.y);
    } else if axis.x > axis.z {
        return vec3<f32>(at.z, at.y, at.x);
    }
    return at;
}

fn snapped(at: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    return at - normal * 0.01;
}

fn masonry(kind: f32, at: vec3<f32>, normal: vec3<f32>, size: f32) -> Finish {
    let face = facing(at - normal * 0.01, normal) * 16.0;
    let flat = face;
    let spot = snapped(at, normal);
    let ancient = kind < MASONRY + 2.5;
    let span = select(8.0, 16.0, ancient);
    let wall = i32(floor(face.z / 16.0));
    let course = floor(flat.y / 8.0);
    let jitter = select(0.0, floor(texel_hash(vec3<i32>(i32(course), wall, 3), 40u) * 4.0), ancient);
    let run = flat.x + course * span * 0.5 + jitter;
    let brick = floor(run / span);
    let u = run - brick * span;
    let v = flat.y - course * 8.0;
    let id = vec3<i32>(i32(brick), i32(course), wall);
    let wear = select(0.3, 0.55, ancient);
    let wobble = (supple(face * 0.55 + 5.0) - 0.5) * 2.2 * wear + (supple(face * 1.7 + 31.0) - 0.5) * 0.6 * wear;
    let edge = min(min(u - 1.0, span - u), min(v - 1.0, 8.0 - v));
    let mortar = below(edge + wobble, 0.0);
    let top = below(8.0 - v, 1.0) * (1.0 - mortar);
    let bottom = below(v - 1.0, 1.0) * (1.0 - mortar);
    let speckle = smoothstep(0.0, 1.0, (supple(face * 0.9 + 13.0) - 0.5) * 2.4 + 0.5);
    let pick = speckle;
    let cloud = supple(spot * 1.4 + 3.0) * 0.6 + supple(spot * 4.0 + 9.0) * 0.4;
    let tone = texel_hash(id, 42u);
    let base = select(vec3<f32>(0.5, 0.5, 0.52), vec3<f32>(0.37, 0.4, 0.41), ancient);
    let joint = select(vec3<f32>(0.27, 0.27, 0.27), vec3<f32>(0.12, 0.13, 0.14), ancient);
    let palette = shades(base);
    let stone = ramped(palette, 0.45 + (tone - 0.5) * 0.4 + (cloud - 0.5) * 0.55 + (pick - 0.5) * 0.18 + top * 0.18 - bottom * 0.14);
    var color = mix(stone, joint * (0.85 + pick * 0.3), mortar);
    var height = (1.0 - mortar) * 0.8 + (cloud - 0.5) * 0.3 + top * 0.2;
    if kind > MASONRY + 0.5 && kind < MASONRY + 1.5 {
        let growth = supple(spot * 0.9 + 11.0) * 0.55 + supple(spot * 2.6 + 3.0) * 0.25 + clamp(normal.y, 0.0, 1.0) * 0.35 + (1.0 - fract(spot.y)) * 0.12 + mortar * 0.2;
        let moss = 1.0 - below(growth, 0.62);
        let fuzz = supple(spot * 24.0);
        color = mix(color, ramped(MOSS, 0.2 + fuzz * 0.6 + (growth - 0.62) * 1.5), moss);
        height = mix(height, 1.0 + fuzz * 0.3, moss);
    } else if kind > MASONRY + 1.5 && kind < MASONRY + 2.5 {
        let split = pebbles(spot * 16.0 / 7.0, false);
        let crack = below(split.edge, 0.07) * select(0.0, 1.0, supple(spot * 1.1 + 21.0) > 0.42);
        color = mix(color, vec3<f32>(0.07, 0.07, 0.08), crack);
        height = height - crack * 1.2;
    }
    let far = mix(ramped(palette, 0.45 + (tone - 0.5) * 0.3), joint, 0.15);
    let fade = smoothstep(1.0, 2.5, size);
    return Finish(mix(color, far, fade), height * (1.0 - fade), vec3<f32>(0.0), 0.9, 0.0, vec3<f32>(0.0));
}

fn stroke(point: vec2<f32>, start: vec2<f32>, end: vec2<f32>) -> f32 {
    let line = end - start;
    let along = clamp(dot(point - start, line) / dot(line, line), 0.0, 1.0);
    return length(point - start - line * along);
}

fn rune(texel: vec2<f32>, seed: vec3<i32>) -> f32 {
    var spots = array<vec2<f32>, 9>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0), vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, 2.0), vec2<f32>(1.0, 2.0), vec2<f32>(2.0, 2.0));
    var lines = array<vec2<u32>, 20>(
        vec2<u32>(0u, 1u), vec2<u32>(1u, 2u), vec2<u32>(3u, 4u), vec2<u32>(4u, 5u), vec2<u32>(6u, 7u), vec2<u32>(7u, 8u),
        vec2<u32>(0u, 4u), vec2<u32>(1u, 3u), vec2<u32>(1u, 5u), vec2<u32>(2u, 4u),
        vec2<u32>(3u, 7u), vec2<u32>(4u, 6u), vec2<u32>(4u, 8u), vec2<u32>(5u, 7u),
        vec2<u32>(0u, 3u), vec2<u32>(3u, 6u), vec2<u32>(2u, 5u), vec2<u32>(5u, 8u), vec2<u32>(1u, 4u), vec2<u32>(4u, 7u));
    let stem = u32(floor(texel_hash(seed, 60u) * 3.0));
    let scale = vec2<f32>(2.0, 1.6);
    var near = stroke(texel, vec2<f32>(f32(stem), 0.0) * scale, vec2<f32>(f32(stem), 2.0) * scale);
    for (var index = 0u; index < 20u; index++) {
        if texel_hash(seed, 61u + index) < 0.2 {
            let pair = lines[index];
            near = min(near, stroke(texel, spots[pair.x] * scale, spots[pair.y] * scale));
        }
    }
    return near;
}

fn carved(at: vec3<f32>, normal: vec3<f32>, size: f32, time: f32) -> Finish {
    let face = facing(at - normal * 0.01, normal) * 16.0;
    let flat = face;
    let inner = flat.xy - floor(face.xy / 16.0) * 16.0;
    let frame = inner.x < 2.0 || inner.x > 14.0 || inner.y < 1.0 || inner.y > 15.0;
    if frame {
        return masonry(MASONRY, at, normal, size);
    }
    let spot = snapped(at, normal);
    let cloud = supple(spot * 1.4 + 3.0) * 0.6 + supple(spot * 4.0 + 9.0) * 0.4;
    let pick = supple(face * 0.9 + 13.0);
    let inset = min(min(inner.x - 2.0, 14.0 - inner.x), min(inner.y - 1.0, 15.0 - inner.y));
    let rim = below(inset, 1.0);
    let panel = ramped(shades(vec3<f32>(0.37, 0.4, 0.41)), 0.4 + (cloud - 0.5) * 0.5 + (pick - 0.5) * 0.12) * (0.8 - rim * 0.12);
    let block = vec3<i32>(floor(at - normal * 0.5));
    let side = vec3<i32>(round(normal * 2.0));
    let row = floor((inner.y - 1.0) / (14.0 / 3.0));
    let seed = block * 7 + side + vec3<i32>(0, i32(row) * 31, 0);
    let corner = vec2<f32>(4.0, 1.0 + row * 14.0 / 3.0 + 0.9);
    let near = rune(inner - corner - vec2<f32>(0.0, 0.0), seed);
    let carving = 1.0 - smoothstep(0.3, 0.5, near) * (1.0 - smoothstep(1.0, 2.5, size));
    let pulse = 0.75 + 0.25 * sin(time * 1.4 + f32(seed.x + seed.z) * 0.37);
    let color = mix(panel, RUNE, carving);
    return Finish(color, -0.6 - carving * 0.6 - rim * 0.3, RUNE * carving * 0.55 * pulse, 0.85, 0.0, vec3<f32>(0.0));
}

fn planked(at: vec3<f32>, normal: vec3<f32>, size: f32) -> Finish {
    let face = facing(at - normal * 0.01, normal) * 16.0;
    let depth = i32(floor(face.z / 16.0));
    let board = floor(face.y / 4.0);
    let row = face.y - board * 4.0;
    let course = vec3<i32>(i32(board), depth, 7);
    let span = 24.0 + floor(texel_hash(course, 57u) * 3.0) * 8.0;
    let run = face.x + texel_hash(course, 50u) * span;
    let piece = floor(run / span);
    let along = run - piece * span;
    let id = vec3<i32>(i32(board), i32(piece), depth);
    let tone = 0.8 + texel_hash(id, 51u) * 0.32;
    let hue = texel_hash(id, 52u) - 0.5;
    let seed = texel_hash(id, 53u) * 91.0;
    let knotty = texel_hash(id, 54u) < 0.35;
    let knot_at = vec2<f32>(4.0 + texel_hash(id, 55u) * (span - 8.0), 0.8 + texel_hash(id, 56u) * 2.4);
    let toward_knot = vec2<f32>((along - knot_at.x) / 1.6, row - knot_at.y);
    let knot_reach = select(99.0, length(toward_knot), knotty);
    let swirl = 2.6 / (1.0 + knot_reach * knot_reach * 0.5);
    let across = row - 2.0 + (texel_hash(id, 58u) - 0.5) * 7.0;
    let tilt = (texel_hash(id, 60u) - 0.5) * 0.35;
    let cut = 2.0 + texel_hash(id, 59u) * 9.0 + (along - span * 0.5) * tilt;
    let wobble = supple(vec3<f32>(along * 0.05 + seed, row * 0.25, 3.0)) * 2.4 + supple(vec3<f32>(along * 0.22 + seed, row * 0.6, 9.0)) * 0.6;
    let radius = sqrt(across * across + cut * cut) + wobble + swirl;
    let ring = fract(radius * 0.55);
    let fine = 1.0 - smoothstep(0.35, 0.9, size);
    let late = (smoothstep(0.6, 0.78, ring) * (1.0 - smoothstep(0.9, 1.0, ring))) * fine;
    let fibre = supple(vec3<f32>(along * 0.04 + seed, row * 2.8, 17.0));
    let pore = supple(vec3<f32>(along * 0.7 + seed, row * 5.0, 29.0));
    let edge = min(row, 4.0 - row);
    let butt = min(along, span - along);
    let gap = 1.0 - smoothstep(0.12, 0.3, edge);
    let joint = 1.0 - smoothstep(0.1, 0.28, butt);
    let bevel = smoothstep(0.1, 0.75, edge) * smoothstep(0.1, 0.6, butt);
    let core = 1.0 - smoothstep(0.35, 0.75, knot_reach);
    let warm = vec3<f32>(1.0 + hue * 0.12, 1.0, 1.0 - hue * 0.16);
    let light = vec3<f32>(0.74, 0.55, 0.33);
    let dark = vec3<f32>(0.47, 0.31, 0.17);
    var color = mix(light, dark, late * 0.6 + (fibre - 0.5) * 0.3 + (pore - 0.5) * 0.12 * fine);
    color = mix(color, vec3<f32>(0.32, 0.2, 0.1), core * 0.85);
    color = color * warm * tone;
    let up = smoothstep(0.6, 0.9, normal.y);
    let scuffed = smoothstep(0.55, 0.85, supple(vec3<f32>(face.x * 0.05, face.y * 0.05, f32(depth) * 2.3))) * up;
    let scratch = below(abs(supple(vec3<f32>(face.x * 0.3, face.y * 0.3, 41.0)) - 0.5), 0.02) * up * fine;
    color = mix(color, color * 1.18 + vec3<f32>(0.03, 0.02, 0.0), scuffed * 0.35 + scratch * 0.25);
    color = color * (0.72 + 0.28 * bevel);
    color = mix(color, vec3<f32>(0.1, 0.065, 0.04), max(gap, joint));
    let nails = select(0.0, 1.0, texel_hash(id, 61u) < 0.8);
    let nail_x = select(span - 1.4, 1.4, along < span * 0.5);
    let nail_spot = vec2<f32>(along - nail_x, min(abs(row - 1.1), abs(row - 2.9)));
    let nail_reach = length(nail_spot);
    let nail = (1.0 - smoothstep(0.3, 0.45, nail_reach)) * nails * fine;
    let rust = (1.0 - smoothstep(0.45, 1.1, nail_reach)) * nails * fine * (1.0 - nail);
    color = mix(color, color * vec3<f32>(0.62, 0.45, 0.32), rust * 0.6);
    color = mix(color, vec3<f32>(0.2, 0.2, 0.21) + vec3<f32>(0.12) * (1.0 - nail_reach / 0.45), nail);
    let far = mix(light, dark, 0.28) * 0.84 * warm * tone;
    let fade = smoothstep(1.2, 3.0, size);
    let height = bevel * 0.9 - max(gap, joint) * 0.6 + (fibre - 0.5) * 0.25 * fine - late * 0.12 - core * 0.3 + nail * 0.2;
    let rough = mix(0.58 + late * 0.12 - scuffed * 0.12, 0.95, max(gap, joint));
    return Finish(mix(color, far, fade), height * (1.0 - fade), vec3<f32>(0.0), mix(rough, 0.7, fade), nail * 0.8, vec3<f32>(0.0));
}

fn barked(kind: f32, at: vec3<f32>, normal: vec3<f32>, size: f32) -> Finish {
    let wood = kind - BARK;
    let middle = floor(at.xz) + 0.5;
    var color = vec3<f32>(0.0);
    var height = 0.0;
    var far = vec3<f32>(0.0);
    if abs(normal.y) > 0.7 {
        let spot = at.xz;
        let toward = spot - middle;
        let reach = length(toward) / 0.47;
        let wobble = supple(vec3<f32>(spot.x * 5.0, floor(at.y) * 3.0, spot.y * 5.0)) - 0.5;
        let band = fract((reach + wobble * 0.08) * 5.5);
        let late = below(band, 0.38);
        var light = vec3<f32>(0.66, 0.52, 0.33);
        var dark = vec3<f32>(0.57, 0.44, 0.27);
        var rind = vec3<f32>(0.36, 0.26, 0.16);
        if wood > 0.5 && wood < 1.5 {
            light = vec3<f32>(0.84, 0.76, 0.6);
            dark = vec3<f32>(0.75, 0.66, 0.5);
            rind = vec3<f32>(0.78, 0.77, 0.72);
        } else if wood > 1.5 && wood < 2.5 {
            light = vec3<f32>(0.7, 0.55, 0.36);
            dark = vec3<f32>(0.6, 0.45, 0.29);
            rind = vec3<f32>(0.24, 0.16, 0.1);
        } else if wood > 2.5 {
            light = vec3<f32>(0.74, 0.64, 0.46);
            dark = vec3<f32>(0.65, 0.55, 0.38);
            rind = vec3<f32>(0.5, 0.4, 0.26);
        }
        let cut = vec3<i32>(floor(at));
        let split = texel_hash(cut, 70u) * 6.2832;
        let angle = atan2(toward.y, toward.x);
        let apart = abs(fract((angle - split) / 6.2832 + 0.5) - 0.5) * 6.2832 * reach * 0.47 * 16.0;
        let crack = select(0.0, below(apart, 0.35) * select(0.0, 1.0, reach > 0.25 && reach < 0.8), texel_hash(cut, 71u) < 0.6);
        let fleck = supple(vec3<f32>(spot.x, 0.0, spot.y) * 40.0);
        color = mix(light, dark, late) * (0.95 + fleck * 0.1);
        color = mix(color, dark * 0.6, below(reach, 0.08));
        color = mix(color, dark * 0.55, crack);
        let bark = 1.0 - below(reach, 0.86);
        color = mix(color, rind * (0.85 + fleck * 0.3), bark);
        height = -late * 0.2 - crack + bark * 0.5;
        far = mix(mix(light, dark, 0.4), rind, 0.25);
    } else {
        let offset = at.xz - middle;
        let around = (atan2(offset.y, offset.x) / 6.2832 + 0.5) * 48.0;
        let tall = at.y * 16.0;
        let a = around;
        let h = tall;
        let angle = (a / 48.0 - 0.5) * 6.2832;
        let p = vec3<f32>(middle.x + cos(angle) * 0.47, h / 16.0, middle.y + sin(angle) * 0.47);
        let texel = vec3<i32>(i32(floor(a)), i32(floor(h)), i32(floor(middle.x) * 31.0 + floor(middle.y)));
        let pick = supple(p * 30.0);
        if wood < 0.5 {
            let palette = array<vec3<f32>, 4>(vec3<f32>(0.22, 0.15, 0.09), vec3<f32>(0.3, 0.21, 0.13), vec3<f32>(0.38, 0.27, 0.17), vec3<f32>(0.45, 0.33, 0.21));
            let ridge = supple(vec3<f32>(p.x * 7.0, p.y * 1.1, p.z * 7.0)) * 0.7 + supple(vec3<f32>(p.x * 16.0, p.y * 2.5, p.z * 16.0)) * 0.3;
            let furrow = below(ridge, 0.42);
            color = ramped(palette, 0.55 + (ridge - 0.5) * 1.2 + (pick - 0.5) * 0.15 - furrow * 0.3);
            height = ridge - furrow;
            far = palette[1];
        } else if wood < 1.5 {
            let palette = array<vec3<f32>, 4>(vec3<f32>(0.6, 0.59, 0.55), vec3<f32>(0.74, 0.73, 0.69), vec3<f32>(0.84, 0.83, 0.79), vec3<f32>(0.92, 0.91, 0.88));
            let cloud = supple(p * 3.0 + 5.0);
            let lenticel = 1.0 - below(supple(vec3<f32>(p.x * 5.0, p.y * 24.0, p.z * 5.0) + 3.0), 0.76);
            let scar = 1.0 - below(supple(p * 1.6 + 17.0), 0.74);
            color = ramped(palette, 0.55 + (cloud - 0.5) * 0.6 + (pick - 0.5) * 0.2);
            color = mix(color, vec3<f32>(0.13, 0.12, 0.11), max(lenticel, scar));
            height = -max(lenticel, scar) * 0.6 + cloud * 0.2;
            far = mix(palette[2], vec3<f32>(0.13, 0.12, 0.11), 0.12);
        } else if wood < 2.5 {
            let palette = array<vec3<f32>, 4>(vec3<f32>(0.18, 0.11, 0.07), vec3<f32>(0.25, 0.16, 0.1), vec3<f32>(0.33, 0.22, 0.13), vec3<f32>(0.4, 0.28, 0.17));
            let plate = pebbles(vec3<f32>(p.x * 7.0, p.y * 4.5, p.z * 7.0), false);
            let seam = below(plate.edge, 0.08);
            let lit = clamp(dot(normalize(vec3<f32>(p.x * 7.0, p.y * 4.5, p.z * 7.0) - plate.centre + vec3<f32>(0.0, 0.001, 0.0)), vec3<f32>(0.0, 1.0, 0.0)), -1.0, 1.0);
            color = ramped(palette, 0.3 + texel_hash(plate.id, 74u) * 0.45 + lit * 0.15 + (pick - 0.5) * 0.12) * (1.0 - seam * 0.5);
            height = 1.0 - seam - plate.near * 0.4;
            far = palette[1];
        } else {
            let palette = array<vec3<f32>, 4>(vec3<f32>(0.38, 0.29, 0.18), vec3<f32>(0.48, 0.38, 0.24), vec3<f32>(0.57, 0.46, 0.3), vec3<f32>(0.66, 0.55, 0.37));
            let ring = fract(p.y * 16.0 / 3.0 + (supple(p * 3.0) - 0.5) * 0.6);
            let scar = below(ring, 0.3);
            let fibre = supple(vec3<f32>(p.x * 22.0, p.y * 2.5, p.z * 22.0));
            color = ramped(palette, 0.55 + (fibre - 0.5) * 0.5 + (pick - 0.5) * 0.15 - scar * 0.35 + (1.0 - below(ring, 0.85)) * 0.12);
            height = ring - scar;
            far = palette[1];
        }
    }
    let near = 1.0 - smoothstep(0.08, 0.3, size);
    let nearest = 1.0 - smoothstep(0.03, 0.12, size);
    let fibre = (supple(vec3<f32>(at.x * 70.0, at.y * 14.0, at.z * 70.0) + 5.0) - 0.5) * near;
    let speck = (supple(at * 190.0 + 23.0) - 0.5) * nearest;
    let flake = smoothstep(0.66, 0.74, supple(at * 110.0 + 41.0)) * nearest;
    let rugged = color * (1.0 + fibre * 0.35 + speck * 0.2 - flake * 0.15);
    let fade = smoothstep(1.0, 2.5, size);
    return Finish(mix(rugged, far, fade * 0.7), height * (1.0 - fade) + fibre * 0.08 + speck * 0.025 - flake * 0.02, vec3<f32>(0.0), 0.9 + fibre * 0.1, 0.0, vec3<f32>(0.0));
}

fn crafted(kind: f32, at: vec3<f32>, normal: vec3<f32>, size: f32, time: f32) -> Finish {
    if kind < CARVED - 0.5 {
        return masonry(kind, at, normal, size);
    } else if kind < CARVED + 0.5 {
        return carved(at, normal, size, time);
    } else if kind < PLANKS + 0.5 {
        return planked(at, normal, size);
    }
    return barked(kind, at, normal, size);
}

fn finish(kind: f32, mask: f32, at: vec3<f32>, spot: Spot, normal: vec3<f32>, eye: vec3<f32>, time: f32, size: f32) -> Finish {
    let axis = abs(normal);
    var first = vec3<f32>(1.0, 0.0, 0.0);
    var second = vec3<f32>(0.0, 1.0, 0.0);
    if axis.x > axis.y && axis.x > axis.z {
        first = vec3<f32>(0.0, 0.0, 1.0);
    } else if axis.y > axis.z {
        second = vec3<f32>(0.0, 0.0, 1.0);
    }
    let dominant = select(select(vec3<f32>(0.0, 0.0, sign(normal.z)), vec3<f32>(0.0, sign(normal.y), 0.0), axis.y > axis.z && axis.y >= axis.x), vec3<f32>(sign(normal.x), 0.0, 0.0), axis.x > axis.y && axis.x > axis.z);
    let cell = floor(at - dominant * 0.5);
    let plane = vec2<f32>(dot(at, first), dot(at, second));
    let along = vec2<f32>(dot(at - cell, first), dot(at - cell, second));
    let rim = border(mask, clamp(along, vec2<f32>(0.0), vec2<f32>(1.0)));
    var out = Finish(vec3<f32>(0.5), 0.0, vec3<f32>(0.0), 0.8, 0.0, vec3<f32>(0.0));
    if kind < 1.5 {
        let dense = select(0.0, 1.0, kind > 0.5);
        let brushed = lumpy(vec3<f32>(plane.x * 40.0, plane.y * 1.5, 0.0)) * 0.08;
        let panel = floor(plane / 3.0 + 0.5 * floor(plane.yx / 3.0));
        let tone = hashed(vec3<f32>(panel, 1.0)).x * 0.06;
        let seam_at = abs(fract(plane / 3.0 + 0.5 * floor(plane.yx / 3.0)) - 0.5);
        let seam = 1.0 - smoothstep(0.004, 0.012, 0.5 - max(seam_at.x, seam_at.y));
        let frame = 1.0 - smoothstep(0.1, 0.13, rim);
        let groove = smoothstep(0.12, 0.14, rim) * (1.0 - smoothstep(0.15, 0.18, rim));
        let bevel = smoothstep(0.0, 0.1, rim) * frame;
        var color = vec3<f32>(0.16, 0.21, 0.23) + vec3<f32>(brushed + tone);
        color = mix(color, vec3<f32>(0.42, 0.5, 0.52), bevel * 0.8);
        color = color * (1.0 - groove * 0.75) * (1.0 - seam * 0.5);
        let circuit = traces(plane, dense, time) * (1.0 - frame) * smoothstep(0.18, 0.3, rim);
        let edge_glow = groove * select(0.25, 0.9, dense > 0.5) * (0.6 + 0.4 * sin(time * 1.3 + plane.x + plane.y));
        out.color = mix(color, AETHER * 0.5, circuit);
        out.glow = AETHER * (circuit * select(0.6, 1.4, dense > 0.5) + edge_glow);
        out.height = frame * 0.6 - groove * 0.8 - seam * 0.3 + brushed;
        out.rough = 0.32 + brushed;
        out.metal = 0.85;
    } else if kind < 2.5 {
        let warp = layered(at * 0.18) * 7.0;
        let band = sin((at.y + warp) * 1.25) * 0.5 + 0.5;
        let cracks = cellular(at * 1.1 + layered(at * 0.6) * 0.6);
        let crack = 1.0 - smoothstep(0.02, 0.08, cracks.edge);
        let grain = layered(at * 3.0);
        var color = mix(vec3<f32>(0.05, 0.06, 0.1), vec3<f32>(0.13, 0.15, 0.23), band * 0.7 + grain * 0.3);
        let worn = 1.0 - smoothstep(0.0, 0.22, rim);
        color = mix(color, vec3<f32>(0.24, 0.27, 0.34), worn * 0.6);
        color = color * (1.0 - crack * 0.7);
        let lichen = step(0.82, hashed(cracks.id).x) * crack;
        out.color = color;
        out.glow = vec3<f32>(0.2, 0.8, 0.9) * lichen * (0.6 + 0.4 * sin(time * 0.7 + cracks.id.x));
        out.height = grain * 0.5 - crack + worn * 0.3;
        out.rough = 0.85 - band * 0.15;
    } else if kind < 3.5 {
        let cracks = cellular(at * 0.8 + layered(at * 0.4) * 0.8);
        let fissure = 1.0 - smoothstep(0.0, 0.1, cracks.edge);
        let heat = layered(at * 0.3 + vec3<f32>(0.0, -time * 0.05, 0.0));
        let crust = smoothstep(0.4, 0.9, normal.y) * (1.0 - smoothstep(0.0, 0.3, rim)) * layered(at * 2.0);
        var color = mix(vec3<f32>(0.06, 0.04, 0.03), vec3<f32>(0.2, 0.15, 0.11), layered(at * 2.5));
        color = mix(color, vec3<f32>(0.75, 0.66, 0.2), crust);
        let flicker = 0.7 + 0.3 * sin(time * 3.0 + cracks.id.y * 5.0);
        out.color = color * (1.0 - fissure * 0.5);
        out.glow = vec3<f32>(1.0, 0.38, 0.08) * fissure * heat * flicker * 1.6;
        out.height = layered(at * 2.5) - fissure;
        out.rough = 0.9;
    } else if kind < 4.5 {
        let pits = cellular(at * 3.2);
        let pit = 1.0 - smoothstep(0.12, 0.3, pits.near);
        let algae = smoothstep(0.55, 0.75, layered(at * 0.5)) * smoothstep(0.2, 0.8, normal.y + 0.4);
        let blush = smoothstep(0.5, 0.8, layered(at * 0.7 + 9.0));
        var color = vec3<f32>(0.82, 0.78, 0.68) * (0.85 + layered(at * 4.0) * 0.25);
        color = mix(color, vec3<f32>(0.85, 0.55, 0.55), blush * 0.5);
        color = mix(color, vec3<f32>(0.3, 0.5, 0.25), algae * 0.7);
        let worn = 1.0 - smoothstep(0.0, 0.15, rim);
        out.color = color * (1.0 - pit * 0.55) * (1.0 - worn * 0.25);
        out.height = -pit + layered(at * 4.0) * 0.4;
        out.rough = 0.95;
    } else if kind < 5.5 {
        let toward = normalize(eye - at);
        let facing = abs(dot(toward, normal));
        let bands = layered(at * 0.6) * 6.0 + dot(at, vec3<f32>(0.3, 0.7, 0.2)) * 2.0;
        let film = bands + facing * 3.0;
        let iris = 0.5 + 0.5 * cos(6.2832 * (vec3<f32>(0.0, 0.33, 0.67) + film * 0.35));
        let ridge = abs(fract(bands * 0.5) - 0.5) * 2.0;
        out.color = mix(vec3<f32>(0.9, 0.88, 0.93), iris, 0.35) * (0.88 + ridge * 0.12);
        out.glow = iris * 0.04;
        out.height = ridge * 0.4;
        out.rough = 0.18;
        out.metal = 0.25;
    } else if kind < 6.5 {
        let warped = at * 3.4 + vec3<f32>(layered(at * 0.8), layered(at * 0.8 + 11.0), layered(at * 0.8 + 23.0)) * 2.2;
        let ridge = abs(lumpy(warped) - 0.5) + abs(lumpy(warped * 2.1 + 5.0) - 0.5) * 0.3;
        let groove = 1.0 - smoothstep(0.04, 0.1, ridge);
        var color = mix(vec3<f32>(0.86, 0.52, 0.34), vec3<f32>(0.96, 0.74, 0.5), layered(at * 1.3));
        color = mix(color, vec3<f32>(0.62, 0.72, 0.42), smoothstep(0.6, 0.8, layered(at * 0.4 + 3.0)) * 0.5);
        out.color = color * (1.0 - groove * 0.6);
        out.height = -groove + ridge;
        out.rough = 0.8;
    } else if kind < 7.5 {
        let pores = cellular(at * 2.6);
        let pore = 1.0 - smoothstep(0.1, 0.22, pores.near);
        let mouth = cellular(at * 0.7);
        let osculum = 1.0 - smoothstep(0.08, 0.16, mouth.near);
        var color = vec3<f32>(0.92, 0.7, 0.18) * (0.8 + layered(at * 2.0) * 0.35);
        color = mix(color, vec3<f32>(0.95, 0.5, 0.15), smoothstep(0.5, 0.8, layered(at * 0.6)) * 0.4);
        out.color = color * (1.0 - max(pore * 0.6, osculum * 0.85));
        out.height = -max(pore, osculum * 1.5);
        out.rough = 1.0;
    } else if kind < 8.5 {
        let veins = cellular(at * 1.5 + layered(at * 0.5));
        let vein = 1.0 - smoothstep(0.0, 0.12, veins.edge);
        let breathe = 0.65 + 0.35 * sin(time * 0.9 + veins.id.x * 2.0 + veins.id.z);
        let core = 1.0 - veins.near;
        out.color = mix(vec3<f32>(0.08, 0.3, 0.26), vec3<f32>(0.3, 0.8, 0.64), core * 0.6);
        out.glow = vec3<f32>(0.25, 1.0, 0.75) * (vein * 0.9 + core * 0.35) * breathe;
        out.height = core - vein * 0.5;
        out.rough = 0.4;
    } else if kind > MASONRY - 0.5 {
        out = crafted(kind, at, normal, size, time);
    } else if kind > TURF - 0.5 {
        out = turfed(kind, at, spot, normal, size, time);
    } else if kind > 22.5 && kind < 23.5 {
        out = earthen(at, spot, normal, size);
    } else if kind > 23.5 && kind < 24.5 {
        out = sandy(at, spot, normal, size);
    } else {
        out = rocky(kind, at, normal, size);
    }
    return out;
}

fn bumped(normal: vec3<f32>, at: vec3<f32>, height: f32, scale: f32) -> vec3<f32> {
    let dx = dpdx(at);
    let dy = dpdy(at);
    let hx = dpdx(height) * scale;
    let hy = dpdy(height) * scale;
    let r1 = cross(dy, normal);
    let r2 = cross(normal, dx);
    let det = dot(dx, r1);
    let grad = sign(det) * (hx * r1 + hy * r2);
    return normalize(abs(det) * normal - grad);
}

fn glow(at: vec3<f32>, time: f32) -> f32 {
    let pulse = 0.5 + 0.5 * sin(time * 4.5);
    let sweep = fract(dot(at, vec3<f32>(0.42, 0.75, 0.31)) * 0.9 - time * 0.8);
    let band = smoothstep(0.0, 0.06, sweep) * (1.0 - smoothstep(0.06, 0.32, sweep));
    return 0.25 + 0.35 * pulse + band;
}

@fragment
fn fragment(passed: Varying, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let in = plain(passed);
    let spot = Spot(passed.near, vec3<i32>(anchor()));
    let at = in.world_position.xyz;
    let dx = dpdx(at);
    let dy = dpdy(at);
    var tinted = in;
    var sky = 1.0;
#ifdef VERTEX_COLORS
    sky = max(in.color.a, CAVE_DARK);
    tinted.color = vec4<f32>(in.color.rgb, 1.0);
#endif
    var pbr_input = pbr_input_from_standard_material(tinted, is_front);
    let normal = normalize(in.world_normal);
    let size = sqrt(length(dx) * length(dy)) * 16.0;
    var mark = 0.0;
    var procedural = false;
#ifdef VERTEX_UVS_B
    procedural = in.uv_b.y < -99999.5;
#endif
    if !procedural {
        pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    }
#ifdef VERTEX_UVS_B
    mark = in.uv_b.x;
#ifdef VERTEX_COLORS
    let packed = round(-in.uv_b.y - 100000.0);
    let kind = floor(packed / 256.0);
    if procedural && kind > LEAVES - 0.5 && kind < MASONRY - 0.5 {
        let leaf = leafy(kind, at - sway(at, vec2<f32>(1.0, 0.0), surface.weather.x, globals.time), normal, size);
        if leaf.a < 0.5 {
            discard;
        }
        pbr_input.material.base_color = vec4<f32>(leaf.rgb, 1.0) * tinted.color;
        pbr_input.material.perceptual_roughness = 0.95;
        pbr_input.material.reflectance = vec3<f32>(0.12);
    } else if procedural {
        let mask = packed - kind * 256.0;
        let made = finish(kind, mask, at, spot, normal, view.world_position, globals.time, size);
        pbr_input.material.base_color = vec4<f32>(made.color, 1.0) * tinted.color;
        pbr_input.material.perceptual_roughness = made.rough;
        pbr_input.material.metallic = made.metal;
        pbr_input.material.emissive = vec4<f32>(made.glow * 1.4, 0.0);
        pbr_input.N = bumped(tilted(pbr_input.N, made.slope), at, made.height, 0.05);
    } else if in.uv_b.y < -0.5 {
        pbr_input.material.base_color = projected(in.uv, in.uv_b.y, at, normal, dx, dy) * tinted.color;
    }
#endif
#endif
    let sunk = max(SEA_TOP - at.y, 0.0) - surface.weather.z;
    let drowned = clamp(exp(-sunk * ABSORBED), vec3<f32>(0.12), vec3<f32>(1.8));
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * select(vec3<f32>(1.0), drowned, at.y < SEA_TOP && surface.weather.z >= 0.0), pbr_input.material.base_color.a);
    let soaked = surface.weather.y * smoothstep(0.3, 0.9, normal.y);
    pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * (1.0 - 0.3 * soaked), pbr_input.material.base_color.a);
    pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 0.28, soaked);
    pbr_input.material.reflectance = mix(pbr_input.material.reflectance, vec3<f32>(0.5), soaked);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = vec4<f32>(out.color.rgb + pbr_input.material.base_color.rgb / sky * shined(at, normal), out.color.a);
    if mark > 1.5 {
        let shine = glow(at, globals.time);
        let base = pbr_input.material.base_color.rgb;
        out.color = vec4<f32>(out.color.rgb * (1.0 + 1.2 * shine) + (base * 0.5 + vec3<f32>(0.1, 0.12, 0.14)) * shine, out.color.a);
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
