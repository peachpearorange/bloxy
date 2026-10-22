use {crate::{block::{Block, Fluid, Look},
             generate,
             hull::{Facet, Hull},
             island::SEA,
             model::{self, Bit},
             noise::{self, fbm2},
             texture::{tile_origin, uv_corner},
             voxels::{Chunk, HEIGHT, SIZE, Voxels, chunk_of, origin_of}},
     bevy::{asset::RenderAssetUsages,
            mesh::{Indices, PrimitiveTopology},
            prelude::*},
     std::sync::Arc};

const PAD: i32 = 2;
const SPAN: i32 = SIZE + 2 * PAD;
const RIM: i32 = SIZE + 2;
const WATER_TOP: f32 = 0.88;
const OCCLUSION: [f32; 4] = [0.42, 0.62, 0.8, 1.0];
const SKY_FALLOFF: f32 = 7.0;
const CAVE_DARK: f32 = 0.05;
const OCEAN_DEPTH: f32 = 14.0;
const SHALLOWS: LinearRgba = LinearRgba::rgb(0.03, 0.3, 0.33);
const OPEN_SEA: LinearRgba = LinearRgba::rgb(0.004, 0.025, 0.11);

pub struct Padded(Vec<Block>);

impl Padded {
  fn index(local: IVec3) -> usize {
    let shifted = local + IVec3::splat(PAD);
    ((shifted.y * SPAN + shifted.z) * SPAN + shifted.x) as usize
  }

  pub fn get(&self, local: IVec3) -> Block { self.0[Padded::index(local)] }

  pub fn gather(
    key: IVec3,
    around: impl Fn(IVec3) -> Option<Arc<Chunk>>
  ) -> Option<Padded> {
    let neighbours: Vec<(IVec3, Option<Arc<Chunk>>)> = (-1..=1)
      .flat_map(|y| {
        (-1..=1).flat_map(move |z| (-1..=1).map(move |x| IVec3::new(x, y, z)))
      })
      .map(|offset| (offset, around(key + offset)))
      .collect();
    let origin = origin_of(key);
    let outside = |offset: IVec3| {
      let y = origin.y + offset.y * SIZE;
      (y < 0).then_some(Block::Bedrock).or((y >= HEIGHT).then_some(Block::Air))
    };
    let ready = neighbours
      .iter()
      .all(|(offset, chunk)| chunk.is_some() || outside(*offset).is_some());
    ready.then(|| {
      Padded(
        (0..SPAN * SPAN * SPAN)
          .map(|index| {
            let local =
              IVec3::new(index % SPAN, index / (SPAN * SPAN), index / SPAN % SPAN)
                - IVec3::splat(PAD);
            let offset = local.div_euclid(IVec3::splat(SIZE));
            let (_, chunk) = &neighbours
              [(((offset.y + 1) * 3 + offset.z + 1) * 3 + offset.x + 1) as usize];
            chunk
              .as_ref()
              .map(|chunk| chunk.get(local.rem_euclid(IVec3::splat(SIZE))))
              .or_else(|| outside(offset))
              .unwrap_or_default()
          })
          .collect()
      )
    })
  }
}

struct Face {
  normal: IVec3,
  across: IVec3,
  up: IVec3,
  base: IVec3,
  side: usize
}

const FACES: [Face; 6] = [
  Face {
    normal: IVec3::X,
    across: IVec3::NEG_Z,
    up: IVec3::Y,
    base: IVec3::new(1, 0, 1),
    side: 1
  },
  Face {
    normal: IVec3::NEG_X,
    across: IVec3::Z,
    up: IVec3::Y,
    base: IVec3::ZERO,
    side: 1
  },
  Face {
    normal: IVec3::Z,
    across: IVec3::X,
    up: IVec3::Y,
    base: IVec3::new(0, 0, 1),
    side: 1
  },
  Face {
    normal: IVec3::NEG_Z,
    across: IVec3::NEG_X,
    up: IVec3::Y,
    base: IVec3::new(1, 0, 0),
    side: 1
  },
  Face {
    normal: IVec3::Y,
    across: IVec3::X,
    up: IVec3::NEG_Z,
    base: IVec3::new(0, 1, 1),
    side: 0
  },
  Face {
    normal: IVec3::NEG_Y,
    across: IVec3::X,
    up: IVec3::Z,
    base: IVec3::ZERO,
    side: 2
  }
];

const TRUNK_SIDES: usize = 12;
const TRUNK_RADIUS: f32 = 0.47;

