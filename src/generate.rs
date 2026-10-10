use {crate::{block::Block,
             island::{Island, Kind, Rise, SEA, Wood},
             noise::{fbm2, hash, perlin2, perlin3, unit},
             voxels::{Chunk, SIZE, VOLUME, origin_of}},
     bevy::prelude::*};

const SNOW_LINE: f32 = 128.0;
const CAVE_GRID: i32 = 4;
const TREE_REACH: i32 = 5;
const TALLEST_GROWTH: i32 = 16;

#[derive(Clone, Copy)]
struct Surface {
  height: f32,
  island: Option<(Island, Rise)>,
  ice: bool
}

fn surface(seed: u32, islands: &[Island], x: i32, z: i32) -> Surface {
  let floor = SEA as f32 - 16.0
    + fbm2(seed.wrapping_add(32), x as f32 / 60.0, z as f32 / 60.0, 2) * 5.0;
  islands
    .iter()
    .filter_map(|island| island.rise(seed, x, z).map(|rise| (*island, rise)))
    .fold(Surface { height: floor, island: None, ice: false }, |below, (island, rise)| {
      let ice = below.ice || rise.floe;
      match rise.height > below.height {
        true => Surface { height: rise.height, island: Some((island, rise)), ice },
        false => Surface { ice, ..below }
      }
    })
}

pub fn height(seed: u32, x: i32, z: i32) -> i32 {
  let spot = IVec2::new(x, z);
  surface(seed, &Island::within(seed, spot, spot), x, z).height.floor() as i32
}

#[derive(Clone, Copy)]
struct Column {
  height: i32,
  top: Block,
  under: Block,
  pool: (Block, i32),
  ice: bool,
  island: Option<(Island, Rise)>
}

fn column(seed: u32, surface: Surface, steep: bool, x: i32, z: i32) -> Column {
  let Surface { height, island, ice } = surface;
  let height = height.floor() as i32;
  let kind = island.map(|(island, _)| island.kind);
  let beach = island.is_some_and(|(island, _)| island.beach);
  let inward = island.map_or(-1.0, |(_, rise)| rise.inward);
  let snow_line = SNOW_LINE + unit(seed ^ 0x5A0, x, 0, z) * 6.0;
  let bed = unit(seed ^ 0xBED, x / 6, 0, z / 6);
  let (top, under) = match kind {
    Some(Kind::Dunes) => (Block::Sand, Block::Sand),
    _ if height < SEA && beach && height >= SEA - 4 => (Block::Sand, Block::Sand),
    _ if height < SEA => match (bed, kind) {
      (bed, Some(Kind::Frost)) if bed < 0.5 => (Block::Gravel, Block::Gravel),
      (bed, _) if bed < 0.2 => (Block::Clay, Block::Clay),
      (bed, _) if bed < 0.6 => (Block::Sand, Block::Sand),
      _ => (Block::Gravel, Block::Gravel)
    },
    Some(Kind::Volcano) if island.is_some_and(|(_, rise)| rise.streak) => {
      (Block::Lava, Block::Basalt)
    }
    Some(Kind::Volcano) if inward > 0.3 => (Block::Basalt, Block::Basalt),
    Some(Kind::Frost) if height <= SEA + 1 => (Block::Gravel, Block::Gravel),
    _ if height <= SEA + 1 && beach => (Block::Sand, Block::Sand),
    _ if height as f32 > snow_line || kind == Some(Kind::Frost) => match steep {
      true => (Block::Stone, Block::Stone),
      false => (Block::Snow, Block::Dirt)
    },
    _ if steep && height > SEA + 24 => (Block::Stone, Block::Stone),
    Some(Kind::Mushroom) => (Block::Mycelium, Block::Dirt),
    _ => (Block::Grass, Block::Dirt)
  };
  let pool = island
    .filter(|(island, rise)| island.kind == Kind::Volcano && rise.inward > 0.7)
    .map_or((Block::Water, SEA), |(island, _)| (Block::Lava, island.crater()));
  Column { height, top, under, pool, ice, island }
}

fn tunnels(seed: u32, at: IVec3) -> f32 {
  let point = at.as_vec3();
  let first = perlin3(seed ^ 0xCA7E, point.x / 52.0, point.y / 30.0, point.z / 52.0);
  let second = perlin3(seed ^ 0x7E11, point.x / 52.0, point.y / 30.0, point.z / 52.0);
  let cavern = perlin3(seed ^ 0xCAFE, point.x / 90.0, point.y / 45.0, point.z / 90.0);
  (first * first + second * second).min(0.4 - cavern * 0.6)
}

