use {crate::{block::Block,
             generate,
             island::{Island, Kind, SEA},
             noise::{hash, perlin3, unit},
             protocol::Stack,
             voxels::{SIZE, chunk_of, local_of}},
     bevy::{platform::collections::HashMap, prelude::*},
     serde::{Deserialize, Serialize},
     std::sync::{Arc, LazyLock, RwLock}};

pub const REACH: i32 = 110;
const HALL_TALL: i32 = 4;
const DEEPEST: i32 = 12;
const KEEP_CLEAR: f32 = 12.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stamp {
  Shell(Block),
  Seal(Block),
  Clear,
  Floor(Block),
  Open,
  Flood,
  Put(Block)
}

impl Stamp {
  fn rank(self) -> u8 {
    match self {
      Stamp::Shell(_) | Stamp::Seal(_) | Stamp::Clear => 0,
      Stamp::Floor(_) => 1,
      Stamp::Open | Stamp::Flood => 2,
      Stamp::Put(_) => 3
    }
  }

  pub fn over(self, was: Block) -> Block {
    match self {
      Stamp::Shell(block) if was.solid() || was.fluid() => block,
      Stamp::Seal(block) if was.fluid() => block,
      Stamp::Shell(_) | Stamp::Seal(_) => was,
      Stamp::Floor(block) | Stamp::Put(block) => block,
      Stamp::Open | Stamp::Clear => Block::Air,
      Stamp::Flood => Block::Water
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Hash)]
pub enum Foe {
  Skeleton,
  Shade,
  Spider,
  Warden,
  Imp,
  Sentinel,
  Custodian
}

#[derive(Clone, Copy, Debug)]
pub struct Lair {
  pub at: IVec3,
  pub foe: Foe,
  pub count: u8
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
  Descent,
  Spire,
  Precursor
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Theme {
  Pillars,
  Crypt,
  Cavern,
  Chasm,
  Flooded,
  Ossuary,
  Vault
}

pub struct Dungeon {
  pub cell: IVec2,
  pub style: Style,
  pub entry: IVec3,
  pub low: IVec3,
  pub high: IVec3,
  pub lights: Vec<IVec3>,
  pub lairs: Vec<Lair>,
  pub chests: Vec<(IVec3, u8)>,
  stamps: HashMap<IVec3, Vec<(u16, Stamp)>>
}

struct Chamber {
  low: IVec3,
  high: IVec3,
  theme: Theme,
  doors: Vec<IVec3>,
  depth: i32
}

pub(crate) struct Plan {
  pub(crate) key: u32,
  pub(crate) rolls: u32,
  pub(crate) centre: Vec2,
  pub(crate) radius: f32,
  pub(crate) cells: HashMap<IVec3, Stamp>,
  pub(crate) lights: Vec<IVec3>,
  pub(crate) lairs: Vec<Lair>,
  pub(crate) chests: Vec<(IVec3, u8)>
}

const WAYS: [IVec3; 4] = [IVec3::X, IVec3::Z, IVec3::NEG_X, IVec3::NEG_Z];

fn side(way: IVec3) -> IVec3 { IVec3::new(-way.z, 0, way.x) }

impl Plan {
  pub(crate) fn roll(&mut self) -> f32 {
    self.rolls += 1;
    unit(self.key, self.rolls as i32, 0x0D, 0x1E)
  }

  pub(crate) fn range(&mut self, low: i32, high: i32) -> i32 {
    (low + (self.roll() * (high - low + 1) as f32) as i32).min(high)
  }

  pub(crate) fn chance(&mut self, odds: f32) -> bool { self.roll() < odds }

  pub(crate) fn speck(&self, at: IVec3, salt: u32) -> f32 {
    unit(self.key ^ salt, at.x, at.y, at.z)
  }

  pub(crate) fn stamp(&mut self, at: IVec3, stamp: Stamp) {
    if at.y > 0 && self.cells.get(&at).is_none_or(|old| old.rank() <= stamp.rank()) {
      self.cells.insert(at, stamp);
    }
  }

  fn masonry(&self, at: IVec3) -> Block {
    match self.speck(at, 0x3A5) {
      roll if roll < 0.16 => Block::MossyBrick,
      roll if roll < 0.27 => Block::CrackedBrick,
      _ => Block::AncientBrick
    }
  }

  pub(crate) fn put(&mut self, at: IVec3, block: Block) {
    self.stamp(at, Stamp::Put(block))
  }

  fn light(&mut self, at: IVec3) {
    self.put(at, Block::Brazier);
    self.lights.push(at)
  }

  fn section(&mut self, at: IVec3, way: IVec3) {
    let across = side(way);
    for lateral in -2..=2 {
      for up in -1..=HALL_TALL {
        let cell = at + across * lateral + IVec3::Y * up;
        let stamp = match (lateral.abs() <= 1, up) {
          (true, -1) => Stamp::Floor(Block::AncientBrick),
          (true, HALL_TALL) => Stamp::Shell(self.masonry(cell)),
          (true, _) => Stamp::Open,
          (false, _) => Stamp::Shell(self.masonry(cell))
        };
        self.stamp(cell, stamp)
      }
    }
    for lateral in [-1, 1] {
      let corner = at + across * lateral + IVec3::Y * (HALL_TALL - 1);
      if self.speck(corner, 0xEB) < 0.07 {
        self.put(corner, Block::Cobweb)
      }
    }
  }

