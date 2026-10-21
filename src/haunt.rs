use {crate::{authority::{Controller, player_of},
             beast::Roam,
             bird::{Bird, Flight},
             block::Block,
             condition::{Conditions, Kind},
             death::{Dying, slay},
             dungeon::{Dungeon, Foe},
             folk::{KNOCK, Raider, Vigour, hurt, ray_box},
             loose::scatter,
             noise::unit,
             player::{Bulk, Marched, march},
             protocol::*,
             skill::Earned,
             spell::{Bolt, Flourish},
             summon::Servant,
             voxels::Voxels},
     bevy::{platform::collections::{HashMap, HashSet},
            prelude::*},
     bevy_replicon::prelude::*,
     std::f32::consts::TAU};

const WAKE: f32 = 30.0;
const SLEEP: f32 = 72.0;
const SENSE: f32 = 18.0;
const SMELL: f32 = 3.5;
const LEASH: f32 = 34.0;
const GRAVITY: f32 = 30.0;
const LEAP: f32 = 9.4;
const RESPAWN: f32 = 600.0;
const WARDEN_RESPAWN: f32 = 1500.0;
const MUSTER_EVERY: f32 = 0.5;
const HURL_WITHIN: f32 = 14.0;
const HURL_BEYOND: f32 = 3.5;
const HURL_EVERY: f32 = 3.0;
const EMBER_SPEED: f32 = 13.0;
const EMBER_LIFE: f32 = 2.0;
const EMBER_BLOW: u8 = 2;
const EMBER_BURNS: f32 = 3.0;
const IMP_HOVER: f32 = 1.0;

type Lair = (IVec2, u16);

impl Foe {
  fn speed(self) -> f32 {
    match self {
      Foe::Skeleton => Band::Skeleton.speed(),
      Foe::Shade => Band::Shade.speed(),
      Foe::Warden => Band::Warden.speed(),
      Foe::Imp => Band::Imp.speed(),
      Foe::Spider => 4.6
    }
  }

  fn reach(self) -> f32 {
    match self {
      Foe::Warden => 2.4,
      Foe::Spider | Foe::Imp => 1.3,
      _ => 1.45
    }
  }

  fn reload(self) -> f32 {
    match self {
      Foe::Skeleton => 1.1,
      Foe::Shade => 1.6,
      Foe::Spider | Foe::Imp => 0.9,
      Foe::Warden => 2.0
    }
  }

  fn blow(self) -> u8 {
    match self {
      Foe::Skeleton => Band::Skeleton.blow(),
      Foe::Shade => Band::Shade.blow(),
      Foe::Warden => Band::Warden.blow(),
      Foe::Imp => Band::Imp.blow(),
      Foe::Spider => 1
    }
  }

  fn bulk(self) -> Bulk {
    match self {
      Foe::Spider => Bulk { half: 0.42, tall: 0.6 },
      Foe::Warden => Bulk { half: 0.45, tall: 2.9 },
      Foe::Shade => Bulk { half: 0.3, tall: 1.7 },
      Foe::Imp => Bulk { half: 0.25, tall: 1.1 },
      Foe::Skeleton => Bulk::PERSON
    }
  }

  fn afflicts(self) -> &'static [(Kind, f32)] {
    match self {
      Foe::Spider => &[(Kind::Poisoned, 6.0)],
      Foe::Shade => &[(Kind::Chilled, 5.0), (Kind::Frightened, 5.0)],
      Foe::Warden => &[(Kind::Weakened, 8.0)],
      Foe::Imp => &[(Kind::Burning, 3.0)],
      Foe::Skeleton => &[]
    }
  }

  fn flies(self) -> bool { matches!(self, Foe::Shade | Foe::Imp) }

  fn respawn(self) -> f32 {
    match self {
      Foe::Warden => WARDEN_RESPAWN,
      _ => RESPAWN
    }
  }
}

