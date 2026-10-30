use {crate::{authority::{Controller, within_reach},
             automaton::{self, BONES, Model, PARENT, Plated, TRIBES},
             block::Block,
             chest::Coffers,
             condition::Conditions,
             conveyor::Riding,
             death::Dying,
             folk::{Raider, struck},
             generate,
             haunt::Haunt,
             island::{self, Island, SEA},
             loose::{Looks, fling},
             machine::Works,
             menu::Menu,
             noise::{hash, unit},
             player::{Bulk, Marched, Pilot, Poke, Selected, captured, march},
             protocol::*,
             spell::{Flourish, harm, hurl},
             stream::Palette,
             voxels::Voxels},
     bevy::{camera::visibility::DynamicSkinnedMeshBounds,
            ecs::entity::MapEntities,
            mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
            platform::collections::{HashMap, HashSet},
            prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize},
     std::f32::consts::{PI, TAU}};

const GUARD: f32 = 16.0;
const TETHER: f32 = 24.0;
const HEEL: f32 = 2.6;
const STRAYED: f32 = 40.0;
const GATHER: f32 = 10.0;
const DEPOT: i32 = 6;
const GRAVITY: f32 = 30.0;
const LEAP: f32 = 9.4;
const BOLT_SPEED: f32 = 24.0;
const MEND_EVERY: f32 = 3.0;
const CAMP_WAKE: f32 = 90.0;
const ORDER_REACH: f32 = REACH + 2.0;

impl Band {
  pub fn model(self) -> Option<Model> {
    match self {
      Band::Cogling => Some(Model::Cogling),
      Band::Scavver => Some(Model::Scavver),
      Band::Gladiator => Some(Model::Gladiator),
      Band::Paragon => Some(Model::Paragon),
      _ => None
    }
  }

  pub fn item(self) -> Option<Block> {
    match self {
      Band::Cogling => Some(Block::Cogling),
      Band::Gladiator => Some(Block::Gladiator),
      Band::Paragon => Some(Block::Paragon),
      _ => None
    }
  }

  fn bulk(self) -> Bulk {
    match self {
      Band::Cogling => Bulk { half: 0.24, tall: 1.2 },
      Band::Scavver => Bulk { half: 0.28, tall: 1.32 },
      Band::Gladiator => Bulk { half: 0.3, tall: 2.15 },
      _ => Bulk { half: 0.3, tall: 1.92 }
    }
  }

  fn lunge(self) -> f32 {
    match self {
      Band::Gladiator => 2.1,
      _ => 1.8
    }
  }
}

