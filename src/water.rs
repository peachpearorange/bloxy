use {crate::{block::{Block, Fluid},
             opts::opts,
             player::Eye,
             protocol::Role,
             sky::daylight,
             voxels::Voxels},
     bevy::{asset::embedded_asset,
            core_pipeline::fullscreen_material::{FullscreenMaterial,
                                                 FullscreenMaterialPlugin},
            light::{NotShadowCaster, NotShadowReceiver},
            pbr::{DistanceFog, ExtendedMaterial, FogFalloff, MaterialExtension},
            prelude::*,
            render::{extract_component::ExtractComponent,
                     render_resource::{AsBindGroup, Face, ShaderType}},
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

#[derive(Component)]
struct Abyss(Handle<StandardMaterial>);

fn enclose(
  eyes: Query<Entity, Added<Eye>>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  for eye in eyes.iter() {
    let material = materials.add(StandardMaterial {
      unlit: true,
      cull_mode: Some(Face::Front),
      ..default()
    });
    commands.spawn((
      Abyss(material.clone()),
      Mesh3d(meshes.add(Sphere::new(SEEN_UNDERWATER + 6.0).mesh().ico(3).unwrap())),
      MeshMaterial3d(material),
      NotShadowCaster,
      NotShadowReceiver,
      Transform::default(),
      Visibility::Hidden,
      ChildOf(eye)
    ));
  }
}

fn submerge(
  time: Res<Time>,
  voxels: Option<Res<Voxels>>,
  mut eyes: Query<
    (Entity, &GlobalTransform, &mut DistanceFog, Option<&mut Submerged>),
    With<Eye>
  >,
  mut abysses: Query<(&Abyss, &mut Visibility)>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  for (entity, place, mut fog, submerged) in eyes.iter_mut() {
    let eye = place.translation();
    let under = voxels.as_deref().is_some_and(|voxels| underwater(voxels, eye));
    for (_, mut visibility) in abysses.iter_mut() {
      visibility.set_if_neq(if under {
        Visibility::Inherited
      } else {
        Visibility::Hidden
      });
    }
    match (under, submerged) {
      (true, submerged) => {
        let dim = 1.0 - daylight(opts().hour).dark * 0.85;
        let murk = MURK * dim;
        fog.color = Color::srgb(murk.x, murk.y, murk.z);
        for (abyss, _) in abysses.iter() {
          if let Some(mut material) = materials.get_mut(&abyss.0)
            && material.base_color != fog.color
          {
            material.base_color = fog.color
          }
        }
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
  }
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
        .add_systems(Update, enclose)
        .add_systems(PostUpdate, submerge);
    }
  }
}