#[derive(Component)]
pub struct Haunt {
  lair: Lair,
  foe: Foe,
  home: Vec3,
  velocity: Vec3,
  stagger: f32,
  reload: f32,
  stuck: f32,
  detour: f32,
  timer: f32,
  hurl: f32,
  goal: Vec3,
  luck: u32
}

impl Haunt {
  pub fn shove(&mut self, push: Vec3) {
    self.velocity = push;
    self.stagger = 0.35
  }
}

#[derive(Component, Clone, Copy)]
pub struct Blamed(pub Entity);

#[derive(Component)]
struct Ember {
  velocity: Vec3,
  life: f32
}

enum Prey {
  Player(Entity),
  Servant(Entity)
}

#[derive(Resource, Default)]
struct Lairs {
  awake: HashSet<Lair>,
  cleared: HashMap<Lair, f32>
}

pub fn fell(commands: &mut Commands, target: Entity) {
  commands.entity(target).remove::<(Raider, Roam, Flight)>();
  slay(commands, target)
}

pub fn rise(commands: &mut Commands, foe: Foe, at: Vec3, lair: Lair, luck: u32) {
  let yaw = unit(luck, 3, 1, 4) * TAU;
  let haunt = Haunt {
    lair,
    foe,
    home: at,
    velocity: Vec3::ZERO,
    stagger: 0.0,
    reload: 1.0,
    stuck: 0.0,
    detour: 0.0,
    timer: 0.0,
    hurl: HURL_EVERY * unit(luck, 7, 2, 9),
    goal: at,
    luck
  };
  let band = match foe {
    Foe::Skeleton => Some(Band::Skeleton),
    Foe::Shade => Some(Band::Shade),
    Foe::Warden => Some(Band::Warden),
    Foe::Imp => Some(Band::Imp),
    Foe::Spider => None
  };
  match band {
    Some(band) => commands.spawn((
      Replicated,
      Folk { band, at, yaw, luck, blows: 0, aboard: false },
      Health(band.health()),
      Conditions::default(),
      haunt
    )),
    None => commands.spawn((
      Replicated,
      Beast { breed: Breed::Spider, at, yaw, pose: Pose::Still },
      Health(Breed::Spider.health()),
      Conditions::default(),
      haunt
    ))
  };
}

fn muster(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  players: Query<&Avatar, With<Controller>>,
  haunts: Query<(Entity, &Haunt, Has<Dying>)>,
  mut lairs: ResMut<Lairs>,
  mut since: Local<f32>,
  mut commands: Commands
) {
  *since += time.delta_secs();
  if *since >= MUSTER_EVERY {
    *since = 0.0;
    let (now, seed) = (time.elapsed_secs(), voxels.seed);
    for avatar in players.iter() {
      let spot = avatar.at.xz().floor().as_ivec2();
      let reach = IVec2::splat(WAKE as i32);
      for dungeon in Dungeon::within(seed, spot - reach, spot + reach).iter() {
        for (index, lair) in dungeon.lairs.iter().enumerate() {
          let key = (dungeon.cell, index as u16);
          let rested =
            lairs.cleared.get(&key).is_none_or(|&when| now - when > lair.foe.respawn());
          if lair.at.as_vec3().distance(avatar.at) < WAKE
            && !lairs.awake.contains(&key)
            && rested
          {
            lairs.awake.insert(key);
            lairs.cleared.remove(&key);
            for member in 0..u32::from(lair.count) {
              let luck = crate::noise::hash(
                seed ^ 0x4A1,
                key.0.x * 31 + member as i32,
                i32::from(key.1),
                key.0.y
              );
              let angle =
                member as f32 / f32::from(lair.count) * TAU + unit(luck, 1, 2, 3);
              let wanted = lair.at
                + (Vec2::from_angle(angle) * 1.6).round().as_ivec2().extend(0).xzy();
              let open = |cell: IVec3, voxels: &mut Voxels| {
                !voxels.ensure(cell).solid() && !voxels.ensure(cell + IVec3::Y).solid()
              };
              let cell = match open(wanted, &mut voxels) {
                true => wanted,
                false => lair.at
              };
              rise(
                &mut commands,
                lair.foe,
                cell.as_vec3() + Vec3::new(0.5, 0.05, 0.5),
                key,
                luck
              )
            }
          }
        }
      }
    }
    let awake: Vec<Lair> = lairs.awake.iter().copied().collect();
    for key in awake.into_iter() {
      let home = Dungeon::at(seed, key.0)
        .and_then(|dungeon| {
          dungeon.lairs.get(usize::from(key.1)).map(|lair| lair.at.as_vec3())
        })
        .unwrap_or_default();
      let members: Vec<(Entity, bool)> = haunts
        .iter()
        .filter(|(_, haunt, _)| haunt.lair == key)
        .map(|(entity, _, dying)| (entity, dying))
        .collect();
      let watched = players.iter().any(|avatar| avatar.at.distance(home) < SLEEP);
      match (watched, members.iter().any(|&(_, dying)| !dying)) {
        (false, _) => {
          for &(entity, dying) in members.iter() {
            if !dying {
              commands.entity(entity).despawn()
            }
          }
          lairs.awake.remove(&key);
        }
        (true, false) => {
          lairs.awake.remove(&key);
          lairs.cleared.insert(key, now);
        }
        _ => ()
      }
    }
  }
}

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn toward(offset: Vec2) -> f32 { (-offset.x).atan2(-offset.y) }

