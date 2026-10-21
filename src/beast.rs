use {crate::{authority::{Controller, player_of},
             bird,
             block::Block,
             cage::Cage,
             facet::{Form, Part},
             folk::ray_box,
             folk::{KNOCK, PUNCH, Vigour, hurt},
             generate,
             island::{Island, Kind, SEA, UNLOAD},
             loose::scatter,
             menu::Menu,
             noise::{hash, perlin3, unit},
             opts::opts,
             player::{Bulk, Marched, Pilot, captured, cells, march},
             protocol::{Avatar, Beast, Breed, EYE, Fleece, Health, Inventory, Knock,
                        Pose, REACH, Strike, authority, plays},
             sculpt::{Daub, sculpt},
             voxels::Voxels},
     bevy::{camera::visibility::DynamicSkinnedMeshBounds,
            mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
            platform::collections::{HashMap, HashSet},
            prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     std::f32::consts::{PI, TAU}};

const WAKE: f32 = 110.0;
const SLEEP: f32 = 170.0;
const GRAVITY: f32 = 30.0;
const LEAP: f32 = 9.2;
const GRAZE: f32 = 1.1;
const STROLL: f32 = 1.7;
const DASH: f32 = 6.5;
const PADDLE: f32 = 1.6;
const SKITTISH: f32 = 6.0;
const REGROW: f32 = 30.0;
const GRUDGE: f32 = 12.0;
const CHASE: f32 = 18.0;
const JAWS: f32 = 1.5;
const BITE: u8 = 2;
const BITE_EVERY: f32 = 1.0;

impl Breed {
  pub fn bulk(self) -> Bulk {
    match self {
      Breed::Sheep(_) => Bulk { half: 0.35, tall: 1.0 },
      Breed::Lizard => Bulk { half: 0.3, tall: 0.95 },
      Breed::Spider => Bulk { half: 0.42, tall: 0.6 }
    }
  }

  pub fn health(self) -> u8 {
    match self {
      Breed::Sheep(_) => 8,
      Breed::Lizard => 6,
      Breed::Spider => 8
    }
  }

  fn size(self) -> f32 {
    match self {
      Breed::Sheep(_) => 1.0,
      Breed::Lizard => 1.35,
      Breed::Spider => 1.0
    }
  }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mood {
  Idle,
  Graze,
  Stroll,
  Dash,
  Swim,
  Landing,
  Seaward,
  Hunt
}

#[derive(Component)]
pub struct Roam {
  home: IVec2,
  velocity: Vec3,
  grounded: bool,
  mood: Mood,
  timer: f32,
  ashore: f32,
  shorn: f32,
  stagger: f32,
  grudge: f32,
  bite: f32,
  luck: u32
}

impl Roam {
  pub fn knock(&mut self, push: Vec3) {
    self.velocity = push;
    self.stagger = 0.4;
    self.mood = Mood::Dash;
    self.timer = 2.5
  }

  fn dice(&mut self) -> f32 {
    self.luck = hash(self.luck, 0x6B, 0x21, 0x3F);
    unit(self.luck, 0, 0, 0)
  }
}

#[derive(Resource, Default)]
pub struct Herds(pub HashSet<IVec2>);

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn toward(offset: Vec2) -> f32 { (-offset.x).atan2(-offset.y) }

fn fleece(roll: f32) -> Fleece {
  [
    (0.45, Fleece::White),
    (0.6, Fleece::Silver),
    (0.72, Fleece::Grey),
    (0.84, Fleece::Black),
    (0.95, Fleece::Brown)
  ]
  .into_iter()
  .find(|&(below, _)| roll < below)
  .map_or(Fleece::Pink, |(_, fleece)| fleece)
}

fn muster(
  voxels: Res<Voxels>,
  mut herds: ResMut<Herds>,
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
    .filter(|island| herds.0.insert(island.cell))
  {
    let key =
      hash(seed ^ 0xBEA57, island.cell.x, 0x31, 0) ^ hash(seed, 0, island.cell.y, 9);
    let roll = |salt: u32, attempt: u32| unit(key ^ salt, attempt as i32, 0, 0);
    let grassy = matches!(island.kind, Kind::Meadow | Kind::Woods);
    let sheep = if grassy { (3 + (island.radius / 14.0) as u32).min(7) } else { 0 };
    let lizards = match island.kind {
      Kind::Frost | Kind::Volcano => 0,
      _ => 2 + (island.radius / 25.0) as u32
    };
    let flock = (0..sheep * 6)
      .filter_map(|attempt| {
        let spot = (island.centre
          + Vec2::new(roll(1, attempt) * 2.0 - 1.0, roll(2, attempt) * 2.0 - 1.0)
            * island.radius
            * 0.6)
          .floor()
          .as_ivec2();
        let ground = generate::height(seed, spot.x, spot.y);
        (ground > SEA + 1).then(|| {
          (
            Breed::Sheep(fleece(roll(3, attempt))),
            Vec3::new(spot.x as f32 + 0.5, ground as f32 + 1.05, spot.y as f32 + 0.5)
          )
        })
      })
      .take(sheep as usize);
    let swimmers = (0..lizards * 8)
      .filter_map(|attempt| {
        let angle = roll(4, attempt) * TAU;
        let reach = island.radius * (1.05 + roll(5, attempt) * 0.3);
        let spot = (island.centre + Vec2::from_angle(angle) * reach).floor().as_ivec2();
        let floor = generate::height(seed, spot.x, spot.y);
        (floor < SEA && floor >= SEA - 8).then(|| {
          (
            Breed::Lizard,
            Vec3::new(spot.x as f32 + 0.5, SEA as f32 + 0.6, spot.y as f32 + 0.5)
          )
        })
      })
      .take(lizards as usize);
    for (index, (breed, at)) in flock.chain(swimmers).enumerate() {
      commands.spawn(herd(
        breed,
        at,
        island.cell,
        key ^ (index as u32).wrapping_mul(0x9E37)
      ));
    }
  }
}

fn disperse(
  voxels: Res<Voxels>,
  players: Query<&Avatar>,
  beasts: Query<(Entity, &Roam)>,
  mut herds: ResMut<Herds>,
  mut commands: Commands
) {
  let seed = voxels.seed;
  let forgotten: Vec<IVec2> = herds
    .0
    .iter()
    .copied()
    .filter(|&cell| {
      Island::at(seed, cell).is_none_or(|island| {
        !players.iter().any(|avatar| avatar.at.xz().distance(island.centre) < UNLOAD)
      })
    })
    .collect();
  for cell in forgotten.iter() {
    herds.0.remove(cell);
  }
  for (entity, roam) in beasts.iter() {
    if forgotten.contains(&roam.home) {
      commands.entity(entity).despawn()
    }
  }
}

pub fn herd(breed: Breed, at: Vec3, home: IVec2, luck: u32) -> impl Bundle {
  (
    Replicated,
    Beast { breed, at, yaw: unit(luck, 7, 0, 0) * TAU, pose: Pose::Still },
    Health(breed.health()),
    Roam {
      home,
      velocity: Vec3::ZERO,
      grounded: false,
      mood: if breed == Breed::Lizard { Mood::Swim } else { Mood::Idle },
      timer: 1.0 + unit(luck, 8, 0, 0) * 3.0,
      ashore: 0.0,
      shorn: 0.0,
      stagger: 0.0,
      grudge: 0.0,
      bite: 0.0,
      luck
    }
  )
}

fn seek(
  voxels: &Voxels,
  at: Vec3,
  wanted: impl Fn(&Voxels, IVec3) -> bool
) -> Option<f32> {
  (1..=16).find_map(|distance| {
    (0..16).find_map(|step| {
      let direction = Vec2::from_angle(step as f32 * TAU / 16.0);
      let spot = (at.xz() + direction * distance as f32).floor().as_ivec2();
      let base = at.y.floor() as i32;
      (-3..=1)
        .any(|rise| wanted(voxels, IVec3::new(spot.x, base + rise, spot.y)))
        .then(|| toward(direction))
    })
  })
}

fn wet(voxels: &Voxels, cell: IVec3) -> bool {
  voxels.block(cell).is_some_and(Block::fluid)
}

fn standable(voxels: &Voxels, cell: IVec3) -> bool {
  voxels.solid(cell)
    && !wet(voxels, cell + IVec3::Y)
    && !voxels.solid(cell + IVec3::Y)
    && !voxels.solid(cell + IVec3::Y * 2)
}

fn surface(voxels: &Voxels, cell: IVec3) -> f32 {
  (0..8)
    .find(|&rise| !wet(voxels, cell + IVec3::Y * (rise + 1)))
    .map_or(cell.y as f32 + 8.88, |rise| (cell.y + rise) as f32 + 0.88)
}

fn mood(
  roam: &mut Roam,
  breed: Breed,
  swimming: bool,
  fright: Option<Vec2>,
  prey: Option<Vec3>
) -> Mood {
  let expired = roam.timer <= 0.0;
  let next = match (breed, roam.mood, swimming) {
    (Breed::Lizard, ..) if prey.is_some() && roam.stagger <= 0.0 => Mood::Hunt,
    (Breed::Sheep(_), current, _) if !expired => current,
    (Breed::Sheep(_), ..) => match roam.dice() {
      roll if roll < 0.35 => Mood::Graze,
      roll if roll < 0.55 => Mood::Idle,
      _ => Mood::Stroll
    },
    (Breed::Lizard, Mood::Swim | Mood::Landing, false) => {
      roam.ashore = 15.0 + roam.dice() * 25.0;
      Mood::Stroll
    }
    (Breed::Lizard, Mood::Swim, true) if expired => Mood::Landing,
    (Breed::Lizard, current @ (Mood::Swim | Mood::Landing), true) => current,
    (Breed::Lizard, _, true) => Mood::Swim,
    (Breed::Lizard, _, false) if fright.is_some() => Mood::Dash,
    (Breed::Lizard, Mood::Seaward, false) => Mood::Seaward,
    (Breed::Lizard, _, false) if roam.ashore <= 0.0 => Mood::Seaward,
    (Breed::Lizard, current, false) if !expired => current,
    (Breed::Lizard | Breed::Spider, _, false) => match roam.dice() {
      roll if roll < 0.2 => Mood::Dash,
      roll if roll < 0.45 => Mood::Idle,
      _ => Mood::Stroll
    },
    (Breed::Spider, _, true) => Mood::Swim
  };
  if next != roam.mood || expired {
    roam.timer = match next {
      Mood::Dash => 1.2 + roam.dice() * 1.5,
      Mood::Swim => 6.0 + roam.dice() * 10.0,
      Mood::Landing | Mood::Seaward => 30.0,
      _ => 1.5 + roam.dice() * 3.5
    }
  }
  next
}

fn roam(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(Entity, &Controller, &Avatar, &mut Health, &mut Vigour)>,
  mut beasts: Query<(&mut Beast, &mut Roam)>,
  mut knocks: MessageWriter<ToClients<Knock>>
) {
  let dt = time.delta_secs().min(0.1);
  let seed = voxels.seed;
  let spots: Vec<(Entity, Vec3)> =
    players.iter().map(|(entity, _, avatar, ..)| (entity, avatar.at)).collect();
  let watched =
    |at: Vec3| spots.iter().any(|(_, spot)| spot.xz().distance(at.xz()) < SLEEP);
  for (mut beast, mut roam) in beasts.iter_mut().filter(|(beast, _)| watched(beast.at)) {
    let Beast { breed, mut at, mut yaw, .. } = *beast;
    let bulk = breed.bulk();
    for offset in [
      IVec3::ZERO,
      IVec3::X * 3,
      IVec3::X * -3,
      IVec3::Z * 3,
      IVec3::Z * -3,
      IVec3::Y * -3
    ]
    .into_iter()
    {
      voxels.ensure(at.floor().as_ivec3() + offset);
    }
    let (low, high) = bulk.body(at);
    if cells(low, high).any(|cell| voxels.solid(cell)) {
      at.y = at.y.floor() + 1.0
    }
    let body = (at + Vec3::Y * 0.3).floor().as_ivec3();
    let swimming = wet(&voxels, body);
    roam.timer -= dt;
    roam.ashore -= dt;
    roam.shorn -= dt;
    roam.stagger -= dt;
    roam.grudge -= dt;
    roam.bite -= dt;
    let prey = spots
      .iter()
      .filter(|(_, spot)| roam.grudge > 0.0 && spot.distance(at) < CHASE)
      .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)))
      .copied();
    let fright = spots
      .iter()
      .map(|(_, spot)| at.xz() - spot.xz())
      .find(|away| away.length() < SKITTISH && breed == Breed::Lizard && prey.is_none());
    if let Some((victim, spot)) = prey
      && spot.xz().distance(at.xz()) < JAWS
      && (spot.y - at.y).abs() < 1.5
      && roam.bite <= 0.0
      && roam.stagger <= 0.0
      && let Ok((_, controller, _, mut health, mut vigour)) = players.get_mut(victim)
    {
      roam.bite = BITE_EVERY;
      hurt(&mut health, &mut vigour, BITE);
      knocks.write(ToClients {
        targets: SendTargets::Single(controller.client),
        message: Knock(
          (spot - at).with_y(0.0).normalize_or_zero() * KNOCK * 0.7 + Vec3::Y * 4.0
        )
      });
    }
    let was = roam.mood;
    roam.mood = mood(&mut roam, breed, swimming, fright, prey.map(|(_, spot)| spot));
    let turned = roam.mood != was || roam.timer <= 0.0;
    let island = Island::at(seed, roam.home);
    let homeward = island.map(|island| (island.centre - at.xz(), island.radius));
    yaw = match roam.mood {
      Mood::Hunt if let Some((_, spot)) = prey => toward(spot.xz() - at.xz()),
      Mood::Dash if let Some(away) = fright => toward(-away),
      Mood::Landing => seek(&voxels, at, standable)
        .or(homeward.map(|(home, _)| toward(home)))
        .unwrap_or(yaw),
      Mood::Seaward => seek(&voxels, at, wet).unwrap_or(yaw),
      Mood::Stroll | Mood::Swim | Mood::Dash if turned => {
        yaw + (roam.dice() - 0.5) * PI * 1.4
      }
      _ => yaw
    };
    let straying = homeward.is_some_and(|(home, radius)| {
      home.length() > radius * if breed == Breed::Lizard { 1.6 } else { 0.8 }
    });
    if straying
      && matches!(roam.mood, Mood::Stroll | Mood::Swim | Mood::Idle | Mood::Graze)
    {
      yaw = homeward.map_or(yaw, |(home, _)| toward(home))
    }
    let probe = (at + ahead(yaw) * 0.8).floor().as_ivec3();
    let drop = (0..4).all(|depth| !voxels.solid(probe - IVec3::Y * (depth + 1)));
    let flooded = [probe, probe - IVec3::Y].into_iter().any(|cell| wet(&voxels, cell));
    let shy = match breed {
      Breed::Sheep(_) => drop || flooded,
      Breed::Lizard | Breed::Spider => drop && !flooded && !swimming
    };
    if shy && roam.grounded && roam.mood != Mood::Seaward {
      yaw += PI;
      roam.timer = roam.timer.max(1.0)
    }
    let pace = match roam.mood {
      Mood::Stroll if matches!(breed, Breed::Sheep(_)) => GRAZE,
      Mood::Stroll => STROLL,
      Mood::Dash if matches!(breed, Breed::Sheep(_)) => STROLL * 2.0,
      Mood::Dash => DASH,
      Mood::Seaward => DASH * 0.85,
      Mood::Hunt
        if prey.is_some_and(|(_, spot)| spot.xz().distance(at.xz()) < JAWS * 0.8) =>
      {
        0.0
      }
      Mood::Hunt => DASH * 0.8,
      Mood::Swim => PADDLE,
      Mood::Landing => PADDLE * 1.4,
      Mood::Idle | Mood::Graze => 0.0
    };
    let walk = match roam.stagger > 0.0 {
      true => roam.velocity.with_y(0.0) * (1.0 - dt * 3.0),
      false => ahead(yaw) * pace
    };
    let vertical = match swimming && roam.velocity.y <= 3.0 {
      true => (surface(&voxels, body) - 0.25 - at.y) * 5.0,
      false => (roam.velocity.y - GRAVITY * dt).max(-40.0)
    };
    let Marched { at: moved, velocity, blocked, landed } =
      march(&voxels, bulk, at, Vec3::new(walk.x, vertical, walk.z), dt);
    roam.velocity = match blocked && (landed || swimming) {
      true => velocity.with_y(LEAP),
      false => velocity
    };
    roam.grounded = landed;
    let pose = match roam.mood {
      _ if swimming => Pose::Swim,
      Mood::Dash | Mood::Seaward | Mood::Hunt => Pose::Run,
      Mood::Graze => Pose::Graze,
      _ if pace > 0.0 => Pose::Walk,
      _ => Pose::Still
    };
    beast.set_if_neq(Beast { breed, at: moved, yaw: yaw.rem_euclid(TAU), pose });
  }
}

