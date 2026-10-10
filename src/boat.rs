use {crate::{authority::{Controller, player_of, within_reach},
             block::{Block, Fluid, Tile},
             menu::Menu,
             player::{Bulk, Pilot, Riding, Selected, captured, slide},
             protocol::*,
             stream::{Palette, ready_around},
             texture::uv_corner,
             voxels::Voxels},
     bevy::{prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*};

const HULL: Vec3 = Vec3::new(0.65, 0.45, 1.05);
const BULK: Bulk = Bulk { half: 0.6, tall: 0.5 };
const DRAFT: f32 = 0.04;
const THRUST: f32 = 7.0;
const TURN: f32 = 1.9;
const DRAG: f32 = 1.2;
const STEP: f32 = 1.0 / 60.0;
const STEER_EVERY: f32 = 0.05;

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

pub fn struck(vessel: &Vessel, from: Vec3, toward: Vec3) -> Option<f32> {
  let turn = Quat::from_rotation_y(-vessel.yaw);
  let (origin, direction) = (turn * (from - vessel.at), turn * toward);
  let (low, high) = (Vec3::new(-HULL.x, 0.0, -HULL.z), HULL);
  let (near, far) = (0..3).fold((0.0_f32, f32::INFINITY), |(near, far), axis| {
    match direction[axis] == 0.0 {
      true => match (low[axis]..=high[axis]).contains(&origin[axis]) {
        true => (near, far),
        false => (1.0, 0.0)
      },
      false => {
        let (first, second) = (
          (low[axis] - origin[axis]) / direction[axis],
          (high[axis] - origin[axis]) / direction[axis]
        );
        (near.max(first.min(second)), far.min(first.max(second)))
      }
    }
  });
  (near <= far).then_some(near)
}

pub fn planked(slabs: &[(Vec3, Vec3)], tint: [f32; 3]) -> Mesh {
  let mut hull = slabs
    .iter()
    .map(|&(low, high)| {
      let mut mesh =
        Cuboid::from_corners(low, high).mesh().build().translated_by((low + high) / 2.0);
      if let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
      {
        uvs
          .iter_mut()
          .for_each(|uv| *uv = uv_corner(Tile::Planks, Vec2::from(*uv)).to_array())
      }
      mesh
    })
    .reduce(|mut all, part| {
      all.merge(&part).ok();
      all
    })
    .unwrap_or_else(|| Cuboid::default().into());
  let [r, g, b] = tint;
  hull
    .insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[r, g, b, 1.0]; hull.count_vertices()]);
  hull
}

fn hull() -> Mesh {
  planked(
    &[
      (Vec3::new(-0.55, 0.0, -0.95), Vec3::new(0.55, 0.1, 0.95)),
      (Vec3::new(-0.65, 0.0, -1.05), Vec3::new(-0.55, HULL.y, 1.05)),
      (Vec3::new(0.55, 0.0, -1.05), Vec3::new(0.65, HULL.y, 1.05)),
      (Vec3::new(-0.55, 0.0, -1.05), Vec3::new(0.55, HULL.y, -0.95)),
      (Vec3::new(-0.55, 0.0, 0.95), Vec3::new(0.55, HULL.y, 1.05)),
      (Vec3::new(-0.55, 0.1, 0.1), Vec3::new(0.55, 0.22, 0.4))
    ],
    [1.0; 3]
  )
}

#[derive(Resource)]
struct Kit(Handle<Mesh>);

fn build(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
  commands.insert_resource(Kit(meshes.add(hull())))
}

fn moor(
  time: Res<Time>,
  kit: Res<Kit>,
  palette: Res<Palette>,
  pilot: Option<Res<Pilot>>,
  mut commands: Commands,
  mut boats: Query<(Entity, &Vessel, Option<&mut Transform>)>
) {
  let blend = (time.delta_secs() * 12.0).min(1.0);
  boats.iter_mut().for_each(|(entity, vessel, transform)| {
    let ridden = pilot.as_ref().and_then(|pilot| {
      pilot
        .riding
        .filter(|riding| riding.boat == entity)
        .map(|riding| (pilot.at, riding.yaw))
    });
    let (at, yaw) = ridden.unwrap_or((vessel.at, vessel.yaw));
    let wanted =
      Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw));
    match transform {
      Some(mut transform) if ridden.is_some() => *transform = wanted,
      Some(mut transform) => {
        transform.translation = transform.translation.lerp(wanted.translation, blend);
        transform.rotation = transform.rotation.slerp(wanted.rotation, blend)
      }
      None => {
        commands.entity(entity).insert((
          Mesh3d(kit.0.clone()),
          MeshMaterial3d(palette.solid.clone()),
          wanted,
          Visibility::default()
        ));
      }
    }
  })
}

