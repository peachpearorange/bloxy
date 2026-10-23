use {crate::noise::{fbm2, hash, perlin2, smoothstep, unit},
     bevy::{platform::collections::HashMap, prelude::*},
     std::sync::{LazyLock, RwLock}};

pub const SEA: i32 = 62;
pub const CELL: i32 = 128;
const DRIFT: f32 = 48.0;
const CRATER: f32 = 0.78;
const FLOES: f32 = 24.0;
pub const FACING_STONE: f32 = std::f32::consts::FRAC_PI_2;
pub const UNLOAD: f32 = 220.0;

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
  Blob,
  Chain,
  Atoll,
  Crescent
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Lobe {
  pub offset: Vec2,
  pub radius: f32
}

#[derive(Clone, Copy, Debug)]
pub struct Island {
  pub cell: IVec2,
  pub centre: Vec2,
  pub radius: f32,
  pub lobes: [Lobe; 6],
  pub axis: Vec2,
  pub stretch: f32,
  pub form: Form,
  pub kind: Kind,
  pub wood: Wood,
  pub beach: bool,
  pub summit: f32,
  pub seep: Option<IVec2>
}

#[derive(Clone, Copy, Debug)]
pub struct Rise {
  pub height: f32,
  pub inward: f32,
  pub floe: bool,
  pub streak: bool
}

#[derive(Clone, Copy, Debug)]
struct Sketch {
  cell: IVec2,
  centre: Vec2,
  radius: f32,
  stretch: f32,
  form: Form,
  climate: f32
}

impl Sketch {
  fn reach(&self) -> f32 {
    self.radius * self.stretch * 1.25
      + match self.form {
        Form::Chain => self.radius * 1.2,
        _ => 0.0
      }
  }

  fn outranks(&self, other: &Sketch) -> bool {
    let home = |sketch: &Sketch| sketch.cell == IVec2::ZERO;
    home(self)
      || (!home(other)
        && (self.radius, self.cell.x, self.cell.y)
          > (other.radius, other.cell.x, other.cell.y))
  }
}

pub fn landmass(seed: u32, at: Vec2) -> f32 {
  let wave = |salt: u32, scale: f32, point: Vec2| {
    fbm2(seed.wrapping_add(salt), point.x / scale, point.y / scale, 3)
  };
  let warped = at + Vec2::new(wave(40, 520.0, at), wave(41, 520.0, at)) * 260.0;
  wave(42, 700.0, warped) + wave(43, 240.0, warped) * 0.35
}

fn scattering(seed: u32, at: Vec2) -> f32 {
  fbm2(seed.wrapping_add(44), at.x / 420.0, at.y / 420.0, 2)
}

const RING: f32 = 0.8;
const SHORE_BEACH: f32 = 1.5;
const SHORE_BANK: f32 = 3.5;

