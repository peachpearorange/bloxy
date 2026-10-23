use {crate::{block::Block,
             climate::{self, ARID, FROZEN, chilled},
             dungeon::Dungeon,
             island::{CELL, Island, Kind, Rise, SEA, SEEP_CLEARANCE, Wood, coined,
                      landmass},
             noise::{fbm2, hash, perlin2, perlin3, smoothstep, unit},
             protocol::REACH,
             voxels::{Chunk, SIZE, VOLUME, origin_of}},
     bevy::{platform::collections::HashMap, prelude::*},
     std::sync::{LazyLock, RwLock}};

const CAVE_GRID: i32 = 4;
const TREE_REACH: i32 = 5;
const SEEP_REACH: i32 = 3;
const COMPASS: [IVec2; 8] = [
  IVec2::new(1, 0),
  IVec2::new(1, 1),
  IVec2::new(0, 1),
  IVec2::new(-1, 1),
  IVec2::new(-1, 0),
  IVec2::new(-1, -1),
  IVec2::new(0, -1),
  IVec2::new(1, -1)
];
const TALLEST_GROWTH: i32 = 16;

#[derive(Clone, Copy)]
struct Surface {
  height: f32,
  island: Option<(Island, Rise)>,
  ice: bool
}

fn surface(seed: u32, islands: &[Island], x: i32, z: i32) -> Surface {
  let deep = smoothstep(0.12, -0.16, landmass(seed, Vec2::new(x as f32, z as f32)));
  let floor = SEA as f32 - 9.0 - deep * 18.0
    + fbm2(seed.wrapping_add(32), x as f32 / 60.0, z as f32 / 60.0, 2) * 4.0;
  let mainland = climate::mainland(seed, Vec2::new(x as f32, z as f32));
  islands
    .iter()
    .filter_map(|island| island.rise(seed, x, z).map(|rise| (*island, rise)))
    .fold(
      Surface { height: floor.max(mainland), island: None, ice: false },
      |below, (island, rise)| {
        let ice = below.ice || rise.floe;
        match rise.height > below.height {
          true => Surface { height: rise.height, island: Some((island, rise)), ice },
          false => Surface { ice, ..below }
        }
      }
    )
}

pub const GENERATION: u32 = 9;

fn seeping(seed: u32, seep: IVec2, column: IVec2) -> bool {
  let ragged = 1.4 + unit(seed ^ 0x0115, column.x, 0, column.y) * 1.6;
  (column - seep).as_vec2().length() <= ragged
}

pub fn overview(seed: u32, x: i32, z: i32) -> (Block, i32, i32) {
  let spot = IVec2::new(x, z);
  let islands = Island::within(seed, spot, spot);
  let ground = column(seed, surface(seed, &islands, x, z), false, x, z);
  let (fluid, level) = ground.pool;
  let seeped =
    islands.iter().filter_map(|island| island.seep).any(|seep| seeping(seed, seep, spot));
  match () {
    () if seeped => (Block::OilSeep, ground.height, ground.height),
    () if ground.height < level && ground.ice => (Block::Ice, level, ground.height),
    () if ground.height < level => (fluid, level, ground.height),
    () => (ground.top, ground.height, ground.height)
  }
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
  island: Option<(Island, Rise)>,
  heat: f32,
  wet: f32
}

