use {crate::{authority::{Controller, player_of, within_reach},
             block::{Block, Fluid, Tile},
             boat::{pierce, planked},
             island::SEA,
             menu::Menu,
             player::{Bulk, Craft, Pilot, Riding, Selected, captured, cells, slide},
             protocol::*,
             stream::{Palette, ready_around},
             texture::uv_corner,
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            ecs::entity::MapEntities,
            mesh::{Indices, PrimitiveTopology},
            prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize},
     std::f32::consts::TAU};

pub const FLOOR: f32 = 0.12;
const BASKET: Bulk = Bulk { half: 0.7, tall: 1.12 };
const RISE: f32 = 2.6;
const ENVELOPE: Bulk = Bulk { half: 2.4, tall: 5.6 };
const BURNER: (Vec3, Vec3) = (Vec3::new(-0.65, 1.9, -0.65), Vec3::new(0.65, 2.6, 0.65));

const GRAVITY: f32 = 9.8;
const AMBIENT: f32 = 290.0;
const HOTTEST: f32 = 400.0;
const BUOYANCY: f32 = 5.5;
const THINNING: f32 = 300.0;
const HEATING: f32 = 14.0;
const COOLING: f32 = 0.035;
const VENTING: f32 = 0.2;
const FAN: f32 = 13.0;
const TURN: f32 = 1.0;
const AIR: f32 = 0.45;
const STILL: f32 = 0.1;
const FLOAT: f32 = 30.0;
const SOAKED: f32 = 2.0;
const FRICTION: f32 = 5.0;
const FASTEST: f32 = 20.0;
const STEP: f32 = 1.0 / 60.0;
const DRIFT_EVERY: f32 = 0.05;
pub const TANK: f32 = 600.0;

