use {crate::{opts::opts, protocol::Role},
     bevy::{asset::embedded_asset,
            pbr::{ExtendedMaterial, MaterialExtension},
            prelude::*,
            render::render_resource::AsBindGroup,
            shader::ShaderRef}};

pub const EMERGING: f32 = 1.1;
pub const LONG_AGO: f32 = -1.0e4;

#[derive(Asset, AsBindGroup, TypePath, Clone, Copy)]
pub struct Emerge {
  #[uniform(100)]
  pub born: Vec4,
  #[uniform(100)]
  pub hidden: Vec4,
  #[uniform(100)]
  pub weather: Vec4
}

impl MaterialExtension for Emerge {
  fn vertex_shader() -> ShaderRef { "embedded://bloxy/ground.wgsl".into() }

  fn prepass_vertex_shader() -> ShaderRef {
    "embedded://bloxy/ground_prepass.wgsl".into()
  }

  fn fragment_shader() -> ShaderRef { "embedded://bloxy/ground.wgsl".into() }

  fn prepass_fragment_shader() -> ShaderRef {
    "embedded://bloxy/ground_prepass.wgsl".into()
  }
}

pub type Ground = ExtendedMaterial<StandardMaterial, Emerge>;

pub fn born(time: &Time) -> f32 {
  match opts().shot {
    Some(_) => LONG_AGO,
    None => time.elapsed_secs_wrapped()
  }
}

pub fn settled() -> Vec4 { Vec4::new(LONG_AGO, 0.0, 0.0, 0.0) }

#[derive(Resource, Default, PartialEq)]
pub struct Concealed(pub Option<IVec3>);

impl Concealed {
  pub fn hidden(&self) -> Vec4 {
    self.0.map_or(Vec4::ZERO, |cell| cell.as_vec3().extend(1.0))
  }
}

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
  elements: Res<Elements>,
  mut grounds: ResMut<Assets<Ground>>
) {
  if concealed.is_changed() || elements.is_changed() {
    let (hidden, weather) = (concealed.hidden(), elements.weather());
    for (_, ground) in grounds.iter_mut() {
      ground.extension.hidden = hidden;
      ground.extension.weather = weather
    }
  }
}

pub struct Grounds;

impl Plugin for Grounds {
  fn build(&self, app: &mut App) {
    if app.world().resource::<Role>().plays() {
      embedded_asset!(app, "ground.wgsl");
      embedded_asset!(app, "ground_prepass.wgsl");
      app
        .add_plugins(MaterialPlugin::<Ground>::default())
        .init_resource::<Concealed>()
        .init_resource::<Elements>()
        .add_systems(PostUpdate, conceal);
    }
  }
}
