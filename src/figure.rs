use {crate::{player::{Eye, Me, Pilot},
             protocol::{Avatar, Player, plays}},
     bevy::prelude::*};

#[derive(Component)]
struct Figure {
  shown: Vec3,
  stride: f32
}

#[derive(Component)]
struct Head;

#[derive(Component)]
struct Limb {
  phase: f32
}

#[derive(Component)]
struct Tag(Entity);

fn dress(
  pilot: Option<Res<Pilot>>,
  mut commands: Commands,
  arrivals: Query<(Entity, &Player, &Avatar), (Without<Figure>, Without<Me>)>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  arrivals
    .iter()
    .filter(|(entity, _, _)| pilot.as_ref().is_some_and(|pilot| pilot.me != *entity))
    .for_each(|(entity, player, avatar)| {
      let [r, g, b] = player.tint;
      let shirt = materials.add(StandardMaterial {
        base_color: Color::srgb(r, g, b),
        perceptual_roughness: 0.9,
        ..default()
      });
      let skin = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.66, 0.52),
        perceptual_roughness: 0.8,
        ..default()
      });
      let trousers = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.22, 0.35),
        perceptual_roughness: 0.9,
        ..default()
      });
      let eyes = materials
        .add(StandardMaterial { base_color: Color::srgb(0.1, 0.1, 0.12), ..default() });
      let (torso, head, eye) = (
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(0.5, 0.7, 0.26)))),
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.46)))),
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(0.08, 0.06, 0.02))))
      );
      let mut hanging = |size: Vec3| {
        Mesh3d(meshes.add(
          Cuboid::from_size(size).mesh().build().translated_by(Vec3::Y * -size.y / 2.0)
        ))
      };
      let (arm, leg) =
        (hanging(Vec3::new(0.2, 0.68, 0.22)), hanging(Vec3::new(0.24, 0.72, 0.24)));
      commands
        .entity(entity)
        .insert((
          Figure { shown: avatar.at, stride: 0.0 },
          Transform::from_translation(avatar.at),
          Visibility::default()
        ))
        .with_children(|figure| {
          figure.spawn((
            torso,
            MeshMaterial3d(shirt.clone()),
            Transform::from_xyz(0.0, 1.07, 0.0)
          ));
          figure
            .spawn((
              Head,
              head,
              MeshMaterial3d(skin.clone()),
              Transform::from_xyz(0.0, 1.65, 0.0)
            ))
            .with_children(|head| {
              [-0.1, 0.1].into_iter().for_each(|x| {
                head.spawn((
                  eye.clone(),
                  MeshMaterial3d(eyes.clone()),
                  Transform::from_xyz(x, 0.02, -0.235)
                ));
              })
            });
          [
            (-0.36, 0.0, shirt.clone(), arm.clone(), 1.42),
            (0.36, std::f32::consts::PI, shirt, arm, 1.42)
          ]
          .into_iter()
          .chain([
            (-0.125, std::f32::consts::PI, trousers.clone(), leg.clone(), 0.72),
            (0.125, 0.0, trousers, leg, 0.72)
          ])
          .for_each(|(x, phase, material, mesh, shoulder)| {
            figure.spawn((
              Limb { phase },
              mesh,
              MeshMaterial3d(material),
              Transform::from_xyz(x, shoulder, 0.0)
            ));
          });
        });
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
    .remove::<(Figure, Tag)>()
    .despawn_related::<Children>();
}

pub struct Figures;

impl Plugin for Figures {
  fn build(&self, app: &mut App) {
    app
      .add_observer(untag)
      .add_observer(undress)
      .add_systems(Update, (dress, animate, label).chain().run_if(plays));
  }
}
