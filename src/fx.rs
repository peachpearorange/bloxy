use {crate::{island::Island, player::Pilot, voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            platform::collections::HashMap,
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}},
     bevy_hanabi::{AccelModifier, AlphaMode as Blending, Attribute,
                   ColorOverLifetimeModifier, EffectAsset, EffectMaterial, ExprWriter,
                   Gradient, HanabiPlugin, LinearDragModifier, OrientMode,
                   OrientModifier, ParticleEffect, ParticleTextureModifier,
                   SetAttributeModifier, SetPositionCircleModifier,
                   SetPositionSphereModifier, SetVelocitySphereModifier,
                   SetVelocityTangentModifier, ShapeDimension, SimulationSpace,
                   SizeOverLifetimeModifier, SpawnerSettings, TangentAccelModifier,
                   VectorType}};

const ENCHANTED: f32 = 96.0;

#[derive(Resource)]
pub struct Effects {
  pub ember: Handle<EffectAsset>,
  pub magic: Handle<EffectAsset>,
  pub poof: Handle<EffectAsset>,
  pub dot: Handle<Image>
}

impl Effects {
  pub fn emit(&self, effect: &Handle<EffectAsset>) -> impl Bundle {
    (ParticleEffect::new(effect.clone()), EffectMaterial {
      images: vec![self.dot.clone()]
    })
  }
}

fn dot() -> Image {
  const SIZE: u32 = 32;
  Image::new(
    Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 1 },
    TextureDimension::D2,
    (0..SIZE * SIZE)
      .flat_map(|index| {
        let at = Vec2::new((index % SIZE) as f32, (index / SIZE) as f32) + 0.5;
        let away = (at / SIZE as f32 - 0.5).length() * 2.0;
        let level = ((1.0 - away).clamp(0.0, 1.0).powf(1.6) * 255.0) as u8;
        [level, level, level, level]
      })
      .collect(),
    TextureFormat::Rgba8Unorm,
    RenderAssetUsages::default()
  )
}

fn size(from: f32, to: f32) -> SizeOverLifetimeModifier {
  SizeOverLifetimeModifier {
    gradient: Gradient::from_keys([(0.0, Vec3::splat(from)), (1.0, Vec3::splat(to))]),
    screen_space_size: false
  }
}

fn ember() -> EffectAsset {
  let writer = ExprWriter::new();
  let drift = (writer.rand(VectorType::VEC3F) * writer.lit(2.0) - writer.lit(1.0))
    * writer.lit(Vec3::new(0.12, 0.05, 0.12));
  let velocity = SetAttributeModifier::new(
    Attribute::VELOCITY,
    (drift + writer.lit(Vec3::Y * 0.45)).expr()
  );
  let age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
  let lifetime = SetAttributeModifier::new(
    Attribute::LIFETIME,
    writer.lit(0.5).uniform(writer.lit(1.0)).expr()
  );
  let position = SetPositionSphereModifier {
    center: writer.lit(Vec3::ZERO).expr(),
    radius: writer.lit(0.035).expr(),
    dimension: ShapeDimension::Volume
  };
  let lift = AccelModifier::new(writer.lit(Vec3::Y * 0.5).expr());
  let drag = LinearDragModifier::new(writer.lit(1.2).expr());
  let slot = writer.lit(0u32).expr();
  let mut module = writer.finish();
  module.add_texture_slot("dot");
  EffectAsset::new(64, SpawnerSettings::rate(5.0.into()), module)
    .with_simulation_space(SimulationSpace::Global)
    .with_alpha_mode(Blending::Add)
    .init(position)
    .init(velocity)
    .init(age)
    .init(lifetime)
    .update(lift)
    .update(drag)
    .render(ColorOverLifetimeModifier::new(Gradient::from_keys([
      (0.0, Vec4::new(9.0, 4.5, 1.2, 1.0)),
      (0.5, Vec4::new(5.0, 1.6, 0.3, 0.8)),
      (1.0, Vec4::new(0.6, 0.12, 0.02, 0.0))
    ])))
    .render(size(0.05, 0.015))
    .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
    .render(ParticleTextureModifier::new(slot))
}