fn prowl(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut haunts: Query<
    (&mut Haunt, Option<&mut Folk>, Option<&mut Beast>, &Conditions),
    Without<Dying>
  >,
  mut players: Query<
    (Entity, &Controller, &Avatar, &mut Health, &mut Vigour, &mut Conditions),
    Without<Haunt>
  >,
  mut servants: Query<
    (Entity, &Folk, &mut Health),
    (With<Servant>, Without<Haunt>, Without<Controller>, Without<Dying>)
  >,
  mut knocks: MessageWriter<ToClients<Knock>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  for (mut haunt, mut folk, mut beast, conditions) in haunts.iter_mut() {
    let (at, yaw) = folk
      .as_ref()
      .map(|folk| (folk.at, folk.yaw))
      .or(beast.as_ref().map(|beast| (beast.at, beast.yaw)))
      .unwrap_or_default();
    let foe = haunt.foe;
    let flying = foe.flies();
    let bulk = foe.bulk();
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
    let eye = at + Vec3::Y * bulk.tall * 0.8;
    let home = haunt.home;
    let sighted = |spot: Vec3| {
      let distance = spot.distance(at);
      let seen = || {
        let sight = spot + Vec3::Y * EYE - eye;
        voxels.cast_for(eye, sight, sight.length(), Block::solid).is_none()
      };
      spot.distance(home) < LEASH && (distance < SMELL || (distance < SENSE && seen()))
    };
    let target = players
      .iter()
      .filter(|(_, _, avatar, health, _, afflicted)| {
        health.0 > 0 && !afflicted.has(Kind::Invisible) && sighted(avatar.at)
      })
      .map(|(entity, _, avatar, ..)| (Prey::Player(entity), avatar.at))
      .chain(
        servants
          .iter()
          .filter(|(_, servant, health)| health.0 > 0 && sighted(servant.at))
          .map(|(entity, servant, _)| (Prey::Servant(entity), servant.at))
      )
      .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)));
    haunt.reload -= dt;
    haunt.stagger -= dt;
    haunt.detour -= dt;
    haunt.timer -= dt;
    haunt.hurl -= dt;
    let pace = foe.speed() * conditions.pace();
    let side = if haunt.luck % 2 == 0 { 1.5 } else { -1.5 };
    let (wish, lift, yaw, chasing) = match target {
      Some((prey, spot)) => {
        let offset = spot - at;
        let facing = toward(offset.xz()) + if haunt.detour > 0.0 { side } else { 0.0 };
        let close =
          offset.xz().length() < foe.reach() && offset.y.abs() < bulk.tall.max(2.0);
        if close && haunt.reload <= 0.0 && haunt.stagger <= 0.0 {
          haunt.reload = foe.reload();
          if let Some(folk) = folk.as_mut() {
            folk.blows = folk.blows.wrapping_add(1)
          }
          match prey {
            Prey::Player(player) => {
              if let Ok((_, controller, _, mut health, mut vigour, mut afflicted)) =
                players.get_mut(player)
              {
                hurt(&mut health, &mut vigour, foe.blow());
                for &(kind, secs) in foe.afflicts() {
                  afflicted.afflict(kind, secs)
                }
                let heft = if foe == Foe::Warden { 1.8 } else { 1.0 };
                knocks.write(ToClients {
                  targets: SendTargets::Single(controller.client),
                  message: Knock(
                    offset.xz().normalize_or_zero().extend(0.0).xzy() * KNOCK * heft
                      + Vec3::Y * 5.0
                  )
                });
              }
            }
            Prey::Servant(servant) => {
              if let Ok((_, _, mut health)) = servants.get_mut(servant) {
                health.0 = health.0.saturating_sub(foe.blow());
                if health.0 == 0 {
                  fell(&mut commands, servant)
                }
              }
            }
          }
        }
        let distance = offset.length();
        if foe == Foe::Imp
          && matches!(prey, Prey::Player(_))
          && (HURL_BEYOND..HURL_WITHIN).contains(&distance)
          && haunt.hurl <= 0.0
          && haunt.stagger <= 0.0
        {
          haunt.hurl = HURL_EVERY
            * (0.8 + unit(haunt.luck, 9, (time.elapsed_secs() * 4.0) as i32, 1) * 0.6);
          let toward = (spot + Vec3::Y * EYE * 0.7 - eye).normalize_or_zero();
          commands.spawn((
            Replicated,
            Bolt { spell: Block::SpellFirebolt, at: eye + toward * 0.3 },
            Ember { velocity: toward * EMBER_SPEED, life: EMBER_LIFE }
          ));
        }
        let weave = match foe == Foe::Imp {
          true => {
            ahead(facing + TAU / 4.0)
              * (time.elapsed_secs() * 2.3 + haunt.luck as f32).sin()
              * pace
              * 0.6
          }
          false => Vec3::ZERO
        };
        let wish = match close {
          true => weave * 0.5,
          false => ahead(facing) * pace + weave
        };
        let above = if foe == Foe::Imp { IMP_HOVER } else { 0.4 };
        (wish, spot.y + above - at.y, facing, true)
      }
      None => {
        if haunt.timer <= 0.0 {
          let (luck, now) = (haunt.luck, (time.elapsed_secs() * 3.0) as i32);
          let roll = |salt: i32| unit(luck, salt, now, 5);
          haunt.timer = 2.0 + roll(1) * 4.0;
          haunt.goal = home + Vec3::new(roll(2) * 8.0 - 4.0, 0.0, roll(3) * 8.0 - 4.0)
        }
        let away = haunt.goal - at;
        match away.xz().length() > 0.8 {
          true => {
            let facing = toward(away.xz());
            (ahead(facing) * pace * 0.4, home.y + 0.3 - at.y, facing, false)
          }
          false => (Vec3::ZERO, home.y + 0.3 - at.y, yaw, false)
        }
      }
    };
    let swimming = !flying
      && voxels
        .block((at + Vec3::Y * bulk.tall * 0.6).floor().as_ivec3())
        .is_some_and(Block::fluid);
    let horizontal = match haunt.stagger > 0.0 {
      true => haunt.velocity * (1.0 - dt * 3.0),
      false => wish
    };
    let vertical = match (flying, swimming) {
      (true, _) => (lift * 2.0).clamp(-2.5, 2.5),
      (_, true) => (haunt.velocity.y + 18.0 * dt).min(2.5),
      _ => (haunt.velocity.y - GRAVITY * dt).max(-40.0)
    };
    let Marched { at: moved, velocity, blocked, landed } =
      march(&voxels, bulk, at, Vec3::new(horizontal.x, vertical, horizontal.z), dt);
    haunt.velocity =
      match blocked && (landed || swimming) && !flying && haunt.stagger <= 0.0 {
        true => velocity.with_y(LEAP),
        false => velocity
      };
    let progress = (moved - at).xz().length() / dt.max(1e-4);
    haunt.stuck = match wish != Vec3::ZERO && haunt.stagger <= 0.0 && progress < 0.4 {
      true => haunt.stuck + dt,
      false => (haunt.stuck - dt).max(0.0)
    };
    if haunt.stuck > 1.2 {
      haunt.stuck = 0.0;
      haunt.detour = 1.6
    }
    let yaw = yaw.rem_euclid(TAU);
    let moved = match moved.y < -16.0 {
      true => home,
      false => moved
    };
    if let Some(folk) = folk.as_mut() {
      let next = Folk { at: moved, yaw, ..**folk };
      folk.set_if_neq(next);
    }
    if let Some(beast) = beast.as_mut() {
      let pose = match (progress > 0.3, chasing) {
        (true, true) => Pose::Run,
        (true, false) => Pose::Walk,
        _ => Pose::Still
      };
      let next = Beast { at: moved, yaw, pose, ..**beast };
      beast.set_if_neq(next);
    }
  }
}