  fn landing(&mut self, at: IVec3) {
    for dx in -2..=2 {
      for dz in -2..=2 {
        for up in -1..=HALL_TALL {
          let cell = at + IVec3::new(dx, up, dz);
          let stamp = match (dx.abs() <= 1 && dz.abs() <= 1, up) {
            (true, -1) => Stamp::Floor(Block::AncientBrick),
            (true, HALL_TALL) => Stamp::Shell(self.masonry(cell)),
            (true, _) => Stamp::Open,
            (false, _) => Stamp::Shell(self.masonry(cell))
          };
          self.stamp(cell, stamp)
        }
      }
    }
  }

  fn outward(&self, at: IVec3) -> f32 {
    at.xz().as_vec2().distance(self.centre) / self.radius.max(1.0)
  }

  fn steer(&mut self, at: IVec3, way: IVec3) -> IVec3 {
    let (left, right) = (side(way), -side(way));
    let home = (self.centre - at.xz().as_vec2()).normalize_or_zero();
    let pull = |turn: IVec3| turn.xz().as_vec2().dot(home);
    match self.outward(at + way * 10) > 0.45 {
      true if pull(left) > pull(right) => left,
      true => right,
      false if self.chance(0.5) => left,
      false => right
    }
  }

  fn walk(&mut self, mut at: IVec3, way: IVec3, steps: i32) -> IVec3 {
    for _ in 0..steps {
      at += way;
      self.section(at, way)
    }
    at
  }

  fn descend(&mut self, mut at: IVec3, mut way: IVec3, floor: i32) -> (IVec3, IVec3) {
    let spiral = self.chance(0.55);
    let turn = if self.chance(0.5) { 1 } else { -1 };
    while at.y > floor {
      let run = match spiral {
        true => self.range(4, 6),
        false => self.range(3, 7)
      }
      .min(at.y - floor);
      for _ in 0..run {
        at += way;
        at.y -= 1;
        self.section(at, way)
      }
      at += way;
      self.section(at, way);
      self.landing(at);
      let next = match spiral {
        true => side(way) * turn,
        false => self.steer(at, way)
      };
      let nook = at + way - next;
      if self.speck(nook, 0xB4A) < 0.45 {
        self.light(nook)
      }
      way = next
    }
    (at, way)
  }

  fn hollow(&mut self, low: IVec3, high: IVec3, theme: Theme) {
    let floor = match theme {
      Theme::Vault => Block::CarvedStone,
      Theme::Ossuary => Block::BonePile,
      _ => Block::AncientBrick
    };
    let wobble = |plan: &Plan, at: IVec3| {
      perlin3(plan.key ^ 0xCA5, at.x as f32 / 5.0, at.y as f32 / 4.0, at.z as f32 / 5.0)
    };
    let (middle, half) =
      ((low + high).as_vec3() / 2.0, (high - low).as_vec3() / 2.0 + 0.5);
    for x in low.x - 1..=high.x + 1 {
      for z in low.z - 1..=high.z + 1 {
        for y in low.y - 1..=high.y + 1 {
          let at = IVec3::new(x, y, z);
          let inside = at.cmpge(low).all() && at.cmple(high).all();
          let stamp = match theme {
            Theme::Cavern => {
              let reach = ((at.as_vec3() - middle) / half)
                .with_y(((at.y as f32 - low.y as f32) / (half.y * 2.0)).max(0.0));
              let round = reach.xz().length_squared() + reach.y * reach.y;
              match (round + wobble(self, at) * 0.35 < 0.9 && y >= low.y, y == low.y - 1)
              {
                (true, _) => Stamp::Open,
                (false, true) => Stamp::Seal(Block::Stone),
                _ => Stamp::Seal(Block::Stone)
              }
            }
            _ => match (inside, y == low.y - 1) {
              (true, _) => Stamp::Open,
              (false, true) if x >= low.x && x <= high.x && z >= low.z && z <= high.z => {
                Stamp::Floor(floor)
              }
              (false, _) if theme == Theme::Ossuary && self.speck(at, 0x05) < 0.5 => {
                Stamp::Shell(Block::BonePile)
              }
              (false, _) => Stamp::Shell(self.masonry(at))
            }
          };
          self.stamp(at, stamp)
        }
      }
    }
  }

