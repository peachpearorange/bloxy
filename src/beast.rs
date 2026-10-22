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
             protocol::{Avatar, Band, Beast, Breed, EYE, Fleece, Folk, Health,
                        Inventory, Knock, Pose, REACH, Strike, authority, plays},
             sculpt::{Daub, merged, sculpt},
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

const SPIDER_HIDE: [f32; 3] = [0.15, 0.11, 0.1];
const SPIDER_HAIR: [f32; 3] = [0.26, 0.19, 0.15];
const SPIDER_MARK: [f32; 3] = [0.72, 0.13, 0.08];
const SPIDER_EYE: [f32; 3] = [1.0, 0.25, 0.12];
const FANG: [f32; 3] = [0.84, 0.8, 0.68];
const SPIDER_LEGS: [f32; 4] = [-0.28, -0.18, -0.08, 0.02];

fn spidery() -> Mesh {
  let body = Cage::rounded(
    Vec3::new(0.0, 0.47, 0.27),
    Vec3::new(0.52, 0.44, 0.58),
    UVec3::splat(3),
    0.6
  )
  .joined(Cage::rounded(
    Vec3::new(0.0, 0.37, -0.17),
    Vec3::new(0.32, 0.2, 0.34),
    UVec3::new(2, 2, 2),
    0.55
  ))
  .subdivided(1);
  let eye = |x: f32, y: f32| Daub { at: Vec3::new(x, y, -0.36), color: SPIDER_EYE };
  let hide = sculpt(
    &body,
    |centre, normal| match () {
      _ if centre.z > 0.05
        && normal.y > 0.35
        && ((centre.x.abs() < 0.05 && centre.z < 0.4)
          || ((centre.z - 0.12) * 14.0).sin().abs() > 0.93) =>
      {
        SPIDER_MARK
      }
      _ if centre.z > 0.0 => SPIDER_HAIR,
      _ => SPIDER_HIDE
    },
    &[eye(0.035, 0.42), eye(-0.035, 0.42), eye(0.07, 0.4), eye(-0.07, 0.4)],
    |_| 2
  );
  let piece =
    |cage: Cage, color: [f32; 3], joint: u16| sculpt(&cage, |_, _| color, &[], |_| joint);
  let fangs = [1.0, -1.0].map(|side| {
    piece(
      Cage::tube(
        &[
          (Vec3::new(side * 0.04, 0.33, -0.34), Vec2::splat(0.025)),
          (Vec3::new(side * 0.045, 0.27, -0.37), Vec2::splat(0.02)),
          (Vec3::new(side * 0.03, 0.22, -0.35), Vec2::splat(0.003))
        ],
        4
      ),
      FANG,
      2
    )
  });
  let bristles = (0..9).map(|tuft| {
    let turn = tuft as f32 / 9.0 * TAU;
    let root = Vec3::new(
      turn.cos() * 0.2,
      0.62 + turn.sin().abs() * 0.02,
      0.27 + turn.sin() * 0.24
    );
    let out = (root - Vec3::new(0.0, 0.42, 0.27)).normalize();
    piece(
      Cage::spike(root, root + out * 0.09 + Vec3::Y * 0.03, Vec2::splat(0.018)),
      SPIDER_HIDE,
      2
    )
  });
  let spinneret = piece(
    Cage::spike(
      Vec3::new(0.0, 0.45, 0.54),
      Vec3::new(0.0, 0.41, 0.62),
      Vec2::splat(0.03)
    ),
    SPIDER_HIDE,
    2
  );
  let legs = [1.0f32, -1.0].into_iter().enumerate().flat_map(|(flank, side)| {
    SPIDER_LEGS.into_iter().enumerate().map(move |(rank, z)| {
      let root = Vec3::new(side * 0.16, 0.34, z);
      let splay = (rank as f32 - 1.5) * 0.42;
      let out = Vec3::new(side * splay.cos(), 0.0, splay.sin());
      let path = [
        (0.0, 0.0, 0.045),
        (0.12, 0.08, 0.04),
        (0.26, 0.16, 0.034),
        (0.36, 0.2, 0.042),
        (0.44, 0.05, 0.03),
        (0.5, -0.15, 0.022),
        (0.53, -0.33, 0.004)
      ]
      .map(|(reach, rise, thick)| {
        (root + out * reach + Vec3::Y * rise, Vec2::splat(thick))
      });
      sculpt(
        &Cage::tube(&path, 5),
        |centre, _| {
          let reach = (centre - root).dot(out);
          match () {
            _ if reach > 0.5 => SPIDER_HIDE.map(|channel| channel * 0.5),
            _ if (0.32..0.4).contains(&reach) || (0.1..0.15).contains(&reach) => {
              SPIDER_MARK
            }
            _ => SPIDER_HAIR
          }
        },
        &[],
        |_| 3 + (flank * 4 + rank) as u16
      )
    })
  });
  merged([hide, spinneret].into_iter().chain(fangs).chain(bristles).chain(legs))
}

