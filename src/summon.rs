use {crate::{authority::Controller,
             block::Block,
             condition::{Conditions, Kind},
             death::Dying,
             folk::Raider,
             haunt::{Haunt, fell},
             noise::unit,
             player::{Bulk, Marched, march},
             protocol::*,
             spell::{Flourish, harm},
             voxels::Voxels},
     bevy::prelude::*,
     bevy_replicon::prelude::*,
     std::f32::consts::TAU};

const GUARD: f32 = 14.0;
const TETHER: f32 = 22.0;
const HEEL: f32 = 3.0;
const STRAYED: f32 = 32.0;
const GRAVITY: f32 = 30.0;
const LEAP: f32 = 9.4;
pub const MOST: usize = 3;

#[derive(Component)]
pub struct Servant {
  pub owner: Entity,
  pub left: f32,
  velocity: Vec3,
  reload: f32,
  stuck: f32,
  detour: f32,
  luck: u32
}

impl Band {
  fn reach(self) -> f32 {
    match self {
      Band::Phantom => 1.7,
      _ => 1.5
    }
  }
}

pub fn summon(
  commands: &mut Commands,
  band: Band,
  at: Vec3,
  owner: Entity,
  secs: f32,
  luck: u32
) {
  commands.spawn((
    Replicated,
    Folk { band, at, yaw: unit(luck, 2, 3, 4) * TAU, luck, blows: 0, aboard: false },
    Health(band.health()),
    Conditions::default(),
    Servant {
      owner,
      left: secs,
      velocity: Vec3::ZERO,
      reload: 0.5,
      stuck: 0.0,
      detour: 0.0,
      luck
    }
  ));
}

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn toward(offset: Vec2) -> f32 { (-offset.x).atan2(-offset.y) }

fn serve(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut servants: Query<(Entity, &mut Servant, &mut Folk, &Conditions), Without<Dying>>,
  owners: Query<&Avatar, With<Controller>>,
  mut foes: Query<
    (
      Entity,
      &mut Health,
      Option<&Folk>,
      Option<&Beast>,
      Option<&mut Haunt>,
      Option<&mut Raider>,
      Option<&mut Conditions>
    ),
    (Or<(With<Haunt>, With<Raider>)>, Without<Servant>, Without<Dying>)
  >,
  mut flourishes: MessageWriter<ToClients<Flourish>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  for (entity, mut servant, mut folk, conditions) in servants.iter_mut() {
    servant.left -= dt;
    servant.reload -= dt;
    servant.detour -= dt;
    let owner = owners.get(servant.owner).ok().map(|avatar| avatar.at);
    match (owner, servant.left > 0.0) {
      (Some(home), true) => {
        let Folk { band, at, yaw, .. } = *folk;
        let flying = band == Band::Phantom;
        for offset in [
          IVec3::ZERO,
          IVec3::X * 2,
          IVec3::X * -2,
          IVec3::Z * 2,
          IVec3::Z * -2,
          IVec3::Y * -2
        ] {
          voxels.ensure(at.floor().as_ivec3() + offset);
        }
        let prey = foes
          .iter()
          .filter_map(|(foe, health, folk, beast, ..)| {
            folk
              .map(|folk| folk.at)
              .or(beast.map(|beast| beast.at))
              .filter(|spot| {
                health.0 > 0 && spot.distance(at) < GUARD && spot.distance(home) < TETHER
              })
              .map(|spot| (foe, spot))
          })
          .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)));
        let pace = band.speed() * conditions.pace();
        let side = if servant.luck % 2 == 0 { 1.5 } else { -1.5 };
        let (wish, lift, facing) = match prey {
          Some((foe, spot)) => {
            let offset = spot - at;
            let facing =
              toward(offset.xz()) + if servant.detour > 0.0 { side } else { 0.0 };
            let close = offset.xz().length() < band.reach() && offset.y.abs() < 2.2;
            if close
              && servant.reload <= 0.0
              && let Ok((foe, mut health, _, _, haunt, raider, afflicted)) =
                foes.get_mut(foe)
            {
              servant.reload = 1.0;
              folk.blows = folk.blows.wrapping_add(1);
              harm(&mut commands, foe, &mut health, band.blow(), servant.owner);
              let push = offset.with_y(0.0).normalize_or_zero() * 5.0 + Vec3::Y * 3.5;
              if let Some(mut haunt) = haunt {
                haunt.shove(push)
              }
              if let Some(mut raider) = raider {
                raider.knock(push)
              }
              if band == Band::Phantom
                && let Some(mut afflicted) = afflicted
              {
                afflicted.afflict(Kind::Chilled, 3.0)
              }
            }
            (
              match close {
                true => Vec3::ZERO,
                false => ahead(facing) * pace
              },
              spot.y + 0.4 - at.y,
              facing
            )
          }
          None => {
            let offset = home - at;
            match offset.xz().length() > HEEL {
              true => {
                let facing = toward(offset.xz());
                (ahead(facing) * pace, home.y + 1.0 - at.y, facing)
              }
              false => (Vec3::ZERO, home.y + 1.0 - at.y, yaw)
            }
          }
        };
        let vertical = match flying {
          true => (lift * 2.0).clamp(-3.0, 3.0),
          false => (servant.velocity.y - GRAVITY * dt).max(-40.0)
        };
        let bulk = if flying { Bulk { half: 0.3, tall: 1.7 } } else { Bulk::PERSON };
        let Marched { at: moved, velocity, blocked, landed } =
          march(&voxels, bulk, at, Vec3::new(wish.x, vertical, wish.z), dt);
        servant.velocity = match blocked && landed && !flying {
          true => velocity.with_y(LEAP),
          false => velocity
        };
        let progress = (moved - at).xz().length() / dt.max(1e-4);
        servant.stuck = match wish != Vec3::ZERO && progress < 0.4 {
          true => servant.stuck + dt,
          false => (servant.stuck - dt).max(0.0)
        };
        if servant.stuck > 1.2 {
          servant.stuck = 0.0;
          servant.detour = 1.4
        }
        let moved = match moved.distance(home) > STRAYED || moved.y < -16.0 {
          true => home + Vec3::new(1.0, 0.5, 1.0),
          false => moved
        };
        let next = Folk { at: moved, yaw: facing.rem_euclid(TAU), ..*folk };
        folk.set_if_neq(next);
      }
      _ => {
        flourishes.write(ToClients {
          targets: SendTargets::All,
          message: Flourish {
            spell: Block::SpellAnimate,
            at: folk.at + Vec3::Y,
            reach: 0.8
          }
        });
        fell(&mut commands, entity)
      }
    }
  }
}

pub struct Summoning;

impl Plugin for Summoning {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, serve.run_if(authority).run_if(resource_exists::<Voxels>));
  }
}
