use {crate::{noise::{fbm2, hash, perlin2, smoothstep, unit},
             protocol::REACH},
     bevy::{platform::collections::HashMap, prelude::*},
     std::sync::{LazyLock, RwLock}};

pub const SEA: i32 = 62;
pub const CELL: i32 = 128;
const DRIFT: f32 = 40.0;
const CRATER: f32 = 0.78;
const FLOES: f32 = 24.0;
pub const FACING_STONE: f32 = std::f32::consts::FRAC_PI_2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
  Meadow,
  Woods,
  Peak,
  Mushroom,
  Volcano,
  Frost,
  Dunes
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wood {
  Oak,
  Birch,
  Spruce,
  Palm
}

#[derive(Clone, Copy, Debug)]
pub struct Island {
  pub cell: IVec2,
  pub centre: Vec2,
  pub radius: f32,
  pub kind: Kind,
  pub wood: Wood,
  pub beach: bool,
  pub warm: bool,
  pub summit: f32,
  pub stone: IVec3
}

#[derive(Clone, Copy, Debug)]
pub struct Rise {
  pub height: f32,
  pub inward: f32,
  pub floe: bool,
  pub streak: bool
}

const SHORE_BEACH: f32 = 1.5;
const SHORE_BANK: f32 = 3.5;

const COMPASS: [Vec2; 8] = [
  Vec2::new(1.0, 0.0),
  Vec2::new(0.7071, 0.7071),
  Vec2::new(0.0, 1.0),
  Vec2::new(-0.7071, 0.7071),
  Vec2::new(-1.0, 0.0),
  Vec2::new(-0.7071, -0.7071),
  Vec2::new(0.0, -1.0),
  Vec2::new(0.7071, -0.7071)
];

const SYLLABLES: [&str; 30] = [
  "Ar", "Bel", "Cor", "Dun", "El", "Fen", "Gal", "Hal", "Is", "Jor", "Kel", "Lun", "Mar",
  "Nor", "Ost", "Pel", "Quin", "Ros", "Sel", "Tor", "Ul", "Vel", "Wyn", "Yr", "Zan",
  "Ash", "Bry", "Cal", "Dra", "Eri"
];
const LINKS: [&str; 10] = ["a", "e", "i", "o", "an", "en", "ar", "or", "il", ""];
const ENDINGS: [&str; 16] = [
  "mar", "dor", "wen", "holm", "ra", "nis", "lis", "thos", "vik", "gard", "mere", "sca",
  "tia", "ven", "rok", "lan"
];

impl Kind {
  fn titles(self) -> [&'static str; 4] {
    match self {
      Kind::Meadow => ["Isle", "Island", "Holm", "Downs"],
      Kind::Woods => ["Wood", "Grove", "Weald", "Isle"],
      Kind::Peak => ["Peak", "Crag", "Tor", "Isle"],
      Kind::Mushroom => ["Spore Isle", "Toadstool", "Hollow", "Cap Isle"],
      Kind::Volcano => ["Caldera", "Ember Isle", "Ashpeak", "Cinder"],
      Kind::Frost => ["Frost Isle", "Floe", "Rime", "Glacier"],
      Kind::Dunes => ["Sands", "Cay", "Dunes", "Key"]
    }
  }

  pub fn describe(self) -> &'static str {
    match self {
      Kind::Meadow => "hills",
      Kind::Woods => "forest",
      Kind::Peak => "mountain",
      Kind::Mushroom => "mushrooms",
      Kind::Volcano => "volcano",
      Kind::Frost => "ice",
      Kind::Dunes => "sand"
    }
  }
}

impl Island {
  fn reach(&self, seed: u32, point: Vec2) -> f32 {
    let warp = fbm2(seed.wrapping_add(30), point.x / 40.0, point.y / 40.0, 3);
    self.radius * (1.0 + warp.clamp(-0.6, 0.6) * 0.5)
  }

  fn shore(&self, inland: f32) -> f32 {
    match self.beach {
      true => (inland * 0.3).min(SHORE_BEACH),
      false => (inland * 1.2).min(SHORE_BANK)
    }
  }

  pub fn crater(&self) -> i32 {
    (SEA as f32 - 0.5 + self.shore(f32::MAX) + self.summit - 6.0).floor() as i32
  }

