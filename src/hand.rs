use {crate::{figure::{Clad, Kit, clothe},
             menu::Menu,
             player::{Aim, Eye, Gait, Pilot},
             protocol::plays,
             skin::Skin},
     bevy::{light::NotShadowCaster, prelude::*},
     std::f32::consts::PI};

const SWING: f32 = 0.28;
const REST: Vec3 = Vec3::new(0.42, -0.4, -0.34);

#[derive(Component)]
struct Hand {
  swung: f32
}

fn raise(
  pilot: Option<Res<Pilot>>,
  kit: Res<Kit>,
  skins: Query<&Skin>,
  eyes: Query<Entity, With<Eye>>,
  hands: Query<(), With<Hand>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  if hands.is_empty()
    && let Some(pilot) = pilot
    && let Ok(skin) = skins.get(pilot.me)
    && let Ok(eye) = eyes.single()
  {
    let clad = clothe(skin, &mut images, &mut materials);
    commands.entity(eye).with_child((
      Hand { swung: SWING },
      Mesh3d(kit.arm.clone()),
      MeshMaterial3d(clad.material.clone()),
      clad,
      NotShadowCaster,
      Transform::from_translation(REST)
    ));
  }
}

fn reskin(
  pilot: Option<Res<Pilot>>,
  skins: Query<&Skin, Changed<Skin>>,
  hands: Query<&Clad, With<Hand>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  if let Some(pilot) = pilot
    && let Ok(skin) = skins.get(pilot.me)
  {
    hands.iter().for_each(|clad| {
      if let Some(mut image) = images.get_mut(&clad.image) {
        image.data = Some(skin.pixels(false))
      }
      materials.get_mut(&clad.material);
    })
  }
}

fn swing(
  time: Res<Time>,
  buttons: Res<ButtonInput<MouseButton>>,
  menu: Res<Menu>,
  aim: Res<Aim>,
  gait: Res<Gait>,
  mut hands: Query<(&mut Hand, &mut Transform)>
) {
  let pressed = !menu.open
    && (buttons.just_pressed(MouseButton::Left)
      || buttons.just_pressed(MouseButton::Right));
  hands.iter_mut().for_each(|(mut hand, mut transform)| {
    hand.swung += time.delta_secs();
    if (pressed || aim.digging.is_some()) && hand.swung >= SWING {
      hand.swung = 0.0
    }
    let progress = (hand.swung / SWING).min(1.0);
    let arc = (progress * PI).sin();
    let sway = gait.sway() * 0.6;
    *transform = Transform::from_translation(
      REST + Vec3::new(-sway.x - arc * 0.08, sway.y * 0.5 - arc * 0.05, -arc * 0.12)
    )
    .with_scale(Vec3::splat(0.6))
    .with_rotation(Quat::from_euler(
      EulerRot::YXZ,
      0.12 + arc * 0.3,
      1.95 - arc * 0.9,
      -0.12
    ))
  })
}

pub struct Hands;

impl Plugin for Hands {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, (raise, reskin, swing).chain().run_if(plays));
  }
}
