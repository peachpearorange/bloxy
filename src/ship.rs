use {crate::{authority::Controller,
             beast::{Grain, Shapes},
             block::Block,
             boat::planked,
             folk::{Raider, Vigour, hurt},
             generate,
             island::SEA,
             noise::unit,
             opts::opts,
             protocol::*,
             stream::Palette,
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            image::ImageSampler,
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}},
     bevy_replicon::prelude::*,
     std::f32::consts::{PI, TAU}};

const WATERLINE: f32 = SEA as f32 + 0.88;
const DECK: f32 = 0.3;
const VANISH: f32 = 220.0;
const PASSING: f32 = 100.0;
const LINGER: f32 = 10.0;
const CANNON_RANGE: f32 = 60.0;
const CANNON_EVERY: f32 = 4.5;
const BALL_GRAVITY: f32 = 20.0;
const BLAST: f32 = 3.5;

impl Band {
  fn length(self) -> f32 {
    match self {
      Band::Trader => 2.6,
      Band::Pirate => 5.0,
      Band::Viking => 6.0
    }
  }

  fn beam(self) -> f32 {
    match self {
      Band::Trader => 1.1,
      Band::Pirate => 1.8,
      Band::Viking => 1.4
    }
  }

  fn cruise(self) -> f32 {
    match self {
      Band::Trader => 3.2,
      Band::Pirate => 4.5,
      Band::Viking => 5.0
    }
  }

  fn seats(self) -> &'static [Vec3] {
    match self {
      Band::Trader => &TRADER_SEATS,
      Band::Pirate => &PIRATE_SEATS,
      Band::Viking => &VIKING_SEATS
    }
  }
}

const TRADER_SEATS: [Vec3; 2] = [Vec3::new(0.0, DECK, -1.0), Vec3::new(0.3, DECK, 1.3)];
const PIRATE_SEATS: [Vec3; 4] = [
  Vec3::new(-0.8, DECK, -2.6),
  Vec3::new(0.8, DECK, -1.2),
  Vec3::new(-0.8, DECK, 0.4),
  Vec3::new(0.0, 1.2, 3.8)
];
const VIKING_SEATS: [Vec3; 5] = [
  Vec3::new(-0.6, DECK, -3.6),
  Vec3::new(0.6, DECK, -1.8),
  Vec3::new(-0.6, DECK, 0.0),
  Vec3::new(0.6, DECK, 1.8),
  Vec3::new(-0.6, DECK, 3.6)
];

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn toward(offset: Vec2) -> f32 { (-offset.x).atan2(-offset.y) }

fn wrap(angle: f32) -> f32 { (angle + PI).rem_euclid(TAU) - PI }

fn deep(seed: u32, spot: Vec2) -> bool {
  generate::height(seed, spot.x.floor() as i32, spot.y.floor() as i32) < SEA - 1
}

#[derive(Component)]
pub struct Voyage {
  landing: Vec2,
  speed: f32,
  age: f32,
  reload: f32,
  beached: bool,
  leaving: bool
}

#[derive(Component)]
pub struct Crew {
  ship: Entity,
  seat: Vec3
}

#[derive(Component)]
struct Flight {
  velocity: Vec3,
  age: f32
}

#[derive(Resource)]
struct Tides {
  trader: f32,
  raid: f32,
  rolls: u32,
  first: Option<Band>
}

impl Tides {
  fn roll(&mut self, seed: u32) -> f32 {
    self.rolls += 1;
    unit(seed ^ 0xF1EE7, self.rolls as i32, 0, 0)
  }
}

impl Default for Tides {
  fn default() -> Self {
    let raid = opts().raid.as_ref();
    Tides {
      trader: opts().trader.unwrap_or(45.0),
      raid: raid.map_or(300.0, |(at, _)| *at),
      rolls: 0,
      first: raid.map(|(_, band)| match band.as_str() {
        "vikings" => Band::Viking,
        _ => Band::Pirate
      })
    }
  }
}