fn magic() -> EffectAsset {
  let writer = ExprWriter::new();
  let centre = writer.lit(Vec3::ZERO).expr();
  let up = writer.lit(Vec3::Y).expr();
  let position = SetPositionCircleModifier {
    center: writer.lit(Vec3::new(0.0, -0.6, 0.0)).expr(),
    axis: writer.lit(Vec3::Y).expr(),
    radius: writer.lit(0.5).uniform(writer.lit(0.75)).expr(),
    dimension: ShapeDimension::Surface
  };
  let velocity = SetVelocityTangentModifier {
    origin: centre,
    axis: up,
    speed: writer.lit(0.5).uniform(writer.lit(0.8)).expr()
  };
  let swirl = TangentAccelModifier::new(
    writer.lit(Vec3::ZERO).expr(),
    writer.lit(Vec3::Y).expr(),
    writer.lit(0.6).expr()
  );
  let lift = AccelModifier::new(writer.lit(Vec3::Y * 0.35).expr());
  let drag = LinearDragModifier::new(writer.lit(0.6).expr());
  let age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
  let lifetime = SetAttributeModifier::new(
    Attribute::LIFETIME,
    writer.lit(2.0).uniform(writer.lit(3.2)).expr()
  );
  let slot = writer.lit(0u32).expr();
  let mut module = writer.finish();
  module.add_texture_slot("dot");
  EffectAsset::new(128, SpawnerSettings::rate(9.0.into()), module)
    .with_simulation_space(SimulationSpace::Local)
    .with_alpha_mode(Blending::Add)
    .init(position)
    .init(velocity)
    .init(age)
    .init(lifetime)
    .update(swirl)
    .update(lift)
    .update(drag)
    .render(ColorOverLifetimeModifier::new(Gradient::from_keys([
      (0.0, Vec4::new(0.5, 2.5, 4.0, 0.0)),
      (0.15, Vec4::new(1.0, 3.5, 6.0, 1.0)),
      (0.7, Vec4::new(3.2, 1.2, 6.0, 0.7)),
      (1.0, Vec4::new(1.2, 0.3, 2.5, 0.0))
    ])))
    .render(size(0.07, 0.02))
    .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
    .render(ParticleTextureModifier::new(slot))
}

fn poof() -> EffectAsset {
  let writer = ExprWriter::new();
  let position = SetPositionSphereModifier {
    center: writer.lit(Vec3::ZERO).expr(),
    radius: writer.lit(0.35).expr(),
    dimension: ShapeDimension::Volume
  };
  let velocity = SetVelocitySphereModifier {
    center: writer.lit(Vec3::ZERO).expr(),
    speed: writer.lit(1.2).uniform(writer.lit(2.6)).expr()
  };
  let age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
  let lifetime = SetAttributeModifier::new(
    Attribute::LIFETIME,
    writer.lit(0.45).uniform(writer.lit(0.8)).expr()
  );
  let lift = AccelModifier::new(writer.lit(Vec3::Y * 1.2).expr());
  let drag = LinearDragModifier::new(writer.lit(4.0).expr());
  let slot = writer.lit(0u32).expr();
  let mut module = writer.finish();
  module.add_texture_slot("dot");
  EffectAsset::new(48, SpawnerSettings::once(36.0.into()), module)
    .with_simulation_space(SimulationSpace::Global)
    .with_alpha_mode(Blending::Blend)
    .init(position)
    .init(velocity)
    .init(age)
    .init(lifetime)
    .update(lift)
    .update(drag)
    .render(ColorOverLifetimeModifier::new(Gradient::from_keys([
      (0.0, Vec4::new(1.0, 1.0, 1.0, 0.95)),
      (0.6, Vec4::new(0.9, 0.9, 0.9, 0.7)),
      (1.0, Vec4::new(0.8, 0.8, 0.8, 0.0))
    ])))
    .render(size(0.22, 0.5))
    .render(OrientModifier::new(OrientMode::ParallelCameraDepthPlane))
    .render(ParticleTextureModifier::new(slot))
}

fn prepare(
  mut commands: Commands,
  mut effects: ResMut<Assets<EffectAsset>>,
  mut images: ResMut<Assets<Image>>
) {
  commands.insert_resource(Effects {
    ember: effects.add(ember()),
    magic: effects.add(magic()),
    poof: effects.add(poof()),
    dot: images.add(dot())
  })
}

#[derive(Resource, Default)]
struct Glimmers(HashMap<IVec2, Entity>);

fn enchant(
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  effects: Res<Effects>,
  mut glimmers: ResMut<Glimmers>,
  mut commands: Commands
) {
  if let Some(voxels) = voxels {
    let around = pilot.at.xz().floor().as_ivec2();
    let reach = IVec2::splat(ENCHANTED as i32);
    let near: Vec<Island> = Island::within(voxels.seed, around - reach, around + reach)
      .into_iter()
      .filter(|island| island.stone.xz().as_vec2().distance(pilot.at.xz()) < ENCHANTED)
      .collect();
    glimmers.0.retain(|cell, entity| {
      let kept = near.iter().any(|island| island.cell == *cell);
      if !kept {
        commands.entity(*entity).despawn()
      }
      kept
    });
    for island in near.iter() {
      glimmers.0.entry(island.cell).or_insert_with(|| {
        commands
          .spawn((
            effects.emit(&effects.magic),
            Transform::from_translation(
              island.stone.as_vec3() + Vec3::new(0.5, 1.4, 0.5)
            )
          ))
          .id()
      });
    }
  }
}

pub struct Sparkle;

impl Plugin for Sparkle {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(HanabiPlugin)
      .init_resource::<Glimmers>()
      .add_systems(PreStartup, prepare)
      .add_systems(Update, enchant.run_if(resource_exists::<Pilot>));
  }
}
