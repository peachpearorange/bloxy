use {crate::{block::Block,
             local,
             menu::{Menu, Tab, closed},
             opts::opts,
             protocol::*,
             settings::Settings,
             sign::Inscription,
             stream::ready_around,
             voxels::{Hit, Voxels}},
     bevy::{input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
            prelude::*,
            window::{CursorGrabMode, CursorOptions, MonitorSelection, PrimaryWindow,
                     WindowMode}},
     bevy_replicon::prelude::*};

const GRAVITY: f32 = 30.0;
const JUMP: f32 = 8.6;
const WALK: f32 = 4.3;
const SPRINT: f32 = 6.2;
const SWIM: f32 = 3.0;
const CLIMB: f32 = 2.8;
const SLIP: f32 = 1.6;
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
  pub swimming: bool,
  pub riding: Option<Riding>
}

#[derive(Clone, Copy)]
pub struct Riding {
  pub boat: Entity,
  pub yaw: f32,
  pub speed: f32
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
      swimming: false,
      riding: None
    })
  }
}

fn spawn_eye(mut commands: Commands, settings: Res<Settings>) {
  commands.spawn((
    Eye,
    Camera3d::default(),
    Projection::Perspective(PerspectiveProjection {
      fov: settings.fov.to_radians(),
      ..default()
    }),
    crate::sky::lens(&settings),
    Transform::default()
  ));
}

fn grab(
  mut windows: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
  buttons: Res<ButtonInput<MouseButton>>,
  menu: Res<Menu>
) {
  let playing = !menu.open && opts().shot.is_none();
  local::capture_on_click(playing);
  if let Ok((mut window, mut cursor)) = windows.single_mut()
    && playing
    && buttons.just_pressed(MouseButton::Left)
  {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
    window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current)
  }
}

pub fn captured(
  cursor: &Query<&CursorOptions, With<PrimaryWindow>>,
  menu: &Menu
) -> bool {
  !menu.open
    && (cursor.single().is_ok_and(|cursor| cursor.grab_mode != CursorGrabMode::None)
      || opts().shot.is_some())
}

fn look(
  mut pilot: ResMut<Pilot>,
  motion: Res<AccumulatedMouseMotion>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  settings: Res<Settings>
) {
  if captured(&cursor, &menu) && motion.delta != Vec2::ZERO {
    let turn = motion.delta * LOOK * settings.sensitivity;
    let rise = if settings.invert { -turn.y } else { turn.y };
    pilot.yaw -= turn.x;
    pilot.pitch = (pilot.pitch - rise).clamp(-1.55, 1.55)
  }
}

#[derive(Clone, Copy)]
pub struct Bulk {
  pub half: f32,
  pub tall: f32
}

impl Bulk {
  pub const PERSON: Bulk = Bulk { half: 0.3, tall: 1.8 };

  pub fn body(self, at: Vec3) -> (Vec3, Vec3) {
    (
      at - Vec3::new(self.half, 0.0, self.half),
      at + Vec3::new(self.half, self.tall, self.half)
    )
  }
}

pub fn cells(low: Vec3, high: Vec3) -> impl Iterator<Item = IVec3> {
  let (from, to) = (low.floor().as_ivec3(), (high - SKIN).floor().as_ivec3());
  (from.y..=to.y).flat_map(move |y| {
    (from.z..=to.z).flat_map(move |z| (from.x..=to.x).map(move |x| IVec3::new(x, y, z)))
  })
}