fn shear(
  mut strikes: MessageReader<FromClient<Strike>>,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Inventory))>,
  mut beasts: Query<(&mut Beast, &mut Roam, &mut Health)>,
  mut commands: Commands
) {
  for &FromClient { client_id, message: Strike(target) } in strikes.read() {
    if let Ok((mut beast, mut roam, mut health)) = beasts.get_mut(target)
      && let Some((player, avatar, mut inventory)) =
        player_of(players.iter_mut(), client_id)
      && (avatar.at + Vec3::Y * EYE).distance(beast.at) <= REACH + 1.0
      && health.0 > 0
    {
      commands.entity(target).insert(crate::haunt::Blamed(player));
      let sheep = matches!(beast.breed, Breed::Sheep(_));
      let woolly = sheep && roam.shorn <= 0.0;
      if woolly && inventory.add(Block::Wool) {
        roam.shorn = REGROW
      }
      health.0 = health.0.saturating_sub(PUNCH);
      let away = (beast.at - avatar.at).with_y(0.0).normalize_or_zero();
      roam.velocity = away * 6.0 + Vec3::Y * 4.5;
      roam.stagger = 0.35;
      roam.mood = Mood::Dash;
      roam.timer = 2.5;
      if beast.breed == Breed::Lizard {
        roam.grudge = GRUDGE
      }
      beast.yaw = toward(away.xz());
      if health.0 == 0 {
        if sheep {
          scatter(
            &mut commands,
            Block::Wool,
            1 + u16::from(woolly),
            beast.at + Vec3::Y * 0.5,
            Vec3::Y * 3.0,
            0.4
          )
        }
        commands.entity(target).remove::<Roam>();
        crate::death::slay(&mut commands, target)
      }
    }
  }
}

