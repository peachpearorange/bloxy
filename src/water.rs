use {crate::{block::{Block, Fluid},
             opts::opts,
             player::Eye,
             protocol::Role,
             sky::daylight,
             voxels::Voxels},
     bevy::{asset::embedded_asset,
            core_pipeline::fullscreen_material::{FullscreenMaterial,
                                                 FullscreenMaterialPlugin},
            pbr::{DistanceFog, ExtendedMaterial, FogFalloff, MaterialExtension},
            prelude::*,
            render::{extract_component::ExtractComponent,
                     render_resource::{AsBindGroup, ShaderType}},
            shader::ShaderRef}};

const MURK: Vec3 = Vec3::new(0.05, 0.3, 0.36);
const SEEN_UNDERWATER: f32 = 18.0;
const SURFACE: f32 = 0.86;

#[derive(Asset, AsBindGroup, TypePath, Clone, Copy)]
pub struct Swell {
  #[uniform(100)]
  settings: Vec4
}

impl MaterialExtension for Swell {
  fn fragment_shader() -> ShaderRef { "embedded://bloxy/water.wgsl".into() }
}

pub type Water = ExtendedMaterial<StandardMaterial, Swell>;

pub fn water() -> Water {
  ExtendedMaterial {
    base: StandardMaterial { perceptual_roughness: 0.08, reflectance: 0.6, ..default() },
    extension: Swell { settings: Vec4::new(0.32, 0.55, 1.0, 0.22) }
  }
}

#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default)]
struct Submerged {
  tint: Vec4,
  time: f32,
  strength: f32,
  pad: Vec2
}

impl FullscreenMaterial for Submerged {
  fn fragment_shader() -> ShaderRef { "embedded://bloxy/underwater.wgsl".into() }
}

fn underwater(voxels: &Voxels, eye: Vec3) -> bool {
  let cell = eye.floor().as_ivec3();
  let water = |block: Option<Block>| {
    block.and_then(Block::liquid).is_some_and(|(fluid, _)| fluid == Fluid::Water)
  };
  water(voxels.block(cell))
    && (water(voxels.block(cell + IVec3::Y)) || eye.y - eye.y.floor() < SURFACE)
}

fn submerge(
  time: Res<Time>,
  voxels: Option<Res<Voxels>>,
  mut eyes: Query<
    (Entity, &GlobalTransform, &mut DistanceFog, Option<&mut Submerged>),
    With<Eye>
  >,
  mut commands: Commands
) {
  eyes.iter_mut().for_each(|(entity, place, mut fog, submerged)| {
    let eye = place.translation();
    let under = voxels.as_deref().is_some_and(|voxels| underwater(voxels, eye));
    match (under, submerged) {
      (true, submerged) => {
        let dim = 1.0 - daylight(opts().hour).dark * 0.85;
        let murk = MURK * dim;
        fog.color = Color::srgb(murk.x, murk.y, murk.z);
        fog.falloff = FogFalloff::Linear { start: 0.5, end: SEEN_UNDERWATER };
        let state = Submerged {
          tint: Vec4::new(0.55, 0.85, 0.95, 1.0),
          time: time.elapsed_secs(),
          strength: 1.0,
          pad: Vec2::ZERO
        };
        match submerged {
          Some(mut submerged) => *submerged = state,
          None => {
            commands.entity(entity).insert(state);
          }
        }
      }
      (false, Some(_)) => {
        commands.entity(entity).remove::<Submerged>();
      }
      (false, None) => ()
    }
  })
}

pub struct Waters;

impl Plugin for Waters {
  fn build(&self, app: &mut App) {
    if app.world().resource::<Role>().plays() {
      embedded_asset!(app, "water.wgsl");
      embedded_asset!(app, "underwater.wgsl");
      app
        .add_plugins((
          MaterialPlugin::<Water>::default(),
          FullscreenMaterialPlugin::<Submerged>::default()
        ))
        .add_systems(PostUpdate, submerge);
    }
  }
}
