use {crate::{island::{Island, Kind},
             noise::{fbm2, hash, unit},
             opts::opts,
             player::{Eye, Pilot},
             protocol::{authority, plays},
             settings::Settings,
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            light::NotShadowCaster,
            mesh::{Indices, PrimitiveTopology},
            pbr::DistanceFog,
            prelude::*},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

const CLOUD_HEIGHT: f32 = 168.0;
const CLOUD_CELL: f32 = 12.0;
const CLOUD_THICK: f32 = 4.0;
const CLOUD_SPAN: i32 = 56;
const DRIFT: f32 = 1.6;
const DROPS: usize = 700;
const SHOWER: f32 = 22.0;
const RAIN_FALL: f32 = 16.0;
const SNOW_FALL: f32 = 2.4;
const SUN: f32 = bevy::light::light_consts::lux::RAW_SUNLIGHT;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sky {
  Clear,
  Cloudy,
  Rain,
  Storm
}

impl Sky {
  fn cover(self) -> f32 {
    match self {
      Sky::Clear => 0.32,
      Sky::Cloudy => 0.55,
      Sky::Rain => 0.74,
      Sky::Storm => 0.86
    }
  }

  fn wet(self) -> f32 {
    match self {
      Sky::Rain => 0.7,
      Sky::Storm => 1.0,
      _ => 0.0
    }
  }

  fn sun(self) -> f32 {
    match self {
      Sky::Clear => 1.0,
      Sky::Cloudy => 0.6,
      Sky::Rain => 0.3,
      Sky::Storm => 0.16
    }
  }

  fn after(self, roll: f32) -> Sky {
    let odds = match self {
      Sky::Clear => [0.55, 0.9, 1.0],
      Sky::Cloudy => [0.4, 0.75, 0.95],
      Sky::Rain => [0.15, 0.55, 0.8],
      Sky::Storm => [0.05, 0.35, 0.85]
    };
    [Sky::Clear, Sky::Cloudy, Sky::Rain]
      .into_iter()
      .zip(odds)
      .find(|&(_, below)| roll < below)
      .map_or(Sky::Storm, |(sky, _)| sky)
  }

  fn named(name: &str) -> Option<Sky> {
    match name {
      "clear" => Some(Sky::Clear),
      "cloudy" => Some(Sky::Cloudy),
      "rain" => Some(Sky::Rain),
      "storm" => Some(Sky::Storm),
      _ => None
    }
  }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Weather {
  pub sky: Sky,
  pub spell: u32
}

#[derive(Component)]
struct Spell {
  left: f32
}

fn forecast(mut commands: Commands) {
  let sky = opts().weather.as_deref().and_then(Sky::named).unwrap_or(Sky::Clear);
  commands.spawn((Replicated, Weather { sky, spell: 0 }, Spell { left: 150.0 }));
}

fn turn(time: Res<Time>, mut weathers: Query<(&mut Weather, &mut Spell)>) {
  weathers.iter_mut().for_each(|(mut weather, mut spell)| {
    spell.left -= time.delta_secs();
    if spell.left <= 0.0 && opts().weather.is_none() {
      let luck = hash(0x5C1E5, weather.spell as i32, 0, 0);
      let sky = weather.sky.after(unit(luck, 1, 0, 0));
      spell.left = 90.0 + unit(luck, 2, 0, 0) * 150.0;
      *weather = Weather { sky, spell: weather.spell + 1 }
    }
  })
}

#[derive(Resource)]
struct Climate {
  cover: f32,
  wet: f32,
  sun: f32,
  flash: f32,
  luck: u32,
  built: Option<(IVec2, f32)>
}

impl Climate {
  fn dice(&mut self) -> f32 {
    self.luck = hash(self.luck, 0x2B, 0x57, 0x11);
    unit(self.luck, 0, 0, 0)
  }
}

#[derive(Component)]
struct Clouds;

#[derive(Component)]
struct Drop {
  floor: f32,
  phase: f32,
  snow: bool
}

#[derive(Component)]
struct Bolt {
  left: f32
}

#[derive(Resource)]
struct Gear {
  rain: Handle<StandardMaterial>,
  snow: Handle<StandardMaterial>,
  streak: Handle<Mesh>,
  flake: Handle<Mesh>,
  bolt: Handle<StandardMaterial>,
  segment: Handle<Mesh>,
  clouds: Handle<StandardMaterial>
}

fn gather(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let gear = Gear {
    rain: materials.add(StandardMaterial {
      base_color: Color::srgba(0.72, 0.8, 0.92, 0.5),
      alpha_mode: AlphaMode::Blend,
      unlit: true,
      ..default()
    }),
    snow: materials.add(StandardMaterial {
      base_color: Color::srgba(1.0, 1.0, 1.0, 0.92),
      alpha_mode: AlphaMode::Blend,
      unlit: true,
      ..default()
    }),
    streak: meshes.add(Cuboid::new(0.025, 0.7, 0.025)),
    flake: meshes.add(Cuboid::new(0.08, 0.08, 0.08)),
    bolt: materials.add(StandardMaterial {
      base_color: Color::srgb(0.9, 0.92, 1.0),
      emissive: LinearRgba::rgb(30.0, 32.0, 48.0),
      unlit: true,
      ..default()
    }),
    segment: meshes.add(Cuboid::new(0.5, 0.5, 1.0)),
    clouds: materials.add(StandardMaterial {
      base_color: Color::srgba(1.0, 1.0, 1.0, 0.85),
      alpha_mode: AlphaMode::Blend,
      perceptual_roughness: 1.0,
      ..default()
    })
  };
  (0..DROPS).for_each(|index| {
    commands.spawn((
      Drop { floor: f32::INFINITY, phase: index as f32 * 0.37, snow: false },
      Mesh3d(gear.streak.clone()),
      MeshMaterial3d(gear.rain.clone()),
      NotShadowCaster,
      Transform::default(),
      Visibility::Hidden
    ));
  });
  commands.spawn((
    Clouds,
    NotShadowCaster,
    MeshMaterial3d(gear.clouds.clone()),
    Transform::default(),
    Visibility::Visible
  ));
  commands.insert_resource(gear);
}

fn cloudy(cell: IVec2, cover: f32) -> bool {
  let (x, z) = (cell.x as f32, cell.y as f32);
  let puff =
    fbm2(0xC10D, x / 7.0, z / 5.0, 3) + 0.5 + fbm2(0xC10E, x / 2.0, z / 2.0, 1) * 0.15;
  puff > 1.0 - cover
}

fn clouds(centre: IVec2, cover: f32) -> Mesh {
  let half = CLOUD_SPAN / 2;
  let cells: Vec<IVec2> = (-half..half)
    .flat_map(|dz| (-half..half).map(move |dx| centre + IVec2::new(dx, dz)))
    .filter(|&cell| cloudy(cell, cover))
    .collect();
  let (low, high) = (CLOUD_HEIGHT, CLOUD_HEIGHT + CLOUD_THICK);
  let faces = cells.iter().flat_map(|&cell| {
    let (x0, z0) = (cell.x as f32 * CLOUD_CELL, cell.y as f32 * CLOUD_CELL);
    let (x1, z1) = (x0 + CLOUD_CELL, z0 + CLOUD_CELL);
    let open = |offset: IVec2| !cloudy(cell + offset, cover);
    [
      Some((
        [[x0, high, z1], [x1, high, z1], [x1, high, z0], [x0, high, z0]],
        Vec3::Y,
        1.0
      )),
      Some((
        [[x0, low, z0], [x1, low, z0], [x1, low, z1], [x0, low, z1]],
        Vec3::NEG_Y,
        0.72
      )),
      open(IVec2::X).then_some((
        [[x1, low, z1], [x1, low, z0], [x1, high, z0], [x1, high, z1]],
        Vec3::X,
        0.86
      )),
      open(IVec2::NEG_X).then_some((
        [[x0, low, z0], [x0, low, z1], [x0, high, z1], [x0, high, z0]],
        Vec3::NEG_X,
        0.86
      )),
      open(IVec2::Y).then_some((
        [[x0, low, z1], [x1, low, z1], [x1, high, z1], [x0, high, z1]],
        Vec3::Z,
        0.8
      )),
      open(IVec2::NEG_Y).then_some((
        [[x1, low, z0], [x0, low, z0], [x0, high, z0], [x1, high, z0]],
        Vec3::NEG_Z,
        0.8
      ))
    ]
    .into_iter()
    .flatten()
  });
  let (positions, normals, colors): (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 4]>) = faces
    .flat_map(|(corners, normal, shade)| {
      corners.map(|corner| (corner, normal.to_array(), [shade, shade, shade, 1.0]))
    })
    .fold(
      (Vec::new(), Vec::new(), Vec::new()),
      |(mut p, mut n, mut c), (corner, normal, color)| {
        p.push(corner);
        n.push(normal);
        c.push(color);
        (p, n, c)
      }
    );
  let indices = (0..positions.len() as u32 / 4)
    .flat_map(|quad| [0, 1, 2, 0, 2, 3].map(|corner| quad * 4 + corner))
    .collect();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn drift(
  time: Res<Time>,
  pilot: Res<Pilot>,
  weathers: Query<&Weather>,
  gear: Res<Gear>,
  mut climate: ResMut<Climate>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut skies: Query<(&mut Transform, Option<&Mesh3d>, Entity), With<Clouds>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  let sky = weathers.iter().next().map_or(Sky::Clear, |weather| weather.sky);
  let ease = |from: f32, to: f32, rate: f32| from + (to - from) * (dt * rate).min(1.0);
  let first = climate.built.is_none() || opts().shot.is_some();
  let rate = if first { 1000.0 } else { 0.05 };
  climate.cover = ease(climate.cover, sky.cover(), rate);
  climate.wet = ease(climate.wet, sky.wet(), rate * 2.0);
  climate.sun = ease(climate.sun, sky.sun(), rate);
  let offset = time.elapsed_secs() * DRIFT;
  let centre = ((pilot.at.xz() - Vec2::new(offset, 0.0)) / CLOUD_CELL).floor().as_ivec2();
  let stale = climate.built.is_none_or(|(built, cover)| {
    (built - centre).abs().max_element() > 6 || (cover - climate.cover).abs() > 0.02
  });
  skies.iter_mut().for_each(|(mut transform, mesh, entity)| {
    transform.translation.x = offset;
    if stale {
      let made = meshes.add(clouds(centre, climate.cover));
      mesh.into_iter().for_each(|old| {
        meshes.remove(&old.0);
      });
      commands.entity(entity).insert(Mesh3d(made));
    }
  });
  if stale {
    climate.built = Some((centre, climate.cover))
  }
  let grey = 1.0 - climate.wet * 0.55;
  let tint = Color::srgba(grey, grey, grey * 1.02, 0.85);
  if materials.get(&gear.clouds).is_some_and(|material| material.base_color != tint)
    && let Some(mut material) = materials.get_mut(&gear.clouds)
  {
    material.base_color = tint
  }
}

fn column_floor(voxels: &Voxels, x: f32, z: f32, from: f32) -> f32 {
  let (x, z) = (x.floor() as i32, z.floor() as i32);
  let top = from as i32 + 24;
  (0..48)
    .map(|depth| IVec3::new(x, top - depth, z))
    .find(|&cell| voxels.block(cell).is_none_or(|block| block.solid() || block.fluid()))
    .map_or(from - 30.0, |cell| cell.y as f32 + 1.0)
}

fn fall(
  time: Res<Time>,
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  gear: Res<Gear>,
  mut climate: ResMut<Climate>,
  mut drops: Query<(
    &mut Drop,
    &mut Transform,
    &mut Visibility,
    &mut Mesh3d,
    &mut MeshMaterial3d<StandardMaterial>
  )>
) {
  let dt = time.delta_secs().min(0.1);
  let now = time.elapsed_secs();
  let eye = pilot.eye();
  let snowing = voxels.as_deref().is_some_and(|voxels| {
    Island::near(voxels.seed, pilot.at)
      .first()
      .is_some_and(|island| island.kind == Kind::Frost)
      || pilot.at.y > 120.0
  });
  let shown = (DROPS as f32 * climate.wet) as usize;
  drops.iter_mut().enumerate().for_each(
    |(index, (mut drop, mut transform, mut visibility, mut mesh, mut material))| {
      let wanted = index < shown && voxels.is_some();
      *visibility = if wanted { Visibility::Visible } else { Visibility::Hidden };
      if drop.snow != snowing {
        drop.snow = snowing;
        mesh.0 = if snowing { gear.flake.clone() } else { gear.streak.clone() };
        material.0 = if snowing { gear.snow.clone() } else { gear.rain.clone() };
      }
      if let Some(voxels) = voxels.as_deref()
        && wanted
      {
        let speed = if snowing { SNOW_FALL } else { RAIN_FALL };
        let at = transform.translation;
        let away = (at.xz() - eye.xz()).abs().max_element();
        let at = match at.y < drop.floor || away > SHOWER {
          true => {
            let spot = eye.xz()
              + Vec2::new(climate.dice() * 2.0 - 1.0, climate.dice() * 2.0 - 1.0)
                * SHOWER;
            drop.floor = column_floor(voxels, spot.x, spot.y, eye.y);
            let height = eye.y.max(drop.floor) + 2.0 + climate.dice() * 16.0;
            Vec3::new(spot.x, height, spot.y)
          }
          false => {
            let sway = match snowing {
              true => {
                Vec3::new((now * 1.3 + drop.phase).sin(), 0.0, (now + drop.phase).cos())
                  * 0.5
              }
              false => Vec3::new(0.8, 0.0, 0.3)
            };
            at + (sway - Vec3::Y * speed) * dt
          }
        };
        transform.translation = at
      }
    }
  )
}

fn darken(
  time: Res<Time>,
  settings: Res<Settings>,
  pilot: Res<Pilot>,
  weathers: Query<&Weather>,
  gear: Res<Gear>,
  voxels: Option<Res<Voxels>>,
  mut climate: ResMut<Climate>,
  mut suns: Query<&mut DirectionalLight>,
  mut fogs: Query<&mut DistanceFog, With<Eye>>,
  mut bolts: Query<(Entity, &mut Bolt)>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  let storm = weathers.iter().next().is_some_and(|weather| weather.sky == Sky::Storm);
  climate.flash = (climate.flash - dt * 5.0).max(0.0);
  if storm
    && let Some(voxels) = voxels
    && climate.dice() < dt * 0.1
  {
    climate.flash = 1.0;
    let angle = climate.dice() * std::f32::consts::TAU;
    let reach = 60.0 + climate.dice() * 90.0;
    let foot = pilot.at.xz() + Vec2::from_angle(angle) * reach;
    let ground = crate::generate::height(voxels.seed, foot.x as i32, foot.y as i32)
      .max(crate::island::SEA) as f32;
    let points: Vec<Vec3> = (0..=10)
      .map(|step| {
        let along = step as f32 / 10.0;
        let jitter = match step {
          0 | 10 => Vec2::ZERO,
          _ => Vec2::new(climate.dice() - 0.5, climate.dice() - 0.5) * 9.0
        };
        let spot = foot + jitter;
        Vec3::new(spot.x, CLOUD_HEIGHT + (ground - CLOUD_HEIGHT) * along, spot.y)
      })
      .collect();
    points.windows(2).for_each(|pair| {
      let (from, to) = (pair[0], pair[1]);
      commands.spawn((
        Bolt { left: 0.3 },
        NotShadowCaster,
        Mesh3d(gear.segment.clone()),
        MeshMaterial3d(gear.bolt.clone()),
        Transform::from_translation((from + to) / 2.0)
          .looking_at(to, Vec3::X)
          .with_scale(Vec3::new(1.0, 1.0, from.distance(to)))
      ));
    })
  }
  bolts.iter_mut().for_each(|(entity, mut bolt)| {
    bolt.left -= dt;
    if bolt.left <= 0.0 {
      commands.entity(entity).despawn()
    }
  });
  suns
    .iter_mut()
    .for_each(|mut sun| sun.illuminance = SUN * (climate.sun + climate.flash * 1.5));
  let wet = climate.wet;
  fogs.iter_mut().for_each(|mut fog| {
    fog.color = Color::srgb(0.66 - wet * 0.22, 0.74 - wet * 0.26, 0.86 - wet * 0.3);
    fog.falloff = match settings.fog() {
      FogFalloff::Linear { start, end } => FogFalloff::Linear {
        start: start * (1.0 - wet * 0.75),
        end: end * (1.0 - wet * 0.45)
      },
      other => other
    }
  })
}

pub struct Weathering;

impl Plugin for Weathering {
  fn build(&self, app: &mut App) {
    app
      .replicate::<Weather>()
      .insert_resource(Climate {
        cover: Sky::Clear.cover(),
        wet: 0.0,
        sun: 1.0,
        flash: 0.0,
        luck: 0xC10D,
        built: None
      })
      .add_systems(Startup, forecast.run_if(authority))
      .add_systems(Update, turn.run_if(authority))
      .add_systems(Startup, gather.run_if(plays))
      .add_systems(
        Update,
        (drift, fall, darken).chain().run_if(plays).run_if(resource_exists::<Pilot>)
      );
  }
}