const OPEN: f32 = 0.006;

#[derive(Clone, Copy)]
struct Strata {
  fold: f32,
  basement: f32,
  magma: Block,
  volcanic: bool
}

fn strata(seed: u32, x: i32, z: i32, ground: &Column) -> Strata {
  let (x, z) = (x as f32, z as f32);
  Strata {
    fold: fbm2(seed.wrapping_add(77), x / 80.0, z / 80.0, 2) * 16.0
      + perlin2(seed ^ 0xF01D, x / 23.0, z / 23.0) * 2.0,
    basement: 22.0 + fbm2(seed.wrapping_add(78), x / 90.0, z / 90.0, 2) * 9.0,
    magma: match perlin2(seed ^ 0x6A6, x / 160.0, z / 160.0) > 0.0 {
      true => Block::Granite,
      false => Block::Diorite
    },
    volcanic: ground.island.is_some_and(|(island, _)| island.kind == Kind::Volcano)
  }
}

fn intrusion(seed: u32, at: IVec3) -> f32 {
  let point = at.as_vec3();
  perlin3(seed ^ 0x6A61, point.x / 56.0, point.y / 36.0, point.z / 56.0)
    - (at.y - 30).max(0) as f32 * 0.006
}

fn rock(seed: u32, at: IVec3, strata: Strata, intruded: f32) -> Block {
  let layer = || {
    perlin3(
      seed ^ 0x57A7,
      (at.y as f32 + strata.fold) / 6.0,
      at.x as f32 / 300.0,
      at.z as f32 / 300.0
    ) * 1.4
  };
  match () {
    () if intruded > 0.28 => strata.magma,
    () if (at.y as f32) < strata.basement => Block::Slate,
    () if strata.volcanic => Block::Andesite,
    () => match layer() {
      layer if layer < -0.3 => Block::Limestone,
      layer if layer > 0.32 => Block::Andesite,
      layer if (0.08..0.17).contains(&layer) => Block::Slate,
      _ => Block::Stone
    }
  }
}

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Growth {
  Tree(Wood),
  RedShroom,
  BrownShroom
}

fn growth(seed: u32, x: i32, z: i32, ground: &Column) -> Option<(Growth, i32)> {
  ground.island.and_then(|(island, _)| {
    let pick = unit(seed ^ 0x7EF, x, 1, z);
    let either = |first: Wood, second: Wood| match pick < 0.6 {
      true => Growth::Tree(first),
      false => Growth::Tree(second)
    };
    let clearing = (island.stone.xz() - IVec2::new(x, z)).abs().max_element() <= 4;
    let above_sea = ground.height > SEA;
    let sown = match (island.kind, ground.top) {
      _ if clearing => None,
      (Kind::Mushroom, Block::Mycelium) => Some((
        match pick < 0.55 {
          true => Growth::RedShroom,
          false => Growth::BrownShroom
        },
        0.007
      )),
      (Kind::Frost, Block::Snow) => Some((Growth::Tree(Wood::Spruce), 0.025)),
      (_, Block::Grass | Block::Snow) if ground.height > SEA + 20 => {
        Some((Growth::Tree(Wood::Spruce), 0.03))
      }
      (Kind::Woods, Block::Grass) => Some((
        match pick < 0.75 {
          true => Growth::Tree(island.wood),
          false => either(Wood::Oak, Wood::Birch)
        },
        0.06
      )),
      (Kind::Volcano, Block::Grass | Block::Sand) => {
        Some((either(Wood::Palm, Wood::Oak), 0.012))
      }
      (Kind::Meadow | Kind::Peak, Block::Grass) => {
        Some((either(Wood::Oak, Wood::Birch), 0.012))
      }
      (Kind::Dunes, Block::Sand) if above_sea => Some((Growth::Tree(Wood::Palm), 0.01)),
      (_, Block::Sand) if island.warm && above_sea => {
        Some((Growth::Tree(Wood::Palm), 0.012))
      }
      _ => None
    };
    let grove =
      fbm2(seed.wrapping_add(9), x as f32 / 40.0, z as f32 / 40.0, 2) * 0.5 + 0.5;
    sown
      .filter(|&(_, density)| unit(seed ^ 0x7EE, x, 0, z) < density * (0.4 + grove * 1.2))
      .map(|(growth, _)| (growth, (hash(seed ^ 0x7A11, x, 0, z) % 3) as i32))
  })
}