fn launch(
  commands: &mut Commands,
  band: Band,
  at: Vec2,
  yaw: f32,
  landing: Vec2,
  luck: u32
) {
  info!("{band:?} ship sets sail at {at} toward {landing}");
  let ship = commands
    .spawn((
      Replicated,
      Ship { band, at: Vec3::new(at.x, WATERLINE, at.y), yaw, aim: 0.0 },
      Voyage {
        landing,
        speed: band.cruise(),
        age: 0.0,
        reload: 2.0,
        beached: false,
        leaving: false
      }
    ))
    .id();
  band.seats().iter().enumerate().for_each(|(index, &seat)| {
    commands.spawn((
      Replicated,
      Folk {
        band,
        at: Vec3::new(at.x, WATERLINE, at.y) + Quat::from_rotation_y(yaw) * seat,
        yaw,
        luck: luck.wrapping_add((index as u32).wrapping_mul(0x9E37_79B9)),
        blows: 0,
        aboard: true
      },
      Health(band.health()),
      Crew { ship, seat }
    ));
  })
}

fn passage(seed: u32, tides: &mut Tides, from: Vec2) -> Option<(Vec2, f32)> {
  (0..12).find_map(|_| {
    let direction = Vec2::from_angle(tides.roll(seed) * TAU);
    let side = direction.perp() * (16.0 + tides.roll(seed) * 14.0);
    let start = from - direction * PASSING + side;
    let course = (0..=20).map(|step| start + direction * step as f32 * 10.0);
    (course.filter(|&spot| deep(seed, spot)).count() >= 18 && deep(seed, start))
      .then(|| (start, toward(direction)))
  })
}

fn approach(seed: u32, from: Vec2) -> Option<(Vec2, Vec2)> {
  (0..16)
    .filter_map(|step| {
      let direction = Vec2::from_angle(step as f32 * TAU / 16.0);
      (1..45)
        .map(|stride| stride as f32 * 2.0)
        .find(|&distance| deep(seed, from + direction * distance))
        .filter(|&coast| deep(seed, from + direction * (coast + 60.0)))
        .map(|coast| (coast, direction))
    })
    .min_by(|a, b| a.0.total_cmp(&b.0))
    .map(|(coast, direction)| {
      (from + direction * (coast + 60.0), from + direction * (coast - 3.0).max(0.0))
    })
}

fn summon(
  time: Res<Time>,
  voxels: Res<Voxels>,
  mut tides: ResMut<Tides>,
  players: Query<&Avatar, With<Controller>>,
  ships: Query<&Ship>,
  mut commands: Commands
) {
  let (now, seed) = (time.elapsed_secs(), voxels.seed);
  let avatars: Vec<Vec3> = players.iter().map(|avatar| avatar.at).collect();
  let pick = |tides: &mut Tides, among: &[Vec3]| {
    (!among.is_empty())
      .then(|| among[(tides.roll(seed) * among.len() as f32) as usize % among.len()])
  };
  if now >= tides.trader {
    tides.trader = now + 100.0 + tides.roll(seed) * 100.0;
    if let Some(player) = pick(&mut tides, &avatars)
      && !ships
        .iter()
        .any(|ship| ship.band == Band::Trader && ship.at.distance(player) < 160.0)
      && let Some((start, yaw)) = passage(seed, &mut tides, player.xz())
    {
      let luck = (tides.roll(seed) * u32::MAX as f32) as u32;
      launch(&mut commands, Band::Trader, start, yaw, player.xz(), luck)
    }
  }
  if now >= tides.raid {
    let ashore: Vec<Vec3> = avatars
      .iter()
      .copied()
      .filter(|at| {
        at.y > SEA as f32 + 1.0 && generate::height(seed, at.x as i32, at.z as i32) >= SEA
      })
      .collect();
    let raided = pick(&mut tides, &ashore)
      .filter(|player| {
        !ships.iter().any(|ship| ship.band.hostile() && ship.at.distance(*player) < 200.0)
      })
      .and_then(|player| approach(seed, player.xz()));
    tides.raid = match raided {
      Some((start, landing)) => {
        let band = tides.first.take().unwrap_or(match tides.roll(seed) < 0.5 {
          true => Band::Pirate,
          false => Band::Viking
        });
        let luck = (tides.roll(seed) * u32::MAX as f32) as u32;
        launch(&mut commands, band, start, toward(landing - start), landing, luck);
        now + 360.0 + tides.roll(seed) * 240.0
      }
      None => now + 30.0
    }
  }
}