pub fn burns(block: Block) -> Option<f32> {
  match block {
    Block::CoalOre | Block::Charcoal => Some(45.0),
    Block::Log | Block::BirchLog | Block::SpruceLog | Block::PalmLog => Some(15.0),
    Block::Planks => Some(4.0),
    _ => None
  }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Aerostat {
  pub at: Vec3,
  pub yaw: f32,
  pub heat: f32,
  pub fuel: f32,
  pub burning: bool,
  pub pattern: u8,
  #[entities]
  pub rider: Option<Entity>
}

impl Aerostat {
  pub fn landed(at: Vec3, yaw: f32, fuel: f32, pattern: u8) -> Aerostat {
    Aerostat { at, yaw, heat: AMBIENT, fuel, burning: false, pattern, rider: None }
  }
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Inflate(pub IVec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Ascend(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Deflate(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Stoke(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Drift {
  pub at: Vec3,
  pub yaw: f32,
  pub velocity: Vec3,
  pub heat: f32,
  pub burning: bool
}

#[derive(Clone, Copy, PartialEq)]
pub struct Flight {
  pub at: Vec3,
  pub velocity: Vec3,
  pub heat: f32,
  pub grounded: bool
}

#[derive(Clone, Copy)]
pub struct Helm {
  pub burn: bool,
  pub vent: bool,
  pub push: f32,
  pub yaw: f32
}

impl Helm {
  const IDLE: Helm = Helm { burn: false, vent: false, push: 0.0, yaw: 0.0 };
}

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn lift(heat: f32, height: f32) -> f32 {
  let thin = (-(height - SEA as f32) / THINNING).exp();
  GRAVITY * BUOYANCY * thin * (1.0 - AMBIENT / heat)
}

fn watery(voxels: &Voxels, cell: IVec3) -> bool {
  voxels
    .block(cell)
    .is_some_and(|block| block.liquid().is_some_and(|(fluid, _)| fluid == Fluid::Water))
}

fn soaked(voxels: &Voxels, at: Vec3) -> f32 {
  let bottom = at.y - FLOOR;
  let cell = (at - Vec3::Y * FLOOR).floor().as_ivec3();
  (0..2)
    .map(|rise| cell + IVec3::Y * rise)
    .filter(|&cell| watery(voxels, cell))
    .map(|cell| {
      let top = cell.y as f32 + if watery(voxels, cell + IVec3::Y) { 1.0 } else { 0.88 };
      (top.min(bottom + BASKET.tall) - bottom.max(cell.y as f32)).max(0.0)
    })
    .sum()
}

fn clear(voxels: &Voxels, bulk: Bulk, at: Vec3) -> bool {
  let (low, high) = bulk.body(at);
  cells(low, high).all(|cell| !voxels.solid(cell))
}

fn shift(voxels: &Voxels, at: Vec3, axis: usize, distance: f32) -> (Vec3, bool) {
  let (travel, hit) = [(BASKET, -FLOOR), (ENVELOPE, RISE)]
    .into_iter()
    .map(|(bulk, rise)| (bulk, at + Vec3::Y * rise))
    .filter(|&(bulk, from)| clear(voxels, bulk, from))
    .map(|(bulk, from)| {
      let (moved, hit) = slide(voxels, bulk, from, axis, distance);
      (moved[axis] - from[axis], hit)
    })
    .fold((distance, false), |(travel, hit), (more, blocked)| {
      (if more.abs() < travel.abs() { more } else { travel }, hit || blocked)
    });
  let mut moved = at;
  moved[axis] += travel;
  (moved, hit)
}

pub fn drift(voxels: &Voxels, flight: Flight, helm: Helm, dt: f32) -> Flight {
  let Flight { at, velocity, heat, grounded } = flight;
  let warming = HEATING * f32::from(u8::from(helm.burn));
  let losing = (heat - AMBIENT) * (COOLING + VENTING * f32::from(u8::from(helm.vent)));
  let heat = (heat + (warming - losing) * dt).clamp(AMBIENT, HOTTEST);
  let heat = if heat - AMBIENT < 0.05 && !helm.burn { AMBIENT } else { heat };
  let soak = soaked(voxels, at);
  let force = Vec3::Y * (lift(heat, at.y) - GRAVITY + FLOAT * soak)
    + ahead(helm.yaw) * helm.push * FAN
    - velocity * velocity.length() * AIR
    - velocity * (STILL + SOAKED * f32::from(u8::from(soak > 0.0)));
  let velocity = (velocity + force * dt).clamp_length_max(FASTEST);
  let velocity = match grounded {
    true => velocity * Vec3::new(1.0 - FRICTION * dt, 1.0, 1.0 - FRICTION * dt),
    false => velocity
  };
  let (at, velocity, landed) = [1, 0, 2].into_iter().fold(
    (at, velocity, false),
    |(at, mut velocity, landed), axis| {
      let (moved, hit) = shift(voxels, at, axis, velocity[axis] * dt);
      let landed = landed || (hit && axis == 1 && velocity.y < 0.0);
      if hit {
        velocity[axis] = 0.0
      }
      (moved, velocity, landed)
    }
  );
  Flight { at, velocity, heat, grounded: landed }
}

pub fn struck(aerostat: &Aerostat, from: Vec3, toward: Vec3) -> Option<f32> {
  let turn = Quat::from_rotation_y(-aerostat.yaw);
  let (origin, direction) = (turn * (from - aerostat.at), turn * toward);
  let basket = (
    Vec3::new(-BASKET.half, -FLOOR, -BASKET.half),
    Vec3::new(BASKET.half, BASKET.tall - FLOOR, BASKET.half)
  );
  let envelope = (
    Vec3::new(-ENVELOPE.half, RISE, -ENVELOPE.half),
    Vec3::new(ENVELOPE.half, RISE + ENVELOPE.tall, ENVELOPE.half)
  );
  [basket, BURNER, envelope]
    .into_iter()
    .filter_map(|(low, high)| pierce(low, high, origin, direction))
    .min_by(f32::total_cmp)
}

pub fn basket(aerostat: &Aerostat) -> (Vec3, f32, Vec2, f32) {
  (aerostat.at - Vec3::Y * FLOOR, aerostat.yaw, Vec2::splat(BASKET.half), BASKET.tall)
}

const PATTERNS: [[[f32; 3]; 2]; 5] = [
  [[0.78, 0.12, 0.1], [0.92, 0.86, 0.7]],
  [[0.12, 0.3, 0.72], [0.95, 0.78, 0.2]],
  [[0.16, 0.55, 0.24], [0.9, 0.9, 0.86]],
  [[0.45, 0.18, 0.6], [0.95, 0.72, 0.25]],
  [[0.92, 0.45, 0.1], [0.12, 0.55, 0.58]]
];

const PROFILE: [(f32, f32); 9] = [
  (0.8, RISE - 0.2),
  (1.5, 3.1),
  (2.2, 4.0),
  (2.6, 5.1),
  (2.6, 6.1),
  (2.25, 7.0),
  (1.6, 7.7),
  (0.8, 8.1),
  (0.0, 8.25)
];
const GORES: usize = 16;

fn envelope([one, two]: [[f32; 3]; 2]) -> Mesh {
  let quads: Vec<([Vec3; 4], [Vec3; 4], [f32; 3])> = (0..GORES)
    .flat_map(|gore| {
      let (from, to) =
        (gore as f32 / GORES as f32 * TAU, (gore + 1) as f32 / GORES as f32 * TAU);
      let color = if gore % 2 == 0 { one } else { two };
      PROFILE.windows(2).map(move |pair| {
        let [(low_r, low_y), (high_r, high_y)] = [pair[0], pair[1]];
        let out = Vec2::new(high_y - low_y, low_r - high_r).normalize();
        let point = |turn: f32, radius: f32, y: f32| {
          Vec3::new(radius * turn.cos(), y, radius * turn.sin())
        };
        let normal = |turn: f32| Vec3::new(out.x * turn.cos(), out.y, out.x * turn.sin());
        (
          [
            point(from, low_r, low_y),
            point(to, low_r, low_y),
            point(to, high_r, high_y),
            point(from, high_r, high_y)
          ],
          [normal(from), normal(to), normal(to), normal(from)],
          color
        )
      })
    })
    .collect();
  let corners =
    [Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.0), Vec2::ZERO];
  let sides = [(1.0, [0, 2, 1, 0, 3, 2]), (-1.0, [0, 1, 2, 0, 2, 3])];
  let faces: Vec<_> = sides
    .iter()
    .flat_map(|&(facing, order)| {
      quads.iter().map(move |&(points, normals, color)| {
        let [r, g, b] =
          color.map(|channel| channel * if facing > 0.0 { 1.0 } else { 0.55 });
        (points, normals.map(|normal| normal * facing), [r, g, b, 1.0], order)
      })
    })
    .collect();
  let positions: Vec<[f32; 3]> =
    faces.iter().flat_map(|(points, ..)| points.map(|point| point.to_array())).collect();
  let normals: Vec<[f32; 3]> = faces
    .iter()
    .flat_map(|(_, normals, ..)| normals.map(|normal| normal.to_array()))
    .collect();
  let colors: Vec<[f32; 4]> =
    faces.iter().flat_map(|&(_, _, color, _)| [color; 4]).collect();
  let uvs: Vec<[f32; 2]> = faces
    .iter()
    .flat_map(|_| corners.map(|corner| uv_corner(Tile::Wool, corner).to_array()))
    .collect();
  let indices: Vec<u32> = faces
    .iter()
    .enumerate()
    .flat_map(|(index, &(.., order))| order.map(|corner| index as u32 * 4 + corner))
    .collect();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn gondola() -> Mesh {
  let (half, wall, rim) = (BASKET.half, 0.08, BASKET.tall - FLOOR);
  let wicker = planked(
    &[
      (Vec3::new(-half, -FLOOR, -half), Vec3::new(half, 0.0, half)),
      (Vec3::new(-half, 0.0, -half), Vec3::new(-half + wall, rim, half)),
      (Vec3::new(half - wall, 0.0, -half), Vec3::new(half, rim, half)),
      (Vec3::new(-half + wall, 0.0, -half), Vec3::new(half - wall, rim, -half + wall)),
      (Vec3::new(-half + wall, 0.0, half - wall), Vec3::new(half - wall, rim, half)),
      (
        Vec3::new(-half - 0.03, rim - 0.1, -half - 0.03),
        Vec3::new(half + 0.03, rim, half + 0.03)
      )
    ],
    [0.82, 0.62, 0.38]
  );
  let post = |x: f32, z: f32| {
    (Vec3::new(x - 0.03, rim, z - 0.03), Vec3::new(x + 0.03, RISE - 0.1, z + 0.03))
  };
  let corner = half - 0.06;
  let iron = planked(
    &[
      post(-corner, -corner),
      post(corner, -corner),
      post(-corner, corner),
      post(corner, corner),
      (Vec3::new(-corner, 2.2, -0.03), Vec3::new(corner, 2.26, 0.03)),
      (Vec3::new(-0.03, 2.2, -corner), Vec3::new(0.03, 2.26, corner)),
      (Vec3::new(-0.22, 2.0, -0.22), Vec3::new(0.22, 2.3, 0.22))
    ],
    [0.32, 0.32, 0.34]
  );
  let mut gondola = wicker;
  gondola.merge(&iron).ok();
  gondola
}

#[derive(Resource)]
struct Kit {
  envelopes: Vec<Handle<Mesh>>,
  gondola: Handle<Mesh>,
  flame: Handle<Mesh>,
  fire: Handle<StandardMaterial>
}

#[derive(Component)]
struct Burner;

#[derive(Resource, Default)]
struct Firing(bool);

fn build(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  commands.insert_resource(Kit {
    envelopes: PATTERNS.iter().map(|&pattern| meshes.add(envelope(pattern))).collect(),
    gondola: meshes.add(gondola()),
    flame: meshes.add(Cone { radius: 0.16, height: 0.55 }),
    fire: materials.add(StandardMaterial {
      base_color: Color::srgb(1.0, 0.7, 0.25),
      emissive: LinearRgba::rgb(14.0, 5.0, 0.8),
      unlit: true,
      ..default()
    })
  })
}

fn moor(
  time: Res<Time>,
  kit: Res<Kit>,
  palette: Res<Palette>,
  pilot: Option<Res<Pilot>>,
  firing: Res<Firing>,
  mut commands: Commands,
  mut aerostats: Query<(Entity, &Aerostat, Option<&mut Transform>), Without<Burner>>,
  mut burners: Query<(&ChildOf, &mut Transform, &mut Visibility), With<Burner>>
) {
  let blend = (time.delta_secs() * 12.0).min(1.0);
  let ridden_by_me = |entity: Entity| {
    pilot.as_ref().and_then(|pilot| {
      pilot
        .riding
        .filter(|riding| riding.craft == entity && riding.kind == Craft::Balloon)
        .map(|riding| (pilot.at, riding.yaw))
    })
  };
  for (entity, aerostat, transform) in aerostats.iter_mut() {
    let ridden = ridden_by_me(entity);
    let (at, yaw) = ridden.unwrap_or((aerostat.at, aerostat.yaw));
    let wanted =
      Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw));
    match transform {
      Some(mut transform) if ridden.is_some() => *transform = wanted,
      Some(mut transform) => {
        transform.translation = transform.translation.lerp(wanted.translation, blend);
        transform.rotation = transform.rotation.slerp(wanted.rotation, blend)
      }
      None => {
        commands.entity(entity).insert((wanted, Visibility::default())).with_children(
          |balloon| {
            let solid = MeshMaterial3d(palette.solid.clone());
            balloon.spawn((
              Mesh3d(
                kit.envelopes[usize::from(aerostat.pattern) % PATTERNS.len()].clone()
              ),
              solid.clone()
            ));
            balloon.spawn((Mesh3d(kit.gondola.clone()), solid));
            balloon
              .spawn((
                Burner,
                Mesh3d(kit.flame.clone()),
                MeshMaterial3d(kit.fire.clone()),
                Transform::from_xyz(0.0, 2.55, 0.0),
                Visibility::Hidden,
                bevy::light::NotShadowCaster
              ))
              .with_children(|flame| {
                flame
                  .spawn((crate::sky::torchlight(), Transform::from_xyz(0.0, 0.3, 0.0)));
              });
          }
        );
      }
    }
  }
  let flicker = 1.0 + (time.elapsed_secs() * 31.0).sin() * 0.12;
  for (&ChildOf(parent), mut transform, mut visibility) in burners.iter_mut() {
    let lit = match ridden_by_me(parent) {
      Some(_) => firing.0,
      None => aerostats.get(parent).is_ok_and(|(_, aerostat, _)| aerostat.burning)
    };
    *visibility = if lit { Visibility::Inherited } else { Visibility::Hidden };
    transform.scale = Vec3::new(1.0, flicker, 1.0)
  }
}

fn embark(
  pilot: Option<ResMut<Pilot>>,
  aerostats: Query<(Entity, &Aerostat)>,
  mut seated: Local<Option<Entity>>
) {
  if let Some(mut pilot) = pilot {
    let me = pilot.me;
    let now = aerostats
      .iter()
      .find(|(_, aerostat)| aerostat.rider == Some(me))
      .map(|(entity, _)| entity);
    match (now, *seated) {
      (Some(craft), before) if before != Some(craft) => {
        if let Ok((_, aerostat)) = aerostats.get(craft) {
          pilot.riding = Some(Riding {
            craft,
            kind: Craft::Balloon,
            yaw: aerostat.yaw,
            speed: 0.0,
            heat: aerostat.heat
          });
          pilot.at = aerostat.at;
          pilot.velocity = Vec3::ZERO;
          pilot.grounded = false
        }
      }
      (None, Some(before))
        if pilot.riding.is_some_and(|riding| riding.craft == before) =>
      {
        pilot.riding = None
      }
      _ => ()
    }
    *seated = now
  }
}

fn soar(
  time: Res<Time>,
  keys: Res<ButtonInput<KeyCode>>,
  menu: Res<Menu>,
  voxels: Option<Res<Voxels>>,
  aerostats: Query<&Aerostat>,
  mut pilot: ResMut<Pilot>,
  mut firing: ResMut<Firing>,
  mut leaves: MessageWriter<Disembark>,
  mut drifts: MessageWriter<Drift>,
  mut since: Local<f32>,
  mut spare: Local<f32>,
  mut grounded: Local<bool>
) {
  if let Some(voxels) = voxels
    && let Some(mut riding) = pilot.riding
    && riding.kind == Craft::Balloon
    && let Ok(aerostat) = aerostats.get(riding.craft)
    && ready_around(&voxels, pilot.at)
  {
    let pressed = |key: KeyCode| menu.idle() && keys.pressed(key);
    let held = |key: KeyCode| f32::from(u8::from(pressed(key)));
    match menu.idle() && keys.just_pressed(KeyCode::ShiftLeft) {
      true => {
        leaves.write(Disembark);
        firing.0 = false;
        pilot.riding = None;
        pilot.at += Vec3::Y * 0.3;
        pilot.velocity += Vec3::Y * 5.0
      }
      false => {
        let burn = pressed(KeyCode::Space) && aerostat.fuel > 0.0;
        firing.0 = burn;
        let banked = (*spare + time.delta_secs()).min(0.25);
        let steps = (banked / STEP).floor();
        *spare = banked - steps * STEP;
        for _ in 0..steps as u32 {
          let turn = (held(KeyCode::KeyA) - held(KeyCode::KeyD)) * TURN * STEP;
          riding.yaw += turn;
          pilot.yaw += turn;
          let helm = Helm {
            burn,
            vent: pressed(KeyCode::KeyC),
            push: held(KeyCode::KeyW) - held(KeyCode::KeyS) * 0.5,
            yaw: riding.yaw
          };
          let flight = drift(
            &voxels,
            Flight {
              at: pilot.at,
              velocity: pilot.velocity,
              heat: riding.heat,
              grounded: *grounded
            },
            helm,
            STEP
          );
          pilot.at = flight.at;
          pilot.velocity = flight.velocity;
          riding.heat = flight.heat;
          *grounded = flight.grounded
        }
        pilot.riding = Some(riding);
        *since += time.delta_secs();
        if *since >= DRIFT_EVERY {
          *since = 0.0;
          drifts.write(Drift {
            at: pilot.at,
            yaw: riding.yaw,
            velocity: pilot.velocity,
            heat: riding.heat,
            burning: burn
          });
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
  aerostats: Query<(Entity, &Aerostat)>,
  mut ascents: MessageWriter<Ascend>,
  mut deflations: MessageWriter<Deflate>,
  mut stokes: MessageWriter<Stoke>,
  mut inflations: MessageWriter<Inflate>
) {
  if let Some(voxels) = voxels
    && captured(&cursor, &menu)
  {
    let (from, toward) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
    let hit = voxels.cast(from, toward, REACH);
    let blocked =
      hit.as_ref().map_or(REACH, |hit| (hit.at.as_vec3() + 0.5).distance(from));
    let riding = pilot.riding.map(|riding| riding.craft);
    let target = aerostats
      .iter()
      .filter(|&(entity, _)| riding.is_none_or(|craft| craft == entity))
      .filter_map(|(entity, aerostat)| {
        struck(aerostat, from, toward).map(|distance| (entity, aerostat, distance))
      })
      .filter(|&(.., distance)| distance <= REACH.min(blocked))
      .min_by(|a, b| a.2.total_cmp(&b.2));
    let holding = inventories
      .get(pilot.me)
      .ok()
      .and_then(|inventory| inventory.slots[selected.0])
      .map(|stack| stack.block);
    let fuel = holding.and_then(burns).is_some();
    match (
      target,
      buttons.just_pressed(MouseButton::Right),
      buttons.just_pressed(MouseButton::Left)
    ) {
      (Some((balloon, ..)), true, _) if fuel => {
        stokes.write(Stoke(balloon));
      }
      (Some((balloon, aerostat, _)), true, _)
        if riding.is_none() && aerostat.rider.is_none() =>
      {
        ascents.write(Ascend(balloon));
      }
      (Some((balloon, aerostat, _)), _, true)
        if riding.is_none() && aerostat.rider.is_none() =>
      {
        deflations.write(Deflate(balloon));
      }
      (None, true, _) if riding.is_none() && holding == Some(Block::Balloon) => {
        if let Some(hit) = hit.filter(|hit| hit.normal == IVec3::Y) {
          inflations.write(Inflate(hit.at + IVec3::Y));
        }
      }
      _ => ()
    }
  }
}

#[derive(Component)]
struct Adrift(Flight);

fn spacious(voxels: &mut Voxels, at: Vec3) -> bool {
  let (low, high) = BASKET.body(at - Vec3::Y * FLOOR);
  cells(low, high).all(|cell| !voxels.ensure(cell).solid())
}

fn serve(
  (mut inflations, mut ascents, mut deflations, mut stokes, mut leaves, mut drifts): (
    MessageReader<FromClient<Inflate>>,
    MessageReader<FromClient<Ascend>>,
    MessageReader<FromClient<Deflate>>,
    MessageReader<FromClient<Stoke>>,
    MessageReader<FromClient<Disembark>>,
    MessageReader<FromClient<Drift>>
  ),
  mut voxels: ResMut<Voxels>,
  mut commands: Commands,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Inventory))>,
  mut aerostats: Query<(Entity, &mut Aerostat, &mut Adrift)>
) {
  for &FromClient { client_id, message: Inflate(cell) } in inflations.read() {
    let at = cell.as_vec3() + Vec3::new(0.5, FLOOR, 0.5);
    if spacious(&mut voxels, at)
      && voxels.ensure(cell - IVec3::Y).solid()
      && let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, cell)
      && inventory.take(Block::Balloon)
    {
      let pattern =
        crate::noise::hash(0xBA11, cell.x, cell.y, cell.z) % PATTERNS.len() as u32;
      spawn(&mut commands, Aerostat::landed(at, avatar.yaw, 0.0, pattern as u8))
    }
  }
  for &FromClient { client_id, message: Ascend(balloon) } in ascents.read() {
    if let Some((player, avatar, _)) = player_of(players.iter_mut(), client_id)
      && aerostats.iter().all(|(_, aerostat, _)| aerostat.rider != Some(player))
      && let Ok((_, mut aerostat, _)) = aerostats.get_mut(balloon)
      && aerostat.rider.is_none()
      && within_reach(avatar, aerostat.at.floor().as_ivec3())
    {
      aerostat.rider = Some(player)
    }
  }
  for &FromClient { client_id, message: Deflate(balloon) } in deflations.read() {
    if let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Ok((_, aerostat, _)) = aerostats.get(balloon)
      && aerostat.rider.is_none()
      && within_reach(avatar, aerostat.at.floor().as_ivec3())
      && inventory.add(Block::Balloon)
    {
      commands.entity(balloon).despawn()
    }
  }
  for &FromClient { client_id, message: Stoke(balloon) } in stokes.read() {
    if let Some((player, avatar, mut inventory)) =
      player_of(players.iter_mut(), client_id)
      && let Ok((_, mut aerostat, _)) = aerostats.get_mut(balloon)
      && (aerostat.rider == Some(player)
        || within_reach(avatar, aerostat.at.floor().as_ivec3()))
      && aerostat.fuel < TANK
      && let Some(stack) = inventory.slots[usize::from(avatar.hand)]
      && let Some(seconds) = burns(stack.block)
      && inventory.take(stack.block)
    {
      aerostat.fuel = (aerostat.fuel + seconds).min(TANK)
    }
  }
  for &FromClient { client_id, .. } in leaves.read() {
    if let Some((player, ..)) = player_of(players.iter_mut(), client_id) {
      for (_, mut aerostat, _) in
        aerostats.iter_mut().filter(|(_, aerostat, _)| aerostat.rider == Some(player))
      {
        aerostat.rider = None;
        aerostat.burning = false
      }
    }
  }
  for &FromClient { client_id, message: Drift { at, yaw, velocity, heat, burning } } in
    drifts.read()
  {
    if let Some((player, ..)) = player_of(players.iter_mut(), client_id)
      && at.is_finite()
      && yaw.is_finite()
      && velocity.is_finite()
      && heat.is_finite()
    {
      for (_, mut aerostat, mut adrift) in
        aerostats.iter_mut().filter(|(_, aerostat, _)| aerostat.rider == Some(player))
      {
        let heat = match aerostat.fuel > 0.0 {
          true => heat.clamp(AMBIENT, HOTTEST),
          false => heat.clamp(AMBIENT, aerostat.heat.max(AMBIENT))
        };
        adrift.0 = Flight { at, velocity, heat, grounded: false };
        let moved = Aerostat {
          at,
          yaw,
          heat,
          burning: burning && aerostat.fuel > 0.0,
          ..*aerostat
        };
        aerostat.set_if_neq(moved);
      }
    }
  }
}

pub fn spawn(commands: &mut Commands, aerostat: Aerostat) {
  commands.spawn((
    Replicated,
    aerostat,
    Adrift(Flight {
      at: aerostat.at,
      velocity: Vec3::ZERO,
      heat: aerostat.heat,
      grounded: true
    })
  ));
}

fn burn(time: Res<Time>, mut aerostats: Query<&mut Aerostat>) {
  for mut aerostat in aerostats.iter_mut().filter(|aerostat| aerostat.burning) {
    aerostat.fuel = (aerostat.fuel - time.delta_secs()).max(0.0);
    aerostat.burning = aerostat.fuel > 0.0 && aerostat.rider.is_some()
  }
}

fn settle(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut aerostats: Query<(&mut Aerostat, &mut Adrift)>
) {
  for (mut aerostat, mut adrift) in
    aerostats.iter_mut().filter(|(aerostat, _)| aerostat.rider.is_none())
  {
    for corner in [-3.0, 3.0].into_iter().flat_map(|x| {
      [-1.0, 9.0]
        .into_iter()
        .flat_map(move |y| [-3.0, 3.0].map(move |z| Vec3::new(x, y, z)))
    }) {
      voxels.ensure((adrift.0.at + corner).floor().as_ivec3());
    }
    let steps = (time.delta_secs() / STEP).ceil().clamp(1.0, 15.0);
    let dt = time.delta_secs() / steps;
    adrift.0 = (0..steps as u32)
      .fold(adrift.0, |flight, _| drift(&voxels, flight, Helm::IDLE, dt));
    let Flight { at, heat, .. } = adrift.0;
    if at.distance(aerostat.at) > 1e-3 || (heat - aerostat.heat).abs() > 0.1 {
      aerostat.at = at;
      aerostat.heat = heat
    }
  }
}

fn unseat(players: Query<(), With<Controller>>, mut aerostats: Query<&mut Aerostat>) {
  for mut aerostat in aerostats.iter_mut() {
    if aerostat.rider.is_some_and(|rider| players.get(rider).is_err()) {
      aerostat.rider = None;
      aerostat.burning = false
    }
  }
}

#[derive(Component)]
struct Gauge;

fn gauge(mut commands: Commands) {
  commands.spawn((
    Gauge,
    Text::new(""),
    TextFont { font_size: FontSize::Px(16.0), ..default() },
    crate::hud::SHADE,
    Node {
      position_type: PositionType::Absolute,
      right: px(12),
      bottom: px(12),
      ..default()
    },
    Visibility::Hidden
  ));
}

fn read(
  pilot: Res<Pilot>,
  aerostats: Query<&Aerostat>,
  mut gauges: Query<(&mut Text, &mut Visibility), With<Gauge>>
) {
  let riding =
    pilot.riding.filter(|riding| riding.kind == Craft::Balloon).and_then(|riding| {
      aerostats.get(riding.craft).ok().map(|aerostat| (riding, aerostat))
    });
  for (mut text, mut visibility) in gauges.iter_mut() {
    *visibility =
      if riding.is_some() { Visibility::Inherited } else { Visibility::Hidden };
    if let Some((riding, aerostat)) = riding {
      let reading = match aerostat.fuel > 0.0 {
        true => format!(
          "Fuel {:.0} s   Envelope {:.0} C   Climb {:+.1}\nSpace burner, C vent, W/S fan, A/D turn, Shift leave",
          aerostat.fuel,
          riding.heat - 273.0,
          pilot.velocity.y
        ),
        false => format!(
          "Out of fuel   Envelope {:.0} C   Climb {:+.1}\nRight-click the burner holding coal, charcoal, logs or planks",
          riding.heat - 273.0,
          pilot.velocity.y
        )
      };
      text.set_if_neq(Text::new(reading));
    }
  }
}

pub struct Balloons;

impl Plugin for Balloons {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Firing>()
      .add_systems(Startup, (build, gauge).run_if(plays))
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
        (burn, settle).run_if(authority).run_if(resource_exists::<Voxels>)
      )
      .add_systems(
        Update,
        (embark, soar, handle, moor, read)
          .chain()
          .run_if(plays)
          .run_if(resource_exists::<Pilot>)
      );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn open_air() -> Voxels { Voxels::new(1) }

  #[test]
  fn hot_air_lifts_and_cold_air_sinks() {
    let voxels = open_air();
    let start = Flight {
      at: Vec3::new(0.5, 400.0, 0.5),
      velocity: Vec3::ZERO,
      heat: AMBIENT,
      grounded: false
    };
    let fly = |helm: Helm, seconds: f32| {
      (0..(seconds / STEP) as u32)
        .fold(start, |flight, _| drift(&voxels, flight, helm, STEP))
    };
    let cold = fly(Helm::IDLE, 5.0);
    assert!(cold.velocity.y < -3.0 && cold.velocity.y > -6.0, "{}", cold.velocity);
    let burning = fly(Helm { burn: true, ..Helm::IDLE }, 20.0);
    assert!(burning.heat > 350.0, "{}", burning.heat);
    assert!(burning.velocity.y > cold.velocity.y, "{}", burning.velocity);
  }

  #[test]
  fn hovering_heat_sits_between_ambient_and_hottest() {
    let hover = AMBIENT / (1.0 - 1.0 / BUOYANCY);
    assert!(hover > AMBIENT + 30.0 && hover < HOTTEST - 20.0, "{hover}");
    assert!(lift(HOTTEST, SEA as f32 + 200.0) < GRAVITY);
    assert!(lift(HOTTEST, SEA as f32) > GRAVITY * 1.3)
  }
}