fn column(seed: u32, surface: Surface, steep: bool, x: i32, z: i32) -> Column {
  let Surface { height, island, ice } = surface;
  let height = height.floor() as i32;
  let point = Vec2::new(x as f32, z as f32);
  let kind = island.map(|(island, _)| island.kind);
  let beach = island.map_or_else(
    || fbm2(seed.wrapping_add(71), point.x / 70.0, point.y / 70.0, 2) > -0.15,
    |(island, _)| island.beach
  );
  let inward = island.map_or(-1.0, |(_, rise)| rise.inward);
  let warmth = climate::heat(seed, point);
  let wet = climate::wet(seed, point);
  let heat = chilled(warmth, height as f32) + (unit(seed ^ 0x5A0, x, 0, z) - 0.5) * 0.05;
  let frozen = heat < FROZEN || kind == Some(Kind::Frost);
  let bed = {
    let (fx, fz) = (x as f32, z as f32);
    let warp = perlin2(seed ^ 0xBEE, fx * 0.043, fz * 0.043) * 7.0;
    let swirl = perlin2(seed ^ 0xBEF, fx * 0.043, fz * 0.043) * 7.0;
    (fbm2(seed ^ 0xBED, (fx + warp) * 0.05, (fz + swirl) * 0.05, 3) * 1.5 + 0.5)
      .clamp(0.0, 1.0)
  };
  let (top, under) = match kind {
    Some(Kind::Dunes) => (Block::Sand, Block::Sand),
    _ if height < SEA && beach && height >= SEA - 4 && !frozen => {
      (Block::Sand, Block::Sand)
    }
    _ if height < SEA => match bed {
      bed if frozen && bed < 0.5 => (Block::Gravel, Block::Gravel),
      bed if bed < 0.2 => (Block::Clay, Block::Clay),
      bed if bed < 0.6 => (Block::Sand, Block::Sand),
      _ => (Block::Gravel, Block::Gravel)
    },
    Some(Kind::Volcano) if island.is_some_and(|(_, rise)| rise.streak) => {
      (Block::Lava, Block::Basalt)
    }
    Some(Kind::Volcano) if inward > 0.3 => (Block::Basalt, Block::Basalt),
    _ if frozen && height <= SEA => (Block::Gravel, Block::Gravel),
    _ if height <= SEA + 1 && beach => (Block::Sand, Block::Sand),
    _ if frozen => match steep {
      true => (Block::Stone, Block::Stone),
      false => (Block::Snow, Block::Dirt)
    },
    _ if steep && height > SEA + 24 => (Block::Stone, Block::Stone),
    Some(Kind::Mushroom) => (Block::Mycelium, Block::Dirt),
    _ if heat > ARID && wet < 0.0 => (Block::Sand, Block::Sand),
    _ => (Block::Grass, Block::Dirt)
  };
  let pack = height < SEA
    && warmth < FROZEN - 0.12
    && perlin2(seed.wrapping_add(70), point.x / 11.0, point.y / 11.0)
      + (FROZEN - 0.12 - warmth) * 2.5
      > 0.3;
  let pool = island
    .filter(|(island, rise)| island.kind == Kind::Volcano && rise.inward > 0.7)
    .map_or((Block::Water, SEA), |(island, _)| (Block::Lava, island.crater()));
  Column { height, top, under, pool, ice: ice || pack, island, heat, wet }
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

const ORES: [Ore; 7] = [
  Ore { block: Block::CoalOre, per_chunk: 22.0, depth: (5, 200), radius: 1.7 },
  Ore { block: Block::CopperOre, per_chunk: 14.0, depth: (20, 110), radius: 1.5 },
  Ore { block: Block::TinOre, per_chunk: 10.0, depth: (10, 90), radius: 1.4 },
  Ore { block: Block::IronOre, per_chunk: 12.0, depth: (5, 80), radius: 1.4 },
  Ore { block: Block::GoldOre, per_chunk: 3.0, depth: (5, 40), radius: 1.2 },
  Ore { block: Block::DiamondOre, per_chunk: 1.5, depth: (5, 20), radius: 1.0 },
  Ore { block: Block::PlatinumOre, per_chunk: 2.5, depth: (5, 32), radius: 1.1 }
];

fn platinum_bearing(seed: u32, islands: &[Island], centre: Vec3) -> bool {
  islands.iter().any(|island| {
    matches!(island.kind, Kind::Peak | Kind::Frost)
      && centre.xz().distance(island.centre) <= island.radius * 1.2
  }) || climate::heat(seed, centre.xz()) < FROZEN
    || climate::mainland(seed, centre.xz()) > SEA as f32 + 45.0
}

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

fn growth(
  seed: u32,
  x: i32,
  z: i32,
  ground: &Column,
  clearing: bool
) -> Option<(Growth, i32)> {
  let pick = unit(seed ^ 0x7EF, x, 1, z);
  let &Column { heat, wet, top, height, island, .. } = ground;
  let kind = island.map(|(island, _)| island.kind);
  let native = Wood::native(heat, pick);
  let forest = smoothstep(-0.25, 0.45, wet);
  let sown = match (kind, top) {
    _ if clearing || height <= SEA => None,
    (Some(Kind::Mushroom), Block::Mycelium) => Some((
      match pick < 0.55 {
        true => Growth::RedShroom,
        false => Growth::BrownShroom
      },
      0.007
    )),
    (_, Block::Snow) => Some((Growth::Tree(Wood::Spruce), 0.008 + 0.025 * forest)),
    (Some(Kind::Woods), Block::Grass) => Some((
      Growth::Tree(match pick < 0.6 {
        true => island.map_or(native, |(island, _)| island.wood),
        false => native
      }),
      0.06
    )),
    (Some(Kind::Meadow), Block::Grass) => {
      Some((Growth::Tree(native), 0.004 + 0.012 * forest))
    }
    (_, Block::Grass) => Some((Growth::Tree(native), 0.004 + 0.05 * forest * forest)),
    (_, Block::Sand) if heat > 0.2 => {
      Some((Growth::Tree(Wood::Palm), 0.005 + 0.01 * forest))
    }
    _ => None
  };
  let grove = fbm2(seed.wrapping_add(9), x as f32 / 40.0, z as f32 / 40.0, 2) * 0.5 + 0.5;
  sown
    .filter(|&(_, density)| unit(seed ^ 0x7EE, x, 0, z) < density * (0.4 + grove * 1.2))
    .map(|(growth, _)| (growth, (hash(seed ^ 0x7A11, x, 0, z) % 3) as i32))
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
  let clump = |centre: IVec3, wide: i32, tall: i32, salt: u32| {
    (-tall..=tall).flat_map(move |dy| {
      (-wide..=wide).flat_map(move |dx| {
        (-wide..=wide).filter_map(move |dz| {
          let offset = IVec3::new(dx, dy, dz);
          let across = (dx * dx + dz * dz) as f32 / ((wide * wide) as f32 + 0.7);
          let up = (dy * dy) as f32 / ((tall * tall) as f32 + 0.7);
          let reach = across + up;
          let at = centre + offset;
          let roll = unit(seed ^ salt, at.x, at.y, at.z);
          (reach <= 1.0 && (reach < 0.55 || roll < 0.62)).then_some(at)
        })
      })
    })
  };
  match growth {
    Growth::Tree(Wood::Oak) => {
      let trunk = 4 + size;
      let spokes = 2 + (hash(seed ^ 0x0A4, root.x, root.y, root.z) % 2) as i32;
      let first = hash(seed ^ 0x0A5, root.x, root.y, root.z) as usize;
      let ends: Vec<IVec3> = (0..spokes)
        .map(|spoke| {
          let way = COMPASS[(first + spoke as usize * 8 / spokes as usize) % 8];
          let reach = 2 + i32::from(speck(way.extend(spoke), 0x0A6) < 0.5);
          let fork = trunk - 1 + i32::from(speck(way.extend(spoke), 0x0A7) < 0.5);
          (fork, way, reach)
        })
        .map(|(fork, way, reach)| {
          for step in 1..=reach {
            put(
              root + IVec3::new(way.x * step, fork + step / 2, way.y * step),
              Block::Log
            )
          }
          root + IVec3::new(way.x * reach, fork + reach / 2, way.y * reach)
        })
        .collect();
      for centre in ends.iter().copied().chain([root + IVec3::Y * (trunk + 1)]) {
        for at in clump(centre, 2, 1, 0x1EAF) {
          put(at, Block::Leaves)
        }
      }
      for dy in 0..trunk {
        put(root + IVec3::Y * dy, Block::Log)
      }
    }
    Growth::Tree(Wood::Birch) => {
      let trunk = 5 + size;
      for at in clump(root + IVec3::Y * (trunk - 1), 2, 3, 0xB1C) {
        put(at, Block::BirchLeaves)
      }
      for dy in 0..trunk {
        put(root + IVec3::Y * dy, Block::BirchLog)
      }
    }
    Growth::Tree(Wood::Spruce) => {
      let trunk = 6 + size * 2;
      for (dx, dz) in disc(3) {
        for dy in 2..=trunk + 1 {
          let tier = (trunk + 2 - dy) / 2 - (trunk - dy).rem_euclid(2);
          let radius = tier.clamp(0, 3);
          let offset = IVec3::new(dx, dy, dz);
          let rim = dx * dx + dz * dz >= radius * radius - 1 && radius > 1;
          if dx * dx + dz * dz <= radius * radius + 1 - i32::from(radius == 0)
            && !(rim && speck(offset, 0x5F7) < 0.35)
          {
            put(root + offset, Block::SpruceLeaves)
          }
        }
      }
      for dy in 0..trunk {
        put(root + IVec3::Y * dy, Block::SpruceLog)
      }
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
      for (dx, dz) in disc(1).filter(|&(dx, dz)| dx != 0 || dz != 0) {
        let way = IVec3::new(dx, 0, dz);
        let length = if dx != 0 && dz != 0 { 2 } else { 3 };
        for step in 1..=length {
          let droop = if step == length { -1 } else { 0 };
          put(crown + way * step + IVec3::Y * droop, Block::PalmLeaves)
        }
      }
      for dy in 0..trunk {
        put(root + lean * shift(dy) + IVec3::Y * dy, Block::PalmLog)
      }
    }
    Growth::RedShroom => {
      let (stem, radius) = (5 + size * 2, 2 + size.min(1) + 1);
      for (dx, dz) in disc(radius) {
        let ring = dx.abs().max(dz.abs());
        let corner = dx.abs() == dz.abs() && ring >= radius - 1;
        if ring < radius && !(corner && ring == radius - 1) {
          put(root + IVec3::new(dx, stem, dz), Block::RedCap)
        }
        if ring == radius && dx.abs() != dz.abs() {
          for dy in stem - 2..stem {
            put(root + IVec3::new(dx, dy, dz), Block::RedCap)
          }
        }
      }
      for dy in 0..stem {
        put(root + IVec3::Y * dy, Block::MushroomStem)
      }
    }
    Growth::BrownShroom => {
      let (stem, radius) = (4 + size * 2, 3 + size.min(2));
      for (dx, dz) in
        disc(radius).filter(|&(dx, dz)| dx * dx + dz * dz <= radius * radius + 1)
      {
        put(root + IVec3::new(dx, stem, dz), Block::BrownCap)
      }
      for dy in 0..stem {
        put(root + IVec3::Y * dy, Block::MushroomStem)
      }
    }
  }
}

fn bloom(seed: u32, x: i32, z: i32, ground: &Column) -> Option<Block> {
  let &Column { heat, wet, top, height, island, .. } = ground;
  let kind = island.map(|(island, _)| island.kind);
  let pick = unit(seed ^ 0xF12, x, 3, z);
  let field =
    fbm2(seed.wrapping_add(41), x as f32 / 24.0, z as f32 / 24.0, 2) * 0.5 + 0.5;
  let forest = smoothstep(-0.25, 0.45, wet);
  let parched = smoothstep(-0.6, -0.1, wet);
  let flower = [Block::Poppy, Block::Dandelion, Block::Cornflower, Block::Daisy]
    [(hash(seed ^ 0xF11, x.div_euclid(7), 0, z.div_euclid(7)) % 4) as usize];
  let mushroom = match pick < 0.5 {
    true => Block::RedMushroom,
    false => Block::BrownMushroom
  };
  let sapling = match Wood::native(heat, unit(seed ^ 0xF14, x, 5, z)) {
    Wood::Oak => Block::OakSapling,
    Wood::Birch => Block::BirchSapling,
    Wood::Spruce => Block::SpruceSapling,
    Wood::Palm => Block::PalmSapling
  };
  let sown = match (kind, top) {
    _ if height <= SEA => None,
    (Some(Kind::Meadow), Block::Grass) => Some((flower, 0.1)),
    (Some(Kind::Mushroom), Block::Mycelium) => Some((mushroom, 0.06)),
    (_, Block::Grass) if forest > 0.6 => Some(match pick {
      pick if pick < 0.4 => (sapling, 0.03),
      pick if pick < 0.7 => (mushroom, 0.03),
      _ => (flower, 0.04)
    }),
    (_, Block::Grass) => Some((flower, (0.03 + 0.05 * (1.0 - forest)) * parched)),
    (_, Block::Snow) => Some((Block::SpruceSapling, 0.005)),
    (_, Block::Sand) if heat > 0.3 => Some((Block::PalmSapling, 0.005)),
    _ => None
  };
  let glimmer =
    [Block::GlimmerPink, Block::GlimmerCyan, Block::GlimmerViolet, Block::GlimmerGold]
      [(hash(seed ^ 0xF15, x.div_euclid(9), 1, z.div_euclid(9)) % 4) as usize];
  let enchanted = top == Block::Grass
    && heat > -0.3
    && unit(seed ^ 0xF16, x, 6, z) < 0.01 * (0.2 + field * field * 2.0);
  let moonlit = (top == Block::Snow || (top == Block::Grass && heat < -0.15))
    && height > SEA + 6
    && unit(seed ^ 0xF17, x, 7, z) < 0.004;
  moonlit
    .then_some(Block::Moonlace)
    .or(enchanted.then_some(glimmer))
    .or(
      sown
        .filter(|&(_, density)| {
          unit(seed ^ 0xF10, x, 2, z) < density * (0.2 + field * field * 2.0)
        })
        .map(|(block, _)| block)
    )
    .or_else(|| {
      let lush = match (kind, top) {
        (Some(Kind::Meadow), Block::Grass) => 0.45,
        (_, Block::Grass) => {
          (0.12 + 0.28 * smoothstep(-0.3, 0.4, wet)) * parched.max(0.3)
        }
        _ => 0.0
      };
      let sward = fbm2(seed.wrapping_add(43), x as f32 / 11.0, z as f32 / 11.0, 2) + 0.5;
      (unit(seed ^ 0xF13, x, 4, z) < lush * sward.clamp(0.0, 1.0))
        .then_some(Block::TallGrass)
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

fn terrain(seed: u32, key: IVec3) -> Chunk {
  let origin = origin_of(key);
  let reach = SIZE + TREE_REACH * 2;
  let wide = reach + 2;
  let islands = Island::within(
    seed,
    origin.xz() - TREE_REACH - 1,
    origin.xz() + SIZE + TREE_REACH + 1
  );
  let stones = Waystone::within(
    seed,
    origin.xz() - TREE_REACH - 3,
    origin.xz() + SIZE + TREE_REACH + 3
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
      for index in 0..VOLUME as i32 {
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
      }
      for (ore, centre, radius) in veins(seed, key).filter(|&(ore, centre, _)| {
        ore != Block::PlatinumOre || platinum_bearing(seed, &islands, centre)
      }) {
        let low = (centre - radius).floor().as_ivec3() - origin;
        let high = (centre + radius).ceil().as_ivec3() - origin;
        for y in low.max(IVec3::ZERO).y..=high.min(IVec3::splat(SIZE - 1)).y {
          for z in low.max(IVec3::ZERO).z..=high.min(IVec3::splat(SIZE - 1)).z {
            for x in low.max(IVec3::ZERO).x..=high.min(IVec3::splat(SIZE - 1)).x {
              let local = IVec3::new(x, y, z);
              let at = origin + local;
              let ragged = radius * (0.75 + unit(seed ^ 0x0AE, at.x, at.y, at.z) * 0.5);
              let slot = &mut blocks[Chunk::index(local)];
              if slot.rock() && at.as_vec3().distance(centre) <= ragged {
                *slot = ore
              }
            }
          }
        }
      }
      let inside = |local: IVec3| {
        local.cmpge(IVec3::ZERO).all() && local.cmplt(IVec3::splat(SIZE)).all()
      };
      for x in -TREE_REACH..SIZE + TREE_REACH {
        for z in -TREE_REACH..SIZE + TREE_REACH {
          let ground = column_at(x, z);
          let column = IVec2::new(origin.x + x, origin.z + z);
          let clearing =
            stones.iter().any(|stone| (stone.at.xz() - column).abs().max_element() <= 4);
          if let Some(sprout) = growth(seed, column.x, column.y, &ground, clearing)
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
        }
      }
      for x in 0..SIZE {
        for z in 0..SIZE {
          let ground = column_at(x, z);
          let local = IVec3::new(x, ground.height + 1 - origin.y, z);
          if let Some(plant) = bloom(seed, origin.x + x, origin.z + z, &ground)
            && inside(local)
            && blocks[Chunk::index(local)] == Block::Air
            && (local.y == 0 || blocks[Chunk::index(local - IVec3::Y)] == ground.top)
          {
            blocks[Chunk::index(local)] = plant
          }
        }
      }
      for stone in stones.iter().map(|stone| stone.at) {
        for dx in -2..=2 {
          for dz in -2..=2 {
            let local = stone - origin + IVec3::new(dx, 0, dz);
            if (0..SIZE).contains(&local.x) && (0..SIZE).contains(&local.z) {
              let ground = column_at(local.x, local.z).height;
              for y in ground.min(stone.y - 1)..=stone.y + 4 {
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
              }
            }
          }
        }
      }
      for seep in islands.iter().filter_map(|island| island.seep) {
        for dx in -SEEP_REACH..=SEEP_REACH {
          for dz in -SEEP_REACH..=SEEP_REACH {
            let column = seep + IVec2::new(dx, dz);
            let local = column - origin.xz();
            if (0..SIZE).contains(&local.x)
              && (0..SIZE).contains(&local.y)
              && seeping(seed, seep, column)
            {
              let ground = column_at(local.x, local.y);
              let depth = if dx == 0 && dz == 0 { 3 } else { 1 };
              for y in ground.height + 1 - depth..=ground.height + 1 {
                let cell = IVec3::new(local.x, y - origin.y, local.y);
                if inside(cell) {
                  let slot = &mut blocks[Chunk::index(cell)];
                  match y > ground.height {
                    true if slot.modelled() => *slot = Block::Air,
                    true => (),
                    false if slot.solid() => *slot = Block::OilSeep,
                    false => ()
                  }
                }
              }
            }
          }
        }
      }
      Chunk::Mixed(blocks).settled()
    }
  }
}

pub fn chunk(seed: u32, key: IVec3) -> Chunk {
  let origin = origin_of(key);
  let dungeons = Dungeon::within(seed, origin.xz(), origin.xz() + SIZE)
    .into_iter()
    .filter(|dungeon| !dungeon.stamps(key).is_empty())
    .collect::<Vec<_>>();
  match (terrain(seed, key), dungeons.is_empty()) {
    (base, true) => base,
    (base, false) => {
      let mut blocks = match base {
        Chunk::Uniform(block) => Box::new([block; VOLUME]),
        Chunk::Mixed(blocks) => blocks
      };
      for &(index, stamp) in dungeons.iter().flat_map(|dungeon| dungeon.stamps(key)) {
        let slot = &mut blocks[usize::from(index)];
        *slot = stamp.over(*slot)
      }
      Chunk::Mixed(blocks).settled()
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Waystone {
  pub cell: IVec2,
  pub at: IVec3,
  pub land: &'static str,
  pub isle: bool
}

const WAYSTONE_CHANCE: f32 = 0.4;

impl Waystone {
  fn found(seed: u32, cell: IVec2) -> Option<Waystone> {
    let home = cell == IVec2::ZERO;
    let wanted = home || unit(seed ^ 0x3A7, cell.x, 0x51, cell.y) < WAYSTONE_CHANCE;
    let (low, high) = (cell * CELL, cell * CELL + CELL - 1);
    let islands = Island::within(seed, low, high);
    let ground = |at: IVec2| {
      let here = surface(seed, &islands, at.x, at.y);
      match here.island {
        Some((island, rise))
          if rise.streak || (island.kind == Kind::Volcano && rise.inward > 0.6) =>
        {
          f32::MIN
        }
        _ => here.height
      }
    };
    let seeps: Vec<IVec2> = islands.iter().filter_map(|island| island.seep).collect();
    let grid = (0..4).flat_map(move |i| {
      (0..4).map(move |j| (low + IVec2::new(16 + i * 32, 16 + j * 32)).as_vec2())
    });
    let spot = wanted.then(|| {
      Island::at(seed, cell)
        .map(|island| island.spots())
        .unwrap_or_default()
        .into_iter()
        .chain(grid)
        .map(|spot| spot.floor().as_ivec2())
        .filter(|&spot| spot.cmpge(low + 3).all() && spot.cmple(high - 3).all())
        .filter(|&spot| {
          seeps
            .iter()
            .all(|&seep| seep.distance_squared(spot) > SEEP_CLEARANCE * SEEP_CLEARANCE)
        })
        .filter_map(|spot| {
          let (lowest, highest) = (-1..=1)
            .flat_map(|dx| (-1..=1).map(move |dz| spot + IVec2::new(dx, dz) * 2))
            .map(ground)
            .fold((f32::MAX, f32::MIN), |(low, high), h| (low.min(h), high.max(h)));
          (lowest >= SEA as f32 + 1.0).then_some((spot, highest - lowest))
        })
        .fold((None, f32::MAX), |(best, cost), (spot, spread)| match spread < cost {
          true => (Some(spot), spread),
          false => (best, cost)
        })
        .0
    });
    spot.flatten().map(|spot| {
      let height = ground(spot).max(SEA as f32 + 1.0).floor() as i32;
      let here = surface(seed, &islands, spot.x, spot.y);
      let isle = here
        .island
        .is_some_and(|(island, rise)| island.cell == cell && rise.inward > 0.0);
      let ground = column(seed, here, false, spot.x, spot.y);
      let land = match here.island {
        Some((island, _)) if isle => island.kind.describe(),
        _ => match ground.top {
          Block::Snow => "snowfields",
          Block::Sand => "desert",
          Block::Mycelium => "mushrooms",
          _ if ground.wet > 0.3 => "forest",
          _ => "hills"
        }
      };
      Waystone { cell, at: IVec3::new(spot.x, height + 1, spot.y), land, isle }
    })
  }

  pub fn at(seed: u32, cell: IVec2) -> Option<Waystone> {
    static KNOWN: LazyLock<RwLock<HashMap<(u32, IVec2), Option<Waystone>>>> =
      LazyLock::new(default);
    let known = KNOWN.read().ok().and_then(|known| known.get(&(seed, cell)).copied());
    known.unwrap_or_else(|| {
      let stone = Waystone::found(seed, cell);
      if let Ok(mut known) = KNOWN.write() {
        known.insert((seed, cell), stone);
      }
      stone
    })
  }

  pub fn within(seed: u32, low: IVec2, high: IVec2) -> Vec<Waystone> {
    let cell_of = |at: IVec2| at.div_euclid(IVec2::splat(CELL));
    let (from, to) = (cell_of(low), cell_of(high));
    (from.x..=to.x)
      .flat_map(|x| (from.y..=to.y).map(move |y| IVec2::new(x, y)))
      .filter_map(|cell| Waystone::at(seed, cell))
      .collect()
  }

  pub fn beside(seed: u32, at: Vec3) -> Option<Waystone> {
    let spot = at.xz().floor().as_ivec2();
    let reach = REACH.ceil() as i32 + 3;
    Waystone::within(seed, spot - reach, spot + reach).into_iter().find(|stone| {
      (stone.at.as_vec3() + Vec3::new(0.5, 1.0, 0.5)).distance(at) <= REACH + 2.0
    })
  }

  pub fn arrival(&self) -> Vec3 { self.at.as_vec3() + Vec3::new(2.5, 0.05, 0.5) }

  pub fn name(&self, seed: u32) -> String {
    let titles = match self.land {
      "snowfields" => ["Drift", "Rime", "Cairn", "Watch"],
      "desert" => ["Sands", "Mesa", "Waste", "Reach"],
      "forest" => ["Glade", "Hollow", "Wood", "Grove"],
      _ => ["Cairn", "Rise", "Watch", "Reach"]
    };
    Island::at(seed, self.cell).filter(|_| self.isle).map_or_else(
      || {
        let pick = hash(seed ^ 0x3A8, self.cell.x, 0x52, self.cell.y) as usize % 4;
        coined(seed, self.cell, "Stone", titles[pick])
      },
      |island| island.name(seed)
    )
  }
}

pub fn spawn_point(seed: u32) -> Vec3 {
  Waystone::at(seed, IVec2::ZERO)
    .map_or(Vec3::new(0.5, 200.0, 0.5), |home| home.arrival())
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
    for wanted in [
      Block::Stone,
      Block::CoalOre,
      Block::IronOre,
      Block::Grass,
      Block::Air,
      Block::Bedrock
    ]
    .into_iter()
    {
      assert!(found(wanted), "no {wanted:?}")
    }
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
    let (span, step) = (5120, 4);
    let pixels = span / step;
    let islands = Island::within(1, IVec2::splat(-span / 2), IVec2::splat(span / 2));
    let stones = Waystone::within(1, IVec2::splat(-span / 2), IVec2::splat(span / 2));
    let image: Vec<u8> = (0..pixels * pixels)
      .flat_map(|index| {
        let (x, z) = (index % pixels * step - span / 2, index / pixels * step - span / 2);
        let here = surface(1, &islands, x, z);
        let ground = column(1, here, false, x, z);
        let stone = stones
          .iter()
          .any(|stone| (stone.at.xz() - IVec2::new(x, z)).abs().max_element() <= 3);
        let wooded = growth(1, x, z, &ground, false).is_some();
        let colour: [u8; 3] = match ground.top {
          _ if stone => [255, 0, 255],
          _ if wooded && ground.height >= SEA => [25, 70, 30],
          _ if ground.height < ground.pool.1 && ground.pool.0 == Block::Lava => {
            [255, 120, 0]
          }
          _ if ground.height < SEA && ground.ice => [200, 225, 245],
          _ if ground.height < SEA => {
            let depth = ((SEA - ground.height) as f32 / 28.0).clamp(0.0, 1.0);
            [60.0 - depth * 40.0, 150.0 - depth * 100.0, 170.0 - depth * 40.0]
              .map(|channel| channel as u8)
          }
          Block::Grass => [70, 130, 50],
          Block::Sand => [220, 205, 140],
          Block::Snow => [240, 240, 250],
          Block::Mycelium => [130, 100, 140],
          Block::Basalt => [50, 50, 55],
          Block::Lava => [255, 90, 0],
          Block::Gravel => [130, 130, 130],
          _ => [110, 110, 110]
        };
        let shade = match ground.height < SEA {
          true => 1.0,
          false => (0.6 + (ground.height - SEA) as f32 / 120.0).clamp(0.5, 1.2)
        };
        colour.map(|channel| (channel as f32 * shade).min(255.0) as u8)
      })
      .collect();
    let mut file = format!("P6 {pixels} {pixels} 255\n").into_bytes();
    file.extend(image);
    std::fs::write("screenshots/map.ppm", file).unwrap()
  }
}