fn trunk() -> [Vec2; TRUNK_SIDES] {
  std::array::from_fn(|side| {
    Vec2::splat(0.5)
      + Vec2::from_angle(side as f32 / TRUNK_SIDES as f32 * std::f32::consts::TAU)
        * TRUNK_RADIUS
  })
}

const CORNERS: [(i32, i32); 4] = [(0, 0), (1, 0), (1, 1), (0, 1)];

const UNIT: i32 = 32;
const ROUND: i32 = 6;

fn turf(block: Block) -> bool {
  matches!(block, Block::Grass | Block::Snow | Block::Mycelium)
}

fn projection(block: Block) -> Option<f32> {
  let [top, side, bottom] = block.tiles();
  let flank = if turf(block) { bottom } else { side };
  (block.look() == Look::Opaque
    && block.softness() > 0.0
    && block != Block::Lumen
    && (turf(block) || top == side || side == bottom))
    .then(|| -1.0 - (flank.index() * 256 + bottom.index()) as f32)
}

const UNOWNED: f32 = -2.0;

fn owner(local: IVec3) -> f32 {
  ((local.y * SIZE + local.z) * SIZE + local.x) as f32 * 2.0
}

fn rounds(block: Block) -> bool { block.look() == Look::Opaque }

type Section = [[bool; 2]; 2];

fn bends(solid: Section) -> bool {
  (0..2).any(|x| solid[x][0] != solid[x][1]) && (0..2).any(|y| solid[0][y] != solid[1][y])
}

fn section(padded: &Padded, axis: usize, line: IVec2, along: i32) -> Section {
  std::array::from_fn(|x| {
    std::array::from_fn(|y| {
      let mut cell = IVec3::ZERO;
      cell[axis] = along;
      cell[(axis + 1) % 3] = line.x - 1 + x as i32;
      cell[(axis + 2) % 3] = line.y - 1 + y as i32;
      rounds(padded.get(cell))
    })
  })
}

fn bent_at(padded: &Padded, corner: IVec3) -> bool {
  (0..3).any(|axis| {
    let line = IVec2::new(corner[(axis + 1) % 3], corner[(axis + 2) % 3]);
    [corner[axis] - 1, corner[axis]]
      .into_iter()
      .any(|along| bends(section(padded, axis, line, along)))
  })
}

struct Bend {
  shift: Vec2,
  normal: Vec2,
  face: Vec2
}

fn bend(solid: Section, at: IVec2) -> Option<Bend> {
  let surface_a = |x: usize| solid[x][0] != solid[x][1];
  let surface_b = |y: usize| solid[0][y] != solid[1][y];
  let turns = |x: usize, y: usize| surface_a(x) && surface_b(y);
  let side = |value: i32| usize::from(value > 0);
  let pick = |candidates: [(usize, usize); 2]| {
    candidates.into_iter().filter(|&(x, y)| turns(x, y)).max_by_key(|&(x, y)| solid[x][y])
  };
  let turn = match (at.x, at.y) {
    (0, 0) => {
      let all: Vec<(usize, usize)> = [(0, 0), (0, 1), (1, 0), (1, 1)]
        .into_iter()
        .filter(|&(x, y)| turns(x, y))
        .collect();
      (all.len() == 1).then(|| all[0])
    }
    (a, 0) if a.abs() < ROUND && surface_a(side(a)) => pick([(side(a), 0), (side(a), 1)]),
    (0, b) if b.abs() < ROUND && surface_b(side(b)) => pick([(0, side(b)), (1, side(b))]),
    _ => None
  };
  turn.map(|(x, y)| {
    let sign = |side: usize| if side == 1 { 1.0 } else { -1.0 };
    let round = ROUND as f32;
    let point = at.as_vec2();
    let centre = Vec2::new(sign(x), sign(y)) * round;
    let out = (point - centre).normalize();
    let normal = if solid[x][y] { out } else { -out };
    Bend {
      shift: (centre + out * round - point) / UNIT as f32,
      normal,
      face: match (at.x, at.y) {
        (0, 0) => normal,
        (a, 0) => Vec2::new(0.0, if solid[side(a)][1] { -1.0 } else { 1.0 }),
        (_, b) => Vec2::new(if solid[1][side(b)] { -1.0 } else { 1.0 }, 0.0)
      }
    }
  })
}

