use {crate::{block::Block,
             noise::{fbm2, hash, perlin3, unit},
             voxels::{Chunk, SIZE, VOLUME, origin_of}},
     bevy::prelude::*};

pub const SEA: i32 = 62;
const SNOW_LINE: f32 = 150.0;
const CAVE_GRID: i32 = 4;
const TREE_REACH: i32 = 3;

fn smoothstep(from: f32, to: f32, value: f32) -> f32 {
  let t = ((value - from) / (to - from)).clamp(0.0, 1.0);
  t * t * (3.0 - 2.0 * t)
}

pub fn height(seed: u32, x: i32, z: i32) -> i32 {
  let (x, z) = (x as f32, z as f32);
  let continent = fbm2(seed, x / 700.0, z / 700.0, 4);
  let hills = fbm2(seed.wrapping_add(1), x / 140.0, z / 140.0, 4);
  let ridges =
    (1.0 - fbm2(seed.wrapping_add(2), x / 300.0, z / 300.0, 5).abs() * 1.6).max(0.0);
  let highland =
    smoothstep(0.0, 0.3, fbm2(seed.wrapping_add(3), x / 900.0, z / 900.0, 3));
  let rise = 68.0 + continent * 34.0 + hills * 9.0 + highland * ridges * ridges * 95.0;
  (rise as i32).clamp(6, 250)
}

#[derive(Clone, Copy)]
struct Column {
  height: i32,
  top: Block,
  under: Block
}

fn column(seed: u32, x: i32, z: i32) -> Column {
  let rise = height(seed, x, z);
  let steep = [(1, 0), (-1, 0), (0, 1), (0, -1)]
    .into_iter()
    .any(|(dx, dz)| (height(seed, x + dx, z + dz) - rise).abs() >= 3);
  let snow_line = SNOW_LINE + unit(seed ^ 0x5A0, x, 0, z) * 6.0;
  let bed = unit(seed ^ 0xBED, x / 6, 0, z / 6);
  let (top, under) = match rise {
    height if height < SEA - 2 => match bed {
      bed if bed < 0.2 => (Block::Clay, Block::Clay),
      bed if bed < 0.6 => (Block::Sand, Block::Sand),
      _ => (Block::Gravel, Block::Gravel)
    },
    height if height <= SEA + 1 => (Block::Sand, Block::Sand),
    height if height as f32 > snow_line => match steep {
      true => (Block::Stone, Block::Stone),
      false => (Block::Snow, Block::Dirt)
    },
    height if steep && height > 110 => (Block::Stone, Block::Stone),
    _ => (Block::Grass, Block::Dirt)
  };
  Column { height: rise, top, under }
}

fn tunnels(seed: u32, at: IVec3) -> f32 {
  let point = at.as_vec3();
  let first = perlin3(seed ^ 0xCA7E, point.x / 52.0, point.y / 30.0, point.z / 52.0);
  let second = perlin3(seed ^ 0x7E11, point.x / 52.0, point.y / 30.0, point.z / 52.0);
  let cavern = perlin3(seed ^ 0xCAFE, point.x / 90.0, point.y / 45.0, point.z / 90.0);
  (first * first + second * second).min(0.4 - cavern * 0.6)
}

const OPEN: f32 = 0.006;

struct Ore {
  block: Block,
  per_chunk: f32,
  depth: (i32, i32),
  radius: f32
}

const ORES: [Ore; 6] = [
  Ore { block: Block::CoalOre, per_chunk: 22.0, depth: (5, 200), radius: 1.7 },
  Ore { block: Block::CopperOre, per_chunk: 14.0, depth: (20, 110), radius: 1.5 },
  Ore { block: Block::TinOre, per_chunk: 10.0, depth: (10, 90), radius: 1.4 },
  Ore { block: Block::IronOre, per_chunk: 12.0, depth: (5, 80), radius: 1.4 },
  Ore { block: Block::GoldOre, per_chunk: 3.0, depth: (5, 40), radius: 1.2 },
  Ore { block: Block::DiamondOre, per_chunk: 1.5, depth: (5, 20), radius: 1.0 }
];

fn neighbours() -> impl Iterator<Item = IVec3> {
  (-1..=1)
    .flat_map(|x| (-1..=1).flat_map(move |y| (-1..=1).map(move |z| IVec3::new(x, y, z))))
}