pub fn struck(beast: &Beast, from: Vec3, toward: Vec3) -> Option<f32> {
  let Bulk { half, tall } = beast.breed.bulk();
  let extent = Vec3::new(half, tall, half) * beast.breed.size();
  ray_box(from, toward, beast.at - extent.with_y(0.0), beast.at + extent.with_y(extent.y))
}

fn prod(
  buttons: Res<ButtonInput<MouseButton>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  beasts: Query<(Entity, &Beast)>,
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
    if let Some((target, _)) = beasts
      .iter()
      .filter_map(|(entity, beast)| {
        struck(beast, from, toward).map(|near| (entity, near))
      })
      .filter(|&(_, near)| near <= blocked)
      .min_by(|a, b| a.1.total_cmp(&b.1))
    {
      strikes.write(Strike(target));
    }
  }
}

#[derive(Resource, Default)]
pub struct Shapes {
  meshes: HashMap<([u32; 5], [u32; 3]), Handle<Mesh>>,
  materials: HashMap<[u8; 3], Handle<StandardMaterial>>,
  hides: HashMap<Breed, Option<(Handle<Mesh>, Handle<SkinnedMeshInverseBindposes>)>>
}

impl Shapes {
  pub fn shaped(
    &mut self,
    meshes: &mut Assets<Mesh>,
    form: Form,
    size: Vec3
  ) -> Handle<Mesh> {
    self
      .meshes
      .entry((form.key(), size.to_array().map(f32::to_bits)))
      .or_insert_with(|| meshes.add(form.mesh(size)))
      .clone()
  }

