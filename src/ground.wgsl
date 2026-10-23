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

fn finish(kind: f32, mask: f32, at: vec3<f32>, normal: vec3<f32>, eye: vec3<f32>, time: f32) -> Finish {
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
        let warped = at * 1.6 + vec3<f32>(layered(at * 0.5), layered(at * 0.5 + 11.0), layered(at * 0.5 + 23.0)) * 1.4;
        let ridge = abs(lumpy(warped) - 0.5) + abs(lumpy(warped * 2.1 + 5.0) - 0.5) * 0.4;
        let groove = 1.0 - smoothstep(0.03, 0.09, ridge);
        var color = mix(vec3<f32>(0.78, 0.6, 0.44), vec3<f32>(0.9, 0.76, 0.58), layered(at * 1.3));
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
    } else {
        let veins = cellular(at * 1.5 + layered(at * 0.5));
        let vein = 1.0 - smoothstep(0.0, 0.12, veins.edge);
        let breathe = 0.65 + 0.35 * sin(time * 0.9 + veins.id.x * 2.0 + veins.id.z);
        let core = 1.0 - veins.near;
        out.color = mix(vec3<f32>(0.08, 0.3, 0.26), vec3<f32>(0.3, 0.8, 0.64), core * 0.6);
        out.glow = vec3<f32>(0.25, 1.0, 0.75) * (vein * 0.9 + core * 0.35) * breathe;
        out.height = core - vein * 0.5;
        out.rough = 0.4;
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
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    let normal = normalize(in.world_normal);
    var mark = 0.0;
#ifdef VERTEX_UVS_B
    mark = in.uv_b.x;
#ifdef VERTEX_COLORS
    if in.uv_b.y < -99999.5 {
        let packed = round(-in.uv_b.y - 100000.0);
        let kind = floor(packed / 256.0);
        let mask = packed - kind * 256.0;
        let made = finish(kind, mask, at, normal, view.world_position, globals.time);
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
