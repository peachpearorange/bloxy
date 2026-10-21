use {crate::{authority::{Controller, player_of},
             beast::{Joint, Rest, Shapes},
             block::Block,
             cage::Cage,
             figure::{Clad, Kit, clothe},
             generate,
             island::FACING_STONE,
             menu::Menu,
             player::{Bulk, Marched, Pilot, captured, march},
             protocol::*,
             sculpt::{Daub, merged, sculpt},
             skin::{Face, PX, Part, Skin},
             voxels::Voxels},
     bevy::{camera::visibility::DynamicSkinnedMeshBounds,
            mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
            platform::collections::HashMap,
            prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     std::f32::consts::{PI, TAU}};

const CHASE: f32 = 64.0;
const GIVE_UP: f32 = 40.0;
const MEND_AFTER: f32 = 8.0;
const MEND_EVERY: f32 = 3.0;
pub const PUNCH: u8 = 3;
const ARM_REACH: f32 = 1.4;
const GRAVITY: f32 = 30.0;
const SCRAMBLE: f32 = 11.5;
const SWING: f32 = 0.4;
pub const KNOCK: f32 = 8.0;

impl Band {
  pub fn health(self) -> u8 {
    match self {
      Band::Trader => Health::FULL,
      Band::Pirate => 9,
      Band::Viking => 12,
      Band::Skeleton => 10,
      Band::Shade => 8,
      Band::Warden => 60,
      Band::Thrall => 12,
      Band::Phantom => 10,
      Band::Imp => 7
    }
  }

  pub fn speed(self) -> f32 {
    match self {
      Band::Viking => 3.8,
      Band::Skeleton => 3.3,
      Band::Shade => 2.8,
      Band::Warden => 2.9,
      Band::Thrall => 3.9,
      Band::Phantom => 3.6,
      Band::Imp => 4.4,
      _ => 3.4
    }
  }

  pub fn blow(self) -> u8 {
    match self {
      Band::Viking => 3,
      Band::Shade | Band::Imp => 1,
      Band::Warden => 5,
      Band::Thrall => 3,
      _ => 2
    }
  }

  pub fn ghostly(self) -> bool { matches!(self, Band::Shade | Band::Phantom) }

  pub fn flies(self) -> bool { self.ghostly() || self == Band::Imp }

  pub fn sculpted(self) -> bool {
    matches!(self, Band::Skeleton | Band::Warden | Band::Thrall | Band::Imp)
  }

  pub fn scale(self) -> f32 {
    match self {
      Band::Warden => 1.6,
      Band::Imp => crate::imp::SCALE,
      _ => 1.0
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

pub fn struck(folk: &Folk, from: Vec3, toward: Vec3) -> Option<f32> {
  let scale = folk.band.scale();
  let (at, half) = (folk.at, 0.35 * scale);
  ray_box(
    from,
    toward,
    at - Vec3::new(half, 0.0, half),
    at + Vec3::new(half, 1.9 * scale, half)
  )
}

#[derive(Component)]
pub struct Vigour {
  calm: f32,
  mending: f32,
  pub swung: f32,
  spare: f32,
  pub most: u8,
  pub ward: f32,
  pub punch: u8,
  pub mend: f32,
  pub quick: f32,
  pub resist: [f32; 3]
}

impl Default for Vigour {
  fn default() -> Self {
    Vigour {
      calm: 0.0,
      mending: 0.0,
      swung: 0.0,
      spare: 0.0,
      most: Health::FULL,
      ward: 1.0,
      punch: PUNCH,
      mend: 1.0,
      quick: crate::stance::STRIKE_EVERY,
      resist: [1.0; 3]
    }
  }
}

pub fn hurt(health: &mut Health, vigour: &mut Vigour, amount: u8) {
  harm(health, vigour, f32::from(amount))
}

pub fn harm(health: &mut Health, vigour: &mut Vigour, amount: f32) {
  let felt = amount * vigour.ward + vigour.spare;
  vigour.spare = felt.fract();
  health.0 = health.0.saturating_sub(felt.floor() as u8);
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
  pub fn knock(&mut self, push: Vec3) {
    self.velocity = push;
    self.stagger = 0.4
  }

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
  mut players: Query<(
    &Controller,
    &Avatar,
    &mut Health,
    &mut Vigour,
    &crate::condition::Conditions
  )>,
  mut knocks: MessageWriter<ToClients<Knock>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  for (entity, mut folk, mut raider) in raiders.iter_mut() {
    let Folk { band, mut at, mut yaw, .. } = *folk;
    for offset in [
      IVec3::ZERO,
      IVec3::X * 2,
      IVec3::X * -2,
      IVec3::Z * 2,
      IVec3::Z * -2,
      IVec3::Y * -2
    ]
    .into_iter()
    {
      voxels.ensure(at.floor().as_ivec3() + offset);
    }
    let swimming =
      voxels.block((at + Vec3::Y * 1.2).floor().as_ivec3()).is_some_and(Block::fluid);
    let target = players
      .iter_mut()
      .filter(|(.., conditions)| !conditions.has(crate::condition::Kind::Invisible))
      .map(|(controller, avatar, health, vigour, _)| {
        (controller, avatar.at, health, vigour)
      })
      .filter(|(_, spot, ..)| spot.distance(at) < CHASE)
      .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)));
    raider.reload -= dt;
    raider.stagger -= dt;
    raider.detour -= dt;
    let side = if folk.luck % 2 == 0 { 1.5 } else { -1.5 };
    let wish = match target {
      Some((controller, spot, mut health, mut vigour)) => {
        raider.idle = 0.0;
        let offset = spot.xz() - at.xz();
        yaw = toward(offset) + if raider.detour > 0.0 { side } else { 0.0 };
        let close = offset.length() < ARM_REACH && (spot.y - at.y).abs() < 2.0;
        if close && raider.reload <= 0.0 && raider.stagger <= 0.0 {
          raider.reload = 1.1;
          folk.blows = folk.blows.wrapping_add(1);
          hurt(&mut health, &mut vigour, band.blow());
          knocks.write(ToClients {
            targets: SendTargets::Single(controller.client),
            message: Knock(
              offset.normalize_or_zero().extend(0.0).xzy() * KNOCK + Vec3::Y * 5.0
            )
          });
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
  }
}

fn strike(
  mut strikes: MessageReader<FromClient<Strike>>,
  time: Res<Time>,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Vigour))>,
  mut folk: Query<(
    &Folk,
    &mut Health,
    Option<&mut Raider>,
    Option<&mut crate::haunt::Haunt>
  )>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  for &FromClient { client_id, message: Strike(target) } in strikes.read() {
    if let Some((player, avatar, mut vigour)) = player_of(players.iter_mut(), client_id)
      && now - vigour.swung >= vigour.quick
      && let Ok((folk, mut health, raider, haunt)) = folk.get_mut(target)
      && folk.band.hostile()
      && (avatar.at + Vec3::Y * EYE).distance(folk.at + Vec3::Y * 0.9 * folk.band.scale())
        <= REACH + folk.band.scale()
      && health.0 > 0
    {
      vigour.swung = now;
      health.0 = health.0.saturating_sub(vigour.punch);
      commands.entity(target).insert(crate::haunt::Blamed(player));
      let push =
        (folk.at - avatar.at).with_y(0.0).normalize_or_zero() / folk.band.scale();
      if let Some(mut raider) = raider {
        raider.velocity = push * 7.0 + Vec3::Y * 5.0;
        raider.stagger = 0.35
      }
      if let Some(mut haunt) = haunt {
        haunt.shove(push * 7.0 + Vec3::Y * 4.0)
      }
      if health.0 == 0 {
        crate::haunt::fell(&mut commands, target)
      }
    }
  }
}

fn mend(time: Res<Time>, mut players: Query<(&mut Health, &mut Vigour)>) {
  let dt = time.delta_secs();
  for (mut health, mut vigour) in players.iter_mut() {
    vigour.calm += dt;
    vigour.mending += dt;
    if vigour.calm > MEND_AFTER
      && vigour.mending >= MEND_EVERY / vigour.mend
      && health.0 < vigour.most
    {
      vigour.mending = 0.0;
      health.0 += 1
    }
  }
}

fn perish(
  mut voxels: ResMut<Voxels>,
  mut players: Query<(
    Entity,
    &Controller,
    &mut Health,
    &mut Avatar,
    &mut Vigour,
    &Bedside,
    &mut crate::condition::Conditions
  )>,
  mut boats: Query<&mut Vessel>,
  mut balloons: Query<&mut crate::balloon::Aerostat>,
  mut teleports: MessageWriter<ToClients<Teleport>>
) {
  for (entity, controller, mut health, mut avatar, mut vigour, bedside, mut conditions) in
    players.iter_mut().filter(|(_, _, health, ..)| health.0 == 0)
  {
    health.0 = vigour.most;
    conditions.0.clear();
    *vigour = Vigour {
      most: vigour.most,
      ward: vigour.ward,
      punch: vigour.punch,
      mend: vigour.mend,
      quick: vigour.quick,
      resist: vigour.resist,
      ..default()
    };
    for mut vessel in boats.iter_mut().filter(|vessel| vessel.rider == Some(entity)) {
      vessel.rider = None
    }
    for mut aerostat in
      balloons.iter_mut().filter(|aerostat| aerostat.rider == Some(entity))
    {
      aerostat.rider = None;
      aerostat.burning = false
    }
    let bed = bedside
      .0
      .filter(|&at| voxels.ensure(at).bed())
      .map(|at| at.as_vec3() + Vec3::new(0.5, 0.05, 0.5));
    let revived = Avatar {
      at: bed.unwrap_or(generate::spawn_point(voxels.seed)),
      yaw: FACING_STONE,
      pitch: 0.0,
      held: avatar.held,
      charging: false,
      hand: avatar.hand
    };
    *avatar = revived;
    teleports.write(ToClients {
      targets: SendTargets::Single(controller.client),
      message: Teleport(revived)
    });
  }
}

fn repaint(skin: &mut Skin, paint: impl Fn(Part, Face, u32, u32, u8) -> u8) {
  for &part in Part::ALL.iter() {
    for (face, rect) in part.faces().into_iter() {
      for y in rect.min.y..rect.max.y {
        for x in rect.min.x..rect.max.x {
          let texel = UVec2::new(x, y);
          let painted =
            paint(part.limb(), face, y - rect.min.y, x - rect.min.x, skin.get(texel));
          skin.set(texel, painted)
        }
      }
    }
  }
}

fn outfit(band: Band, luck: u32) -> Skin {
  let mut skin = Skin::fresh(u64::from(luck));
  match band {
    Band::Shade | Band::Phantom => {
      let (robe, fleck) = if band == Band::Phantom { (19, 21) } else { (0, 18) };
      repaint(&mut skin, |part, face, row, col, _| match (part, face) {
        (Part::Head, Face::Front) if row == 4 && (col == 2 || col == 5) => 22,
        (Part::Head, Face::Front) if (2..7).contains(&row) && (1..7).contains(&col) => 0,
        _ if (row + col) % 5 == 0 => fleck,
        _ => robe
      })
    }
    _ => clothes(&mut skin, band, luck)
  }
  skin
}

fn clothes(skin: &mut Skin, band: Band, luck: u32) {
  let pick = |salt: u32, choices: &[u8]| {
    choices[(crate::noise::hash(luck, salt as i32, 3, 5) % choices.len() as u32) as usize]
  };
  let shirt = match band {
    Band::Pirate => pick(1, &[14, 15, 4, 31]),
    Band::Viking => pick(1, &[8, 9, 19, 24, 28]),
    _ => pick(1, &[17, 18, 23, 20])
  };
  let trim = match band {
    Band::Pirate => pick(2, &[0, 30, 19]),
    Band::Viking => pick(2, &[1, 10, 2]),
    _ => 11
  };
  let beard = pick(3, &[11, 12, 13, 10, 0]);
  let striped = pick(4, &[0, 1]) == 1;
  let patched = pick(5, &[0, 0, 1]) == 1;
  repaint(skin, |part, face, row, col, was| match (band, part, face) {
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
  })
}

type Gear = Vec<(Vec3, Vec3, [f32; 3])>;

const IRON: [f32; 3] = [0.55, 0.56, 0.6];
const STEEL: [f32; 3] = [0.8, 0.82, 0.86];
const GOLD: [f32; 3] = [0.86, 0.68, 0.2];
const SOOT: [f32; 3] = [0.08, 0.07, 0.07];
const HAFT: [f32; 3] = [0.45, 0.3, 0.16];
const BONE: [f32; 3] = [0.92, 0.88, 0.75];
const GHOSTLY: f32 = 0.72;
const RUST: [f32; 3] = [0.45, 0.28, 0.18];
const SHROUD: [f32; 3] = [0.06, 0.04, 0.09];

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
    ],
    Band::Skeleton | Band::Thrall | Band::Warden | Band::Imp => vec![],
    Band::Phantom => vec![
      (at(0.0, 4.5, 0.6), at(9.2, 9.4, 9.0), [0.1, 0.16, 0.4]),
      (at(0.0, 9.6, 1.5), at(7.0, 1.4, 7.0), [0.1, 0.16, 0.4]),
    ],
    Band::Shade => vec![
      (at(0.0, 4.5, 0.6), at(9.2, 9.4, 9.0), SHROUD),
      (at(0.0, 9.6, 1.5), at(7.0, 1.4, 7.0), SHROUD),
    ]
  }
}

