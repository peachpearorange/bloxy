use {crate::{bird::Bird,
             fx::Effects,
             protocol::{Beast, Folk, Health, authority, plays}},
     bevy::prelude::*,
     std::f32::consts::FRAC_PI_2};

const LINGER: f32 = 1.1;
const TOPPLE: f32 = 0.35;
const POOF_AT: f32 = 0.85;
const SMOKE_FOR: f32 = 1.5;

#[derive(Component)]
pub struct Dying(f32);

pub fn slay(commands: &mut Commands, target: Entity) {
  commands.entity(target).insert(Dying(LINGER));
}

fn bury(time: Res<Time>, mut dying: Query<(Entity, &mut Dying)>, mut commands: Commands) {
  for (entity, mut dying) in dying.iter_mut() {
    dying.0 -= time.delta_secs();
    if dying.0 <= 0.0 {
      commands.entity(entity).despawn()
    }
  }
}

#[derive(Component)]
struct Fallen {
  since: f32,
  poofed: bool
}

#[derive(Component)]
struct Smoke(f32);

fn fall(
  time: Res<Time>,
  struck: Query<
    (Entity, &Health),
    (Changed<Health>, Without<Fallen>, Or<(With<Beast>, With<Bird>, With<Folk>)>)
  >,
  mut commands: Commands
) {
  for (entity, health) in struck.iter() {
    if health.0 == 0 {
      commands
        .entity(entity)
        .insert(Fallen { since: time.elapsed_secs(), poofed: false });
    }
  }
}

fn topple(
  time: Res<Time>,
  effects: Res<Effects>,
  mut fallen: Query<(&mut Fallen, &mut Transform, &mut Visibility)>,
  mut smoke: Query<(Entity, &mut Smoke)>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  for (mut fallen, mut transform, mut visibility) in fallen.iter_mut() {
    let gone = now - fallen.since;
    let tipped = (gone / TOPPLE).min(1.0);
    let eased = 1.0 - (1.0 - tipped) * (1.0 - tipped);
    let lift = transform.scale.y * 0.3 * eased;
    transform.rotation = transform.rotation * Quat::from_rotation_z(eased * FRAC_PI_2);
    transform.translation.y += lift;
    if gone >= POOF_AT && !fallen.poofed {
      fallen.poofed = true;
      *visibility = Visibility::Hidden;
      commands.spawn((
        Smoke(SMOKE_FOR),
        effects.emit(&effects.poof),
        Transform::from_translation(transform.translation + Vec3::Y * 0.3)
      ));
    }
  }
  for (entity, mut smoke) in smoke.iter_mut() {
    smoke.0 -= time.delta_secs();
    if smoke.0 <= 0.0 {
      commands.entity(entity).despawn()
    }
  }
}

pub struct Deaths;

impl Plugin for Deaths {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, bury.run_if(authority)).add_systems(
      PostUpdate,
      (fall, topple).chain().before(TransformSystems::Propagate).run_if(plays)
    );
  }
}