fn sail(
  time: Res<Time>,
  voxels: Res<Voxels>,
  players: Query<&Avatar, With<Controller>>,
  mut ships: Query<(Entity, &mut Ship, &mut Voyage)>,
  mut crew: Query<(Entity, &mut Folk, &Crew)>,
  raiders: Query<&Raider>,
  mut commands: Commands
) {
  let (dt, seed) = (time.delta_secs().min(0.1), voxels.seed);
  ships.iter_mut().for_each(|(entity, mut ship, mut voyage)| {
    let Ship { band, at, mut yaw, mut aim } = *ship;
    voyage.age += dt;
    let nearest = players
      .iter()
      .map(|avatar| avatar.at)
      .min_by(|a, b| a.distance(at).total_cmp(&b.distance(at)));
    let distance = nearest.map_or(f32::INFINITY, |player| player.distance(at));
    let manned = crew.iter().any(|(_, _, seat)| seat.ship == entity)
      || raiders.iter().any(|raider| raider.ship == entity);
    if voyage.beached && !manned {
      voyage.beached = false;
      voyage.leaving = true
    }
    let bow = |yaw: f32, by: f32| at.xz() + ahead(yaw).xz() * (band.length() + by);
    let homing = band.hostile() && !voyage.leaving && !voyage.beached;
    let shoal = !deep(seed, bow(yaw, 1.5));
    if homing && (shoal || voyage.age > 150.0) {
      voyage.beached = true;
      crew.iter_mut().filter(|(_, _, seat)| seat.ship == entity).for_each(
        |(sailor, mut folk, _)| {
          folk.aboard = false;
          commands
            .entity(sailor)
            .remove::<Crew>()
            .insert(Raider::overboard(entity, ahead(yaw)));
        }
      )
    }
    let desired = match (band.hostile(), voyage.leaving) {
      (true, false) => toward(voyage.landing - at.xz()),
      (true, true) => toward(at.xz() - voyage.landing),
      (false, _) => yaw
    };
    let near_landing = homing && at.xz().distance(voyage.landing) < 30.0;
    let clear =
      |yaw: f32| [2.0, 6.0, 10.0].into_iter().all(|by| deep(seed, bow(yaw, by)));
    let heading = match near_landing {
      true => desired,
      false => [0.0, 0.35, -0.35, 0.7, -0.7, 1.1, -1.1, 1.6, -1.6, PI]
        .into_iter()
        .map(|swerve| desired + swerve)
        .find(|&yaw| clear(yaw))
        .unwrap_or(desired)
    };
    yaw += wrap(heading - yaw).clamp(-0.5 * dt, 0.5 * dt);
    let wanted = match () {
      () if voyage.beached => 0.0,
      () if band == Band::Trader && distance < LINGER => 0.0,
      () => band.cruise()
    };
    voyage.speed += (wanted - voyage.speed) * (dt * 0.8).min(1.0);
    let moved = at + ahead(yaw) * voyage.speed * dt;
    if band == Band::Pirate
      && !voyage.leaving
      && let Some(target) = nearest.filter(|_| distance < CANNON_RANGE)
    {
      aim = wrap(toward(target.xz() - at.xz()) - yaw);
      voyage.reload -= dt;
      if voyage.reload <= 0.0 {
        voyage.reload = CANNON_EVERY;
        let muzzle =
          moved + Quat::from_rotation_y(yaw) * Vec3::new(0.0, 0.8, -band.length() + 0.8);
        let offset = target + Vec3::Y - muzzle;
        let flight = (offset.xz().length() / 22.0).clamp(0.6, 3.0);
        commands.spawn((Replicated, Cannonball(muzzle), Flight {
          velocity: offset / flight + Vec3::Y * 0.5 * BALL_GRAVITY * flight,
          age: 0.0
        }));
      }
    }
    match voyage.age > 30.0 && distance > VANISH {
      true => {
        commands.entity(entity).despawn();
        crew
          .iter()
          .filter(|(_, _, seat)| seat.ship == entity)
          .for_each(|(sailor, ..)| commands.entity(sailor).despawn())
      }
      false => {
        ship.set_if_neq(Ship { band, at: moved, yaw: yaw.rem_euclid(TAU), aim });
      }
    }
  })
}

