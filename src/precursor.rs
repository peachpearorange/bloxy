use {crate::{block::Block,
             dungeon::{Dungeon, Foe, Lair, Plan, Stamp, Style},
             generate::{self, ABYSS},
             island::{CELL, SEA},
             noise::{hash, unit}},
     bevy::prelude::*};

const CHANCE: f32 = 0.12;
const SHALLOWEST: i32 = SEA - 20;
const HALL: (i32, i32) = (8, 11);
const POD: i32 = 5;
const TUBE: i32 = 2;
const DECAY: f32 = 0.035;

pub const TIER: u8 = 3;

const WAYS: [IVec3; 4] = [IVec3::X, IVec3::Z, IVec3::NEG_X, IVec3::NEG_Z];

fn side(way: IVec3) -> IVec3 { IVec3::new(-way.z, 0, way.x) }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pod {
  Vault,
  Garden,
  Reactor
}

fn alloy(plan: &Plan, at: IVec3) -> Stamp {
  match plan.speck(at, 0xA110) {
    roll if roll < DECAY => Stamp::Flood,
    roll if roll < DECAY * 1.6 => Stamp::Shell(Block::Abyssite),
    _ => Stamp::Shell(Block::PrecursorAlloy)
  }
}

fn dome(plan: &mut Plan, centre: IVec3, radius: i32, windows: bool) {
  let reach = radius as f32;
  for x in -radius - 1..=radius + 1 {
    for z in -radius - 1..=radius + 1 {
      for y in -1..=radius + 1 {
        let offset = IVec3::new(x, y, z);
        let at = centre + offset;
        let distance = Vec3::new(x as f32, y as f32 * 1.15, z as f32).length();
        let flat = Vec2::new(x as f32, z as f32).length();
        let band = (2..=3).contains(&y) && (x.abs() <= 1 || z.abs() <= 1);
        let crown = y >= radius - 2 && flat < 2.5;
        let stamp = match () {
          () if y == -1 && flat <= reach + 0.5 => {
            Some(match (x + z).rem_euclid(4) == 0 {
              true => Stamp::Floor(Block::AetherConduit),
              false => Stamp::Floor(Block::PrecursorAlloy)
            })
          }
          () if y < 0 => None,
          () if distance <= reach - 0.5 => Some(Stamp::Flood),
          () if distance <= reach + 0.5 && windows && band => {
            Some(Stamp::Shell(Block::PrecursorGlass))
          }
          () if distance <= reach + 0.5 && crown => {
            Some(Stamp::Shell(Block::AetherConduit))
          }
          () if distance <= reach + 0.5 => Some(alloy(plan, at)),
          () => None
        };
        if let Some(stamp) = stamp {
          plan.stamp(at, stamp)
        }
      }
    }
  }
}

fn tube(plan: &mut Plan, from: IVec3, way: IVec3, length: i32) {
  let across = side(way);
  for step in 0..length {
    let at = from + way * step;
    let rib = step % 4 == 2;
    for lateral in -TUBE..=TUBE {
      for up in -1..=TUBE * 2 {
        let cell = at + across * lateral + IVec3::Y * up;
        let inner = lateral.abs() < TUBE && (0..TUBE * 2).contains(&up);
        let stamp = match () {
          () if inner => Stamp::Flood,
          () if up == -1 => Stamp::Floor(Block::PrecursorAlloy),
          () if rib => Stamp::Shell(Block::AetherConduit),
          () if up == 1 && lateral.abs() == TUBE => Stamp::Shell(Block::PrecursorGlass),
          () => alloy(plan, cell)
        };
        plan.stamp(cell, stamp)
      }
    }
  }
}

fn lamp(plan: &mut Plan, at: IVec3) {
  plan.put(at, Block::AetherLamp);
  plan.lights.push(at)
}

fn sanctum(plan: &mut Plan, centre: IVec3) {
  for x in -2..=2 {
    for z in -2..=2 {
      for y in 0..=4 {
        let at = centre + IVec3::new(x, y, z);
        let wall = x.abs() == 2 || z.abs() == 2 || y == 4;
        let corner = x.abs() == 2 && z.abs() == 2;
        let block = match (wall, corner || y == 4) {
          (true, true) => Block::PrecursorAlloy,
          (true, false) => Block::PrecursorGlass,
          _ => Block::Air
        };
        plan.put(at, block)
      }
    }
  }
  plan.put(centre, Block::Chest);
  plan.chests.push((centre, TIER));
  plan.put(centre + IVec3::new(1, 0, 1), Block::AetherLamp);
  plan.lights.push(centre + IVec3::new(1, 0, 1))
}

