use {crate::{authority::{Controller, player_of},
             beast::Roam,
             bird::Bird,
             block::Block,
             condition::{Conditions, Kind},
             death::Dying,
             figure::Clad,
             folk::{Raider, Vigour, ray_box},
             fx::{Aura, Drift, Effects, flare},
             hand::{Charge, Grip, HandSystems},
             haunt::{Blamed, Haunt, fell},
             menu::Menu,
             player::{Me, Pilot, Poke, Selected, captured},
             protocol::*,
             skill::Skills,
             stance::Stance,
             summon::{MOST, Servant, summon},
             voxels::Voxels},
     bevy::{light::NotShadowCaster,
            platform::collections::HashMap,
            prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_hanabi::EffectAsset,
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

const BOLT_LIFE: f32 = 2.2;
const SEEK: f32 = 24.0;
const FLOURISH_FOR: f32 = 0.6;
const WISP_LUMENS: f32 = 90_000.0;

pub fn cooldown(spell: Block) -> f32 {
  match spell {
    Block::SpellFirebolt => 0.8,
    Block::SpellFrost => 0.6,
    Block::SpellMissile => 1.5,
    Block::SpellHeal => 4.0,
    Block::SpellThunder => 3.0,
    Block::SpellLight => 5.0,
    Block::SpellFireball => 2.5,
    Block::SpellAnimate | Block::SpellGuardian => 12.0,
    Block::SpellInvisibility => 15.0,
    _ => 8.0
  }
}

pub fn recharge(spell: Block, stance: Option<Stance>) -> f32 {
  cooldown(spell) * stance.map_or(1.0, Stance::recharge)
}

fn offensive(spell: Block) -> bool {
  matches!(
    spell,
    Block::SpellFirebolt
      | Block::SpellFrost
      | Block::SpellMissile
      | Block::SpellThunder
      | Block::SpellFireball
      | Block::SpellIceStorm
  )
}

fn hue(spell: Block) -> [f32; 3] {
  match spell {
    Block::SpellFirebolt => [1.0, 0.45, 0.1],
    Block::SpellFrost => [0.45, 0.8, 1.0],
    Block::SpellMissile => [0.75, 0.45, 1.0],
    Block::SpellHeal => [0.4, 1.0, 0.45],
    Block::SpellHaste => [1.0, 0.85, 0.3],
    Block::SpellStoneskin => [0.7, 0.68, 0.6],
    Block::SpellLeap => [0.75, 0.92, 1.0],
    Block::SpellLight => [1.0, 0.9, 0.6],
    Block::SpellFireball => [1.0, 0.5, 0.12],
    Block::SpellIceStorm => [0.65, 0.88, 1.0],
    Block::SpellAnimate => [0.6, 1.0, 0.45],
    Block::SpellGuardian => [0.55, 0.75, 1.0],
    Block::SpellInvisibility => [0.75, 0.6, 0.95],
    _ => [0.55, 0.7, 1.0]
  }
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Invoke {
  pub spell: Block,
  pub toward: Vec3
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Flourish {
  pub spell: Block,
  pub at: Vec3,
  pub reach: f32
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Bolt {
  pub spell: Block,
  pub at: Vec3
}

#[derive(Component)]
struct Flight {
  velocity: Vec3,
  caster: Entity,
  life: f32,
  damage: u8,
  rank: u8,
  homing: Option<Entity>
}

#[derive(Component, Default)]
pub struct Spellbook {
  ready: HashMap<Block, f32>,
  pub lunge: f32
}

#[derive(Component)]
struct Storm {
  at: Vec3,
  left: f32,
  radius: f32,
  damage: u8,
  caster: Entity,
  hail: f32,
  sting: f32
}

pub struct Target {
  pub entity: Entity,
  pub centre: Vec3,
  pub low: Vec3,
  pub high: Vec3
}

pub fn targets(
  folk: &Query<(Entity, &Folk), Without<Dying>>,
  beasts: &Query<(Entity, &Beast), Without<Dying>>,
  birds: &Query<(Entity, &Bird), Without<Dying>>
) -> Vec<Target> {
  let people =
    folk.iter().filter(|(_, folk)| folk.band.hostile()).map(|(entity, folk)| {
      let scale = folk.band.scale();
      let half = Vec3::new(0.4 * scale, 0.0, 0.4 * scale);
      Target {
        entity,
        centre: folk.at + Vec3::Y * scale,
        low: folk.at - half,
        high: folk.at + half + Vec3::Y * 1.95 * scale
      }
    });
  let animals = beasts.iter().map(|(entity, beast)| Target {
    entity,
    centre: beast.at + Vec3::Y * 0.4,
    low: beast.at - Vec3::new(0.5, 0.0, 0.5),
    high: beast.at + Vec3::new(0.5, 1.0, 0.5)
  });
  let fowl = birds.iter().map(|(entity, bird)| Target {
    entity,
    centre: bird.at + Vec3::Y * 0.3,
    low: bird.at - Vec3::new(0.4, 0.0, 0.4),
    high: bird.at + Vec3::new(0.4, 0.7, 0.4)
  });
  people.chain(animals).chain(fowl).collect()
}

fn invoke(
  mut invocations: MessageReader<FromClient<Invoke>>,
  time: Res<Time>,
  mut players: Query<
    (
      &Controller,
      (
        Entity,
        &Avatar,
        &Skills,
        &mut Inventory,
        &mut Health,
        &Vigour,
        &mut Conditions,
        &mut Spellbook
      )
    ),
    Without<Haunt>
  >,
  folk: Query<(Entity, &Folk), Without<Dying>>,
  beasts: Query<(Entity, &Beast), Without<Dying>>,
  birds: Query<(Entity, &Bird), Without<Dying>>,
  mut creatures: Query<
    (&mut Health, Option<&mut Haunt>, Option<&mut Raider>, Option<&mut Roam>),
    Without<Controller>
  >,
  servants: Query<(Entity, &Servant), Without<Dying>>,
  voxels: Res<Voxels>,
  mut flourishes: MessageWriter<ToClients<Flourish>>,
  mut notices: MessageWriter<ToClients<Notice>>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  for &FromClient { client_id, message: Invoke { spell, toward } } in invocations.read() {
    if let Some((
      caster,
      avatar,
      skills,
      mut inventory,
      mut health,
      vigour,
      mut conditions,
      mut book
    )) = player_of(players.iter_mut(), client_id)
      && spell.spell()
      && toward.is_finite()
      && book.ready.get(&spell).is_none_or(|&ready| now >= ready)
    {
      let rank = skills.rank(spell);
      let thrifty =
        rand_unit(caster, now) < skills.geared(Some(&inventory), avatar.hand).thrift;
      let paid = thrifty || inventory.take(spell);
      match paid {
        false => {
          notices.write(ToClients {
            targets: SendTargets::Single(client_id),
            message: Notice(format!("No {} left: craft more", spell.name()))
          });
        }
        true => {
          book.ready.insert(spell, now + recharge(spell, skills.stance) * 0.9);
          if offensive(spell) {
            conditions.0.retain(|condition| condition.kind != Kind::Invisible)
          }
          let toward = toward.normalize_or(Vec3::NEG_Z);
          let eye = avatar.at + Vec3::Y * EYE;
          let mut launch =
            |speed: f32, damage: u8, homing: Option<Entity>, spread: Vec3| {
              let velocity = (toward + spread).normalize_or(toward) * speed;
              commands.spawn((
                Replicated,
                Bolt { spell, at: eye + toward * 0.6 },
                Flight { velocity, caster, life: BOLT_LIFE, damage, rank, homing }
              ));
            };
          let mut flourish = |at: Vec3, reach: f32| {
            flourishes.write(ToClients {
              targets: SendTargets::All,
              message: Flourish { spell, at, reach }
            });
          };
          match spell {
            Block::SpellFirebolt => launch(22.0, 4 + rank * 2, None, Vec3::ZERO),
            Block::SpellFrost => launch(34.0, 2 + rank, None, Vec3::ZERO),
            Block::SpellMissile => {
              let mut seen: Vec<(Entity, f32)> = targets(&folk, &beasts, &birds)
                .into_iter()
                .filter(|target| {
                  let away = target.centre - eye;
                  away.length() < SEEK && away.normalize_or_zero().dot(toward) > 0.6
                })
                .map(|target| (target.entity, target.centre.distance(eye)))
                .collect();
              seen.sort_by(|a, b| a.1.total_cmp(&b.1));
              for dart in 0..2 + rank {
                let homing =
                  (!seen.is_empty()).then(|| seen[usize::from(dart) % seen.len()].0);
                let fan = (f32::from(dart) - f32::from(1 + rank) / 2.0) * 0.18;
                launch(
                  15.0,
                  2,
                  homing,
                  Quat::from_rotation_y(fan) * Vec3::Y * 0.25
                    + toward.cross(Vec3::Y) * fan
                )
              }
            }
            Block::SpellHeal => {
              health.0 = (health.0 + 6 + rank * 2).min(vigour.most);
              if rank >= 2 {
                conditions.afflict(Kind::Regenerating, 8.0)
              }
              conditions.0.retain(|condition| condition.kind != Kind::Poisoned);
              flourish(avatar.at + Vec3::Y, 1.2)
            }
            Block::SpellHaste => {
              conditions.afflict(Kind::Hasted, 25.0 + f32::from(rank) * 15.0);
              flourish(avatar.at + Vec3::Y, 1.0)
            }
            Block::SpellStoneskin => {
              conditions.afflict(Kind::Stoneskin, 30.0 + f32::from(rank) * 15.0);
              flourish(avatar.at + Vec3::Y, 1.0)
            }
            Block::SpellLeap => {
              conditions.afflict(Kind::Leaping, 40.0 + f32::from(rank) * 20.0);
              flourish(avatar.at + Vec3::Y * 0.3, 1.0)
            }
            Block::SpellLight => {
              conditions.afflict(Kind::Lit, 120.0 + f32::from(rank) * 60.0);
              flourish(avatar.at + Vec3::Y * 2.2, 0.8)
            }
            Block::SpellInvisibility => {
              conditions.afflict(Kind::Invisible, 30.0 + f32::from(rank) * 10.0);
              flourish(avatar.at + Vec3::Y, 1.0)
            }
            Block::SpellFireball => launch(18.0, 5 + rank * 2, None, Vec3::ZERO),
            Block::SpellIceStorm => {
              let point = voxels
                .cast(eye, toward, SEEK)
                .map_or(eye + toward * SEEK, |hit| (hit.at + hit.normal).as_vec3() + 0.5);
              let radius = 3.5 + f32::from(rank) * 0.5;
              commands.spawn(Storm {
                at: point,
                left: 6.0 + f32::from(rank),
                radius,
                damage: 2 + rank,
                caster,
                hail: 0.0,
                sting: 0.0
              });
              flourish(point, radius)
            }
            Block::SpellAnimate | Block::SpellGuardian => {
              let (band, secs, count) = match spell {
                Block::SpellAnimate => (
                  Band::Thrall,
                  45.0 + f32::from(rank) * 15.0,
                  1 + usize::from(rank >= 2)
                ),
                _ => (Band::Phantom, 40.0 + f32::from(rank) * 15.0, 1)
              };
              let mut mine: Vec<(Entity, f32)> = servants
                .iter()
                .filter(|(_, servant)| servant.owner == caster)
                .map(|(entity, servant)| (entity, servant.left))
                .collect();
              mine.sort_by(|a, b| a.1.total_cmp(&b.1));
              for &(old, _) in mine.iter().take((mine.len() + count).saturating_sub(MOST))
              {
                fell(&mut commands, old)
              }
              let across = toward.cross(Vec3::Y).normalize_or(Vec3::X);
              for index in 0..count {
                let side = if index % 2 == 0 { 1.2 } else { -1.2 };
                let at = avatar.at + across * side + Vec3::Y * 0.2;
                let luck = crate::noise::hash(
                  caster.index_u32(),
                  (now * 100.0) as i32,
                  index as i32,
                  9
                );
                summon(&mut commands, band, at, caster, secs, luck);
                flourish(at + Vec3::Y, 1.0)
              }
            }
            _ => {
              let reach = 4.0 + f32::from(rank) * 0.75;
              let damage = 3 + rank;
              for target in targets(&folk, &beasts, &birds)
                .into_iter()
                .filter(|target| target.centre.distance(avatar.at + Vec3::Y) < reach)
              {
                let push = (target.centre - avatar.at).with_y(0.0).normalize_or_zero()
                  * 9.0
                  + Vec3::Y * 6.0;
                if let Ok((mut health, haunt, raider, roam)) =
                  creatures.get_mut(target.entity)
                {
                  harm(&mut commands, target.entity, &mut health, damage, caster);
                  if let Some(mut haunt) = haunt {
                    haunt.shove(push)
                  }
                  if let Some(mut raider) = raider {
                    raider.knock(push)
                  }
                  if let Some(mut roam) = roam {
                    roam.knock(push)
                  }
                }
              }
              flourish(avatar.at + Vec3::Y * 0.8, reach)
            }
          }
        }
      }
    }
  }
}

fn rand_unit(entity: Entity, now: f32) -> f32 {
  crate::noise::unit(entity.index_u32(), (now * 1000.0) as i32, 7, 1)
}

pub fn hurl(
  commands: &mut Commands,
  spell: Block,
  from: Vec3,
  velocity: Vec3,
  damage: u8,
  caster: Entity,
  homing: Option<Entity>
) {
  commands.spawn((Replicated, Bolt { spell, at: from }, Flight {
    velocity,
    caster,
    life: BOLT_LIFE,
    damage,
    rank: 2,
    homing
  }));
}

pub fn harm(
  commands: &mut Commands,
  target: Entity,
  health: &mut Health,
  amount: u8,
  by: Entity
) {
  if health.0 > 0 {
    health.0 = health.0.saturating_sub(amount);
    commands.entity(target).insert(Blamed(by));
    if health.0 == 0 {
      fell(commands, target)
    }
  }
}

fn fly(
  time: Res<Time>,
  voxels: Res<Voxels>,
  mut bolts: Query<(Entity, &mut Bolt, &mut Flight)>,
  folk: Query<(Entity, &Folk), Without<Dying>>,
  beasts: Query<(Entity, &Beast), Without<Dying>>,
  birds: Query<(Entity, &Bird), Without<Dying>>,
  mut creatures: Query<
    (&mut Health, Option<&mut Conditions>, Option<&mut Haunt>),
    Without<Controller>
  >,
  mut flourishes: MessageWriter<ToClients<Flourish>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  let all = targets(&folk, &beasts, &birds);
  for (entity, mut bolt, mut flight) in bolts.iter_mut() {
    if let Some(chased) = flight.homing
      && let Some(target) = all.iter().find(|target| target.entity == chased)
    {
      let speed = flight.velocity.length();
      let wanted = (target.centre - bolt.at).normalize_or_zero() * speed;
      flight.velocity =
        flight.velocity.lerp(wanted, (dt * 6.0).min(1.0)).normalize_or_zero() * speed
    }
    let step = flight.velocity * dt;
    let (from, length) = (bolt.at, step.length());
    let direction = step.normalize_or_zero();
    let wall = voxels
      .cast_for(from, direction, length, Block::solid)
      .map(|hit| (hit.at.as_vec3() + 0.5).distance(from) - 0.5);
    let struck = all
      .iter()
      .filter_map(|target| {
        ray_box(from, direction, target.low, target.high)
          .filter(|&near| near <= length)
          .map(|near| (target.entity, near))
      })
      .min_by(|a, b| a.1.total_cmp(&b.1));
    flight.life -= dt;
    let ended = match (struck, wall) {
      (Some((target, near)), wall) if wall.is_none_or(|wall| near <= wall) => {
        if bolt.spell != Block::SpellFireball
          && let Ok((mut health, conditions, haunt)) = creatures.get_mut(target)
        {
          harm(&mut commands, target, &mut health, flight.damage, flight.caster);
          if let Some(mut conditions) = conditions {
            match bolt.spell {
              Block::SpellFirebolt if flight.rank >= 2 => {
                conditions.afflict(Kind::Burning, 4.0)
              }
              Block::SpellFrost => {
                conditions.afflict(Kind::Chilled, 4.0 + f32::from(flight.rank) * 2.0)
              }
              _ => ()
            }
          }
          if let Some(mut haunt) = haunt {
            haunt.shove(direction.with_y(0.0) * 4.0 + Vec3::Y * 2.0)
          }
        }
        Some(from + direction * near)
      }
      (_, Some(wall)) => Some(from + direction * wall.max(0.0)),
      _ if flight.life <= 0.0 => Some(from),
      _ => None
    };
    match ended {
      Some(at) => {
        let blast = match bolt.spell {
          Block::SpellFireball => 3.5 + f32::from(flight.rank) * 0.5,
          _ => 0.7
        };
        if bolt.spell == Block::SpellFireball {
          for target in all.iter().filter(|target| target.centre.distance(at) < blast) {
            if let Ok((mut health, conditions, haunt)) = creatures.get_mut(target.entity)
            {
              harm(
                &mut commands,
                target.entity,
                &mut health,
                flight.damage,
                flight.caster
              );
              if let Some(mut conditions) = conditions {
                conditions.afflict(Kind::Burning, 3.0)
              }
              if let Some(mut haunt) = haunt {
                haunt.shove(
                  (target.centre - at).with_y(0.0).normalize_or_zero() * 6.0
                    + Vec3::Y * 4.0
                )
              }
            }
          }
        }
        flourishes.write(ToClients {
          targets: SendTargets::All,
          message: Flourish { spell: bolt.spell, at, reach: blast }
        });
        commands.entity(entity).despawn()
      }
      None => bolt.at = from + step
    }
  }
}

fn rage(
  time: Res<Time>,
  mut storms: Query<(Entity, &mut Storm)>,
  folk: Query<(Entity, &Folk), Without<Dying>>,
  beasts: Query<(Entity, &Beast), Without<Dying>>,
  birds: Query<(Entity, &Bird), Without<Dying>>,
  mut creatures: Query<(&mut Health, Option<&mut Conditions>), Without<Controller>>,
  mut flourishes: MessageWriter<ToClients<Flourish>>,
  mut commands: Commands
) {
  let dt = time.delta_secs();
  for (entity, mut storm) in storms.iter_mut() {
    storm.left -= dt;
    storm.hail += dt;
    storm.sting += dt;
    if storm.hail >= 0.2 {
      storm.hail = 0.0;
      let roll = |salt: i32| {
        crate::noise::unit(entity.index_u32(), (storm.left * 50.0) as i32, salt, 3)
      };
      let angle = roll(1) * std::f32::consts::TAU;
      let spot = storm.at
        + Vec3::new(angle.cos(), 0.0, angle.sin()) * storm.radius * roll(2).sqrt();
      flourishes.write(ToClients {
        targets: SendTargets::All,
        message: Flourish {
          spell: Block::SpellFrost,
          at: spot + Vec3::Y * roll(3) * 2.0,
          reach: 0.5
        }
      });
    }
    if storm.sting >= 1.0 {
      storm.sting = 0.0;
      for target in targets(&folk, &beasts, &birds)
        .into_iter()
        .filter(|target| {
          target.centre.with_y(0.0).distance(storm.at.with_y(0.0)) < storm.radius
        })
        .filter(|target| (target.centre.y - storm.at.y).abs() < 4.0)
      {
        if let Ok((mut health, conditions)) = creatures.get_mut(target.entity) {
          harm(&mut commands, target.entity, &mut health, storm.damage, storm.caster);
          if let Some(mut conditions) = conditions {
            conditions.afflict(Kind::Chilled, 3.0)
          }
        }
      }
    }
    if storm.left <= 0.0 {
      commands.entity(entity).despawn()
    }
  }
}

fn unveil(
  mut strikes: MessageReader<FromClient<Strike>>,
  mut players: Query<(&Controller, &mut Conditions)>
) {
  for &FromClient { client_id, .. } in strikes.read() {
    if let Some(mut conditions) = player_of(players.iter_mut(), client_id)
      && conditions.has(Kind::Invisible)
    {
      conditions.0.retain(|condition| condition.kind != Kind::Invisible)
    }
  }
}

fn veil(
  players: Query<(Entity, &Clad, &Conditions), With<Player>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut hidden: Local<Vec<Entity>>
) {
  for (entity, clad, conditions) in players.iter() {
    let unseen = conditions.has(Kind::Invisible);
    if unseen != hidden.contains(&entity)
      && let Some(mut material) = materials.get_mut(&clad.material)
    {
      match unseen {
        true => {
          material.alpha_mode = AlphaMode::Blend;
          material.base_color = Color::WHITE.with_alpha(0.15);
          hidden.push(entity)
        }
        false => {
          material.alpha_mode = AlphaMode::Opaque;
          material.base_color = Color::WHITE;
          hidden.retain(|&other| other != entity)
        }
      }
    }
  }
}

#[derive(Resource, Default)]
pub struct Recharge(pub HashMap<Block, f32>);

pub fn charge(spell: Block) -> f32 {
  match spell {
    Block::SpellFirebolt | Block::SpellFrost => 0.35,
    Block::SpellMissile | Block::SpellLight | Block::SpellLeap => 0.5,
    Block::SpellFireball | Block::SpellIceStorm | Block::SpellThunder => 0.8,
    Block::SpellAnimate | Block::SpellGuardian => 1.0,
    _ => 0.6
  }
}

#[derive(Resource, Default)]
pub struct Charging {
  spell: Option<Block>,
  since: f32,
  takes: f32
}

impl Charging {
  pub fn level(&self, now: f32) -> Option<f32> {
    self.spell.map(|_| ((now - self.since) / self.takes.max(0.05)).min(1.0))
  }
}

fn cast(
  time: Res<Time>,
  buttons: Res<ButtonInput<MouseButton>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  mut pilot: ResMut<Pilot>,
  selected: Res<Selected>,
  inventories: Query<&Inventory>,
  skills: Query<&Skills>,
  mut recharge: ResMut<Recharge>,
  mut charging: ResMut<Charging>,
  mut invocations: MessageWriter<Invoke>,
  mut pokes: MessageWriter<Poke>,
  mut sounds: MessageWriter<crate::audio::Sound>
) {
  let now = time.elapsed_secs();
  let free = captured(&cursor, &menu);
  let held = inventories
    .get(pilot.me)
    .ok()
    .and_then(|inventory| inventory.slots[selected.0])
    .map(|stack| stack.block)
    .filter(|block| block.spell());
  match charging.spell {
    Some(spell) if free && held == Some(spell) && buttons.pressed(MouseButton::Right) => {
    }
    Some(spell) => {
      if free && held == Some(spell) && now - charging.since >= charging.takes {
        let stance = skills.get(pilot.me).ok().and_then(|skills| skills.stance);
        recharge.0.insert(spell, now + crate::spell::recharge(spell, stance));
        invocations.write(Invoke { spell, toward: pilot.facing() * Vec3::NEG_Z });
        sounds.write(crate::audio::Sound::here(crate::audio::Cue::Cast));
        pokes.write(Poke);
      }
      charging.spell = None
    }
    None => {
      if free
        && buttons.just_pressed(MouseButton::Right)
        && let Some(spell) = held
        && recharge.0.get(&spell).is_none_or(|&ready| now >= ready)
      {
        let quick = skills.get(pilot.me).map_or(1.0, |skills| {
          skills.geared(inventories.get(pilot.me).ok(), pilot.hand).cast
        });
        *charging =
          Charging { spell: Some(spell), since: now, takes: charge(spell) * quick }
      }
    }
  }
  if pilot.charging != charging.spell.is_some() {
    pilot.charging = charging.spell.is_some()
  }
}

fn aura(spell: Block) -> Aura {
  let [r, g, b] = hue(spell);
  let tint = |by: f32| Vec4::new(r * by, g * by, b * by, 1.0);
  let (drift, rate, speed, size, life) = match spell {
    Block::SpellFirebolt => (Drift::Rise, 40.0, 0.5, 0.0525, 0.7),
    Block::SpellFireball => (Drift::Rise, 60.0, 0.6, 0.075, 0.9),
    Block::SpellFrost => (Drift::Fall, 28.0, 0.25, 0.0375, 1.2),
    Block::SpellIceStorm => (Drift::Swirl, 50.0, 0.6, 0.045, 0.9),
    Block::SpellMissile => (Drift::Swirl, 40.0, 0.5, 0.042, 0.8),
    Block::SpellHeal => (Drift::Rise, 18.0, 0.25, 0.045, 1.4),
    Block::SpellHaste => (Drift::Rays, 36.0, 1.7, 0.032, 0.4),
    Block::SpellStoneskin => (Drift::Fall, 22.0, 0.15, 0.045, 1.0),
    Block::SpellLeap => (Drift::Rise, 30.0, 0.7, 0.033, 0.8),
    Block::SpellLight => (Drift::Rays, 30.0, 1.1, 0.035, 0.5),
    Block::SpellThunder => (Drift::Rays, 50.0, 2.8, 0.026, 0.3),
    Block::SpellAnimate => (Drift::Swirl, 25.0, 0.3, 0.045, 1.2),
    Block::SpellGuardian => (Drift::Swirl, 30.0, 0.45, 0.045, 1.1),
    _ => (Drift::Rise, 14.0, 0.2, 0.0525, 1.6)
  };
  let glow = match spell {
    Block::SpellFirebolt | Block::SpellFireball => [
      Vec4::new(9.0, 5.0, 1.5, 1.0),
      Vec4::new(5.0, 1.4, 0.2, 0.9),
      Vec4::new(0.8, 0.1, 0.02, 0.0)
    ],
    Block::SpellFrost | Block::SpellIceStorm => {
      [Vec4::new(6.0, 7.0, 8.0, 1.0), tint(4.0), tint(1.5)]
    }
    Block::SpellThunder => [Vec4::new(10.0, 10.0, 12.0, 1.0), tint(6.0), tint(2.0)],
    Block::SpellStoneskin => [tint(2.0), tint(1.2), tint(0.4)],
    Block::SpellInvisibility => [tint(2.5), tint(1.5), tint(0.3)],
    _ => [tint(6.0), tint(3.5), tint(1.0)]
  };
  Aura { drift, glow, rate, speed, size, life }
}

#[derive(Resource)]
struct Sparks {
  ball: Handle<Mesh>,
  looks: HashMap<Block, Handle<StandardMaterial>>,
  auras: HashMap<Block, Handle<EffectAsset>>,
  flares: HashMap<Block, Handle<EffectAsset>>
}

fn glowing(
  materials: &mut Assets<StandardMaterial>,
  [r, g, b]: [f32; 3],
  alpha: f32
) -> Handle<StandardMaterial> {
  materials.add(StandardMaterial {
    base_color: Color::srgba(r, g, b, alpha),
    emissive: LinearRgba::rgb(r, g, b) * 7.0,
    alpha_mode: AlphaMode::Blend,
    unlit: true,
    ..default()
  })
}

fn kindle(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut effects: ResMut<Assets<EffectAsset>>
) {
  let looks = Block::SPELLS
    .iter()
    .map(|&spell| (spell, glowing(&mut materials, hue(spell), 0.9)))
    .collect();
  let auras = Block::SPELLS
    .iter()
    .map(|&spell| (spell, effects.add(crate::fx::aura(&aura(spell)))))
    .collect();
  let flares = Block::SPELLS
    .iter()
    .map(|&spell| (spell, effects.add(flare(aura(spell).glow))))
    .collect();
  commands.insert_resource(Sparks {
    ball: meshes.add(Sphere::new(0.5).mesh().uv(12, 8)),
    looks,
    auras,
    flares
  })
}

const FLASH: f32 = 0.22;

#[derive(Component)]
struct Kindled {
  spell: Block,
  charge: f32,
  flash: f32,
  orb: Entity
}

#[derive(Component)]
struct Fleeting(f32);

fn ignite(
  grips: Query<(Entity, &Grip, Option<&Children>), Changed<Grip>>,
  kindled: Query<&Kindled>,
  sparks: Res<Sparks>,
  effects: Res<Effects>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  for (grip, &Grip(held, _), children) in grips.iter() {
    let lit: Vec<(Entity, Block)> = children
      .into_iter()
      .flatten()
      .filter_map(|&child| kindled.get(child).ok().map(|kindled| (child, kindled.spell)))
      .collect();
    let wanted = held.filter(|block| block.spell());
    if lit.iter().map(|&(_, spell)| spell).ne(wanted) {
      for &(old, _) in lit.iter() {
        commands.entity(old).despawn()
      }
      if let Some(spell) = wanted
        && let Some(asset) = sparks.auras.get(&spell)
      {
        let [r, g, b] = hue(spell);
        let orb = commands
          .spawn((
            Mesh3d(sparks.ball.clone()),
            MeshMaterial3d(glowing(&mut materials, hue(spell), 0.8)),
            NotShadowCaster,
            Transform::from_scale(Vec3::ZERO),
            PointLight {
              intensity: 0.0,
              range: 5.0,
              color: Color::srgb(r, g, b),
              shadow_maps_enabled: false,
              ..default()
            }
          ))
          .id();
        commands
          .spawn((
            Kindled { spell, charge: 0.0, flash: FLASH, orb },
            effects.emit(asset),
            Transform::default(),
            Visibility::default(),
            ChildOf(grip)
          ))
          .add_child(orb);
      }
    }
  }
}

fn brighten(
  time: Res<Time>,
  sparks: Res<Sparks>,
  effects: Res<Effects>,
  grips: Query<(&Charge, &Children, Has<crate::hand::Fist>), With<Grip>>,
  own: Res<Charging>,
  mut kindled: Query<(&mut Kindled, &GlobalTransform)>,
  mut orbs: Query<(&mut Transform, &MeshMaterial3d<StandardMaterial>, &mut PointLight)>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut fleeting: Query<(Entity, &mut Fleeting)>,
  mut commands: Commands
) {
  let (dt, now) = (time.delta_secs(), time.elapsed_secs());
  for (&Charge(charging), children, fist) in grips.iter() {
    for child in children.iter() {
      if let Ok((mut lit, place)) = kindled.get_mut(child) {
        let before = lit.charge;
        let takes = match fist && own.spell.is_some() {
          true => own.takes,
          false => charge(lit.spell)
        };
        lit.charge = match charging {
          true => before + dt / takes.max(0.05),
          false => 0.0
        };
        lit.flash += dt;
        if before < 1.0
          && lit.charge >= 1.0
          && let Some(asset) = sparks.flares.get(&lit.spell)
        {
          lit.flash = 0.0;
          commands.spawn((
            Fleeting(1.0),
            effects.emit(asset),
            Transform::from_translation(place.translation())
          ));
        }
        let level = lit.charge.min(1.0);
        let flash = (1.0 - lit.flash / FLASH).max(0.0);
        let full = f32::from(u8::from(lit.charge >= 1.0));
        if let Ok((mut transform, material, mut light)) = orbs.get_mut(lit.orb) {
          let pulse = full * (now * 14.0).sin() * 0.012;
          transform.scale = Vec3::splat(
            f32::from(u8::from(charging)) * 0.02 + level * 0.1 + flash * 0.16 + pulse
          );
          light.intensity = level * 40_000.0 + full * 20_000.0 + flash * 300_000.0;
          if (charging || before > 0.0 || flash > 0.0)
            && let Some(mut material) = materials.get_mut(&material.0)
          {
            let [r, g, b] = hue(lit.spell);
            let whiten = |channel: f32| channel + (1.0 - channel) * flash;
            let bright = 1.5 + level * 3.0 + flash * 12.0;
            material.base_color = Color::LinearRgba(LinearRgba::new(
              whiten(r) * bright,
              whiten(g) * bright,
              whiten(b) * bright,
              0.55 + 0.35 * level.max(flash)
            ))
          }
        }
      }
    }
  }
  for (entity, mut left) in fleeting.iter_mut() {
    left.0 -= dt;
    if left.0 <= 0.0 {
      commands.entity(entity).despawn()
    }
  }
}

#[derive(Component)]
struct Shown(Vec3);

fn show_bolts(
  time: Res<Time>,
  sparks: Res<Sparks>,
  arrivals: Query<(Entity, &Bolt), Without<Shown>>,
  mut bolts: Query<(&Bolt, &mut Shown, &mut Transform)>,
  mut commands: Commands
) {
  for (entity, bolt) in arrivals.iter() {
    let [r, g, b] = hue(bolt.spell);
    let size = match bolt.spell {
      Block::SpellMissile => 0.16,
      Block::SpellFrost => 0.14,
      _ => 0.26
    };
    commands
      .entity(entity)
      .insert((
        Shown(bolt.at),
        Mesh3d(sparks.ball.clone()),
        MeshMaterial3d(sparks.looks.get(&bolt.spell).cloned().unwrap_or_default()),
        Transform::from_translation(bolt.at).with_scale(Vec3::splat(size)),
        Visibility::default()
      ))
      .with_child(PointLight {
        intensity: 40_000.0,
        range: 7.0,
        color: Color::srgb(r, g, b),
        shadow_maps_enabled: false,
        ..default()
      });
  }
  let dt = time.delta_secs();
  for (bolt, mut shown, mut transform) in bolts.iter_mut() {
    shown.0 = shown.0.lerp(bolt.at, (dt * 20.0).min(1.0));
    transform.translation = shown.0
  }
}

#[derive(Component)]
struct Bloom {
  age: f32,
  reach: f32,
  flat: bool,
  glow: LinearRgba
}

fn bloom(
  time: Res<Time>,
  sparks: Res<Sparks>,
  mut flourishes: MessageReader<Flourish>,
  mut blooms: Query<(
    Entity,
    &mut Bloom,
    &mut Transform,
    &MeshMaterial3d<StandardMaterial>
  )>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  for &Flourish { spell, at, reach } in flourishes.read() {
    let [r, g, b] = hue(spell);
    commands.spawn((
      Bloom {
        age: 0.0,
        reach,
        flat: matches!(spell, Block::SpellThunder | Block::SpellIceStorm),
        glow: LinearRgba::rgb(r, g, b) * 7.0
      },
      Mesh3d(sparks.ball.clone()),
      MeshMaterial3d(glowing(&mut materials, hue(spell), 0.7)),
      Transform::from_translation(at).with_scale(Vec3::splat(0.05))
    ));
  }
  let dt = time.delta_secs();
  for (entity, mut bloom, mut transform, material) in blooms.iter_mut() {
    bloom.age += dt;
    let grown = (bloom.age / FLOURISH_FOR).min(1.0);
    let size = bloom.reach * 2.0 * (1.0 - (1.0 - grown) * (1.0 - grown));
    transform.scale = match bloom.flat {
      true => Vec3::new(size, size * 0.12, size),
      false => Vec3::splat(size)
    };
    if let Some(mut material) = materials.get_mut(&material.0) {
      let fade = 1.0 - grown;
      material.base_color.set_alpha(0.7 * fade);
      material.emissive = bloom.glow * fade
    }
    if bloom.age >= FLOURISH_FOR {
      commands.entity(entity).despawn()
    }
  }
}

#[derive(Component)]
struct Wisp(Entity);

fn wisps(
  time: Res<Time>,
  sparks: Res<Sparks>,
  pilot: Option<Res<Pilot>>,
  lit: Query<(Entity, &Conditions, &Avatar, Has<Me>)>,
  mut floating: Query<(Entity, &Wisp, &mut Transform)>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  for (entity, wisp, mut transform) in floating.iter_mut() {
    match lit.get(wisp.0) {
      Ok((_, conditions, avatar, me)) if conditions.has(Kind::Lit) => {
        let feet = match (me, &pilot) {
          (true, Some(pilot)) => pilot.at,
          _ => avatar.at
        };
        let orbit = Vec3::new(
          (now * 1.3).cos() * 0.7,
          2.3 + (now * 2.0).sin() * 0.12,
          (now * 1.3).sin() * 0.7
        );
        let wanted = feet + orbit;
        transform.translation =
          transform.translation.lerp(wanted, (time.delta_secs() * 6.0).min(1.0))
      }
      _ => commands.entity(entity).despawn()
    }
  }
  for (owner, conditions, avatar, _) in lit.iter() {
    if conditions.has(Kind::Lit) && !floating.iter().any(|(_, wisp, _)| wisp.0 == owner) {
      commands
        .spawn((
          Wisp(owner),
          Mesh3d(sparks.ball.clone()),
          MeshMaterial3d(
            sparks.looks.get(&Block::SpellLight).cloned().unwrap_or_default()
          ),
          Transform::from_translation(avatar.at + Vec3::Y * 2.3)
            .with_scale(Vec3::splat(0.18)),
          Visibility::default()
        ))
        .with_child(PointLight {
          intensity: WISP_LUMENS,
          range: 18.0,
          color: Color::srgb(1.0, 0.92, 0.75),
          shadow_maps_enabled: false,
          ..default()
        });
    }
  }
}

pub struct Spells;

impl Plugin for Spells {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Recharge>()
      .init_resource::<Charging>()
      .add_systems(PreUpdate, invoke.after(ServerSystems::Receive).run_if(authority))
      .add_systems(
        Update,
        (fly, rage).run_if(authority).run_if(resource_exists::<Voxels>)
      )
      .add_systems(PreUpdate, unveil.after(ServerSystems::Receive).run_if(authority))
      .add_systems(Update, veil.run_if(plays))
      .add_systems(Startup, kindle.run_if(plays))
      .add_systems(Update, (show_bolts, bloom, wisps).run_if(plays))
      .add_systems(Update, (ignite, brighten).chain().after(HandSystems).run_if(plays))
      .add_systems(Update, cast.run_if(plays).run_if(resource_exists::<Pilot>));
  }
}
