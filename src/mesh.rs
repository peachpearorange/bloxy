use {crate::{block::{Block, Fluid, Look},
             generate,
             island::SEA,
             model::{self, Bit},
             noise::fbm2,
             texture::uv_corner,
             voxels::{Chunk, HEIGHT, SIZE, chunk_of, origin_of}},
     bevy::{asset::RenderAssetUsages,
            mesh::{Indices, PrimitiveTopology},
            prelude::*},
     std::sync::Arc};

const PAD: i32 = 3;
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

const CUT: f32 = 5.0 / 16.0;

const OCTAGON: [Vec2; 8] = [
  Vec2::new(CUT, 0.0),
  Vec2::new(1.0 - CUT, 0.0),
  Vec2::new(1.0, CUT),
  Vec2::new(1.0, 1.0 - CUT),
  Vec2::new(1.0 - CUT, 1.0),
  Vec2::new(CUT, 1.0),
  Vec2::new(0.0, 1.0 - CUT),
  Vec2::new(0.0, CUT)
];

const CORNERS: [(i32, i32); 4] = [(0, 0), (1, 0), (1, 1), (0, 1)];

const LATTICE: i32 = SIZE + 1;
const MOST_SHIFT: f32 = 0.45;
const STEADY: f32 = 0.05;
const LEANING: f32 = 0.3;
const BLUR: [f32; 3] = [1.0, 0.6, 0.25];
const DEPTH: [(i32, f32); 4] = [(-1, 0.5), (0, 1.0), (1, 1.0), (2, 0.5)];

fn occupies(block: Block) -> bool {
  matches!(block.look(), Look::Opaque | Look::Liquid | Look::Log)
    || block.leafy()
    || block == Block::Glass
}

#[derive(Clone, Copy)]
struct Point {
  at: Vec3,
  normal: Vec3,
  soft: f32
}

struct Contour<'a> {
  padded: &'a Padded,
  points: Vec<Option<Point>>
}