pub const SEEP_CLEARANCE: i32 = 8;

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
  fn swirl(&self) -> f32 {
    match self.form {
      Form::Atoll => (self.radius * 0.15).min(8.0),
      _ => (self.radius * 0.4).min(22.0 + self.radius * 0.15)
    }
  }

  fn extent(&self) -> f32 {
    let lobe = |Lobe { offset, radius }: Lobe| offset.length() + radius;
    self.lobes.into_iter().map(lobe).fold(self.radius, f32::max) * self.stretch * 1.3
      + self.swirl()
      + FLOES
      + 6.0
  }

  fn inland(&self, seed: u32, point: Vec2) -> f32 {
    let wave = |salt: u32, scale: f32| {
      fbm2(seed.wrapping_add(salt), point.x / scale, point.y / scale, 2)
    };
    let warped = point + Vec2::new(wave(37, 34.0), wave(38, 34.0)) * self.swirl();
    let edge = 1.0
      + wave(30, 40.0).clamp(-0.6, 0.6) * 0.5
      + wave(46, 110.0) * 0.4
      + wave(39, 11.0) * 0.18;
    let along = self.axis;
    let squeeze =
      |away: Vec2| Vec2::new(away.dot(along) / self.stretch, away.perp_dot(along));
    let reach = |Lobe { offset, radius }: Lobe| {
      radius * edge - squeeze(warped - self.centre - offset).length()
    };
    let body = match self.form {
      Form::Atoll => {
        let width = (self.radius * 0.2).max(4.0);
        let gaps = wave(45, 13.0).max(0.0) * width * 1.6;
        width * edge
          - (squeeze(warped - self.centre).length() - self.radius * RING).abs()
          - gaps
      }
      _ => reach(Lobe { offset: Vec2::ZERO, radius: self.radius })
    };
    let land = self
      .lobes
      .into_iter()
      .filter(|lobe| lobe.radius > 0.0)
      .map(reach)
      .fold(body, f32::max);
    match self.form {
      Form::Crescent => {
        let bite = along.perp() * self.radius * 0.9;
        land.min((warped - self.centre - bite).length() - self.radius * 0.72)
      }
      _ => land
    }
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
    (distance < self.extent()).then(|| {
      let inland = self.inland(seed, point);
      let t = inland / self.radius;
      let lift = smoothstep(0.0, 0.4, t);
      let hills =
        fbm2(seed.wrapping_add(31), point.x / 30.0, point.y / 30.0, 3) * 0.5 + 0.5;
      let ridges =
        1.0 - fbm2(seed.wrapping_add(36), point.x / 22.0, point.y / 22.0, 3).abs() * 1.5;
      let core = (1.0 - distance / self.radius).max(0.0).min(t.max(0.0));
      let elevation = match self.kind {
        Kind::Peak => {
          core * core * (3.0 - 2.0 * core) * self.summit
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
      let lagoon = self.form == Form::Atoll && distance < self.radius * RING;
      let height = match inland < 0.0 {
        true if lagoon => (SEA as f32 - 2.0 + inland * 0.2).max(SEA as f32 - 6.0),
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
        && (0.22..CRATER).contains(&core)
        && perlin2(
          seed.wrapping_add(33),
          toward.x * 2.2 + self.cell.x as f32 * 5.3,
          toward.y * 2.2 + self.cell.y as f32 * 5.3
        )
        .abs()
          * distance
          < 1.6;
      Rise { height, inward: core, floe, streak }
    })
  }

  pub fn spots(&self) -> Vec<Vec2> {
    let fractions = match self.form {
      Form::Atoll => [RING; 3],
      _ => [0.2, 0.35, 0.5]
    };
    std::iter::once(Vec2::ZERO)
      .filter(|_| self.kind != Kind::Volcano && self.form != Form::Atoll)
      .chain(fractions.into_iter().flat_map(|fraction| {
        COMPASS.into_iter().map(move |direction| direction * fraction * self.radius)
      }))
      .chain(
        self.lobes.into_iter().filter(|lobe| lobe.radius > 0.0).map(|lobe| lobe.offset)
      )
      .map(|offset| self.centre + offset)
      .collect()
  }

  fn place_seep(&self, seed: u32, turn: f32) -> Option<IVec2> {
    let start = (turn * COMPASS.len() as f32) as usize;
    [0.55, 0.4, 0.7, 0.3]
      .into_iter()
      .flat_map(|fraction| {
        (0..COMPASS.len())
          .map(move |step| COMPASS[(start + step) % COMPASS.len()] * fraction)
      })
      .map(|offset| (self.centre + offset * self.radius).floor().as_ivec2())
      .find(|&spot| {
        (-2..=2).all(|dx| {
          (-2..=2).all(|dz| {
            self.rise(seed, spot.x + dx, spot.y + dz).is_some_and(|rise| {
              !rise.streak && rise.height >= SEA as f32 + 1.5 && rise.inward < 0.95
            })
          })
        })
      })
  }

  fn key(seed: u32, cell: IVec2) -> u32 {
    hash(seed, cell.x, 0x15, 0) ^ hash(seed ^ 0x5EED, 0, cell.y, 0x17)
  }

  fn dice(seed: u32, cell: IVec2) -> impl Fn(u32) -> f32 {
    let key = Island::key(seed, cell);
    move |salt: u32| unit(key ^ salt.wrapping_mul(0x9E37), salt as i32, 0, 0)
  }

  fn sketch(seed: u32, cell: IVec2) -> Option<Sketch> {
    let roll = Island::dice(seed, cell);
    let home = cell == IVec2::ZERO;
    let jitter = Vec2::new(roll(7) * 2.0 - 1.0, roll(8) * 2.0 - 1.0) * DRIFT;
    let centre = (cell * CELL).as_vec2()
      + CELL as f32 / 2.0
      + if home { Vec2::ZERO } else { jitter };
    let land = landmass(seed, centre);
    let present = home || roll(1) < 0.04 + smoothstep(-0.14, 0.08, land) * 0.92;
    present.then(|| {
      let climate = crate::climate::heat(seed, centre);
      let near_home = cell.abs().max_element() <= 2;
      let big = if near_home { 0.0 } else { smoothstep(0.05, 0.35, land) * 0.55 };
      let tiny = 0.1 + smoothstep(0.0, -0.3, land) * 0.7;
      let size = roll(6);
      let radius = match roll(5) {
        _ if home => 46.0,
        class if class < big => 70.0 + size * 50.0,
        class if class > 1.0 - tiny => 6.0 + size * 7.0,
        class if class < big + 0.4 => 10.0 + size * 12.0,
        class if class < big + 0.75 => 24.0 + size * 16.0,
        _ => 44.0 + size * 22.0
      } * 0.85;
      let strewn = smoothstep(-0.1, 0.35, scattering(seed, centre));
      let open = smoothstep(0.0, -0.2, land);
      let form = match roll(11) {
        _ if home => Form::Blob,
        pick if radius >= 12.0 && pick < 0.08 + strewn * 0.5 => Form::Chain,
        pick
          if climate > 0.15
            && (8.0..55.0).contains(&radius)
            && pick > 0.72 - open * 0.3 =>
        {
          Form::Atoll
        }
        pick if radius >= 18.0 && (0.55..0.7).contains(&pick) => Form::Crescent,
        _ => Form::Blob
      };
      let radius = match form {
        Form::Atoll => (radius * 1.3).max(22.0),
        _ => radius
      };
      let stretch = match form {
        Form::Chain | Form::Atoll => 1.0,
        _ if radius > 60.0 => 1.0 + roll(32) * 0.5,
        _ => 1.0 + roll(32) * 0.8
      };
      Sketch { cell, centre, radius, stretch, form, climate }
    })
  }

  fn crowded(seed: u32, sketch: &Sketch) -> bool {
    (-2..=2)
      .flat_map(|x| (-2..=2).map(move |y| IVec2::new(x, y)))
      .filter(|&offset| offset != IVec2::ZERO)
      .filter_map(|offset| Island::sketch(seed, sketch.cell + offset))
      .any(|other| {
        other.outranks(sketch)
          && other.centre.distance(sketch.centre) < other.reach() + sketch.radius * 0.6
      })
  }

  fn found(seed: u32, cell: IVec2) -> Option<Island> {
    let roll = Island::dice(seed, cell);
    let home = cell == IVec2::ZERO;
    Island::sketch(seed, cell).filter(|sketch| !Island::crowded(seed, sketch)).map(
      |Sketch { centre, radius, stretch, form, climate, .. }| {
        let pick = roll(2);
        let wet = crate::climate::wet(seed, centre);
        let kind = match roll(4) {
          _ if home => Kind::Meadow,
          _ if form == Form::Atoll => match roll(3) < 0.6 && wet < 0.2 {
            true => Kind::Dunes,
            false => Kind::Meadow
          },
          _ if climate < -0.3 && pick < 0.7 => Kind::Frost,
          _ if climate > 0.32 && pick < 0.55 => match roll(3) < 0.6 && wet < 0.15 {
            true => Kind::Dunes,
            false => Kind::Volcano
          },
          weight if weight < 0.24 => Kind::Meadow,
          weight if weight < 0.24 + 0.2 + wet.clamp(-0.5, 0.5) * 0.2 => Kind::Woods,
          weight if weight < 0.66 => Kind::Peak,
          weight if weight < 0.8 => Kind::Mushroom,
          weight if weight < 0.9 && climate > -0.1 => Kind::Volcano,
          _ => Kind::Meadow
        };
        let form = match kind {
          Kind::Peak | Kind::Volcano => Form::Blob,
          _ => form
        };
        let size = roll(6);
        let radius = match (kind, form) {
          (_, Form::Chain) => radius * 0.5,
          (Kind::Peak | Kind::Volcano, _) => radius.max(22.0),
          (Kind::Dunes, Form::Blob | Form::Crescent) => radius.min(34.0),
          _ => radius
        };
        let axis =
          Vec2::new(roll(30) * 2.0 - 1.0, roll(31) * 2.0 - 1.0).normalize_or(Vec2::X);
        let lobes = match form {
          Form::Chain => {
            let bend = roll(33) * 2.0 - 1.0;
            std::array::from_fn(|index| {
              let salt = 40 + index as u32 * 3;
              let along = (index as f32 + 0.5) / 3.0 - 1.0;
              let offset = (axis * along
                + axis.perp() * bend * (1.0 - along * along) * 0.6)
                * radius
                * 4.0
                + Vec2::new(roll(salt) - 0.5, roll(salt + 1) - 0.5) * radius;
              let shrink = 1.0 - along.abs() * 0.45;
              match roll(salt + 2) < 0.85 {
                true => Lobe {
                  offset,
                  radius: radius * (0.45 + roll(salt + 2) * 0.55) * shrink
                },
                false => Lobe::default()
              }
            })
          }
          Form::Atoll => default(),
          _ => std::array::from_fn(|index| {
            let salt = 20 + index as u32 * 3;
            let toward = COMPASS[(roll(salt) * 8.0) as usize];
            let most = if radius > 50.0 { 6.0 } else { 3.0 };
            match index < most as usize && roll(19) * (most + 1.0) > index as f32 + 0.3 {
              true => Lobe {
                offset: toward * radius * (0.5 + roll(salt + 1) * 0.45),
                radius: radius * (0.3 + roll(salt + 2) * 0.4)
              },
              false => Lobe::default()
            }
          })
        };
        let wood = match kind {
          _ if form == Form::Atoll => Wood::Palm,
          Kind::Frost => Wood::Spruce,
          Kind::Dunes => Wood::Palm,
          _ => Wood::native(climate, roll(9))
        };
        let summit = match kind {
          _ if form == Form::Atoll => 2.0 + size * 2.0,
          Kind::Peak => (radius * 1.5).clamp(40.0, 110.0),
          Kind::Volcano => (radius * 1.2).clamp(32.0, 90.0),
          Kind::Dunes => 3.0 + size * 2.0,
          Kind::Mushroom => 6.0 + size * 4.0,
          Kind::Frost => (radius * 0.3).clamp(6.0, 22.0),
          Kind::Woods => (radius * 0.25).clamp(5.0, 18.0),
          Kind::Meadow => (radius * 0.3).clamp(5.0, 20.0)
        };
        let beach = match kind {
          _ if home || form == Form::Atoll => true,
          Kind::Dunes => true,
          Kind::Frost => roll(10) < 0.3,
          _ => roll(10) < 0.6
        };
        let island = Island {
          cell,
          centre,
          radius,
          lobes,
          axis,
          stretch,
          form,
          kind,
          wood,
          beach,
          summit,
          seep: None
        };
        let seeping = match kind {
          _ if home => 0.0,
          Kind::Dunes => 0.5,
          Kind::Volcano => 0.4,
          Kind::Frost => 0.2,
          Kind::Mushroom => 0.05,
          _ => 0.1
        };
        let seep =
          (roll(11) < seeping).then(|| island.place_seep(seed, roll(12))).flatten();
        Island { seep, ..island }
      }
    )
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
    let (from, to) = (Island::cell_of(low) - 3, Island::cell_of(high) + 3);
    (from.x..=to.x)
      .flat_map(|x| (from.y..=to.y).map(move |y| IVec2::new(x, y)))
      .filter_map(|cell| Island::at(seed, cell))
      .collect()
  }

  pub fn near(seed: u32, at: Vec3) -> Vec<Island> {
    let spot = at.xz().floor().as_ivec2();
    Island::within(seed, spot, spot)
  }

  pub fn name(&self, seed: u32) -> String {
    let pick = coinage(seed, self.cell);
    let title = match self.form {
      Form::Chain => ["Isles", "Skerries", "Chain", "Islets"][pick(4, 4)],
      Form::Atoll => ["Atoll", "Lagoon", "Reef", "Ring"][pick(4, 4)],
      _ if self.radius < 18.0 => ["Rock", "Islet", "Skerry", "Key"][pick(4, 4)],
      _ => self.kind.titles()[pick(4, 4)]
    };
    coined(seed, self.cell, "Isle", title)
  }
}

fn coinage(seed: u32, cell: IVec2) -> impl Fn(u32, usize) -> usize {
  let key = Island::key(seed, cell);
  move |salt: u32, count: usize| {
    hash(key ^ 0x4A3E ^ salt.wrapping_mul(0x01F3), salt as i32, 0, 0) as usize % count
  }
}

pub fn coined(seed: u32, cell: IVec2, of: &str, title: &str) -> String {
  let pick = coinage(seed, cell);
  let core = format!(
    "{}{}{}",
    SYLLABLES[pick(1, SYLLABLES.len())],
    LINKS[pick(2, LINKS.len())],
    ENDINGS[pick(3, ENDINGS.len())]
  );
  match pick(5, 5) {
    0 => format!("{of} of {core}"),
    1 => core,
    _ => format!("{core} {title}")
  }
}

impl Wood {
  pub fn native(heat: f32, roll: f32) -> Wood {
    match () {
      () if heat < -0.2 => Wood::Spruce,
      () if heat < 0.05 => [Wood::Birch, Wood::Spruce][usize::from(roll > 0.5)],
      () if heat < 0.35 => [Wood::Oak, Wood::Birch][usize::from(roll > 0.65)],
      () => [Wood::Palm, Wood::Oak][usize::from(roll > 0.6)]
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_kind_appears_near_home() {
    let islands = Island::within(1, IVec2::splat(-CELL * 5), IVec2::splat(CELL * 5));
    for kind in [
      Kind::Meadow,
      Kind::Woods,
      Kind::Peak,
      Kind::Mushroom,
      Kind::Volcano,
      Kind::Frost,
      Kind::Dunes
    ]
    .into_iter()
    {
      assert!(islands.iter().any(|island| island.kind == kind), "no {kind:?}")
    }
    let mut nearest = islands.clone();
    nearest.sort_by_key(|island| island.cell.length_squared());
    for island in nearest.iter().take(25) {
      println!(
        "{:?} {:?} r{:.0} centre {} summit {:.0} {}",
        island.cell,
        island.kind,
        island.radius,
        island.centre.floor(),
        island.summit,
        island.name(1)
      )
    }
  }
}