  fn chamber(
    &mut self,
    door: IVec3,
    way: IVec3,
    (wide, deep, tall): (i32, i32, i32),
    theme: Theme,
    depth: i32
  ) -> Chamber {
    let across = side(way);
    let slack = (wide / 2 - 2).max(0);
    let shift = self.range(-slack, slack);
    let near = door + way;
    let first = near + across * (shift - wide / 2);
    let last =
      near + way * (deep - 1) + across * (shift + wide / 2) + IVec3::Y * (tall - 1);
    let (low, high) = (first.min(last), first.max(last));
    self.hollow(low, high, theme);
    if theme == Theme::Cavern {
      self.tunnel(door, way)
    }
    Chamber { low, high, theme, doors: vec![door], depth }
  }

  fn exit(&mut self, room: &mut Chamber, way: IVec3) -> IVec3 {
    let (low, high) = (room.low, room.high);
    let along =
      |low: i32, high: i32, plan: &mut Plan| plan.range(low + 2, (high - 2).max(low + 2));
    let door = match way {
      IVec3::X => IVec3::new(high.x + 1, low.y, along(low.z, high.z, self)),
      IVec3::NEG_X => IVec3::new(low.x - 1, low.y, along(low.z, high.z, self)),
      IVec3::Z => IVec3::new(along(low.x, high.x, self), low.y, high.z + 1),
      _ => IVec3::new(along(low.x, high.x, self), low.y, low.z - 1)
    };
    room.doors.push(door);
    self.section(door, way);
    if room.theme == Theme::Cavern {
      self.tunnel(door, -way)
    }
    door
  }

  fn floor_cells(&self, room: &Chamber, inset: i32) -> Vec<IVec3> {
    let (low, high) = (room.low, room.high);
    (low.x + inset..=high.x - inset)
      .flat_map(|x| {
        (low.z + inset..=high.z - inset).map(move |z| IVec3::new(x, low.y, z))
      })
      .collect()
  }

  fn near_door(room: &Chamber, at: IVec3, by: i32) -> bool {
    room.doors.iter().any(|door| (door.xz() - at.xz()).abs().max_element() <= by)
  }

  fn rim(&self, room: &Chamber) -> Vec<IVec3> {
    let (low, high) = (room.low, room.high);
    self
      .floor_cells(room, 0)
      .into_iter()
      .filter(|at| at.x == low.x || at.x == high.x || at.z == low.z || at.z == high.z)
      .filter(|&at| !Plan::near_door(room, at, 2))
      .collect()
  }

  fn tunnel(&mut self, door: IVec3, inward: IVec3) {
    for step in 1..=5 {
      self.section(door + inward * step, inward)
    }
  }

