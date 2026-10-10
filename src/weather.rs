use {crate::{island::{Island, Kind},
             noise::{hash, unit},
             opts::opts,
             player::{Eye, Pilot},
             protocol::{Role, authority, plays},
             sky::daylight,
             voxels::Voxels},
     bevy::{asset::embedded_asset,
            light::{NotShadowCaster, NotShadowReceiver,
                    light_consts::lux::RAW_SUNLIGHT},
            mesh::MeshVertexBufferLayoutRef,
            pbr::{DistanceFog, Material, MaterialPipeline, MaterialPipelineKey,
                  MaterialPlugin},
            prelude::*,
            render::render_resource::{AsBindGroup, Face, RenderPipelineDescriptor,
                                      SpecializedMeshPipelineError},
            shader::ShaderRef},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

const RAIN_FOG: f32 = 0.008;
const CLOUD_HEIGHT: f32 = 170.0;
const CLOUD_THICK: f32 = 80.0;
const CLOUD_SPAN: f32 = 5200.0;
const DRIFT: f32 = 2.5;
const DROPS: usize = 700;
const SHOWER: f32 = 22.0;
const RAIN_FALL: f32 = 16.0;
const SNOW_FALL: f32 = 2.4;
const FLASH: f32 = 1500.0;

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
  for (mut weather, mut spell) in weathers.iter_mut() {
    spell.left -= time.delta_secs();
    if spell.left <= 0.0 && opts().weather.is_none() {
      let luck = hash(0x5C1E5, weather.spell as i32, 0, 0);
      let sky = weather.sky.after(unit(luck, 1, 0, 0));
      spell.left = 90.0 + unit(luck, 2, 0, 0) * 150.0;
      *weather = Weather { sky, spell: weather.spell + 1 }
    }
  }
}

#[derive(Resource)]
struct Climate {
  cover: f32,
  wet: f32,
  sun: f32,
  flash: f32,
  luck: u32,
  settled: bool
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
  segment: Handle<Mesh>
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
    segment: meshes.add(Cuboid::new(0.5, 0.5, 1.0))
  };
  for index in 0..DROPS {
    commands.spawn((
      Drop { floor: f32::INFINITY, phase: index as f32 * 0.37, snow: false },
      Mesh3d(gear.streak.clone()),
      MeshMaterial3d(gear.rain.clone()),
      NotShadowCaster,
      Transform::default(),
      Visibility::Hidden
    ));
  }
  commands.insert_resource(gear);
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
struct Cloud {
  #[uniform(0)]
  toward_light: Vec4,
  #[uniform(1)]
  light: Vec4,
  #[uniform(2)]
  ambient: Vec4,
  #[uniform(3)]
  drift: Vec4,
  #[uniform(4)]
  haze: Vec4
}

impl Material for Cloud {
  fn fragment_shader() -> ShaderRef { "embedded://bloxy/cloud.wgsl".into() }

  fn alpha_mode(&self) -> AlphaMode { AlphaMode::Premultiplied }

  fn depth_bias(&self) -> f32 { -1.0e6 }

  fn enable_prepass() -> bool { false }

  fn enable_shadows() -> bool { false }

  fn specialize(
    _: &MaterialPipeline,
    descriptor: &mut RenderPipelineDescriptor,
    _: &MeshVertexBufferLayoutRef,
    _: MaterialPipelineKey<Self>
  ) -> Result<(), SpecializedMeshPipelineError> {
    descriptor.primitive.cull_mode = Some(Face::Front);
    Ok(())
  }
}

#[derive(Resource)]
struct Canopy(Handle<Cloud>);

fn spread(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut clouds: ResMut<Assets<Cloud>>
) {
  let cloud = clouds.add(Cloud::default());
  commands.insert_resource(Canopy(cloud.clone()));
  commands.spawn((
    Clouds,
    Mesh3d(meshes.add(Cuboid::new(CLOUD_SPAN, CLOUD_THICK, CLOUD_SPAN))),
    MeshMaterial3d(cloud),
    Transform::from_xyz(0.0, CLOUD_HEIGHT + CLOUD_THICK / 2.0, 0.0),
    NotShadowCaster,
    NotShadowReceiver
  ));
}