impl<'a> Contour<'a> {
  fn new(padded: &'a Padded) -> Contour<'a> {
    Contour { padded, points: vec![None; (LATTICE * LATTICE * LATTICE) as usize] }
  }

  fn filled(&self, cell: IVec3, sunk: bool) -> bool {
    let block = self.padded.get(cell);
    occupies(block) && !(sunk && block.fluid())
  }

  fn bent(&self, inside: IVec3, out: IVec3, sunk: bool) -> Vec3 {
    let centre = inside.as_vec3() + 0.5 + out.as_vec3() * 0.5;
    let (across, up) = match out.abs() {
      IVec3::X => (IVec3::Y, IVec3::Z),
      IVec3::Y => (IVec3::X, IVec3::Z),
      _ => (IVec3::X, IVec3::Y)
    };
    let pull = DEPTH.iter().fold(Vec3::ZERO, |pull, &(depth, deep)| {
      (-2..=2).flat_map(|a| (-2..=2).map(move |b| (a, b))).fold(pull, |pull, (a, b)| {
        let cell = inside + out * depth + across * a + up * b;
        match self.filled(cell, sunk) {
          true => {
            let weight =
              deep * BLUR[a.unsigned_abs() as usize] * BLUR[b.unsigned_abs() as usize];
            pull + (cell.as_vec3() + 0.5 - centre) * weight
          }
          false => pull
        }
      })
    });
    (out.as_vec3() * LEANING - pull).normalize_or(out.as_vec3())
  }

  fn solve(&self, corner: IVec3) -> Point {
    let cells = (0..8).map(|index| {
      corner - IVec3::ONE + IVec3::new(index & 1, index >> 1 & 1, index >> 2)
    });
    let sunk = cells.clone().all(|cell| occupies(self.padded.get(cell)));
    let crossings: Vec<(Vec3, Vec3)> = cells
      .clone()
      .flat_map(|cell| {
        [IVec3::X, IVec3::Y, IVec3::Z]
          .into_iter()
          .filter(move |&axis| (cell + axis - corner).cmple(IVec3::ZERO).all())
          .map(move |axis| (cell, cell + axis))
      })
      .filter(|&(low, high)| self.filled(low, sunk) != self.filled(high, sunk))
      .map(|(low, high)| {
        let (inside, outside) = match self.filled(low, sunk) {
          true => (low, high),
          false => (high, low)
        };
        ((low + high).as_vec3() / 2.0 + 0.5, self.bent(inside, outside - inside, sunk))
      })
      .collect();
    let (soft, mass) = cells
      .map(|cell| self.padded.get(cell))
      .filter(|block| matches!(block.look(), Look::Opaque | Look::Cutout))
      .fold((0.0, 0.0), |(soft, mass), block| (soft + block.softness(), mass + 1.0));
    let at = corner.as_vec3();
    match crossings.is_empty() || mass == 0.0 {
      true => Point { at, normal: Vec3::Y, soft: 0.0 },
      false => {
        let middle = crossings.iter().map(|&(point, _)| point).sum::<Vec3>()
          / crossings.len() as f32;
        let (weights, aims) = crossings.iter().fold(
          (Mat3::from_diagonal(Vec3::splat(STEADY)), middle * STEADY),
          |(weights, aims), &(point, normal)| {
            (
              weights
                + Mat3::from_cols(
                  normal * normal.x,
                  normal * normal.y,
                  normal * normal.z
                ),
              aims + normal * normal.dot(point)
            )
          }
        );
        let best = (weights.inverse() * aims).clamp(at - MOST_SHIFT, at + MOST_SHIFT);
        let soft = soft / mass;
        Point {
          at: at + (best - at) * soft,
          normal: crossings
            .iter()
            .map(|&(_, normal)| normal)
            .sum::<Vec3>()
            .normalize_or(Vec3::Y),
          soft
        }
      }
    }
  }

  fn point(&mut self, corner: IVec3) -> Point {
    let index = ((corner.y * LATTICE + corner.z) * LATTICE + corner.x) as usize;
    match self.points[index] {
      Some(point) => point,
      None => {
        let point = self.solve(corner);
        self.points[index] = Some(point);
        point
      }
    }
  }
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
    bend: impl Fn(usize) -> f32
  ) {
    let first = self.positions.len() as u32;
    for (index, (point, uv)) in points.iter().zip(uvs).enumerate() {
      self.sways.push([0.0, bend(index)]);
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
  pub liquid: Option<Mesh>,
  pub unshadowed: Option<Mesh>
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

fn builders(
  padded: &Padded,
  key: IVec3,
  seed: u32,
  torches: &[IVec3]
) -> (Builder, Builder, Builder) {
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
  let torchlight = |point: Vec3| {
    torches
      .iter()
      .map(|torch| {
        let flame = torch.as_vec3() + Vec3::new(0.5, 0.7, 0.5);
        (1.0 - point.distance(flame) / TORCH_REACH).max(0.0)
      })
      .fold(0.0, f32::max)
      .powf(1.4)
  };
  let mut contour = Contour::new(padded);
  let mut solid = Builder::default();
  let mut liquid = Builder::default();
  let mut unshadowed = Builder::default();
  for index in 0..SIZE * SIZE * SIZE {
    let local = IVec3::new(index % SIZE, index / (SIZE * SIZE), index / SIZE % SIZE);
    let block = padded.get(local);
    if block != Block::Air {
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
        let builder = match block.liquid() {
          Some((Fluid::Water, _)) => &mut liquid,
          _ => &mut solid
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
            .iter()
            .sum::<f32>()
            / 4.0
        });
        let tile = block.tiles()[face.side];
        let first = builder.positions.len() as u32;
        let shades = occlusion.map(|shade| OCCLUSION[shade as usize]);
        for ((&(a, b), shade), daylight) in CORNERS.iter().zip(shades).zip(daylight) {
          let corner = local + face.base + face.across * a + face.up * b;
          let flat = corner.as_vec3();
          let (position, normal) = match kind(block) {
            Some(_) => (
              flat.with_y(match flat.y > local.y as f32 + 0.5 {
                true => local.y as f32 + top,
                false => local.y as f32 + floor
              }),
              face.normal.as_vec3()
            ),
            None => {
              let Point { at, normal, soft } = contour.point(corner);
              (at, face.normal.as_vec3().lerp(normal, soft).normalize_or(normal))
            }
          };
          builder.positions.push(position.to_array());
          builder.normals.push(normal.to_array());
          builder
            .uvs
            .push(uv_corner(tile, Vec2::new(a as f32, 1.0 - b as f32)).to_array());
          let hue = match (kind(block), block) {
            (Some(Fluid::Water), _) => tint(flat),
            (_, Block::Grass) => meadow(origin.as_vec3() + flat),
            _ => LinearRgba::WHITE
          };
          let torch = torchlight(origin.as_vec3() + position);
          builder.colors.push(lit(hue, daylight, torch, shade));
          builder.sways.push([f32::from(u8::from(block.leafy())), 0.0])
        }
        let order = match shades[1] + shades[3] > shades[0] + shades[2] {
          true => [1, 2, 3, 1, 3, 0],
          false => [0, 1, 2, 0, 2, 3]
        };
        builder.indices.extend(order.map(|corner| first + corner))
      }
      let shift = model::shift(block, seed, origin + local);
      let pose = model::pose(block);
      let modelled = match block.torch() {
        true => &mut unshadowed,
        false => &mut solid
      };
      let light = sky(local);
      let torch = torchlight(origin.as_vec3() + local.as_vec3() + 0.5);
      for &Bit { low, high, color: [r, g, b], tile } in model::bits(block).iter() {
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
        for edge in 0..8 {
          let (from, to) = (OCTAGON[edge], OCTAGON[(edge + 1) % 8]);
          let outward = ((from + to) / 2.0 - 0.5).normalize();
          let normal = Vec3::new(outward.x, 0.0, outward.y);
          let along_x = outward.y.abs() > outward.x.abs();
          let across = |point: Vec2| if along_x { point.x } else { point.y };
          let covered =
            edge % 2 == 0 && padded.get(local + normal.round().as_ivec3()).opaque();
          if !covered {
            solid.polygon(
              &[(from, 0.0), (to, 0.0), (to, 1.0), (from, 1.0)]
                .map(|(point, y)| corner + Vec3::new(point.x, y, point.y)),
              &[(from, 1.0), (to, 1.0), (to, 0.0), (from, 0.0)]
                .map(|(point, v)| uv_corner(side, Vec2::new(across(point), v))),
              normal,
              hue,
              |_| 0.0
            )
          }
        }
        for (y, normal, tile) in
          [(1.0, IVec3::Y, top), (0.0, IVec3::NEG_Y, bottom)].into_iter()
        {
          let next = padded.get(local + normal);
          if !next.opaque() && next.look() != Look::Log {
            solid.polygon(
              &OCTAGON.map(|point| corner + Vec3::new(point.x, y, point.y)),
              &OCTAGON.map(|point| uv_corner(tile, point)),
              normal.as_vec3(),
              hue,
              |_| 0.0
            )
          }
        }
      }
    }
  }
  (solid, liquid, unshadowed)
}

pub fn build(padded: &Padded, key: IVec3, seed: u32, torches: &[IVec3]) -> Meshes {
  let (solid, liquid, unshadowed) = builders(padded, key, seed, torches);
  Meshes { solid: solid.mesh(), liquid: liquid.mesh(), unshadowed: unshadowed.mesh() }
}

#[derive(Clone, Copy)]
pub struct Corner {
  pub position: Vec3,
  pub normal: Vec3,
  pub uv: Vec2,
  pub color: [f32; 4]
}

pub fn lone(block: Block, at: IVec3, seed: u32, torches: &[IVec3]) -> Vec<[Corner; 3]> {
  let key = chunk_of(at);
  let local = at - origin_of(key);
  let padded = Padded(
    (0..SPAN * SPAN * SPAN)
      .map(|index| {
        let cell = IVec3::new(index % SPAN, index / (SPAN * SPAN), index / SPAN % SPAN)
          - IVec3::splat(PAD);
        if cell == local { block } else { Block::Air }
      })
      .collect()
  );
  let (solid, _, unshadowed) = builders(&padded, key, seed, torches);
  [solid, unshadowed]
    .iter()
    .flat_map(|built| {
      let corner = |index: u32| {
        let index = index as usize;
        Corner {
          position: Vec3::from(built.positions[index]) - local.as_vec3(),
          normal: Vec3::from(built.normals[index]),
          uv: Vec2::from(built.uvs[index]),
          color: built.colors[index]
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