  pub fn rise(&self, seed: u32, x: i32, z: i32) -> Option<Rise> {
    let point = Vec2::new(x as f32, z as f32);
    let distance = point.distance(self.centre);
    (distance < self.radius * 1.5 + 30.0).then(|| {
      let reach = self.reach(seed, point);
      let t = 1.0 - distance / reach;
      let inland = t * reach;
      let lift = smoothstep(0.0, 0.4, t);
      let hills =
        fbm2(seed.wrapping_add(31), point.x / 30.0, point.y / 30.0, 3) * 0.5 + 0.5;
      let ridges =
        1.0 - fbm2(seed.wrapping_add(36), point.x / 22.0, point.y / 22.0, 3).abs() * 1.5;
      let core = t.max(0.0);
      let elevation = match self.kind {
        Kind::Peak => {
          core * core * self.summit
            + core * 6.0
            + lift * (hills * 5.0 + ridges * 7.0 * core)
        }
        Kind::Volcano => {
          let cone = (core / CRATER).min(1.0);
          let pit = ((core - CRATER) / (1.0 - CRATER) * 2.0).clamp(0.0, 1.0);
          (cone * 0.5 + cone * cone * 0.5) * self.summit - pit * 15.0 + lift * hills * 3.0
        }
        _ => lift * hills * self.summit
      };
      let height = match inland < 0.0 {
        true => SEA as f32 - 1.0 + inland * 0.7,
        false => SEA as f32 - 0.5 + self.shore(inland) + elevation
      };
      let floe = self.kind == Kind::Frost
        && (-FLOES..0.0).contains(&inland)
        && perlin2(seed.wrapping_add(34), point.x / 9.0, point.y / 9.0)
          - inland / FLOES * 0.9
          < 0.4;
      let toward = (point - self.centre) / distance.max(1.0);
      let streak = self.kind == Kind::Volcano
        && (0.22..CRATER).contains(&t)
        && perlin2(
          seed.wrapping_add(33),
          toward.x * 2.2 + self.cell.x as f32 * 5.3,
          toward.y * 2.2 + self.cell.y as f32 * 5.3
        )
        .abs()
          * distance
          < 1.6;
      Rise { height, inward: t, floe, streak }
    })
  }

  fn place_stone(&self, seed: u32) -> IVec3 {
    let ground = |at: IVec2| {
      self.rise(seed, at.x, at.y).map_or(f32::MIN, |rise| match rise.streak {
        true => f32::MIN,
        false => rise.height
      })
    };
    let candidates = [0.2, 0.35, 0.5].into_iter().flat_map(|fraction| {
      COMPASS.into_iter().map(move |direction| direction * fraction * self.radius)
    });
    let (spot, _) = std::iter::once(Vec2::ZERO)
      .filter(|_| self.kind != Kind::Volcano)
      .chain(candidates)
      .map(|offset| (self.centre + offset).floor().as_ivec2())
      .filter_map(|spot| {
        let heights: Vec<f32> = (-1..=1)
          .flat_map(|dx| (-1..=1).map(move |dz| spot + IVec2::new(dx, dz) * 2))
          .map(ground)
          .collect();
        let (low, high) = heights
          .iter()
          .fold((f32::MAX, f32::MIN), |(low, high), &h| (low.min(h), high.max(h)));
        let crater = self.kind == Kind::Volcano
          && self.rise(seed, spot.x, spot.y).is_some_and(|rise| rise.inward > 0.6);
        (low >= SEA as f32 + 1.0 && !crater).then_some((spot, high - low))
      })
      .fold((None, f32::MAX), |(best, cost), (spot, spread)| match spread < cost {
        true => (Some(spot), spread),
        false => (best, cost)
      });
    let spot = spot.unwrap_or(self.centre.floor().as_ivec2());
    let height = ground(spot).max(SEA as f32 + 1.0).floor() as i32;
    IVec3::new(spot.x, height + 1, spot.y)
  }

  fn key(seed: u32, cell: IVec2) -> u32 {
    hash(seed, cell.x, 0x15, 0) ^ hash(seed ^ 0x5EED, 0, cell.y, 0x17)
  }

  fn found(seed: u32, cell: IVec2) -> Option<Island> {
    let key = Island::key(seed, cell);
    let roll = |salt: u32| unit(key ^ salt.wrapping_mul(0x9E37), salt as i32, 0, 0);
    let home = cell == IVec2::ZERO;
    (home || roll(1) > 0.08).then(|| {
      let climate = fbm2(
        seed.wrapping_add(35),
        cell.x as f32 / 5.0 + 0.37,
        cell.y as f32 / 5.0 + 0.61,
        2
      );
      let pick = roll(2);
      let kind = match roll(4) {
        _ if home => Kind::Meadow,
        _ if climate < -0.18 && pick < 0.65 => Kind::Frost,
        _ if climate > 0.18 && pick < 0.5 => match roll(3) < 0.55 {
          true => Kind::Dunes,
          false => Kind::Volcano
        },
        weight if weight < 0.22 => Kind::Meadow,
        weight if weight < 0.44 => Kind::Woods,
        weight if weight < 0.62 => Kind::Peak,
        weight if weight < 0.75 => Kind::Mushroom,
        weight if weight < 0.85 => Kind::Volcano,
        weight if weight < 0.92 => Kind::Frost,
        _ => Kind::Dunes
      };
      let size = roll(6);
      let radius = match roll(5) {
        _ if home => 46.0,
        class if class < 0.4 => 10.0 + size * 12.0,
        class if class < 0.8 => 24.0 + size * 16.0,
        _ => 44.0 + size * 22.0
      };
      let radius = match kind {
        Kind::Peak | Kind::Volcano => radius.max(26.0),
        Kind::Dunes => radius.min(40.0),
        _ => radius
      };
      let jitter = Vec2::new(roll(7) * 2.0 - 1.0, roll(8) * 2.0 - 1.0) * DRIFT;
      let centre = (cell * CELL).as_vec2()
        + CELL as f32 / 2.0
        + if home { Vec2::ZERO } else { jitter };
      let warm = climate > 0.05;
      let wood = match kind {
        Kind::Frost | Kind::Peak => Wood::Spruce,
        Kind::Dunes | Kind::Volcano => Wood::Palm,
        _ => [Wood::Oak, Wood::Birch, Wood::Spruce, Wood::Oak][(roll(9) * 4.0) as usize]
      };
      let summit = match kind {
        Kind::Peak => (radius * 1.5).clamp(40.0, 95.0),
        Kind::Volcano => (radius * 1.2).clamp(32.0, 80.0),
        Kind::Dunes => 3.0 + size * 2.0,
        Kind::Mushroom => 6.0 + size * 4.0,
        Kind::Frost => (radius * 0.3).clamp(6.0, 18.0),
        Kind::Woods => (radius * 0.25).clamp(5.0, 14.0),
        Kind::Meadow => (radius * 0.3).clamp(5.0, 16.0)
      };
      let beach = match kind {
        _ if home => true,
        Kind::Dunes => true,
        Kind::Frost => roll(10) < 0.3,
        _ => roll(10) < 0.6
      };
      let island = Island {
        cell,
        centre,
        radius,
        kind,
        wood,
        beach,
        warm,
        summit,
        stone: IVec3::ZERO
      };
      Island { stone: island.place_stone(seed), ..island }
    })
  }