pub fn awakened(block: Block) -> Option<Band> {
  [Band::Cogling, Band::Gladiator, Band::Paragon]
    .into_iter()
    .find(|band| band.item() == Some(block))
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Automaton {
  pub owner: Option<u32>,
  pub staying: bool,
  pub carrying: Option<Block>
}

#[derive(Component)]
pub struct Drive {
  post: Vec3,
  velocity: Vec3,
  reload: f32,
  bolt: f32,
  stuck: f32,
  detour: f32,
  calm: f32,
  wander: f32,
  goal: Vec3,
  load: Option<Stack>,
  depot: Option<IVec3>,
  survey: f32
}

impl Drive {
  fn at(post: Vec3, load: Option<Stack>) -> Drive {
    Drive {
      post,
      velocity: Vec3::ZERO,
      reload: 0.5,
      bolt: 1.0,
      stuck: 0.0,
      detour: 0.0,
      calm: 0.0,
      wander: 0.0,
      goal: post,
      load,
      depot: None,
      survey: 0.0
    }
  }
}

#[derive(Component)]
struct Camp(IVec2);

#[derive(Serialize, Deserialize, Clone)]
pub struct Kept {
  pub folk: Folk,
  pub automaton: Automaton,
  pub health: u8,
  pub post: Vec3,
  pub load: Option<Stack>
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Rouse(pub IVec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Order {
  #[entities]
  pub robot: Entity,
  pub pack: bool
}

pub fn lodge(commands: &mut Commands, kept: Kept) {
  let Kept { folk, automaton, health, post, load } = kept;
  commands.spawn((
    Replicated,
    folk,
    automaton,
    Health(health.max(1)),
    Conditions::default(),
    Drive::at(post, load)
  ));
}

pub fn kept(robots: &Query<(&Folk, &Automaton, &Health, &Drive)>) -> Vec<Kept> {
  robots
    .iter()
    .filter(|(_, automaton, ..)| automaton.owner.is_some())
    .map(|(&folk, &automaton, health, drive)| Kept {
      folk,
      automaton,
      health: health.0,
      post: drive.post,
      load: drive.load
    })
    .collect()
}

fn tell(notices: &mut MessageWriter<ToClients<Notice>>, client: ClientId, word: String) {
  notices
    .write(ToClients { targets: SendTargets::Single(client), message: Notice(word) });
}

fn title(band: Band, luck: u32) -> String {
  let tribe = TRIBES[luck as usize % TRIBES.len()].name;
  let kind = match band {
    Band::Cogling => "Cogling",
    Band::Gladiator => "Gladiator",
    Band::Paragon => "Paragon",
    _ => "Scavver"
  };
  format!("{tribe} {kind}")
}

fn command(
  (mut rousings, mut orders): (
    MessageReader<FromClient<Rouse>>,
    MessageReader<FromClient<Order>>
  ),
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  mut robots: Query<(&mut Folk, &mut Automaton, &mut Drive), Without<Dying>>,
  mut notices: MessageWriter<ToClients<Notice>>,
  mut commands: Commands
) {
  for &FromClient { client_id, message: Rouse(cell) } in rousings.read() {
    let room = (0..3).all(|up| !voxels.ensure(cell + IVec3::Y * up).solid())
      && voxels.ensure(cell - IVec3::Y).solid();
    if let Some((account, (avatar, mut inventory))) = players
      .iter_mut()
      .find(|(controller, _)| controller.client == client_id)
      .map(|(controller, found)| (controller.account, found))
      && within_reach(avatar, cell)
      && let Some(stack) = inventory.slots[usize::from(avatar.hand)]
      && let Some(band) = awakened(stack.block)
    {
      match room && inventory.take(stack.block) {
        true => {
          let at = cell.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
          let luck = hash(account as u32, cell.x, cell.y, cell.z);
          let yaw = (avatar.at - at).xz();
          commands.spawn((
            Replicated,
            Folk { band, at, yaw: (-yaw.x).atan2(-yaw.y), luck, blows: 0, aboard: false },
            Automaton { owner: Some(account as u32), staying: false, carrying: None },
            Health(band.health()),
            Conditions::default(),
            Drive::at(at, None)
          ));
          tell(
            &mut notices,
            client_id,
            format!("{} wakes and follows you", title(band, luck))
          )
        }
        false => tell(&mut notices, client_id, "It needs open ground to stand on".into())
      }
    }
  }
  for &FromClient { client_id, message: Order { robot, pack } } in orders.read() {
    if let Some((account, (avatar, mut inventory))) = players
      .iter_mut()
      .find(|(controller, _)| controller.client == client_id)
      .map(|(controller, found)| (controller.account, found))
      && let Ok((mut folk, mut automaton, mut drive)) = robots.get_mut(robot)
      && (avatar.at + Vec3::Y * EYE).distance(folk.at + Vec3::Y) <= ORDER_REACH + 1.0
    {
      let name = title(folk.band, folk.luck);
      let owned = automaton.owner == Some(account as u32);
      let folded =
        owned && pack && folk.band.item().is_some_and(|item| inventory.add(item));
      match (owned, pack) {
        (false, _) => {
          tell(&mut notices, client_id, format!("{name} answers to someone else"))
        }
        (true, true) if folded => {
          if let Some(load) = drive.load.take() {
            fling(&mut commands, load, folk.at + Vec3::Y, Vec3::Y * 2.0, 0.3)
          }
          commands.entity(robot).despawn();
          tell(&mut notices, client_id, format!("{name} folds up"))
        }
        (true, true) => tell(&mut notices, client_id, "No room to carry it".into()),
        (true, false) => {
          automaton.staying = !automaton.staying;
          drive.post = folk.at;
          drive.depot = None;
          drive.survey = 0.0;
          folk.blows = folk.blows.wrapping_add(1);
          tell(&mut notices, client_id, match (automaton.staying, folk.band) {
            (true, Band::Cogling) => {
              format!("{name} keeps this spot, gathering into chests nearby")
            }
            (true, _) => format!("{name} guards this spot"),
            (false, _) => format!("{name} follows you")
          })
        }
      }
    }
  }
}

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn toward(offset: Vec2) -> f32 { (-offset.x).atan2(-offset.y) }

fn depot(voxels: &mut Voxels, post: Vec3) -> Option<IVec3> {
  let centre = post.floor().as_ivec3();
  (-DEPOT..=DEPOT)
    .flat_map(|x| {
      (-2..=3).flat_map(move |y| (-DEPOT..=DEPOT).map(move |z| IVec3::new(x, y, z)))
    })
    .map(|offset| centre + offset)
    .filter(|&at| matches!(voxels.block(at), Some(Block::Chest | Block::EmptyChest)))
    .min_by_key(|at| (*at - centre).length_squared())
}

fn element(luck: u32) -> Block {
  match luck % 6 {
    0 | 5 => Block::SpellFirebolt,
    1 | 3 => Block::SpellFrost,
    _ => Block::SpellMissile
  }
}

#[allow(clippy::too_many_arguments)]
fn labour(
  time: Res<Time>,
  (mut voxels, mut coffers, mut works): (ResMut<Voxels>, ResMut<Coffers>, ResMut<Works>),
  mut robots: Query<
    (Entity, &mut Folk, &mut Automaton, &mut Drive, &mut Health, &Conditions),
    Without<Dying>
  >,
  mut owners: Query<(Entity, &Controller, &Avatar, &mut Inventory)>,
  mut foes: Query<
    (
      Entity,
      &mut Health,
      Option<&Folk>,
      Option<&Beast>,
      Option<&mut Haunt>,
      Option<&mut Raider>
    ),
    (Or<(With<Haunt>, With<Raider>)>, Without<Automaton>, Without<Dying>)
  >,
  litter: Query<(Entity, &Loose), (Without<Gathered>, Without<Riding>)>,
  (mut flourishes, mut changes): (
    MessageWriter<ToClients<Flourish>>,
    MessageWriter<ToClients<Altered>>
  ),
  mut commands: Commands,
  mut taken: Local<HashSet<Entity>>
) {
  let dt = time.delta_secs().min(0.1);
  taken.clear();
  for (entity, mut folk, mut automaton, mut drive, mut health, conditions) in
    robots.iter_mut()
  {
    let Folk { band, at, yaw, luck, .. } = *folk;
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
    drive.reload -= dt;
    drive.bolt -= dt;
    drive.detour -= dt;
    drive.calm += dt;
    drive.wander -= dt;
    drive.survey -= dt;
    let master = automaton
      .owner
      .and_then(|account| {
        owners.iter().find(|(_, controller, ..)| controller.account as u32 == account)
      })
      .map(|(player, _, avatar, _)| (player, avatar.at));
    let following = !automaton.staying && master.is_some();
    let anchor = match (following, master) {
      (true, Some((_, spot))) => spot,
      _ => drive.post
    };
    let fights = matches!(band, Band::Gladiator | Band::Paragon);
    let prey = foes
      .iter()
      .filter(|_| fights)
      .filter_map(|(foe, health, folk, beast, ..)| {
        folk
          .map(|folk| folk.at)
          .or(beast.map(|beast| beast.at))
          .filter(|spot| {
            health.0 > 0 && spot.distance(at) < GUARD && spot.distance(anchor) < TETHER
          })
          .map(|spot| (foe, spot))
      })
      .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)));
    let blamed = master.map_or(entity, |(player, _)| player);
    let pace = band.speed() * conditions.pace();
    let spread =
      Vec3::new(unit(luck, 1, 2, 3) - 0.5, 0.0, unit(luck, 4, 5, 6) - 0.5) * 2.4;
    let heel = |spot: Vec3, close: f32| {
      let offset = spot - at;
      match offset.xz().length() > close {
        true => (ahead(toward(offset.xz())) * pace, toward(offset.xz())),
        false => (Vec3::ZERO, yaw)
      }
    };
    if band == Band::Cogling && automaton.staying && drive.survey <= 0.0 {
      drive.survey = 3.0;
      drive.depot = depot(&mut voxels, drive.post)
    }
    let (wish, facing) = match (prey, band) {
      (Some((foe, spot)), _) => {
        drive.calm = 0.0;
        let offset = spot - at;
        let reach = band.lunge();
        let close = offset.xz().length() < reach && offset.y.abs() < 2.4;
        let aim = toward(offset.xz());
        if band == Band::Paragon && !close && drive.bolt <= 0.0 && offset.length() < GUARD
        {
          drive.bolt = 1.8;
          folk.blows = folk.blows.wrapping_add(1);
          let eye = at + Vec3::Y * band.bulk().tall * 0.75;
          let spell = element(luck);
          hurl(
            &mut commands,
            spell,
            eye,
            (spot + Vec3::Y - eye).normalize_or_zero() * BOLT_SPEED,
            4,
            blamed,
            (spell == Block::SpellMissile).then_some(foe)
          );
          flourishes.write(ToClients {
            targets: SendTargets::All,
            message: Flourish { spell, at: eye, reach: 0.6 }
          });
        }
        if close
          && drive.reload <= 0.0
          && let Ok((foe, mut health, _, _, haunt, raider)) = foes.get_mut(foe)
        {
          drive.reload = if band == Band::Gladiator { 0.85 } else { 1.0 };
          folk.blows = folk.blows.wrapping_add(1);
          harm(&mut commands, foe, &mut health, band.blow(), blamed);
          let push = offset.with_y(0.0).normalize_or_zero() * 6.0 + Vec3::Y * 4.0;
          if let Some(mut haunt) = haunt {
            haunt.shove(push)
          }
          if let Some(mut raider) = raider {
            raider.knock(push)
          }
        }
        let keep =
          if band == Band::Paragon && drive.bolt > 0.6 { reach * 0.8 } else { 0.0 };
        match close || (keep > 0.0 && offset.length() < 6.0) {
          true => (Vec3::ZERO, aim),
          false => (ahead(aim + if drive.detour > 0.0 { 1.4 } else { 0.0 }) * pace, aim)
        }
      }
      (None, Band::Cogling) => match drive.load {
        Some(load) => match (following, master, drive.depot) {
          (true, Some((player, spot)), _) => {
            if at.distance(spot) < 1.8
              && let Ok((.., mut inventory)) = owners.get_mut(player)
            {
              let left = inventory.gain(load);
              drive.load = (left > 0).then_some(Stack { count: left, ..load });
              folk.blows = folk.blows.wrapping_add(1)
            }
            heel(spot, 1.2)
          }
          (_, _, Some(chest)) => {
            let spot = chest.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
            if at.xz().distance(spot.xz()) < 1.6 {
              let stored = crate::machine::deliver(
                &mut works.states,
                &mut coffers,
                &mut voxels,
                &mut changes,
                chest,
                0,
                load
              );
              drive.load = (!stored).then_some(load);
              drive.depot = drive.depot.filter(|_| stored);
              folk.blows = folk.blows.wrapping_add(1)
            }
            heel(spot, 1.0)
          }
          _ => match master {
            Some((player, spot)) if at.distance(spot) < 3.0 => {
              if let Ok((.., mut inventory)) = owners.get_mut(player) {
                let left = inventory.gain(load);
                drive.load = (left > 0).then_some(Stack { count: left, ..load })
              }
              heel(spot, 1.2)
            }
            _ => heel(drive.post, 0.6)
          }
        },
        None => {
          let found = litter
            .iter()
            .filter(|(item, loose)| {
              !taken.contains(item)
                && loose.at.distance(anchor) < GATHER
                && loose.at.distance(at) < GATHER + 6.0
                && !(following && loose.at.distance(anchor) < 1.6)
            })
            .min_by(|a, b| a.1.at.distance(at).total_cmp(&b.1.at.distance(at)));
          match found {
            Some((item, loose)) => {
              if at.xz().distance(loose.at.xz()) < 0.9 && (loose.at.y - at.y).abs() < 1.6
              {
                taken.insert(item);
                drive.load = Some(Stack {
                  block: loose.block,
                  count: loose.count,
                  gear: loose.gear
                });
                commands.entity(item).despawn()
              }
              heel(loose.at, 0.5)
            }
            None => heel(anchor + spread, if following { HEEL } else { 0.5 })
          }
        }
      },
      (None, Band::Scavver) => {
        if drive.wander <= 0.0 {
          drive.wander = 3.0 + unit(luck, (time.elapsed_secs() * 3.0) as i32, 7, 1) * 5.0;
          let roll = |salt: i32| {
            unit(luck, salt, (time.elapsed_secs() * 7.0) as i32, 5) * 2.0 - 1.0
          };
          drive.goal = drive.post + Vec3::new(roll(1), 0.0, roll(2)) * 6.0
        }
        let (wish, facing) = heel(drive.goal, 0.6);
        (wish * 0.45, facing)
      }
      (None, _) => heel(anchor + spread, if following { HEEL + 0.6 } else { 0.5 })
    };
    if drive.calm > 5.0 && health.0 < band.health() && (drive.calm % MEND_EVERY) < dt {
      health.0 += 1
    }
    let bulk = band.bulk();
    let vertical = (drive.velocity.y - GRAVITY * dt).max(-40.0);
    let Marched { at: moved, velocity, blocked, landed } =
      march(&voxels, bulk, at, Vec3::new(wish.x, vertical, wish.z), dt);
    drive.velocity = match blocked && landed {
      true => velocity.with_y(LEAP),
      false => velocity
    };
    let progress = (moved - at).xz().length() / dt.max(1e-4);
    drive.stuck = match wish != Vec3::ZERO && progress < 0.4 {
      true => drive.stuck + dt,
      false => (drive.stuck - dt).max(0.0)
    };
    if drive.stuck > 1.2 {
      drive.stuck = 0.0;
      drive.detour = 1.4
    }
    let moved = match (following, moved.distance(anchor) > STRAYED || moved.y < -16.0) {
      (true, true) => anchor + Vec3::new(1.0, 0.5, 1.0),
      (false, true) if moved.y < -16.0 => drive.post + Vec3::Y,
      _ => moved
    };
    automaton.set_if_neq(Automaton {
      carrying: drive.load.map(|stack| stack.block),
      ..*automaton
    });
    folk.set_if_neq(Folk { at: moved, yaw: facing.rem_euclid(TAU), ..*folk });
  }
}