const LIZARD_GREEN: [f32; 3] = [0.34, 0.54, 0.21];
const LIZARD_DARK: [f32; 3] = [0.21, 0.38, 0.14];
const LIZARD_BELLY: [f32; 3] = [0.74, 0.76, 0.45];
const LIZARD_CREST: [f32; 3] = [0.93, 0.5, 0.12];
const LIZARD_EYE: [f32; 3] = [0.98, 0.86, 0.2];
const LIZARD_CLAW: [f32; 3] = [0.85, 0.8, 0.6];

fn scaly() -> Mesh {
  let trunk = Cage::rounded(
    Vec3::new(0.0, 0.42, -0.2),
    Vec3::new(0.26, 0.22, 0.52),
    UVec3::new(3, 2, 4),
    0.45
  );
  let middle = |cage: &Cage, facing: f32| {
    cage.faces_where(|centre, normal| normal.z * facing > 0.6 && centre.x.abs() < 0.05)
  };
  let below = |cage: &Cage, front: bool| {
    cage.faces_where(|centre, normal| {
      normal.y < -0.6
        && centre.x.abs() > 0.05
        && if front { centre.z < -0.33 } else { centre.z > -0.07 }
    })
  };
  let (head, tail) = (middle(&trunk, -1.0), middle(&trunk, 1.0));
  let (arms, legs) = (below(&trunk, true), below(&trunk, false));
  let headed = trunk.grown_along(&head, &[
    (Vec3::new(0.0, 0.04, -0.08), Vec3::new(1.5, 0.8, 1.0)),
    (Vec3::new(0.0, 0.03, -0.12), Vec3::new(1.3, 1.0, 1.0)),
    (Vec3::new(0.0, -0.02, -0.13), Vec3::new(0.62, 0.6, 1.0)),
    (Vec3::new(0.0, -0.01, -0.05), Vec3::new(0.6, 0.6, 1.0))
  ]);
  let tailed = headed.grown_along(&tail, &[
    (Vec3::new(0.0, 0.0, 0.12), Vec3::new(1.2, 0.85, 1.0)),
    (Vec3::new(0.0, -0.01, 0.13), Vec3::new(0.85, 0.85, 1.0)),
    (Vec3::new(0.0, -0.01, 0.13), Vec3::new(0.8, 0.8, 1.0)),
    (Vec3::new(0.0, -0.01, 0.13), Vec3::new(0.75, 0.75, 1.0)),
    (Vec3::new(0.0, 0.0, 0.13), Vec3::new(0.65, 0.65, 1.0)),
    (Vec3::new(0.0, 0.0, 0.13), Vec3::new(0.2, 0.25, 1.0))
  ]);
  let limbed = |cage: Cage, faces: &[usize], steps: &[(Vec3, Vec3)]| {
    faces.iter().fold(cage, |cage, &face| {
      let side = cage.centre(face).x.signum();
      cage.grown_along(
        &[face],
        &steps
          .iter()
          .map(|&(toward, scale)| (toward * Vec3::new(side, 1.0, 1.0), scale))
          .collect::<Vec<_>>()
      )
    })
  };
  let armed = limbed(tailed, &arms, &[
    (Vec3::new(0.06, -0.05, 0.0), Vec3::new(0.85, 1.0, 0.85)),
    (Vec3::new(0.01, -0.1, -0.01), Vec3::new(0.8, 1.0, 0.8)),
    (Vec3::new(0.0, -0.035, -0.035), Vec3::new(1.4, 1.0, 1.6))
  ]);
  let legged = limbed(armed, &legs, &[
    (Vec3::new(0.03, -0.1, 0.0), Vec3::new(1.2, 1.0, 1.2)),
    (Vec3::new(0.0, -0.13, 0.02), Vec3::new(0.65, 1.0, 0.65)),
    (Vec3::new(0.0, -0.06, -0.06), Vec3::new(1.3, 1.0, 2.2))
  ]);
  let joint = |at: Vec3| match () {
    _ if at.y < 0.3 && at.z < -0.25 => {
      if at.x > 0.0 {
        6
      } else {
        7
      }
    }
    _ if at.y < 0.3 => {
      if at.x > 0.0 {
        8
      } else {
        9
      }
    }
    _ if at.z < -0.47 => 3,
    _ if at.z > 0.45 => 5,
    _ if at.z > 0.07 => 4,
    _ => 2
  };
  let body = legged.subdivided(1);
  let eye =
    |side: f32| Daub { at: Vec3::new(side * 0.12, 0.53, -0.6), color: LIZARD_EYE };
  let nostril = |side: f32| Daub {
    at: Vec3::new(side * 0.025, 0.47, -0.84),
    color: LIZARD_DARK.map(|c| c * 0.4)
  };
  let hide = sculpt(
    &body,
    |centre, normal| match () {
      _ if centre.y < 0.3 && normal.y < -0.5 => LIZARD_DARK,
      _ if normal.y < -0.35 => LIZARD_BELLY,
      _ if normal.y > 0.4 && ((centre.z + 1.0) * 9.0).rem_euclid(3.0) < 1.0 => {
        LIZARD_DARK
      }
      _ => LIZARD_GREEN
    },
    &[eye(1.0), eye(-1.0), nostril(1.0), nostril(-1.0)],
    joint
  );
  let fin = |z: f32, y: f32, tall: f32, lean: f32| {
    let root = Vec3::new(0.0, y, z);
    sculpt(
      &Cage::spike(root, root + Vec3::new(0.0, tall, lean), Vec2::new(0.012, 0.045)),
      |_, _| LIZARD_CREST,
      &[],
      |_| joint(root)
    )
  };
  let claws = [(true, 1.0), (true, -1.0), (false, 1.0), (false, -1.0)]
    .into_iter()
    .flat_map(|(arm, side)| {
      let (x, y, z, reach) = match arm {
        true => (0.15, 0.21, -0.48, 0.06),
        false => (0.12, 0.01, -0.12, 0.07)
      };
      [-0.025, 0.0, 0.025].map(|spread| {
        let root = Vec3::new(side * (x + spread), y, z);
        sculpt(
          &Cage::spike(
            root,
            root + Vec3::new(side * spread, -0.015, -reach),
            Vec2::splat(0.012)
          ),
          |_, _| LIZARD_CLAW,
          &[],
          |_| {
            if arm {
              if side > 0.0 { 6 } else { 7 }
            } else if side > 0.0 {
              8
            } else {
              9
            }
          }
        )
      })
    });
  merged(
    [
      hide,
      fin(-0.08, 0.52, 0.07, 0.03),
      fin(-0.2, 0.53, 0.09, 0.03),
      fin(-0.32, 0.52, 0.07, 0.03),
      fin(-0.58, 0.57, 0.1, 0.04),
      fin(0.16, 0.49, 0.06, 0.03),
      fin(0.3, 0.47, 0.045, 0.03)
    ]
    .into_iter()
    .chain(claws)
  )
}