  fn decorate(&mut self, room: &Chamber) {
    let (low, high) = (room.low, room.high);
    let centre = IVec3::new((low.x + high.x) / 2, low.y, (low.z + high.z) / 2);
    let tall = high.y - low.y + 1;
    let more = (room.depth / 2) as u8;
    let lair = |plan: &mut Plan, at: IVec3, foe: Foe, count: u8| {
      plan.lairs.push(Lair { at, foe, count })
    };
    let rim = self.rim(room);
    for &at in rim.iter() {
      if self.speck(at, 0x0A2) < 0.12 && room.theme != Theme::Cavern {
        self.put(at, Block::Urn)
      }
      if self.speck(at, 0x0A3) < 0.18 {
        self.put(at + IVec3::Y * (tall - 1), Block::Cobweb)
      }
    }
    match room.theme {
      Theme::Pillars | Theme::Vault => {
        let gap = if room.theme == Theme::Vault { 4 } else { 3 };
        for at in self.floor_cells(room, 2) {
          let (dx, dz) = (at.x - low.x - 2, at.z - low.z - 2);
          if dx % gap == 0
            && dz % gap == 0
            && !Plan::near_door(room, at, 2)
            && (at - centre).abs().max_element() > 2
          {
            for up in 0..tall {
              let block = match up % 4 == 1 {
                true => Block::CarvedStone,
                false => self.masonry(at + IVec3::Y * up)
              };
              self.put(at + IVec3::Y * up, block)
            }
            let beside = at + side(IVec3::X) * if dx % (gap * 2) == 0 { 1 } else { -1 };
            if self.speck(at, 0xB2) < 0.4 && !Plan::near_door(room, beside, 2) {
              self.light(beside)
            }
          }
        }
        match room.theme {
          Theme::Vault => {
            for dx in -2..=2 {
              for dz in -2..=2 {
                self.put(centre + IVec3::new(dx, 0, dz), Block::CarvedStone)
              }
            }
            self.put(centre + IVec3::Y, Block::Chest);
            self.chests.push((centre + IVec3::Y, 2));
            for corner in [
              IVec3::new(2, 1, 2),
              IVec3::new(-2, 1, 2),
              IVec3::new(2, 1, -2),
              IVec3::new(-2, 1, -2)
            ] {
              self.light(centre + corner)
            }
            lair(self, centre + IVec3::new(0, 0, 5), Foe::Warden, 1);
            lair(self, centre + IVec3::new(4, 0, 0), Foe::Skeleton, 2 + more);
          }
          _ => {
            lair(self, centre, Foe::Skeleton, 2 + more);
            if self.chance(0.3) {
              lair(self, centre, Foe::Shade, 1)
            }
          }
        }
      }
      Theme::Crypt | Theme::Ossuary => {
        let along_x = high.x - low.x >= high.z - low.z;
        for at in self.floor_cells(room, 2) {
          let (across, along) = match along_x {
            true => (at.z - low.z, at.x - low.x),
            false => (at.x - low.x, at.z - low.z)
          };
          let span = if along_x { high.z - low.z } else { high.x - low.x };
          let row = across == 2 || across == span - 2;
          if row && along % 4 < 2 && !Plan::near_door(room, at, 2) {
            self.put(at, Block::CarvedStone);
            if self.speck(at, 0xB0) < 0.5 {
              self.put(at + IVec3::Y, Block::BonePile)
            }
          }
        }
        for &at in rim.iter() {
          if self.speck(at, 0x91) < 0.25 {
            self.put(at + IVec3::Y, Block::BonePile)
          }
        }
        let candle = centre + if along_x { IVec3::Z } else { IVec3::X };
        self.light(candle);
        lair(self, centre, Foe::Skeleton, 2 + more);
        if room.theme == Theme::Ossuary {
          lair(self, centre, Foe::Shade, 1)
        }
        if self.chance(0.65) && !rim.is_empty() {
          let spot = rim[self.range(0, rim.len() as i32 - 1) as usize];
          self.put(spot, Block::Chest);
          self.chests.push((spot, 1))
        }
      }
      Theme::Cavern => {
        let span = (high - low).max_element();
        for x in low.x - 1..=high.x + 1 {
          for z in low.z - 1..=high.z + 1 {
            for y in low.y - 1..=high.y + 1 {
              let at = IVec3::new(x, y, z);
              let open = self.cells.get(&at) == Some(&Stamp::Open);
              let bordered = !open
                && [
                  IVec3::X,
                  IVec3::NEG_X,
                  IVec3::Y,
                  IVec3::NEG_Y,
                  IVec3::Z,
                  IVec3::NEG_Z
                ]
                .into_iter()
                .any(|way| self.cells.get(&(at + way)) == Some(&Stamp::Open));
              if bordered && self.speck(at, 0xAD7A) < 0.06 {
                self.put(at, Block::Lumen)
              }
            }
          }
        }
        for _ in 0..span / 2 {
          let at = IVec3::new(
            self.range(low.x + 1, high.x - 1),
            low.y,
            self.range(low.z + 1, high.z - 1)
          );
          if self.cells.get(&at) == Some(&Stamp::Open) && !Plan::near_door(room, at, 2) {
            let tall = self.range(1, 3);
            for up in 0..tall {
              self.put(at + IVec3::Y * up, Block::Stone)
            }
          }
        }
        for at in self.floor_cells(room, 1) {
          if self.cells.get(&at) == Some(&Stamp::Open) {
            match self.speck(at, 0x5B) {
              roll if roll < 0.05 => self.put(at, Block::BrownMushroom),
              roll if roll < 0.12 => self.put(at, Block::Cobweb),
              _ => ()
            }
          }
        }
        lair(self, centre, Foe::Spider, 2 + more);
      }
      Theme::Chasm => {
        let along_x =
          room.doors.first().is_some_and(|door| door.x < low.x || door.x > high.x);
        let (from, to) = match along_x {
          true => (low.x + (high.x - low.x) / 3, high.x - (high.x - low.x) / 3),
          false => (low.z + (high.z - low.z) / 3, high.z - (high.z - low.z) / 3)
        };
        let bridge = match along_x {
          true => centre.z,
          false => centre.x
        };
        let molten = self.chance(0.6);
        for at in self.floor_cells(room, 0) {
          let (along, across) = match along_x {
            true => (at.x, at.z),
            false => (at.z, at.x)
          };
          if (from..=to).contains(&along)
            && across != bridge
            && !Plan::near_door(room, at, 2)
          {
            for down in 1..=9 {
              let cell = at - IVec3::Y * down;
              let block = match (down, molten) {
                (9, _) => Block::Basalt,
                (7 | 8, true) => Block::Lava,
                (7 | 8, false) => Block::Water,
                _ => Block::Air
              };
              self.put(cell, block)
            }
            for wall in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
              for down in 1..=9 {
                let cell = at + wall - IVec3::Y * down;
                self.stamp(cell, Stamp::Shell(self.masonry(cell)))
              }
            }
          }
        }
        for corner in [
          IVec3::new(low.x + 1, low.y, low.z + 1),
          IVec3::new(high.x - 1, low.y, high.z - 1),
          IVec3::new(low.x + 1, low.y, high.z - 1),
          IVec3::new(high.x - 1, low.y, low.z + 1)
        ] {
          if !Plan::near_door(room, corner, 2) {
            self.light(corner)
          }
        }
        let far = match along_x {
          true => IVec3::new(to + 2, low.y, bridge),
          false => IVec3::new(bridge, low.y, to + 2)
        };
        match molten {
          true => lair(self, far, Foe::Imp, 2 + more / 2),
          false => lair(self, far, Foe::Shade, 2 + more / 2)
        }
      }
      Theme::Flooded => {
        let path =
          room.doors.iter().copied().fold(Vec::new(), |mut path: Vec<IVec3>, door| {
            let toward = (centre - door).signum();
            let mut at = door.with_y(low.y);
            while at.xz() != centre.xz() {
              match (at.x != centre.x, at.z != centre.z) {
                (true, _) => at.x += toward.x,
                _ => at.z += toward.z
              }
              path.push(at)
            }
            path
          });
        for at in self.floor_cells(room, 1) {
          let dry = path.contains(&at) || (at - centre).abs().max_element() <= 1;
          if !dry {
            for down in 1..=2 {
              self.put(at - IVec3::Y * down, Block::Water)
            }
            self.stamp(at - IVec3::Y * 3, Stamp::Shell(Block::AncientBrick))
          }
        }
        for corner in [
          IVec3::new(low.x + 1, low.y, low.z + 1),
          IVec3::new(high.x - 1, low.y, high.z - 1),
          IVec3::new(low.x + 1, low.y, high.z - 1),
          IVec3::new(high.x - 1, low.y, low.z + 1)
        ] {
          for up in -2..tall {
            self.put(corner + IVec3::Y * up, Block::Lumen)
          }
        }
        self.light(centre);
        lair(self, centre, Foe::Spider, 1 + more);
        lair(self, centre, Foe::Shade, 1);
        if self.chance(0.5) {
          self.put(centre + IVec3::X, Block::Chest);
          self.chests.push((centre + IVec3::X, 1))
        }
      }
    }
  }

