use {crate::{block::Block,
             generate,
             island::{Island, Kind, SEA},
             noise::{hash, unit},
             player::{Bulk, Marched, cells, march},
             protocol::{Avatar, Hopper, authority, plays},
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            image::ImageSampler,
            mesh::{Indices, PrimitiveTopology},
            platform::collections::HashSet,
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}},
     bevy_replicon::prelude::*,
     std::f32::consts::{PI, TAU}};

const BULK: Bulk = Bulk { half: 0.25, tall: 0.85 };
const WAKE: f32 = 110.0;
const SLEEP: f32 = 170.0;
const WALK: f32 = 1.6;
const HOP: f32 = 6.0;
const LEAP: f32 = 8.0;
const GRAVITY: f32 = 30.0;

#[derive(Component)]
pub struct Wander {
  pub home: IVec2,
  velocity: Vec3,
  grounded: bool,
  resting: f32,
  turning: f32,
  hopping: f32,
  luck: u32
}

impl Wander {
  fn dice(&mut self) -> f32 {
    self.luck = hash(self.luck, 0x5A, 0x11, 0x0D);
    unit(self.luck, 0, 0, 0)
  }
}

#[derive(Resource, Default)]
pub struct Colonies(pub HashSet<IVec2>);

pub fn lodge(commands: &mut Commands, home: IVec2, hopper: Hopper, luck: u32) {
  commands.spawn((Replicated, hopper, Wander {
    home,
    velocity: Vec3::ZERO,
    grounded: false,
    resting: 0.0,
    turning: 0.0,
    hopping: (luck % 5) as f32,
    luck
  }));
}

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn muster(
  voxels: Res<Voxels>,
  mut colonies: ResMut<Colonies>,
  players: Query<&Avatar>,
  mut commands: Commands
) {
  let seed = voxels.seed;
  players
    .iter()
    .flat_map(|avatar| {
      Island::near(seed, avatar.at).into_iter().filter(|island| {
        island.kind == Kind::Mushroom && avatar.at.xz().distance(island.centre) < WAKE
      })
    })
    .collect::<Vec<_>>()
    .into_iter()
    .for_each(|island| {
      if colonies.0.insert(island.cell) {
        let key =
          hash(seed ^ 0x5B0, island.cell.x, 0x77, 0) ^ hash(seed, 0, island.cell.y, 7);
        let count = 3 + (island.radius / 8.0) as u32;
        (0..count * 4)
          .filter_map(|attempt| {
            let roll = |salt: u32| unit(key ^ salt, attempt as i32, 0, 0) * 2.0 - 1.0;
            let spot = (island.centre
              + Vec2::new(roll(1), roll(2)) * island.radius * 0.6)
              .floor()
              .as_ivec2();
            let ground = generate::height(seed, spot.x, spot.y);
            (ground > SEA + 1).then(|| {
              (
                Vec3::new(spot.x as f32 + 0.5, ground as f32 + 1.05, spot.y as f32 + 0.5),
                roll(3) * PI
              )
            })
          })
          .take(count as usize)
          .enumerate()
          .for_each(|(index, (at, yaw))| {
            lodge(
              &mut commands,
              island.cell,
              Hopper { at, yaw, aloft: false },
              key ^ index as u32
            )
          })
      }
    })
}

fn wander(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  players: Query<&Avatar>,
  mut shroomlings: Query<(&mut Hopper, &mut Wander)>
) {
  let dt = time.delta_secs().min(0.1);
  let seed = voxels.seed;
  let watched =
    |at: Vec3| players.iter().any(|avatar| avatar.at.xz().distance(at.xz()) < SLEEP);
  shroomlings.iter_mut().filter(|(hopper, _)| watched(hopper.at)).for_each(
    |(mut hopper, mut wander)| {
      let Hopper { mut at, mut yaw, .. } = *hopper;
      [
        IVec3::ZERO,
        IVec3::X * 2,
        IVec3::X * -2,
        IVec3::Z * 2,
        IVec3::Z * -2,
        IVec3::Y * -2
      ]
      .into_iter()
      .for_each(|offset| {
        voxels.ensure(at.floor().as_ivec3() + offset);
      });
      let (low, high) = BULK.body(at);
      if cells(low, high).any(|cell| voxels.solid(cell)) {
        at.y = at.y.floor() + 1.0
      }
      wander.turning -= dt;
      wander.resting -= dt;
      wander.hopping -= dt;
      if wander.turning <= 0.0 {
        wander.turning = 2.0 + wander.dice() * 4.0;
        let roll = wander.dice();
        match roll {
          roll if roll < 0.3 => wander.resting = 1.0 + wander.dice() * 2.5,
          _ => yaw += (wander.dice() - 0.5) * PI * 1.4
        }
      }
      if let Some(island) = Island::at(seed, wander.home)
        && at.xz().distance(island.centre) > island.radius * 0.8
      {
        let home = island.centre - at.xz();
        yaw = (-home.x).atan2(-home.y)
      }
      let toward = ahead(yaw);
      let probe = (at + toward * 0.7).floor().as_ivec3();
      let drop = (0..4).all(|depth| !voxels.solid(probe - IVec3::Y * (depth + 1)));
      let wet = [probe, probe - IVec3::Y]
        .into_iter()
        .any(|cell| voxels.block(cell).is_some_and(Block::fluid));
      if (drop || wet) && wander.grounded {
        yaw += PI;
        wander.turning = 1.0
      }
      let pace = if wander.resting > 0.0 { 0.0 } else { WALK };
      let walk = ahead(yaw) * pace;
      let jump = match (wander.grounded, wander.hopping <= 0.0) {
        (true, true) => {
          wander.hopping = 1.0 + wander.dice() * 5.0;
          Some(HOP)
        }
        _ => None
      };
      let mut velocity = Vec3::new(walk.x, wander.velocity.y, walk.z);
      velocity.y = jump.unwrap_or((velocity.y - GRAVITY * dt).max(-40.0));
      let Marched { at: moved, velocity, blocked, landed } =
        march(&voxels, BULK, at, velocity, dt);
      wander.velocity = match blocked && landed {
        true => velocity.with_y(LEAP),
        false => velocity
      };
      wander.grounded = landed;
      hopper.set_if_neq(Hopper { at: moved, yaw: yaw.rem_euclid(TAU), aloft: !landed });
    }
  )
}

