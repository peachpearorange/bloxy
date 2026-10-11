use {crate::{authority::{Controller, player_of},
             beast::Roam,
             bird::Bird,
             block::Block,
             condition::{Conditions, Kind},
             death::Dying,
             folk::{Raider, Vigour, ray_box},
             haunt::{Blamed, Haunt, fell},
             menu::Menu,
             player::{Me, Pilot, Poke, Selected, captured},
             protocol::*,
             skill::Skills,
             voxels::Voxels},
     bevy::{platform::collections::HashMap,
            prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

const BOLT_LIFE: f32 = 2.2;
const SEEK: f32 = 24.0;
const FLOURISH_FOR: f32 = 0.6;
const WISP_LUMENS: f32 = 90_000.0;

pub fn fuel(spell: Block) -> Block {
  match spell {
    Block::SpellFirebolt | Block::SpellFrost | Block::SpellMissile => Block::ArcaneDust,
    Block::SpellHeal | Block::SpellLight => Block::Candle,
    Block::SpellHaste => Block::LizardClaw,
    Block::SpellLeap => Block::Feather,
    _ => Block::BoneDust
  }
}

pub fn cooldown(spell: Block) -> f32 {
  match spell {
    Block::SpellFirebolt => 0.8,
    Block::SpellFrost => 0.6,
    Block::SpellMissile => 1.5,
    Block::SpellHeal => 4.0,
    Block::SpellThunder => 3.0,
    Block::SpellLight => 5.0,
    _ => 8.0
  }
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
    _ => [0.55, 0.7, 1.0]
  }
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Invoke {
  pub spell: Block,
  pub toward: Vec3
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Conjure(pub Block);

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
pub struct Spellbook(HashMap<Block, f32>);

fn conjure(
  mut conjures: MessageReader<FromClient<Conjure>>,
  mut players: Query<(&Controller, (&Skills, &mut Inventory))>
) {
  for &FromClient { client_id, message: Conjure(spell) } in conjures.read() {
    if let Some((skills, mut inventory)) = player_of(players.iter_mut(), client_id)
      && skills.spells().contains(&spell)
      && inventory.slots[CURSOR].is_none()
    {
      inventory.slots[CURSOR] = Some(Stack { block: spell, count: 1 })
    }
  }
}

struct Target {
  entity: Entity,
  centre: Vec3,
  low: Vec3,
  high: Vec3
}

fn targets(
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
      && skills.spells().contains(&spell)
      && toward.is_finite()
      && book.0.get(&spell).is_none_or(|&ready| now >= ready)
    {
      let rank = skills.rank(spell);
      let thrifty = rand_unit(caster, now) < skills.stats().thrift;
      let paid = thrifty || inventory.take(fuel(spell));
      match paid {
        false => {
          notices.write(ToClients {
            targets: SendTargets::Single(client_id),
            message: Notice(format!("{} needs {}", spell.name(), fuel(spell).name()))
          });
        }
        true => {
          book.0.insert(spell, now + cooldown(spell) * 0.9);
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
        if let Ok((mut health, conditions, haunt)) = creatures.get_mut(target) {
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
        flourishes.write(ToClients {
          targets: SendTargets::All,
          message: Flourish { spell: bolt.spell, at, reach: 0.7 }
        });
        commands.entity(entity).despawn()
      }
      None => bolt.at = from + step
    }
  }
}

#[derive(Resource, Default)]
pub struct Recharge(pub HashMap<Block, f32>);

fn cast(
  time: Res<Time>,
  buttons: Res<ButtonInput<MouseButton>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  pilot: Res<Pilot>,
  selected: Res<Selected>,
  inventories: Query<&Inventory>,
  mut recharge: ResMut<Recharge>,
  mut invocations: MessageWriter<Invoke>,
  mut pokes: MessageWriter<Poke>
) {
  let now = time.elapsed_secs();
  if captured(&cursor, &menu)
    && buttons.just_pressed(MouseButton::Right)
    && let Ok(inventory) = inventories.get(pilot.me)
    && let Some(stack) = inventory.slots[selected.0]
    && stack.block.spell()
    && recharge.0.get(&stack.block).is_none_or(|&ready| now >= ready)
  {
    recharge.0.insert(stack.block, now + cooldown(stack.block));
    invocations
      .write(Invoke { spell: stack.block, toward: pilot.facing() * Vec3::NEG_Z });
    pokes.write(Poke);
  }
}

#[derive(Resource)]
struct Sparks {
  ball: Handle<Mesh>,
  looks: HashMap<Block, Handle<StandardMaterial>>
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
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let looks = Block::SPELLS
    .iter()
    .map(|&spell| (spell, glowing(&mut materials, hue(spell), 0.9)))
    .collect();
  commands.insert_resource(Sparks {
    ball: meshes.add(Sphere::new(0.5).mesh().uv(12, 8)),
    looks
  })
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
        flat: spell == Block::SpellThunder,
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
      .add_systems(
        PreUpdate,
        (conjure, invoke).after(ServerSystems::Receive).run_if(authority)
      )
      .add_systems(Update, fly.run_if(authority).run_if(resource_exists::<Voxels>))
      .add_systems(Startup, kindle.run_if(plays))
      .add_systems(Update, (show_bolts, bloom, wisps).run_if(plays))
      .add_systems(Update, cast.run_if(plays).run_if(resource_exists::<Pilot>));
  }
}