fn wreck(
  fallen: Query<(&Folk, &Automaton, &Drive), Added<Dying>>,
  mut commands: Commands
) {
  for (folk, automaton, drive) in fallen.iter() {
    let at = folk.at + Vec3::Y;
    let remains = match (automaton.owner, folk.band.item()) {
      (Some(_), Some(item)) => Stack::of(item, 1),
      _ => Stack::of(Block::Cogwheel, 3)
    };
    for stack in [Some(remains), drive.load].into_iter().flatten() {
      fling(&mut commands, stack, at, Vec3::new(0.0, 3.0, 0.0), 0.6)
    }
  }
}

#[derive(Resource, Default)]
struct Camps(HashSet<IVec2>);

fn camping(island: &Island, seed: u32) -> bool {
  matches!(island.kind, island::Kind::Dunes | island::Kind::Volcano | island::Kind::Peak)
    && island.radius > 14.0
    && unit(seed ^ 0xC06, island.cell.x, island.cell.y, 0x5CA) < 0.65
}

fn muster(
  voxels: Res<Voxels>,
  mut camps: ResMut<Camps>,
  players: Query<&Avatar>,
  mut commands: Commands
) {
  let seed = voxels.seed;
  for island in players
    .iter()
    .flat_map(|avatar| {
      Island::near(seed, avatar.at)
        .into_iter()
        .filter(|island| avatar.at.xz().distance(island.centre) < CAMP_WAKE)
    })
    .collect::<Vec<_>>()
    .into_iter()
    .filter(|island| camping(island, seed) && camps.0.insert(island.cell))
  {
    let key = hash(seed ^ 0x5CA77, island.cell.x, island.cell.y, 3);
    let roll = |salt: u32, attempt: u32| unit(key ^ salt, attempt as i32, 0, 0);
    let site = (0..24).find_map(|attempt| {
      let spot = (island.centre
        + Vec2::new(roll(1, attempt) * 2.0 - 1.0, roll(2, attempt) * 2.0 - 1.0)
          * island.radius
          * 0.45)
        .floor()
        .as_ivec2();
      let ground = generate::height(seed, spot.x, spot.y);
      (ground > SEA + 1).then(|| {
        Vec3::new(spot.x as f32 + 0.5, ground as f32 + 1.05, spot.y as f32 + 0.5)
      })
    });
    for (index, centre) in site
      .into_iter()
      .flat_map(|centre| (0..3 + key % 2).map(move |index| (index, centre)))
    {
      let angle = index as f32 / 3.5 * TAU;
      let at = centre + Vec3::new(angle.cos(), 0.6, angle.sin()) * 2.0;
      let luck = hash(key, index as i32, 1, 2);
      commands.spawn((
        Replicated,
        Folk { band: Band::Scavver, at, yaw: angle + PI, luck, blows: 0, aboard: false },
        Automaton { owner: None, staying: true, carrying: None },
        Health(Band::Scavver.health()),
        Conditions::default(),
        Drive::at(at, None),
        Camp(island.cell)
      ));
    }
  }
}

