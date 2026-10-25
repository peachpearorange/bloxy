use {crate::{block::Block,
             fx::Effects,
             opts::opts,
             player::Pilot,
             protocol::plays,
             voxels::{Voxels, chunk_of}},
     bevy::{camera::Exposure,
            core_pipeline::tonemapping::Tonemapping,
            light::{AtmosphereEnvironmentMapLight, CascadeShadowConfigBuilder,
                    GlobalAmbientLight, SunDisk, atmosphere::ScatteringMedium,
                    light_consts::lux},
            pbr::{AtmosphereSettings, DistanceFog},
            platform::collections::HashMap,
            post_process::bloom::Bloom,
            prelude::*,
            render::view::ColorGrading},
     std::f32::consts::PI};

const DAY_EXPOSURE: f32 = 13.6;
const NIGHT_EXPOSURE: f32 = 7.2;
const EXPOSURE_BY_ELEVATION: [(f32, f32); 6] = [
  (-0.2, NIGHT_EXPOSURE),
  (-0.06, 7.8),
  (0.0, 10.2),
  (0.08, 11.9),
  (0.25, 13.0),
  (0.5, DAY_EXPOSURE)
];
const DAY_HAZE: Vec3 = Vec3::new(0.66, 0.74, 0.86);
const DUSK_HAZE: Vec3 = Vec3::new(0.8, 0.64, 0.6);
const NIGHT_HAZE: Vec3 = Vec3::new(0.05, 0.07, 0.11);
const MOONLIGHT: f32 = 60.0;
const MOON_TINT: Color = Color::srgb(0.62, 0.74, 1.0);
const SUN_DISK: SunDisk = SunDisk { angular_size: 0.028, intensity: 1.0 };
const MOON_DISK: SunDisk = SunDisk { angular_size: 0.035, intensity: 40.0 };
const TORCH_LUMENS: f32 = 70_000.0;
const TORCH_EMBERS: usize = 16;
const TORCH_TINT: Color = Color::srgb(1.0, 0.86, 0.68);
const TORCH_SEEN: f32 = 28.0;

fn toward_sun(hour: f32) -> Vec3 {
  let arc = (hour - 6.0) / 12.0 * PI;
  Vec3::new(arc.cos(), arc.sin(), 0.35).normalize()
}

fn smooth(from: f32, to: f32, at: f32) -> f32 {
  let t = ((at - from) / (to - from)).clamp(0.0, 1.0);
  t * t * (3.0 - 2.0 * t)
}

#[derive(Clone)]
pub struct Daylight {
  pub toward: Vec3,
  pub lux: f32,
  pub tint: Color,
  pub disk: SunDisk,
  pub ev100: f32,
  pub haze: Vec3,
  pub dark: f32
}

pub fn daylight(hour: f32) -> Daylight {
  let sun = toward_sun(hour);
  let above = sun.y > -0.03;
  let sunlit = smooth(-0.03, 0.04, sun.y);
  let moonlit = smooth(-0.03, -0.12, sun.y);
  let golden = smooth(0.3, 0.04, sun.y) * smooth(-0.12, -0.01, sun.y);
  let dark = smooth(-0.02, -0.18, sun.y);
  let ev100 = EXPOSURE_BY_ELEVATION.windows(2).find(|pair| sun.y <= pair[1].0).map_or(
    DAY_EXPOSURE,
    |pair| {
      let ((low, dim), (high, bright)) = (pair[0], pair[1]);
      dim + (bright - dim) * ((sun.y - low) / (high - low)).clamp(0.0, 1.0)
    }
  );
  let warm = Color::srgb(1.0, 0.92 - golden * 0.25, 0.82 - golden * 0.4);
  Daylight {
    toward: if above { sun } else { Vec3::new(-sun.x, -sun.y, 0.3).normalize() },
    lux: if above { lux::RAW_SUNLIGHT * sunlit } else { MOONLIGHT * moonlit },
    tint: if above { warm } else { MOON_TINT },
    disk: if above { SUN_DISK } else { MOON_DISK },
    ev100,
    haze: DAY_HAZE.lerp(DUSK_HAZE, golden).lerp(NIGHT_HAZE, dark),
    dark
  }
}

pub const PLAIN: bool = cfg!(feature = "plain");
pub const PLAIN_NOTE: &str = "Plain mode: this browser has no WebGPU, so the sky is plain and there are no particles. Recent Chrome or Edge show the full game.";
const PLAIN_SKY: Vec3 = Vec3::new(0.7, 0.82, 1.0);
const PLAIN_AMBIENT: f32 = 1.6;