fn grow(
  seed: u32,
  (growth, size): (Growth, i32),
  root: IVec3,
  mut put: impl FnMut(IVec3, Block)
) {
  let speck = |offset: IVec3, salt: u32| {
    let at = root + offset;
    unit(seed ^ salt, at.x, at.y, at.z)
  };
  let disc = |radius: i32| {
    (-radius..=radius).flat_map(move |dx| (-radius..=radius).map(move |dz| (dx, dz)))
  };
  match growth {
    Growth::Tree(Wood::Oak) => {
      let trunk = 4 + size;
      disc(2).for_each(|(dx, dz)| {
        (trunk - 2..=trunk + 1).for_each(|dy| {
          let corner = dx.abs() == 2 && dz.abs() == 2;
          let inside = match dy < trunk {
            true => !corner || speck(IVec3::new(dx, dy, dz), 0x1EAF) < 0.4,
            false => dx.abs() + dz.abs() <= 1
          };
          if inside {
            put(root + IVec3::new(dx, dy, dz), Block::Leaves)
          }
        })
      });
      (0..trunk).for_each(|dy| put(root + IVec3::Y * dy, Block::Log))
    }
    Growth::Tree(Wood::Birch) => {
      let trunk = 5 + size;
      disc(2).for_each(|(dx, dz)| {
        (trunk - 3..=trunk + 1).for_each(|dy| {
          let offset = IVec3::new(dx, dy, dz);
          let inside = match dy {
            dy if dy >= trunk => dx.abs() + dz.abs() <= 1,
            dy if dy == trunk - 1 => dx.abs().max(dz.abs()) <= 1,
            _ => dx.abs() + dz.abs() <= 3 && speck(offset, 0xB1C) < 0.85
          };
          if inside {
            put(root + offset, Block::BirchLeaves)
          }
        })
      });
      (0..trunk).for_each(|dy| put(root + IVec3::Y * dy, Block::BirchLog))
    }
    Growth::Tree(Wood::Spruce) => {
      let trunk = 6 + size * 2;
      disc(3).for_each(|(dx, dz)| {
        (2..=trunk + 1).for_each(|dy| {
          let tier = (trunk + 2 - dy) / 2 - (trunk - dy).rem_euclid(2);
          let radius = tier.clamp(0, 3);
          if dx * dx + dz * dz <= radius * radius + 1 - i32::from(radius == 0) {
            put(root + IVec3::new(dx, dy, dz), Block::SpruceLeaves)
          }
        })
      });
      (0..trunk).for_each(|dy| put(root + IVec3::Y * dy, Block::SpruceLog))
    }
    Growth::Tree(Wood::Palm) => {
      let trunk = 5 + size;
      let lean = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z]
        [(hash(seed ^ 0xBA1, root.x, root.y, root.z) % 4) as usize];
      let shift = |dy: i32| match dy {
        dy if dy >= trunk - 1 => 2,
        dy if dy >= trunk / 2 => 1,
        _ => 0
      };
      let crown = root + lean * shift(trunk) + IVec3::Y * trunk;
      put(crown, Block::PalmLeaves);
      put(crown + IVec3::Y, Block::PalmLeaves);
      disc(1).filter(|&(dx, dz)| dx != 0 || dz != 0).for_each(|(dx, dz)| {
        let way = IVec3::new(dx, 0, dz);
        let length = if dx != 0 && dz != 0 { 2 } else { 3 };
        (1..=length).for_each(|step| {
          let droop = if step == length { -1 } else { 0 };
          put(crown + way * step + IVec3::Y * droop, Block::PalmLeaves)
        })
      });
      (0..trunk)
        .for_each(|dy| put(root + lean * shift(dy) + IVec3::Y * dy, Block::PalmLog))
    }
    Growth::RedShroom => {
      let (stem, radius) = (5 + size * 2, 2 + size.min(1) + 1);
      disc(radius).for_each(|(dx, dz)| {
        let ring = dx.abs().max(dz.abs());
        let corner = dx.abs() == dz.abs() && ring >= radius - 1;
        if ring < radius && !(corner && ring == radius - 1) {
          put(root + IVec3::new(dx, stem, dz), Block::RedCap)
        }
        if ring == radius && dx.abs() != dz.abs() {
          (stem - 2..stem)
            .for_each(|dy| put(root + IVec3::new(dx, dy, dz), Block::RedCap))
        }
      });
      (0..stem).for_each(|dy| put(root + IVec3::Y * dy, Block::MushroomStem))
    }
    Growth::BrownShroom => {
      let (stem, radius) = (4 + size * 2, 3 + size.min(2));
      disc(radius)
        .filter(|&(dx, dz)| dx * dx + dz * dz <= radius * radius + 1)
        .for_each(|(dx, dz)| put(root + IVec3::new(dx, stem, dz), Block::BrownCap));
      (0..stem).for_each(|dy| put(root + IVec3::Y * dy, Block::MushroomStem))
    }
  }
}

