use {crate::{hand::{Grip, grip},
             plate::{Plate, plate},
             player::{Me, Pilot, View},
             protocol::{Avatar, Player, hue, plays},
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
struct Mirror;

#[derive(Component)]
pub struct Limb {
  phase: f32
}

#[derive(Component)]
struct Tag(Entity);

#[derive(Resource, Clone)]
pub struct Kit {
  pub head: Handle<Mesh>,
  pub body: Handle<Mesh>,
  pub arm: Handle<Mesh>,
  pub leg: Handle<Mesh>
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
      let mut limb = figure.spawn((
        Limb { phase },
        Mesh3d(mesh),
        material.clone(),
        layers.clone(),
        Transform::from_xyz(x * PX, pivot * PX, 0.0)
      ));
      if x == 6.0 && pivot == 24.0 {
        limb.with_child((grip(), layers.clone()));
      }
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
      let tag = commands.spawn(plate(player.name.clone(), Color::WHITE, avatar.at)).id();
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

fn mirror(
  pilot: Option<Res<Pilot>>,
  view: Res<View>,
  kit: Res<Kit>,
  skins: Query<Ref<Skin>>,
  mut mirrors: Query<(Entity, &mut Visibility), With<Mirror>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  if let Some(pilot) = pilot
    && let Ok(skin) = skins.get(pilot.me)
  {
    let shown = match *view {
      View::Behind => Visibility::Inherited,
      View::First => Visibility::Hidden
    };
    if skin.is_changed() || mirrors.is_empty() {
      mirrors.iter().for_each(|(old, _)| commands.entity(old).despawn());
      let clad = clothe(&skin, &mut images, &mut materials);
      let mut figure = commands.spawn((
        Mirror,
        Figure { shown: pilot.at, stride: 0.0 },
        Transform::from_translation(pilot.at),
        shown
      ));
      assemble(&mut figure, &kit, &clad, RenderLayers::default());
      figure.insert(clad);
    }
    mirrors.iter_mut().for_each(|(_, mut visibility)| {
      visibility.set_if_neq(shown);
    })
  }
}

fn animate(
  time: Res<Time>,
  pilot: Option<Res<Pilot>>,
  mut figures: Query<
    (Option<&Avatar>, Has<Mirror>, &mut Figure, &mut Transform, &Children),
    Without<Me>
  >,
  mut heads: Query<&mut Transform, (With<Head>, Without<Figure>, Without<Limb>)>,
  mut limbs: Query<
    (&Limb, &mut Transform, Option<&Children>),
    (Without<Figure>, Without<Head>)
  >,
  mut grips: Query<&mut Grip>
) {
  let dt = time.delta_secs();
  figures.iter_mut().for_each(
    |(avatar, mirrored, mut figure, mut transform, children)| {
      let avatar = match mirrored {
        true => pilot.as_ref().map_or_else(Avatar::default, |pilot| pilot.avatar()),
        false => avatar.copied().unwrap_or_default()
      };
      let before = figure.shown;
      let follow = if mirrored { 1.0 } else { (dt * 12.0).min(1.0) };
      figure.shown = before.lerp(avatar.at, follow);
      let pace = (figure.shown - before).xz().length() / dt.max(1e-4);
      figure.stride += pace * dt * 2.2;
      let swing = (figure.stride.sin() * (pace / 4.3).min(1.0)) * 0.7;
      *transform = Transform::from_translation(figure.shown)
        .with_rotation(Quat::from_rotation_y(avatar.yaw));
      children.iter().for_each(|child| {
        if let Ok(mut head) = heads.get_mut(child) {
          head.rotation = Quat::from_rotation_x(avatar.pitch)
        }
        if let Ok((limb, mut limb_transform, held)) = limbs.get_mut(child) {
          let raised = held.is_some() && avatar.held.is_some();
          limb_transform.rotation = Quat::from_rotation_x(
            swing * (limb.phase).cos() * if raised { 0.4 } else { 1.0 }
              + if raised { 0.35 } else { 0.0 }
          );
          held.into_iter().flatten().for_each(|&grip| {
            if let Ok(mut grip) = grips.get_mut(grip) {
              grip.set_if_neq(Grip(avatar.held));
            }
          })
        }
      })
    }
  )
}

fn label(
  figures: Query<(&Figure, &Player, &Tag), Without<Me>>,
  mut plates: Query<&mut Plate>
) {
  figures.iter().for_each(|(figure, player, tag)| {
    if let Ok(mut plate) = plates.get_mut(tag.0) {
      plate.at = figure.shown + Vec3::Y * 2.05;
      if plate.text != player.name {
        plate.text = player.name.clone()
      }
      if plate.color != hue(player.hue) {
        plate.color = hue(player.hue)
      }
    }
  })
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
      .add_systems(Update, (dress, mirror, reskin, animate, label).chain().run_if(plays));
  }
}