fn disperse(
  voxels: Res<Voxels>,
  players: Query<&Avatar>,
  campers: Query<(Entity, &Camp)>,
  mut camps: ResMut<Camps>,
  mut commands: Commands
) {
  let seed = voxels.seed;
  let forgotten: Vec<IVec2> = camps
    .0
    .iter()
    .copied()
    .filter(|&cell| {
      Island::at(seed, cell).is_none_or(|island| {
        !players
          .iter()
          .any(|avatar| avatar.at.xz().distance(island.centre) < island::UNLOAD)
      })
    })
    .collect();
  for cell in forgotten.iter() {
    camps.0.remove(cell);
  }
  for (entity, camp) in campers.iter() {
    if forgotten.contains(&camp.0) {
      commands.entity(entity).despawn()
    }
  }
}

#[derive(Resource, Default)]
struct Foundry {
  meshes: HashMap<(Band, usize), (Handle<Mesh>, Handle<SkinnedMeshInverseBindposes>)>,
  material: Option<Handle<Plated>>
}

#[derive(Component)]
struct Assembled {
  at: Vec3,
  yaw: f32,
  stride: f32,
  pace: f32,
  blows: u8,
  swung: f32,
  carried: Option<Block>
}

#[derive(Component)]
struct Joint(u16);