pub fn slide(
  voxels: &Voxels,
  bulk: Bulk,
  at: Vec3,
  axis: usize,
  distance: f32
) -> (Vec3, bool) {
  let mut moved = at;
  moved[axis] += distance;
  let (low, high) = bulk.body(moved);
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

pub struct Marched {
  pub at: Vec3,
  pub velocity: Vec3,
  pub blocked: bool,
  pub landed: bool
}

pub fn march(voxels: &Voxels, bulk: Bulk, at: Vec3, velocity: Vec3, dt: f32) -> Marched {
  let steps = (dt / (1.0 / 60.0)).ceil().max(1.0);
  (0..steps as u32).fold(
    Marched { at, velocity, blocked: false, landed: false },
    |marched, _| {
      [1, 0, 2].into_iter().fold(
        marched,
        |Marched { at, mut velocity, blocked, landed }, axis| {
          let falling = velocity[axis] < 0.0;
          let (at, hit) = slide(voxels, bulk, at, axis, velocity[axis] * dt / steps);
          if hit {
            velocity[axis] = 0.0
          }
          Marched {
            at,
            velocity,
            blocked: blocked || (hit && axis != 1),
            landed: landed || (hit && axis == 1 && falling)
          }
        }
      )
    }
  )
}

fn fly(
  time: Res<Time>,
  keys: Res<ButtonInput<KeyCode>>,
  menu: Res<Menu>,
  voxels: Option<Res<Voxels>>,
  mut pilot: ResMut<Pilot>,
  mut spare: Local<f32>
) {
  if let Some(voxels) = voxels
    && ready_around(&voxels, pilot.at)
    && pilot.riding.is_none()
  {
    let pressed = |key: KeyCode| !menu.open && keys.pressed(key);
    let held = |key: KeyCode| f32::from(u8::from(pressed(key)));
    let wish = Vec2::new(
      held(KeyCode::KeyD) - held(KeyCode::KeyA),
      held(KeyCode::KeyS) - held(KeyCode::KeyW)
    );
    let speed = match (pilot.swimming, pressed(KeyCode::ShiftLeft)) {
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
      let swimming = voxels
        .block((pilot.at + Vec3::Y * 0.6).floor().as_ivec3())
        .is_some_and(Block::fluid);
      let (low, high) = Bulk::PERSON.body(pilot.at);
      let climbing = cells(low - Vec3::splat(0.05), high + Vec3::splat(0.05))
        .any(|cell| voxels.block(cell).is_some_and(Block::ladder));
      let blend = if pilot.grounded || climbing { 0.35 } else { 0.06 };
      let velocity = pilot.velocity;
      let horizontal = velocity.xz().lerp(walk.xz(), blend);
      let rising = pressed(KeyCode::Space) || pressed(KeyCode::KeyW);
      let vertical = match (swimming, pressed(KeyCode::Space), pilot.grounded) {
        _ if climbing && !swimming && rising => CLIMB,
        _ if climbing && !swimming && pressed(KeyCode::KeyS) => -CLIMB,
        _ if climbing && !swimming => (velocity.y - GRAVITY * STEP).max(-SLIP),
        (true, true, _) => (velocity.y + 20.0 * STEP).min(3.0),
        (true, false, _) => (velocity.y - 8.0 * STEP).max(-2.5),
        (false, true, true) => JUMP,
        (false, _, _) => (velocity.y - GRAVITY * STEP).max(-60.0)
      };
      pilot.velocity = Vec3::new(horizontal.x, vertical, horizontal.y);
      pilot.swimming = swimming;
      let (at, mut velocity) = (pilot.at, pilot.velocity);
      let (at, landed) = [1, 0, 2].into_iter().fold((at, false), |(at, landed), axis| {
        let (at, hit) = slide(&voxels, Bulk::PERSON, at, axis, velocity[axis] * STEP);
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

#[derive(Resource, Default)]
pub struct Gait {
  pub stride: f32,
  pub bob: f32
}

impl Gait {
  pub fn sway(&self) -> Vec3 {
    let phase = self.stride * std::f32::consts::PI / STEP_LENGTH;
    Vec3::new(phase.sin() * 0.04, -phase.cos().abs() * 0.06, 0.0) * self.bob
  }
}

const STEP_LENGTH: f32 = 1.3;

fn follow(
  time: Res<Time>,
  pilot: Res<Pilot>,
  mut gait: ResMut<Gait>,
  mut eyes: Query<&mut Transform, With<Eye>>
) {
  let dt = time.delta_secs();
  let pace = match pilot.grounded && !pilot.swimming {
    true => pilot.velocity.xz().length(),
    false => 0.0
  };
  gait.stride += pace * dt;
  gait.bob += ((pace / WALK).min(1.4) - gait.bob) * (dt * 8.0).min(1.0);
  let sway = gait.sway();
  let roll = Quat::from_rotation_z(sway.x * 0.25);
  eyes.iter_mut().for_each(|mut eye| {
    *eye = Transform::from_translation(pilot.eye() + pilot.facing() * sway)
      .with_rotation(pilot.facing() * roll)
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
  mut menu: ResMut<Menu>,
  role: Res<Role>,
  pilot: Res<Pilot>,
  selected: Res<Selected>,
  inventories: Query<&Inventory>,
  mut voxels: Option<ResMut<Voxels>>,
  mut aim: ResMut<Aim>,
  (mut digs, mut puts, mut attunes, mut scoops, mut pours): (
    MessageWriter<Dig>,
    MessageWriter<Put>,
    MessageWriter<Attune>,
    MessageWriter<Scoop>,
    MessageWriter<Pour>
  ),
  (boats, folk): (Query<&Vessel>, Query<&Folk>),
  (mut inscription, signs, mut rests): (
    ResMut<Inscription>,
    Query<&Sign>,
    MessageWriter<Rest>
  ),
  mut cooldown: Local<f32>
) {
  if let Some(voxels) = voxels.as_deref_mut() {
    let (eye, toward) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
    aim.hit = voxels.cast(eye, toward, REACH).filter(|hit| {
      let distance = (hit.at.as_vec3() + 0.5).distance(eye);
      let boat = boats.iter().any(|vessel| {
        crate::boat::struck(vessel, eye, toward).is_some_and(|near| near < distance)
      });
      let person = folk.iter().any(|folk| {
        crate::folk::struck(folk.at, eye, toward).is_some_and(|near| near < distance)
      });
      !boat && !person
    });
    let active = captured(&cursor, &menu);
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
    let used = aim.hit.as_ref().filter(|hit| {
      hit.block.waystone() || hit.block.station() || hit.block.sign() || hit.block.bed()
    });
    if active
      && buttons.just_pressed(MouseButton::Right)
      && let Some(hit) = used
    {
      match hit.block {
        waystone if waystone.waystone() => {
          attunes.write(Attune(hit.at));
          menu.show(Tab::Waystones, time.elapsed_secs())
        }
        sign if sign.sign() => {
          let text =
            signs.iter().find(|sign| sign.at == hit.at).map(|sign| sign.text.as_str());
          inscription.begin(hit.at, text.unwrap_or_default());
          menu.show(Tab::Sign, time.elapsed_secs())
        }
        bed if bed.bed() => {
          rests.write(Rest(hit.at));
        }
        _ => menu.show(Tab::Inventory, time.elapsed_secs())
      }
    } else if active
      && buttons.just_pressed(MouseButton::Right)
      && let Some(stack) = stack.filter(|stack| {
        matches!(stack.block, Block::Bucket | Block::WaterBucket | Block::LavaBucket)
      })
      && let Some(hit) =
        voxels.cast_for(eye, toward, REACH, |block| block.targetable() || block.fluid())
    {
      match stack.block.carrying() {
        None => {
          if hit.block.liquid().is_some_and(|(_, level)| level == 0) {
            scoops.write(Scoop(hit.at));
          }
        }
        Some(fluid) => {
          let at = if hit.block.fluid() { hit.at } else { hit.at + hit.normal };
          pours.write(Pour { at, fluid });
        }
      }
    } else if active
      && used.is_none()
      && buttons.pressed(MouseButton::Right)
      && (*cooldown <= 0.0 || buttons.just_pressed(MouseButton::Right))
      && let Some(hit) = &aim.hit
      && let Some(stack) = stack.filter(|stack| stack.block.placeable())
    {
      *cooldown = PLACE_EVERY;
      let at = hit.at + hit.normal;
      let ahead = (pilot.facing() * Vec3::NEG_Z).xz();
      let facing = match ahead.x.abs() > ahead.y.abs() {
        true => IVec3::X * ahead.x.signum() as i32,
        false => IVec3::Z * ahead.y.signum() as i32
      };
      let wall = if hit.normal.y == 0 { -hit.normal } else { facing };
      let laid = (stack.block == Block::Bed).then(|| Block::laid(facing)).flatten();
      let block = laid.map_or(stack.block.against(wall), |(foot, _)| foot);
      let (low, high) = Bulk::PERSON.body(pilot.at);
      let inside = cells(low, high).any(|cell| cell == at) && block.solid();
      let hung = block.wall().is_none_or(|wall| voxels.solid(at + wall));
      let open = |cell: IVec3| voxels.block(cell).is_some_and(|block| !block.solid());
      let roomy =
        laid.is_none() || open(at + facing) && voxels.solid(at + facing - IVec3::Y);
      if !inside && hung && roomy && open(at) {
        puts.write(Put { at, block });
        if *role == Role::Guest {
          voxels.set(at, block);
          laid.into_iter().for_each(|(_, head)| voxels.set(at + facing, head))
        }
        if stack.block.sign() && voxels.block(at - IVec3::Y).is_some_and(Block::solid) {
          inscription.begin(at, "");
          menu.show(Tab::Sign, time.elapsed_secs())
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

fn arrive(mut commands: Commands, mut welcomes: MessageReader<Welcome>) {
  welcomes.read().for_each(|welcome| {
    let mut voxels = Voxels::new(welcome.seed);
    welcome.edits.iter().for_each(|&(at, block)| voxels.set(at, block));
    commands.insert_resource(voxels)
  })
}

pub struct Piloting;

impl Plugin for Piloting {
  fn build(&self, app: &mut App) {
    local::listen_for_clicks();
    app
      .init_resource::<Selected>()
      .init_resource::<Aim>()
      .init_resource::<Gait>()
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
          (look, fly, follow, select.run_if(closed), work, report)
            .chain()
            .run_if(resource_exists::<Pilot>)
        )
          .chain()
          .run_if(plays)
      );
  }
}