fn eyes(band: Band) -> Gear {
  let at = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * PX;
  match band {
    Band::Shade => vec![
      (at(1.5, 4.5, -4.05), at(1.2, 1.0, 0.2), [0.5, 1.0, 1.0]),
      (at(-1.5, 4.5, -4.05), at(1.2, 1.0, 0.2), [0.5, 1.0, 1.0]),
    ],
    Band::Phantom => vec![
      (at(1.5, 4.5, -4.05), at(1.2, 1.0, 0.2), [0.9, 0.95, 1.0]),
      (at(-1.5, 4.5, -4.05), at(1.2, 1.0, 0.2), [0.9, 0.95, 1.0]),
    ],
    Band::Skeleton | Band::Warden | Band::Thrall => {
      let glow = match band {
        Band::Thrall => [0.4, 1.0, 0.35],
        _ => [1.0, 0.35, 0.1]
      };
      [1.0, -1.0]
        .map(|side| {
          (
            SOCKET * Vec3::new(side, 1.0, 1.0) - SKULL_PIVOT - Vec3::Z * 0.006,
            Vec3::new(0.03, 0.028, 0.01),
            glow
          )
        })
        .to_vec()
    }
    Band::Imp => [1.0, -1.0]
      .map(|side| {
        let [r, g, b] = crate::imp::GLARE;
        (
          crate::imp::SOCKET * Vec3::new(side, 1.0, 1.0) - crate::imp::HEAD
            + Vec3::new(0.0, 0.0, -0.012),
          Vec3::new(0.05, 0.03, 0.012),
          [r, g, b]
        )
      })
      .to_vec(),
    _ => vec![]
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
    _ => vec![]
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

const SKULL: u16 = 1;
const LIMBS: [(u16, f32, bool); 4] =
  [(2, 1.0, true), (3, -1.0, true), (4, 1.0, false), (5, -1.0, false)];
const SKULL_PIVOT: Vec3 = Vec3::new(0.0, 24.0 * PX, 0.0);
const SHOULDER: f32 = 0.28;
const IRONRUST: [f32; 3] = [0.33, 0.29, 0.27];
const SOCKET: Vec3 = Vec3::new(0.07, 1.618, -0.139);
const MARROW: [f32; 3] = [0.7, 0.66, 0.55];
const HOLLOW: [f32; 3] = [0.07, 0.05, 0.05];

fn pivot(joint: u16) -> Vec3 {
  LIMBS.iter().find(|&&(limb, _, _)| limb == joint).map_or(
    match joint {
      SKULL => SKULL_PIVOT,
      _ => Vec3::ZERO
    },
    |&(_, side, arm)| match arm {
      true => Vec3::new(side * SHOULDER, 24.0 * PX, 0.0),
      false => Vec3::new(side * 2.0, 12.0, 0.0) * PX
    }
  )
}

fn long_bone(from: Vec3, to: Vec3, thick: f32) -> Cage {
  Cage::tube(
    &[(0.0, 1.25), (0.5, 0.75), (1.0, 1.2)]
      .map(|(t, girth)| (from.lerp(to, t), Vec2::splat(thick * girth))),
    4
  )
}

fn rod(from: Vec3, to: Vec3, thick: f32, sides: u32) -> Cage {
  Cage::tube(&[(from, Vec2::splat(thick)), (to, Vec2::splat(thick))], sides)
}

fn ribs(side: f32) -> Vec<Cage> {
  [(1.32, 0.155), (1.25, 0.19), (1.18, 0.195), (1.11, 0.17)]
    .into_iter()
    .map(|(y, wide)| {
      let deep = wide * 0.68;
      let path: Vec<(Vec3, Vec2)> = (0..4)
        .map(|step| {
          let turn = 0.35 + step as f32 / 3.0 * 2.3;
          let flare = 1.0 + (step as f32 / 3.0).powi(3) * 0.18;
          (
            Vec3::new(
              side * wide * turn.sin() * flare,
              y - turn / PI * 0.06,
              deep * turn.cos() * flare + 0.02
            ),
            Vec2::new(0.018, 0.028)
          )
        })
        .collect();
      Cage::tube(&path, 3)
    })
    .collect()
}

fn skull(band: Band, luck: u32) -> Mesh {
  let cranium = Cage::rounded(
    Vec3::new(0.0, 1.63, 0.0),
    Vec3::new(0.29, 0.27, 0.31),
    UVec3::splat(3),
    0.55
  )
  .shaped(|at| {
    let cheek = ((1.6 - at.y) / 0.1).clamp(0.0, 1.0);
    at * Vec3::new(1.0 - cheek * 0.18, 1.0, 1.0)
      + Vec3::Z * (cheek * 0.03 * (-at.z).max(0.0) / 0.15)
  });
  let jaw = Cage::rounded(
    Vec3::new(0.0, 1.475, -0.06),
    Vec3::new(0.19, 0.06, 0.14),
    UVec3::new(2, 1, 2),
    0.4
  );
  let mouth = Cage::rounded(
    Vec3::new(0.0, 1.508, -0.07),
    Vec3::new(0.15, 0.03, 0.12),
    UVec3::ONE,
    0.0
  );
  let teeth = [-0.05, -0.017, 0.017, 0.05].map(|x| {
    Cage::tube(
      &[
        (Vec3::new(x, 1.497, -0.128), Vec2::splat(0.012)),
        (Vec3::new(x, 1.527, -0.126), Vec2::splat(0.01))
      ],
      3
    )
  });
  let socket = |side: f32| Daub {
    at: SOCKET * Vec3::new(side, 1.0, 1.0) + Vec3::new(0.0, 0.0, -0.05),
    color: HOLLOW
  };
  let face = sculpt(
    &cranium,
    |_, _| BONE,
    &[socket(1.0), socket(-1.0), Daub { at: Vec3::new(0.0, 1.57, -0.22), color: HOLLOW }],
    |_| SKULL
  );
  let plain = |cage: &Cage, color: [f32; 3]| sculpt(cage, |_, _| color, &[], |_| SKULL);
  let dome = |size: Vec3, rim: f32| {
    Cage::rounded(Vec3::new(0.0, rim + 0.05, 0.005), size, UVec3::new(2, 1, 2), 0.6)
      .shaped(move |at| at.with_y(at.y.max(rim)))
  };
  let helm: Vec<Mesh> = match band {
    Band::Warden => {
      let horn = |side: f32| {
        Cage::tube(
          &[
            (Vec3::new(side * 0.15, 1.73, 0.0), Vec2::splat(0.045)),
            (Vec3::new(side * 0.3, 1.8, -0.02), Vec2::splat(0.035)),
            (Vec3::new(side * 0.35, 1.95, -0.06), Vec2::splat(0.02)),
            (Vec3::new(side * 0.32, 2.05, -0.12), Vec2::splat(0.004))
          ],
          4
        )
      };
      let crest = Cage::tube(
        &[
          (Vec3::new(0.0, 1.8, -0.19), Vec2::new(0.014, 0.02)),
          (Vec3::new(0.0, 1.86, 0.0), Vec2::new(0.014, 0.03)),
          (Vec3::new(0.0, 1.76, 0.2), Vec2::new(0.014, 0.02))
        ],
        3
      );
      let cheek = |side: f32| {
        Cage::rounded(
          Vec3::new(side * 0.15, 1.6, -0.06),
          Vec3::new(0.03, 0.12, 0.15),
          UVec3::ONE,
          0.2
        )
      };
      vec![
        plain(&dome(Vec3::new(0.35, 0.2, 0.37), 1.66), IRON),
        plain(&horn(1.0), BONE),
        plain(&horn(-1.0), BONE),
        plain(&crest, GOLD),
        plain(&cheek(1.0), IRON),
        plain(&cheek(-1.0), IRON),
      ]
    }
    Band::Skeleton if luck % 3 == 0 => {
      let brim = Cage::prism(8, 0.215, 0.022)
        .shaped(|at| at * Vec3::new(1.0, 1.0, 1.05) + Vec3::Y * 1.685);
      let guard = Cage::tube(
        &[
          (Vec3::new(0.0, 1.69, -0.19), Vec2::new(0.022, 0.01)),
          (Vec3::new(0.0, 1.6, -0.2), Vec2::new(0.016, 0.01))
        ],
        3
      );
      vec![
        plain(&dome(Vec3::new(0.33, 0.17, 0.35), 1.68), IRONRUST),
        plain(&brim, IRONRUST),
        plain(&guard, IRONRUST),
      ]
    }
    _ => vec![]
  };
  merged(
    [face, plain(&jaw, BONE), plain(&mouth, HOLLOW)]
      .into_iter()
      .chain(teeth.iter().map(|tooth| plain(tooth, BONE)))
      .chain(helm)
  )
}

fn sword(
  side: f32,
  length: f32,
  wide: f32,
  blade: [f32; 3],
  guard: [f32; 3]
) -> Vec<(Cage, [f32; 3])> {
  let at = |x: f32, y: f32, z: f32| Vec3::new(side * 0.315 + x, y, z);
  vec![
    (rod(at(0.0, 0.79, 0.07), at(0.0, 0.79, -0.04), 0.019, 4), HAFT),
    (Cage::rounded(at(0.0, 0.79, 0.085), Vec3::splat(0.045), UVec3::ONE, 0.6), guard),
    (
      Cage::tube(
        &[(-0.09, 0.765), (0.0, 0.79), (0.09, 0.765)]
          .map(|(x, y)| (at(x, y, -0.05), Vec2::new(0.016, 0.02))),
        4
      ),
      guard
    ),
    (
      Cage::tube(
        &[(-0.06, 1.0), (-0.6, 0.9), (-0.92, 0.55), (-1.0, 0.08)]
          .map(|(z, girth)| (at(0.0, 0.79, z * length), Vec2::new(0.011, wide * girth))),
        4
      ),
      blade
    ),
  ]
}

fn tower(side: f32) -> Vec<(Cage, [f32; 3])> {
  let middle = Vec3::new(side * 0.43, 1.08, 0.0);
  let bowed = |at: Vec3| at + Vec3::X * side * (at.z - middle.z).powi(2) * 0.5;
  let face = middle + Vec3::X * side * 0.03;
  let bar = |from: Vec3, to: Vec3| {
    Cage::tube(
      &[(bowed(face + from), Vec2::splat(0.022)), (bowed(face + to), Vec2::splat(0.022))],
      3
    )
  };
  let boss = bowed(face + Vec3::Y * 0.1);
  let rivets =
    [(0.33, 0.24), (0.33, -0.24), (-0.33, 0.24), (-0.33, -0.24)].map(|(y, z)| {
      let root = bowed(face + Vec3::new(0.0, y, z));
      (Cage::spike(root, root + Vec3::X * side * 0.035, Vec2::splat(0.018)), GOLD)
    });
  [
    vec![
      (
        Cage::rounded(middle, Vec3::new(0.045, 0.78, 0.6), UVec3::new(1, 2, 3), 0.12)
          .shaped(bowed),
        [0.2, 0.2, 0.24]
      ),
      (bar(Vec3::new(0.0, 0.3, 0.0), Vec3::new(0.0, -0.3, 0.0)), GOLD),
      (bar(Vec3::new(0.0, 0.1, 0.2), Vec3::new(0.0, 0.1, -0.2)), GOLD),
      (Cage::rounded(boss, Vec3::splat(0.09), UVec3::ONE, 0.6), IRON),
      (Cage::spike(boss, boss + Vec3::X * side * 0.12, Vec2::splat(0.03)), STEEL),
    ],
    rivets.to_vec()
  ]
  .concat()
}

fn limb(side: f32, arm: bool, band: Band) -> Vec<(Cage, [f32; 3])> {
  let warden = band == Band::Warden;
  let at = |x: f32, y: f32, z: f32| Vec3::new(side * x, y, z);
  let bit =
    |from: Vec3, to: Vec3, thick: f32| (Cage::spike(from, to, Vec2::splat(thick)), BONE);
  let bones = match arm {
    true => [
      vec![
        (Cage::rounded(at(0.275, 1.4, 0.0), Vec3::splat(0.075), UVec3::ONE, 0.6), MARROW),
        (long_bone(at(0.28, 1.38, 0.0), at(0.3, 1.13, 0.0), 0.032), BONE),
        (long_bone(at(0.3, 1.1, -0.01), at(0.31, 0.87, -0.03), 0.024), BONE),
        (rod(at(0.325, 1.07, 0.02), at(0.33, 0.9, 0.01), 0.018, 3), MARROW),
        (
          Cage::rounded(
            at(0.315, 0.835, -0.02),
            Vec3::new(0.05, 0.06, 0.075),
            UVec3::ONE,
            0.5
          ),
          BONE
        ),
        bit(at(0.3, 0.84, -0.06), at(0.28, 0.8, -0.1), 0.016),
      ],
      [-0.045, -0.015, 0.015]
        .map(|z| bit(at(0.315, 0.81, z), at(0.32, 0.745, z - 0.012), 0.016))
        .to_vec()
    ]
    .concat(),
    false => [
      vec![
        (long_bone(at(0.12, 0.77, 0.0), at(0.12, 0.43, -0.01), 0.037), BONE),
        (
          Cage::rounded(at(0.12, 0.41, -0.045), Vec3::splat(0.05), UVec3::ONE, 0.6),
          MARROW
        ),
        (
          Cage::rounded(
            at(0.12, 0.04, -0.04),
            Vec3::new(0.095, 0.06, 0.17),
            UVec3::new(1, 1, 2),
            0.35
          ),
          BONE
        ),
      ],
      match warden {
        true => vec![],
        false => vec![
          (long_bone(at(0.115, 0.39, -0.01), at(0.115, 0.09, 0.0), 0.03), BONE),
          (rod(at(0.15, 0.36, 0.015), at(0.15, 0.12, 0.02), 0.016, 3), MARROW),
        ]
      },
      [0.09, 0.12, 0.15]
        .map(|x| bit(at(x, 0.03, -0.11), at(x + 0.005, 0.02, -0.19), 0.018))
        .to_vec()
    ]
    .concat()
  };
  let armour = match (warden, arm) {
    (false, _) => vec![],
    (true, true) => vec![
      (
        Cage::rounded(
          at(0.28, 1.43, 0.0),
          Vec3::new(0.2, 0.11, 0.22),
          UVec3::new(2, 1, 2),
          0.6
        ),
        IRON
      ),
      (Cage::spike(at(0.32, 1.47, 0.0), at(0.41, 1.6, 0.0), Vec2::splat(0.03)), STEEL),
      (Cage::spike(at(0.27, 1.48, 0.05), at(0.31, 1.6, 0.1), Vec2::splat(0.025)), STEEL),
    ],
    (true, false) => vec![
      (
        Cage::tube(
          &[
            (at(0.12, 0.37, -0.01), Vec2::splat(0.06)),
            (at(0.12, 0.12, 0.0), Vec2::splat(0.05))
          ],
          6
        ),
        IRON
      ),
      (
        Cage::rounded(
          at(0.12, 0.41, -0.05),
          Vec3::new(0.08, 0.07, 0.04),
          UVec3::ONE,
          0.5
        ),
        GOLD
      ),
    ]
  };
  let arms = match (arm, side > 0.0, warden) {
    (true, true, false) => sword(side, 0.58, 0.042, RUST, IRONRUST),
    (true, true, true) => sword(side, 1.12, 0.075, STEEL, GOLD),
    (true, false, true) => tower(side),
    _ => vec![]
  };
  bones.into_iter().chain(armour).chain(arms).collect()
}

fn torso(warden: bool) -> Vec<(Cage, [f32; 3])> {
  let vertebrae = (0..5).flat_map(|step| {
    let y = 0.9 + step as f32 * 0.12;
    let z = if y > 1.08 { 0.1 } else { 0.05 };
    [
      (rod(Vec3::new(0.0, y - 0.022, z), Vec3::new(0.0, y + 0.022, z), 0.036, 4), MARROW),
      (
        Cage::spike(
          Vec3::new(0.0, y, z + 0.025),
          Vec3::new(0.0, y - 0.03, z + 0.085),
          Vec2::splat(0.016)
        ),
        BONE
      )
    ]
  });
  let wing = |side: f32| {
    let middle = Vec3::new(side * 0.09, 0.84, 0.02);
    (
      Cage::rounded(middle, Vec3::new(0.13, 0.12, 0.035), UVec3::new(2, 1, 1), 0.45)
        .shaped(|at| {
          Quat::from_euler(EulerRot::YXZ, side * 0.55, 0.0, side * -0.25)
            .mul_vec3(at - middle)
            + middle
        }),
      BONE
    )
  };
  let pelvis = match warden {
    true => vec![],
    false => vec![
      wing(1.0),
      wing(-1.0),
      (
        Cage::rounded(
          Vec3::new(0.0, 0.83, 0.06),
          Vec3::new(0.08, 0.1, 0.05),
          UVec3::ONE,
          0.5
        ),
        MARROW
      ),
      (
        Cage::tube(
          &[(-0.1, 0.78, 0.0), (0.0, 0.75, -0.045), (0.1, 0.78, 0.0)]
            .map(|(x, y, z)| (Vec3::new(x, y, z), Vec2::splat(0.022))),
          4
        ),
        BONE
      ),
    ]
  };
  let sternum = Cage::tube(
    &[
      (Vec3::new(0.0, 1.36, -0.13), Vec2::new(0.03, 0.02)),
      (Vec3::new(0.0, 1.25, -0.15), Vec2::new(0.026, 0.018)),
      (Vec3::new(0.0, 1.15, -0.14), Vec2::new(0.016, 0.014))
    ],
    3
  );
  let collar = |side: f32| {
    [
      (
        Cage::tube(
          &[(0.03, 1.37, -0.12), (0.14, 1.39, -0.09), (0.25, 1.41, -0.02)]
            .map(|(x, y, z)| (Vec3::new(side * x, y, z), Vec2::splat(0.022))),
          3
        ),
        BONE
      ),
      (
        Cage::rounded(
          Vec3::new(side * 0.13, 1.29, 0.135),
          Vec3::new(0.13, 0.16, 0.02),
          UVec3::ONE,
          0.2
        )
        .shaped(|at| {
          let low = ((1.37 - at.y) / 0.16).clamp(0.0, 1.0);
          Vec3::new(side * 0.15 + (at.x - side * 0.15) * (1.0 - low * 0.8), at.y, at.z)
        }),
        BONE
      )
    ]
  };
  let armour = match warden {
    false => vec![],
    true => [
      vec![(
        Cage::prism(8, 0.17, 0.045)
          .shaped(|at| at * Vec3::new(1.05, 1.0, 0.8) + Vec3::new(0.0, 0.875, 0.02)),
        [0.28, 0.18, 0.1]
      )],
      (0..4)
        .map(|plate| {
          let turn = -1.0 + plate as f32 * 0.667;
          let out = Vec3::new(turn.sin(), 0.0, -turn.cos());
          let middle = Vec3::new(0.0, 0.79, 0.02) + out * 0.17;
          (
            Cage::rounded(middle, Vec3::new(0.11, 0.15, 0.02), UVec3::ONE, 0.2).shaped(
              |at| {
                let lean = (0.87 - at.y).max(0.0) * 0.35;
                Quat::from_rotation_y(-turn).mul_vec3(at - middle) + middle + out * lean
              }
            ),
            IRON
          )
        })
        .collect(),
      vec![(
        Cage::rounded(
          Vec3::new(0.0, 0.875, -0.14),
          Vec3::new(0.06, 0.05, 0.02),
          UVec3::ONE,
          0.3
        ),
        GOLD
      )]
    ]
    .concat()
  };
  vertebrae
    .chain(ribs(1.0).into_iter().chain(ribs(-1.0)).map(|rib| (rib, BONE)))
    .chain(pelvis)
    .chain([(sternum, BONE)])
    .chain(collar(1.0))
    .chain(collar(-1.0))
    .chain(armour)
    .collect()
}

pub fn ossified(band: Band, luck: u32) -> Mesh {
  let warden = band == Band::Warden;
  let part = |joint: u16| {
    move |(cage, color): (Cage, [f32; 3])| sculpt(&cage, |_, _| color, &[], |_| joint)
  };
  merged(
    [skull(band, luck)].into_iter().chain(torso(warden).into_iter().map(part(0))).chain(
      LIMBS.iter().flat_map(|&(joint, side, arm)| {
        limb(side, arm, band).into_iter().map(part(joint))
      })
    )
  )
}

pub fn bindings() -> Vec<Mat4> {
  (0..6).map(|joint| Mat4::from_translation(-pivot(joint))).collect()
}

#[derive(Resource, Default)]
struct Ossuary(
  HashMap<(Band, bool), (Handle<Mesh>, Handle<SkinnedMeshInverseBindposes>)>
);

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
  mut poses: ResMut<Assets<SkinnedMeshInverseBindposes>>,
  mut ossuary: ResMut<Ossuary>,
  mut commands: Commands
) {
  for (entity, folk) in arrivals.iter() {
    let bony = folk.band.sculpted();
    let imp = folk.band == Band::Imp;
    let bone = |joint: u16| match imp {
      true => crate::imp::pivot(joint),
      false => pivot(joint)
    };
    let clad = match bony {
      true => Clad {
        material: materials.add(StandardMaterial {
          perceptual_roughness: 0.9,
          reflectance: 0.2,
          ..default()
        }),
        image: Handle::default()
      },
      false => clothe(&outfit(folk.band, folk.luck), &mut images, &mut materials)
    };
    if folk.band.ghostly()
      && let Some(mut material) = materials.get_mut(&clad.material)
    {
      material.alpha_mode = AlphaMode::Blend;
      material.base_color = Color::WHITE.with_alpha(GHOSTLY);
      material.emissive = LinearRgba::rgb(0.06, 0.02, 0.12)
    }
    let material = MeshMaterial3d(clad.material.clone());
    let glints: Vec<(Vec3, Handle<Mesh>, Handle<StandardMaterial>)> = eyes(folk.band)
      .into_iter()
      .map(|(centre, size, [r, g, b])| {
        (
          centre,
          shapes.cuboid(&mut meshes, size),
          materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            emissive: LinearRgba::rgb(r, g, b) * 8.0,
            unlit: true,
            ..default()
          })
        )
      })
      .collect();
    let mut gear = |parent: Entity, gear: Gear, commands: &mut Commands| {
      for (centre, size, color) in gear.into_iter() {
        commands.spawn((
          Mesh3d(shapes.cuboid(&mut meshes, size)),
          MeshMaterial3d(shapes.paint(&mut materials, color)),
          Transform::from_translation(centre),
          ChildOf(parent)
        ));
      }
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
    let fleshed = |part: Part| (Mesh3d(kit.mesh(part)), material.clone());
    if !bony {
      commands.spawn((
        fleshed(Part::Body),
        Transform::from_xyz(0.0, 18.0 * PX, 0.0),
        ChildOf(entity)
      ));
    }
    let head = commands
      .spawn((
        Joint::Neck,
        Rest(bone(SKULL)),
        Transform::from_translation(bone(SKULL)),
        Visibility::default(),
        ChildOf(entity)
      ))
      .id();
    if !bony {
      commands.entity(head).insert(fleshed(Part::Head));
      gear(head, hat(folk.band, folk.luck), &mut commands)
    }
    for (centre, mesh, glow) in glints.into_iter() {
      commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(glow),
        Transform::from_translation(centre),
        ChildOf(head)
      ));
    }
    let limbs: Vec<Entity> = LIMBS
      .iter()
      .zip([(0.0, Part::Arm), (PI, Part::LeftArm), (PI, Part::Leg), (0.0, Part::LeftLeg)])
      .map(|(&(joint, side, arm), (phase, part))| {
        let rest = match (bony, arm) {
          (true, _) => bone(joint),
          (false, true) => Vec3::new(side * 6.0, 24.0, 0.0) * PX,
          (false, false) => Vec3::new(side * 2.0, 12.0, 0.0) * PX
        };
        let limb = commands
          .spawn((
            if arm { Joint::Arm(phase) } else { Joint::Leg(phase) },
            Rest(rest),
            Transform::from_translation(rest),
            Visibility::default(),
            ChildOf(entity)
          ))
          .id();
        if !bony {
          commands.entity(limb).insert(fleshed(part));
        }
        match (arm, side > 0.0) {
          (true, true) => gear(limb, weapon(folk.band), &mut commands),
          (true, false) => gear(limb, shield(folk.band, folk.luck), &mut commands),
          _ => ()
        }
        limb
      })
      .collect();
    let wings: Vec<Entity> = match imp {
      true => [(6, 1.0), (7, -1.0)]
        .map(|(joint, side)| {
          commands
            .spawn((
              Joint::Wing(side),
              Rest(bone(joint)),
              Transform::from_translation(bone(joint)),
              Visibility::default(),
              ChildOf(entity)
            ))
            .id()
        })
        .to_vec(),
      false => vec![]
    };
    if bony {
      let (mesh, inverse_bindposes) = ossuary
        .0
        .entry((folk.band, folk.band == Band::Skeleton && folk.luck % 3 == 0))
        .or_insert_with(|| match imp {
          true => (meshes.add(crate::imp::fiendish()), poses.add(crate::imp::bindings())),
          false => (meshes.add(ossified(folk.band, folk.luck)), poses.add(bindings()))
        })
        .clone();
      commands.spawn((
        Mesh3d(mesh),
        material.clone(),
        SkinnedMesh {
          inverse_bindposes,
          joints: [vec![entity, head], limbs, wings].concat()
        },
        DynamicSkinnedMeshBounds,
        ChildOf(entity)
      ));
    }
    commands.entity(entity).insert(clad);
  }
}

