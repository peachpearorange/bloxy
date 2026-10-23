use {crate::{authority::Controller, block::Block, noise::unit, protocol::*,
             voxels::Voxels},
     bevy::prelude::*,
     bevy_replicon::prelude::*};

const GROW_EVERY: f32 = 3.0;
const TRIES: i32 = 48;
const AROUND: i32 = 14;
const TALLEST_KELP: i32 = 12;
const TALLEST_GLOWKELP: i32 = 3;
const CROWD: usize = 7;

const SIDES: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];

fn column(voxels: &mut Voxels, at: IVec3, block: Block) -> i32 {
  (1..=TALLEST_KELP)
    .take_while(|&down| voxels.ensure(at - IVec3::Y * down) == block)
    .count() as i32
}

fn crowded(voxels: &mut Voxels, at: IVec3) -> bool {
  (-2..=2)
    .flat_map(|x| (-1..=1).flat_map(move |y| (-2..=2).map(move |z| IVec3::new(x, y, z))))
    .filter(|&offset| voxels.ensure(at + offset).organism())
    .count()
    >= CROWD
}

pub fn sprout(voxels: &mut Voxels, at: IVec3, roll: f32) -> Option<(IVec3, Block)> {
  let block = voxels.ensure(at);
  let above = at + IVec3::Y;
  match block {
    Block::Kelp
      if voxels.ensure(above) == Block::Water
        && voxels.ensure(above + IVec3::Y).wet()
        && column(voxels, at, Block::Kelp) < TALLEST_KELP =>
    {
      Some((above, Block::Kelp))
    }
    Block::Glowkelp
      if voxels.ensure(above) == Block::Air
        && column(voxels, at, Block::Glowkelp) < TALLEST_GLOWKELP =>
    {
      Some((above, Block::Glowkelp))
    }
    living if living.organism() && living != Block::Kelp && roll < 0.35 => {
      let spot = at + SIDES[(roll * 40.0) as usize % 4];
      let grounded = voxels.ensure(spot - IVec3::Y).solid();
      (voxels.ensure(spot) == Block::Water && grounded && !crowded(voxels, spot))
        .then_some((spot, living))
    }
    _ => None
  }
}

fn grow(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  players: Query<&Avatar, With<Controller>>,
  mut changes: MessageWriter<ToClients<Altered>>,
  mut since: Local<f32>,
  mut beat: Local<i32>
) {
  *since += time.delta_secs();
  if *since >= GROW_EVERY {
    *since = 0.0;
    *beat += 1;
    let seed = voxels.seed;
    let homes: Vec<IVec3> =
      players.iter().map(|avatar| avatar.at.floor().as_ivec3()).collect();
    for home in homes {
      for attempt in 0..TRIES {
        let roll = |salt: u32| unit(seed ^ salt, *beat, attempt, home.x ^ home.z);
        let offset = IVec3::new(
          (roll(0xB10) * (AROUND * 2 + 1) as f32) as i32 - AROUND,
          (roll(0xB11) * 17.0) as i32 - 8,
          (roll(0xB12) * (AROUND * 2 + 1) as f32) as i32 - AROUND
        );
        let at = home + offset;
        if let Some((spot, block)) = sprout(&mut voxels, at, roll(0xB13)) {
          voxels.set(spot, block);
          changes.write(ToClients {
            targets: SendTargets::All,
            message: Altered { at: spot, block }
          });
        }
      }
    }
  }
}

pub struct Biology;

impl Plugin for Biology {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, grow.run_if(authority).run_if(resource_exists::<Voxels>));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn kelp_climbs_toward_the_surface_and_coral_spreads_on_the_floor() {
    let mut voxels = Voxels::new(1);
    let floor = IVec3::new(4000, 20, 4000);
    for x in -3..=3 {
      for z in -3..=3 {
        for y in 0..12 {
          let at = floor + IVec3::new(x, y, z);
          voxels.ensure(at);
          voxels.set(at, if y == 0 { Block::Sand } else { Block::Water })
        }
      }
    }
    voxels.set(floor + IVec3::Y, Block::Kelp);
    assert_eq!(
      sprout(&mut voxels, floor + IVec3::Y, 0.9),
      Some((floor + IVec3::Y * 2, Block::Kelp))
    );
    voxels.set(floor + IVec3::new(2, 1, 0), Block::FanCoral);
    let spread = sprout(&mut voxels, floor + IVec3::new(2, 1, 0), 0.1);
    assert!(
      spread
        .is_some_and(|(spot, block)| block == Block::FanCoral && spot.y == floor.y + 1)
    )
  }
}
