use {crate::{opts::opts, protocol::plays},
     bevy::{camera::Exposure,
            core_pipeline::tonemapping::Tonemapping,
            light::{AtmosphereEnvironmentMapLight, CascadeShadowConfigBuilder,
                    SunDisk, atmosphere::ScatteringMedium, light_consts::lux},
            pbr::{AtmosphereSettings, DistanceFog, FogFalloff},
            post_process::bloom::Bloom,
            prelude::*},
     std::f32::consts::PI};

const EXPOSURE: f32 = 13.0;
const HAZE: Color = Color::srgb(0.66, 0.74, 0.86);

pub fn lens() -> impl Bundle {
  let far = (opts().reach as f32 - 0.5) * crate::stream::CHUNK_METRES;
  (
    AtmosphereSettings::default(),
    AtmosphereEnvironmentMapLight::default(),
    Exposure { ev100: EXPOSURE },
    Tonemapping::AcesFitted,
    Bloom { intensity: 0.18, ..Bloom::NATURAL },
    DistanceFog {
      color: HAZE,
      directional_light_color: Color::srgba(1.0, 0.92, 0.75, 0.5),
      directional_light_exponent: 24.0,
      falloff: FogFalloff::Linear { start: far * 0.55, end: far },
      ..default()
    },
    Msaa::Off
  )
}

fn toward_sun(hour: f32) -> Vec3 {
  let arc = (hour - 6.0) / 12.0 * PI;
  Vec3::new(arc.cos(), arc.sin(), 0.35).normalize()
}

fn light(mut commands: Commands, mut media: ResMut<Assets<ScatteringMedium>>) {
  commands
    .spawn(bevy::light::Atmosphere::earth(media.add(ScatteringMedium::earth(256, 256))));
  commands.spawn((
    DirectionalLight {
      illuminance: lux::RAW_SUNLIGHT,
      shadow_maps_enabled: true,
      ..default()
    },
    SunDisk::EARTH,
    CascadeShadowConfigBuilder {
      num_cascades: 3,
      first_cascade_far_bound: 16.0,
      maximum_distance: 140.0,
      ..default()
    }
    .build(),
    Transform::default().looking_to(-toward_sun(opts().hour), Vec3::Y)
  ));
}

pub struct Sky;

impl Plugin for Sky {
  fn build(&self, app: &mut App) { app.add_systems(Startup, light.run_if(plays)); }
}