fn ferry(ships: Query<&Ship>, mut crew: Query<(&mut Folk, &Crew)>) {
  crew.iter_mut().for_each(|(mut folk, crew)| {
    if let Ok(ship) = ships.get(crew.ship) {
      let placed = Folk {
        at: ship.at + Quat::from_rotation_y(ship.yaw) * crew.seat,
        yaw: ship.yaw,
        ..*folk
      };
      folk.set_if_neq(placed);
    }
  })
}

fn fly(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut balls: Query<(Entity, &mut Cannonball, &mut Flight)>,
  mut players: Query<(&Avatar, &mut Health, &mut Vigour)>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  balls.iter_mut().for_each(|(entity, mut ball, mut flight)| {
    flight.age += dt;
    flight.velocity.y -= BALL_GRAVITY * dt;
    let at = ball.0 + flight.velocity * dt;
    let cell = at.floor().as_ivec3();
    let landed =
      voxels.ensure(cell).solid() || voxels.block(cell).is_some_and(Block::fluid);
    let hit =
      players.iter().any(|(avatar, ..)| (avatar.at + Vec3::Y * 0.9).distance(at) < 1.0);
    match landed || hit || flight.age > 6.0 {
      true => {
        players.iter_mut().for_each(|(avatar, mut health, mut vigour)| {
          let distance = (avatar.at + Vec3::Y * 0.9).distance(at);
          if distance < BLAST {
            hurt(&mut health, &mut vigour, (7.0 * (1.0 - distance / BLAST)).ceil() as u8)
          }
        });
        commands.entity(entity).despawn()
      }
      false => ball.0 = at
    }
  })
}

const SKULL: [&str; 16] = [
  "................",
  ".....111111.....",
  "....11111111....",
  "....11111111....",
  "....1..11..1....",
  "....1..11..1....",
  "....11111111....",
  ".....111111.....",
  "......1.1.1.....",
  "................",
  "..11........11..",
  "....11....11....",
  "......1111......",
  "....11....11....",
  "..11........11..",
  "................"
];

fn canvas(band: Band) -> Image {
  let texel = |x: u32, y: u32| -> [f32; 3] {
    let weave = 0.94 + unit(0x5A11, x as i32, y as i32, 0) * 0.06;
    let [r, g, b] = match band {
      Band::Trader if y % 8 == 0 => [0.8, 0.74, 0.6],
      Band::Trader => [0.93, 0.89, 0.77],
      Band::Pirate => {
        let (u, v) = (x.wrapping_sub(8), y.wrapping_sub(6));
        match SKULL.get(v as usize).and_then(|row| row.as_bytes().get(u as usize)) {
          Some(b'1') => [0.9, 0.88, 0.82],
          _ => [0.09, 0.09, 0.1]
        }
      }
      Band::Viking if (x / 4) % 2 == 0 => [0.74, 0.12, 0.1],
      Band::Viking => [0.92, 0.88, 0.8]
    };
    [r * weave, g * weave, b * weave]
  };
  let mut image = Image::new(
    Extent3d { width: 32, height: 32, depth_or_array_layers: 1 },
    TextureDimension::D2,
    (0..32 * 32)
      .flat_map(|index| {
        let [r, g, b] = texel(index % 32, index / 32);
        [r, g, b, 1.0].map(|channel| (channel * 255.0) as u8)
      })
      .collect(),
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  );
  image.sampler = ImageSampler::nearest();
  image
}