fn veins(seed: u32, key: IVec3) -> impl Iterator<Item = (Block, Vec3, f32)> {
  neighbours().flat_map(move |offset| {
    let near = key + offset;
    ORES.iter().enumerate().flat_map(move |(kind, ore)| {
      let salt = seed ^ (0x0E5 + kind as u32 * 0x9F1);
      let tries = ore.per_chunk.floor() as i32
        + i32::from(unit(salt, near.x, near.y, near.z) < ore.per_chunk.fract());
      (0..tries).filter_map(move |attempt| {
        let pick = |axis: u32| {
          hash(salt ^ axis, near.x * 31 + attempt, near.y * 17 + attempt, near.z)
            % SIZE as u32
        };
        let centre = origin_of(near) + UVec3::new(pick(1), pick(2), pick(3)).as_ivec3();
        (ore.depth.0..ore.depth.1).contains(&centre.y).then_some((
          ore.block,
          centre.as_vec3(),
          ore.radius
        ))
      })
    })
  })
}

fn tree_at(seed: u32, x: i32, z: i32, ground: &Column) -> Option<i32> {
  let forest =
    (fbm2(seed.wrapping_add(9), x as f32 / 220.0, z as f32 / 220.0, 3) + 0.05) * 0.06;
  (ground.top == Block::Grass && unit(seed ^ 0x7EE, x, 0, z) < forest.max(0.002))
    .then(|| 4 + (hash(seed ^ 0x7A11, x, 0, z) % 3) as i32)
}

pub fn chunk(seed: u32, key: IVec3) -> Chunk {
  let origin = origin_of(key);
  let reach = SIZE + TREE_REACH * 2;
  let columns: Vec<Column> = (0..reach * reach)
    .map(|index| {
      let (x, z) = (index % reach - TREE_REACH, index / reach - TREE_REACH);
      column(seed, origin.x + x, origin.z + z)
    })
    .collect();
  let column_at =
    |x: i32, z: i32| columns[((z + TREE_REACH) * reach + x + TREE_REACH) as usize];
  let highest = columns.iter().map(|column| column.height).max().unwrap_or(0);
  let lowest = columns.iter().map(|column| column.height).min().unwrap_or(0);
  match () {
    () if origin.y > highest.max(SEA) + 10 => Chunk::Uniform(Block::Air),
    () => {
      let corners = SIZE / CAVE_GRID + 1;
      let carved: Vec<f32> = (0..corners * corners * corners)
        .map(|index| {
          let corner = IVec3::new(
            index % corners,
            index / (corners * corners),
            index / corners % corners
          );
          tunnels(seed, origin + corner * CAVE_GRID)
        })
        .collect();
      let carving = |local: IVec3| {
        let cell = local / CAVE_GRID;
        let blend = (local % CAVE_GRID).as_vec3() / CAVE_GRID as f32;
        let sample = |offset: IVec3| {
          let corner = cell + offset;
          carved[((corner.y * corners + corner.z) * corners + corner.x) as usize]
        };
        let along_x = |y: i32, z: i32| {
          sample(IVec3::new(0, y, z)).lerp(sample(IVec3::new(1, y, z)), blend.x)
        };
        let along_z = |y: i32| along_x(y, 0).lerp(along_x(y, 1), blend.z);
        along_z(0).lerp(along_z(1), blend.y)
      };
      let mut blocks = Box::new([Block::Air; VOLUME]);
      (0..VOLUME as i32).for_each(|index| {
        let local = IVec3::new(index % SIZE, index / (SIZE * SIZE), index / SIZE % SIZE);
        let at = origin + local;
        let ground = column_at(local.x, local.z);
        let depth = ground.height - at.y;
        let bedrock =
          at.y == 0 || (at.y < 4 && unit(seed ^ 0xB0, at.x, at.y, at.z) < 0.5);
        let open = at.y > 4
          && depth >= 0
          && (ground.height > SEA + 2 || depth > 5)
          && at.y < lowest.max(SEA) + 40
          && carving(local) < OPEN;
        blocks[Chunk::index(local)] = match () {
          () if bedrock => Block::Bedrock,
          () if depth < 0 && at.y <= SEA => Block::Water,
          () if depth < 0 || open => Block::Air,
          () if depth == 0 => ground.top,
          () if depth < 4 => ground.under,
          () => Block::Stone
        };
      });
      veins(seed, key).for_each(|(ore, centre, radius)| {
        let low = (centre - radius).floor().as_ivec3() - origin;
        let high = (centre + radius).ceil().as_ivec3() - origin;
        (low.max(IVec3::ZERO).y..=high.min(IVec3::splat(SIZE - 1)).y).for_each(|y| {
          (low.max(IVec3::ZERO).z..=high.min(IVec3::splat(SIZE - 1)).z).for_each(|z| {
            (low.max(IVec3::ZERO).x..=high.min(IVec3::splat(SIZE - 1)).x).for_each(|x| {
              let local = IVec3::new(x, y, z);
              let at = origin + local;
              let ragged = radius * (0.75 + unit(seed ^ 0x0AE, at.x, at.y, at.z) * 0.5);
              let slot = &mut blocks[Chunk::index(local)];
              if *slot == Block::Stone && at.as_vec3().distance(centre) <= ragged {
                *slot = ore
              }
            })
          })
        })
      });
      (-TREE_REACH..SIZE + TREE_REACH).for_each(|x| {
        (-TREE_REACH..SIZE + TREE_REACH).for_each(|z| {
          let ground = column_at(x, z);
          if let Some(trunk) = tree_at(seed, origin.x + x, origin.z + z, &ground)
            && ground.height + trunk + 2 >= origin.y
            && ground.height < origin.y + SIZE
          {
            let base = IVec3::new(x, ground.height + 1 - origin.y, z);
            let mut put = |local: IVec3, block: Block| {
              if local.cmpge(IVec3::ZERO).all() && local.cmplt(IVec3::splat(SIZE)).all() {
                let slot = &mut blocks[Chunk::index(local)];
                if *slot == Block::Air || (block == Block::Log && *slot == Block::Leaves)
                {
                  *slot = block
                }
              }
            };
            (-2..=2i32).for_each(|dx| {
              (-2..=2i32).for_each(|dz| {
                (trunk - 2..=trunk + 1).for_each(|dy| {
                  let wide = dy < trunk;
                  let corner = dx.abs() == 2 && dz.abs() == 2;
                  let inside = match wide {
                    true => {
                      !corner
                        || unit(seed ^ 0x1EAF, origin.x + x + dx, dy, origin.z + z + dz)
                          < 0.4
                    }
                    false => dx.abs() + dz.abs() <= 1
                  };
                  if inside {
                    put(base + IVec3::new(dx, dy, dz), Block::Leaves)
                  }
                })
              })
            });
            (0..trunk).for_each(|dy| put(base + IVec3::Y * dy, Block::Log))
          }
        })
      });
      Chunk::Mixed(blocks).settled()
    }
  }
}

