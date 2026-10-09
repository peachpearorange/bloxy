use {crate::{block::Block,
             opts::opts,
             protocol::*,
             stream::ready_around,
             voxels::{Hit, Voxels}},
     bevy::{input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
            prelude::*,
            window::{CursorGrabMode, CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*};

const HALF_WIDTH: f32 = 0.3;
const TALL: f32 = 1.8;
const GRAVITY: f32 = 30.0;
const JUMP: f32 = 8.6;
const WALK: f32 = 4.3;
const SPRINT: f32 = 6.2;
const SWIM: f32 = 3.0;
const LOOK: f32 = 0.0022;
const SKIN: f32 = 0.001;
const STEP: f32 = 1.0 / 120.0;
const SEND_EVERY: f32 = 0.05;
const PLACE_EVERY: f32 = 0.22;

#[derive(Resource)]
pub struct Pilot {
  pub me: Entity,
  pub at: Vec3,
  pub velocity: Vec3,
  pub yaw: f32,
  pub pitch: f32,
  pub grounded: bool,
  pub swimming: bool
}

impl Pilot {
  pub fn eye(&self) -> Vec3 { self.at + Vec3::Y * EYE }

  pub fn facing(&self) -> Quat {
    Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0)
  }

  pub fn avatar(&self) -> Avatar {
    Avatar { at: self.at, yaw: self.yaw, pitch: self.pitch }
  }
}

#[derive(Resource, Default)]
pub struct Selected(pub usize);

#[derive(Resource, Default)]
pub struct Aim {
  pub hit: Option<Hit>,
  pub progress: f32,
  pub digging: Option<IVec3>
}

#[derive(Component)]
pub struct Eye;

#[derive(Component)]
pub struct Me;

fn possess(
  mut commands: Commands,
  mut possessions: MessageReader<Possess>,
  avatars: Query<&Avatar>,
  voxels: Option<Res<Voxels>>,
  mut waiting: Local<Option<Entity>>
) {
  *waiting = possessions.read().last().map(|possess| possess.0).or(*waiting);
  if let Some(me) = *waiting
    && let Ok(avatar) = avatars.get(me)
    && let Some(voxels) = voxels
  {
    *waiting = None;
    commands.entity(me).insert(Me);
    let at = opts().at.map_or(avatar.at, |[x, z]| {
      let (x, z) = (avatar.at.x + x, avatar.at.z + z);
      let ground =
        crate::generate::height(voxels.seed, x.floor() as i32, z.floor() as i32);
      Vec3::new(x, ground as f32 + 1.05, z)
    });
    commands.insert_resource(Pilot {
      me,
      at,
      velocity: Vec3::ZERO,
      yaw: opts().yaw.map_or(avatar.yaw, f32::to_radians),
      pitch: opts().pitch.map_or(avatar.pitch, f32::to_radians),
      grounded: false,
      swimming: false
    })
  }
}

fn spawn_eye(mut commands: Commands) {
  commands.spawn((
    Eye,
    Camera3d::default(),
    Projection::Perspective(PerspectiveProjection {
      fov: 75f32.to_radians(),
      ..default()
    }),
    crate::sky::lens(),
    Transform::default()
  ));
}

fn grab(
  mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
  buttons: Res<ButtonInput<MouseButton>>,
  keys: Res<ButtonInput<KeyCode>>
) {
  if let Ok(mut cursor) = cursor.single_mut() {
    if buttons.just_pressed(MouseButton::Left) && cursor.grab_mode == CursorGrabMode::None
    {
      cursor.grab_mode = CursorGrabMode::Locked;
      cursor.visible = false
    }
    if keys.just_pressed(KeyCode::Escape) {
      cursor.grab_mode = CursorGrabMode::None;
      cursor.visible = true
    }
  }
}

fn captured(cursor: &Query<&CursorOptions, With<PrimaryWindow>>) -> bool {
  cursor.single().is_ok_and(|cursor| cursor.grab_mode != CursorGrabMode::None)
    || opts().shot.is_some()
}

fn look(
  mut pilot: ResMut<Pilot>,
  motion: Res<AccumulatedMouseMotion>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>
) {
  if captured(&cursor) && motion.delta != Vec2::ZERO {
    pilot.yaw -= motion.delta.x * LOOK;
    pilot.pitch = (pilot.pitch - motion.delta.y * LOOK).clamp(-1.55, 1.55)
  }
}

fn body(at: Vec3) -> (Vec3, Vec3) {
  (
    at - Vec3::new(HALF_WIDTH, 0.0, HALF_WIDTH),
    at + Vec3::new(HALF_WIDTH, TALL, HALF_WIDTH)
  )
}

fn cells(low: Vec3, high: Vec3) -> impl Iterator<Item = IVec3> {
  let (from, to) = (low.floor().as_ivec3(), (high - SKIN).floor().as_ivec3());
  (from.y..=to.y).flat_map(move |y| {
    (from.z..=to.z).flat_map(move |z| (from.x..=to.x).map(move |x| IVec3::new(x, y, z)))
  })
}

fn slide(voxels: &Voxels, at: Vec3, axis: usize, distance: f32) -> (Vec3, bool) {
  let mut moved = at;
  moved[axis] += distance;
  let (low, high) = body(moved);
  let blocking: Vec<IVec3> =
    cells(low, high).filter(|&cell| voxels.solid(cell)).collect();
  match (blocking.is_empty(), distance > 0.0) {
    (true, _) => (moved, false),
    (false, true) => {
      let wall = blocking.iter().map(|cell| cell[axis]).min().unwrap_or_default() as f32;
      moved[axis] = wall - (high[axis] - moved[axis]) - SKIN;
      (moved, true)
    }
    (false, false) => {
      let wall =
        blocking.iter().map(|cell| cell[axis]).max().unwrap_or_default() as f32 + 1.0;
      moved[axis] = wall + (moved[axis] - low[axis]) + SKIN;
      (moved, true)
    }
  }
}

fn fly(
  time: Res<Time>,
  keys: Res<ButtonInput<KeyCode>>,
  voxels: Option<Res<Voxels>>,
  mut pilot: ResMut<Pilot>,
  mut spare: Local<f32>
) {
  if let Some(voxels) = voxels
    && ready_around(&voxels, pilot.at)
  {
    let held = |key: KeyCode| f32::from(u8::from(keys.pressed(key)));
    let wish = Vec2::new(
      held(KeyCode::KeyD) - held(KeyCode::KeyA),
      held(KeyCode::KeyS) - held(KeyCode::KeyW)
    );
    let speed = match (pilot.swimming, keys.pressed(KeyCode::ControlLeft)) {
      (true, _) => SWIM,
      (false, true) => SPRINT,
      (false, false) => WALK
    };
    let heading = Quat::from_rotation_y(pilot.yaw);
    let walk = heading * Vec3::new(wish.x, 0.0, wish.y).normalize_or_zero() * speed;
    let banked = (*spare + time.delta_secs()).min(0.25);
    let steps = (banked / STEP).floor();
    *spare = banked - steps * STEP;
    (0..steps as u32).for_each(|_| {
      let swimming =
        voxels.block((pilot.at + Vec3::Y * 0.6).floor().as_ivec3()) == Some(Block::Water);
      let blend = if pilot.grounded { 0.35 } else { 0.06 };
      let velocity = pilot.velocity;
      let horizontal = velocity.xz().lerp(walk.xz(), blend);
      let vertical = match (swimming, keys.pressed(KeyCode::Space), pilot.grounded) {
        (true, true, _) => (velocity.y + 20.0 * STEP).min(3.0),
        (true, false, _) => (velocity.y - 8.0 * STEP).max(-2.5),
        (false, true, true) => JUMP,
        (false, _, _) => (velocity.y - GRAVITY * STEP).max(-60.0)
      };
      pilot.velocity = Vec3::new(horizontal.x, vertical, horizontal.y);
      pilot.swimming = swimming;
      let (at, mut velocity) = (pilot.at, pilot.velocity);
      let (at, landed) = [1, 0, 2].into_iter().fold((at, false), |(at, landed), axis| {
        let (at, hit) = slide(&voxels, at, axis, velocity[axis] * STEP);
        if hit {
          velocity[axis] = 0.0
        }
        (at, landed || (axis == 1 && hit && pilot.velocity.y < 0.0))
      });
      pilot.at = at;
      pilot.velocity = velocity;
      pilot.grounded = landed
    })
  }
}

fn follow(pilot: Res<Pilot>, mut eyes: Query<&mut Transform, With<Eye>>) {
  eyes.iter_mut().for_each(|mut eye| {
    *eye = Transform::from_translation(pilot.eye()).with_rotation(pilot.facing())
  })
}

fn report(
  time: Res<Time>,
  pilot: Res<Pilot>,
  mut moves: MessageWriter<Moved>,
  mut since: Local<f32>,
  mut last: Local<Avatar>
) {
  *since += time.delta_secs();
  let avatar = pilot.avatar();
  if *since >= SEND_EVERY && avatar != *last {
    *since = 0.0;
    *last = avatar;
    moves.write(Moved(avatar));
  }
}

fn select(
  keys: Res<ButtonInput<KeyCode>>,
  scroll: Res<AccumulatedMouseScroll>,
  mut selected: ResMut<Selected>
) {
  let digits = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9
  ];
  let scrolled = match scroll.delta.y {
    0.0 => 0,
    delta => -delta.signum() as i32
  };
  selected.0 = digits
    .iter()
    .position(|&key| keys.just_pressed(key))
    .unwrap_or((selected.0 as i32 + scrolled).rem_euclid(HOTBAR as i32) as usize)
}