fn hull(band: Band) -> Vec<(Vec3, Vec3)> {
  let (length, beam) = (band.length(), band.beam());
  let wall = match band {
    Band::Viking => 0.45,
    _ => 0.7
  };
  let (top, keel) = (DECK + wall, -0.55);
  let mut slabs = vec![
    (Vec3::new(-beam, keel, -length), Vec3::new(beam, DECK, length)),
    (Vec3::new(-beam - 0.12, keel + 0.2, -length), Vec3::new(-beam, top, length)),
    (Vec3::new(beam, keel + 0.2, -length), Vec3::new(beam + 0.12, top, length)),
    (
      Vec3::new(-beam * 0.6, keel + 0.1, -length - 0.9),
      Vec3::new(beam * 0.6, top, -length)
    ),
    (
      Vec3::new(-beam * 0.25, keel + 0.3, -length - 1.5),
      Vec3::new(beam * 0.25, top + 0.1, -length - 0.9)
    ),
    (Vec3::new(-beam, keel + 0.2, length), Vec3::new(beam, top, length + 0.3)),
  ];
  match band {
    Band::Pirate => slabs.extend([
      (Vec3::new(-beam, DECK, length - 2.2), Vec3::new(beam, 1.2, length)),
      (Vec3::new(-0.35, DECK, -length + 0.4), Vec3::new(0.35, 0.55, -length + 1.2))
    ]),
    Band::Viking => slabs.extend([
      (Vec3::new(-0.15, top, -length - 1.6), Vec3::new(0.15, top + 0.7, -length - 1.2)),
      (
        Vec3::new(-0.15, top + 0.5, -length - 2.0),
        Vec3::new(0.15, top + 1.2, -length - 1.5)
      ),
      (
        Vec3::new(-0.2, top + 1.1, -length - 2.6),
        Vec3::new(0.2, top + 1.45, -length - 1.8)
      ),
      (Vec3::new(-0.15, top, length + 0.3), Vec3::new(0.15, top + 1.2, length + 0.6)),
      (
        Vec3::new(-0.15, top + 1.0, length + 0.6),
        Vec3::new(0.15, top + 1.3, length + 1.0)
      )
    ]),
    Band::Trader => slabs.extend([
      (Vec3::new(-0.5, DECK, 0.2), Vec3::new(0.3, DECK + 0.6, 0.9)),
      (Vec3::new(0.35, DECK, 0.0), Vec3::new(0.85, DECK + 0.45, 0.5))
    ])
  }
  slabs
}

fn tint(band: Band) -> [f32; 3] {
  match band {
    Band::Trader => [1.0, 1.0, 1.0],
    Band::Pirate => [0.42, 0.36, 0.34],
    Band::Viking => [0.9, 0.8, 0.66]
  }
}

#[derive(Component)]
struct Cannon;

#[derive(Component)]
struct Shown {
  at: Vec3,
  yaw: f32,
  phase: f32
}

#[derive(Resource)]
struct Rigging {
  ball: Handle<Mesh>,
  sails: [Handle<StandardMaterial>; 3]
}

fn rope(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let mut sail = |band: Band| {
    materials.add(StandardMaterial {
      base_color_texture: Some(images.add(canvas(band))),
      perceptual_roughness: 0.95,
      double_sided: true,
      cull_mode: None,
      ..default()
    })
  };
  commands.insert_resource(Rigging {
    ball: meshes.add(Sphere::new(0.18)),
    sails: [sail(Band::Trader), sail(Band::Pirate), sail(Band::Viking)]
  })
}

