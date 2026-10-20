use {crate::{block::Block,
             machine::{Works, deliver, side_of},
             protocol::{Altered, Loose, Put, Stack, authority},
             voxels::Voxels},
     bevy::prelude::*,
     bevy_replicon::prelude::*};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slope {
  Flat,
  Up,
  Down
}

const HEADINGS: [IVec3; 4] = [IVec3::NEG_Z, IVec3::X, IVec3::Z, IVec3::NEG_X];

const BELTS: [(Slope, [Block; 4]); 3] = [
  (Slope::Flat, [
    Block::Conveyor,
    Block::ConveyorEast,
    Block::ConveyorSouth,
    Block::ConveyorWest
  ]),
  (Slope::Up, [
    Block::ConveyorUp,
    Block::ConveyorUpEast,
    Block::ConveyorUpSouth,
    Block::ConveyorUpWest
  ]),
  (Slope::Down, [
    Block::ConveyorDown,
    Block::ConveyorDownEast,
    Block::ConveyorDownSouth,
    Block::ConveyorDownWest
  ])
];

pub fn belt(block: Block) -> Option<(IVec3, Slope)> {
  BELTS.iter().find_map(|&(slope, blocks)| {
    blocks.iter().position(|&belt| belt == block).map(|index| (HEADINGS[index], slope))
  })
}

pub fn shaped(heading: IVec3, slope: Slope) -> Block {
  let index = HEADINGS.iter().position(|&known| known == heading).unwrap_or(0);
  BELTS
    .iter()
    .find(|&&(known, _)| known == slope)
    .map_or(Block::Conveyor, |&(_, blocks)| blocks[index])
}

pub fn laid(near: impl Fn(IVec3) -> Option<Block>, at: IVec3, heading: IVec3) -> Block {
  let belted = |cell: IVec3| near(cell).is_some_and(Block::conveyor);
  shaped(heading, match () {
    () if belted(at + heading + IVec3::Y) => Slope::Up,
    () if belted(at + heading - IVec3::Y) => Slope::Down,
    () => Slope::Flat
  })
}

const BELT_TOP: f32 = 3.0 / 16.0;
pub const SPEED: f32 = 1.6;
const GAP: f32 = 0.3;
const CENTRING: f32 = 3.0;
const HOLD_AT: f32 = 0.8;
const CATCH: f32 = 0.3;

fn along(cell: IVec3, heading: IVec3, point: Vec3) -> f32 {
  let local = point - cell.as_vec3();
  let flat = Vec2::new(local.x - 0.5, local.z - 0.5).dot(heading.xz().as_vec2());
  (flat + 0.5).clamp(0.0, 1.0)
}

fn surface(cell: IVec3, heading: IVec3, slope: Slope, point: Vec3) -> f32 {
  let t = along(cell, heading, point);
  cell.y as f32
    + BELT_TOP
    + match slope {
      Slope::Flat => 0.0,
      Slope::Up => t,
      Slope::Down => 1.0 - t
    }
}

#[derive(Clone, Copy)]
struct Support {
  cell: IVec3,
  heading: IVec3,
  slope: Slope
}

fn support(voxels: &Voxels, point: Vec3) -> Option<Support> {
  let below = point.floor().as_ivec3();
  [below, below - IVec3::Y].into_iter().find_map(|cell| {
    voxels
      .block(cell)
      .and_then(belt)
      .map(|(heading, slope)| Support { cell, heading, slope })
      .filter(|&Support { cell, heading, slope }| {
        (point.y - surface(cell, heading, slope, point)).abs() < CATCH
      })
  })
}

#[derive(Component)]
pub struct Riding;

fn reshape(
  mut puts: MessageReader<FromClient<Put>>,
  mut voxels: ResMut<Voxels>,
  mut changes: MessageWriter<ToClients<Altered>>
) {
  for &FromClient { message: Put { at, block }, .. } in puts.read() {
    if let Some((heading, _)) = belt(block)
      && voxels.ensure(at) == block
    {
      for (cell, slope) in
        [(at - heading - IVec3::Y, Slope::Up), (at - heading + IVec3::Y, Slope::Down)]
      {
        if voxels.ensure(cell) == shaped(heading, Slope::Flat) {
          let block = shaped(heading, slope);
          voxels.set(cell, block);
          changes.write(ToClients {
            targets: SendTargets::All,
            message: Altered { at: cell, block }
          });
        }
      }
    }
  }
}