  pub fn cuboid(&mut self, meshes: &mut Assets<Mesh>, size: Vec3) -> Handle<Mesh> {
    self.shaped(meshes, Form::Block, size)
  }

  pub fn paint(
    &mut self,
    materials: &mut Assets<StandardMaterial>,
    color: [f32; 3]
  ) -> Handle<StandardMaterial> {
    self
      .materials
      .entry(color.map(|channel| (channel * 255.0) as u8))
      .or_insert_with(|| {
        materials.add(StandardMaterial {
          base_color: Color::srgb(color[0], color[1], color[2]),
          perceptual_roughness: 0.95,
          reflectance: 0.2,
          ..default()
        })
      })
      .clone()
  }
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub enum Joint {
  Fixed,
  Hips,
  Trunk,
  Neck,
  Tail,
  TailTip,
  Leg(f32),
  Arm(f32),
  Wing(f32)
}

#[derive(Component)]
pub struct Rest(pub Vec3);

pub struct Bone {
  pub parent: Option<usize>,
  pub joint: Joint,
  pub pivot: Vec3,
  pub parts: Vec<Part>
}

pub fn rig(
  root: Entity,
  bones: &[Bone],
  shapes: &mut Shapes,
  meshes: &mut Assets<Mesh>,
  materials: &mut Assets<StandardMaterial>,
  commands: &mut Commands
) -> Vec<Entity> {
  bones.iter().fold(Vec::new(), |mut spawned: Vec<Entity>, bone| {
    let parent = bone.parent.map_or(root, |index| spawned[index]);
    let entity = commands
      .spawn((
        bone.joint,
        Rest(bone.pivot),
        Transform::from_translation(bone.pivot),
        Visibility::default(),
        ChildOf(parent)
      ))
      .id();
    for &Part { at, size, color, form, turn } in bone.parts.iter() {
      commands.spawn((
        Mesh3d(shapes.shaped(meshes, form, size)),
        MeshMaterial3d(shapes.paint(materials, color)),
        Transform::from_translation(at).with_rotation(turn),
        ChildOf(entity)
      ));
    }
    spawned.push(entity);
    spawned
  })
}

fn bound(bones: &[Bone]) -> Vec<Mat4> {
  bones
    .iter()
    .fold(Vec::new(), |mut placed: Vec<Vec3>, bone| {
      placed.push(bone.pivot + bone.parent.map_or(Vec3::ZERO, |parent| placed[parent]));
      placed
    })
    .into_iter()
    .map(|at| Mat4::from_translation(-at))
    .collect()
}

const BLACK: [f32; 3] = [0.06, 0.05, 0.05];
const AMBER: [f32; 3] = [0.85, 0.6, 0.15];

fn sided(side: f32, parts: Vec<Part>) -> Vec<Part> {
  match side < 0.0 {
    true => parts.into_iter().map(Part::mirrored).collect(),
    false => parts
  }
}

const SHEEP_LEGS: [(f32, f32, f32); 4] =
  [(0.22, -0.345, 0.0), (-0.22, 0.345, 0.0), (-0.22, -0.345, PI), (0.22, 0.345, PI)];
const SHEEP_NECK: Vec3 = Vec3::new(0.0, 0.84, -0.46);
const SHEEP_HIPS: f32 = 0.45;

fn sheep() -> Vec<Bone> {
  [Bone { parent: None, joint: Joint::Fixed, pivot: Vec3::ZERO, parts: vec![] }, Bone {
    parent: Some(0),
    joint: Joint::Neck,
    pivot: SHEEP_NECK,
    parts: vec![]
  }]
  .into_iter()
  .chain(SHEEP_LEGS.map(|(x, z, phase)| Bone {
    parent: Some(0),
    joint: Joint::Leg(phase),
    pivot: Vec3::new(x, SHEEP_HIPS, z),
    parts: vec![]
  }))
  .collect()
}

fn fleeced(fleece: Fleece) -> Mesh {
  let wool = match fleece {
    Fleece::White => [0.9, 0.88, 0.8],
    Fleece::Silver => [0.66, 0.66, 0.62],
    Fleece::Grey => [0.4, 0.4, 0.39],
    Fleece::Black => [0.12, 0.11, 0.1],
    Fleece::Brown => [0.45, 0.3, 0.18],
    Fleece::Pink => [0.94, 0.62, 0.72]
  };
  let face = match fleece {
    Fleece::Black => [0.17, 0.15, 0.13],
    Fleece::Brown => [0.32, 0.22, 0.14],
    _ => [0.25, 0.22, 0.2]
  };
  let hoof = [0.14, 0.12, 0.1];
  let (middle, half) = (Vec3::Y * 0.74, Vec3::new(0.34, 0.27, 0.48));
  let body = Cage::block(half * 2.0, UVec3::new(3, 2, 3)).shaped(|at| {
    let cube = at / half;
    let ball = cube.normalize_or_zero() * cube.abs().max_element();
    let round = cube.lerp(ball, 0.45) * half;
    middle
      + round * Vec3::new(1.0 + at.z * 0.12, 1.0, 1.0)
      + Vec3::Y * (at.z * 0.06).min(0.0)
  });
  let legs = body.faces_where(|centre, normal| {
    normal.y < -0.6 && centre.x.abs() > 0.12 && centre.z.abs() > 0.2
  });
  let legged = legs.iter().fold(body, |cage, &leg| {
    let top = cage.centre(leg);
    let foot = Vec3::new(top.x.signum() * 0.21, 0.0, top.z.signum() * 0.32);
    [(0.38, 0.62), (0.3, 0.72), (0.07, 1.0), (0.0, 1.15)].iter().fold(
      cage,
      |cage, &(y, narrow)| {
        let middle = cage.centre(leg);
        cage.grown(&[leg], foot.with_y(y) - middle, Vec3::new(narrow, 1.0, narrow))
      }
    )
  });
  let front = legged.faces_where(|centre, normal| {
    normal.z < -0.6 && centre.x.abs() < 0.1 && centre.y > 0.75
  })[0];
  let necked = legged
    .grown(&[front], Vec3::new(0.0, 0.07, -0.08), Vec3::new(0.95, 0.95, 1.0))
    .grown(&[front], Vec3::new(0.0, 0.03, -0.17), Vec3::new(1.2, 1.15, 1.0));
  let head_sides = necked.faces_where(|centre, normal| {
    normal.x.abs() > 0.6 && centre.z < -0.6 && centre.y > 0.7
  });
  let muzzled = necked
    .grown(&[front], Vec3::new(0.0, -0.07, -0.17), Vec3::new(0.68, 0.7, 1.0))
    .inset(&head_sides, 0.05);
  let eared = head_sides.iter().fold(muzzled, |cage, &side| {
    let out = cage.normal(side).x.signum();
    cage.grown(&[side], Vec3::new(out * 0.15, -0.02, 0.04), Vec3::new(1.0, 0.3, 0.75))
  });
  let back = eared.faces_where(|centre, normal| {
    normal.z > 0.6 && centre.x.abs() < 0.1 && centre.y > 0.75
  })[0];
  let tailed = eared.grown(&[back], Vec3::new(0.0, -0.08, 0.06), Vec3::splat(0.5));
  let woolly = |at: Vec3| at.y > SHEEP_HIPS - 0.04 && at.z > -0.55;
  let cage = tailed.subdivided(1).displaced(|at, _| {
    woolly(at)
      .then(|| 0.02 + perlin3(0x3001, at.x * 6.0, at.y * 6.0, at.z * 6.0) * 0.06)
      .unwrap_or(0.0)
  });
  let eye = |side: f32| Daub { at: Vec3::new(side * 0.2, 0.97, -0.78), color: AMBER };
  let nostril =
    |side: f32| Daub { at: Vec3::new(side * 0.05, 0.86, -1.02), color: BLACK };
  sculpt(
    &cage,
    |centre, normal| match () {
      _ if centre.y < 0.07 => hoof,
      _ if centre.y < SHEEP_HIPS - 0.06 => face,
      _ if centre.z < -0.6
        && !(normal.y > 0.5 && centre.z > -0.8 && centre.x.abs() < 0.13) =>
      {
        face
      }
      _ if normal.y < -0.5 => wool.map(|channel| channel * 0.85),
      _ => wool
    },
    &[eye(1.0), eye(-1.0), nostril(1.0), nostril(-1.0)],
    |at| match () {
      _ if at.y < SHEEP_HIPS - 0.03 => {
        2 + SHEEP_LEGS
          .iter()
          .position(|&(x, z, _)| {
            x.signum() == at.x.signum() && z.signum() == at.z.signum()
          })
          .unwrap_or(0) as u16
      }
      _ if at.z < SHEEP_NECK.z - 0.03 => 1,
      _ => 0
    }
  )
}

fn lizard() -> Vec<Bone> {
  let green = [0.32, 0.52, 0.2];
  let dark = [0.22, 0.4, 0.15];
  let belly = [0.7, 0.74, 0.42];
  let crest = [0.93, 0.5, 0.12];
  let eye = [0.98, 0.86, 0.2];
  let arm = |x: f32, phase: f32| Bone {
    parent: Some(2),
    joint: Joint::Arm(phase),
    pivot: Vec3::new(x, -0.04, -0.34),
    parts: vec![
      Part::taper(IVec3::NEG_Y, 0.7, [0.0, -0.08, -0.02], [0.065, 0.17, 0.065], green),
      Part::pebble([0.0, -0.17, -0.04], [0.09, 0.045, 0.09], dark),
    ]
  };
  let leg = |x: f32, phase: f32| Bone {
    parent: Some(1),
    joint: Joint::Leg(phase),
    pivot: Vec3::new(x, 0.0, 0.02),
    parts: vec![
      Part::lump([0.0, -0.11, 0.0], [0.11, 0.28, 0.13], green),
      Part::taper(IVec3::NEG_Y, 0.7, [0.0, -0.31, 0.01], [0.07, 0.2, 0.07], green),
      Part::taper(IVec3::NEG_Z, 0.4, [0.0, -0.405, -0.06], [0.11, 0.04, 0.2], dark),
    ]
  };
  let spike = |z: f32, y: f32, height: f32| {
    Part::taper(IVec3::Y, 0.2, [0.0, y, z], [0.035, height, 0.08], crest)
  };
  vec![
    Bone { parent: None, joint: Joint::Fixed, pivot: Vec3::ZERO, parts: vec![] },
    Bone {
      parent: Some(0),
      joint: Joint::Hips,
      pivot: Vec3::new(0.0, 0.42, 0.0),
      parts: vec![]
    },
    Bone {
      parent: Some(1),
      joint: Joint::Trunk,
      pivot: Vec3::ZERO,
      parts: vec![
        Part::lump([0.0, 0.0, -0.2], [0.25, 0.22, 0.52], green),
        Part::lump([0.0, -0.06, -0.2], [0.2, 0.12, 0.42], belly),
        spike(-0.06, 0.13, 0.1),
        spike(-0.2, 0.14, 0.13),
        spike(-0.34, 0.12, 0.1),
      ]
    },
    Bone {
      parent: Some(2),
      joint: Joint::Neck,
      pivot: Vec3::new(0.0, 0.03, -0.42),
      parts: vec![
        Part::lump([0.0, 0.03, -0.1], [0.18, 0.15, 0.23], green),
        Part::taper(IVec3::NEG_Z, 0.55, [0.0, 0.0, -0.25], [0.13, 0.09, 0.15], green),
        Part::pebble([0.075, 0.075, -0.12], [0.05, 0.06, 0.06], eye),
        Part::pebble([-0.075, 0.075, -0.12], [0.05, 0.06, 0.06], eye),
        Part::taper(IVec3::Y, 0.3, [0.0, 0.14, -0.03], [0.035, 0.1, 0.14], crest),
      ]
    },
    Bone {
      parent: Some(2),
      joint: Joint::Tail,
      pivot: Vec3::new(0.0, 0.0, 0.02),
      parts: vec![Part::taper(
        IVec3::Z,
        0.55,
        [0.0, 0.0, 0.2],
        [0.15, 0.13, 0.42],
        green
      )]
    },
    Bone {
      parent: Some(4),
      joint: Joint::TailTip,
      pivot: Vec3::new(0.0, 0.0, 0.4),
      parts: vec![Part::taper(
        IVec3::Z,
        0.15,
        [0.0, 0.0, 0.22],
        [0.085, 0.075, 0.46],
        dark
      )]
    },
    arm(0.13, 0.0),
    arm(-0.13, PI),
    leg(0.1, 0.0),
    leg(-0.1, PI),
  ]
}

fn spider() -> Vec<Bone> {
  let hide = [0.14, 0.11, 0.11];
  let hair = [0.24, 0.19, 0.16];
  let mark = [0.7, 0.12, 0.08];
  let eye = [1.0, 0.2, 0.1];
  let fang = [0.82, 0.78, 0.66];
  let leg = |x: f32, z: f32, phase: f32| Bone {
    parent: Some(2),
    joint: Joint::Leg(phase),
    pivot: Vec3::new(x * 0.16, 0.02, z),
    parts: sided(x, vec![
      Part::taper(IVec3::X, 0.7, [0.2, 0.1, 0.0], [0.44, 0.08, 0.08], hair)
        .turned(0.0, 0.0, 0.3),
      Part::pebble([0.41, 0.17, 0.0], [0.1, 0.1, 0.1], hide),
      Part::taper(IVec3::NEG_Y, 0.3, [0.47, -0.08, 0.0], [0.065, 0.48, 0.065], hide)
        .turned(0.0, 0.0, 0.12),
    ])
  };
  vec![
    Bone { parent: None, joint: Joint::Fixed, pivot: Vec3::ZERO, parts: vec![] },
    Bone {
      parent: Some(0),
      joint: Joint::Hips,
      pivot: Vec3::new(0.0, 0.32, 0.0),
      parts: vec![]
    },
    Bone {
      parent: Some(1),
      joint: Joint::Trunk,
      pivot: Vec3::ZERO,
      parts: vec![
        Part::lump([0.0, 0.04, -0.16], [0.38, 0.25, 0.38], hide),
        Part::lump([0.0, 0.15, 0.27], [0.54, 0.45, 0.6], hair),
        Part::pebble([0.0, 0.34, 0.2], [0.14, 0.08, 0.16], mark),
        Part::pebble([0.0, 0.33, 0.36], [0.2, 0.08, 0.08], mark),
        Part::pebble([0.06, 0.1, -0.33], [0.06, 0.06, 0.04], eye),
        Part::pebble([-0.06, 0.1, -0.33], [0.06, 0.06, 0.04], eye),
        Part::pebble([0.12, 0.07, -0.31], [0.045, 0.045, 0.04], eye),
        Part::pebble([-0.12, 0.07, -0.31], [0.045, 0.045, 0.04], eye),
        Part::taper(IVec3::NEG_Y, 0.3, [0.05, -0.08, -0.33], [0.05, 0.12, 0.05], fang),
        Part::taper(IVec3::NEG_Y, 0.3, [-0.05, -0.08, -0.33], [0.05, 0.12, 0.05], fang),
      ]
    },
    leg(1.0, -0.28, 0.0),
    leg(1.0, -0.18, PI),
    leg(1.0, -0.08, 0.0),
    leg(1.0, 0.02, PI),
    leg(-1.0, -0.28, PI),
    leg(-1.0, -0.18, 0.0),
    leg(-1.0, -0.08, PI),
    leg(-1.0, 0.02, 0.0),
  ]
}

#[derive(Component)]
struct Shown {
  at: Vec3,
  yaw: f32,
  stride: f32,
  pace: f32,
  stance: f32
}

fn dress(
  arrivals: Query<(Entity, &Beast), Without<Shown>>,
  mut shapes: ResMut<Shapes>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut poses: ResMut<Assets<SkinnedMeshInverseBindposes>>,
  mut commands: Commands
) {
  for (entity, beast) in arrivals.iter() {
    commands.entity(entity).insert((
      Shown { at: beast.at, yaw: beast.yaw, stride: 0.0, pace: 0.0, stance: 0.0 },
      Transform::from_translation(beast.at),
      Visibility::default()
    ));
    let bones = match beast.breed {
      Breed::Sheep(_) => sheep(),
      Breed::Lizard => lizard(),
      Breed::Spider => spider()
    };
    let joints =
      rig(entity, &bones, &mut shapes, &mut meshes, &mut materials, &mut commands);
    let hide = shapes
      .hides
      .entry(beast.breed)
      .or_insert_with(|| {
        let mesh = match beast.breed {
          Breed::Sheep(fleece) => Some(fleeced(fleece)),
          _ => None
        };
        mesh.map(|mesh| (meshes.add(mesh), poses.add(bound(&bones))))
      })
      .clone();
    if let Some((mesh, inverse_bindposes)) = hide {
      commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(shapes.paint(&mut materials, [1.0; 3])),
        SkinnedMesh { inverse_bindposes, joints },
        DynamicSkinnedMeshBounds,
        ChildOf(entity)
      ));
    }
  }
}

