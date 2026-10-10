use {crate::{block::Block,
             figure::{Clad, Kit, clothe},
             loose::{FLAT, Looks, SIZE, cubic},
             menu::Menu,
             player::{Aim, Eye, Gait, Pilot, Poke, Selected, View},
             protocol::{Inventory, plays},
             skin::{PX, Skin},
             stream::Palette},
     bevy::{light::NotShadowCaster, prelude::*},
     std::f32::consts::PI};

const SWING: f32 = 0.28;
const POKE: f32 = 0.3;
const REST: Vec3 = Vec3::new(0.42, -0.4, -0.34);

const GRIP: Vec3 = Vec3::new(0.3, -0.3, -0.5);
const HELD_LARGER: f32 = 1.6;
const ROD_LENGTH: f32 = 1.3;

#[derive(Component)]
struct Hand {
  swung: f32,
  poked: f32
}

#[derive(Component)]
struct Rod;

#[derive(Component, Default, PartialEq)]
pub struct Grip(pub Option<Block>);

pub fn grip() -> impl Bundle {
  (Grip::default(), Transform::from_xyz(0.0, -12.0 * PX, 0.0), Visibility::Hidden)
}

fn hold(
  mut grips: Query<
    (Entity, &Grip, Has<Fist>, &mut Transform, &mut Visibility),
    Changed<Grip>
  >,
  palette: Res<Palette>,
  mut looks: ResMut<Looks>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut commands: Commands
) {
  for (entity, grip, fist, mut transform, mut visibility) in grips.iter_mut() {
    match grip.0 {
      Some(block) => {
        let hand = Vec3::new(0.0, -12.0 * PX, -1.0 * PX);
        *transform = match cubic(block) {
          true => {
            Transform::from_translation(hand + Vec3::new(0.0, -SIZE * 0.55, -SIZE * 0.25))
              .with_rotation(Quat::from_rotation_y(PI / 4.0))
          }
          false => {
            let side = Quat::from_rotation_y(PI / 2.0);
            let turned = match fist {
              true => {
                let arm = Quat::from_euler(EulerRot::YXZ, 0.12, 1.95, -0.12);
                let shaft = Vec3::new(-0.25, 0.85, -0.45).normalize();
                let face = Vec3::new(0.25, 0.45, 0.85).reject_from(shaft).normalize();
                let diagonal = Vec3::new(1.0, 1.0, 0.0).normalize();
                let held = Mat3::from_cols(shaft, face, shaft.cross(face))
                  * Mat3::from_cols(diagonal, Vec3::Z, diagonal.cross(Vec3::Z))
                    .transpose();
                arm.inverse() * Quat::from_mat3(&held)
              }
              false => side
            };
            Transform::from_translation(hand + turned * Vec3::new(0.3, 0.3, 0.0) * FLAT)
              .with_rotation(turned)
          }
        };
        if fist {
          transform.translation = hand + (transform.translation - hand) * HELD_LARGER;
          transform.scale = Vec3::splat(HELD_LARGER)
        }
        *visibility = Visibility::Inherited;
        commands.entity(entity).insert((
          Mesh3d(looks.mesh(block, &mut meshes)),
          MeshMaterial3d(palette.solid.clone()),
          NotShadowCaster
        ));
        match block == Block::Torch {
          true => commands.entity(entity).insert(crate::sky::torchlight()),
          false => commands.entity(entity).remove::<PointLight>()
        };
      }
      None => {
        *visibility = Visibility::Hidden;
        commands.entity(entity).remove::<PointLight>();
      }
    }
  }
}

#[derive(Component)]
pub struct RodTip;

#[derive(Component)]
struct Fist;

fn carve(
  eyes: Query<Entity, Added<Eye>>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  for eye in eyes.iter() {
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
    for part in parts.into_iter() {
      commands.spawn((part, ChildOf(rod)));
    }
    commands.spawn((
      RodTip,
      Transform::from_translation(Vec3::Z * -ROD_LENGTH),
      ChildOf(rod)
    ));
  }
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
    commands
      .spawn((
        Hand { swung: SWING, poked: POKE },
        Mesh3d(kit.arm.clone()),
        MeshMaterial3d(clad.material.clone()),
        clad,
        NotShadowCaster,
        Transform::from_translation(REST),
        ChildOf(eye)
      ))
      .with_child((grip(), Fist));
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
    for clad in hands.iter() {
      if let Some(mut image) = images.get_mut(&clad.image) {
        image.data = Some(skin.pixels(false))
      }
      materials.get_mut(&clad.material);
    }
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
  view: Res<View>,
  inventories: Query<&Inventory>,
  mut hands: Query<(&mut Hand, &mut Transform, &mut Visibility), Without<Rod>>,
  mut rods: Query<(&mut Transform, &mut Visibility), With<Rod>>,
  mut grips: Query<(&mut Grip, &ChildOf)>,
  mut pokes: MessageReader<Poke>
) {
  let poking = pokes.read().count() > 0;
  for (mut grip, _) in
    grips.iter_mut().filter(|(_, parent)| hands.contains(parent.parent()))
  {
    grip.set_if_neq(Grip(pilot.held.filter(|&block| block != Block::FishingRod)));
  }
  let fishing = inventories
    .get(pilot.me)
    .ok()
    .and_then(|inventory| inventory.slots[selected.0])
    .is_some_and(|stack| stack.block == Block::FishingRod);
  let pressed = !menu.open
    && (buttons.just_pressed(MouseButton::Left)
      || buttons.just_pressed(MouseButton::Right));
  let first = *view == View::First;
  for (mut hand, mut transform, mut visibility) in hands.iter_mut() {
    visibility.set_if_neq(match first {
      true => Visibility::Inherited,
      false => Visibility::Hidden
    });
    hand.swung += time.delta_secs();
    hand.poked += time.delta_secs();
    if poking {
      hand.poked = 0.0;
      hand.swung = SWING
    } else if (pressed || aim.digging.is_some()) && hand.swung >= SWING {
      hand.swung = 0.0
    }
    let arc = ((hand.swung / SWING).min(1.0) * PI).sin();
    let thrust = ((hand.poked / POKE).min(1.0) * PI).sin();
    let sway = gait.sway() * 0.6;
    for (mut rod, mut visibility) in rods.iter_mut() {
      *visibility =
        if fishing && first { Visibility::Visible } else { Visibility::Hidden };
      *rod = Transform::from_translation(
        GRIP + Vec3::new(-sway.x - arc * 0.04, sway.y * 0.5 + arc * 0.06, -arc * 0.05)
      )
      .with_rotation(Quat::from_euler(EulerRot::YXZ, 0.12, 0.55 + arc * 0.5, 0.0))
    }
    *transform = Transform::from_translation(
      REST
        + Vec3::new(
          -sway.x - arc * 0.08 - thrust * 0.1,
          sway.y * 0.5 - arc * 0.05 + thrust * 0.08,
          -arc * 0.12 - thrust * 0.22
        )
    )
    .with_scale(Vec3::splat(0.6))
    .with_rotation(Quat::from_euler(
      EulerRot::YXZ,
      0.12 + arc * 0.3 + thrust * 0.15,
      1.95 - arc * 0.9 - thrust * 0.25,
      -0.12
    ))
  }
}

pub struct Hands;

impl Plugin for Hands {
  fn build(&self, app: &mut App) {
    app.add_systems(
      Update,
      (raise, reskin, carve, swing.run_if(resource_exists::<Pilot>), hold)
        .chain()
        .run_if(plays)
    );
  }
}
