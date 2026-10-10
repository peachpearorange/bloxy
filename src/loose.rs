use {crate::{authority::{Controller, player_of},
             block::{Block, Look},
             menu::closed,
             player::{Bulk, Marched, Pilot, Selected, march},
             protocol::*,
             stream::Palette,
             texture::{PIXELS, paint, uv_corner},
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            mesh::{Indices, PrimitiveTopology},
            platform::collections::HashMap,
            prelude::*},
     bevy_replicon::prelude::*};

const GRAVITY: f32 = 22.0;
const THROW: f32 = 5.0;
const GATHER: f32 = 1.6;
const TOSSED_WAIT: f32 = 1.5;
const DROPPED_WAIT: f32 = 0.4;
const LASTS: f32 = 300.0;
pub const SIZE: f32 = 0.25;
const BULK: Bulk = Bulk { half: SIZE / 2.0, tall: SIZE };

#[derive(Component)]
pub struct Tumble {
  velocity: Vec3,
  wait: f32,
  age: f32
}

pub fn scatter(
  commands: &mut Commands,
  block: Block,
  count: u16,
  at: Vec3,
  velocity: Vec3,
  wait: f32
) {
  commands.spawn((Replicated, Loose { block, count, at }, Tumble {
    velocity,
    wait,
    age: 0.0
  }));
}

pub fn fall(commands: &mut Commands, block: Block, at: IVec3) {
  let spin = (at.x * 73 + at.y * 31 + at.z * 17) as f32;
  scatter(
    commands,
    block,
    1,
    at.as_vec3() + Vec3::new(0.5, 0.3, 0.5),
    Vec3::new(spin.sin(), 2.5, spin.cos()) * 1.2,
    DROPPED_WAIT
  )
}

fn toss(
  mut tosses: MessageReader<FromClient<Toss>>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  mut commands: Commands
) {
  tosses.read().for_each(|&FromClient { client_id, message: Toss { slot, all } }| {
    if let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Some(stack) = inventory.slots.get(usize::from(slot)).copied().flatten()
    {
      let count = if all { stack.count } else { 1 };
      inventory.slots[usize::from(slot)] =
        (stack.count > count).then_some(Stack { count: stack.count - count, ..stack });
      let ahead =
        Quat::from_euler(EulerRot::YXZ, avatar.yaw, avatar.pitch, 0.0) * Vec3::NEG_Z;
      let from = avatar.at + Vec3::Y * (EYE - 0.35) + ahead.with_y(0.0) * 0.3;
      scatter(
        &mut commands,
        stack.block,
        count,
        from,
        ahead * THROW + Vec3::Y * 1.5,
        TOSSED_WAIT
      )
    }
  })
}

fn tumble(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut loose: Query<(Entity, &mut Loose, &mut Tumble)>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  loose.iter_mut().for_each(|(entity, mut loose, mut tumble)| {
    tumble.age += dt;
    tumble.wait -= dt;
    let at = loose.at;
    [IVec3::ZERO, IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z]
      .into_iter()
      .for_each(|offset| {
        voxels.ensure(at.floor().as_ivec3() + offset);
      });
    let floating = voxels
      .block((at + Vec3::Y * SIZE * 0.5).floor().as_ivec3())
      .is_some_and(Block::fluid);
    let velocity = match floating {
      true => {
        let slowed = tumble.velocity * (1.0 - dt * 3.0);
        slowed.with_y((tumble.velocity.y + 10.0 * dt).min(1.2))
      }
      false => tumble.velocity.with_y((tumble.velocity.y - GRAVITY * dt).max(-30.0))
    };
    let Marched { at: moved, velocity, landed, .. } =
      march(&voxels, BULK, at, velocity, dt);
    tumble.velocity = match landed {
      true => {
        velocity * Vec3::new(1.0 - (dt * 10.0).min(1.0), 1.0, 1.0 - (dt * 10.0).min(1.0))
      }
      false => velocity
    };
    match tumble.age > LASTS || moved.y < -16.0 {
      true => commands.entity(entity).despawn(),
      false => {
        if moved.distance(at) > 0.001 {
          loose.at = moved
        }
      }
    }
  })
}