struct Stance {
  hips: f32,
  trunk: Quat,
  neck: Quat,
  tail: Quat,
  tip: Quat,
  leg: f32,
  arm: (f32, f32, f32)
}

fn stance(breed: Breed, pose: Pose, stride: f32, beat: f32, now: f32) -> Stance {
  let (swing, twice) = (stride.sin(), (stride * 2.0).sin());
  match (breed, pose) {
    (Breed::Sheep(_), pose) => Stance {
      hips: 0.0,
      trunk: Quat::IDENTITY,
      neck: match pose {
        Pose::Graze => Quat::from_rotation_x(-0.95 + (now * 9.0).sin() * 0.06),
        _ => Quat::from_rotation_x(twice * 0.04 * beat)
      },
      tail: Quat::IDENTITY,
      tip: Quat::IDENTITY,
      leg: swing * 0.6 * beat,
      arm: (0.0, 0.0, 0.0)
    },
    (Breed::Spider, pose) => Stance {
      hips: 0.32 + twice.abs() * 0.03 * beat,
      trunk: Quat::from_rotation_x(if pose == Pose::Run { -0.08 } else { 0.0 }),
      neck: Quat::IDENTITY,
      tail: Quat::IDENTITY,
      tip: Quat::IDENTITY,
      leg: (stride * 1.6).sin() * 0.55 * beat,
      arm: (0.0, 0.0, 0.0)
    },
    (Breed::Lizard, Pose::Swim) => {
      let wiggle = (now * 7.0).sin();
      Stance {
        hips: 0.06 + (now * 3.0).sin() * 0.02,
        trunk: Quat::from_rotation_y(wiggle * 0.08),
        neck: Quat::from_rotation_x(0.15),
        tail: Quat::from_rotation_y(-wiggle * 0.5),
        tip: Quat::from_rotation_y(-(now * 7.0 - 1.2).sin() * 0.6),
        leg: -1.3,
        arm: (1.2, 0.0, 0.2)
      }
    }
    (Breed::Lizard, Pose::Run) => {
      let pitch = 0.5 + twice * 0.12;
      Stance {
        hips: 0.44 + twice.abs() * 0.07,
        trunk: Quat::from_rotation_x(pitch) * Quat::from_rotation_z(swing * 0.18),
        neck: Quat::from_rotation_x(-pitch * 0.7 + twice * 0.3),
        tail: Quat::from_rotation_x(-pitch + 0.3) * Quat::from_rotation_y(swing * 0.6),
        tip: Quat::from_rotation_x(0.2)
          * Quat::from_rotation_y((stride - 1.0).sin() * 0.5),
        leg: swing * 1.35,
        arm: (0.2, twice * 1.3, 0.7 + twice * 0.5)
      }
    }
    (Breed::Lizard, _) => {
      let pitch = 0.78 + twice * 0.05 * beat;
      Stance {
        hips: 0.42 + twice.abs() * 0.02 * beat,
        trunk: Quat::from_rotation_x(pitch),
        neck: Quat::from_rotation_x(-pitch * 0.8 + (now * 1.7).sin() * 0.1),
        tail: Quat::from_rotation_x(-pitch - 0.12)
          * Quat::from_rotation_y(swing * 0.25 * beat),
        tip: Quat::from_rotation_y((stride - 1.0).sin() * 0.2 * beat),
        leg: swing * 0.6 * beat,
        arm: (0.3, swing * 0.3 * beat, 0.1)
      }
    }
  }
}