fn embark(pilot: Option<ResMut<Pilot>>, boats: Query<(Entity, &Vessel)>) {
  if let Some(mut pilot) = pilot {
    let me = pilot.me;
    let aboard = boats.iter().find(|(_, vessel)| vessel.rider == Some(me));
    match (pilot.riding, aboard) {
      (None, Some((boat, vessel))) => {
        pilot.riding = Some(Riding { boat, yaw: vessel.yaw, speed: 0.0 });
        pilot.at = vessel.at;
        pilot.velocity = Vec3::ZERO;
        pilot.grounded = false
      }
      (Some(riding), None) if boats.get(riding.boat).is_err() => pilot.riding = None,
      _ => ()
    }
  }
}

fn float(voxels: &Voxels, at: Vec3) -> Option<f32> {
  let cell = (at - Vec3::Y * 0.4).floor().as_ivec3();
  [cell + IVec3::Y, cell]
    .into_iter()
    .find(|&cell| {
      voxels.block(cell).is_some_and(|block| {
        block.liquid().is_some_and(|(fluid, _)| fluid == Fluid::Water)
      })
    })
    .map(|cell| cell.y as f32 + 0.88 - DRAFT)
}

fn row(
  time: Res<Time>,
  keys: Res<ButtonInput<KeyCode>>,
  menu: Res<Menu>,
  voxels: Option<Res<Voxels>>,
  mut pilot: ResMut<Pilot>,
  mut leaves: MessageWriter<Disembark>,
  mut steers: MessageWriter<Steer>,
  mut since: Local<f32>,
  mut spare: Local<f32>
) {
  if let Some(voxels) = voxels
    && let Some(mut riding) = pilot.riding
    && ready_around(&voxels, pilot.at)
  {
    let pressed = |key: KeyCode| !menu.open && keys.pressed(key);
    let held = |key: KeyCode| f32::from(u8::from(pressed(key)));
    match !menu.open && keys.just_pressed(KeyCode::ShiftLeft) {
      true => {
        leaves.write(Disembark);
        pilot.riding = None;
        pilot.at += Vec3::Y * 0.6;
        pilot.velocity = Vec3::Y * 4.0
      }
      false => {
        let banked = (*spare + time.delta_secs()).min(0.25);
        let steps = (banked / STEP).floor();
        *spare = banked - steps * STEP;
        (0..steps as u32).for_each(|_| {
          let surface = float(&voxels, pilot.at);
          let afloat = surface.is_some();
          riding.yaw += (held(KeyCode::KeyA) - held(KeyCode::KeyD)) * TURN * STEP;
          let push = held(KeyCode::KeyW) - held(KeyCode::KeyS) * 0.5;
          let drag = if afloat { DRAG } else { 8.0 };
          riding.speed +=
            (push * THRUST * f32::from(u8::from(afloat)) - riding.speed * drag) * STEP;
          let vertical = match surface {
            Some(level) => (level - pilot.at.y) * 6.0,
            None => (pilot.velocity.y - 30.0 * STEP).max(-40.0)
          };
          let travel = ahead(riding.yaw) * riding.speed;
          pilot.velocity = Vec3::new(travel.x, vertical, travel.z);
          let velocity = pilot.velocity;
          let at = [1, 0, 2].into_iter().fold(pilot.at, |at, axis| {
            let (moved, hit) = slide(&voxels, BULK, at, axis, velocity[axis] * STEP);
            if hit && axis != 1 {
              riding.speed *= 0.3
            }
            moved
          });
          pilot.at = at
        });
        pilot.riding = Some(riding);
        *since += time.delta_secs();
        if *since >= STEER_EVERY {
          *since = 0.0;
          steers.write(Steer { at: pilot.at, yaw: riding.yaw });
        }
      }
    }
  }
}