fn animate(
  time: Res<Time>,
  mut folk: Query<(Entity, &Folk, &Health, &Clad, &mut Shown, &mut Transform)>,
  family: Query<&Children>,
  mut joints: Query<(&Joint, &Rest, &mut Transform), Without<Shown>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let (dt, now) = (time.delta_secs(), time.elapsed_secs());
  for (entity, folk, health, clad, mut shown, mut transform) in folk.iter_mut() {
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
      let alpha = if folk.band.ghostly() { GHOSTLY } else { 1.0 };
      material.base_color = match shown.flushed > 0.0 {
        true => Color::srgb(1.0, 0.35, 0.35),
        false => Color::WHITE
      }
      .with_alpha(alpha)
    }
    let floating = folk.band.flies();
    let imp = folk.band == Band::Imp;
    let beat = now * 14.0 + folk.luck as f32;
    let swing = match floating {
      true => 0.0,
      false => shown.stride.sin() * (shown.pace / 4.3).min(1.0) * 0.7
    };
    let chop = (shown.swung / SWING).min(1.0);
    let brandish = match (folk.aboard, folk.band.hostile()) {
      (true, true) => Some(2.3 + (now * 3.0 + folk.luck as f32).sin() * 0.3),
      _ => None
    };
    let hover = match (imp, floating) {
      (true, _) => Vec3::Y * (0.3 + beat.sin() * 0.05 + (now * 1.7).sin() * 0.08),
      (_, true) => Vec3::Y * (0.25 + (now * 2.1 + folk.luck as f32).sin() * 0.12),
      _ => Vec3::ZERO
    };
    *transform = Transform::from_translation(shown.at + hover)
      .with_rotation(Quat::from_rotation_y(shown.yaw))
      .with_scale(Vec3::splat(folk.band.scale()));
    for child in family.iter_descendants(entity) {
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
          Joint::Arm(phase) if imp => {
            Quat::from_rotation_x(0.55 + (beat * 0.5 + phase).sin() * 0.12)
          }
          Joint::Leg(phase) if imp => {
            Quat::from_rotation_x(-0.35 + (beat * 0.3 + phase).sin() * 0.15)
          }
          Joint::Wing(side) => Quat::from_euler(
            EulerRot::YZX,
            side * 0.25,
            side * (0.15 + beat.sin() * 0.6),
            0.0
          ),
          Joint::Arm(_) if floating => {
            Quat::from_rotation_x(1.3 + (now * 3.0).sin() * 0.15)
          }
          Joint::Arm(phase) | Joint::Leg(phase) => {
            Quat::from_rotation_x(swing * phase.cos())
          }
          _ => Quat::IDENTITY
        }
      }
    }
  }
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
      .filter_map(|(entity, folk)| struck(folk, from, toward).map(|near| (entity, near)))
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
      .init_resource::<Ossuary>()
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

#[cfg(test)]
mod tests {
  use {super::*, crate::sculpt::sheet};

  #[test]
  #[ignore]
  fn ossuary() {
    sheet(
      &[(Band::Skeleton, 0), (Band::Skeleton, 1), (Band::Warden, 0)]
        .map(|(band, luck)| ossified(band, luck)),
      "screenshots/ossuary.ppm"
    )
  }
}