#[derive(Component)]
struct Shown {
  at: Vec3,
  yaw: f32,
  stride: f32,
  squash: f32
}

#[derive(Component)]
struct Foot(f32);

#[derive(Component)]
struct Trunk;

#[derive(Resource)]
struct Mold {
  body: Handle<Mesh>,
  cap: Handle<Mesh>,
  foot: Handle<Mesh>,
  material: Handle<StandardMaterial>
}

const PATCHES: u32 = 5;
const CAP_SIDE: u32 = 0;
const CAP_TOP: u32 = 1;
const GILLS: u32 = 2;
const FLESH: u32 = 3;
const FACE: u32 = 4;

fn texel(patch: u32, x: u32, y: u32) -> [u8; 4] {
  let shade = |[r, g, b]: [f32; 3], by: f32| {
    [r, g, b].map(|channel| (channel * by * 255.0).min(255.0) as u8)
  };
  let spotted = |spots: &[(u32, u32)]| {
    spots.iter().any(|&(sx, sy)| x.abs_diff(sx) + y.abs_diff(sy) <= 1)
  };
  let [r, g, b] = match patch {
    CAP_SIDE if spotted(&[(1, 3), (5, 1), (6, 5)]) => shade([0.95, 0.93, 0.9], 1.0),
    CAP_TOP if spotted(&[(2, 2), (6, 3), (3, 6)]) => shade([0.95, 0.93, 0.9], 1.0),
    CAP_SIDE | CAP_TOP => {
      shade([0.82, 0.1, 0.09], if (x + y) % 5 == 0 { 0.88 } else { 1.0 })
    }
    GILLS => shade([0.86, 0.79, 0.66], if x % 2 == 0 { 0.85 } else { 1.0 }),
    FACE if (x == 2 || x == 5) && (2..=3).contains(&y) => [18, 16, 20],
    _ => shade([0.95, 0.93, 0.88], if y == 7 { 0.85 } else { 1.0 })
  };
  [r, g, b, 255]
}

fn sprite() -> Image {
  let mut image = Image::new(
    Extent3d { width: PATCHES * 8, height: 8, depth_or_array_layers: 1 },
    TextureDimension::D2,
    (0..PATCHES * 64)
      .flat_map(|index| {
        let (x, y) = (index % (PATCHES * 8), index / (PATCHES * 8));
        texel(x / 8, x % 8, y)
      })
      .collect(),
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  );
  image.sampler = ImageSampler::nearest();
  image
}

const FACES: [(Vec3, Vec3, Vec3); 6] = [
  (Vec3::X, Vec3::Z, Vec3::NEG_Y),
  (Vec3::NEG_X, Vec3::NEG_Z, Vec3::NEG_Y),
  (Vec3::Y, Vec3::X, Vec3::NEG_Z),
  (Vec3::NEG_Y, Vec3::X, Vec3::Z),
  (Vec3::Z, Vec3::NEG_X, Vec3::NEG_Y),
  (Vec3::NEG_Z, Vec3::X, Vec3::NEG_Y)
];