fn bloom(seed: u32, x: i32, z: i32, ground: &Column) -> Option<Block> {
  ground.island.and_then(|(island, _)| {
    let pick = unit(seed ^ 0xF12, x, 3, z);
    let field =
      fbm2(seed.wrapping_add(41), x as f32 / 24.0, z as f32 / 24.0, 2) * 0.5 + 0.5;
    let flower = [Block::Poppy, Block::Dandelion, Block::Cornflower, Block::Daisy]
      [(hash(seed ^ 0xF11, x.div_euclid(7), 0, z.div_euclid(7)) % 4) as usize];
    let mushroom = match pick < 0.5 {
      true => Block::RedMushroom,
      false => Block::BrownMushroom
    };
    let sapling = match island.wood {
      Wood::Oak => Block::OakSapling,
      Wood::Birch => Block::BirchSapling,
      Wood::Spruce => Block::SpruceSapling,
      Wood::Palm => Block::PalmSapling
    };
    let sown = match (island.kind, ground.top) {
      (Kind::Meadow, Block::Grass) => Some((flower, 0.1)),
      (Kind::Woods, Block::Grass) => Some(match pick {
        pick if pick < 0.4 => (sapling, 0.03),
        pick if pick < 0.7 => (mushroom, 0.03),
        _ => (flower, 0.04)
      }),
      (Kind::Peak | Kind::Volcano, Block::Grass) => Some((flower, 0.04)),
      (Kind::Mushroom, Block::Mycelium) => Some((mushroom, 0.06)),
      (Kind::Frost, Block::Snow) => Some((Block::SpruceSapling, 0.005)),
      (Kind::Dunes, Block::Sand) if ground.height > SEA => {
        Some((Block::PalmSapling, 0.005))
      }
      _ => None
    };
    sown
      .filter(|&(_, density)| {
        unit(seed ^ 0xF10, x, 2, z) < density * (0.2 + field * field * 2.0)
      })
      .map(|(block, _)| block)
  })
}

fn trunk(block: Block) -> bool {
  matches!(
    block,
    Block::Log
      | Block::BirchLog
      | Block::SpruceLog
      | Block::PalmLog
      | Block::MushroomStem
  )
}