fn rounded(padded: &Padded, at: IVec3) -> (Vec3, Option<Vec3>) {
  let bends: Vec<(Vec3, Vec3, Vec3)> = (0..3)
    .flat_map(|axis| {
      let (first, second) = ((axis + 1) % 3, (axis + 2) % 3);
      let line = IVec2::new(
        (at[first] + UNIT / 2).div_euclid(UNIT),
        (at[second] + UNIT / 2).div_euclid(UNIT)
      );
      let offset = IVec2::new(at[first], at[second]) - line * UNIT;
      let along = at[axis].div_euclid(UNIT);
      let segments = match at[axis].rem_euclid(UNIT) {
        0 => vec![along - 1, along],
        _ => vec![along]
      };
      let weight = 1.0 / segments.len() as f32;
      let lift = move |flat: Vec2| {
        let mut lifted = Vec3::ZERO;
        lifted[first] = flat.x;
        lifted[second] = flat.y;
        lifted * weight
      };
      segments.into_iter().filter_map(move |along| {
        bend(section(padded, axis, line, along), offset)
          .map(|Bend { shift, normal, face }| (lift(shift), lift(normal), lift(face)))
      })
    })
    .collect();
  let position =
    at.as_vec3() / UNIT as f32 + bends.iter().map(|&(shift, _, _)| shift).sum::<Vec3>();
  let base = bends.iter().map(|&(_, _, face)| face).sum::<Vec3>().normalize_or_zero();
  let normal = (!bends.is_empty()).then(|| {
    bends
      .iter()
      .fold(base, |normal, &(_, bent, face)| normal + bent - face)
      .normalize_or(base)
  });
  (position, normal)
}

fn kind(block: Block) -> Option<Fluid> { block.liquid().map(|(fluid, _)| fluid) }

fn shows(block: Block, neighbour: Block) -> bool {
  match (block.look(), neighbour.look()) {
    (Look::Invisible | Look::Model | Look::Log, _) => false,
    (_, Look::Invisible | Look::Model | Look::Log) => true,
    (_, Look::Opaque) => false,
    (Look::Liquid, Look::Liquid) => kind(block) != kind(neighbour),
    (Look::Liquid, Look::Cutout) => true,
    (Look::Cutout, _) => block != neighbour || block.leafy(),
    (Look::Opaque, _) => true
  }
}

#[derive(Default)]
struct Builder {
  positions: Vec<[f32; 3]>,
  normals: Vec<[f32; 3]>,
  uvs: Vec<[f32; 2]>,
  colors: Vec<[f32; 4]>,
  sways: Vec<[f32; 2]>,
  indices: Vec<u32>
}

impl Builder {
  fn polygon(
    &mut self,
    points: &[Vec3],
    uvs: &[Vec2],
    normal: Vec3,
    hue: LinearRgba,
    owner: f32,
    bend: impl Fn(usize) -> f32
  ) {
    let first = self.positions.len() as u32;
    for (index, (point, uv)) in points.iter().zip(uvs).enumerate() {
      self.sways.push([owner, bend(index)]);
      self.positions.push(point.to_array());
      self.normals.push(normal.to_array());
      self.uvs.push(uv.to_array());
      self.colors.push(hue.with_alpha(1.0).to_f32_array())
    }
    let facing = (points[1] - points[0]).cross(points[2] - points[0]).dot(normal) > 0.0;
    for corner in 1..points.len() as u32 - 1 {
      self.indices.extend(match facing {
        true => [first, first + corner, first + corner + 1],
        false => [first, first + corner + 1, first + corner]
      })
    }
  }

  fn strip(&mut self, corners: [(Vec3, Vec3, Vec2); 4], hue: LinearRgba, owner: f32) {
    let first = self.positions.len() as u32;
    for (point, normal, uv) in corners {
      self.positions.push(point.to_array());
      self.normals.push(normal.to_array());
      self.uvs.push(uv.to_array());
      self.colors.push(hue.with_alpha(1.0).to_f32_array());
      self.sways.push([owner, 0.0])
    }
    let facing =
      (corners[1].0 - corners[0].0).cross(corners[2].0 - corners[0].0).dot(corners[0].1)
        > 0.0;
    self.indices.extend(
      match facing {
        true => [0, 1, 2, 0, 2, 3],
        false => [0, 2, 1, 0, 3, 2]
      }
      .map(|corner| first + corner)
    )
  }

  fn mesh(self) -> Option<Mesh> {
    (!self.indices.is_empty()).then(|| {
      Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, self.sways)
        .with_inserted_indices(Indices::U32(self.indices))
    })
  }
}

pub struct Meshes {
  pub solid: Option<Mesh>,
  pub cutout: Option<Mesh>,
  pub liquid: Option<Mesh>,
  pub unshadowed: Option<Mesh>,
  pub hull: Hull
}

fn crest(block: Block, above: Block) -> f32 {
  match block.liquid() {
    Some((fluid, _)) if kind(above) == Some(fluid) => 1.0,
    Some((fluid, level)) => {
      let reach = f32::from(fluid.reach()) + 1.0;
      WATER_TOP * (reach - f32::from(level)) / reach
    }
    None => 1.0
  }
}

