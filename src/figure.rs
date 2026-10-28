use {crate::{hand::{Charge, Grip, grip},
             opts::opts,
             person::{Flesh, flesh, likeness, surface},
             plate::{Plate, plate},
             player::{Me, Pilot, View},
             protocol::{Avatar, Inventory, Player, Role, hue, plays},
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
  pub leg: Handle<Mesh>,
  pub left_arm: Handle<Mesh>,
  pub left_leg: Handle<Mesh>
}

impl Kit {
  pub fn mesh(&self, part: Part) -> Handle<Mesh> {
    match part {
      Part::Head => self.head.clone(),
      Part::Body => self.body.clone(),
      Part::Arm => self.arm.clone(),
      Part::Leg => self.leg.clone(),
      Part::LeftArm => self.left_arm.clone(),
      Part::LeftLeg => self.left_leg.clone()
    }
  }
}

#[derive(Component)]
pub struct Clad {
  pub material: Handle<StandardMaterial>
}

pub fn sew(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
  let mut shaped =
    |part: Part, lift: f32| meshes.add(part.mesh().translated_by(Vec3::Y * lift * PX));
  commands.insert_resource(Kit {
    head: shaped(Part::Head, 4.0),
    body: shaped(Part::Body, 0.0),
    arm: shaped(Part::Arm, -6.0),
    leg: shaped(Part::Leg, -6.0),
    left_arm: shaped(Part::LeftArm, -6.0),
    left_leg: shaped(Part::LeftLeg, -6.0)
  })
}

pub fn clothe(
  skin: &Skin,
  images: &mut Assets<Image>,
  materials: &mut Assets<StandardMaterial>
) -> Clad {
  let image = images.add(skin.image(false));
  let material = materials.add(StandardMaterial {
    base_color_texture: Some(image),
    perceptual_roughness: 0.85,
    ..default()
  });
  Clad { material }
}

#[derive(Component)]
pub struct Sculpted {
  meshes: [Handle<Mesh>; 6],
  material: Handle<Flesh>,
  image: Handle<Image>
}

fn shaped(skin: &Skin) -> [Mesh; 6] {
  let made = likeness(skin);
  Part::ALL.map(|part| match part {
    Part::Head => made.head.clone(),
    Part::Body => made.body.clone(),
    Part::Arm => made.arm.clone(),
    Part::LeftArm => made.left_arm.clone(),
    Part::Leg => made.leg.clone(),
    Part::LeftLeg => made.left_leg.clone()
  })
}

pub fn sculpted(
  skin: &Skin,
  meshes: &mut Assets<Mesh>,
  images: &mut Assets<Image>,
  fleshes: &mut Assets<Flesh>
) -> Sculpted {
  let image = images.add(surface(skin));
  Sculpted {
    meshes: shaped(skin).map(|mesh| meshes.add(mesh)),
    material: fleshes.add(flesh(skin, image.clone())),
    image
  }
}

pub fn assemble(figure: &mut EntityCommands, sculpted: &Sculpted, layers: RenderLayers) {
  let material = MeshMaterial3d(sculpted.material.clone());
  let mesh = |part: Part| Mesh3d(sculpted.meshes[part as usize].clone());
  figure.with_children(|figure| {
    figure.spawn((
      mesh(Part::Body),
      material.clone(),
      layers.clone(),
      Transform::from_translation(Part::Body.pivot())
    ));
    figure.spawn((
      Head,
      mesh(Part::Head),
      material.clone(),
      layers.clone(),
      Transform::from_translation(Part::Head.pivot())
    ));
    for (part, phase) in
      [(Part::Arm, 0.0), (Part::LeftArm, PI), (Part::Leg, PI), (Part::LeftLeg, 0.0)]
    {
      let mut limb = figure.spawn((
        Limb { phase },
        mesh(part),
        material.clone(),
        layers.clone(),
        Transform::from_translation(part.pivot())
      ));
      if part == Part::Arm {
        limb.with_child((grip(), layers.clone()));
      }
    }
  });
}

fn dress(
  pilot: Option<Res<Pilot>>,
  mut commands: Commands,
  arrivals: Query<(Entity, &Player, &Avatar, &Skin), (Without<Figure>, Without<Me>)>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut images: ResMut<Assets<Image>>,
  mut fleshes: ResMut<Assets<Flesh>>
) {
  for (entity, player, avatar, skin) in arrivals
    .iter()
    .filter(|(entity, ..)| pilot.as_ref().is_some_and(|pilot| pilot.me != *entity))
  {
    let sculpted = sculpted(skin, &mut meshes, &mut images, &mut fleshes);
    let mut figure = commands.entity(entity);
    figure.insert((
      Figure { shown: avatar.at, stride: 0.0 },
      Transform::from_translation(avatar.at),
      Visibility::default()
    ));
    assemble(&mut figure, &sculpted, RenderLayers::default());
    figure.insert(sculpted);
    let tag = commands.spawn(plate(player.name.clone(), Color::WHITE, avatar.at)).id();
    commands.entity(entity).insert(Tag(tag));
  }
}

fn reskin(
  figures: Query<(&Skin, &Sculpted), Changed<Skin>>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut images: ResMut<Assets<Image>>,
  mut fleshes: ResMut<Assets<Flesh>>
) {
  for (skin, sculpted) in figures.iter() {
    if let Some(mut image) = images.get_mut(&sculpted.image) {
      *image = surface(skin)
    }
    for (handle, mesh) in sculpted.meshes.iter().zip(shaped(skin)) {
      if let Some(mut old) = meshes.get_mut(handle) {
        *old = mesh
      }
    }
    fleshes.get_mut(&sculpted.material);
  }
}

fn mirror(
  pilot: Option<Res<Pilot>>,
  view: Res<View>,
  skins: Query<Ref<Skin>>,
  mut mirrors: Query<(Entity, &mut Visibility), With<Mirror>>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut images: ResMut<Assets<Image>>,
  mut fleshes: ResMut<Assets<Flesh>>,
  mut commands: Commands
) {
  if let Some(pilot) = pilot
    && let Ok(skin) = skins.get(pilot.me)
  {
    let shown = match (*view, opts().eye) {
      (View::Behind, _) | (_, Some(_)) => Visibility::Inherited,
      (View::First, None) => Visibility::Hidden
    };
    if skin.is_changed() || mirrors.is_empty() {
      for (old, _) in mirrors.iter() {
        commands.entity(old).despawn()
      }
      let sculpted = sculpted(&skin, &mut meshes, &mut images, &mut fleshes);
      let mut figure = commands.spawn((
        Mirror,
        Figure { shown: pilot.at, stride: 0.0 },
        Transform::from_translation(pilot.at),
        shown
      ));
      assemble(&mut figure, &sculpted, RenderLayers::default());
      figure.insert(sculpted);
    }
    for (_, mut visibility) in mirrors.iter_mut() {
      visibility.set_if_neq(shown);
    }
  }
}

fn animate(
  time: Res<Time>,
  pilot: Option<Res<Pilot>>,
  mut figures: Query<
    (
      Option<&Avatar>,
      Option<&Inventory>,
      Has<Mirror>,
      &mut Figure,
      &mut Transform,
      &Children
    ),
    Without<Me>
  >,
  inventories: Query<&Inventory>,
  mut heads: Query<&mut Transform, (With<Head>, Without<Figure>, Without<Limb>)>,
  mut limbs: Query<
    (&Limb, &mut Transform, Option<&Children>),
    (Without<Figure>, Without<Head>)
  >,
  mut grips: Query<(&mut Grip, &mut Charge)>
) {
  let dt = time.delta_secs();
  for (avatar, inventory, mirrored, mut figure, mut transform, children) in
    figures.iter_mut()
  {
    let avatar = match mirrored {
      true => pilot.as_ref().map_or_else(Avatar::default, |pilot| {
        let avatar = pilot.avatar();
        Avatar {
          at: pilot.at + Vec3::Y * pilot.lift,
          yaw: avatar.yaw + opts().turn.to_radians(),
          ..avatar
        }
      }),
      false => avatar.copied().unwrap_or_default()
    };
    let inventory = match mirrored {
      true => pilot.as_ref().and_then(|pilot| inventories.get(pilot.me).ok()),
      false => inventory
    };
    let made = inventory
      .and_then(|inventory| {
        inventory.slots.get(usize::from(avatar.hand)).copied().flatten()
      })
      .filter(|stack| Some(stack.block) == avatar.held)
      .and_then(|stack| stack.gear)
      .and_then(|gear| gear.material);
    let before = figure.shown;
    let follow = if mirrored { 1.0 } else { (dt * 12.0).min(1.0) };
    figure.shown = before.lerp(avatar.at, follow);
    let pace = (figure.shown - before).xz().length() / dt.max(1e-4);
    figure.stride += pace * dt * 2.2;
    let swing = (figure.stride.sin() * (pace / 4.3).min(1.0)) * 0.7;
    *transform = Transform::from_translation(figure.shown)
      .with_rotation(Quat::from_rotation_y(avatar.yaw));
    for child in children.iter() {
      if let Ok(mut head) = heads.get_mut(child) {
        head.rotation = Quat::from_rotation_x(avatar.pitch)
      }
      if let Ok((limb, mut limb_transform, held)) = limbs.get_mut(child) {
        let raised = held.is_some() && avatar.held.is_some();
        let casting = held.is_some() && avatar.charging;
        limb_transform.rotation = match casting {
          true => Quat::from_euler(EulerRot::YXZ, -0.3, 2.5 + avatar.pitch * 0.5, 0.0),
          false => Quat::from_rotation_x(
            swing * (limb.phase).cos() * if raised { 0.4 } else { 1.0 }
              + if raised { 0.35 } else { 0.0 }
          )
        };
        for &grip in held.into_iter().flatten() {
          if let Ok((mut grip, mut charge)) = grips.get_mut(grip) {
            grip.set_if_neq(Grip(avatar.held, made));
            charge.set_if_neq(Charge(avatar.charging));
          }
        }
      }
    }
  }
}

fn label(
  figures: Query<(&Figure, &Player, &Tag), Without<Me>>,
  mut plates: Query<&mut Plate>
) {
  for (figure, player, tag) in figures.iter() {
    if let Ok(mut plate) = plates.get_mut(tag.0) {
      plate.at = figure.shown + Vec3::Y * 2.05;
      if plate.text != player.name {
        plate.text = player.name.clone()
      }
      if plate.color != hue(player.hue) {
        plate.color = hue(player.hue)
      }
    }
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
    .remove::<(Figure, Tag, Sculpted)>()
    .despawn_related::<Children>();
}

pub struct Figures;

impl Plugin for Figures {
  fn build(&self, app: &mut App) {
    if app.world().resource::<Role>().plays() {
      crate::person::register(app)
    }
    app
      .add_observer(untag)
      .add_observer(undress)
      .add_systems(Startup, sew.run_if(plays))
      .add_systems(Update, (dress, mirror, reskin, animate, label).chain().run_if(plays));
  }
}