pub fn chunk(seed: u32, key: IVec3) -> Chunk {
  let origin = origin_of(key);
  let reach = SIZE + TREE_REACH * 2;
  let wide = reach + 2;
  let islands = Island::within(
    seed,
    origin.xz() - TREE_REACH - 1,
    origin.xz() + SIZE + TREE_REACH + 1
  );
  let surfaces: Vec<Surface> = (0..wide * wide)
    .map(|index| {
      let (x, z) = (index % wide - TREE_REACH - 1, index / wide - TREE_REACH - 1);
      surface(seed, &islands, origin.x + x, origin.z + z)
    })
    .collect();
  let surface_at = |x: i32, z: i32| {
    surfaces[((z + TREE_REACH + 1) * wide + x + TREE_REACH + 1) as usize]
  };
  let columns: Vec<Column> = (0..reach * reach)
    .map(|index| {
      let (x, z) = (index % reach - TREE_REACH, index / reach - TREE_REACH);
      let here = surface_at(x, z);
      let steep = [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().any(|(dx, dz)| {
        (surface_at(x + dx, z + dz).height.floor() - here.height.floor()).abs() >= 3.0
      });
      column(seed, here, steep, origin.x + x, origin.z + z)
    })
    .collect();
  let column_at =
    |x: i32, z: i32| columns[((z + TREE_REACH) * reach + x + TREE_REACH) as usize];
  let highest = columns.iter().map(|column| column.height.max(column.pool.1)).max();
  let lowest = columns.iter().map(|column| column.height).min().unwrap_or(0);
  match () {
    () if origin.y > highest.unwrap_or(0).max(SEA) + TALLEST_GROWTH => {
      Chunk::Uniform(Block::Air)
    }
    () => {
      let corners = SIZE / CAVE_GRID + 1;
      let grid = |field: fn(u32, IVec3) -> f32| -> Vec<f32> {
        (0..corners * corners * corners)
          .map(|index| {
            let corner = IVec3::new(
              index % corners,
              index / (corners * corners),
              index / corners % corners
            );
            field(seed, origin + corner * CAVE_GRID)
          })
          .collect()
      };
      let carved = grid(tunnels);
      let intrusions = grid(intrusion);
      let blended = |samples: &[f32], local: IVec3| {
        let cell = local / CAVE_GRID;
        let blend = (local % CAVE_GRID).as_vec3() / CAVE_GRID as f32;
        let sample = |offset: IVec3| {
          let corner = cell + offset;
          samples[((corner.y * corners + corner.z) * corners + corner.x) as usize]
        };
        let along_x = |y: i32, z: i32| {
          sample(IVec3::new(0, y, z)).lerp(sample(IVec3::new(1, y, z)), blend.x)
        };
        let along_z = |y: i32| along_x(y, 0).lerp(along_x(y, 1), blend.z);
        along_z(0).lerp(along_z(1), blend.y)
      };
      let carving = |local: IVec3| blended(&carved, local);
      let layers: Vec<Strata> = (0..SIZE * SIZE)
        .map(|index| {
          let (x, z) = (index % SIZE, index / SIZE);
          strata(seed, origin.x + x, origin.z + z, &column_at(x, z))
        })
        .collect();
      let mut blocks = Box::new([Block::Air; VOLUME]);
      (0..VOLUME as i32).for_each(|index| {
        let local = IVec3::new(index % SIZE, index / (SIZE * SIZE), index / SIZE % SIZE);
        let at = origin + local;
        let ground = column_at(local.x, local.z);
        let depth = ground.height - at.y;
        let bedrock =
          at.y == 0 || (at.y < 4 && unit(seed ^ 0xB0, at.x, at.y, at.z) < 0.5);
        let wet = ground.height <= SEA + 2
          || ground.top == Block::Lava
          || ground.pool.0 == Block::Lava;
        let open = at.y > 4
          && depth >= 0
          && (!wet || depth > 5)
          && at.y < lowest.max(SEA) + 40
          && carving(local) < OPEN;
        let (pool, level) = ground.pool;
        let block = match () {
          () if bedrock => Block::Bedrock,
          () if depth < 0 && at.y == SEA && ground.ice => Block::Ice,
          () if depth < 0 && at.y <= level => pool,
          () if depth < 0 || open => Block::Air,
          () if depth == 0 => ground.top,
          () if depth < 4 => ground.under,
          () => Block::Stone
        };
        blocks[Chunk::index(local)] = match block {
          Block::Stone => rock(
            seed,
            at,
            layers[(local.z * SIZE + local.x) as usize],
            blended(&intrusions, local)
          ),
          other => other
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
              if slot.rock() && at.as_vec3().distance(centre) <= ragged {
                *slot = ore
              }
            })
          })
        })
      });
      let inside = |local: IVec3| {
        local.cmpge(IVec3::ZERO).all() && local.cmplt(IVec3::splat(SIZE)).all()
      };
      (-TREE_REACH..SIZE + TREE_REACH).for_each(|x| {
        (-TREE_REACH..SIZE + TREE_REACH).for_each(|z| {
          let ground = column_at(x, z);
          if let Some(sprout) = growth(seed, origin.x + x, origin.z + z, &ground)
            && ground.height + TALLEST_GROWTH >= origin.y
            && ground.height < origin.y + SIZE
          {
            grow(
              seed,
              sprout,
              IVec3::new(origin.x + x, ground.height + 1, origin.z + z),
              |at, block| {
                let local = at - origin;
                if inside(local) {
                  let slot = &mut blocks[Chunk::index(local)];
                  if *slot == Block::Air || (trunk(block) && slot.leafy()) {
                    *slot = block
                  }
                }
              }
            )
          }
        })
      });
      (0..SIZE).for_each(|x| {
        (0..SIZE).for_each(|z| {
          let ground = column_at(x, z);
          let local = IVec3::new(x, ground.height + 1 - origin.y, z);
          if let Some(plant) = bloom(seed, origin.x + x, origin.z + z, &ground)
            && inside(local)
            && blocks[Chunk::index(local)] == Block::Air
            && (local.y == 0 || blocks[Chunk::index(local - IVec3::Y)] == ground.top)
          {
            blocks[Chunk::index(local)] = plant
          }
        })
      });
      islands.iter().for_each(|island| {
        let stone = island.stone;
        (-2..=2).for_each(|dx| {
          (-2..=2).for_each(|dz| {
            let local = stone - origin + IVec3::new(dx, 0, dz);
            if (0..SIZE).contains(&local.x) && (0..SIZE).contains(&local.z) {
              let ground = column_at(local.x, local.z).height;
              (ground.min(stone.y - 1)..=stone.y + 4).for_each(|y| {
                let cell = IVec3::new(local.x, y - origin.y, local.z);
                let block = match y - stone.y {
                  0 if dx == 0 && dz == 0 => Block::Waystone,
                  1 if dx == 0 && dz == 0 => Block::WaystoneTop,
                  -1 => Block::Cobblestone,
                  rise if rise < -1 => Block::Stone,
                  _ => Block::Air
                };
                if inside(cell) {
                  blocks[Chunk::index(cell)] = block
                }
              })
            }
          })
        })
      });
      Chunk::Mixed(blocks).settled()
    }
  }
}