  pub fn at(seed: u32, cell: IVec2) -> Option<Island> {
    static KNOWN: LazyLock<RwLock<HashMap<(u32, IVec2), Option<Island>>>> =
      LazyLock::new(default);
    let known = KNOWN.read().ok().and_then(|known| known.get(&(seed, cell)).copied());
    known.unwrap_or_else(|| {
      let island = Island::found(seed, cell);
      if let Ok(mut known) = KNOWN.write() {
        known.insert((seed, cell), island);
      }
      island
    })
  }

  fn cell_of(at: IVec2) -> IVec2 { at.div_euclid(IVec2::splat(CELL)) }

  pub fn within(seed: u32, low: IVec2, high: IVec2) -> Vec<Island> {
    let (from, to) = (Island::cell_of(low) - 1, Island::cell_of(high) + 1);
    (from.x..=to.x)
      .flat_map(|x| (from.y..=to.y).map(move |y| IVec2::new(x, y)))
      .filter_map(|cell| Island::at(seed, cell))
      .collect()
  }

  pub fn near(seed: u32, at: Vec3) -> Vec<Island> {
    let spot = at.xz().floor().as_ivec2();
    Island::within(seed, spot, spot)
  }

  pub fn beside(seed: u32, at: Vec3) -> Option<Island> {
    Island::near(seed, at).into_iter().find(|island| {
      (island.stone.as_vec3() + Vec3::new(0.5, 1.0, 0.5)).distance(at) <= REACH + 2.0
    })
  }

  pub fn arrival(&self) -> Vec3 { self.stone.as_vec3() + Vec3::new(2.5, 0.05, 0.5) }

  pub fn name(&self, seed: u32) -> String {
    let key = Island::key(seed, self.cell);
    let pick = |salt: u32, count: usize| {
      hash(key ^ 0x4A3E ^ salt.wrapping_mul(0x01F3), salt as i32, 0, 0) as usize % count
    };
    let core = format!(
      "{}{}{}",
      SYLLABLES[pick(1, SYLLABLES.len())],
      LINKS[pick(2, LINKS.len())],
      ENDINGS[pick(3, ENDINGS.len())]
    );
    let title = match self.radius < 18.0 {
      true => ["Rock", "Islet", "Skerry", "Key"][pick(4, 4)],
      false => self.kind.titles()[pick(4, 4)]
    };
    match pick(5, 5) {
      0 => format!("Isle of {core}"),
      1 => core,
      _ => format!("{core} {title}")
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_kind_appears_near_home() {
    let islands = Island::within(1, IVec2::splat(-CELL * 5), IVec2::splat(CELL * 5));
    [
      Kind::Meadow,
      Kind::Woods,
      Kind::Peak,
      Kind::Mushroom,
      Kind::Volcano,
      Kind::Frost,
      Kind::Dunes
    ]
    .into_iter()
    .for_each(|kind| {
      assert!(islands.iter().any(|island| island.kind == kind), "no {kind:?}")
    });
    let mut nearest = islands.clone();
    nearest.sort_by_key(|island| island.cell.length_squared());
    nearest.iter().take(25).for_each(|island| {
      println!(
        "{:?} {:?} r{:.0} centre {} stone {} summit {:.0} {}",
        island.cell,
        island.kind,
        island.radius,
        island.centre.floor(),
        island.stone,
        island.summit,
        island.name(1)
      )
    })
  }
}