fn drift(
  time: Res<Time>,
  pilot: Res<Pilot>,
  weathers: Query<&Weather>,
  canopy: Res<Canopy>,
  mut climate: ResMut<Climate>,
  mut clouds: ResMut<Assets<Cloud>>,
  mut skies: Query<&mut Transform, With<Clouds>>
) {
  let dt = time.delta_secs().min(0.1);
  let sky = weathers.iter().next().map_or(Sky::Clear, |weather| weather.sky);
  let ease = |from: f32, to: f32, rate: f32| from + (to - from) * (dt * rate).min(1.0);
  let first = !climate.settled || opts().shot.is_some();
  climate.settled = true;
  let rate = if first { 1000.0 } else { 0.05 };
  climate.cover = ease(climate.cover, sky.cover(), rate);
  climate.wet = ease(climate.wet, sky.wet(), rate * 2.0);
  climate.sun = ease(climate.sun, sky.sun(), rate);
  for mut transform in skies.iter_mut() {
    transform.translation = pilot.at.with_y(CLOUD_HEIGHT + CLOUD_THICK / 2.0).floor()
  }
  let day = daylight(opts().hour);
  let lit = day.lux / RAW_SUNLIGHT;
  let [r, g, b, _] = day.tint.to_linear().to_f32_array();
  let sky_glow = Vec3::new(0.55, 0.64, 0.82) * (9000.0 * lit + 6.0) * climate.sun.sqrt();
  if let Some(mut cloud) = clouds.get_mut(&canopy.0) {
    *cloud = Cloud {
      toward_light: day.toward.extend(0.0),
      light: (Vec3::new(r, g, b) * day.lux * climate.sun).extend(0.0),
      ambient: sky_glow.extend(0.0),
      drift: Vec4::new(
        time.elapsed_secs() * DRIFT,
        time.elapsed_secs() * DRIFT * 0.3,
        climate.cover - 0.3,
        climate.wet
      ),
      haze: (sky_glow * 1.3).extend(1.0)
    }
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
  for (index, (mut drop, mut transform, mut visibility, mut mesh, mut material)) in
    drops.iter_mut().enumerate()
  {
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
            + Vec2::new(climate.dice() * 2.0 - 1.0, climate.dice() * 2.0 - 1.0) * SHOWER;
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
}

fn darken(
  time: Res<Time>,
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
    for pair in points.windows(2) {
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
    }
  }
  for (entity, mut bolt) in bolts.iter_mut() {
    bolt.left -= dt;
    if bolt.left <= 0.0 {
      commands.entity(entity).despawn()
    }
  }
  let day = daylight(opts().hour);
  let flash = FLASH * 2f32.powf(day.ev100 - 7.2);
  for mut sun in suns.iter_mut() {
    sun.illuminance = day.lux * climate.sun + climate.flash * flash
  }
  let wet = climate.wet;
  for mut fog in fogs.iter_mut() {
    let haze = day.haze * (1.0 - wet * 0.35);
    fog.color = Color::srgb(haze.x, haze.y, haze.z);
    fog.falloff = FogFalloff::Exponential { density: wet * RAIN_FOG }
  }
}

pub struct Weathering;

impl Plugin for Weathering {
  fn build(&self, app: &mut App) {
    if app.world().resource::<Role>().plays() {
      embedded_asset!(app, "cloud.wgsl");
      app.add_plugins(MaterialPlugin::<Cloud>::default());
    }
    app
      .replicate::<Weather>()
      .insert_resource(Climate {
        cover: Sky::Clear.cover(),
        wet: 0.0,
        sun: 1.0,
        flash: 0.0,
        luck: 0xC10D,
        settled: false
      })
      .add_systems(Startup, forecast.run_if(authority))
      .add_systems(Update, turn.run_if(authority))
      .add_systems(Startup, (gather, spread).run_if(plays))
      .add_systems(
        Update,
        (drift, fall, darken).chain().run_if(plays).run_if(resource_exists::<Pilot>)
      );
  }
}