fn scorch(
  time: Res<Time>,
  voxels: Res<Voxels>,
  mut embers: Query<(Entity, &mut Bolt, &mut Ember)>,
  mut players: Query<(&Controller, &Avatar, &mut Health, &mut Vigour, &mut Conditions)>,
  mut knocks: MessageWriter<ToClients<Knock>>,
  mut flourishes: MessageWriter<ToClients<Flourish>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  for (entity, mut bolt, mut ember) in embers.iter_mut() {
    let (from, step) = (bolt.at, ember.velocity * dt);
    let (direction, length) = (step.normalize_or_zero(), step.length());
    let wall = voxels
      .cast_for(from, direction, length, Block::solid)
      .map(|hit| (hit.at.as_vec3() + 0.5).distance(from) - 0.5);
    let struck = players
      .iter_mut()
      .filter(|(_, _, health, ..)| health.0 > 0)
      .filter_map(|player| {
        let at = player.1.at;
        ray_box(
          from,
          direction,
          at - Vec3::new(0.3, 0.0, 0.3),
          at + Vec3::new(0.3, 1.8, 0.3)
        )
        .filter(|&near| near <= length && wall.is_none_or(|wall| near <= wall))
        .map(|near| (near, player))
      })
      .min_by(|a, b| a.0.total_cmp(&b.0));
    ember.life -= dt;
    let ended = match (struck, wall) {
      (Some((near, (controller, _, mut health, mut vigour, mut afflicted))), _) => {
        hurt(&mut health, &mut vigour, EMBER_BLOW);
        afflicted.afflict(Kind::Burning, EMBER_BURNS);
        knocks.write(ToClients {
          targets: SendTargets::Single(controller.client),
          message: Knock(
            direction.with_y(0.0).normalize_or_zero() * KNOCK * 0.5 + Vec3::Y * 3.0
          )
        });
        Some(from + direction * near)
      }
      (_, Some(wall)) => Some(from + direction * wall.max(0.0)),
      _ if ember.life <= 0.0 => Some(from),
      _ => None
    };
    match ended {
      Some(at) => {
        flourishes.write(ToClients {
          targets: SendTargets::All,
          message: Flourish { spell: Block::SpellFirebolt, at, reach: 0.7 }
        });
        commands.entity(entity).despawn()
      }
      None => bolt.at = from + step
    }
  }
}