pub fn sea_tint(floor: f32) -> LinearRgba {
  let deep = ((SEA as f32 - floor) / OCEAN_DEPTH).clamp(0.0, 1.0);
  SHALLOWS.mix(&OPEN_SEA, deep.sqrt())
}

const DRY: LinearRgba = LinearRgba::rgb(1.08, 1.03, 0.78);
const LUSH: LinearRgba = LinearRgba::rgb(0.86, 1.0, 0.92);

fn meadow(at: Vec3) -> LinearRgba {
  let patch = (fbm2(0x9A55, at.x / 40.0, at.z / 40.0, 2) * 1.6 + 0.5).clamp(0.0, 1.0);
  DRY.mix(&LUSH, patch)
}

pub const TORCH_REACH: f32 = 14.0;
const TORCH_GLOW: LinearRgba = LinearRgba::rgb(1.0, 0.86, 0.68);

fn lit(hue: LinearRgba, sky: f32, torch: f32, shade: f32) -> [f32; 4] {
  let channel = |hue: f32, warm: f32| hue * sky.max(torch * warm) * shade;
  [
    channel(hue.red, TORCH_GLOW.red),
    channel(hue.green, TORCH_GLOW.green),
    channel(hue.blue, TORCH_GLOW.blue),
    1.0
  ]
}

fn torchlight(torches: &[IVec3], point: Vec3) -> f32 {
  torches
    .iter()
    .map(|torch| {
      let flame = torch.as_vec3() + Vec3::new(0.5, 0.7, 0.5);
      (1.0 - point.distance(flame) / TORCH_REACH).max(0.0)
    })
    .fold(0.0, f32::max)
    .powf(1.4)
}