  fn shrine(&mut self, at: IVec3, way: IVec3) {
    let ground = at.y - 1;
    self.clearing(at, 7, 16);
    for dx in -4..=4 {
      for dz in -4..=4 {
        let cell = IVec3::new(at.x + dx, ground, at.z + dz);
        let rim = dx.abs() == 4 || dz.abs() == 4;
        self.stamp(
          cell,
          Stamp::Floor(if rim { self.masonry(cell) } else { Block::AncientBrick })
        );
        for below in 1..=3 {
          self.stamp(cell - IVec3::Y * below, Stamp::Floor(Block::AncientBrick))
        }
        for up in 1..=7 {
          self.stamp(cell + IVec3::Y * up, Stamp::Open)
        }
      }
    }
    for corner in [
      IVec3::new(4, 0, 4),
      IVec3::new(-4, 0, 4),
      IVec3::new(4, 0, -4),
      IVec3::new(-4, 0, -4)
    ] {
      let base = at + corner;
      let tall = self.range(1, 5);
      for up in 0..tall {
        self.put(
          base + IVec3::Y * up,
          if up == 1 { Block::CarvedStone } else { self.masonry(base + IVec3::Y * up) }
        )
      }
    }
    let across = side(way);
    let mouth = at - way;
    for lateral in [-2, 2] {
      for up in 0..5 {
        self.put(mouth + across * lateral + IVec3::Y * up, Block::CarvedStone)
      }
    }
    for lateral in -2..=2 {
      self.put(
        mouth + across * lateral + IVec3::Y * 5,
        self.masonry(mouth + across * lateral)
      )
    }
    self.light(at + way * 3 + across * 3);
    self.light(at + way * 3 - across * 3)
  }

  fn clearing(&mut self, at: IVec3, radius: i32, tall: i32) {
    for dx in -radius..=radius {
      for dz in -radius..=radius {
        if dx * dx + dz * dz <= radius * radius {
          for up in 0..tall {
            self.stamp(at + IVec3::new(dx, up, dz), Stamp::Clear)
          }
        }
      }
    }
  }