fn animate(
  time: Res<Time>,
  mut beasts: Query<(Entity, &Beast, &mut Shown, &mut Transform)>,
  family: Query<&Children>,
  mut joints: Query<(&Joint, &Rest, &mut Transform), Without<Shown>>
) {
  let (dt, now) = (time.delta_secs(), time.elapsed_secs());
  for (entity, beast, mut shown, mut transform) in beasts.iter_mut() {
    let before = shown.at;
    shown.at = before.lerp(beast.at, (dt * 12.0).min(1.0));
    let turn = (beast.yaw - shown.yaw + PI).rem_euclid(TAU) - PI;
    shown.yaw += turn * (dt * 8.0).min(1.0);
    let pace = (shown.at - before).xz().length() / dt.max(1e-4);
    shown.pace += (pace - shown.pace) * (dt * 6.0).min(1.0);
    shown.stride += shown.pace * dt * 5.0;
    let beat = (shown.pace / 1.2).min(1.0);
    shown.stance += (beat - shown.stance) * (dt * 5.0).min(1.0);
    *transform = Transform::from_translation(shown.at)
      .with_rotation(Quat::from_rotation_y(shown.yaw))
      .with_scale(Vec3::splat(beast.breed.size()));
    let Stance { hips, trunk, neck, tail, tip, leg, arm } =
      stance(beast.breed, beast.pose, shown.stride, shown.stance, now);
    for child in family.iter_descendants(entity) {
      if let Ok((joint, rest, mut transform)) = joints.get_mut(child) {
        let (translation, rotation) = match *joint {
          Joint::Fixed => (rest.0, Quat::IDENTITY),
          Joint::Hips => (rest.0.with_y(hips), Quat::IDENTITY),
          Joint::Trunk => (rest.0, trunk),
          Joint::Neck => (rest.0, neck),
          Joint::Tail => (rest.0, tail),
          Joint::TailTip => (rest.0, tip),
          Joint::Leg(phase) => (rest.0, Quat::from_rotation_x(leg * phase.cos())),
          Joint::Wing(_) => (rest.0, Quat::IDENTITY),
          Joint::Arm(phase) => {
            let (base, swing, spread) = arm;
            (
              rest.0,
              Quat::from_rotation_z(rest.0.x.signum() * spread)
                * Quat::from_rotation_x(base + swing * phase.cos())
            )
          }
        };
        transform.translation = translation;
        transform.rotation = rotation
      }
    }
  }
}