fn swat(
  mut strikes: MessageReader<FromClient<Strike>>,
  time: Res<Time>,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Vigour))>,
  mut beasts: Query<(&Beast, &mut Health, &mut Haunt)>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  for &FromClient { client_id, message: Strike(target) } in strikes.read() {
    if let Ok((beast, mut health, mut haunt)) = beasts.get_mut(target)
      && let Some((player, avatar, mut vigour)) = player_of(players.iter_mut(), client_id)
      && now - vigour.swung >= vigour.quick
      && (avatar.at + Vec3::Y * EYE).distance(beast.at) <= REACH + 1.0
      && health.0 > 0
    {
      vigour.swung = now;
      health.0 = health.0.saturating_sub(vigour.punch);
      commands.entity(target).insert(Blamed(player));
      haunt.shove(
        (beast.at - avatar.at).with_y(0.0).normalize_or_zero() * 6.0 + Vec3::Y * 4.0
      );
      if health.0 == 0 {
        fell(&mut commands, target)
      }
    }
  }
}

fn spoils(
  folk: Option<&Folk>,
  beast: Option<&Beast>,
  bird: Option<&Bird>
) -> (Vec<(Block, u16)>, u32) {
  let luck = folk.map(|folk| folk.luck).unwrap_or_default();
  let some = |low: u16, spread: u32| low + (luck % spread) as u16;
  match (folk.map(|folk| folk.band), beast.map(|beast| beast.breed), bird.is_some()) {
    (Some(Band::Skeleton), ..) => (vec![(Block::BoneDust, some(1, 2))], 15),
    (Some(Band::Shade), ..) => (vec![(Block::ArcaneDust, some(1, 2))], 20),
    (Some(Band::Imp), ..) => {
      (vec![(Block::Charcoal, some(1, 2)), (Block::ArcaneDust, some(0, 2))], 18)
    }
    (Some(Band::Warden), ..) => (
      vec![
        (Block::ArcaneDust, 5),
        (Block::BoneDust, 5),
        (Block::Candle, 2),
        (Block::DiamondOre, 1),
      ],
      160
    ),
    (Some(Band::Pirate), ..) => (vec![], 10),
    (Some(Band::Viking), ..) => (vec![], 14),
    (_, Some(Breed::Spider), _) => (vec![(Block::Cobweb, 1), (Block::LizardClaw, 1)], 12),
    (_, Some(Breed::Lizard), _) => (
      vec![(
        Block::LizardClaw,
        1 + (beast.map_or(0.0, |beast| beast.at.x.fract().abs()) * 2.0) as u16
      )],
      6
    ),
    (_, Some(Breed::Sheep(_)), _) => (vec![], 3),
    (_, _, true) => (vec![], 3),
    _ => (vec![], 0)
  }
}