#[derive(Component)]
struct Burden;

fn assemble(
  arrivals: Query<(Entity, &Folk), (With<Automaton>, Without<Assembled>)>,
  mut foundry: ResMut<Foundry>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut poses: ResMut<Assets<SkinnedMeshInverseBindposes>>,
  mut materials: ResMut<Assets<Plated>>,
  mut commands: Commands
) {
  for (entity, folk) in arrivals.iter() {
    if let Some(model) = folk.band.model() {
      let tribe = folk.luck as usize % TRIBES.len();
      let material = foundry
        .material
        .get_or_insert_with(|| materials.add(automaton::plated()))
        .clone();
      let (mesh, inverse_bindposes) = foundry
        .meshes
        .entry((folk.band, tribe))
        .or_insert_with(|| {
          (
            meshes.add(automaton::mesh(model, tribe)),
            poses.add(automaton::bindings(model))
          )
        })
        .clone();
      let frame = model.frame();
      let joints = (0..BONES as u16).fold(Vec::<Entity>::new(), |mut joints, bone| {
        let pivot = frame.pivot(bone);
        let (parent, base) = PARENT[bone as usize]
          .map_or((entity, Vec3::ZERO), |parent| {
            (joints[parent as usize], frame.pivot(parent))
          });
        joints.push(
          commands
            .spawn((
              Joint(bone),
              Transform::from_translation(pivot - base),
              Visibility::default(),
              ChildOf(parent)
            ))
            .id()
        );
        joints
      });
      commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        SkinnedMesh { inverse_bindposes, joints },
        DynamicSkinnedMeshBounds,
        ChildOf(entity)
      ));
      commands.entity(entity).insert((
        Assembled {
          at: folk.at,
          yaw: folk.yaw,
          stride: 0.0,
          pace: 0.0,
          blows: folk.blows,
          swung: 9.0,
          carried: None
        },
        Transform::from_translation(folk.at),
        Visibility::default()
      ));
    }
  }
}

