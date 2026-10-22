use {crate::protocol::Role,
     bevy::{asset::embedded_asset,
            pbr::{ExtendedMaterial, MaterialExtension},
            prelude::*,
            render::render_resource::AsBindGroup,
            shader::ShaderRef}};

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct Surface {
  #[uniform(100)]
  pub dug: Vec4,
  #[uniform(100)]
  pub aimed: Vec4,
  #[uniform(100)]
  pub weather: Vec4,
  #[uniform(100)]
  pub turf: Vec4,
  #[texture(101)]
  #[sampler(102)]
  pub atlas: Option<Handle<Image>>
}

impl MaterialExtension for Surface {
  fn vertex_shader() -> ShaderRef { "embedded://bloxy/ground.wgsl".into() }

  fn prepass_vertex_shader() -> ShaderRef {
    "embedded://bloxy/ground_prepass.wgsl".into()
  }

  fn fragment_shader() -> ShaderRef { "embedded://bloxy/ground.wgsl".into() }

  fn prepass_fragment_shader() -> ShaderRef {
    "embedded://bloxy/ground_prepass.wgsl".into()
  }
}

pub type Ground = ExtendedMaterial<StandardMaterial, Surface>;

fn spot(cell: Option<IVec3>) -> Vec4 {
  cell.map_or(Vec4::ZERO, |cell| cell.as_vec3().extend(1.0))
}

#[derive(Resource, Default, PartialEq)]
pub struct Concealed(pub Option<IVec3>);

#[derive(Resource, Default, PartialEq)]
pub struct Aimed(pub Option<IVec3>);

#[derive(Resource, Default, PartialEq, Clone, Copy)]
pub struct Elements {
  pub wind: f32,
  pub wet: f32
}

impl Elements {
  pub fn weather(self) -> Vec4 { Vec4::new(self.wind, self.wet, 0.0, 0.0) }
}

fn conceal(
  concealed: Res<Concealed>,
  aimed: Res<Aimed>,
  elements: Res<Elements>,
  mut grounds: ResMut<Assets<Ground>>
) {
  if concealed.is_changed() || aimed.is_changed() || elements.is_changed() {
    let (dug, aimed, weather) = (spot(concealed.0), spot(aimed.0), elements.weather());
    for (_, ground) in grounds.iter_mut() {
      ground.extension.dug = dug;
      ground.extension.aimed = aimed;
      ground.extension.weather = weather
    }
  }
}

pub struct Grounds;

impl Plugin for Grounds {
  fn build(&self, app: &mut App) {
    if app.world().resource::<Role>().plays() {
      bevy::shader::load_shader_library!(app, "meadow.wgsl");
      embedded_asset!(app, "ground.wgsl");
      embedded_asset!(app, "ground_prepass.wgsl");
      app
        .add_plugins(MaterialPlugin::<Ground>::default())
        .init_resource::<Concealed>()
        .init_resource::<Aimed>()
        .init_resource::<Elements>()
        .add_systems(PostUpdate, conceal);
    }
  }
}