fn gather(
  mut players: Query<(&Avatar, &mut Inventory), With<Controller>>,
  mut loose: Query<(Entity, &mut Loose, &Tumble)>,
  mut commands: Commands
) {
  loose.iter_mut().filter(|(_, _, tumble)| tumble.wait <= 0.0).for_each(
    |(entity, mut loose, _)| {
      if let Some((_, mut inventory)) = players
        .iter_mut()
        .find(|(avatar, _)| (avatar.at + Vec3::Y * 0.9).distance(loose.at) < GATHER)
      {
        let kept =
          (0..loose.count).filter(|_| !inventory.add(loose.block)).count() as u16;
        match kept {
          0 => commands.entity(entity).despawn(),
          kept if kept != loose.count => loose.count = kept,
          _ => ()
        }
      }
    }
  )
}

fn throw(
  keys: Res<ButtonInput<KeyCode>>,
  selected: Res<Selected>,
  mut tosses: MessageWriter<Toss>
) {
  if keys.just_pressed(KeyCode::KeyQ) {
    tosses.write(Toss {
      slot: selected.0 as u8,
      all: keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
    });
  }
}

pub const FLAT: f32 = 0.4;

#[derive(Resource, Default)]
pub struct Looks(HashMap<Block, Handle<Mesh>>);

impl Looks {
  pub fn mesh(&mut self, block: Block, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    self
      .0
      .entry(block)
      .or_insert_with(|| {
        meshes.add(if cubic(block) { cube(block) } else { sliver(block) })
      })
      .clone()
  }
}

#[derive(Component)]
struct Shown {
  at: Vec3,
  turned: Quat
}

fn quads(faces: Vec<([Vec3; 4], Vec3, [Vec2; 4])>) -> Mesh {
  let count = faces.len() as u32;
  let (positions, normals, uvs) = faces.into_iter().fold(
    (vec![], vec![], vec![]),
    |(mut p, mut n, mut u): (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>),
     (corners, normal, uv)| {
      p.extend(corners.map(|corner| corner.to_array()));
      n.extend([normal.to_array(); 4]);
      u.extend(uv.map(|uv| uv.to_array()));
      (p, n, u)
    }
  );
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(
      (0..count)
        .flat_map(|face| [0, 1, 2, 0, 2, 3].map(|corner| face * 4 + corner))
        .collect()
    ))
}

fn cube(block: Block) -> Mesh {
  let [top, side, bottom] = block.tiles();
  let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
  quads(
    [
      (Vec3::Y, Vec3::X, Vec3::NEG_Z, top),
      (Vec3::NEG_Y, Vec3::X, Vec3::Z, bottom),
      (Vec3::X, Vec3::NEG_Z, Vec3::Y, side),
      (Vec3::NEG_X, Vec3::Z, Vec3::Y, side),
      (Vec3::Z, Vec3::X, Vec3::Y, side),
      (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y, side)
    ]
    .into_iter()
    .map(|(normal, across, up, tile)| {
      (
        corners.map(|(a, b)| {
          (normal + across * (a * 2.0 - 1.0) + up * (b * 2.0 - 1.0)) * SIZE / 2.0
            + Vec3::Y * SIZE / 2.0
        }),
        normal,
        corners.map(|(a, b)| uv_corner(tile, Vec2::new(a, 1.0 - b)))
      )
    })
    .collect()
  )
}