fn furnish(plan: &mut Plan, centre: IVec3, pod: Pod) {
  let ring = |turn: i32| {
    let angle = turn as f32 / 6.0 * std::f32::consts::TAU;
    centre
      + (Vec2::from_angle(angle) * (POD - 2) as f32).round().as_ivec2().extend(0).xzy()
  };
  match pod {
    Pod::Vault => {
      plan.put(centre, Block::Chest);
      plan.chests.push((centre, TIER));
      for turn in [0, 2, 4] {
        lamp(plan, ring(turn))
      }
      plan.lairs.push(Lair { at: centre + IVec3::Y * 2, foe: Foe::Sentinel, count: 2 })
    }
    Pod::Garden => {
      for turn in 0..6 {
        let at = ring(turn);
        plan.put(at - IVec3::Y, Block::Biolume);
        plan.put(at, [Block::GlowPolyp, Block::Anemone, Block::Kelp][turn as usize % 3]);
      }
      plan.put(centre, Block::BioLab);
      plan.put(centre + IVec3::X, Block::Chest);
      plan.chests.push((centre + IVec3::X, 2));
      lamp(plan, centre + IVec3::Y * 3)
    }
    Pod::Reactor => {
      for up in 0..=POD - 1 {
        plan.put(centre + IVec3::Y * up, Block::AetherConduit)
      }
      for turn in [1, 3, 5] {
        lamp(plan, ring(turn))
      }
      plan.put(ring(0), Block::Chest);
      plan.chests.push((ring(0), TIER));
      plan.lairs.push(Lair { at: ring(3), foe: Foe::Custodian, count: 1 })
    }
  }
}

fn site(seed: u32, cell: IVec2) -> Option<IVec3> {
  let rolled = unit(seed ^ 0x9EC0, cell.x, 0x31, cell.y) < CHANCE;
  let home = cell.abs().max_element() <= 1;
  (rolled && !home)
    .then(|| {
      (0..6).find_map(|attempt| {
        let pick = |salt: u32| {
          (hash(seed ^ salt, cell.x * 7 + attempt, 0x32, cell.y) % (CELL as u32 - 48))
            as i32
            + 24
        };
        let spot = cell * CELL + IVec2::new(pick(0x9EC1), pick(0x9EC2));
        let ground = generate::height(seed, spot.x, spot.y);
        let (low, high) = [-12, 0, 12]
          .into_iter()
          .flat_map(|dx| [-12, 0, 12].map(move |dz| IVec2::new(dx, dz)))
          .map(|offset| generate::height(seed, spot.x + offset.x, spot.y + offset.y))
          .fold((i32::MAX, i32::MIN), |(low, high), h| (low.min(h), high.max(h)));
        (high < SHALLOWEST && high - low <= 7)
          .then(|| IVec3::new(spot.x, ground + 1, spot.y))
      })
    })
    .flatten()
}

pub fn found(seed: u32, cell: IVec2) -> Option<Dungeon> {
  site(seed, cell).map(|entry| {
    let mut plan = Plan {
      key: hash(seed ^ 0x9EC3, cell.x, 0x33, cell.y),
      rolls: 0,
      centre: entry.xz().as_vec2(),
      radius: 30.0,
      cells: default(),
      lights: default(),
      lairs: default(),
      chests: default()
    };
    let radius = plan.range(HALL.0, HALL.1);
    dome(&mut plan, entry, radius, true);
    sanctum(&mut plan, entry);
    for corner in [
      IVec3::new(1, 0, 1),
      IVec3::new(-1, 0, 1),
      IVec3::new(1, 0, -1),
      IVec3::new(-1, 0, -1)
    ] {
      let pillar = entry + corner * (radius / 2 + 1);
      for up in 0..radius - 2 {
        plan.put(pillar + IVec3::Y * up, Block::AetherConduit)
      }
      lamp(&mut plan, pillar + corner * -1)
    }
    let first = plan.range(0, 3) as usize;
    let deep = entry.y < ABYSS;
    let pods: Vec<Pod> = match deep {
      true => vec![Pod::Reactor, Pod::Vault, Pod::Garden],
      false => vec![Pod::Garden, Pod::Vault]
    };
    for (index, pod) in pods.into_iter().enumerate() {
      let way = WAYS[(first + index) % 4];
      let door = entry + way * radius;
      let length = plan.range(7, 13);
      tube(&mut plan, door - way * 2, way, length + 4);
      let centre = door + way * (length + POD);
      dome(&mut plan, centre, POD, false);
      furnish(&mut plan, centre, pod)
    }
    let open = WAYS[(first + 3) % 4];
    for step in radius - 1..=radius + 1 {
      for lateral in -1..=1 {
        for up in 0..3 {
          plan.stamp(
            entry + open * step + side(open) * lateral + IVec3::Y * up,
            Stamp::Flood
          )
        }
      }
    }
    plan.lairs.push(Lair {
      at: entry + open * (radius - 3) + IVec3::Y * 3,
      foe: Foe::Sentinel,
      count: 2 + u8::from(deep)
    });
    plan.finish(cell, Style::Precursor, entry + open * (radius + 1))
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn precursor_bases_lie_deep_with_guards_and_caches() {
    let bases: Vec<_> = Dungeon::within(1, IVec2::splat(-2500), IVec2::splat(2500))
      .into_iter()
      .filter(|dungeon| dungeon.style == Style::Precursor)
      .collect();
    for base in bases.iter() {
      println!(
        "base {:?} entry {} lairs {} chests {}",
        base.cell,
        base.entry,
        base.lairs.len(),
        base.chests.len()
      )
    }
    assert!(!bases.is_empty(), "no precursor bases");
    assert!(bases.iter().all(|base| base.entry.y < SHALLOWEST + 2
      && !base.lairs.is_empty()
      && !base.chests.is_empty()))
  }
}