fn hoist(
  mut robots: Query<(Entity, &Automaton, &mut Assembled)>,
  family: Query<&Children>,
  joints: Query<&Joint>,
  burdens: Query<Entity, With<Burden>>,
  mut looks: ResMut<Looks>,
  mut meshes: ResMut<Assets<Mesh>>,
  palette: Option<Res<Palette>>,
  mut commands: Commands
) {
  for (entity, automaton, mut assembled) in robots.iter_mut() {
    if assembled.carried != automaton.carrying
      && let Some(palette) = palette.as_ref()
    {
      assembled.carried = automaton.carrying;
      for burden in
        family.iter_descendants(entity).filter(|child| burdens.contains(*child))
      {
        commands.entity(burden).despawn()
      }
      if let Some(block) = automaton.carrying
        && let Some(chest) = family
          .iter_descendants(entity)
          .find(|&child| joints.get(child).is_ok_and(|joint| joint.0 == automaton::CHEST))
      {
        commands.spawn((
          Burden,
          Mesh3d(looks.mesh(block, None, &mut meshes)),
          MeshMaterial3d(palette.solid.clone()),
          Transform::from_xyz(0.0, 0.62, -0.02).with_scale(Vec3::splat(1.6)),
          ChildOf(chest)
        ));
      }
    }
  }
}