fn sliver(block: Block) -> Mesh {
  let tile = block.tiles()[1];
  let edge = PIXELS as i32;
  let solid = |x: i32, y: i32| {
    (0..edge).contains(&x)
      && (0..edge).contains(&y)
      && paint(tile, x as u32, y as u32).color[3] > 0.5
  };
  let texel = FLAT / PIXELS as f32;
  let point = |x: f32, y: f32, z: f32| {
    Vec3::new(x * texel - FLAT / 2.0, FLAT / 2.0 - y * texel, z * texel)
  };
  let uv = |x: f32, y: f32| uv_corner(tile, Vec2::new(x, y) / PIXELS as f32);
  let whole = [(0.0, 0.0), (16.0, 0.0), (16.0, 16.0), (0.0, 16.0)];
  let faces = [(0.5, Vec3::Z), (-0.5, Vec3::NEG_Z)].into_iter().map(|(z, normal)| {
    let mut corners = whole;
    if z > 0.0 {
      corners.reverse()
    }
    (corners.map(|(x, y)| point(x, y, z)), normal, corners.map(|(x, y)| uv(x, y)))
  });
  let rims = (0..edge * edge)
    .filter(|&index| solid(index % edge, index / edge))
    .flat_map(|index| {
      let (x, y) = (index % edge, index / edge);
      let centre = uv(x as f32 + 0.5, y as f32 + 0.5);
      let (left, right, top, bottom) =
        (x as f32, x as f32 + 1.0, y as f32, y as f32 + 1.0);
      [
        ((-1, 0), Vec3::NEG_X, [
          (left, top, -0.5),
          (left, bottom, -0.5),
          (left, bottom, 0.5),
          (left, top, 0.5)
        ]),
        ((1, 0), Vec3::X, [
          (right, top, 0.5),
          (right, bottom, 0.5),
          (right, bottom, -0.5),
          (right, top, -0.5)
        ]),
        ((0, -1), Vec3::Y, [
          (left, top, -0.5),
          (left, top, 0.5),
          (right, top, 0.5),
          (right, top, -0.5)
        ]),
        ((0, 1), Vec3::NEG_Y, [
          (left, bottom, 0.5),
          (left, bottom, -0.5),
          (right, bottom, -0.5),
          (right, bottom, 0.5)
        ])
      ]
      .into_iter()
      .filter(move |&((dx, dy), ..)| !solid(x + dx, y + dy))
      .map(move |(_, normal, corners)| {
        (corners.map(|(x, y, z)| point(x, y, z)), normal, [centre; 4])
      })
    });
  quads(faces.chain(rims).collect())
}

pub fn cubic(block: Block) -> bool {
  matches!(block.look(), Look::Opaque | Look::Cutout | Look::Log)
}

fn show(
  arrivals: Query<(Entity, &Loose), Without<Shown>>,
  palette: Res<Palette>,
  mut looks: ResMut<Looks>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut commands: Commands
) {
  arrivals.iter().for_each(|(entity, loose)| {
    let block = loose.block;
    let yaw = Quat::from_rotation_y((entity.to_bits() % 997) as f32 * 2.399);
    let turned = match cubic(block) {
      true => yaw,
      false => yaw * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    };
    commands.entity(entity).insert((
      Shown { at: loose.at, turned },
      Mesh3d(looks.mesh(block, &mut meshes)),
      MeshMaterial3d(palette.solid.clone()),
      Transform::from_translation(loose.at).with_rotation(turned),
      Visibility::default()
    ));
  })
}

fn settle(time: Res<Time>, mut shown: Query<(&Loose, &mut Shown, &mut Transform)>) {
  let dt = time.delta_secs();
  shown.iter_mut().for_each(|(loose, mut shown, mut transform)| {
    shown.at = shown.at.lerp(loose.at, (dt * 15.0).min(1.0));
    let lift = match cubic(loose.block) {
      true => 0.0,
      false => FLAT / PIXELS as f32 / 2.0 + 0.004
    };
    *transform =
      Transform::from_translation(shown.at + Vec3::Y * lift).with_rotation(shown.turned)
  })
}

pub struct Litter;

impl Plugin for Litter {
  fn build(&self, app: &mut App) {
    app
      .add_systems(PreUpdate, toss.after(ServerSystems::Receive).run_if(authority))
      .add_systems(
        Update,
        (tumble, gather).chain().run_if(authority).run_if(resource_exists::<Voxels>)
      )
      .init_resource::<Looks>()
      .add_systems(
        Update,
        (throw.run_if(resource_exists::<Pilot>.and_then(closed)), show, settle)
          .chain()
          .run_if(plays)
      );
  }
}