const UNHAZED: f32 = 1.0;

pub fn atmosphere() -> impl Bundle {
  (
    AtmosphereSettings { aerial_view_lut_max_distance: UNHAZED, ..default() },
    AtmosphereEnvironmentMapLight::default()
  )
}

pub fn lens() -> impl Bundle {
  let Daylight { ev100, haze, .. } = daylight(opts().hour);
  (
    Exposure { ev100 },
    Tonemapping::AcesFitted,
    Bloom { intensity: 0.18, ..Bloom::NATURAL },
    DistanceFog {
      color: Color::srgb(haze.x, haze.y, haze.z),
      directional_light_color: Color::srgba(1.0, 0.92, 0.75, 0.5),
      directional_light_exponent: 24.0,
      falloff: FogFalloff::Exponential { density: 0.0 },
      ..default()
    },
    Msaa::Off,
    ColorGrading::default()
  )
}

fn light(
  mut commands: Commands,
  mut media: ResMut<Assets<ScatteringMedium>>,
  mut ambient: ResMut<GlobalAmbientLight>
) {
  let Daylight { toward, lux, tint, disk, dark, haze, ev100 } = daylight(opts().hour);
  match PLAIN {
    true => {
      let sky = haze * PLAIN_SKY;
      commands.insert_resource(ClearColor(Color::srgb(sky.x, sky.y, sky.z)));
      warn!("{PLAIN_NOTE}")
    }
    false => {
      commands.spawn(bevy::light::Atmosphere::earth(
        media.add(ScatteringMedium::earth(256, 256))
      ));
    }
  }
  commands.spawn((
    DirectionalLight {
      illuminance: lux,
      color: tint,
      shadow_maps_enabled: true,
      ..default()
    },
    disk,
    CascadeShadowConfigBuilder {
      num_cascades: 2,
      first_cascade_far_bound: 20.0,
      maximum_distance: 90.0,
      ..default()
    }
    .build(),
    Transform::default().looking_to(-toward, Vec3::Y)
  ));
  *ambient = GlobalAmbientLight {
    color: Color::srgb(0.6, 0.7, 1.0),
    brightness: if PLAIN {
      PLAIN_AMBIENT * 1.2 * 2f32.powf(ev100)
    } else {
      80.0 + 60.0 * dark
    },
    ..default()
  }
}

#[derive(Resource, Default)]
struct Flames(HashMap<IVec3, Entity>);

pub fn torchlight() -> PointLight {
  PointLight {
    intensity: TORCH_LUMENS * 2f32.powf(daylight(opts().hour).ev100 - NIGHT_EXPOSURE),
    range: 20.0,
    color: TORCH_TINT,
    shadow_maps_enabled: false,
    ..default()
  }
}

fn kindle(
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  mut flames: ResMut<Flames>,
  effects: Res<Effects>,
  mut commands: Commands
) {
  if let Some(voxels) = voxels {
    let eye = pilot.eye();
    let burning = |block: Block| block.torch() || block == Block::Brazier;
    let mut near: Vec<IVec3> = voxels
      .torches_near(chunk_of(eye.floor().as_ivec3()))
      .into_iter()
      .filter(|torch| voxels.block(*torch).is_some_and(burning))
      .filter(|torch| torch.as_vec3().distance(eye) < TORCH_SEEN)
      .collect();
    near.sort_by(|a, b| a.as_vec3().distance(eye).total_cmp(&b.as_vec3().distance(eye)));
    near.truncate(TORCH_EMBERS);
    flames.0.retain(|torch, entity| {
      let kept = near.contains(torch);
      if !kept {
        commands.entity(*entity).despawn()
      }
      kept
    });
    let fresh: Vec<IVec3> =
      near.into_iter().filter(|torch| !flames.0.contains_key(torch)).collect();
    for torch in fresh {
      let block = voxels.block(torch).unwrap_or_default();
      let flame = crate::model::flame(block);
      let entity = commands
        .spawn((
          effects.emit(match block {
            Block::Brazier => &effects.blaze,
            _ => &effects.ember
          }),
          Transform::from_translation(torch.as_vec3() + flame + Vec3::Y * 0.08)
        ))
        .id();
      flames.0.insert(torch, entity);
    }
  }
}

pub struct Sky;

impl Plugin for Sky {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Flames>()
      .add_systems(Startup, light.run_if(plays))
      .add_systems(Update, kindle.run_if(plays).run_if(resource_exists::<Pilot>));
  }
}