fn builders(
  padded: &Padded,
  key: IVec3,
  seed: u32,
  torches: &[IVec3],
  only: Option<IVec3>,
  painted: bool
) -> (Builder, Builder, Builder, Builder, Hull) {
  let origin = origin_of(key);
  let surface: Vec<i32> = (0..RIM * RIM)
    .map(|index| {
      generate::height(seed, origin.x + index % RIM - 1, origin.z + index / RIM - 1)
    })
    .collect();
  let sky = |at: IVec3| {
    let depth = surface[((at.z + 1) * RIM + at.x + 1) as usize] - origin.y - at.y;
    (1.0 - depth as f32 / SKY_FALLOFF).clamp(CAVE_DARK, 1.0)
  };
  let column = |x: i32, z: i32| surface[((z + 1) * RIM + x + 1) as usize];
  let tint = |corner: Vec3| {
    let (x, z) = (corner.x as i32, corner.z as i32);
    sea_tint(
      [(x - 1, z - 1), (x, z - 1), (x - 1, z), (x, z)]
        .map(|(x, z)| column(x.clamp(-1, SIZE), z.clamp(-1, SIZE)))
        .iter()
        .sum::<i32>() as f32
        / 4.0
    )
  };
  let torchlight = |point: Vec3| torchlight(torches, point);
  let mut solid = Builder::default();
  let mut cutout = Builder::default();
  let mut liquid = Builder::default();
  let mut unshadowed = Builder::default();
  let mut hull = Hull::default();
  let cells: Vec<IVec3> = match only {
    Some(local) => vec![local],
    None => (0..SIZE * SIZE * SIZE)
      .map(|index| IVec3::new(index % SIZE, index / (SIZE * SIZE), index / SIZE % SIZE))
      .collect()
  };
  for local in cells {
    let block = padded.get(local);
    let owned = only.map_or(owner(local), |_| UNOWNED);
    if block != Block::Air {
      if only.is_none() && block.solid() {
        let facets: Vec<Facet> = FACES
          .iter()
          .filter(|face| !padded.get(local + face.normal).solid())
          .map(|face| Facet {
            normal: face.normal,
            corners: CORNERS.map(|(a, b)| {
              (origin + local + face.base + face.across * a + face.up * b).as_vec3()
            })
          })
          .collect();
        if !facets.is_empty() {
          hull.0.insert(origin + local, facets);
        }
      }
      let top = crest(block, padded.get(local + IVec3::Y));
      let floor = |face: &Face| {
        let neighbour = padded.get(local + face.normal);
        match face.normal.y == 0
          && kind(block).is_some()
          && kind(block) == kind(neighbour)
        {
          true => Some(crest(neighbour, padded.get(local + face.normal + IVec3::Y)))
            .filter(|&height| height < top - 0.01),
          false => shows(block, neighbour).then_some(0.0)
        }
      };
      for (face, floor) in
        FACES.iter().filter_map(|face| floor(face).map(|floor| (face, floor)))
      {
        let builder = match (block.look(), block.liquid()) {
          (_, Some((Fluid::Water, _))) => &mut liquid,
          (Look::Opaque | Look::Liquid, _) => &mut solid,
          _ => &mut cutout
        };
        let outside = local + face.normal;
        let blocks = |at: IVec3| u8::from(padded.get(at).opaque());
        let clamp = |at: IVec3| at.clamp(IVec3::NEG_ONE, IVec3::splat(SIZE));
        let occlusion = CORNERS.map(|(a, b)| {
          let across = face.across * (a * 2 - 1);
          let up = face.up * (b * 2 - 1);
          let (first, second) = (blocks(outside + across), blocks(outside + up));
          let corner = blocks(outside + across + up);
          match first + second {
            2 => 0,
            sides => 3 - sides - corner
          }
        });
        let daylight = CORNERS.map(|(a, b)| {
          let (across, up) = (face.across * (a * 2 - 1), face.up * (b * 2 - 1));
          [outside, outside + across, outside + up, outside + across + up]
            .map(|at| sky(clamp(at)))
        });
        let tile = block.tiles()[face.side];
        let rounded = rounds(block) && kind(block).is_none();
        let bent = |a: i32, b: i32| {
          rounded && bent_at(padded, local + face.base + face.across * a + face.up * b)
        };
        let stops = |near: bool, far: bool| -> Vec<i32> {
          [(true, 0), (near, ROUND), (far, UNIT - ROUND), (true, UNIT)]
            .into_iter()
            .filter_map(|(kept, stop)| kept.then_some(stop))
            .collect()
        };
        let across = stops(bent(0, 0) || bent(0, 1), bent(1, 0) || bent(1, 1));
        let up = stops(bent(0, 0) || bent(1, 0), bent(0, 1) || bent(1, 1));
        let shades = occlusion.map(|shade| OCCLUSION[shade as usize]);
        let skies = daylight.map(|daylight| daylight.iter().sum::<f32>() / 4.0);
        let blend = |values: [f32; 4], u: f32, v: f32| {
          (values[0] * (1.0 - u) + values[1] * u) * (1.0 - v)
            + (values[3] * (1.0 - u) + values[2] * u) * v
        };
        let projected = projection(block).filter(|_| painted);
        let first = builder.positions.len() as u32;
        let mut shading = Vec::with_capacity(across.len() * up.len());
        for &b in &up {
          for &a in &across {
            let (u, v) = (a as f32 / UNIT as f32, b as f32 / UNIT as f32);
            let at = (local + face.base) * UNIT + face.across * a + face.up * b;
            let flat = at.as_vec3() / UNIT as f32;
            let (position, normal) = match (kind(block), rounded) {
              (Some(_), _) => (
                flat.with_y(match flat.y > local.y as f32 + 0.5 {
                  true => local.y as f32 + top,
                  false => local.y as f32 + floor
                }),
                face.normal.as_vec3()
              ),
              (None, true) => {
                let (position, normal) = self::rounded(padded, at);
                (position, normal.unwrap_or(face.normal.as_vec3()))
              }
              (None, false) => (flat, face.normal.as_vec3())
            };
            let shade = blend(shades, u, v);
            shading.push(shade);
            builder.positions.push(position.to_array());
            builder.normals.push(normal.to_array());
            builder.uvs.push(
              match projected {
                Some(_) => tile_origin(block.tiles()[0]),
                None => uv_corner(tile, Vec2::new(u, 1.0 - v))
              }
              .to_array()
            );
            let hue = match (kind(block), block) {
              (Some(Fluid::Water), _) => tint(flat),
              (_, Block::Grass) => meadow(origin.as_vec3() + flat),
              _ => LinearRgba::WHITE
            };
            let torch = torchlight(origin.as_vec3() + position);
            builder.colors.push(lit(hue, blend(skies, u, v), torch, shade));
            builder.sways.push([
              owned + f32::from(u8::from(block.leafy() && only.is_none())),
              projected.unwrap_or(0.0)
            ])
          }
        }
        let width = across.len();
        for row in 0..up.len() - 1 {
          for column in 0..width - 1 {
            let corners = [
              row * width + column,
              row * width + column + 1,
              (row + 1) * width + column + 1,
              (row + 1) * width + column
            ];
            let placed = corners
              .map(|corner| Vec3::from(builder.positions[first as usize + corner]));
            let shade = corners.map(|corner| shading[corner]);
            let facing = |[a, b, c]: [usize; 3]| {
              (placed[b] - placed[a])
                .cross(placed[c] - placed[a])
                .dot(face.normal.as_vec3())
                > 0.0
            };
            let (straight, crossed) = (
              facing([0, 1, 2]) && facing([0, 2, 3]),
              facing([1, 2, 3]) && facing([1, 3, 0])
            );
            let order = match (straight, crossed) {
              (false, true) => [1, 2, 3, 1, 3, 0],
              (true, false) => [0, 1, 2, 0, 2, 3],
              _ if shade[1] + shade[3] > shade[0] + shade[2] => [1, 2, 3, 1, 3, 0],
              _ => [0, 1, 2, 0, 2, 3]
            };
            builder.indices.extend(order.map(|corner| first + corners[corner] as u32))
          }
        }
      }
      let shift = model::shift(block, seed, origin + local);
      let pose = model::pose(block);
      let modelled = match block.torch() {
        true => &mut unshadowed,
        false => &mut cutout
      };
      let light = sky(local);
      let torch = torchlight(origin.as_vec3() + local.as_vec3() + 0.5);
      let links = model::links(block, |side| padded.get(local + side));
      for &Bit { low, high, color: [r, g, b], tile } in model::shape(block, links).iter()
      {
        let (low, high) =
          (Vec3::from(low.map(f32::from)), Vec3::from(high.map(f32::from)));
        let hue = LinearRgba::from_f32_array(lit(
          LinearRgba::from(Color::srgb(r, g, b)),
          light,
          torch,
          1.0
        ));
        for face in FACES.iter() {
          let texels = CORNERS.map(|(a, b)| {
            low + (high - low) * (face.base + face.across * a + face.up * b).as_vec3()
          });
          modelled.polygon(
            &texels.map(|texel| {
              local.as_vec3() + (pose.transform_point3(texel) + shift) / 16.0
            }),
            &texels.map(|texel| {
              let (u, v) = (
                texel.dot(face.across.abs().as_vec3()),
                texel.dot(face.up.abs().as_vec3())
              );
              uv_corner(tile, Vec2::new(u / 16.0, 1.0 - v / 16.0))
            }),
            pose.transform_vector3(face.normal.as_vec3()),
            hue,
            owned,
            |index| match block.plant() {
              true => texels[index].y / 16.0,
              false => 0.0
            }
          )
        }
      }
      if block.look() == Look::Log {
        let [top, side, bottom] = block.tiles();
        let hue = LinearRgba::from_f32_array(lit(LinearRgba::WHITE, light, torch, 1.0));
        let corner = local.as_vec3();
        let ring = trunk();
        for edge in 0..TRUNK_SIDES {
          let (from, to) = (ring[edge], ring[(edge + 1) % TRUNK_SIDES]);
          let middle = ((from + to) / 2.0 - 0.5).normalize();
          let facing = Vec3::new(middle.x, 0.0, middle.y);
          let covered = facing.abs().max_element() > 0.9
            && padded.get(local + facing.round().as_ivec3()).opaque();
          if !covered {
            let quarter = (edge % (TRUNK_SIDES / 4)) as f32 / (TRUNK_SIDES / 4) as f32;
            let width = 4.0 / TRUNK_SIDES as f32;
            let outward = |point: Vec2| {
              let out = (point - 0.5).normalize();
              Vec3::new(out.x, 0.0, out.y)
            };
            cutout.strip(
              [
                (from, 0.0, quarter, 1.0),
                (to, 0.0, quarter + width, 1.0),
                (to, 1.0, quarter + width, 0.0),
                (from, 1.0, quarter, 0.0)
              ]
              .map(|(point, y, u, v)| {
                (
                  corner + Vec3::new(point.x, y, point.y),
                  outward(point),
                  uv_corner(side, Vec2::new(u, v))
                )
              }),
              hue,
              owned
            )
          }
        }
        for (y, normal, tile) in
          [(1.0, IVec3::Y, top), (0.0, IVec3::NEG_Y, bottom)].into_iter()
        {
          let next = padded.get(local + normal);
          if !next.opaque() && next.look() != Look::Log {
            cutout.polygon(
              &ring.map(|point| corner + Vec3::new(point.x, y, point.y)),
              &ring.map(|point| uv_corner(tile, point)),
              normal.as_vec3(),
              hue,
              owned,
              |_| 0.0
            )
          }
        }
      }
    }
  }
  (solid, cutout, liquid, unshadowed, hull)
}