fn reward(
  slain: On<Add, Dying>,
  victims: Query<(Option<&Folk>, Option<&Beast>, Option<&Bird>, Option<&Blamed>)>,
  mut earned: MessageWriter<Earned>,
  mut commands: Commands
) {
  if let Ok((folk, beast, bird, blamed)) = victims.get(slain.entity) {
    let at = folk
      .map(|folk| folk.at)
      .or(beast.map(|beast| beast.at))
      .or(bird.map(|bird| bird.at))
      .unwrap_or_default();
    let (loot, xp) = spoils(folk, beast, bird);
    for (block, count) in loot.into_iter().filter(|&(_, count)| count > 0) {
      scatter(&mut commands, block, count, at + Vec3::Y * 0.6, Vec3::Y * 3.0, 0.4)
    }
    if let Some(folk) = folk.filter(|folk| folk.band == Band::Warden) {
      crate::loose::fling(
        &mut commands,
        crate::gear::found(folk.luck, folk.luck.rotate_left(7), 2),
        at + Vec3::Y * 0.6,
        Vec3::Y * 3.5,
        0.4
      )
    }
    if let Some(&Blamed(player)) = blamed
      && xp > 0
    {
      earned.write(Earned { player, xp });
    }
  }
}

pub struct Haunting;

impl Plugin for Haunting {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Lairs>()
      .add_systems(
        Update,
        (muster, prowl, scorch)
          .chain()
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
      )
      .add_systems(PreUpdate, swat.after(ServerSystems::Receive).run_if(authority))
      .add_observer(reward);
  }
}