fn animate(
  time: Res<Time>,
  mut robots: Query<(Entity, &Folk, &Automaton, &mut Assembled, &mut Transform)>,
  family: Query<&Children>,
  mut joints: Query<(&Joint, &mut Transform), Without<Assembled>>
) {
  let (dt, now) = (time.delta_secs(), time.elapsed_secs());
  for (entity, folk, automaton, mut shown, mut transform) in robots.iter_mut() {
    let model = folk.band.model().unwrap_or(Model::Cogling);
    let before = shown.at;
    shown.at = before.lerp(folk.at, (dt * 12.0).min(1.0));
    let turn = (folk.yaw - shown.yaw + PI).rem_euclid(TAU) - PI;
    shown.yaw += turn * (dt * 9.0).min(1.0);
    let pace = (shown.at - before).xz().length() / dt.max(1e-4);
    shown.pace += (pace - shown.pace) * (dt * 6.0).min(1.0);
    let legs = model.frame().hip.y;
    shown.stride += shown.pace * dt * 2.6 / legs.max(0.4);
    if shown.blows != folk.blows {
      shown.blows = folk.blows;
      shown.swung = 0.0
    }
    shown.swung += dt;
    *transform = Transform::from_translation(shown.at)
      .with_rotation(Quat::from_rotation_y(shown.yaw));
    let walk = (shown.pace / 2.5).min(1.0);
    let phase = shown.stride;
    let seed = folk.luck as f32 * 0.37;
    let breath = (now * 1.7 + seed).sin();
    let strike = (shown.swung / 0.45).min(1.0);
    let lifting = (strike * PI).sin();
    let carrying = automaton.carrying.is_some();
    let fighter = matches!(folk.band, Band::Gladiator | Band::Paragon);
    for child in family.iter_descendants(entity) {
      if let Ok((joint, mut transform)) = joints.get_mut(child) {
        let side = if matches!(joint.0, 3 | 4 | 7 | 8) { 1.0 } else { -1.0 };
        let swing = phase.sin() * side;
        let bend = (-(phase + if side > 0.0 { 0.0 } else { PI }).cos()).max(0.0);
        transform.rotation = match joint.0 {
          automaton::PELVIS => {
            transform.translation.y =
              legs - (phase * 2.0).cos().abs() * 0.025 * walk * legs;
            Quat::from_rotation_y(swing * 0.08 * walk)
          }
          automaton::CHEST => Quat::from_euler(
            EulerRot::YXZ,
            -swing * 0.12 * walk + (now * 0.4 + seed).sin() * 0.05 * (1.0 - walk),
            0.04 + breath * 0.015 + walk * 0.06,
            0.0
          ),
          automaton::HEAD => Quat::from_euler(
            EulerRot::YXZ,
            (now * 0.53 + seed).sin() * 0.35 * (1.0 - walk),
            (now * 0.31 + seed * 2.0).sin() * 0.1
              - 0.05
              - if carrying { 0.25 } else { 0.0 },
            (now * 0.27 + seed).sin() * 0.08
          ),
          3 | 5 if carrying => {
            Quat::from_euler(EulerRot::XYZ, 2.75 + breath * 0.03, 0.0, side * -0.3)
          }
          4 | 6 if carrying => Quat::from_rotation_x(0.55),
          3 if fighter && strike < 1.0 => {
            Quat::from_euler(EulerRot::XYZ, 0.4 + lifting * 2.2, 0.0, -lifting * 0.25)
          }
          4 if fighter && strike < 1.0 => Quat::from_rotation_x(0.5 + lifting * 0.6),
          3 | 5 if !fighter && strike < 1.0 && folk.band == Band::Cogling => {
            Quat::from_euler(EulerRot::XYZ, lifting * 1.1, 0.0, side * -0.2 * lifting)
          }
          3 | 5 => Quat::from_euler(
            EulerRot::XYZ,
            -swing * 0.55 * walk + if fighter && side > 0.0 { 0.35 } else { 0.0 },
            0.0,
            side * (0.1 + breath * 0.02)
          ),
          4 | 6 => Quat::from_rotation_x(
            0.25
              + walk * 0.3 * (1.0 + swing)
              + if fighter && side > 0.0 { 0.5 } else { 0.0 }
          ),
          7 | 9 => Quat::from_rotation_x(swing * 0.55 * walk),
          _ => Quat::from_rotation_x(-bend * 0.95 * walk)
        }
      }
    }
  }
}