fn rig(
  ships: Query<(Entity, &Ship), Without<Shown>>,
  rigging: Res<Rigging>,
  palette: Res<Palette>,
  mut shapes: ResMut<Shapes>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  ships.iter().for_each(|(entity, ship)| {
    let band = ship.band;
    let (length, beam) = (band.length(), band.beam());
    let mast = match band {
      Band::Trader => 4.2,
      Band::Pirate => 7.5,
      Band::Viking => 6.0
    };
    let (sail_wide, sail_tall) = match band {
      Band::Trader => (2.4, 2.8),
      Band::Pirate => (4.4, 4.2),
      Band::Viking => (4.0, 3.4)
    };
    let cannon = (band == Band::Pirate).then(|| {
      commands
        .spawn((
          Cannon,
          Transform::from_xyz(0.0, 0.75, -length + 0.8),
          Visibility::default(),
          ChildOf(entity)
        ))
        .id()
    });
    let mut part = |size: Vec3,
                    color: [f32; 3],
                    sail: Option<Handle<StandardMaterial>>,
                    at: Vec3,
                    parent: Entity| {
      commands.spawn((
        Mesh3d(shapes.cuboid(&mut meshes, size)),
        MeshMaterial3d(sail.unwrap_or_else(|| {
          shapes.paint(&mut materials, &mut images, color, Grain::Plain)
        })),
        Transform::from_translation(at),
        ChildOf(parent)
      ));
    };
    let spar = [0.38, 0.26, 0.15];
    part(
      Vec3::new(0.2, mast, 0.2),
      spar,
      None,
      Vec3::new(0.0, DECK + mast / 2.0, -0.3),
      entity
    );
    part(
      Vec3::new(sail_wide + 0.4, 0.14, 0.14),
      spar,
      None,
      Vec3::new(0.0, DECK + mast - 0.4, -0.15),
      entity
    );
    part(
      Vec3::new(sail_wide, sail_tall, 0.04),
      spar,
      Some(rigging.sails[band as usize].clone()),
      Vec3::new(0.0, DECK + mast - 0.5 - sail_tall / 2.0, -0.1),
      entity
    );
    if band == Band::Viking {
      let paints =
        [[0.7, 0.15, 0.12], [0.85, 0.7, 0.2], [0.18, 0.3, 0.6], [0.9, 0.88, 0.8]];
      (0..((length * 2.0 - 2.0) / 1.1) as i32).for_each(|index| {
        let z = -length + 1.2 + index as f32 * 1.1;
        [-1.0, 1.0].into_iter().for_each(|side: f32| {
          part(
            Vec3::new(0.06, 0.62, 0.62),
            paints[(index as usize + usize::from(side > 0.0)) % 4],
            None,
            Vec3::new(side * (beam + 0.16), DECK + 0.2, z),
            entity
          )
        })
      });
      part(
        Vec3::new(0.42, 0.08, 0.2),
        [0.95, 0.85, 0.2],
        None,
        Vec3::new(0.0, DECK + 1.75, -length - 2.3),
        entity
      );
    }
    if let Some(cannon) = cannon {
      part(
        Vec3::new(0.3, 0.3, 1.3),
        [0.12, 0.12, 0.13],
        None,
        Vec3::new(0.0, 0.0, -0.4),
        cannon
      );
      part(
        Vec3::new(0.5, 0.5, 0.5),
        [0.1, 0.1, 0.11],
        None,
        Vec3::new(0.0, DECK + mast + 0.1, -0.3),
        entity
      );
    }
    commands.entity(entity).insert((
      Shown { at: ship.at, yaw: ship.yaw, phase: (entity.index_u32() % 7) as f32 },
      Mesh3d(meshes.add(planked(&hull(band), tint(band)))),
      MeshMaterial3d(palette.solid.clone()),
      Transform::from_translation(ship.at).with_rotation(Quat::from_rotation_y(ship.yaw)),
      Visibility::default()
    ));
  })
}