  fn spire(&mut self, at: IVec3, way: IVec3) -> IVec3 {
    const RADIUS: f32 = 5.0;
    const STOREY: i32 = 6;
    let storeys = self.range(3, 4);
    let top = at.y - 1 + storeys * STOREY;
    self.clearing(at, 9, storeys * STOREY + 14);
    let ladder = IVec2::new(0, -4);
    for dx in -6..=6 {
      for dz in -6..=6 {
        let distance = Vec2::new(dx as f32, dz as f32).length();
        let sector = ((dz as f32).atan2(dx as f32) * 4.0) as i32;
        let crown = top + 2 - (hash(self.key, sector, 3, 1) % 4) as i32;
        for y in at.y - 4..=top + 2 {
          let cell = IVec3::new(at.x + dx, y, at.z + dz);
          let level = (y - (at.y - 1)).rem_euclid(STOREY);
          let floor = level == 0 && y <= top;
          let column = IVec2::new(dx, dz);
          if distance < RADIUS - 0.5 {
            let stamp = match (floor, column == ladder) {
              _ if y < at.y - 1 => Stamp::Floor(Block::AncientBrick),
              (true, true) if y > at.y - 1 => Stamp::Put(Block::Ladder),
              (true, _) if y > at.y - 1 && self.speck(cell, 0xF1) < 0.12 => Stamp::Open,
              (true, _) => Stamp::Floor(if y == at.y - 1 {
                Block::AncientBrick
              } else {
                Block::Planks
              }),
              (false, true) if y < top => Stamp::Put(Block::Ladder),
              _ => Stamp::Open
            };
            self.stamp(cell, stamp)
          } else if distance < RADIUS + 0.5 {
            let window =
              (2..=3).contains(&level) && sector.rem_euclid(3) == 0 && y > at.y;
            let crumbled = y > crown
              || perlin3(
                self.key ^ 0x7A,
                cell.x as f32 / 4.0,
                y as f32 / 5.0,
                cell.z as f32 / 4.0
              ) > 0.55 - (y - at.y) as f32 * 0.012;
            match (window || (crumbled && y > at.y + 2), y < at.y - 1) {
              (_, true) => self.stamp(cell, Stamp::Floor(Block::AncientBrick)),
              (true, _) => (),
              _ => self.stamp(cell, Stamp::Floor(self.masonry(cell)))
            }
          } else if distance < RADIUS + 1.5 && y >= at.y - 1 && y <= at.y + 1 {
            self.stamp(cell, Stamp::Open)
          }
        }
      }
    }
    let across = side(way);
    let door = at + way * 5;
    for lateral in -1..=1 {
      for up in 0..3 {
        self.stamp(door + across * lateral + IVec3::Y * up, Stamp::Open)
      }
    }
    for storey in 0..storeys {
      let floor = at + IVec3::Y * (storey * STOREY);
      let brazier = floor + IVec3::new(3, 0, 1);
      self.light(brazier);
      let foe = if storey + 1 == storeys { Foe::Shade } else { Foe::Skeleton };
      self.lairs.push(Lair { at: floor + IVec3::new(0, 0, 1), foe, count: 2 });
    }
    let summit = at + IVec3::Y * ((storeys - 1) * STOREY) + IVec3::new(-2, 0, 2);
    self.put(summit, Block::Chest);
    self.chests.push((summit, 2));
    at
  }