fn parade(pilot: Res<Pilot>, mut done: Local<bool>, mut commands: Commands) {
  if !*done && pilot.grounded {
    *done = true;
    let beasts = [
      Breed::Sheep(Fleece::White),
      Breed::Sheep(Fleece::Black),
      Breed::Sheep(Fleece::Brown),
      Breed::Lizard,
      Breed::Spider
    ];
    let birds =
      [bird::Breed::Swan, bird::Breed::Raven, bird::Breed::Gull, bird::Breed::Parrot];
    let count = (beasts.len() + birds.len()) as f32;
    let (forward, right) = (ahead(pilot.yaw), Quat::from_rotation_y(pilot.yaw) * Vec3::X);
    let spot = |index: usize| {
      pilot.at + forward * 3.4 + right * (index as f32 - (count - 1.0) * 0.5) * 0.95
    };
    let yaw = pilot.yaw + PI - 0.7;
    for (index, &breed) in beasts.iter().enumerate() {
      commands.spawn(Beast { breed, at: spot(index), yaw, pose: Pose::Still });
    }
    for (index, &breed) in birds.iter().enumerate() {
      commands.spawn(bird::Bird {
        breed,
        at: spot(beasts.len() + index),
        yaw,
        stance: bird::Stance::Rest
      });
    }
  }
}