fn bob(
  time: Res<Time>,
  mut ships: Query<(&Ship, &mut Shown, &mut Transform, &Children)>,
  mut cannons: Query<&mut Transform, (With<Cannon>, Without<Shown>)>
) {
  let (dt, now) = (time.delta_secs(), time.elapsed_secs());
  ships.iter_mut().for_each(|(ship, mut shown, mut transform, children)| {
    shown.at = shown.at.lerp(ship.at, (dt * 8.0).min(1.0));
    shown.yaw += wrap(ship.yaw - shown.yaw) * (dt * 6.0).min(1.0);
    let phase = now + shown.phase;
    *transform =
      Transform::from_translation(shown.at + Vec3::Y * (phase * 1.3).sin() * 0.05)
        .with_rotation(
          Quat::from_rotation_y(shown.yaw)
            * Quat::from_rotation_z((phase * 1.1).sin() * 0.035)
            * Quat::from_rotation_x((phase * 0.8).sin() * 0.02)
        );
    children.iter().for_each(|child| {
      if let Ok(mut cannon) = cannons.get_mut(child) {
        cannon.rotation = Quat::from_rotation_y(ship.aim.clamp(-1.2, 1.2))
      }
    })
  })
}

#[derive(Component)]
struct Blast(f32);

fn fire(
  rigging: Res<Rigging>,
  time: Res<Time>,
  mut shapes: ResMut<Shapes>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut images: ResMut<Assets<Image>>,
  mut balls: Query<(Entity, &Cannonball, Option<&mut Transform>)>,
  mut commands: Commands
) {
  let blend = (time.delta_secs() * 20.0).min(1.0);
  balls.iter_mut().for_each(|(entity, ball, transform)| match transform {
    Some(mut transform) => {
      transform.translation = transform.translation.lerp(ball.0, blend)
    }
    None => {
      commands.entity(entity).insert((
        Mesh3d(rigging.ball.clone()),
        MeshMaterial3d(shapes.paint(
          &mut materials,
          &mut images,
          [0.05, 0.05, 0.06],
          Grain::Plain
        )),
        Transform::from_translation(ball.0),
        Visibility::default()
      ));
    }
  })
}

fn burst(
  gone: On<Remove, Cannonball>,
  balls: Query<&Transform>,
  rigging: Option<Res<Rigging>>,
  materials: Option<ResMut<Assets<StandardMaterial>>>,
  mut commands: Commands
) {
  if let Ok(transform) = balls.get(gone.entity)
    && let Some(mut materials) = materials
    && let Some(rigging) = rigging
  {
    commands.spawn((
      Blast(0.0),
      Mesh3d(rigging.ball.clone()),
      MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.6, 0.2, 0.9),
        emissive: LinearRgba::rgb(6.0, 2.5, 0.6),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
      })),
      Transform::from_translation(transform.translation)
    ));
  }
}

fn fade(
  time: Res<Time>,
  mut blasts: Query<(
    Entity,
    &mut Blast,
    &mut Transform,
    &MeshMaterial3d<StandardMaterial>
  )>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  blasts.iter_mut().for_each(|(entity, mut blast, mut transform, material)| {
    blast.0 += time.delta_secs();
    let progress = blast.0 / 0.45;
    transform.scale = Vec3::splat(2.0 + progress * 12.0);
    if let Some(mut material) = materials.get_mut(&material.0) {
      material.base_color = Color::srgba(1.0, 0.6, 0.2, (0.9 * (1.0 - progress)).max(0.0))
    }
    if progress >= 1.0 {
      materials.remove(&material.0);
      commands.entity(entity).despawn()
    }
  })
}

pub struct Ships;

impl Plugin for Ships {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Tides>()
      .add_systems(
        Update,
        (summon, sail, ferry, fly)
          .chain()
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
      )
      .add_systems(Startup, rope.run_if(plays))
      .add_observer(burst)
      .add_systems(Update, (rig, bob, fire, fade).chain().run_if(plays));
  }
}
