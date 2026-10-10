use {crate::{player::{Eye, Me, Pilot},
             protocol::{Avatar, Player, plays},
             skin::{PX, Part, Skin}},
     bevy::{camera::visibility::RenderLayers, prelude::*},
     std::f32::consts::PI};

#[derive(Component)]
struct Figure {
  shown: Vec3,
  stride: f32
}

#[derive(Component)]
pub struct Head;

#[derive(Component)]
pub struct Limb {
  phase: f32
}

#[derive(Component)]
struct Tag(Entity);

#[derive(Resource, Clone)]
pub struct Kit {
  head: Handle<Mesh>,
  body: Handle<Mesh>,
  pub arm: Handle<Mesh>,
  leg: Handle<Mesh>
}

#[derive(Component)]
pub struct Clad {
  pub material: Handle<StandardMaterial>,
  pub image: Handle<Image>
}

pub fn sew(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
  let mut shaped =
    |part: Part, lift: f32| meshes.add(part.mesh().translated_by(Vec3::Y * lift * PX));
  commands.insert_resource(Kit {
    head: shaped(Part::Head, 4.0),
    body: shaped(Part::Body, 0.0),
    arm: shaped(Part::Arm, -6.0),
    leg: shaped(Part::Leg, -6.0)
  })
}

pub fn clothe(
  skin: &Skin,
  images: &mut Assets<Image>,
  materials: &mut Assets<StandardMaterial>
) -> Clad {
  let image = images.add(skin.image(false));
  let material = materials.add(StandardMaterial {
    base_color_texture: Some(image.clone()),
    perceptual_roughness: 0.85,
    ..default()
  });
  Clad { material, image }
}

pub fn assemble(
  figure: &mut EntityCommands,
  kit: &Kit,
  clad: &Clad,
  layers: RenderLayers
) {
  let material = MeshMaterial3d(clad.material.clone());
  figure.with_children(|figure| {
    figure.spawn((
      Mesh3d(kit.body.clone()),
      material.clone(),
      layers.clone(),
      Transform::from_xyz(0.0, 18.0 * PX, 0.0)
    ));
    figure.spawn((
      Head,
      Mesh3d(kit.head.clone()),
      material.clone(),
      layers.clone(),
      Transform::from_xyz(0.0, 24.0 * PX, 0.0)
    ));
    [
      (6.0, 0.0, kit.arm.clone(), 24.0),
      (-6.0, PI, kit.arm.clone(), 24.0),
      (2.0, PI, kit.leg.clone(), 12.0),
      (-2.0, 0.0, kit.leg.clone(), 12.0)
    ]
    .into_iter()
    .for_each(|(x, phase, mesh, pivot)| {
      figure.spawn((
        Limb { phase },
        Mesh3d(mesh),
        material.clone(),
        layers.clone(),
        Transform::from_xyz(x * PX, pivot * PX, 0.0)
      ));
    });
  });
}

fn dress(
  pilot: Option<Res<Pilot>>,
  kit: Res<Kit>,
  mut commands: Commands,
  arrivals: Query<(Entity, &Player, &Avatar, &Skin), (Without<Figure>, Without<Me>)>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  arrivals
    .iter()
    .filter(|(entity, ..)| pilot.as_ref().is_some_and(|pilot| pilot.me != *entity))
    .for_each(|(entity, player, avatar, skin)| {
      let clad = clothe(skin, &mut images, &mut materials);
      let mut figure = commands.entity(entity);
      figure.insert((
        Figure { shown: avatar.at, stride: 0.0 },
        Transform::from_translation(avatar.at),
        Visibility::default()
      ));
      assemble(&mut figure, &kit, &clad, RenderLayers::default());
      figure.insert(clad);
      let tag = commands
        .spawn((
          Text::new(player.name.clone()),
          TextFont { font_size: FontSize::Px(15.0), ..default() },
          TextColor(Color::WHITE),
          TextShadow {
            offset: Vec2::splat(1.5),
            color: Color::srgba(0.0, 0.0, 0.0, 0.7)
          },
          Node { position_type: PositionType::Absolute, ..default() }
        ))
        .id();
      commands.entity(entity).insert(Tag(tag));
    })
}

fn reskin(
  figures: Query<(&Skin, &Clad), Changed<Skin>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  figures.iter().for_each(|(skin, clad)| {
    if let Some(mut image) = images.get_mut(&clad.image) {
      image.data = Some(skin.pixels(false))
    }
    materials.get_mut(&clad.material);
  })
}

fn animate(
  time: Res<Time>,
  mut figures: Query<(&Avatar, &mut Figure, &mut Transform, &Children), Without<Me>>,
  mut heads: Query<&mut Transform, (With<Head>, Without<Figure>, Without<Limb>)>,
  mut limbs: Query<(&Limb, &mut Transform), (Without<Figure>, Without<Head>)>
) {
  let dt = time.delta_secs();
  figures.iter_mut().for_each(|(avatar, mut figure, mut transform, children)| {
    let before = figure.shown;
    figure.shown = before.lerp(avatar.at, (dt * 12.0).min(1.0));
    let pace = (figure.shown - before).xz().length() / dt.max(1e-4);
    figure.stride += pace * dt * 2.2;
    let swing = (figure.stride.sin() * (pace / 4.3).min(1.0)) * 0.7;
    *transform = Transform::from_translation(figure.shown)
      .with_rotation(Quat::from_rotation_y(avatar.yaw));
    children.iter().for_each(|child| {
      if let Ok(mut head) = heads.get_mut(child) {
        head.rotation = Quat::from_rotation_x(avatar.pitch)
      }
      if let Ok((limb, mut limb_transform)) = limbs.get_mut(child) {
        limb_transform.rotation = Quat::from_rotation_x(swing * (limb.phase).cos())
      }
    })
  })
}

fn label(
  figures: Query<(&Figure, &Player, &Tag), Without<Me>>,
  eyes: Query<(&Camera, &GlobalTransform), With<Eye>>,
  mut tags: Query<(&mut Node, &mut Visibility, &mut Text)>
) {
  if let Ok((camera, eye)) = eyes.single() {
    figures.iter().for_each(|(figure, player, tag)| {
      if let Ok((mut node, mut visibility, mut text)) = tags.get_mut(tag.0) {
        let over = figure.shown + Vec3::Y * 2.15;
        let near = eye.translation().distance(over) < 48.0;
        match camera.world_to_viewport(eye, over).ok().filter(|_| near) {
          Some(spot) => {
            node.left = Val::Px(spot.x - player.name.len() as f32 * 4.0);
            node.top = Val::Px(spot.y - 10.0);
            *visibility = Visibility::Inherited
          }
          None => *visibility = Visibility::Hidden
        }
        if text.0 != player.name {
          text.0 = player.name.clone()
        }
      }
    })
  }
}

fn untag(gone: On<Remove, Tag>, tags: Query<&Tag>, mut commands: Commands) {
  if let Ok(tag) = tags.get(gone.entity) {
    commands.entity(tag.0).despawn()
  }
}

fn undress(possessed: On<Add, Me>, mut commands: Commands) {
  commands
    .entity(possessed.entity)
    .remove::<(Figure, Tag, Clad)>()
    .despawn_related::<Children>();
}

pub struct Figures;

impl Plugin for Figures {
  fn build(&self, app: &mut App) {
    app
      .add_observer(untag)
      .add_observer(undress)
      .add_systems(Startup, sew.run_if(plays))
      .add_systems(Update, (dress, reskin, animate, label).chain().run_if(plays));
  }
}