pub fn build(padded: &Padded, key: IVec3, seed: u32, torches: &[IVec3]) -> Meshes {
  let (solid, cutout, liquid, unshadowed, hull) =
    builders(padded, key, seed, torches, None, true);
  Meshes {
    solid: solid.mesh(),
    cutout: cutout.mesh(),
    liquid: liquid.mesh(),
    unshadowed: unshadowed.mesh(),
    hull
  }
}

#[derive(Clone, Copy)]
pub struct Corner {
  pub position: Vec3,
  pub normal: Vec3,
  pub uv: Vec2,
  pub color: [f32; 4],
  pub sway: [f32; 2]
}

pub fn lone(
  block: Block,
  at: IVec3,
  seed: u32,
  torches: &[IVec3],
  around: Option<&Voxels>
) -> Vec<[Corner; 3]> {
  let key = chunk_of(at);
  let local = at - origin_of(key);
  let padded = Padded(
    (0..SPAN * SPAN * SPAN)
      .map(|index| {
        let cell = IVec3::new(index % SPAN, index / (SPAN * SPAN), index / SPAN % SPAN)
          - IVec3::splat(PAD);
        match (cell == local, around) {
          (true, _) => block,
          (false, Some(voxels)) => {
            voxels.block(origin_of(key) + cell).unwrap_or(Block::Air)
          }
          (false, None) => Block::Air
        }
      })
      .collect()
  );
  let (solid, cutout, _, unshadowed, _) =
    builders(&padded, key, seed, torches, Some(local), around.is_some());
  [solid, cutout, unshadowed]
    .iter()
    .flat_map(|built| {
      let corner = |index: u32| {
        let index = index as usize;
        Corner {
          position: Vec3::from(built.positions[index]) - local.as_vec3(),
          normal: Vec3::from(built.normals[index]),
          uv: Vec2::from(built.uvs[index]),
          color: built.colors[index],
          sway: built.sways[index]
        }
      };
      built
        .indices
        .chunks(3)
        .map(move |triangle| {
          [corner(triangle[0]), corner(triangle[1]), corner(triangle[2])]
        })
        .collect::<Vec<_>>()
    })
    .collect()
}