fn lizard() -> Vec<Bone> {
  let bone = |parent: Option<usize>, joint: Joint, pivot: [f32; 3]| Bone {
    parent,
    joint,
    pivot: Vec3::from(pivot),
    parts: vec![]
  };
  vec![
    bone(None, Joint::Fixed, [0.0; 3]),
    bone(Some(0), Joint::Hips, [0.0, 0.42, 0.0]),
    bone(Some(1), Joint::Trunk, [0.0; 3]),
    bone(Some(2), Joint::Neck, [0.0, 0.03, -0.42]),
    bone(Some(2), Joint::Tail, [0.0, 0.0, 0.02]),
    bone(Some(4), Joint::TailTip, [0.0, 0.0, 0.4]),
    bone(Some(2), Joint::Arm(0.0), [0.13, -0.04, -0.34]),
    bone(Some(2), Joint::Arm(PI), [-0.13, -0.04, -0.34]),
    bone(Some(1), Joint::Leg(0.0), [0.1, 0.0, 0.02]),
    bone(Some(1), Joint::Leg(PI), [-0.1, 0.0, 0.02]),
  ]
}

fn spider() -> Vec<Bone> {
  let bone = |parent: Option<usize>, joint: Joint, pivot: Vec3| Bone {
    parent,
    joint,
    pivot,
    parts: vec![]
  };
  [
    bone(None, Joint::Fixed, Vec3::ZERO),
    bone(Some(0), Joint::Hips, Vec3::Y * 0.32),
    bone(Some(1), Joint::Trunk, Vec3::ZERO)
  ]
  .into_iter()
  .chain([1.0f32, -1.0].into_iter().flat_map(|side| {
    SPIDER_LEGS.into_iter().enumerate().map(move |(rank, z)| {
      let phase = if (rank % 2 == 0) == (side > 0.0) { 0.0 } else { PI };
      bone(Some(2), Joint::Leg(phase), Vec3::new(side * 0.16, 0.02, z))
    })
  }))
  .collect()
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
          Breed::Sheep(fleece) => fleeced(fleece),
          Breed::Lizard => scaly(),
          Breed::Spider => spidery()
        };
        Some((meshes.add(mesh), poses.add(bound(&bones))))
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
    let folk = [
      (Band::Skeleton, 0),
      (Band::Skeleton, 1),
      (Band::Thrall, 2),
      (Band::Warden, 0),
      (Band::Imp, 0),
      (Band::Pirate, 0),
      (Band::Pirate, 5),
      (Band::Viking, 2),
      (Band::Viking, 3)
    ];
    let (forward, right) = (ahead(pilot.yaw), Quat::from_rotation_y(pilot.yaw) * Vec3::X);
    let spot = |index: usize| {
      let (row, column) = ((index / 5) as f32, (index % 5) as f32);
      pilot.at + forward * (3.2 + row * 2.0) + right * (column - 2.0) * 1.15
    };
    let yaw = pilot.yaw + PI - 0.7;
    for (index, &breed) in beasts.iter().enumerate() {
      commands.spawn(Beast { breed, at: spot(index), yaw, pose: Pose::Still });
    }
    for (index, &(band, luck)) in folk.iter().enumerate() {
      let at = spot(beasts.len() + index);
      commands.spawn((
        Folk { band, at, yaw, luck, blows: 0, aboard: false },
        Health(band.health())
      ));
    }
    for (index, &breed) in birds.iter().enumerate() {
      commands.spawn(bird::Bird {
        breed,
        at: spot(beasts.len() + folk.len() + index),
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
  use {super::*, crate::sculpt::sheet};

  #[test]
  #[ignore]
  fn hides() {
    sheet(
      &[fleeced(Fleece::White), fleeced(Fleece::Brown), scaly(), spidery()],
      "screenshots/hides.ppm"
    )
  }
}
