use {crate::{authority::{Controller, player_of},
             beast::{Grain, Joint, Rest, Shapes},
             block::Block,
             figure::{Clad, Kit, clothe},
             generate,
             island::FACING_STONE,
             menu::Menu,
             player::{Bulk, Marched, Pilot, captured, march},
             protocol::*,
             skin::{Face, PX, Part, Skin},
             voxels::Voxels},
     bevy::{prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     std::f32::consts::{PI, TAU}};

const CHASE: f32 = 64.0;
const GIVE_UP: f32 = 40.0;
const MEND_AFTER: f32 = 8.0;
const MEND_EVERY: f32 = 3.0;
const PUNCH: u8 = 3;
const STRIKE_EVERY: f32 = 0.35;
const ARM_REACH: f32 = 1.4;
const GRAVITY: f32 = 30.0;
const SCRAMBLE: f32 = 11.5;
const SWING: f32 = 0.4;

impl Band {
  pub fn health(self) -> u8 {
    match self {
      Band::Trader => Health::FULL,
      Band::Pirate => 9,
      Band::Viking => 12
    }
  }

  fn speed(self) -> f32 {
    match self {
      Band::Viking => 3.8,
      _ => 3.4
    }
  }

  fn blow(self) -> u8 {
    match self {
      Band::Viking => 3,
      _ => 2
    }
  }
}

pub fn ray_box(from: Vec3, toward: Vec3, low: Vec3, high: Vec3) -> Option<f32> {
  let (near, far) =
    (0..3).fold((0.0_f32, f32::INFINITY), |(near, far), axis| {
      match toward[axis] == 0.0 {
        true => match (low[axis]..=high[axis]).contains(&from[axis]) {
          true => (near, far),
          false => (1.0, 0.0)
        },
        false => {
          let (first, second) = (
            (low[axis] - from[axis]) / toward[axis],
            (high[axis] - from[axis]) / toward[axis]
          );
          (near.max(first.min(second)), far.min(first.max(second)))
        }
      }
    });
  (near <= far).then_some(near)
}

pub fn struck(at: Vec3, from: Vec3, toward: Vec3) -> Option<f32> {
  ray_box(from, toward, at - Vec3::new(0.35, 0.0, 0.35), at + Vec3::new(0.35, 1.9, 0.35))
}

#[derive(Component, Default)]
pub struct Vigour {
  calm: f32,
  mending: f32,
  swung: f32
}

pub fn hurt(health: &mut Health, vigour: &mut Vigour, amount: u8) {
  health.0 = health.0.saturating_sub(amount);
  vigour.calm = 0.0
}

#[derive(Component)]
pub struct Raider {
  pub ship: Entity,
  velocity: Vec3,
  grounded: bool,
  reload: f32,
  idle: f32,
  stagger: f32,
  stuck: f32,
  detour: f32
}

impl Raider {
  pub fn overboard(ship: Entity, toward: Vec3) -> Raider {
    Raider {
      ship,
      velocity: toward * 4.0 + Vec3::Y * 6.0,
      grounded: false,
      reload: 1.0,
      idle: 0.0,
      stagger: 0.6,
      stuck: 0.0,
      detour: 0.0
    }
  }
}

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn toward(offset: Vec2) -> f32 { (-offset.x).atan2(-offset.y) }

fn raid(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut raiders: Query<(Entity, &mut Folk, &mut Raider)>,
  mut players: Query<(&Avatar, &mut Health, &mut Vigour), With<Controller>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  raiders.iter_mut().for_each(|(entity, mut folk, mut raider)| {
    let Folk { band, mut at, mut yaw, .. } = *folk;
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
    let swimming =
      voxels.block((at + Vec3::Y * 1.2).floor().as_ivec3()).is_some_and(Block::fluid);
    let target = players
      .iter_mut()
      .map(|(avatar, health, vigour)| (avatar.at, health, vigour))
      .filter(|(spot, ..)| spot.distance(at) < CHASE)
      .min_by(|a, b| a.0.distance(at).total_cmp(&b.0.distance(at)));
    raider.reload -= dt;
    raider.stagger -= dt;
    raider.detour -= dt;
    let side = if folk.luck % 2 == 0 { 1.5 } else { -1.5 };
    let wish = match target {
      Some((spot, mut health, mut vigour)) => {
        raider.idle = 0.0;
        let offset = spot.xz() - at.xz();
        yaw = toward(offset) + if raider.detour > 0.0 { side } else { 0.0 };
        let close = offset.length() < ARM_REACH && (spot.y - at.y).abs() < 2.0;
        if close && raider.reload <= 0.0 && raider.stagger <= 0.0 {
          raider.reload = 1.1;
          folk.blows = folk.blows.wrapping_add(1);
          hurt(&mut health, &mut vigour, band.blow())
        }
        match close {
          true => Vec3::ZERO,
          false => ahead(yaw) * band.speed() * if swimming { 0.6 } else { 1.0 }
        }
      }
      None => {
        raider.idle += dt;
        Vec3::ZERO
      }
    };
    let horizontal = match raider.stagger > 0.0 {
      true => raider.velocity * (1.0 - dt * 3.0),
      false => wish
    };
    let vertical = match swimming && raider.velocity.y <= 2.5 {
      true => (raider.velocity.y + 18.0 * dt).min(2.5),
      false => (raider.velocity.y - GRAVITY * dt).max(-40.0)
    };
    let Marched { at: moved, velocity, blocked, landed } = march(
      &voxels,
      Bulk::PERSON,
      at,
      Vec3::new(horizontal.x, vertical, horizontal.z),
      dt
    );
    raider.velocity = match blocked && (landed || swimming) && raider.stagger <= 0.0 {
      true => velocity.with_y(SCRAMBLE),
      false => velocity
    };
    raider.grounded = landed;
    let progress = (moved - at).xz().length() / dt.max(1e-4);
    raider.stuck = match wish != Vec3::ZERO && raider.stagger <= 0.0 && progress < 0.4 {
      true => raider.stuck + dt,
      false => (raider.stuck - dt).max(0.0)
    };
    if raider.stuck > 1.2 {
      raider.stuck = 0.0;
      raider.detour = 2.0
    }
    at = moved;
    match raider.idle > GIVE_UP || at.y < -16.0 {
      true => commands.entity(entity).despawn(),
      false => {
        let moved = Folk { at, yaw: yaw.rem_euclid(TAU), ..*folk };
        folk.set_if_neq(moved);
      }
    }
  })
}

fn strike(
  mut strikes: MessageReader<FromClient<Strike>>,
  time: Res<Time>,
  mut players: Query<(&Controller, (&Avatar, &mut Vigour))>,
  mut folk: Query<(&Folk, &mut Health, Option<&mut Raider>)>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  strikes.read().for_each(|&FromClient { client_id, message: Strike(target) }| {
    if let Some((avatar, mut vigour)) = player_of(players.iter_mut(), client_id)
      && now - vigour.swung >= STRIKE_EVERY
      && let Ok((folk, mut health, raider)) = folk.get_mut(target)
      && folk.band.hostile()
      && (avatar.at + Vec3::Y * EYE).distance(folk.at + Vec3::Y * 0.9) <= REACH + 1.0
    {
      vigour.swung = now;
      health.0 = health.0.saturating_sub(PUNCH);
      if let Some(mut raider) = raider {
        let push = (folk.at - avatar.at).with_y(0.0).normalize_or_zero();
        raider.velocity = push * 7.0 + Vec3::Y * 5.0;
        raider.stagger = 0.35
      }
      if health.0 == 0 {
        commands.entity(target).despawn()
      }
    }
  })
}

fn mend(time: Res<Time>, mut players: Query<(&mut Health, &mut Vigour)>) {
  let dt = time.delta_secs();
  players.iter_mut().for_each(|(mut health, mut vigour)| {
    vigour.calm += dt;
    vigour.mending += dt;
    if vigour.calm > MEND_AFTER && vigour.mending >= MEND_EVERY && health.0 < Health::FULL
    {
      vigour.mending = 0.0;
      health.0 += 1
    }
  })
}

fn perish(
  mut voxels: ResMut<Voxels>,
  mut players: Query<(
    Entity,
    &Controller,
    &mut Health,
    &mut Avatar,
    &mut Vigour,
    &Bedside
  )>,
  mut boats: Query<&mut Vessel>,
  mut teleports: MessageWriter<ToClients<Teleport>>
) {
  players.iter_mut().filter(|(_, _, health, ..)| health.0 == 0).for_each(
    |(entity, controller, mut health, mut avatar, mut vigour, bedside)| {
      health.0 = Health::FULL;
      *vigour = Vigour::default();
      boats
        .iter_mut()
        .filter(|vessel| vessel.rider == Some(entity))
        .for_each(|mut vessel| vessel.rider = None);
      let bed = bedside
        .0
        .filter(|&at| voxels.ensure(at) == Block::Bed)
        .map(|at| at.as_vec3() + Vec3::new(0.5, 0.05, 0.5));
      let revived = Avatar {
        at: bed.unwrap_or(generate::spawn_point(voxels.seed)),
        yaw: FACING_STONE,
        pitch: 0.0
      };
      *avatar = revived;
      teleports.write(ToClients {
        targets: SendTargets::Single(controller.client),
        message: Teleport(revived)
      });
    }
  )
}

fn repaint(skin: &mut Skin, paint: impl Fn(Part, Face, u32, u32, u8) -> u8) {
  Part::ALL.iter().for_each(|&part| {
    part.faces().into_iter().for_each(|(face, rect)| {
      (rect.min.y..rect.max.y).for_each(|y| {
        (rect.min.x..rect.max.x).for_each(|x| {
          let texel = UVec2::new(x, y);
          let painted =
            paint(part, face, y - rect.min.y, x - rect.min.x, skin.get(texel));
          skin.set(texel, painted)
        })
      })
    })
  })
}

fn outfit(band: Band, luck: u32) -> Skin {
  let mut skin = Skin::fresh(u64::from(luck));
  let pick = |salt: u32, choices: &[u8]| {
    choices[(crate::noise::hash(luck, salt as i32, 3, 5) % choices.len() as u32) as usize]
  };
  let shirt = match band {
    Band::Pirate => pick(1, &[14, 15, 4, 31]),
    Band::Viking => pick(1, &[8, 9, 19, 24, 28]),
    Band::Trader => pick(1, &[17, 18, 23, 20])
  };
  let trim = match band {
    Band::Pirate => pick(2, &[0, 30, 19]),
    Band::Viking => pick(2, &[1, 10, 2]),
    Band::Trader => 11
  };
  let beard = pick(3, &[11, 12, 13, 10, 0]);
  let striped = pick(4, &[0, 1]) == 1;
  let patched = pick(5, &[0, 0, 1]) == 1;
  repaint(&mut skin, |part, face, row, col, was| match (band, part, face) {
    (Band::Pirate, Part::Head, Face::Front) if patched && row == 4 && col == 2 => 0,
    (Band::Pirate, Part::Head, Face::Top) => 14,
    (Band::Pirate, Part::Head, _) if row < 2 && face != Face::Bottom => 14,
    (Band::Pirate | Band::Viking, Part::Head, Face::Front)
      if row >= 5 && (1..7).contains(&col) =>
    {
      match band == Band::Viking || row == 7 {
        true => beard,
        false => was
      }
    }
    (Band::Viking, Part::Head, Face::Left | Face::Right) if row >= 5 => beard,
    (_, Part::Body, Face::Top | Face::Bottom) => shirt,
    (Band::Pirate, Part::Body, _) if striped && row < 10 => [4, shirt][row as usize % 2],
    (Band::Trader, Part::Body, _) if row == 7 => trim,
    (_, Part::Body, _) if row == 10 => trim,
    (Band::Trader, Part::Body, _) => shirt,
    (_, Part::Body, _) if row < 11 => shirt,
    (_, Part::Arm, Face::Top) => shirt,
    (_, Part::Arm, _) if row < 6 => shirt,
    (Band::Trader, Part::Leg, _) if row < 10 => shirt,
    (_, Part::Leg, _) if row < 10 => trim,
    _ => was
  });
  skin
}

type Gear = Vec<(Vec3, Vec3, [f32; 3])>;

const IRON: [f32; 3] = [0.55, 0.56, 0.6];
const STEEL: [f32; 3] = [0.8, 0.82, 0.86];
const GOLD: [f32; 3] = [0.86, 0.68, 0.2];
const SOOT: [f32; 3] = [0.08, 0.07, 0.07];
const HAFT: [f32; 3] = [0.45, 0.3, 0.16];
const BONE: [f32; 3] = [0.92, 0.88, 0.75];

fn hat(band: Band, luck: u32) -> Gear {
  let at = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * PX;
  match band {
    Band::Pirate if luck % 2 == 0 => vec![
      (at(0.0, 8.5, 0.0), at(11.0, 1.0, 11.0), SOOT),
      (at(0.0, 10.0, 0.0), at(8.4, 3.0, 8.4), SOOT),
      (at(0.0, 10.0, -4.3), at(2.0, 2.0, 0.3), BONE),
    ],
    Band::Pirate => vec![],
    Band::Viking => vec![
      (at(0.0, 6.6, 0.0), at(9.0, 3.6, 9.0), IRON),
      (at(0.0, 4.0, -4.6), at(1.0, 3.0, 0.5), IRON),
      (at(5.4, 7.6, 0.0), at(2.0, 2.0, 2.0), BONE),
      (at(-5.4, 7.6, 0.0), at(2.0, 2.0, 2.0), BONE),
      (at(6.4, 9.8, 0.0), at(1.5, 3.0, 1.5), BONE),
      (at(-6.4, 9.8, 0.0), at(1.5, 3.0, 1.5), BONE),
    ],
    Band::Trader => vec![
      (at(0.0, 8.6, 0.0), at(9.0, 3.0, 9.0), [0.94, 0.9, 0.8]),
      (at(0.0, 8.8, -4.6), at(1.5, 1.5, 0.4), [0.2, 0.8, 0.5]),
    ]
  }
}

fn weapon(band: Band) -> Gear {
  let at = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * PX;
  match band {
    Band::Pirate => vec![
      (at(0.0, -11.0, -6.5), at(0.8, 1.6, 10.0), STEEL),
      (at(0.0, -11.0, -1.2), at(3.0, 2.0, 0.8), GOLD),
    ],
    Band::Viking => vec![
      (at(0.0, -11.0, -5.0), at(1.0, 1.0, 12.0), HAFT),
      (at(0.0, -9.5, -10.0), at(0.8, 5.0, 3.0), IRON),
    ],
    Band::Trader => vec![]
  }
}

fn shield(band: Band, luck: u32) -> Gear {
  let paint =
    [[0.7, 0.15, 0.12], [0.85, 0.7, 0.2], [0.18, 0.3, 0.6]][(luck % 3) as usize];
  match band {
    Band::Viking => vec![
      (Vec3::new(-2.5, -6.0, 0.0) * PX, Vec3::new(0.8, 9.0, 9.0) * PX, paint),
      (Vec3::new(-3.0, -6.0, 0.0) * PX, Vec3::new(0.6, 2.0, 2.0) * PX, IRON),
    ],
    _ => vec![]
  }
}

#[derive(Component)]
struct Shown {
  at: Vec3,
  yaw: f32,
  stride: f32,
  pace: f32,
  blows: u8,
  swung: f32,
  health: u8,
  flushed: f32
}

fn dress(
  kit: Res<Kit>,
  arrivals: Query<(Entity, &Folk), Without<Shown>>,
  mut shapes: ResMut<Shapes>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  arrivals.iter().for_each(|(entity, folk)| {
    let clad = clothe(&outfit(folk.band, folk.luck), &mut images, &mut materials);
    let material = MeshMaterial3d(clad.material.clone());
    let mut gear = |parent: Entity, gear: Gear, commands: &mut Commands| {
      gear.into_iter().for_each(|(centre, size, color)| {
        commands.spawn((
          Mesh3d(shapes.cuboid(&mut meshes, size)),
          MeshMaterial3d(shapes.paint(&mut materials, &mut images, color, Grain::Plain)),
          Transform::from_translation(centre),
          ChildOf(parent)
        ));
      })
    };
    commands.entity(entity).insert((
      Shown {
        at: folk.at,
        yaw: folk.yaw,
        stride: 0.0,
        pace: 0.0,
        blows: folk.blows,
        swung: SWING,
        health: folk.band.health(),
        flushed: 0.0
      },
      Transform::from_translation(folk.at),
      Visibility::default()
    ));
    commands.spawn((
      Mesh3d(kit.body.clone()),
      material.clone(),
      Transform::from_xyz(0.0, 18.0 * PX, 0.0),
      ChildOf(entity)
    ));
    let head = commands
      .spawn((
        Joint::Neck,
        Rest(Vec3::Y * 24.0 * PX),
        Mesh3d(kit.head.clone()),
        material.clone(),
        Transform::from_xyz(0.0, 24.0 * PX, 0.0),
        ChildOf(entity)
      ))
      .id();
    gear(head, hat(folk.band, folk.luck), &mut commands);
    [
      (6.0, 0.0, kit.arm.clone(), 24.0, true),
      (-6.0, PI, kit.arm.clone(), 24.0, true),
      (2.0, PI, kit.leg.clone(), 12.0, false),
      (-2.0, 0.0, kit.leg.clone(), 12.0, false)
    ]
    .into_iter()
    .for_each(|(x, phase, mesh, pivot, arm)| {
      let rest = Vec3::new(x, pivot, 0.0) * PX;
      let limb = commands
        .spawn((
          if arm { Joint::Arm(phase) } else { Joint::Leg(phase) },
          Rest(rest),
          Mesh3d(mesh),
          material.clone(),
          Transform::from_translation(rest),
          ChildOf(entity)
        ))
        .id();
      match (arm, x > 0.0) {
        (true, true) => gear(limb, weapon(folk.band), &mut commands),
        (true, false) => gear(limb, shield(folk.band, folk.luck), &mut commands),
        _ => ()
      }
    });
    commands.entity(entity).insert(clad);
  })
}

fn animate(
  time: Res<Time>,
  mut folk: Query<(Entity, &Folk, &Health, &Clad, &mut Shown, &mut Transform)>,
  family: Query<&Children>,
  mut joints: Query<(&Joint, &Rest, &mut Transform), Without<Shown>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let (dt, now) = (time.delta_secs(), time.elapsed_secs());
  folk.iter_mut().for_each(|(entity, folk, health, clad, mut shown, mut transform)| {
    let before = shown.at;
    shown.at = before.lerp(folk.at, (dt * 12.0).min(1.0));
    let turn = (folk.yaw - shown.yaw + PI).rem_euclid(TAU) - PI;
    shown.yaw += turn * (dt * 10.0).min(1.0);
    let pace = match folk.aboard {
      true => 0.0,
      false => (shown.at - before).xz().length() / dt.max(1e-4)
    };
    shown.pace += (pace - shown.pace) * (dt * 6.0).min(1.0);
    shown.stride += shown.pace * dt * 2.2;
    if shown.blows != folk.blows {
      shown.blows = folk.blows;
      shown.swung = 0.0
    }
    shown.swung += dt;
    if health.0 < shown.health {
      shown.flushed = 0.3
    }
    shown.health = health.0;
    shown.flushed -= dt;
    if let Some(mut material) = materials.get_mut(&clad.material) {
      material.base_color = match shown.flushed > 0.0 {
        true => Color::srgb(1.0, 0.35, 0.35),
        false => Color::WHITE
      }
    }
    let swing = shown.stride.sin() * (shown.pace / 4.3).min(1.0) * 0.7;
    let chop = (shown.swung / SWING).min(1.0);
    let brandish = match (folk.aboard, folk.band.hostile()) {
      (true, true) => Some(2.3 + (now * 3.0 + folk.luck as f32).sin() * 0.3),
      _ => None
    };
    *transform = Transform::from_translation(shown.at)
      .with_rotation(Quat::from_rotation_y(shown.yaw));
    family.iter_descendants(entity).for_each(|child| {
      if let Ok((joint, rest, mut transform)) = joints.get_mut(child) {
        transform.translation = rest.0;
        transform.rotation = match *joint {
          Joint::Arm(phase) if phase == 0.0 && chop < 1.0 => {
            Quat::from_rotation_x((chop * PI).sin() * 2.6)
          }
          Joint::Arm(phase)
            if phase == 0.0
              && let Some(raised) = brandish =>
          {
            Quat::from_rotation_x(raised)
          }
          Joint::Arm(phase) | Joint::Leg(phase) => {
            Quat::from_rotation_x(swing * phase.cos())
          }
          _ => Quat::IDENTITY
        }
      }
    })
  })
}

fn fight(
  buttons: Res<ButtonInput<MouseButton>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  folk: Query<(Entity, &Folk)>,
  mut strikes: MessageWriter<Strike>
) {
  if let Some(voxels) = voxels
    && captured(&cursor, &menu)
    && buttons.just_pressed(MouseButton::Left)
  {
    let (from, toward) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
    let blocked = voxels
      .cast(from, toward, REACH)
      .map_or(REACH, |hit| (hit.at.as_vec3() + 0.5).distance(from));
    if let Some((target, _)) = folk
      .iter()
      .filter(|(_, folk)| folk.band.hostile())
      .filter_map(|(entity, folk)| {
        struck(folk.at, from, toward).map(|near| (entity, near))
      })
      .filter(|&(_, near)| near <= blocked)
      .min_by(|a, b| a.1.total_cmp(&b.1))
    {
      strikes.write(Strike(target));
    }
  }
}

fn revive(mut teleports: MessageReader<Teleport>, pilot: Option<ResMut<Pilot>>) {
  if teleports.read().count() > 0
    && let Some(mut pilot) = pilot
  {
    pilot.riding = None
  }
}

pub struct People;

impl Plugin for People {
  fn build(&self, app: &mut App) {
    app
      .add_systems(PreUpdate, strike.after(ServerSystems::Receive).run_if(authority))
      .add_systems(
        Update,
        (raid, mend, perish).chain().run_if(authority).run_if(resource_exists::<Voxels>)
      )
      .add_systems(
        Update,
        (dress, animate, fight.run_if(resource_exists::<Pilot>), revive)
          .chain()
          .run_if(plays)
      );
  }
}