  pub(crate) fn finish(self, cell: IVec2, style: Style, entry: IVec3) -> Dungeon {
    let (low, high) = self
      .cells
      .keys()
      .fold((IVec3::MAX, IVec3::MIN), |(low, high), &at| (low.min(at), high.max(at)));
    let stamps = self.cells.into_iter().fold(
      HashMap::<IVec3, Vec<(u16, Stamp)>>::default(),
      |mut stamps, (at, stamp)| {
        let local = local_of(at);
        stamps
          .entry(chunk_of(at))
          .or_default()
          .push((((local.y * SIZE + local.z) * SIZE + local.x) as u16, stamp));
        stamps
      }
    );
    Dungeon {
      cell,
      style,
      entry,
      low,
      high,
      lights: self.lights,
      lairs: self.lairs,
      chests: self.chests,
      stamps
    }
  }
}

fn hub(plan: &mut Plan, level: i32) -> Theme {
  match (level, plan.roll()) {
    (0, roll) if roll < 0.6 => Theme::Pillars,
    (0, _) => Theme::Crypt,
    (_, roll) if roll < 0.3 => Theme::Chasm,
    (_, roll) if roll < 0.5 => Theme::Cavern,
    (_, roll) if roll < 0.7 => Theme::Flooded,
    _ => Theme::Pillars
  }
}

fn nook(plan: &mut Plan) -> Theme {
  match plan.roll() {
    roll if roll < 0.35 => Theme::Crypt,
    roll if roll < 0.6 => Theme::Ossuary,
    roll if roll < 0.82 => Theme::Cavern,
    _ => Theme::Flooded
  }
}

fn size(plan: &mut Plan, theme: Theme) -> (i32, i32, i32) {
  match theme {
    Theme::Vault => (17, 17, 9),
    Theme::Cavern => (plan.range(6, 8) * 2 + 1, plan.range(12, 17), plan.range(7, 10)),
    Theme::Chasm => (plan.range(5, 6) * 2 + 1, plan.range(15, 19), plan.range(7, 9)),
    Theme::Pillars => (plan.range(5, 7) * 2 + 1, plan.range(11, 16), plan.range(6, 8)),
    Theme::Flooded => (plan.range(4, 6) * 2 + 1, plan.range(10, 14), 6),
    Theme::Crypt | Theme::Ossuary => (plan.range(3, 4) * 2 + 1, plan.range(9, 14), 5)
  }
}

fn delve(plan: &mut Plan, mut at: IVec3, mut way: IVec3, levels: i32) {
  let fall = ((at.y - DEEPEST) / (levels + 1)).clamp(7, 16);
  let mut rooms = Vec::new();
  for level in 0..levels {
    let last = level + 1 == levels;
    let floor = (at.y - fall).max(DEEPEST);
    (at, way) = plan.descend(at, way, floor);
    let run = plan.range(2, 5);
    at = plan.walk(at, way, run);
    let theme = match (last, levels > 1) {
      (true, true) => Theme::Vault,
      (true, false) => Theme::Crypt,
      _ => hub(plan, level)
    };
    let measure = size(plan, theme);
    let mut room = plan.chamber(at, way, measure, theme, level);
    let aside = match plan.chance(0.5) {
      true => side(way),
      false => -side(way)
    };
    if !last || plan.chance(0.5) {
      let door = plan.exit(&mut room, aside);
      let reach = plan.range(4, 9);
      let mut path = door;
      for _ in 0..reach {
        path += aside;
        if plan.chance(0.3) && path.y > DEEPEST + 2 {
          path.y -= 1
        }
        plan.section(path, aside)
      }
      let theme = nook(plan);
      let measure = size(plan, theme);
      rooms.push(plan.chamber(path, aside, measure, theme, level));
    }
    if !last {
      let onward = match plan.chance(0.6) {
        true => way,
        false => -aside
      };
      at = plan.exit(&mut room, onward);
      way = onward
    }
    rooms.push(room);
  }
  for room in rooms.iter() {
    plan.decorate(room)
  }
}

fn found(seed: u32, cell: IVec2) -> Option<Dungeon> {
  match Island::at(seed, cell) {
    Some(island) => delved(seed, cell, island),
    None => crate::precursor::found(seed, cell)
  }
}

fn delved(seed: u32, cell: IVec2, island: Island) -> Option<Dungeon> {
  {
    let key = hash(seed ^ 0xD0C0, cell.x, 0x77, cell.y);
    let home = cell == IVec2::ZERO;
    let mut plan = Plan {
      key,
      rolls: 0,
      centre: island.centre,
      radius: island.radius,
      cells: default(),
      lights: default(),
      lairs: default(),
      chests: default()
    };
    let wanted = island.radius >= 20.0 && (home || plan.chance(0.6));
    let spot = (0..8).find_map(|attempt| {
      let angle = plan.roll() * std::f32::consts::TAU + attempt as f32;
      let reach = island.radius * (0.25 + plan.roll() * 0.2);
      let point = (island.centre + Vec2::from_angle(angle) * reach).floor().as_ivec2();
      let ground = generate::height(seed, point.x, point.y);
      let flat =
        [IVec2::new(4, 0), IVec2::new(-4, 0), IVec2::new(0, 4), IVec2::new(0, -4)]
          .into_iter()
          .all(|offset| {
            (generate::height(seed, point.x + offset.x, point.y + offset.y) - ground)
              .abs()
              <= 3
          });
      let clear = generate::Waystone::at(seed, cell).is_none_or(|stone| {
        point.as_vec2().distance(stone.at.xz().as_vec2()) > KEEP_CLEAR
      });
      (ground > SEA + 2 && flat && clear)
        .then(|| IVec3::new(point.x, ground + 1, point.y))
    });
    spot.filter(|_| wanted).map(|entry| {
      let way = WAYS[plan.range(0, 3) as usize];
      let style = match island.kind {
        _ if home => Style::Descent,
        Kind::Volcano | Kind::Frost => Style::Descent,
        _ if plan.chance(0.3) => Style::Spire,
        _ => Style::Descent
      };
      let levels =
        (2 + i32::from(island.radius > 34.0) + i32::from(entry.y > SEA + 30)).min(4);
      match style {
        Style::Descent => {
          plan.shrine(entry, way);
          let start = entry - way;
          plan.section(start, way);
          delve(&mut plan, start, way, levels)
        }
        Style::Spire | Style::Precursor => {
          let base = plan.spire(entry, way);
          let down = if way.x == 0 { IVec3::X } else { IVec3::Z };
          let start = base + down;
          plan.section(start, down);
          delve(&mut plan, start, down, 1)
        }
      }
      plan.finish(cell, style, entry)
    })
  }
}

impl Dungeon {
  pub fn at(seed: u32, cell: IVec2) -> Option<Arc<Dungeon>> {
    static KNOWN: LazyLock<RwLock<HashMap<(u32, IVec2), Option<Arc<Dungeon>>>>> =
      LazyLock::new(default);
    let known = KNOWN.read().ok().and_then(|known| known.get(&(seed, cell)).cloned());
    known.unwrap_or_else(|| {
      let dungeon = found(seed, cell).map(Arc::new);
      if let Ok(mut known) = KNOWN.write() {
        known.insert((seed, cell), dungeon.clone());
      }
      dungeon
    })
  }

  pub fn within(seed: u32, low: IVec2, high: IVec2) -> Vec<Arc<Dungeon>> {
    let cell_of = |at: IVec2| at.div_euclid(IVec2::splat(crate::island::CELL));
    let (from, to) = (cell_of(low - REACH), cell_of(high + REACH));
    (from.x..=to.x)
      .flat_map(|x| (from.y..=to.y).map(move |y| IVec2::new(x, y)))
      .filter_map(|cell| Dungeon::at(seed, cell))
      .filter(|dungeon| {
        dungeon.low.xz().cmple(high).all() && dungeon.high.xz().cmpge(low).all()
      })
      .collect()
  }