pub fn spawn_point(seed: u32) -> Vec3 {
  let (x, z) = (0..400)
    .map(|step| {
      let turn = step as f32 * 2.4;
      let distance = (step as f32).sqrt() * 12.0;
      ((turn.cos() * distance) as i32, (turn.sin() * distance) as i32)
    })
    .find(|&(x, z)| {
      let ground = column(seed, x, z);
      let shaded = (-2..=2).any(|dx| {
        (-2..=2).any(|dz| {
          tree_at(seed, x + dx, z + dz, &column(seed, x + dx, z + dz)).is_some()
        })
      });
      ground.top == Block::Grass && !shaded
    })
    .unwrap_or((0, 0));
  Vec3::new(x as f32 + 0.5, height(seed, x, z) as f32 + 1.05, z as f32 + 0.5)
}

#[cfg(test)]
mod tests {
  use {super::*,
       crate::voxels::{Voxels, chunk_of}};

  #[test]
  fn spawn_stands_on_ground_in_open_air() {
    let at = spawn_point(1).floor().as_ivec3();
    let mut voxels = Voxels::new(1);
    let (feet, below) = (voxels.ensure(at), voxels.ensure(at - IVec3::Y));
    let head = voxels.ensure(at + IVec3::Y);
    assert!(
      !feet.solid() && !head.solid() && below.solid(),
      "{feet:?} {head:?} {below:?} at {at}"
    );
    assert_eq!(chunk_of(at).y, at.y / SIZE);
  }

  #[test]
  fn generation_is_repeatable_and_varied() {
    let key = IVec3::new(3, 1, -2);
    let (first, second) = (chunk(7, key), chunk(7, key));
    let blocks = |chunk: &Chunk| {
      (0..VOLUME as i32)
        .map(|index| {
          chunk.get(IVec3::new(index % SIZE, index / (SIZE * SIZE), index / SIZE % SIZE))
        })
        .collect::<Vec<_>>()
    };
    assert_eq!(blocks(&first), blocks(&second));
    let found = |wanted: Block| {
      (-3..3)
        .any(|x| (0..3).any(|y| blocks(&chunk(1, IVec3::new(x, y, 0))).contains(&wanted)))
    };
    [
      Block::Stone,
      Block::CoalOre,
      Block::IronOre,
      Block::Grass,
      Block::Air,
      Block::Bedrock
    ]
    .into_iter()
    .for_each(|wanted| assert!(found(wanted), "no {wanted:?}"));
  }
}