fn heed(
  buttons: Res<ButtonInput<MouseButton>>,
  keys: Res<ButtonInput<KeyCode>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  pilot: Res<Pilot>,
  selected: Res<Selected>,
  inventories: Query<&Inventory>,
  voxels: Option<Res<Voxels>>,
  robots: Query<(Entity, &Folk), With<Automaton>>,
  (mut rousings, mut orders, mut pokes): (
    MessageWriter<Rouse>,
    MessageWriter<Order>,
    MessageWriter<Poke>
  )
) {
  if let Some(voxels) = voxels
    && captured(&cursor, &menu)
    && pilot.riding.is_none()
    && buttons.just_pressed(MouseButton::Right)
  {
    let (from, ahead) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
    let wall = voxels.cast(from, ahead, REACH);
    let blocked =
      wall.as_ref().map_or(REACH, |hit| (hit.at.as_vec3() + 0.5).distance(from));
    let holding = inventories
      .get(pilot.me)
      .ok()
      .and_then(|inventory| inventory.slots[selected.0])
      .map(|stack| stack.block);
    let target = robots
      .iter()
      .filter(|(_, folk)| folk.band != Band::Scavver)
      .filter_map(|(entity, folk)| struck(folk, from, ahead).map(|near| (entity, near)))
      .filter(|&(_, near)| near <= ORDER_REACH.min(blocked + 0.5))
      .min_by(|a, b| a.1.total_cmp(&b.1));
    match (target, holding.and_then(awakened), wall) {
      (Some((robot, _)), ..) => {
        orders.write(Order {
          robot,
          pack: keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
        });
        pokes.write(Poke);
      }
      (None, Some(_), Some(hit)) => {
        rousings.write(Rouse(hit.at + hit.normal));
        pokes.write(Poke);
      }
      _ => ()
    }
  }
}

fn lineup(
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  mut done: Local<bool>,
  mut commands: Commands
) {
  if !*done
    && pilot.at != Vec3::ZERO
    && let Some(voxels) = voxels
  {
    *done = true;
    let (forward, right) = (ahead(pilot.yaw), Quat::from_rotation_y(pilot.yaw) * Vec3::X);
    let yaw = pilot.yaw + PI;
    for (index, &(band, luck)) in [
      (Band::Cogling, 0u32),
      (Band::Scavver, 4),
      (Band::Gladiator, 0),
      (Band::Paragon, 1),
      (Band::Cogling, 1),
      (Band::Gladiator, 2)
    ]
    .iter()
    .enumerate()
    {
      let (row, column) = ((index / 4) as f32, (index % 4) as f32);
      let spot = pilot.at + forward * (3.2 + row * 2.4) + right * (column - 1.5) * 1.5;
      let at = spot.with_y(
        generate::height(voxels.seed, spot.x.floor() as i32, spot.z.floor() as i32)
          as f32
          + 1.0
      );
      info!("{band:?} stands at {at}");
      commands.spawn((
        Folk { band, at, yaw, luck, blows: 0, aboard: false },
        Automaton { owner: None, staying: true, carrying: None },
        Health(band.health())
      ));
    }
  }
}

pub struct Robots;

impl Plugin for Robots {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Camps>()
      .add_systems(
        PreUpdate,
        command
          .after(ServerSystems::Receive)
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
      )
      .add_systems(
        Update,
        (muster, disperse, labour, wreck)
          .chain()
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
          .run_if(resource_exists::<Works>)
      );
    if app.world().resource::<Role>().plays() {
      automaton::register(app);
      app.init_resource::<Foundry>().add_systems(
        Update,
        (assemble, hoist, animate, heed.run_if(resource_exists::<Pilot>))
          .chain()
          .run_if(plays)
      );
      app.add_systems(
        Update,
        lineup.run_if(|| crate::opts::opts().robots).run_if(resource_exists::<Pilot>)
      );
    }
  }
}