struct Blade;

impl Blade {
  const ACROSS: i32 = 5;
  const DOWN: i32 = 4;
  const CLUMPS: i32 = 3;
  const SHORTEST: f32 = 0.16;
  const TALLEST: f32 = 0.42;
  const NARROWEST: f32 = 0.055;
  const WIDEST: f32 = 0.1;
  const LEAN: f32 = 0.55;
  const GATHER: f32 = 0.35;
  const LEVELS: [f32; 4] = [0.0, 0.3, 0.58, 0.82];
}

struct Clump {
  at: Vec2,
  lean: Vec2,
  tall: f32,
  facing: f32
}

pub fn lawn(padded: &Padded, key: IVec3, seed: u32, torches: &[IVec3]) -> Option<Mesh> {
  let origin = origin_of(key);
  let turf = {
    let [r, g, b] = crate::minimap::hue(Block::Grass);
    LinearRgba::from(Color::srgb(r, g, b))
  };
  let tops: Vec<IVec3> = (0..SIZE * SIZE * SIZE)
    .map(|index| IVec3::new(index % SIZE, index / (SIZE * SIZE), index / SIZE % SIZE))
    .filter(|&local| {
      let above = padded.get(local + IVec3::Y);
      padded.get(local) == Block::Grass && !above.solid() && above.liquid().is_none()
    })
    .collect();
  let (
    mut positions,
    mut normals,
    mut steps,
    mut seeds,
    mut colors,
    mut shapes,
    mut indices
  ) = (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
  for local in tops {
    let world = origin + local;
    let roll = |salt: i32, index: i32| {
      noise::unit(seed ^ 0x6A55, world.x * 64 + index, world.y * 16 + salt, world.z)
    };
    let depth = generate::height(seed, world.x, world.z) - world.y;
    let sky = (1.0 - depth as f32 / SKY_FALLOFF).clamp(CAVE_DARK, 1.0);
    let patch = (fbm2(0x9A55, world.x as f32 / 40.0, world.z as f32 / 40.0, 2) * 1.6
      + 0.5)
      .clamp(0.0, 1.0);
    let tint = {
      let field = meadow(world.as_vec3());
      LinearRgba::rgb(
        field.red * turf.red,
        field.green * turf.green,
        field.blue * turf.blue
      )
    };
    let torch = torchlight(torches, world.as_vec3() + Vec3::new(0.5, 1.0, 0.5));
    let [r, g, b, _] = lit(tint, sky, torch, 1.0);
    let owned = owner(local);
    let clumps: Vec<Clump> = (0..Blade::CLUMPS)
      .map(|index| Clump {
        at: Vec2::new(roll(0, index), roll(1, index)),
        lean: Vec2::from_angle(roll(2, index) * std::f32::consts::TAU)
          * roll(3, index)
          * Blade::LEAN,
        tall: (Blade::SHORTEST + (Blade::TALLEST - Blade::SHORTEST) * roll(4, index))
          * (0.7 + 0.6 * patch),
        facing: roll(5, index) * std::f32::consts::PI
      })
      .collect();
    for spot in 0..Blade::ACROSS * Blade::DOWN {
      let cell = Vec2::new(
        ((spot % Blade::ACROSS) as f32 + roll(6, spot)) / Blade::ACROSS as f32,
        ((spot / Blade::ACROSS) as f32 + roll(7, spot)) / Blade::DOWN as f32
      );
      let clump = clumps
        .iter()
        .min_by(|a, b| a.at.distance(cell).total_cmp(&b.at.distance(cell)))
        .expect("clumps");
      let spot_at =
        cell.lerp(clump.at, Blade::GATHER).clamp(Vec2::splat(0.02), Vec2::splat(0.98));
      let at = IVec3::new(
        local.x * UNIT + (spot_at.x * UNIT as f32) as i32,
        (local.y + 1) * UNIT,
        local.z * UNIT + (spot_at.y * UNIT as f32) as i32
      );
      let (root, normal) = rounded(padded, at);
      let ground = normal.unwrap_or(Vec3::Y);
      if ground.y > 0.3 {
        let lean =
          clump.lean + Vec2::from_angle(roll(8, spot) * std::f32::consts::TAU) * 0.15;
        let tall = clump.tall * (0.75 + 0.5 * roll(9, spot));
        let facing = clump.facing + (roll(10, spot) - 0.5) * 1.4;
        let wide = Blade::NARROWEST + (Blade::WIDEST - Blade::NARROWEST) * roll(11, spot);
        let blade_seed = roll(12, spot);
        let first = positions.len() as u32;
        let corners = Blade::LEVELS
          .iter()
          .flat_map(|&level| [(level, -1.0), (level, 1.0)])
          .chain([(1.0, 0.0)]);
        for (level, side) in corners {
          positions.push((root - ground * 0.03).to_array());
          normals.push(ground.to_array());
          steps.push([level, side]);
          seeds.push([owned, blade_seed]);
          colors.push([r, g, b, tall]);
          shapes.push([lean.x, lean.y, facing, wide]);
        }
        let quads = (0..Blade::LEVELS.len() as u32 - 1).flat_map(|level| {
          let (low, high) = (first + level * 2, first + level * 2 + 2);
          [low, low + 1, high + 1, low, high + 1, high]
        });
        let top = first + (Blade::LEVELS.len() as u32 - 1) * 2;
        indices.extend(quads.chain([top, top + 1, top + 2]))
      }
    }
  }
  (!indices.is_empty()).then(|| {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
      .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
      .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
      .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, steps)
      .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, seeds)
      .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
      .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, shapes)
      .with_inserted_indices(Indices::U32(indices))
  })
}

