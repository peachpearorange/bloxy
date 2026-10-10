use {crate::{block::{Block, Fluid, Look},
             generate,
             island::SEA,
             model::{self, Bit},
             texture::uv_corner,
             voxels::{Chunk, HEIGHT, SIZE, origin_of}},
     bevy::{asset::RenderAssetUsages,
            mesh::{Indices, PrimitiveTopology},
            prelude::*},
     std::sync::Arc};

const SPAN: i32 = SIZE + 2;
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
    let shifted = local + IVec3::ONE;
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
                - IVec3::ONE;
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
  indices: Vec<u32>
}

impl Builder {
  fn polygon(&mut self, points: &[Vec3], uvs: &[Vec2], normal: Vec3, hue: LinearRgba) {
    let first = self.positions.len() as u32;
    points.iter().zip(uvs).for_each(|(point, uv)| {
      self.positions.push(point.to_array());
      self.normals.push(normal.to_array());
      self.uvs.push(uv.to_array());
      self.colors.push(hue.with_alpha(1.0).to_f32_array())
    });
    let facing = (points[1] - points[0]).cross(points[2] - points[0]).dot(normal) > 0.0;
    (1..points.len() as u32 - 1).for_each(|corner| {
      self.indices.extend(match facing {
        true => [first, first + corner, first + corner + 1],
        false => [first, first + corner + 1, first + corner]
      })
    })
  }

  fn mesh(self) -> Option<Mesh> {
    (!self.indices.is_empty()).then(|| {
      Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    })
  }
}

pub struct Meshes {
  pub solid: Option<Mesh>,
  pub liquid: Option<Mesh>
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

pub const TORCH_REACH: f32 = 10.0;
const TORCH_GLOW: LinearRgba = LinearRgba::rgb(1.0, 0.72, 0.42);

fn lit(hue: LinearRgba, sky: f32, torch: f32, shade: f32) -> [f32; 4] {
  let channel = |hue: f32, warm: f32| hue * sky.max(torch * warm) * shade;
  [
    channel(hue.red, TORCH_GLOW.red),
    channel(hue.green, TORCH_GLOW.green),
    channel(hue.blue, TORCH_GLOW.blue),
    1.0
  ]
}

pub fn build(padded: &Padded, key: IVec3, seed: u32, torches: &[IVec3]) -> Meshes {
  let origin = origin_of(key);
  let surface: Vec<i32> = (0..SPAN * SPAN)
    .map(|index| {
      generate::height(seed, origin.x + index % SPAN - 1, origin.z + index / SPAN - 1)
    })
    .collect();
  let sky = |at: IVec3| {
    let depth = surface[((at.z + 1) * SPAN + at.x + 1) as usize] - origin.y - at.y;
    (1.0 - depth as f32 / SKY_FALLOFF).clamp(CAVE_DARK, 1.0)
  };
  let column = |x: i32, z: i32| surface[((z + 1) * SPAN + x + 1) as usize];
  let tint = |corner: Vec3| {
    let (x, z) = (corner.x as i32, corner.z as i32);
    let floor = [(x - 1, z - 1), (x, z - 1), (x - 1, z), (x, z)]
      .map(|(x, z)| column(x.clamp(-1, SIZE), z.clamp(-1, SIZE)))
      .iter()
      .sum::<i32>() as f32
      / 4.0;
    let deep = ((SEA as f32 - floor) / OCEAN_DEPTH).clamp(0.0, 1.0);
    SHALLOWS.mix(&OPEN_SEA, deep.sqrt())
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
  let mut solid = Builder::default();
  let mut liquid = Builder::default();
  (0..SIZE * SIZE * SIZE).for_each(|index| {
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
      FACES.iter().filter_map(|face| floor(face).map(|floor| (face, floor))).for_each(
        |(face, floor)| {
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
          CORNERS.iter().zip(occlusion).zip(daylight).for_each(
            |((&(a, b), shade), daylight)| {
              let corner = (local + face.base + face.across * a + face.up * b).as_vec3();
              let height = match corner.y > local.y as f32 + 0.5 {
                true => local.y as f32 + top,
                false => local.y as f32 + floor
              };
              builder.positions.push([corner.x, height, corner.z]);
              builder.normals.push(face.normal.as_vec3().to_array());
              builder
                .uvs
                .push(uv_corner(tile, Vec2::new(a as f32, 1.0 - b as f32)).to_array());
              let hue = match kind(block) {
                Some(Fluid::Water) => tint(corner),
                _ => LinearRgba::WHITE
              };
              let torch = torchlight(origin.as_vec3() + corner.with_y(height));
              builder.colors.push(lit(hue, daylight, torch, OCCLUSION[shade as usize]));
            }
          );
          let flipped = occlusion[1] + occlusion[3] > occlusion[0] + occlusion[2];
          let order = match flipped {
            true => [1, 2, 3, 1, 3, 0],
            false => [0, 1, 2, 0, 2, 3]
          };
          builder.indices.extend(order.map(|corner| first + corner))
        }
      );
      let shift = model::shift(block, seed, origin + local);
      let light = sky(local);
      let torch = torchlight(origin.as_vec3() + local.as_vec3() + 0.5);
      model::bits(block).iter().for_each(|&Bit { low, high, color: [r, g, b], tile }| {
        let (low, high) =
          (Vec3::from(low.map(f32::from)), Vec3::from(high.map(f32::from)));
        let hue = LinearRgba::from_f32_array(lit(
          LinearRgba::from(Color::srgb(r, g, b)),
          light,
          torch,
          1.0
        ));
        FACES.iter().for_each(|face| {
          let texels = CORNERS.map(|(a, b)| {
            low + (high - low) * (face.base + face.across * a + face.up * b).as_vec3()
          });
          solid.polygon(
            &texels.map(|texel| local.as_vec3() + (texel + shift) / 16.0),
            &texels.map(|texel| {
              let (u, v) = (
                texel.dot(face.across.abs().as_vec3()),
                texel.dot(face.up.abs().as_vec3())
              );
              uv_corner(tile, Vec2::new(u / 16.0, 1.0 - v / 16.0))
            }),
            face.normal.as_vec3(),
            hue
          )
        })
      });
      if block.look() == Look::Log {
        let [top, side, bottom] = block.tiles();
        let hue = LinearRgba::from_f32_array(lit(LinearRgba::WHITE, light, torch, 1.0));
        let corner = local.as_vec3();
        (0..8).for_each(|edge| {
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
              hue
            )
          }
        });
        [(1.0, IVec3::Y, top), (0.0, IVec3::NEG_Y, bottom)].into_iter().for_each(
          |(y, normal, tile)| {
            let next = padded.get(local + normal);
            if !next.opaque() && next.look() != Look::Log {
              solid.polygon(
                &OCTAGON.map(|point| corner + Vec3::new(point.x, y, point.y)),
                &OCTAGON.map(|point| uv_corner(tile, point)),
                normal.as_vec3(),
                hue
              )
            }
          }
        )
      }
    }
  });
  Meshes { solid: solid.mesh(), liquid: liquid.mesh() }
}