fn boxes(parts: &[(Vec3, Vec3, [u32; 6])]) -> Mesh {
  let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
  let quads: Vec<(Vec3, Vec3, Vec2)> = parts
    .iter()
    .flat_map(|&(low, high, patches)| {
      let (centre, half) = ((low + high) / 2.0, (high - low) / 2.0);
      FACES.iter().zip(patches).flat_map(move |(&(normal, across, down), patch)| {
        corners.map(|(a, b): (f32, f32)| {
          let at = centre
            + normal * half
            + across * half * (a * 2.0 - 1.0)
            + down * half * (b * 2.0 - 1.0);
          (at, normal, Vec2::new((patch as f32 + a) / PATCHES as f32, b))
        })
      })
    })
    .collect();
  let indices = (0..quads.len() as u32 / 4)
    .flat_map(|quad| [0, 1, 2, 0, 2, 3].map(|corner| quad * 4 + corner))
    .collect();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      quads.iter().map(|(at, ..)| at.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_NORMAL,
      quads.iter().map(|(_, normal, _)| normal.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_0,
      quads.iter().map(|(.., uv)| uv.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_indices(Indices::U32(indices))
}

fn mould(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let flesh = [FLESH; 6];
  let cap = [CAP_SIDE, CAP_SIDE, CAP_TOP, GILLS, CAP_SIDE, CAP_SIDE];
  commands.insert_resource(Mold {
    body: meshes.add(boxes(&[(
      Vec3::new(-0.22, 0.0, -0.2),
      Vec3::new(0.22, 0.36, 0.2),
      [FLESH, FLESH, FLESH, FLESH, FLESH, FACE]
    )])),
    cap: meshes.add(boxes(&[
      (Vec3::new(-0.4, 0.0, -0.4), Vec3::new(0.4, 0.28, 0.4), cap),
      (Vec3::new(-0.27, 0.28, -0.27), Vec3::new(0.27, 0.4, 0.27), cap)
    ])),
    foot: meshes.add(boxes(&[(
      Vec3::new(-0.08, -0.1, -0.12),
      Vec3::new(0.08, 0.0, 0.1),
      flesh
    )])),
    material: materials.add(StandardMaterial {
      base_color_texture: Some(images.add(sprite())),
      perceptual_roughness: 0.8,
      ..default()
    })
  })
}

fn dress(
  mold: Res<Mold>,
  arrivals: Query<(Entity, &Hopper), Without<Shown>>,
  mut commands: Commands
) {
  arrivals.iter().for_each(|(entity, hopper)| {
    let material = MeshMaterial3d(mold.material.clone());
    commands
      .entity(entity)
      .insert((
        Shown { at: hopper.at, yaw: hopper.yaw, stride: 0.0, squash: 0.0 },
        Transform::from_translation(hopper.at),
        Visibility::default()
      ))
      .with_children(|figure| {
        figure
          .spawn((Trunk, Transform::from_xyz(0.0, 0.1, 0.0), Visibility::default()))
          .with_children(|trunk| {
            trunk.spawn((
              Mesh3d(mold.body.clone()),
              material.clone(),
              Transform::default()
            ));
            trunk.spawn((
              Mesh3d(mold.cap.clone()),
              material.clone(),
              Transform::from_xyz(0.0, 0.33, 0.0)
            ));
          });
        [(0.12, 0.0), (-0.12, PI)].into_iter().for_each(|(x, phase)| {
          figure.spawn((
            Foot(phase),
            Mesh3d(mold.foot.clone()),
            material.clone(),
            Transform::from_xyz(x, 0.1, 0.0)
          ));
        })
      });
  })
}

fn animate(
  time: Res<Time>,
  mut shroomlings: Query<(&Hopper, &mut Shown, &mut Transform, &Children)>,
  mut trunks: Query<&mut Transform, (With<Trunk>, Without<Shown>, Without<Foot>)>,
  mut feet: Query<(&Foot, &mut Transform), (Without<Shown>, Without<Trunk>)>
) {
  let dt = time.delta_secs();
  shroomlings.iter_mut().for_each(|(hopper, mut shown, mut transform, children)| {
    let before = shown.at;
    shown.at = before.lerp(hopper.at, (dt * 14.0).min(1.0));
    let turn = (hopper.yaw - shown.yaw + PI).rem_euclid(TAU) - PI;
    shown.yaw += turn * (dt * 8.0).min(1.0);
    let pace = (shown.at - before).xz().length() / dt.max(1e-4);
    shown.stride += pace * dt * 9.0;
    let aim = if hopper.aloft { 0.18 } else { 0.0 };
    shown.squash += (aim - shown.squash) * (dt * 10.0).min(1.0);
    let swing = shown.stride.sin() * (pace / WALK).min(1.0);
    *transform = Transform::from_translation(shown.at)
      .with_rotation(Quat::from_rotation_y(shown.yaw));
    children.iter().for_each(|child| {
      if let Ok(mut trunk) = trunks.get_mut(child) {
        trunk.translation.y = 0.1 + swing.abs() * 0.03;
        trunk.scale = Vec3::new(
          1.0 - shown.squash * 0.4,
          1.0 + shown.squash,
          1.0 - shown.squash * 0.4
        )
      }
      if let Ok((foot, mut foot_transform)) = feet.get_mut(child) {
        foot_transform.rotation = Quat::from_rotation_x(swing * foot.0.cos() * 0.7)
      }
    })
  })
}

pub struct Shroomlings;

impl Plugin for Shroomlings {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Colonies>()
      .add_systems(
        Update,
        (muster, wander).chain().run_if(authority).run_if(resource_exists::<Voxels>)
      )
      .add_systems(Startup, mould.run_if(plays))
      .add_systems(Update, (dress, animate).chain().run_if(plays));
  }
}
