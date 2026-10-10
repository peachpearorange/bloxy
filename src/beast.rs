use {crate::{authority::{Controller, player_of},
             block::Block,
             folk::ray_box,
             generate,
             island::{Island, Kind, SEA},
             menu::Menu,
             noise::{hash, unit},
             player::{Bulk, Marched, Pilot, captured, cells, march},
             protocol::{Avatar, Beast, Breed, EYE, Fleece, Inventory, Pose, REACH,
                        Strike, authority, plays},
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            image::ImageSampler,
            platform::collections::{HashMap, HashSet},
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     std::f32::consts::{PI, TAU}};

const WAKE: f32 = 110.0;
const SLEEP: f32 = 170.0;
const GRAVITY: f32 = 30.0;
const LEAP: f32 = 7.5;
const GRAZE: f32 = 1.1;
const STROLL: f32 = 1.7;
const DASH: f32 = 6.5;
const PADDLE: f32 = 1.6;
const SKITTISH: f32 = 6.0;
const REGROW: f32 = 30.0;

impl Breed {
  fn bulk(self) -> Bulk {
    match self {
      Breed::Sheep(_) => Bulk { half: 0.35, tall: 1.0 },
      Breed::Lizard => Bulk { half: 0.3, tall: 0.95 }
    }
  }

  fn size(self) -> f32 {
    match self {
      Breed::Sheep(_) => 1.0,
      Breed::Lizard => 1.35
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
  Seaward
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
  luck: u32
}

impl Roam {
  fn dice(&mut self) -> f32 {
    self.luck = hash(self.luck, 0x6B, 0x21, 0x3F);
    unit(self.luck, 0, 0, 0)
  }
}

#[derive(Resource, Default)]
struct Herds(HashSet<IVec2>);

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
  players
    .iter()
    .flat_map(|avatar| {
      Island::near(seed, avatar.at)
        .into_iter()
        .filter(|island| avatar.at.xz().distance(island.centre) < WAKE)
    })
    .collect::<Vec<_>>()
    .into_iter()
    .filter(|island| herds.0.insert(island.cell))
    .for_each(|island| {
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
      flock.chain(swimmers).enumerate().for_each(|(index, (breed, at))| {
        let luck = key ^ (index as u32).wrapping_mul(0x9E37);
        commands.spawn((
          Replicated,
          Beast { breed, at, yaw: unit(luck, 7, 0, 0) * TAU, pose: Pose::Still },
          Roam {
            home: island.cell,
            velocity: Vec3::ZERO,
            grounded: false,
            mood: if breed == Breed::Lizard { Mood::Swim } else { Mood::Idle },
            timer: 1.0 + unit(luck, 8, 0, 0) * 3.0,
            ashore: 0.0,
            shorn: 0.0,
            luck
          }
        ));
      })
    })
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

fn mood(roam: &mut Roam, breed: Breed, swimming: bool, fright: Option<Vec2>) -> Mood {
  let expired = roam.timer <= 0.0;
  let next = match (breed, roam.mood, swimming) {
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
    (Breed::Lizard, _, false) => match roam.dice() {
      roll if roll < 0.2 => Mood::Dash,
      roll if roll < 0.45 => Mood::Idle,
      _ => Mood::Stroll
    }
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
  players: Query<&Avatar>,
  mut beasts: Query<(&mut Beast, &mut Roam)>
) {
  let dt = time.delta_secs().min(0.1);
  let seed = voxels.seed;
  let watched =
    |at: Vec3| players.iter().any(|avatar| avatar.at.xz().distance(at.xz()) < SLEEP);
  beasts.iter_mut().filter(|(beast, _)| watched(beast.at)).for_each(
    |(mut beast, mut roam)| {
      let Beast { breed, mut at, mut yaw, .. } = *beast;
      let bulk = breed.bulk();
      [
        IVec3::ZERO,
        IVec3::X * 3,
        IVec3::X * -3,
        IVec3::Z * 3,
        IVec3::Z * -3,
        IVec3::Y * -3
      ]
      .into_iter()
      .for_each(|offset| {
        voxels.ensure(at.floor().as_ivec3() + offset);
      });
      let (low, high) = bulk.body(at);
      if cells(low, high).any(|cell| voxels.solid(cell)) {
        at.y = at.y.floor() + 1.0
      }
      let body = (at + Vec3::Y * 0.3).floor().as_ivec3();
      let swimming = wet(&voxels, body);
      let fright = players
        .iter()
        .map(|avatar| at.xz() - avatar.at.xz())
        .find(|away| away.length() < SKITTISH && breed == Breed::Lizard);
      roam.timer -= dt;
      roam.ashore -= dt;
      roam.shorn -= dt;
      let was = roam.mood;
      roam.mood = mood(&mut roam, breed, swimming, fright);
      let turned = roam.mood != was || roam.timer <= 0.0;
      let island = Island::at(seed, roam.home);
      let homeward = island.map(|island| (island.centre - at.xz(), island.radius));
      yaw = match roam.mood {
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
        Breed::Lizard => drop && !flooded && !swimming
      };
      if shy && roam.grounded && roam.mood != Mood::Seaward {
        yaw += PI;
        roam.timer = roam.timer.max(1.0)
      }
      let pace = match roam.mood {
        Mood::Stroll if matches!(breed, Breed::Sheep(_)) => GRAZE,
        Mood::Stroll => STROLL,
        Mood::Dash => DASH,
        Mood::Seaward => DASH * 0.85,
        Mood::Swim => PADDLE,
        Mood::Landing => PADDLE * 1.4,
        Mood::Idle | Mood::Graze => 0.0
      };
      let walk = ahead(yaw) * pace;
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
        Mood::Dash | Mood::Seaward => Pose::Run,
        Mood::Graze => Pose::Graze,
        _ if pace > 0.0 => Pose::Walk,
        _ => Pose::Still
      };
      beast.set_if_neq(Beast { breed, at: moved, yaw: yaw.rem_euclid(TAU), pose });
    }
  )
}

fn shear(
  mut strikes: MessageReader<FromClient<Strike>>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  mut beasts: Query<(&Beast, &mut Roam)>
) {
  strikes.read().for_each(|&FromClient { client_id, message: Strike(target) }| {
    if let Ok((beast, mut roam)) = beasts.get_mut(target)
      && matches!(beast.breed, Breed::Sheep(_))
      && let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && (avatar.at + Vec3::Y * EYE).distance(beast.at) <= REACH + 1.0
    {
      if roam.shorn <= 0.0 && inventory.add(Block::Wool) {
        roam.shorn = REGROW
      }
      roam.mood = Mood::Stroll;
      roam.timer = 2.0
    }
  })
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
  meshes: HashMap<[u32; 3], Handle<Mesh>>,
  materials: HashMap<([u8; 3], Grain), Handle<StandardMaterial>>,
  grains: HashMap<Grain, Handle<Image>>
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Grain {
  Plain,
  Wool,
  Scales
}

impl Grain {
  fn texel(self, x: u32, y: u32) -> f32 {
    let noise = unit(self as u32 * 0x77 + 3, x as i32, y as i32, 0);
    match self {
      Grain::Plain => 1.0,
      Grain::Wool => {
        let curl = unit(0x3A, (x / 2) as i32, (y / 2) as i32, 0);
        0.78 + curl * 0.14 + noise * 0.08
      }
      Grain::Scales => match (x + (y / 2 % 2) * 2) % 4 == 0 || y % 2 == 0 && noise < 0.3 {
        true => 0.8,
        false => 0.93 + noise * 0.07
      }
    }
  }

  fn image(self) -> Image {
    let mut image = Image::new(
      Extent3d { width: 16, height: 16, depth_or_array_layers: 1 },
      TextureDimension::D2,
      (0..256)
        .flat_map(|index| {
          let shade = (self.texel(index % 16, index / 16) * 255.0) as u8;
          [shade, shade, shade, 255]
        })
        .collect(),
      TextureFormat::Rgba8UnormSrgb,
      RenderAssetUsages::default()
    );
    image.sampler = ImageSampler::nearest();
    image
  }
}

impl Shapes {
  pub fn cuboid(&mut self, meshes: &mut Assets<Mesh>, size: Vec3) -> Handle<Mesh> {
    self
      .meshes
      .entry(size.to_array().map(f32::to_bits))
      .or_insert_with(|| meshes.add(Cuboid::from_size(size)))
      .clone()
  }

  pub fn paint(
    &mut self,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    color: [f32; 3],
    grain: Grain
  ) -> Handle<StandardMaterial> {
    let key = (color.map(|channel| (channel * 255.0) as u8), grain);
    let texture = (grain != Grain::Plain).then(|| {
      self.grains.entry(grain).or_insert_with(|| images.add(grain.image())).clone()
    });
    self
      .materials
      .entry(key)
      .or_insert_with(|| {
        materials.add(StandardMaterial {
          base_color: Color::srgb(color[0], color[1], color[2]),
          base_color_texture: texture,
          perceptual_roughness: 0.9,
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
  pub boxes: Vec<(Vec3, Vec3, [f32; 3], Grain)>
}

pub fn rig(
  root: Entity,
  bones: &[Bone],
  shapes: &mut Shapes,
  meshes: &mut Assets<Mesh>,
  materials: &mut Assets<StandardMaterial>,
  images: &mut Assets<Image>,
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
    bone.boxes.iter().for_each(|&(centre, size, color, grain)| {
      commands.spawn((
        Mesh3d(shapes.cuboid(meshes, size)),
        MeshMaterial3d(shapes.paint(materials, images, color, grain)),
        Transform::from_translation(centre),
        ChildOf(entity)
      ));
    });
    spawned.push(entity);
    spawned
  })
}

const BLACK: [f32; 3] = [0.06, 0.05, 0.05];

fn sheep(fleece: Fleece) -> Vec<Bone> {
  let wool = match fleece {
    Fleece::White => [0.93, 0.92, 0.88],
    Fleece::Silver => [0.68, 0.68, 0.66],
    Fleece::Grey => [0.4, 0.4, 0.41],
    Fleece::Black => [0.11, 0.1, 0.1],
    Fleece::Brown => [0.45, 0.29, 0.17],
    Fleece::Pink => [0.96, 0.62, 0.74]
  };
  let face = match fleece {
    Fleece::Black => [0.16, 0.14, 0.13],
    Fleece::Brown => [0.3, 0.2, 0.13],
    _ => [0.86, 0.77, 0.68]
  };
  let leg = |x: f32, z: f32, phase: f32| Bone {
    parent: Some(0),
    joint: Joint::Leg(phase),
    pivot: Vec3::new(x, 0.45, z),
    boxes: vec![
      (Vec3::new(0.0, -0.225, 0.0), Vec3::new(0.13, 0.45, 0.13), face, Grain::Plain),
      (Vec3::new(0.0, -0.06, 0.0), Vec3::new(0.17, 0.14, 0.17), wool, Grain::Wool),
    ]
  };
  vec![
    Bone {
      parent: None,
      joint: Joint::Fixed,
      pivot: Vec3::ZERO,
      boxes: vec![(
        Vec3::new(0.0, 0.68, 0.0),
        Vec3::new(0.62, 0.5, 0.95),
        wool,
        Grain::Wool
      )]
    },
    Bone {
      parent: Some(0),
      joint: Joint::Neck,
      pivot: Vec3::new(0.0, 0.8, -0.42),
      boxes: vec![
        (Vec3::new(0.0, 0.03, -0.2), Vec3::new(0.3, 0.3, 0.34), face, Grain::Plain),
        (Vec3::new(0.0, 0.2, -0.16), Vec3::new(0.34, 0.08, 0.3), wool, Grain::Wool),
        (Vec3::new(0.09, 0.07, -0.375), Vec3::new(0.06, 0.06, 0.02), BLACK, Grain::Plain),
        (
          Vec3::new(-0.09, 0.07, -0.375),
          Vec3::new(0.06, 0.06, 0.02),
          BLACK,
          Grain::Plain
        ),
        (Vec3::new(0.19, 0.11, -0.12), Vec3::new(0.1, 0.05, 0.07), face, Grain::Plain),
        (Vec3::new(-0.19, 0.11, -0.12), Vec3::new(0.1, 0.05, 0.07), face, Grain::Plain),
      ]
    },
    leg(0.18, -0.3, 0.0),
    leg(-0.18, 0.3, 0.0),
    leg(-0.18, -0.3, PI),
    leg(0.18, 0.3, PI),
  ]
}

fn lizard() -> Vec<Bone> {
  let green = [0.3, 0.56, 0.2];
  let belly = [0.68, 0.76, 0.42];
  let crest = [0.95, 0.52, 0.12];
  let eye = [0.98, 0.86, 0.2];
  let skin = |centre: Vec3, size: Vec3| (centre, size, green, Grain::Scales);
  let arm = |x: f32, phase: f32| Bone {
    parent: Some(2),
    joint: Joint::Arm(phase),
    pivot: Vec3::new(x, -0.04, -0.34),
    boxes: vec![
      skin(Vec3::new(0.0, -0.08, -0.02), Vec3::new(0.05, 0.16, 0.05)),
      skin(Vec3::new(0.0, -0.17, -0.04), Vec3::new(0.07, 0.03, 0.07)),
    ]
  };
  let leg = |x: f32, phase: f32| Bone {
    parent: Some(1),
    joint: Joint::Leg(phase),
    pivot: Vec3::new(x, 0.0, 0.02),
    boxes: vec![
      skin(Vec3::new(0.0, -0.12, 0.0), Vec3::new(0.08, 0.26, 0.09)),
      skin(Vec3::new(0.0, -0.32, 0.0), Vec3::new(0.06, 0.2, 0.06)),
      skin(Vec3::new(0.0, -0.405, -0.06), Vec3::new(0.09, 0.03, 0.18)),
    ]
  };
  vec![
    Bone { parent: None, joint: Joint::Fixed, pivot: Vec3::ZERO, boxes: vec![] },
    Bone {
      parent: Some(0),
      joint: Joint::Hips,
      pivot: Vec3::new(0.0, 0.42, 0.0),
      boxes: vec![]
    },
    Bone {
      parent: Some(1),
      joint: Joint::Trunk,
      pivot: Vec3::ZERO,
      boxes: vec![
        skin(Vec3::new(0.0, 0.0, -0.2), Vec3::new(0.22, 0.2, 0.46)),
        (Vec3::new(0.0, -0.09, -0.2), Vec3::new(0.18, 0.04, 0.4), belly, Grain::Scales),
        (Vec3::new(0.0, 0.16, -0.18), Vec3::new(0.03, 0.12, 0.34), crest, Grain::Plain),
      ]
    },
    Bone {
      parent: Some(2),
      joint: Joint::Neck,
      pivot: Vec3::new(0.0, 0.03, -0.42),
      boxes: vec![
        skin(Vec3::new(0.0, 0.03, -0.1), Vec3::new(0.17, 0.14, 0.22)),
        skin(Vec3::new(0.0, 0.0, -0.24), Vec3::new(0.12, 0.09, 0.1)),
        (Vec3::new(0.09, 0.07, -0.1), Vec3::new(0.03, 0.05, 0.05), eye, Grain::Plain),
        (Vec3::new(-0.09, 0.07, -0.1), Vec3::new(0.03, 0.05, 0.05), eye, Grain::Plain),
        (Vec3::new(0.0, 0.14, -0.02), Vec3::new(0.03, 0.08, 0.12), crest, Grain::Plain),
      ]
    },
    Bone {
      parent: Some(2),
      joint: Joint::Tail,
      pivot: Vec3::new(0.0, 0.0, 0.02),
      boxes: vec![skin(Vec3::new(0.0, 0.0, 0.2), Vec3::new(0.14, 0.12, 0.4))]
    },
    Bone {
      parent: Some(4),
      joint: Joint::TailTip,
      pivot: Vec3::new(0.0, 0.0, 0.4),
      boxes: vec![skin(Vec3::new(0.0, 0.0, 0.22), Vec3::new(0.08, 0.07, 0.45))]
    },
    arm(0.13, 0.0),
    arm(-0.13, PI),
    leg(0.1, 0.0),
    leg(-0.1, PI),
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
  mut images: ResMut<Assets<Image>>,
  mut commands: Commands
) {
  arrivals.iter().for_each(|(entity, beast)| {
    commands.entity(entity).insert((
      Shown { at: beast.at, yaw: beast.yaw, stride: 0.0, pace: 0.0, stance: 0.0 },
      Transform::from_translation(beast.at),
      Visibility::default()
    ));
    let bones = match beast.breed {
      Breed::Sheep(fleece) => sheep(fleece),
      Breed::Lizard => lizard()
    };
    rig(
      entity,
      &bones,
      &mut shapes,
      &mut meshes,
      &mut materials,
      &mut images,
      &mut commands
    );
  })
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
  beasts.iter_mut().for_each(|(entity, beast, mut shown, mut transform)| {
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
    family.iter_descendants(entity).for_each(|child| {
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
    })
  })
}

pub struct Beasts;

impl Plugin for Beasts {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Herds>()
      .init_resource::<Shapes>()
      .add_systems(
        Update,
        (muster, roam).chain().run_if(authority).run_if(resource_exists::<Voxels>)
      )
      .add_systems(PreUpdate, shear.after(ServerSystems::Receive).run_if(authority))
      .add_systems(Update, (dress, animate).chain().run_if(plays))
      .add_systems(Update, prod.run_if(plays).run_if(resource_exists::<Pilot>));
  }
}