fn work(
  time: Res<Time>,
  buttons: Res<ButtonInput<MouseButton>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  role: Res<Role>,
  pilot: Res<Pilot>,
  selected: Res<Selected>,
  inventories: Query<&Inventory>,
  mut voxels: Option<ResMut<Voxels>>,
  mut aim: ResMut<Aim>,
  mut digs: MessageWriter<Dig>,
  mut puts: MessageWriter<Put>,
  mut cooldown: Local<f32>
) {
  if let Some(voxels) = voxels.as_deref_mut() {
    aim.hit = voxels.cast(pilot.eye(), pilot.facing() * Vec3::NEG_Z, REACH);
    let active = captured(&cursor);
    let target = aim.hit.as_ref().map(|hit| (hit.at, hit.block));
    match (target, active && buttons.pressed(MouseButton::Left)) {
      (Some((at, block)), true) if block.breakable() => {
        aim.progress = match aim.digging == Some(at) {
          true => {
            aim.progress
              + time.delta_secs() / block.seconds_to_break()
                * if opts().creative { 8.0 } else { 1.0 }
          }
          false => 0.0
        };
        aim.digging = Some(at);
        if aim.progress >= 1.0 {
          aim.progress = 0.0;
          aim.digging = None;
          digs.write(Dig { at });
          if *role == Role::Guest {
            voxels.set(at, Block::Air)
          }
        }
      }
      _ => {
        aim.progress = 0.0;
        aim.digging = None
      }
    }
    *cooldown = (*cooldown - time.delta_secs()).max(0.0);
    let stack =
      inventories.get(pilot.me).ok().and_then(|inventory| inventory.slots[selected.0]);
    if active
      && buttons.pressed(MouseButton::Right)
      && (*cooldown <= 0.0 || buttons.just_pressed(MouseButton::Right))
      && let Some(hit) = &aim.hit
      && let Some(stack) = stack
    {
      *cooldown = PLACE_EVERY;
      let at = hit.at + hit.normal;
      let (low, high) = body(pilot.at);
      let inside = cells(low, high).any(|cell| cell == at);
      if !inside && voxels.block(at).is_some_and(|block| !block.solid()) {
        puts.write(Put { at, block: stack.block });
        if *role == Role::Guest {
          voxels.set(at, stack.block)
        }
      }
    }
  }
}

