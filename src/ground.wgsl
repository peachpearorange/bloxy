#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{Vertex, VertexOutput, FragmentOutput},
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{globals, view},
    mesh_functions,
    view_transformations::position_world_to_clip,
}
#import bloxy::meadow::{lawn, fuzz}

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
const SEA_TOP: f32 = 62.88;
const ABSORBED: vec3<f32> = vec3<f32>(0.22, 0.09, 0.06);
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
    let soil = vec4<f32>(rocky(23.0, at, normal, max(length(dx), length(dy)) * 16.0).color, 1.0);
    let ragged = (lumpy(at * 2.3) - 0.5) * 0.3 + (lumpy(at * 7.0) - 0.5) * 0.12;
    let covered = smoothstep(0.34, 0.5, normal.y + ragged);
    var cover = painted(top, tile, at.xz, dx.xz, dy.xz);
    if all(abs(top - surface.turf.xy) < tile * 0.5) {
        let size = max(length(dx.xz), length(dy.xz)) * 16.0;
        cover = vec4<f32>(select(lawn(at.xz, globals.time, distance(at.xz, view.world_position.xz)), fuzz(at.xz, globals.time, size, surface.turf.w < 0.5), surface.turf.z > 0.5), 1.0);
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
    let position = clamp(along, 0.0, 1.0) * 3.999;
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

fn below(value: f32, edge: f32, pixel: bool) -> f32 {
    return select(1.0 - smoothstep(edge - 0.08, edge + 0.08, value), select(0.0, 1.0, value < edge), pixel);
}

fn rocky(kind: f32, at: vec3<f32>, normal: vec3<f32>, size: f32) -> Finish {
    let fine = at * 16.0 - normal * 0.02;
    let gravel = kind > 14.5 && kind < 15.5;
    let cobbled = kind > 27.5;
    let pixel = surface.turf.w < 0.5 && !gravel;
    let cell = vec3<i32>(floor(fine));
    let point = select(fine, vec3<f32>(cell) + 0.5, pixel);
    let centre = point / 16.0;
    let palette = rock_palette(kind);
    let speckle = smoothstep(0.0, 1.0, (supple(fine * 0.9 + 13.0) - 0.5) * 2.4 + 0.5);
    let pick = select(speckle, texel_hash(cell, 1u), pixel);
    let cloud = supple(centre * 3.0) * 0.55 + supple(centre * 1.1 + 7.0) * 0.45;
    var along = 0.5 + (cloud - 0.5) * 1.5 + (pick - 0.5) * 0.12;
    var color = vec3<f32>(0.0);
    var glow = 0.0;
    var rough = 0.9;
    let clump = select(smoothstep(0.0, 1.0, (supple(fine * 0.45 + 29.0) - 0.5) * 2.2 + 0.5), texel_hash(vec3<i32>(floor(vec3<f32>(cell) / 2.0)), 2u), pixel);
    if cobbled || gravel {
        let scale = select(3.0, 5.0, cobbled);
        let stone = pebbles(point / scale, false);
        let tone = texel_hash(stone.id, 9u);
        let lit = dot(normalize(point / scale - stone.centre + vec3<f32>(0.0, 0.001, 0.0)), normalize(vec3<f32>(-0.4, 0.8, -0.4)));
        let rim = select(0.42, 0.9, cobbled);
        let body = 1.0 - smoothstep(rim - 0.08, rim, stone.near);
        let seam = smoothstep(0.02, 0.08, stone.edge);
        let warm = select(vec3<f32>(1.0), vec3<f32>(1.1, 0.98, 0.86), texel_hash(stone.id, 16u) < 0.3);
        let grit = palette[0] * (0.42 + clump * 0.12);
        along = 0.25 + tone * 0.5 + lit * 0.22 * smoothstep(0.0, 0.35, stone.near) + (pick - 0.5) * 0.08;
        color = mix(grit, ramped(palette, along) * warm, body * seam);
    } else if kind > 9.5 && kind < 10.5 {
        color = ramped(palette, along + 0.2 * (1.0 - below(clump, 0.72, pixel)));
        color = mix(color, vec3<f32>(0.12, 0.1, 0.1), below(pick, 0.12, pixel));
        color = mix(color, vec3<f32>(0.86, 0.74, 0.7), 1.0 - below(pick, 0.94, pixel));
    } else if kind > 10.5 && kind < 11.5 {
        color = ramped(palette, along + 0.1);
        color = mix(color, vec3<f32>(0.16, 0.16, 0.17), below(clump, 0.16, pixel) * below(pick, 0.7, pixel));
        color = mix(color, vec3<f32>(0.34, 0.34, 0.35), below(pick, 0.05, pixel));
    } else if kind > 11.5 && kind < 12.5 {
        color = ramped(palette, 0.5 + (cloud - 0.5) * 0.8 + (pick - 0.5) * 0.45);
    } else if kind > 12.5 && kind < 13.5 {
        let bed = supple(vec3<f32>(centre.x * 0.7, centre.y * 5.0, centre.z * 0.7));
        color = ramped(palette, 0.2 + bed * 0.6 + (cloud - 0.5) * 0.4);
        color = mix(color, palette[0] * 0.85, below(pick, 0.04, pixel));
    } else if kind > 13.5 && kind < 14.5 {
        let row = point.y + supple(centre * 1.3) * 3.0;
        let seam = select(smoothstep(0.33, 0.43, abs(fract(row / 3.0) - 0.5)), select(0.0, 1.0, abs(floor(row) - 3.0 * floor(floor(row) / 3.0)) < 0.5), pixel);
        color = ramped(palette, along - 0.25 * seam - 0.05);
        rough = 0.62;
    } else if kind > 22.5 && kind < 23.5 {
        let warped = point / 1.8 + (vec3<f32>(supple(point * 0.35), supple(point * 0.35 + 19.0), supple(point * 0.35 + 37.0)) - 0.5) * 1.6;
        let clod = pebbles(warped, false);
        let lit = dot(normalize(warped - clod.centre + vec3<f32>(0.0, 0.001, 0.0)), normalize(vec3<f32>(-0.4, 0.8, -0.4)));
        let crumb = 1.0 - smoothstep(0.1, 0.6, clod.near);
        let crevice = (1.0 - smoothstep(0.0, 0.14, clod.edge)) * smoothstep(0.4, 0.7, supple(point * 0.5 + 71.0));
        let grain = supple(point * 1.7 + 61.0);
        let crumbs = smoothstep(0.66, 0.78, supple(point * 0.8 + 83.0));
        let flecks = smoothstep(0.7, 0.8, supple(point * 1.2 + 97.0));
        let broad = supple(centre * 1.6 + 23.0) * 0.6 + supple(centre * 4.5 + 3.0) * 0.4;
        let soil = 0.45 + (broad - 0.5) * 0.7 + (texel_hash(clod.id, 18u) - 0.5) * 0.12 + lit * 0.1 * crumb + (grain - 0.5) * 0.25 - crevice * 0.14 - crumbs * 0.22 + flecks * 0.14;
        color = ramped(palette, soil);
        let stone = pebbles(point / 5.0 + 7.3, false);
        let present = texel_hash(stone.id, 19u) < 0.14;
        let shine = dot(normalize(point / 5.0 + 7.3 - stone.centre + vec3<f32>(0.0, 0.001, 0.0)), normalize(vec3<f32>(-0.4, 0.8, -0.4)));
        let pebble = select(0.0, select(1.0 - smoothstep(0.2, 0.32, stone.near), select(0.0, 1.0, stone.near < 0.26), pixel), present);
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
        let crack = 1.0 - below(abs(fract((point.y + texel_hash(column.id, 7u) * 16.0) / 16.0) - 0.5), 0.47, pixel);
        color = ramped(palette, 0.35 + texel_hash(column.id, 8u) * 0.3 + (pick - 0.5) * 0.15) * (1.0 - joint * 0.6) * (1.0 - 0.4 * crack);
        rough = 0.75;
    } else if kind > 26.5 && kind < 27.5 {
        let crack = pebbles(point / 4.0, false);
        color = ramped(palette, along * 0.8 + 0.1) * (0.5 + 0.5 * smoothstep(0.02, 0.1, crack.edge));
    } else {
        color = ramped(palette, along - 0.28 * below(pick, 0.06, pixel) + 0.14 * (1.0 - below(pick, 0.95, pixel)));
    }
    if kind > 15.5 && kind < 22.5 {
        let ore = vein(kind);
        let blob = floor(point / 5.0);
        let present = texel_hash(vec3<i32>(blob), 11u) < 0.7;
        let middle = blob * 5.0 + 1.0 + vec3<f32>(texel_hash(vec3<i32>(blob), 12u), texel_hash(vec3<i32>(blob), 13u), texel_hash(vec3<i32>(blob), 14u)) * 3.0;
        let toward = point - middle;
        let inside = select(0.0, below(length(toward), 1.3 + texel_hash(vec3<i32>(blob), 15u) * 0.8, pixel), present);
        let lean = dot(toward, vec3<f32>(-0.5, 0.7, -0.5));
        let stained = select(mix(mix(ore.dark, ore.base, smoothstep(-0.8, -0.2, lean)), ore.light, smoothstep(0.2, 0.7, lean)), select(select(ore.base, ore.light, lean > 0.4), ore.dark, lean < -0.5), pixel);
        color = mix(color, stained, inside);
        glow = ore.glow * inside;
        rough = mix(rough, 0.5, inside);
    }
    let flat = ramped(palette, select(0.5, 0.4, cobbled || gravel)) * select(1.0, 0.85, cobbled || gravel);
    let shown = mix(color, flat, smoothstep(0.8, 2.0, size) * (1.0 - smoothstep(0.0, 0.05, glow)));
    return Finish(shown, 0.0, shown * glow * 0.6, rough, 0.0);
}

const TURF: f32 = 34.0;
const LEAVES: f32 = 37.0;

fn turfed(kind: f32, at: vec3<f32>, normal: vec3<f32>, size: f32, time: f32) -> vec3<f32> {
    let soil = rocky(23.0, at, normal, size).color;
    let ragged = (supple(at * 2.3) - 0.5) * 0.3 + (supple(at * 7.0) - 0.5) * 0.12;
    let inside = at - normal * 0.02;
    let texel = vec3<i32>(floor(inside * 16.0));
    let block = floor(inside);
    let pixel = surface.turf.w < 0.5;
    let dangle = select(supple(vec3<f32>(inside.x * 5.0, block.y * 7.0, inside.z * 5.0) + 41.0) * 0.6 + supple(vec3<f32>(inside.x * 13.0, block.y, inside.z * 13.0)) * 0.4, texel_hash(vec3<i32>(texel.x, i32(block.y), texel.z), 21u), pixel);
    let hang = (2.0 + dangle * 4.0) / 16.0;
    let fringe = abs(normal.y) < 0.6 && block.y + 1.0 - at.y < hang;
    let covered = max(smoothstep(0.34, 0.5, normal.y + ragged), select(0.0, 1.0, fringe));
    let pick = select(smoothstep(0.0, 1.0, (supple(inside * 14.0 + 3.0) - 0.5) * 2.6 + 0.5), texel_hash(texel, 22u), pixel);
    let cloud = supple(select(inside, (vec3<f32>(texel) + 0.5) / 16.0, pixel) * 3.0) * 0.6 + supple(at * 0.9 + 5.0) * 0.4;
    var cover = vec3<f32>(0.0);
    if kind < TURF + 0.5 {
        cover = select(lawn(at.xz, time, distance(at.xz, view.world_position.xz)), fuzz(at.xz, time, size, pixel), surface.turf.z > 0.5);
        cover = cover * select(1.0, 0.82, fringe && normal.y < 0.6);
    } else if kind < TURF + 1.5 {
        let snow = array<vec3<f32>, 4>(vec3<f32>(0.7, 0.75, 0.82), vec3<f32>(0.82, 0.86, 0.92), vec3<f32>(0.92, 0.95, 0.99), vec3<f32>(1.0, 1.0, 1.0));
        cover = ramped(snow, 0.55 + (cloud - 0.5) * 0.6 + (pick - 0.5) * 0.1) + vec3<f32>(select(0.0, 0.25, pick > 0.985 && size < 1.0 && pixel));
    } else {
        let spores = array<vec3<f32>, 4>(vec3<f32>(0.32, 0.26, 0.35), vec3<f32>(0.4, 0.33, 0.43), vec3<f32>(0.47, 0.39, 0.49), vec3<f32>(0.58, 0.5, 0.58));
        cover = ramped(spores, 0.45 + (cloud - 0.5) * 0.9 + (pick - 0.5) * 0.3);
        cover = mix(cover, vec3<f32>(0.72, 0.66, 0.7), (1.0 - below(pick, 0.94, pixel)) * (1.0 - smoothstep(0.8, 1.2, size)));
    }
    return mix(soil, cover, covered);
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
    let pixel = surface.turf.w < 0.5;
    let point = select(fine, floor(fine) + 0.5, pixel);
    let leaf = pebbles(point / 3.0 * stretch, false);
    let clump = supple(point / 16.0 * 2.5);
    let lit = dot(normalize(point / 3.0 * stretch - leaf.centre + vec3<f32>(0.0, 0.001, 0.0)), normalize(vec3<f32>(-0.4, 0.8, -0.4)));
    let tone = texel_hash(leaf.id, 17u);
    let body = leaf.near < 0.4 && leaf.edge > 0.06;
    let open = !body && clump < 0.66 && size < 1.2;
    let color = ramped(palette, 0.05 + tone * 0.4 + lit * 0.22 * smoothstep(0.0, 0.35, leaf.near) + (clump - 0.5) * 0.3);
    let filled = select(smoothstep(0.42, 0.36, leaf.near) * smoothstep(0.04, 0.08, leaf.edge), select(0.0, 1.0, body), pixel);
    let shown = mix(palette[0] * 0.55, color, filled);
    return vec4<f32>(mix(shown, ramped(palette, 0.45), smoothstep(0.8, 2.0, size)), select(1.0, 0.0, open));
}

fn finish(kind: f32, mask: f32, at: vec3<f32>, normal: vec3<f32>, eye: vec3<f32>, time: f32, size: f32) -> Finish {
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
    var out = Finish(vec3<f32>(0.5), 0.0, vec3<f32>(0.0), 0.8, 0.0);
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
    } else if kind > TURF - 0.5 {
        out = Finish(turfed(kind, at, normal, size, time), 0.0, vec3<f32>(0.0), 0.95, 0.0);
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
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let at = in.world_position.xyz;
    let dx = dpdx(at);
    let dy = dpdy(at);
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let normal = normalize(in.world_normal);
    let size = max(length(dx), length(dy)) * 16.0;
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
    if procedural && kind > LEAVES - 0.5 {
        let leaf = leafy(kind, at - sway(at, vec2<f32>(1.0, 0.0), surface.weather.x, globals.time), normal, size);
        if leaf.a < 0.5 {
            discard;
        }
        pbr_input.material.base_color = vec4<f32>(leaf.rgb, 1.0) * in.color;
        pbr_input.material.perceptual_roughness = 0.95;
        pbr_input.material.reflectance = vec3<f32>(0.12);
    } else if procedural {
        let mask = packed - kind * 256.0;
        let made = finish(kind, mask, at, normal, view.world_position, globals.time, size);
        pbr_input.material.base_color = vec4<f32>(made.color, 1.0) * in.color;
        pbr_input.material.perceptual_roughness = made.rough;
        pbr_input.material.metallic = made.metal;
        pbr_input.material.emissive = vec4<f32>(made.glow * 1.4, 0.0);
        pbr_input.N = bumped(pbr_input.N, at, made.height, 0.05);
    } else if in.uv_b.y < -0.5 {
        pbr_input.material.base_color = projected(in.uv, in.uv_b.y, at, normal, dx, dy) * in.color;
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
    if mark > 1.5 {
        let shine = glow(at, globals.time);
        let base = pbr_input.material.base_color.rgb;
        out.color = vec4<f32>(out.color.rgb * (1.0 + 1.2 * shine) + (base * 0.5 + vec3<f32>(0.1, 0.12, 0.14)) * shine, out.color.a);
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