  pub fn stamps(&self, key: IVec3) -> &[(u16, Stamp)] {
    self.stamps.get(&key).map_or(&[], Vec::as_slice)
  }

  pub fn chest(&self, at: IVec3) -> Option<u8> {
    self.chests.iter().find(|(chest, _)| *chest == at).map(|&(_, tier)| tier)
  }
}

pub fn lights_near(seed: u32, low: Vec3, high: Vec3) -> Vec<IVec3> {
  Dungeon::within(seed, low.xz().floor().as_ivec2(), high.xz().ceil().as_ivec2())
    .iter()
    .flat_map(|dungeon| dungeon.lights.iter().copied())
    .filter(|light| {
      let centre = light.as_vec3() + 0.5;
      centre.cmpge(low).all() && centre.cmple(high).all()
    })
    .collect()
}

pub fn loot(seed: u32, at: IVec3, tier: u8) -> Vec<Stack> {
  let roll =
    |salt: u32| unit(seed ^ 0x100F ^ salt.wrapping_mul(0x9E37), at.x, at.y, at.z);
  let table: [(Block, u16, u16); 10] = [
    (Block::ArcaneDust, 2, 6),
    (Block::BoneDust, 2, 5),
    (Block::Candle, 1, 3),
    (Block::LizardClaw, 1, 3),
    (Block::Feather, 2, 4),
    (Block::CoalOre, 3, 8),
    (Block::IronOre, 2, 5),
    (Block::GoldOre, 1, 3),
    (Block::Torch, 4, 8),
    (Block::Lumen, 1, 3)
  ];
  let precursor: [(Block, u16, u16); 7] = [
    (Block::AlloyScrap, 3, 9),
    (Block::AetherCell, 1, 2),
    (Block::ResonantCrystal, 1, 2),
    (Block::Datacore, 1, 1),
    (Block::Pearl, 1, 3),
    (Block::Torpedo, 2, 6),
    (Block::Biolume, 2, 5)
  ];
  let table: &[(Block, u16, u16)] = match tier >= crate::precursor::TIER {
    true => &precursor,
    false => &table
  };
  let picks = 2 + u32::from(tier) * 2;
  let mut found: Vec<(Block, u16)> = (0..picks)
    .map(|pick| {
      let (block, low, high) =
        table[(roll(pick * 2) * table.len() as f32) as usize % table.len()];
      (block, low + (roll(pick * 2 + 1) * f32::from(high - low + 1)) as u16)
    })
    .collect();
  if tier >= 2 {
    found.push((Block::DiamondOre, 1 + (roll(99) * 2.0) as u16));
    found.push((Block::ArcaneDust, 4))
  }
  let salt = hash(seed ^ 0x6EA3, at.x, at.y, at.z);
  found
    .into_iter()
    .map(|(block, count)| Stack::of(block, count))
    .chain((0..u32::from(tier.clamp(1, 2))).map(|pick| {
      crate::gear::found(seed, salt.wrapping_add(pick * 0x9E37), tier.min(2))
    }))
    .collect()
}

pub fn urn(seed: u32, at: IVec3, salt: u32) -> Option<Block> {
  let roll = unit(seed ^ 0x0B5 ^ salt, at.x, at.y, at.z);
  [Block::ArcaneDust, Block::BoneDust, Block::Candle, Block::Feather]
    .get((roll * 7.0) as usize)
    .copied()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_home_island_has_a_dungeon_with_lairs_lights_and_treasure() {
    let dungeon = Dungeon::at(1, IVec2::ZERO).expect("no dungeon at home");
    println!(
      "{:?} entry {} spans {} .. {} lights {} lairs {} chests {}",
      dungeon.style,
      dungeon.entry,
      dungeon.low,
      dungeon.high,
      dungeon.lights.len(),
      dungeon.lairs.len(),
      dungeon.chests.len()
    );
    for lair in dungeon.lairs.iter() {
      println!("lair {:?} x{} at {}", lair.foe, lair.count, lair.at)
    }
    for chest in dungeon.chests.iter() {
      println!("chest {chest:?}")
    }
    assert!(
      !dungeon.lights.is_empty()
        && !dungeon.lairs.is_empty()
        && !dungeon.chests.is_empty()
    );
    assert!(dungeon.low.y >= DEEPEST - 6, "{}", dungeon.low)
  }

  #[test]
  #[ignore]
  fn spires() {
    for dungeon in Dungeon::within(1, IVec2::splat(-700), IVec2::splat(700)).iter() {
      println!("{:?} {:?} entry {}", dungeon.cell, dungeon.style, dungeon.entry)
    }
  }

  #[test]
  fn dungeons_are_repeatable() {
    let count = |seed| {
      Dungeon::within(seed, IVec2::splat(-600), IVec2::splat(600))
        .iter()
        .map(|dungeon| dungeon.stamps.values().map(Vec::len).sum::<usize>())
        .collect::<Vec<_>>()
    };
    assert_eq!(count(3), count(3));
    assert!(!count(3).is_empty())
  }
}
