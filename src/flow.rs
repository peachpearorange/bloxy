use {crate::{block::{Block, Fluid},
             protocol::{Altered, authority},
             voxels::Voxels},
     bevy::prelude::*,
     bevy_replicon::prelude::*};

const LEVEL: [IVec3; 4] = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z];
const AROUND: [IVec3; 6] =
  [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z, IVec3::Y, IVec3::NEG_Y];
const PER_FRAME: usize = 400;

#[derive(Resource, Default)]
pub struct Flows(Vec<(f32, IVec3)>);

impl Flows {
  pub fn stir(&mut self, at: IVec3, when: f32) {
    for cell in std::iter::once(at).chain(AROUND.map(|side| at + side)) {
      self.0.push((when, cell))
    }
  }
}

fn replaceable(block: Block) -> bool {
  block == Block::Air || (block.modelled() && !block.ladder() && !block.bed())
}

fn next(voxels: &mut Voxels, at: IVec3) -> Option<Block> {
  let here = voxels.ensure(at);
  let above = voxels.ensure(at + IVec3::Y);
  let sides =
    LEVEL.map(|side| (voxels.ensure(at + side), voxels.ensure(at + side - IVec3::Y)));
  let touching: Vec<Fluid> = AROUND
    .map(|side| voxels.ensure(at + side))
    .iter()
    .filter_map(|block| block.liquid().map(|(fluid, _)| fluid))
    .collect();
  let falls = |block: Block, fluid: Fluid| {
    replaceable(block)
      || block.liquid().is_some_and(|(below, level)| below == fluid && level > 0)
  };
  let incoming = above.liquid().map(|(fluid, _)| (fluid, 1)).or_else(|| {
    sides
      .iter()
      .filter_map(|&(side, below)| {
        side
          .liquid()
          .filter(|&(fluid, level)| level < fluid.reach() && !falls(below, fluid))
          .map(|(fluid, level)| (fluid, level + 1))
      })
      .min_by_key(|&(_, level)| level)
  });
  let wanted = match (here.liquid(), incoming) {
    (Some((Fluid::Lava, 0)), _) if touching.contains(&Fluid::Water) => Block::Basalt,
    (Some((_, 0)), _) => here,
    (Some(_), None) => Block::Air,
    (Some(_), Some((fluid, level))) | (None, Some((fluid, level)))
      if replaceable(here) || here.fluid() =>
    {
      match touching.iter().any(|&other| other != fluid) {
        true => Block::Cobblestone,
        false => fluid.flowing(level)
      }
    }
    _ => here
  };
  (wanted != here).then_some(wanted)
}

fn flow(
  time: Res<Time>,
  mut flows: ResMut<Flows>,
  mut voxels: ResMut<Voxels>,
  mut changes: MessageWriter<ToClients<Altered>>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  let (ready, waiting): (Vec<_>, Vec<_>) =
    flows.0.drain(..).partition(|&(when, _)| when <= now);
  let mut cells: Vec<IVec3> = ready.into_iter().map(|(_, at)| at).collect();
  cells.sort_by_key(|cell| cell.to_array());
  cells.dedup();
  let later = cells.split_off(cells.len().min(PER_FRAME));
  flows.0 =
    waiting.into_iter().chain(later.into_iter().map(|cell| (now, cell))).collect();
  let altered: Vec<(IVec3, Block, Block)> = cells
    .into_iter()
    .filter_map(|at| {
      let before = voxels.ensure(at);
      next(&mut voxels, at).map(|block| (at, before, block))
    })
    .collect();
  for (at, before, block) in altered.into_iter() {
    voxels.set(at, block);
    if before.modelled() && !before.fluid() {
      crate::loose::fall(&mut commands, before.drop(), at)
    }
    changes
      .write(ToClients { targets: SendTargets::All, message: Altered { at, block } });
    let lava = [before, block]
      .iter()
      .any(|block| block.liquid().is_some_and(|(fluid, _)| fluid == Fluid::Lava));
    let fluid = if lava { Fluid::Lava } else { Fluid::Water };
    flows.stir(at, now + fluid.delay())
  }
}

pub struct Flowing;

impl Plugin for Flowing {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Flows>()
      .add_systems(Update, flow.run_if(authority).run_if(resource_exists::<Voxels>));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn settle(voxels: &mut Voxels, around: IVec3, rounds: usize) {
    for _ in 0..rounds {
      let span = 9;
      let changes: Vec<(IVec3, Block)> = (-span..=span)
        .flat_map(|x| {
          (-span..=span)
            .flat_map(move |y| (-span..=span).map(move |z| IVec3::new(x, y, z)))
        })
        .filter_map(|offset| {
          next(voxels, around + offset).map(|block| (around + offset, block))
        })
        .collect();
      for (at, block) in changes.into_iter() {
        voxels.set(at, block)
      }
    }
  }

  #[test]
  fn water_falls_spreads_and_dries() {
    let mut voxels = Voxels::new(1);
    let spring = IVec3::new(4000, 220, 4000);
    for x in -8..=8 {
      for z in -8..=8 {
        voxels.set(spring + IVec3::new(x, -3, z), Block::Stone)
      }
    }
    voxels.set(spring, Block::Water);
    settle(&mut voxels, spring, 12);
    assert_eq!(voxels.ensure(spring - IVec3::Y), Block::WaterFlow1);
    assert_eq!(voxels.ensure(spring - IVec3::Y * 2), Block::WaterFlow1);
    assert_eq!(voxels.ensure(spring + IVec3::new(3, -2, 0)), Block::WaterFlow4);
    assert_eq!(voxels.ensure(spring + IVec3::new(7, -2, 0)), Block::Air);
    voxels.set(spring, Block::Air);
    settle(&mut voxels, spring, 30);
    assert_eq!(voxels.ensure(spring + IVec3::new(3, -2, 0)), Block::Air);
  }

  #[test]
  fn lava_meeting_water_hardens() {
    let mut voxels = Voxels::new(1);
    let at = IVec3::new(4000, 220, 4100);
    voxels.set(at - IVec3::Y, Block::Stone);
    voxels.set(at + IVec3::X - IVec3::Y, Block::Stone);
    voxels.set(at, Block::Lava);
    voxels.set(at + IVec3::X * 2, Block::Water);
    voxels.set(at + IVec3::X * 2 - IVec3::Y, Block::Stone);
    settle(&mut voxels, at, 3);
    assert_eq!(voxels.ensure(at + IVec3::X), Block::Cobblestone);
  }
}