fn apply_changes(
  mut changes: MessageReader<Altered>,
  role: Res<Role>,
  voxels: Option<ResMut<Voxels>>
) {
  if let Some(mut voxels) = voxels
    && *role == Role::Guest
  {
    changes.read().for_each(|change| voxels.set(change.at, change.block))
  }
}

fn arrive(
  mut commands: Commands,
  mut welcomes: MessageReader<Welcome>,
  mut hello: MessageWriter<Hello>
) {
  welcomes.read().for_each(|welcome| {
    let mut voxels = Voxels::new(welcome.seed);
    welcome.edits.iter().for_each(|&(at, block)| voxels.set(at, block));
    commands.insert_resource(voxels);
    hello.write(Hello { name: opts().called() });
  })
}

pub struct Piloting;

impl Plugin for Piloting {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Selected>()
      .init_resource::<Aim>()
      .add_systems(Startup, spawn_eye.run_if(plays))
      .add_systems(
        PreUpdate,
        arrive.after(ClientSystems::Receive).run_if(in_state(ClientState::Connected))
      )
      .add_systems(
        PreUpdate,
        apply_changes.after(ClientSystems::Receive).after(ServerSystems::Receive)
      )
      .add_systems(
        Update,
        (
          possess,
          grab,
          (look, fly, follow, select, work, report)
            .chain()
            .run_if(resource_exists::<Pilot>)
        )
          .chain()
          .run_if(plays)
      );
  }
}