pub fn spawn_point(seed: u32) -> Vec3 {
  Island::at(seed, IVec2::ZERO).map_or(Vec3::new(0.5, 200.0, 0.5), |home| home.arrival())
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

  #[test]
  #[ignore]
  fn section() {
    let (span, tall, z) = (768, 160, 40);
    let chunks: Vec<(IVec3, Chunk)> = (-span / 2 / SIZE..span / 2 / SIZE)
      .flat_map(|x| (0..tall / SIZE).map(move |y| IVec3::new(x, y, z / SIZE)))
      .map(|key| (key, chunk(1, key)))
      .collect();
    let block = |x: i32, y: i32| {
      let at = IVec3::new(x, y, z);
      chunks
        .iter()
        .find(|(key, _)| *key == crate::voxels::chunk_of(at))
        .map_or(Block::Air, |(key, chunk)| chunk.get(at - origin_of(*key)))
    };
    let image: Vec<u8> = (0..span * tall)
      .flat_map(|index| {
        let (x, y) = (index % span - span / 2, tall - 1 - index / span);
        let [r, g, b, _] = crate::texture::paint(block(x, y).tiles()[1], 7, 7).color;
        let sky = block(x, y) == Block::Air;
        match sky {
          true => [200, 220, 240],
          false => [r, g, b].map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u8)
        }
      })
      .collect();
    let mut file = format!("P6 {span} {tall} 255\n").into_bytes();
    file.extend(image);
    std::fs::write("screenshots/section.ppm", file).unwrap()
  }

  #[test]
  #[ignore]
  fn map() {
    let (span, step) = (1280, 2);
    let pixels = span / step;
    let islands = Island::within(1, IVec2::splat(-span / 2), IVec2::splat(span / 2));
    let image: Vec<u8> = (0..pixels * pixels)
      .flat_map(|index| {
        let (x, z) = (index % pixels * step - span / 2, index / pixels * step - span / 2);
        let here = surface(1, &islands, x, z);
        let ground = column(1, here, false, x, z);
        let stone = islands
          .iter()
          .any(|island| (island.stone.xz() - IVec2::new(x, z)).abs().max_element() <= 3);
        let colour: [u8; 3] = match ground.top {
          _ if stone => [255, 0, 255],
          _ if ground.height < ground.pool.1 && ground.pool.0 == Block::Lava => {
            [255, 120, 0]
          }
          _ if ground.height < SEA && ground.ice => [200, 225, 245],
          _ if ground.height < SEA => [30, 60, 140],
          Block::Grass => [70, 130, 50],
          Block::Sand => [220, 205, 140],
          Block::Snow => [240, 240, 250],
          Block::Mycelium => [130, 100, 140],
          Block::Basalt => [50, 50, 55],
          Block::Lava => [255, 90, 0],
          Block::Gravel => [130, 130, 130],
          _ => [110, 110, 110]
        };
        let shade = (0.6 + (ground.height - SEA) as f32 / 120.0).clamp(0.5, 1.2);
        colour.map(|channel| (channel as f32 * shade).min(255.0) as u8)
      })
      .collect();
    let mut file = format!("P6 {pixels} {pixels} 255\n").into_bytes();
    file.extend(image);
    std::fs::write("screenshots/map.ppm", file).unwrap()
  }
}
