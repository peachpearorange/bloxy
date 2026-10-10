use {crate::{authority::{Controller, player_of},
             beast::{Bone, Grain, Joint, Rest, Shapes, rig},
             block::Block,
             folk::{PUNCH, ray_box},
             generate,
             island::{Island, Kind, SEA, Wood},
             loose::scatter,
             menu::Menu,
             noise::{hash, unit},
             player::{Pilot, captured},
             protocol::{Avatar, EYE, Health, Inventory, REACH, Strike, authority,
                        plays},
             voxels::Voxels},
     bevy::{platform::collections::HashSet,
            prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize},
     std::f32::consts::{PI, TAU}};

const WAKE: f32 = 120.0;
const SLEEP: f32 = 180.0;
const STARTLE: f32 = 5.0;
const PLUCK_AGAIN: f32 = 20.0;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Breed {
  Swan,
  Raven,
  Gull,
  Parrot
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stance {
  Rest,
  Walk,
  Swim,
  Fly,
  Glide
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Bird {
  pub breed: Breed,
  pub at: Vec3,
  pub yaw: f32,
  pub stance: Stance
}

impl Breed {
  fn speed(self) -> f32 {
    match self {
      Breed::Swan => 7.0,
      Breed::Raven => 8.0,
      Breed::Gull => 9.0,
      Breed::Parrot => 7.5
    }
  }

  fn waterborne(self) -> Option<bool> {
    match self {
      Breed::Swan => Some(true),
      Breed::Gull => None,
      _ => Some(false)
    }
  }

  pub const HEALTH: u8 = 4;

  fn size(self) -> f32 {
    match self {
      Breed::Swan => 1.0,
      Breed::Raven => 0.75,
      Breed::Gull => 0.8,
      Breed::Parrot => 0.6
    }
  }

  fn half(self) -> Vec3 {
    match self {
      Breed::Swan => Vec3::new(0.3, 0.9, 0.45),
      _ => Vec3::new(0.25, 0.4, 0.3)
    }
  }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mood {
  Ground,
  Aloft,
  Descend
}

#[derive(Component)]
struct Flight {
  home: Vec2,
  radius: f32,
  mood: Mood,
  timer: f32,
  angle: f32,
  altitude: f32,
  target: Vec3,
  velocity: Vec3,
  plucked: f32,
  luck: u32
}

impl Flight {
  fn new(breed: Breed, home: Vec2, radius: f32, luck: u32) -> Flight {
    Flight {
      home,
      radius: radius * if breed == Breed::Swan { 1.1 } else { 0.9 },
      mood: Mood::Ground,
      timer: 4.0 + unit(luck, 1, 0, 0) * 20.0,
      angle: unit(luck, 2, 0, 0) * TAU,
      altitude: 0.0,
      target: Vec3::ZERO,
      velocity: Vec3::ZERO,
      plucked: 0.0,
      luck
    }
  }

  fn dice(&mut self) -> f32 {
    self.luck = hash(self.luck, 0x8B, 0x1D, 0x47);
    unit(self.luck, 0, 0, 0)
  }
}

#[derive(Resource, Default)]
pub struct Flocks(pub HashSet<IVec2>);

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn toward(offset: Vec2) -> f32 { (-offset.x).atan2(-offset.y) }

fn perch(voxels: &mut Voxels, spot: Vec2, from: f32) -> (f32, bool) {
  let cell = spot.floor().as_ivec2();
  let top = (from as i32 + 3).max(SEA + 1);
  (0..(top - SEA + 16).max(1))
    .map(|depth| IVec3::new(cell.x, top - depth, cell.y))
    .find_map(|at| {
      let block = voxels.ensure(at);
      match (block.solid(), block.fluid()) {
        (true, _) => Some((at.y as f32 + 1.0, false)),
        (_, true) => Some((at.y as f32 + 0.88, true)),
        _ => None
      }
    })
    .unwrap_or((SEA as f32 + 0.88, true))
}

fn landing(voxels: &mut Voxels, flight: &mut Flight, breed: Breed) -> Vec3 {
  let seed = voxels.seed;
  (0..12)
    .map(|_| {
      let angle = flight.dice() * TAU;
      let reach = flight.radius * (0.2 + flight.dice() * 1.1);
      flight.home + Vec2::from_angle(angle) * reach
    })
    .find(|spot| {
      let wet = generate::height(seed, spot.x as i32, spot.y as i32) < SEA;
      breed.waterborne().is_none_or(|water| water == wet)
    })
    .map(|spot| {
      let (y, _) = perch(voxels, spot, 110.0);
      Vec3::new(spot.x.floor() + 0.5, y, spot.y.floor() + 0.5)
    })
    .unwrap_or(flight.home.extend(SEA as f32 + 0.88).xzy())
}

fn hatch(voxels: &Voxels, breed: Breed, at: Vec3, mut flight: Flight) -> impl Bundle {
  let wet =
    voxels.block((at - Vec3::Y * 0.5).floor().as_ivec3()).is_some_and(Block::fluid);
  (
    Replicated,
    Bird {
      breed,
      at,
      yaw: flight.dice() * TAU,
      stance: if wet { Stance::Swim } else { Stance::Rest }
    },
    Health(Breed::HEALTH),
    flight
  )
}

#[cfg_attr(target_arch = "wasm32", expect(dead_code))]
pub fn release(voxels: &Voxels, breed: Breed, at: Vec3, luck: u32) -> impl Bundle {
  let (home, radius) = Island::near(voxels.seed, at)
    .first()
    .map_or((at.xz(), 24.0), |island| (island.centre, island.radius));
  hatch(voxels, breed, at, Flight::new(breed, home, radius, luck))
}

fn muster(
  mut voxels: ResMut<Voxels>,
  mut flocks: ResMut<Flocks>,
  players: Query<&Avatar>,
  mut commands: Commands
) {
  let seed = voxels.seed;
  for island in players
    .iter()
    .flat_map(|avatar| {
      Island::near(seed, avatar.at)
        .into_iter()
        .filter(|island| avatar.at.xz().distance(island.centre) < WAKE)
    })
    .collect::<Vec<_>>()
    .into_iter()
    .filter(|island| flocks.0.insert(island.cell))
  {
    let key =
      hash(seed ^ 0xB1D5, island.cell.x, 0x17, 0) ^ hash(seed, 0, island.cell.y, 0x29);
    let counts: &[(Breed, u32)] = match (island.kind, island.wood) {
      (_, Wood::Palm) => &[(Breed::Parrot, 4), (Breed::Gull, 3)],
      (Kind::Meadow, _) => &[(Breed::Swan, 3), (Breed::Gull, 3)],
      (Kind::Woods, _) => &[(Breed::Raven, 3), (Breed::Swan, 2), (Breed::Gull, 1)],
      (Kind::Peak, _) => &[(Breed::Raven, 4), (Breed::Gull, 2)],
      (Kind::Frost, _) => &[(Breed::Raven, 3), (Breed::Gull, 2)],
      (Kind::Dunes, _) => &[(Breed::Gull, 5)],
      (Kind::Volcano | Kind::Mushroom, _) => &[(Breed::Raven, 2), (Breed::Gull, 1)]
    };
    for (index, (breed, _)) in counts
      .iter()
      .flat_map(|&(breed, count)| (0..count).map(move |index| (breed, index)))
      .enumerate()
    {
      let luck = key ^ (index as u32 + 1).wrapping_mul(0x9E37_79B9);
      let mut flight = Flight::new(breed, island.centre, island.radius, luck);
      let at = landing(&mut voxels, &mut flight, breed);
      commands.spawn(hatch(&voxels, breed, at, flight));
    }
  }
}

fn fly(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  players: Query<&Avatar>,
  mut birds: Query<(&mut Bird, &mut Flight)>
) {
  let dt = time.delta_secs().min(0.1);
  let watched =
    |at: Vec3| players.iter().any(|avatar| avatar.at.xz().distance(at.xz()) < SLEEP);
  for (mut bird, mut flight) in birds.iter_mut().filter(|(bird, _)| watched(bird.at)) {
    let Bird { breed, mut at, mut yaw, .. } = *bird;
    flight.timer -= dt;
    flight.plucked -= dt;
    let startled = players.iter().any(|avatar| avatar.at.distance(at) < STARTLE);
    let ground = (at - Vec3::Y * 0.4).floor().as_ivec3();
    let swimming =
      voxels.ensure(ground).fluid() || voxels.ensure(ground + IVec3::Y).fluid();
    let stance = match flight.mood {
      Mood::Ground if flight.timer <= 0.0 || startled => {
        let (floor, _) = perch(&mut voxels, at.xz(), at.y + 1.0);
        flight.mood = Mood::Aloft;
        flight.timer = 15.0 + flight.dice() * 30.0;
        flight.altitude = floor.max(SEA as f32) + 10.0 + flight.dice() * 18.0;
        flight.angle = toward(flight.home - at.xz()) + PI;
        flight.velocity = ahead(yaw) * 2.0 + Vec3::Y * 4.0;
        Stance::Fly
      }
      Mood::Ground => {
        let pace = match (swimming, flight.dice() < dt * 0.4) {
          (_, true) => {
            yaw += (flight.dice() - 0.5) * PI;
            0.0
          }
          (true, _) => 0.5,
          _ => 0.35
        };
        let resting = (flight.timer * 0.7).sin() > 0.3;
        let step = if resting { 0.0 } else { pace };
        let next = at + ahead(yaw) * step * dt;
        let (floor, wet) = perch(&mut voxels, next.xz(), at.y + 1.0);
        let suits = breed.waterborne().is_none_or(|water| water == wet);
        let straying = next.xz().distance(flight.home) > flight.radius * 1.4;
        match suits && (floor - at.y).abs() < 1.1 && !straying {
          true => at = next.with_y(floor),
          false => yaw += PI * 0.5
        }
        match (wet, step > 0.0) {
          (true, _) => Stance::Swim,
          (false, true) => Stance::Walk,
          _ => Stance::Rest
        }
      }
      Mood::Aloft | Mood::Descend => {
        if flight.mood == Mood::Aloft && flight.timer <= 0.0 {
          flight.mood = Mood::Descend;
          flight.target = landing(&mut voxels, &mut flight, breed)
        }
        let speed = breed.speed();
        flight.angle += speed / flight.radius.max(12.0) * dt;
        let circling =
          flight.home + Vec2::from_angle(flight.angle) * flight.radius.max(12.0);
        let goal = match flight.mood {
          Mood::Descend => flight.target,
          _ => circling.extend(flight.altitude).xzy()
        };
        let offset = goal - at;
        let wish = offset.normalize_or_zero()
          * match flight.mood {
            Mood::Descend => speed.min(offset.length() * 1.5 + 1.0),
            _ => speed
          };
        let below = at.floor().as_ivec3() - IVec3::Y * 2;
        let lift = match voxels.ensure(at.floor().as_ivec3()).solid()
          || voxels.ensure(below).solid() && flight.mood == Mood::Aloft
        {
          true => Vec3::Y * 6.0,
          false => Vec3::ZERO
        };
        flight.velocity = flight.velocity.lerp(wish + lift, (dt * 1.8).min(1.0));
        at += flight.velocity * dt;
        if flight.velocity.xz().length() > 0.3 {
          yaw = toward(flight.velocity.xz())
        }
        match flight.mood == Mood::Descend && offset.length() < 0.4 {
          true => {
            at = flight.target;
            flight.mood = Mood::Ground;
            flight.velocity = Vec3::ZERO;
            flight.timer = 12.0 + flight.dice() * 30.0;
            Stance::Rest
          }
          false => match flight.velocity.y < -0.5 || (flight.angle * 1.7).sin() > 0.6 {
            true => Stance::Glide,
            false => Stance::Fly
          }
        }
      }
    };
    bird.set_if_neq(Bird { breed, at, yaw: yaw.rem_euclid(TAU), stance });
  }
}

fn pluck(
  mut strikes: MessageReader<FromClient<Strike>>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  mut birds: Query<(&Bird, &mut Flight, &mut Health)>,
  mut commands: Commands
) {
  for &FromClient { client_id, message: Strike(target) } in strikes.read() {
    if let Ok((bird, mut flight, mut health)) = birds.get_mut(target)
      && let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && (avatar.at + Vec3::Y * EYE).distance(bird.at) <= REACH + 1.5
    {
      if flight.plucked <= 0.0 {
        flight.plucked = PLUCK_AGAIN;
        let feathers = 1 + (flight.dice() * 2.0) as u32;
        for _ in 0..feathers {
          inventory.add(Block::Feather);
        }
      }
      flight.timer = 0.0;
      health.0 = health.0.saturating_sub(PUNCH);
      if health.0 == 0 {
        let feathers = 1 + (flight.dice() * 2.0) as u16;
        scatter(&mut commands, Block::Feather, feathers, bird.at, Vec3::Y * 2.0, 0.4);
        commands.entity(target).despawn()
      }
    }
  }
}

pub fn struck(bird: &Bird, from: Vec3, toward: Vec3) -> Option<f32> {
  let half = bird.breed.half() * bird.breed.size();
  ray_box(from, toward, bird.at - half.with_y(0.0), bird.at + half)
}

fn poke(
  buttons: Res<ButtonInput<MouseButton>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  birds: Query<(Entity, &Bird)>,
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
    if let Some((target, _)) = birds
      .iter()
      .filter_map(|(entity, bird)| struck(bird, from, toward).map(|near| (entity, near)))
      .filter(|&(_, near)| near <= blocked)
      .min_by(|a, b| a.1.total_cmp(&b.1))
    {
      strikes.write(Strike(target));
    }
  }
}

type Cuboid3 = (Vec3, Vec3, [f32; 3], Grain);

fn plain(centre: [f32; 3], size: [f32; 3], color: [f32; 3]) -> Cuboid3 {
  (Vec3::from(centre), Vec3::from(size), color, Grain::Plain)
}

struct Plumage {
  body: [f32; 3],
  belly: [f32; 3],
  wing: [f32; 3],
  tip: [f32; 3],
  beak: [f32; 3],
  legs: [f32; 3]
}

fn plumage(breed: Breed) -> Plumage {
  match breed {
    Breed::Swan => Plumage {
      body: [0.95, 0.95, 0.93],
      belly: [0.9, 0.9, 0.88],
      wing: [0.97, 0.97, 0.95],
      tip: [0.88, 0.88, 0.86],
      beak: [0.95, 0.45, 0.1],
      legs: [0.1, 0.1, 0.1]
    },
    Breed::Raven => Plumage {
      body: [0.07, 0.07, 0.09],
      belly: [0.1, 0.1, 0.13],
      wing: [0.08, 0.08, 0.12],
      tip: [0.05, 0.05, 0.07],
      beak: [0.15, 0.15, 0.16],
      legs: [0.12, 0.12, 0.12]
    },
    Breed::Gull => Plumage {
      body: [0.95, 0.95, 0.95],
      belly: [0.92, 0.92, 0.92],
      wing: [0.62, 0.66, 0.7],
      tip: [0.08, 0.08, 0.08],
      beak: [0.98, 0.8, 0.15],
      legs: [0.95, 0.7, 0.4]
    },
    Breed::Parrot => Plumage {
      body: [0.88, 0.12, 0.1],
      belly: [0.95, 0.3, 0.15],
      wing: [0.15, 0.35, 0.9],
      tip: [0.98, 0.85, 0.15],
      beak: [0.9, 0.85, 0.7],
      legs: [0.3, 0.3, 0.32]
    }
  }
}

fn body(breed: Breed) -> Vec<Bone> {
  let Plumage { body, belly, wing, tip, beak, legs } = plumage(breed);
  let swan = breed == Breed::Swan;
  let (girth, length, lift) = match breed {
    Breed::Swan => (0.42, 0.7, 0.32),
    Breed::Parrot => (0.2, 0.28, 0.24),
    _ => (0.24, 0.4, 0.22)
  };
  let neck = match swan {
    true => vec![
      plain([0.0, 0.18, -0.02], [0.1, 0.42, 0.1], body),
      plain([0.0, 0.42, -0.08], [0.12, 0.12, 0.2], body),
      plain([0.0, 0.4, -0.23], [0.06, 0.05, 0.12], beak),
      plain([0.0, 0.42, -0.17], [0.07, 0.07, 0.03], [0.05, 0.05, 0.05]),
      plain([0.065, 0.45, -0.1], [0.01, 0.03, 0.03], [0.05, 0.05, 0.05]),
      plain([-0.065, 0.45, -0.1], [0.01, 0.03, 0.03], [0.05, 0.05, 0.05]),
    ],
    false => vec![
      plain([0.0, 0.06, -0.06], [girth * 0.62, girth * 0.62, girth * 0.62], body),
      plain([0.0, 0.04, -0.06 - girth * 0.45], [0.05, 0.05, 0.12], beak),
      plain([girth * 0.32, 0.1, -0.12], [0.01, 0.035, 0.035], [0.02, 0.02, 0.02]),
      plain([-girth * 0.32, 0.1, -0.12], [0.01, 0.035, 0.035], [0.02, 0.02, 0.02]),
    ]
  };
  let span = girth * if swan { 1.6 } else { 1.9 };
  let wing_bone = |side: f32| Bone {
    parent: Some(0),
    joint: Joint::Wing(side),
    pivot: Vec3::new(side * girth * 0.45, lift + girth * 0.3, -length * 0.1),
    boxes: vec![
      plain(
        [side * span * 0.35, 0.0, length * 0.1],
        [span * 0.7, 0.03, length * 0.7],
        wing
      ),
      plain(
        [side * span * 0.85, 0.0, length * 0.2],
        [span * 0.3, 0.025, length * 0.5],
        tip
      ),
    ]
  };
  let leg = |side: f32, phase: f32| Bone {
    parent: Some(0),
    joint: Joint::Leg(phase),
    pivot: Vec3::new(side * girth * 0.22, lift - girth * 0.4, 0.0),
    boxes: vec![
      plain([0.0, -0.06, 0.0], [0.03, 0.14, 0.03], legs),
      plain([0.0, -0.13, -0.03], [0.07, 0.02, 0.09], legs),
    ]
  };
  vec![
    Bone {
      parent: None,
      joint: Joint::Trunk,
      pivot: Vec3::ZERO,
      boxes: vec![
        plain([0.0, lift, 0.0], [girth, girth * 0.8, length], body),
        plain(
          [0.0, lift - girth * 0.3, -0.02],
          [girth * 0.85, girth * 0.25, length * 0.8],
          belly
        ),
        plain(
          [0.0, lift + girth * 0.1, length * 0.6],
          [girth * 0.6, 0.04, length * 0.4],
          tip
        ),
      ]
    },
    Bone {
      parent: Some(0),
      joint: Joint::Neck,
      pivot: Vec3::new(0.0, lift + girth * 0.3, -length * 0.45),
      boxes: neck
    },
    wing_bone(1.0),
    wing_bone(-1.0),
    leg(1.0, 0.0),
    leg(-1.0, PI),
  ]
}

#[derive(Component)]
struct Shown {
  at: Vec3,
  yaw: f32,
  pitch: f32,
  flap: f32,
  stride: f32
}

fn dress(
  arrivals: Query<(Entity, &Bird), Without<Shown>>,
  mut shapes: ResMut<Shapes>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut images: ResMut<Assets<Image>>,
  mut commands: Commands
) {
  for (entity, bird) in arrivals.iter() {
    commands.entity(entity).insert((
      Shown { at: bird.at, yaw: bird.yaw, pitch: 0.0, flap: 0.0, stride: 0.0 },
      Transform::from_translation(bird.at),
      Visibility::default()
    ));
    rig(
      entity,
      &body(bird.breed),
      &mut shapes,
      &mut meshes,
      &mut materials,
      &mut images,
      &mut commands
    );
  }
}

fn animate(
  time: Res<Time>,
  mut birds: Query<(Entity, &Bird, &mut Shown, &mut Transform)>,
  family: Query<&Children>,
  mut joints: Query<(&Joint, &Rest, &mut Transform), Without<Shown>>
) {
  let (dt, now) = (time.delta_secs().max(1e-4), time.elapsed_secs());
  for (entity, bird, mut shown, mut transform) in birds.iter_mut() {
    let before = shown.at;
    shown.at = before.lerp(bird.at, (dt * 10.0).min(1.0));
    let turn = (bird.yaw - shown.yaw + PI).rem_euclid(TAU) - PI;
    shown.yaw += turn * (dt * 6.0).min(1.0);
    let moved = shown.at - before;
    let climb = match bird.stance {
      Stance::Fly | Stance::Glide => {
        (moved.y / moved.xz().length().max(1e-3)).clamp(-0.6, 0.6)
      }
      _ => 0.0
    };
    shown.pitch += (climb - shown.pitch) * (dt * 4.0).min(1.0);
    let beat = match bird.breed {
      Breed::Swan => 7.0,
      Breed::Raven => 11.0,
      Breed::Gull => 9.0,
      Breed::Parrot => 16.0
    };
    shown.flap += dt * if bird.stance == Stance::Fly { beat } else { 2.0 };
    shown.stride += moved.xz().length() * 14.0;
    *transform = Transform::from_translation(shown.at)
      .with_rotation(
        Quat::from_rotation_y(shown.yaw) * Quat::from_rotation_x(shown.pitch)
      )
      .with_scale(Vec3::splat(bird.breed.size()));
    let (stance, flap, stride) = (bird.stance, shown.flap, shown.stride);
    for child in family.iter_descendants(entity) {
      if let Ok((joint, rest, mut transform)) = joints.get_mut(child) {
        transform.rotation = match (*joint, stance) {
          (Joint::Wing(side), Stance::Fly) => {
            Quat::from_rotation_z(side * flap.sin() * 0.9)
          }
          (Joint::Wing(side), Stance::Glide) => {
            Quat::from_rotation_z(side * (0.08 + (now * 2.0).sin() * 0.05))
          }
          (Joint::Wing(side), _) => {
            Quat::from_rotation_z(-side * 1.35) * Quat::from_rotation_y(side * 0.15)
          }
          (Joint::Leg(_), Stance::Fly | Stance::Glide) => Quat::from_rotation_x(1.2),
          (Joint::Leg(phase), Stance::Walk) => {
            Quat::from_rotation_x((stride + phase).sin() * 0.6)
          }
          (Joint::Neck, Stance::Walk) => {
            Quat::from_rotation_x((stride * 2.0).sin() * 0.12)
          }
          (Joint::Neck, Stance::Rest | Stance::Swim) => {
            Quat::from_rotation_y((now * 0.7 + rest.0.x).sin() * 0.5)
          }
          (Joint::Neck, _) => Quat::from_rotation_x(0.25),
          _ => Quat::IDENTITY
        };
        transform.translation = rest.0
      }
    }
  }
}

pub struct Birds;

impl Plugin for Birds {
  fn build(&self, app: &mut App) {
    app
      .replicate::<Bird>()
      .init_resource::<Flocks>()
      .add_systems(
        Update,
        (muster, fly).chain().run_if(authority).run_if(resource_exists::<Voxels>)
      )
      .add_systems(PreUpdate, pluck.after(ServerSystems::Receive).run_if(authority))
      .add_systems(Update, (dress, animate).chain().run_if(plays))
      .add_systems(Update, poke.run_if(plays).run_if(resource_exists::<Pilot>));
  }
}