fn convey(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut works: ResMut<Works>,
  mut coffers: ResMut<crate::chest::Coffers>,
  mut loose: Query<(Entity, &mut Loose, &mut crate::loose::Tumble, Has<Riding>)>,
  mut changes: MessageWriter<ToClients<Altered>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  let riders: Vec<(Entity, Vec3)> = loose
    .iter()
    .filter(|(.., riding)| *riding)
    .map(|(entity, loose, ..)| (entity, loose.at))
    .collect();
  for (entity, mut item, mut tumble, riding) in loose.iter_mut() {
    let at = item.at;
    match (support(&voxels, at), riding) {
      (Some(_), false) if tumble.rising() => (),
      (Some(_), false) => {
        commands.entity(entity).insert(Riding);
      }
      (None, true) => {
        commands.entity(entity).remove::<Riding>();
      }
      (Some(Support { cell, heading, slope }), true) => {
        let toward = heading.as_vec3();
        let crowded = riders.iter().any(|&(other, there)| {
          let offset = there - at;
          let ahead = offset.dot(toward);
          other != entity
            && ahead > 0.0
            && ahead < GAP
            && (offset - toward * ahead).with_y(0.0).length() < 0.25
            && offset.y.abs() < 0.6
        });
        let t = along(cell, heading, at);
        let front = cell
          + heading
          + match slope {
            Slope::Up => IVec3::Y,
            _ => IVec3::ZERO
          };
        let next_belt = [front, front - IVec3::Y]
          .into_iter()
          .any(|near| voxels.block(near).is_some_and(Block::conveyor));
        let ahead = voxels.block(front).unwrap_or_default();
        let end = !next_belt && t >= HOLD_AT;
        let stack = Stack { block: item.block, count: item.count, gear: item.gear };
        let delivered = end
          && deliver(
            &mut works.states,
            &mut coffers,
            &mut voxels,
            &mut changes,
            front,
            side_of(-heading),
            stack
          );
        let blocked = end && (ahead.solid() || ahead.conveyor());
        match (delivered, crowded || blocked) {
          (true, _) => {
            commands.entity(entity).despawn();
          }
          (false, true) => tumble.hold(),
          (false, false) => {
            let middle = cell.as_vec3() + Vec3::splat(0.5);
            let offset = (at - middle).with_y(0.0);
            let lateral = offset - toward * offset.dot(toward);
            let moved = at + toward * SPEED * dt - lateral * (CENTRING * dt).min(1.0);
            let lifted = moved.with_y(surface(cell, heading, slope, moved));
            tumble.carry(toward * SPEED);
            item.at = lifted
          }
        }
      }
      (None, false) => ()
    }
  }
}

pub fn surface_at(voxels: &Voxels, cell: IVec3, point: Vec3) -> Option<f32> {
  voxels
    .block(cell)
    .and_then(belt)
    .map(|(heading, slope)| surface(cell, heading, slope, point))
}

pub fn carry(voxels: &Voxels, feet: Vec3) -> Vec3 {
  let cell = (feet + Vec3::Y * 0.05).floor().as_ivec3();
  voxels.block(cell).and_then(belt).map_or(Vec3::ZERO, |(heading, _)| {
    let toward = heading.as_vec3();
    let offset = (feet - (cell.as_vec3() + Vec3::splat(0.5))).with_y(0.0);
    let lateral = offset - toward * offset.dot(toward);
    toward * SPEED - lateral * CENTRING
  })
}

const NUDGE: f32 = 0.6;
const CREATURE: crate::player::Bulk = crate::player::Bulk { half: 0.3, tall: 0.9 };

fn nudged(voxels: &Voxels, at: Vec3, dt: f32) -> Vec3 {
  let push = carry(voxels, at) * NUDGE * dt;
  [0, 2].into_iter().fold(at, |at, axis| match push[axis] == 0.0 {
    true => at,
    false => crate::player::slide(voxels, CREATURE, at, axis, push[axis]).0
  })
}

fn jostle(
  time: Res<Time>,
  voxels: Res<Voxels>,
  mut beasts: Query<&mut crate::protocol::Beast>,
  mut folk: Query<&mut crate::protocol::Folk>,
  mut hoppers: Query<&mut crate::protocol::Hopper>
) {
  let dt = time.delta_secs().min(0.1);
  for mut beast in beasts.iter_mut() {
    let at = nudged(&voxels, beast.at, dt);
    if at != beast.at {
      beast.at = at
    }
  }
  for mut person in folk.iter_mut().filter(|person| !person.aboard) {
    let at = nudged(&voxels, person.at, dt);
    if at != person.at {
      person.at = at
    }
  }
  for mut hopper in hoppers.iter_mut().filter(|hopper| !hopper.aloft) {
    let at = nudged(&voxels, hopper.at, dt);
    if at != hopper.at {
      hopper.at = at
    }
  }
}

pub struct Conveying;

impl Plugin for Conveying {
  fn build(&self, app: &mut App) {
    app.add_systems(
      Update,
      (reshape, convey, jostle)
        .chain()
        .before(crate::loose::Settling)
        .run_if(authority)
        .run_if(resource_exists::<Voxels>)
    );
  }
}