fn handle(
  buttons: Res<ButtonInput<MouseButton>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  pilot: Res<Pilot>,
  selected: Res<Selected>,
  inventories: Query<&Inventory>,
  voxels: Option<Res<Voxels>>,
  boats: Query<(Entity, &Vessel)>,
  mut boards: MessageWriter<Board>,
  mut wrecks: MessageWriter<Wreck>,
  mut launches: MessageWriter<Launch>
) {
  if let Some(voxels) = voxels
    && captured(&cursor, &menu)
    && pilot.riding.is_none()
  {
    let (from, toward) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
    let blocked = voxels
      .cast(from, toward, REACH)
      .map_or(REACH, |hit| (hit.at.as_vec3() + 0.5).distance(from));
    let target = boats
      .iter()
      .filter_map(|(entity, vessel)| {
        struck(vessel, from, toward).map(|distance| (entity, vessel, distance))
      })
      .filter(|&(_, vessel, distance)| {
        distance <= REACH.min(blocked) && vessel.rider.is_none()
      })
      .min_by(|a, b| a.2.total_cmp(&b.2));
    let holding = inventories
      .get(pilot.me)
      .ok()
      .and_then(|inventory| inventory.slots[selected.0])
      .is_some_and(|stack| stack.block == Block::Boat);
    match (
      target,
      buttons.just_pressed(MouseButton::Right),
      buttons.just_pressed(MouseButton::Left)
    ) {
      (Some((boat, ..)), true, _) => {
        boards.write(Board(boat));
      }
      (Some((boat, ..)), _, true) => {
        wrecks.write(Wreck(boat));
      }
      (None, true, _) if holding => {
        let water = (1..=(REACH * 10.0) as i32)
          .map(|step| (from + toward * step as f32 * 0.1).floor().as_ivec3())
          .take_while(|&cell| !voxels.solid(cell))
          .find(|&cell| {
            voxels.block(cell).is_some_and(|block| {
              block.liquid().is_some_and(|(fluid, _)| fluid == Fluid::Water)
            })
          });
        if let Some(cell) = water {
          launches.write(Launch(cell));
        }
      }
      _ => ()
    }
  }
}

fn serve(
  mut launches: MessageReader<FromClient<Launch>>,
  mut boardings: MessageReader<FromClient<Board>>,
  mut wrecks: MessageReader<FromClient<Wreck>>,
  mut leaves: MessageReader<FromClient<Disembark>>,
  mut steers: MessageReader<FromClient<Steer>>,
  mut voxels: ResMut<Voxels>,
  mut commands: Commands,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Inventory))>,
  mut boats: Query<(Entity, &mut Vessel)>
) {
  launches.read().for_each(|&FromClient { client_id, message: Launch(cell) }| {
    if voxels.ensure(cell).liquid().is_some_and(|(fluid, _)| fluid == Fluid::Water)
      && let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, cell)
      && inventory.take(Block::Boat)
    {
      commands.spawn((Replicated, Vessel {
        at: cell.as_vec3() + Vec3::new(0.5, 0.88 - DRAFT, 0.5),
        yaw: avatar.yaw,
        rider: None
      }));
    }
  });
  boardings.read().for_each(|&FromClient { client_id, message: Board(boat) }| {
    if let Some((player, avatar, _)) = player_of(players.iter_mut(), client_id)
      && boats.iter().all(|(_, vessel)| vessel.rider != Some(player))
      && let Ok((_, mut vessel)) = boats.get_mut(boat)
      && vessel.rider.is_none()
      && within_reach(avatar, vessel.at.floor().as_ivec3())
    {
      vessel.rider = Some(player)
    }
  });
  wrecks.read().for_each(|&FromClient { client_id, message: Wreck(boat) }| {
    if let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Ok((_, vessel)) = boats.get(boat)
      && vessel.rider.is_none()
      && within_reach(avatar, vessel.at.floor().as_ivec3())
      && inventory.add(Block::Boat)
    {
      commands.entity(boat).despawn()
    }
  });
  leaves.read().for_each(|&FromClient { client_id, .. }| {
    if let Some((player, ..)) = player_of(players.iter_mut(), client_id) {
      boats
        .iter_mut()
        .filter(|(_, vessel)| vessel.rider == Some(player))
        .for_each(|(_, mut vessel)| vessel.rider = None)
    }
  });
  steers.read().for_each(|&FromClient { client_id, message: Steer { at, yaw } }| {
    if let Some((player, ..)) = player_of(players.iter_mut(), client_id)
      && at.is_finite()
      && yaw.is_finite()
    {
      boats.iter_mut().filter(|(_, vessel)| vessel.rider == Some(player)).for_each(
        |(_, mut vessel)| {
          let moved = Vessel { at, yaw, ..*vessel };
          vessel.set_if_neq(moved);
        }
      )
    }
  });
}

fn unseat(players: Query<(), With<Controller>>, mut boats: Query<&mut Vessel>) {
  boats.iter_mut().for_each(|mut vessel| {
    if vessel.rider.is_some_and(|rider| players.get(rider).is_err()) {
      vessel.rider = None
    }
  })
}

pub struct Boats;

impl Plugin for Boats {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, build.run_if(plays))
      .add_systems(
        PreUpdate,
        (serve, unseat)
          .chain()
          .after(ServerSystems::Receive)
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
      )
      .add_systems(
        Update,
        (embark, row, handle, moor)
          .chain()
          .run_if(plays)
          .run_if(resource_exists::<Pilot>)
      );
  }
}