pub struct Beasts;

impl Plugin for Beasts {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Herds>()
      .init_resource::<Shapes>()
      .add_systems(
        Update,
        (muster, roam, disperse)
          .chain()
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
      )
      .add_systems(PreUpdate, shear.after(ServerSystems::Receive).run_if(authority))
      .add_systems(Update, (dress, animate).chain().run_if(plays))
      .add_systems(Update, prod.run_if(plays).run_if(resource_exists::<Pilot>))
      .add_systems(
        Update,
        parade.run_if(|| opts().parade).run_if(resource_exists::<Pilot>)
      );
  }
}

#[cfg(test)]
mod tests {
  use {super::*, crate::sculpt::portrait};

  #[test]
  #[ignore]
  fn hides() {
    let (size, views) = (300, [200.0, 160.0, 90.0, 20.0]);
    let hides = [Fleece::White, Fleece::Black, Fleece::Brown].map(fleeced);
    let rows: Vec<Vec<Vec<[u8; 3]>>> = hides
      .iter()
      .flat_map(|hide| {
        [false, true]
          .map(|wire| views.iter().map(|&yaw| portrait(hide, yaw, size, wire)).collect())
      })
      .collect();
    let image: Vec<u8> = rows
      .iter()
      .flat_map(|row| {
        (0..size).flat_map(move |y| {
          row.iter().flat_map(move |tile| {
            tile[y * size..(y + 1) * size].iter().flatten().copied()
          })
        })
      })
      .collect();
    let mut file =
      format!("P6 {} {} 255\n", size * views.len(), size * rows.len()).into_bytes();
    file.extend(image);
    std::fs::create_dir_all("screenshots").unwrap();
    std::fs::write("screenshots/hides.ppm", file).unwrap()
  }
}
