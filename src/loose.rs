use {crate::{authority::{Controller, player_of},
             block::{Block, Look},
             hud::icon_rect,
             menu::closed,
             player::{Bulk, Marched, Pilot, Selected, march},
             protocol::*,
             stream::Palette,
             texture::{ICON, ICON_COLUMNS, uv_corner},
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
const LASTS: f32 = 300.0;
const SIZE: f32 = 0.25;
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

#[derive(Resource)]
struct Looks {
  meshes: HashMap<Block, Handle<Mesh>>,
  flat: Handle<StandardMaterial>
}

#[derive(Component)]
struct Shown {
  at: Vec3,
  phase: f32
}

fn cube(block: Block) -> Mesh {
  let [top, side, bottom] = block.tiles();
  let faces = [
    (Vec3::Y, Vec3::X, Vec3::NEG_Z, top),
    (Vec3::NEG_Y, Vec3::X, Vec3::Z, bottom),
    (Vec3::X, Vec3::NEG_Z, Vec3::Y, side),
    (Vec3::NEG_X, Vec3::Z, Vec3::Y, side),
    (Vec3::Z, Vec3::X, Vec3::Y, side),
    (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y, side)
  ];
  let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
  let (positions, normals, uvs): (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>) = faces
    .iter()
    .flat_map(|&(normal, across, up, tile)| {
      corners.map(|(a, b)| {
        let point =
          (normal + across * (a * 2.0 - 1.0) + up * (b * 2.0 - 1.0)) * SIZE / 2.0;
        (
          (point + Vec3::Y * SIZE / 2.0).to_array(),
          normal.to_array(),
          uv_corner(tile, Vec2::new(a, 1.0 - b)).to_array()
        )
      })
    })
    .fold((vec![], vec![], vec![]), |(mut p, mut n, mut u), (point, normal, uv)| {
      p.push(point);
      n.push(normal);
      u.push(uv);
      (p, n, u)
    });
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(
      (0..6u32)
        .flat_map(|face| [0, 1, 2, 0, 2, 3].map(|corner| face * 4 + corner))
        .collect()
    ))
}

fn card(block: Block) -> Mesh {
  let rect = icon_rect(block);
  let atlas = Vec2::new(
    (ICON_COLUMNS * ICON) as f32,
    ((Block::ALL.len() as u32 / ICON_COLUMNS + 1) * ICON) as f32
  );
  let (low, high) = (rect.min / atlas, rect.max / atlas);
  let half = SIZE * 0.8;
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![
      [-half, 0.0, 0.0],
      [half, 0.0, 0.0],
      [half, half * 2.0, 0.0],
      [-half, half * 2.0, 0.0],
    ])
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![
      [low.x, high.y],
      [high.x, high.y],
      [high.x, low.y],
      [low.x, low.y],
    ])
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}

fn cubic(block: Block) -> bool {
  matches!(block.look(), Look::Opaque | Look::Cutout | Look::Log)
}

fn prepare(
  palette: Res<Palette>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  commands.insert_resource(Looks {
    meshes: default(),
    flat: materials.add(StandardMaterial {
      base_color_texture: Some(palette.icons.clone()),
      alpha_mode: AlphaMode::Mask(0.5),
      cull_mode: None,
      double_sided: true,
      perceptual_roughness: 0.9,
      ..default()
    })
  })
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
    let mesh = looks
      .meshes
      .entry(block)
      .or_insert_with(|| meshes.add(if cubic(block) { cube(block) } else { card(block) }))
      .clone();
    let material = match cubic(block) {
      true => palette.solid.clone(),
      false => looks.flat.clone()
    };
    commands.entity(entity).insert((
      Shown { at: loose.at, phase: (entity.to_bits() % 64) as f32 * 1.7 },
      Mesh3d(mesh),
      MeshMaterial3d(material),
      Transform::from_translation(loose.at),
      Visibility::default()
    ));
  })
}

fn spin(time: Res<Time>, mut shown: Query<(&Loose, &mut Shown, &mut Transform)>) {
  let (dt, now) = (time.delta_secs(), time.elapsed_secs());
  shown.iter_mut().for_each(|(loose, mut shown, mut transform)| {
    shown.at = shown.at.lerp(loose.at, (dt * 15.0).min(1.0));
    let bob = ((now * 2.5 + shown.phase).sin() + 1.0) * 0.06;
    *transform = Transform::from_translation(shown.at + Vec3::Y * bob)
      .with_rotation(Quat::from_rotation_y(now * 1.6 + shown.phase))
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
      .add_systems(Startup, prepare.after(crate::stream::paint).run_if(plays))
      .add_systems(
        Update,
        (throw.run_if(closed).run_if(resource_exists::<Pilot>), show, spin)
          .chain()
          .run_if(plays)
      );
  }
}