#[cfg(test)]
mod tests {
  use {super::*,
       crate::voxels::{Chunk, VOLUME}};

  #[test]
  fn flowing_water_steps_down() {
    let mut blocks = Box::new([Block::Air; VOLUME]);
    for x in 0..SIZE {
      for z in 0..SIZE {
        blocks[Chunk::index(IVec3::new(x, 4, z))] = Block::Stone
      }
    }
    for (x, block) in
      [Block::Water, Block::WaterFlow1, Block::WaterFlow2, Block::WaterFlow3]
        .into_iter()
        .enumerate()
    {
      blocks[Chunk::index(IVec3::new(x as i32 + 8, 5, 8))] = block
    }
    let chunk = Arc::new(Chunk::Mixed(blocks));
    let key = IVec3::new(500, 3, 500);
    let padded = Padded::gather(key, |near| {
      Some(if near == key { chunk.clone() } else { Arc::new(Chunk::Uniform(Block::Air)) })
    })
    .unwrap();
    let meshes = build(&padded, key, 1, &[]);
    let liquid = meshes.liquid.expect("water is meshed");
    let positions = match liquid.attribute(Mesh::ATTRIBUTE_POSITION) {
      Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) => positions.clone(),
      _ => panic!("no positions")
    };
    let mut tops: Vec<i32> =
      positions.iter().map(|p| (p[1] * 100.0).round() as i32).collect();
    tops.sort();
    tops.dedup();
    assert!(tops.len() >= 5, "heights {tops:?}");
  }
}
