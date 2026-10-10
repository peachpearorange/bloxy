use {crate::{block::Block,
             figure::{Clad, Kit, clothe},
             menu::Menu,
             player::{Aim, Eye, Gait, Pilot, Selected},
             protocol::{Inventory, plays},
             skin::Skin},
     bevy::{light::NotShadowCaster, prelude::*},
     std::f32::consts::PI};

const SWING: f32 = 0.28;
const REST: Vec3 = Vec3::new(0.42, -0.4, -0.34);

const GRIP: Vec3 = Vec3::new(0.3, -0.3, -0.5);
const ROD_LENGTH: f32 = 1.3;

#[derive(Component)]
struct Hand {
  swung: f32
}

#[derive(Component)]
struct Rod;

#[derive(Component)]
pub struct RodTip;

fn carve(
  eyes: Query<Entity, Added<Eye>>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  eyes.iter().for_each(|eye| {
    let wood = materials.add(StandardMaterial {
      base_color: Color::srgb(0.55, 0.38, 0.2),
      perceptual_roughness: 0.7,
      ..default()
    });
    let cork = materials.add(Color::srgb(0.3, 0.2, 0.12));
    let metal = materials.add(StandardMaterial {
      base_color: Color::srgb(0.6, 0.6, 0.64),
      metallic: 0.6,
      perceptual_roughness: 0.4,
      ..default()
    });
    let mut piece = |size: Vec3, at: Vec3, material: &Handle<StandardMaterial>| {
      (
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(material.clone()),
        NotShadowCaster,
        Transform::from_translation(at)
      )
    };
    let parts = [
      piece(Vec3::new(0.022, 0.022, ROD_LENGTH), Vec3::Z * -ROD_LENGTH / 2.0, &wood),
      piece(Vec3::new(0.036, 0.036, 0.26), Vec3::Z * 0.02, &cork),
      piece(Vec3::new(0.05, 0.07, 0.07), Vec3::new(0.0, -0.05, -0.08), &metal)
    ];
    let rod = commands
      .spawn((Rod, Transform::from_translation(GRIP), Visibility::Hidden, ChildOf(eye)))
      .id();
    parts.into_iter().for_each(|part| {
      commands.spawn((part, ChildOf(rod)));
    });
    commands.spawn((
      RodTip,
      Transform::from_translation(Vec3::Z * -ROD_LENGTH),
      ChildOf(rod)
    ));
  })
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
  pilot: Res<Pilot>,
  selected: Res<Selected>,
  inventories: Query<&Inventory>,
  mut hands: Query<(&mut Hand, &mut Transform), Without<Rod>>,
  mut rods: Query<(&mut Transform, &mut Visibility), With<Rod>>
) {
  let fishing = inventories
    .get(pilot.me)
    .ok()
    .and_then(|inventory| inventory.slots[selected.0])
    .is_some_and(|stack| stack.block == Block::FishingRod);
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
    rods.iter_mut().for_each(|(mut rod, mut visibility)| {
      *visibility = if fishing { Visibility::Visible } else { Visibility::Hidden };
      *rod = Transform::from_translation(
        GRIP + Vec3::new(-sway.x - arc * 0.04, sway.y * 0.5 + arc * 0.06, -arc * 0.05)
      )
      .with_rotation(Quat::from_euler(EulerRot::YXZ, 0.12, 0.55 + arc * 0.5, 0.0))
    });
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
    app.add_systems(
      Update,
      (raise, reskin, carve, swing.run_if(resource_exists::<Pilot>))
        .chain()
        .run_if(plays)
    );
  }
}
