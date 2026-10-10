use {crate::{player::Eye, protocol::plays, sign::inked},
     bevy::{asset::RenderAssetUsages,
            image::ImageSampler,
            light::NotShadowCaster,
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}}};

const TEXEL: f32 = 0.045;
const PAD: u32 = 2;
const BAR: u32 = 24;
const SEEN_WITHIN: f32 = 40.0;
const BACKDROP: [u8; 4] = [0, 0, 0, 120];
const EMPTY: [u8; 4] = [40, 40, 40, 220];
const FULL: Color = Color::srgb(0.35, 0.85, 0.35);
const LOW: Color = Color::srgb(0.9, 0.25, 0.2);

#[derive(Component, Clone)]
pub struct Plate {
  pub text: String,
  pub color: Color,
  pub health: Option<f32>,
  pub at: Vec3
}

pub fn plate(text: impl Into<String>, color: Color, at: Vec3) -> impl Bundle {
  (
    Plate { text: text.into(), color, health: None, at },
    Drawn::default(),
    NotShadowCaster,
    Transform::from_translation(at),
    Visibility::Hidden
  )
}

#[derive(Component, Default)]
struct Drawn(Option<(String, [u8; 4], Option<u8>, UVec2)>);

#[derive(Resource)]
struct Quad(Handle<Mesh>);

fn quad(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
  commands.insert_resource(Quad(meshes.add(Rectangle::new(1.0, 1.0))))
}

fn picture(text: &str, ink: [u8; 4], share: Option<f32>) -> Image {
  let letters: Vec<char> = text.chars().collect();
  let written = (letters.len() as u32 * 4).saturating_sub(1);
  let wide = written.max(share.map_or(0, |_| BAR)) + PAD * 2;
  let lines = 5 + PAD * 2;
  let tall = lines + share.map_or(0, |_| 3 + PAD);
  let left = (wide - written) / 2;
  let filled = share.map_or(0, |share| (share * (wide - PAD * 2) as f32).round() as u32);
  let fill =
    share.map(|share| LOW.mix(&FULL, share).to_srgba().to_u8_array()).unwrap_or_default();
  let texel = |x: u32, y: u32| -> [u8; 4] {
    let lettered = y >= PAD
      && y < PAD + 5
      && x >= left
      && x < left + written
      && (x - left) % 4 < 3
      && letters
        .get(((x - left) / 4) as usize)
        .is_some_and(|&letter| inked(letter, (x - left) % 4, y - PAD));
    let barred =
      share.is_some() && y >= lines && y < lines + 3 && x >= PAD && x < wide - PAD;
    match (lettered, barred) {
      (true, _) => ink,
      (_, true) if x - PAD < filled => fill,
      (_, true) => EMPTY,
      _ => BACKDROP
    }
  };
  let mut image = Image::new(
    Extent3d { width: wide, height: tall, depth_or_array_layers: 1 },
    TextureDimension::D2,
    (0..wide * tall).flat_map(|index| texel(index % wide, index / wide)).collect(),
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  );
  image.sampler = ImageSampler::nearest();
  image
}

fn draw(
  quad: Res<Quad>,
  mut plates: Query<(Entity, &Plate, &mut Drawn), Changed<Plate>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  plates.iter_mut().for_each(|(entity, plate, mut drawn)| {
    let ink = plate.color.to_srgba().to_u8_array();
    let health = plate.health.map(|share| (share.clamp(0.0, 1.0) * 255.0) as u8);
    if drawn.0.as_ref().is_none_or(|(text, was, had, _)| {
      *text != plate.text || *was != ink || *had != health
    }) {
      let image =
        picture(&plate.text, ink, plate.health.map(|share| share.clamp(0.0, 1.0)));
      let size = image.size();
      let material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(image)),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        fog_enabled: false,
        ..default()
      });
      drawn.0 = Some((plate.text.clone(), ink, health, size));
      commands.entity(entity).insert((Mesh3d(quad.0.clone()), MeshMaterial3d(material)));
    }
  })
}

fn face(
  eyes: Query<&GlobalTransform, With<Eye>>,
  mut plates: Query<(&Plate, &Drawn, &mut Transform, &mut Visibility)>
) {
  if let Ok(eye) = eyes.single() {
    let (_, turned, from) = eye.to_scale_rotation_translation();
    plates.iter_mut().for_each(|(plate, drawn, mut transform, mut visibility)| {
      if let Some((.., size)) = drawn.0 {
        let scale = size.as_vec2().extend(1.0) * Vec3::new(TEXEL, TEXEL, 1.0);
        *transform =
          Transform::from_translation(plate.at + turned * Vec3::Y * scale.y / 2.0)
            .with_rotation(turned)
            .with_scale(scale);
        visibility.set_if_neq(match from.distance(plate.at) < SEEN_WITHIN {
          true => Visibility::Inherited,
          false => Visibility::Hidden
        });
      }
    })
  }
}

pub struct Plates;

impl Plugin for Plates {
  fn build(&self, app: &mut App) {
    app.add_systems(Startup, quad.run_if(plays)).add_systems(
      PostUpdate,
      (draw, face).chain().before(TransformSystems::Propagate).run_if(plays)
    );
  }
}
