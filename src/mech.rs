use {crate::{audio::{Cue, Sound},
             authority::{Controller, player_of, within_reach},
             block::{Block, Fluid},
             death::Dying,
             haunt::Blamed,
             island::SEA,
             menu::Menu,
             player::{Bulk, Craft, Pilot, Riding, Selected, captured, cells, march,
                      slide},
             protocol::*,
             spell::{Bolt, Flourish},
             stream::ready_around,
             submarine::Drill,
             voxels::Voxels},
     bevy::{ecs::entity::MapEntities,
            prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize},
     std::f32::consts::{PI, TAU}};

pub const HARDPOINTS: usize = 6;
pub const BULK: Bulk = Bulk { half: 0.85, tall: 2.95 };
pub const SEAT: f32 = 0.85;
pub const BUILD_REACH: f32 = 9.0;
pub const CRUSHING: u8 = 3;
const HULL: f32 = 150.0;
const CHARGE: f32 = 600.0;
const WALK: f32 = 2.6;
const WADE: f32 = 0.6;
const STARVED: f32 = 0.3;
const RATED: f32 = 40.0;
const PLATED: f32 = 40.0;
const CRUSH: f32 = 4.0;
const SCALD: f32 = 8.0;
const MEND: f32 = 20.0;
const SPARK: f32 = 4.0;
const DRAW_IDLE: f32 = 0.05;
const DRAW_WALK: f32 = 0.35;
const DRAW_JETS: f32 = 2.5;
const DRAW_LIGHT: f32 = 0.15;
const DRAW_REPAIR: f32 = 0.4;
const DRAW_SLEEP: f32 = 2.0;
const DYNAMO: f32 = 0.4;
const REPAIR: f32 = 1.5;
const SLEEP_EVERY: f32 = 2.0;
const GRAVITY: f32 = 22.0;
const SINKING: f32 = 6.0;
const HOP: f32 = 7.0;
const THRUST: f32 = 32.0;
const CLIMB: f32 = 6.0;
const STEP_UP: f32 = 1.05;
const STEP: f32 = 1.0 / 60.0;
const STRIDE_EVERY: f32 = 0.05;
const BLOWN: f32 = 3.0;
const LASER_COST: f32 = 4.0;
const LASER_EVERY: f32 = 0.35;
const LASER_SPEED: f32 = 45.0;
const LASER_BLOW: u8 = 6;
const MISSILE_EVERY: f32 = 1.2;
const MISSILE_SPEED: f32 = 22.0;
const MISSILE_BLOW: u8 = 14;
const MISSILE_BLAST: f32 = 3.0;
const SHOT_LIFE: f32 = 2.5;
const FLOOR: f32 = 1.62;
const ROOF: f32 = 2.92;
const FRONT: f32 = -0.82;
const BACK: f32 = 0.8;
const MID: f32 = 0.12;
const HALF: f32 = 0.74;
const HIP: Vec3 = Vec3::new(0.55, 1.2, 0.0);
const THIGH: f32 = 0.62;
const SHIN: f32 = 0.62;
const ANKLE: f32 = 0.14;
const STANCE: f32 = 0.6;
const STRIDE: f32 = 0.55;
const LIFT: f32 = 0.3;

#[derive(Component, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Mech {
  pub at: Vec3,
  pub yaw: f32,
  pub hull: f32,
  pub charge: f32,
  pub modules: Vec<Block>,
  pub lit: bool,
  pub jets: bool,
  pub striding: bool,
  #[entities]
  pub rider: Option<Entity>
}

impl Mech {
  pub fn deployed(at: Vec3, yaw: f32) -> Mech {
    Mech {
      at,
      yaw,
      hull: HULL,
      charge: CHARGE * 0.5,
      modules: vec![],
      lit: false,
      jets: false,
      striding: false,
      rider: None
    }
  }

  pub fn has(&self, module: Block) -> bool { self.modules.contains(&module) }

  pub fn rated(&self) -> Option<f32> {
    match (self.has(Block::ResonantPlating), self.has(Block::HullPlating)) {
      (true, _) => None,
      (_, true) => Some(RATED + PLATED),
      _ => Some(RATED)
    }
  }

  pub fn depth(&self) -> f32 { SEA as f32 - (self.at.y + SEAT + EYE) }

  pub fn capacity(&self) -> f32 {
    CHARGE
      * (1.0
        + self.modules.iter().filter(|&&module| module == Block::PowerCell).count()
          as f32)
  }

  pub fn armour(&self) -> f32 {
    match (self.has(Block::ResonantPlating), self.has(Block::HullPlating)) {
      (true, _) => 0.4,
      (_, true) => 0.6,
      _ => 1.0
    }
  }

  pub fn speed(&self) -> f32 { WALK * if self.charge > 0.0 { 1.0 } else { STARVED } }

  pub fn drill(&self) -> Option<Drill> {
    match (self.has(Block::DiamondDrill), self.has(Block::MechDrill)) {
      (true, _) => Some(Drill { speed: 6.0, reach: 7.0, wide: true }),
      (_, true) => Some(Drill { speed: 2.8, reach: 6.0, wide: false }),
      _ => None
    }
  }

  pub fn reach(&self) -> Option<f32> { self.has(Block::BuildArm).then_some(BUILD_REACH) }

  pub fn sealed(&self) -> bool { self.charge > 0.0 }
}

pub fn fitting(block: Block) -> bool {
  block.rig()
    || matches!(
      block,
      Block::Floodlight | Block::HullPlating | Block::ResonantPlating | Block::PowerCell
    )
}

fn fits(mech: &Mech, module: Block) -> bool {
  fitting(module)
    && mech.modules.len() < HARDPOINTS
    && (module == Block::PowerCell || !mech.has(module))
}

fn wet(voxels: &Voxels, at: Vec3) -> bool {
  voxels.block(at.floor().as_ivec3()).is_some_and(Block::wet)
}

fn scalding(voxels: &Voxels, at: Vec3) -> bool {
  let (low, high) = BULK.body(at);
  cells(low, high).any(|cell| {
    voxels
      .block(cell)
      .and_then(Block::liquid)
      .is_some_and(|(fluid, _)| fluid == Fluid::Lava)
  })
}

pub fn struck(mech: &Mech, from: Vec3, toward: Vec3) -> Option<f32> {
  let turn = Quat::from_rotation_y(-mech.yaw);
  crate::boat::pierce(
    Vec3::new(-BULK.half - 0.2, 0.0, FRONT - 0.1),
    Vec3::new(BULK.half + 0.2, BULK.tall, BACK + 0.4),
    turn * (from - mech.at),
    turn * toward
  )
}

pub fn shield(mechs: &mut Query<&mut Mech>, player: Entity, blow: u8) -> bool {
  mechs
    .iter_mut()
    .find(|mech| mech.rider == Some(player))
    .map(|mut mech| {
      mech.hull = (mech.hull - f32::from(blow) * BLOWN * mech.armour()).max(0.0)
    })
    .is_some()
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Deploy(pub IVec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Board(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Pack(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Refit(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Unfit(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Stride {
  pub at: Vec3,
  pub yaw: f32,
  pub jets: bool,
  pub lit: bool,
  pub striding: bool
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Weapon {
  Laser,
  Missile
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Shoot {
  pub weapon: Weapon,
  pub toward: Vec3
}

#[derive(Component)]
struct Shot {
  velocity: Vec3,
  life: f32,
  owner: Entity,
  blow: u8,
  blast: f32
}

#[derive(Component, Default)]
struct Armed {
  laser: f32,
  missile: f32,
  sleeping: f32,
  falling: f32
}

pub fn spawn(commands: &mut Commands, mech: Mech) {
  commands.spawn((Replicated, mech, Armed::default()));
}

fn tell(notices: &mut MessageWriter<ToClients<Notice>>, client: ClientId, word: String) {
  notices
    .write(ToClients { targets: SendTargets::Single(client), message: Notice(word) });
}

fn serve(
  (
    mut deploys,
    mut boards,
    mut packs,
    mut refits,
    mut unfits,
    mut leaves,
    mut strides,
    mut shots
  ): (
    MessageReader<FromClient<Deploy>>,
    MessageReader<FromClient<Board>>,
    MessageReader<FromClient<Pack>>,
    MessageReader<FromClient<Refit>>,
    MessageReader<FromClient<Unfit>>,
    MessageReader<FromClient<Disembark>>,
    MessageReader<FromClient<Stride>>,
    MessageReader<FromClient<Shoot>>
  ),
  mut voxels: ResMut<Voxels>,
  mut commands: Commands,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Inventory))>,
  mut mechs: Query<(Entity, &mut Mech, &mut Armed)>,
  mut notices: MessageWriter<ToClients<Notice>>
) {
  let near = |avatar: &Avatar, mech: &Mech| {
    within_reach(avatar, (mech.at + Vec3::Y * 1.5).floor().as_ivec3())
  };
  for &FromClient { client_id, message: Deploy(cell) } in deploys.read() {
    let base = cell.as_vec3() + Vec3::new(0.5, 0.0, 0.5);
    let (low, high) = BULK.body(base);
    let roomy = cells(low, high).all(|cell| !voxels.ensure(cell).solid());
    if let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id) {
      match roomy && within_reach(avatar, cell) {
        true if inventory.take(Block::Mech) => {
          spawn(&mut commands, Mech::deployed(base, avatar.yaw))
        }
        false if !roomy => tell(
          &mut notices,
          client_id,
          "A mech needs room: 2 blocks wide and 3 tall".into()
        ),
        _ => ()
      }
    }
  }
  for &FromClient { client_id, message: Board(entity) } in boards.read() {
    if let Some((player, avatar, _)) = player_of(players.iter_mut(), client_id)
      && mechs.iter().all(|(_, mech, _)| mech.rider != Some(player))
      && let Ok((_, mut mech, _)) = mechs.get_mut(entity)
      && mech.rider.is_none()
      && near(avatar, &mech)
    {
      mech.rider = Some(player)
    }
  }
  for &FromClient { client_id, message: Pack(entity) } in packs.read() {
    if let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Ok((_, mech, _)) = mechs.get(entity)
      && mech.rider.is_none()
      && near(avatar, mech)
      && inventory.add(Block::Mech)
    {
      for &module in mech.modules.iter().filter(|&&module| !inventory.add(module)) {
        crate::loose::scatter(
          &mut commands,
          module,
          1,
          avatar.at + Vec3::Y,
          Vec3::Y * 2.0,
          0.5
        )
      }
      commands.entity(entity).despawn()
    }
  }
  for &FromClient { client_id, message: Refit(entity) } in refits.read() {
    if let Some((player, avatar, mut inventory)) =
      player_of(players.iter_mut(), client_id)
      && let Ok((_, mut mech, _)) = mechs.get_mut(entity)
      && (mech.rider == Some(player) || near(avatar, &mech))
      && let Some(stack) = inventory.slots[usize::from(avatar.hand)]
    {
      let held = stack.block;
      let fuel = crate::balloon::burns(held).map(|seconds| seconds * SPARK);
      let done = match () {
        () if fits(&mech, held) && inventory.take(held) => {
          mech.modules.push(held);
          Some(format!("Fitted the {}", held.name()))
        }
        () if fitting(held) => Some(match mech.modules.len() >= HARDPOINTS {
          true => format!(
            "All {HARDPOINTS} hardpoints are full; Shift+right-click to take one out"
          ),
          false => format!("A {} is already fitted", held.name())
        }),
        () if held == Block::IronOre && mech.hull < HULL && inventory.take(held) => {
          mech.hull = (mech.hull + MEND).min(HULL);
          Some(format!("Frame patched: {:.0}%", mech.hull / HULL * 100.0))
        }
        () if held == Block::AetherCell && inventory.take(held) => {
          mech.charge = mech.capacity();
          Some("Aether cell slotted: fully charged".into())
        }
        () if let Some(spark) = fuel
          && mech.charge < mech.capacity()
          && inventory.take(held) =>
        {
          mech.charge = (mech.charge + spark).min(mech.capacity());
          Some(format!("Charge {:.0} s", mech.charge))
        }
        () => None
      };
      if let Some(word) = done {
        tell(&mut notices, client_id, word)
      }
    }
  }
  for &FromClient { client_id, message: Unfit(entity) } in unfits.read() {
    if let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Ok((_, mut mech, _)) = mechs.get_mut(entity)
      && near(avatar, &mech)
      && let Some(&module) = mech.modules.last()
      && inventory.add(module)
    {
      mech.modules.pop();
      mech.charge = mech.charge.min(mech.capacity())
    }
  }
  for &FromClient { client_id, .. } in leaves.read() {
    if let Some((player, ..)) = player_of(players.iter_mut(), client_id) {
      for (_, mut mech, _) in
        mechs.iter_mut().filter(|(_, mech, _)| mech.rider == Some(player))
      {
        mech.rider = None;
        mech.jets = false;
        mech.striding = false
      }
    }
  }
  for &FromClient { client_id, message: Stride { at, yaw, jets, lit, striding } } in
    strides.read()
  {
    if let Some((player, ..)) = player_of(players.iter_mut(), client_id)
      && at.is_finite()
      && yaw.is_finite()
    {
      for (_, mut mech, _) in
        mechs.iter_mut().filter(|(_, mech, _)| mech.rider == Some(player))
      {
        let jets = jets && mech.has(Block::Jumpjets) && mech.charge > 0.0;
        let lit = lit && mech.has(Block::Floodlight);
        let moved = Mech { at, yaw, jets, lit, striding, ..mech.clone() };
        mech.set_if_neq(moved);
      }
    }
  }
  for &FromClient { client_id, message: Shoot { weapon, toward } } in shots.read() {
    if let Some((player, _, mut inventory)) = player_of(players.iter_mut(), client_id)
      && toward.is_finite()
      && let Some((_, mut mech, mut armed)) =
        mechs.iter_mut().find(|(_, mech, _)| mech.rider == Some(player))
    {
      let toward = toward.normalize_or(Vec3::NEG_Z);
      let muzzle = mech.at + Vec3::Y * 2.2 + toward * 1.4;
      let fired = match weapon {
        Weapon::Laser
          if mech.has(Block::MechLaser)
            && armed.laser <= 0.0
            && mech.charge >= LASER_COST =>
        {
          armed.laser = LASER_EVERY;
          mech.charge -= LASER_COST;
          Some((Block::SpellFirebolt, LASER_SPEED, LASER_BLOW, 0.0))
        }
        Weapon::Missile
          if mech.has(Block::MissileRack)
            && armed.missile <= 0.0
            && inventory.take(Block::Torpedo) =>
        {
          armed.missile = MISSILE_EVERY;
          Some((Block::SpellFireball, MISSILE_SPEED, MISSILE_BLOW, MISSILE_BLAST))
        }
        _ => None
      };
      if let Some((spell, speed, blow, blast)) = fired {
        commands.spawn((Replicated, Bolt { spell, at: muzzle }, Shot {
          velocity: toward * speed,
          life: SHOT_LIFE,
          owner: player,
          blow,
          blast
        }));
      }
    }
  }
}

fn endure(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut mechs: Query<(Entity, &mut Mech, &mut Armed)>,
  mut players: Query<(&Controller, &mut Health, &crate::folk::Vigour)>,
  mut notices: MessageWriter<ToClients<Notice>>,
  mut warned: Local<f32>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  *warned -= dt;
  for (entity, mut mech, mut armed) in mechs.iter_mut() {
    armed.laser -= dt;
    armed.missile -= dt;
    let rider = mech.rider.and_then(|rider| players.get_mut(rider).ok());
    let client = rider.as_ref().map(|(controller, ..)| controller.client);
    let mut warn = |word: String| {
      if let Some(client) = client
        && *warned <= 0.0
      {
        *warned = 3.0;
        tell(&mut notices, client, word)
      }
    };
    let crushed = mech.rated().is_some_and(|rated| mech.depth() > rated);
    let scalded = !mech.has(Block::HeatShield) && scalding(&voxels, mech.at);
    let mending = mech.has(Block::RepairDroid) && mech.hull < HULL && mech.charge > 0.0;
    let draw = match rider.is_some() {
      true => {
        DRAW_IDLE
          + DRAW_WALK * f32::from(u8::from(mech.striding))
          + DRAW_JETS * f32::from(u8::from(mech.jets))
          + DRAW_LIGHT * f32::from(u8::from(mech.lit))
      }
      false => 0.0
    } + DRAW_REPAIR * f32::from(u8::from(mending));
    let gain = DYNAMO * f32::from(u8::from(mech.has(Block::Dynamo)));
    let charge = (mech.charge + (gain - draw) * dt).clamp(0.0, mech.capacity());
    if (charge - mech.charge).abs() > 0.0 {
      mech.charge = charge
    }
    if mending {
      mech.hull = (mech.hull + REPAIR * dt).min(HULL)
    }
    if crushed {
      mech.hull = (mech.hull - CRUSH * dt).max(0.0);
      warn(format!(
        "Frame groaning: rated to {:.0} m, at {:.0} m ({:.0}%)",
        mech.rated().unwrap_or_default(),
        mech.depth(),
        mech.hull / HULL * 100.0
      ))
    }
    if scalded {
      mech.hull = (mech.hull - SCALD * dt).max(0.0);
      warn(format!(
        "Lava is melting the frame ({:.0}%): fit heat shielding",
        mech.hull / HULL * 100.0
      ))
    }
    armed.sleeping += dt;
    if let Some((_, mut health, vigour)) = rider
      && mech.has(Block::Sleeper)
      && health.0 > 0
      && health.0 < vigour.most
      && mech.charge >= DRAW_SLEEP
      && armed.sleeping >= SLEEP_EVERY
    {
      armed.sleeping = 0.0;
      health.0 += 1;
      mech.charge -= DRAW_SLEEP
    }
    if mech.rider.is_none() {
      let at = mech.at;
      for offset in [IVec3::ZERO, IVec3::NEG_Y, IVec3::Y * 3] {
        voxels.ensure(at.floor().as_ivec3() + offset);
      }
      armed.falling = match wet(&voxels, at + Vec3::Y) {
        true => (armed.falling - SINKING * dt).max(-3.0),
        false => (armed.falling - GRAVITY * dt).max(-40.0)
      };
      let marched = march(&voxels, BULK, at, Vec3::Y * armed.falling, dt);
      if marched.landed {
        armed.falling = 0.0
      }
      if marched.at.distance(at) > 1e-3 {
        mech.at = marched.at
      }
    }
    if mech.hull <= 0.0 {
      if let Some(client) = client {
        tell(&mut notices, client, "The frame gave way! The mech is wrecked".into())
      }
      let at = mech.at + Vec3::Y * 1.5;
      for &module in mech.modules.iter() {
        crate::loose::scatter(&mut commands, module, 1, at, Vec3::Y * 3.0, 0.5)
      }
      crate::loose::scatter(&mut commands, Block::IronOre, 6, at, Vec3::Y * 3.0, 0.8);
      commands.entity(entity).despawn()
    }
  }
}

fn unseat(players: Query<(), With<Controller>>, mut mechs: Query<&mut Mech>) {
  for mut mech in mechs.iter_mut() {
    if mech.rider.is_some_and(|rider| players.get(rider).is_err()) {
      mech.rider = None;
      mech.jets = false;
      mech.striding = false
    }
  }
}

fn volley(
  time: Res<Time>,
  voxels: Res<Voxels>,
  mut shots: Query<(Entity, &mut Bolt, &mut Shot)>,
  mut victims: Query<
    (Entity, Option<&Folk>, Option<&Beast>, &mut Health),
    (Without<Controller>, Without<Dying>)
  >,
  mut flourishes: MessageWriter<ToClients<Flourish>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  let place = |folk: Option<&Folk>, beast: Option<&Beast>| {
    folk.map(|folk| folk.at).or(beast.map(|beast| beast.at)).map(|at| at + Vec3::Y * 0.8)
  };
  for (entity, mut bolt, mut shot) in shots.iter_mut() {
    let (from, step) = (bolt.at, shot.velocity * dt);
    let (direction, length) = (step.normalize_or_zero(), step.length());
    let wall = voxels.cast_for(from, direction, length, Block::solid).is_some();
    let hit = victims
      .iter()
      .filter(|(.., health)| health.0 > 0)
      .filter_map(|(victim, folk, beast, _)| {
        place(folk, beast)
          .filter(|&at| at.distance(from + step) < 1.4)
          .map(|at| (victim, at))
      })
      .min_by(|a, b| a.1.distance(from).total_cmp(&b.1.distance(from)));
    shot.life -= dt;
    match wall || hit.is_some() || shot.life <= 0.0 {
      true => {
        let at = hit.map_or(from + step * 0.5, |(_, at)| at);
        let blasted: Vec<Entity> = match shot.blast > 0.0 {
          true => victims
            .iter()
            .filter(|&(_, folk, beast, _)| {
              place(folk, beast).is_some_and(|spot| spot.distance(at) < shot.blast)
            })
            .map(|(victim, ..)| victim)
            .collect(),
          false => hit.map(|(victim, _)| victim).into_iter().collect()
        };
        for victim in blasted {
          if let Ok((_, _, _, mut health)) = victims.get_mut(victim)
            && health.0 > 0
          {
            health.0 = health.0.saturating_sub(shot.blow);
            commands.entity(victim).insert(Blamed(shot.owner));
            if health.0 == 0 {
              crate::haunt::fell(&mut commands, victim)
            }
          }
        }
        flourishes.write(ToClients {
          targets: SendTargets::All,
          message: Flourish { spell: bolt.spell, at, reach: shot.blast.max(0.6) }
        });
        commands.entity(entity).despawn()
      }
      false => bolt.at = from + step
    }
  }
}

fn muster(
  players: Query<(Entity, &Avatar), With<Controller>>,
  mut done: Local<bool>,
  mut commands: Commands
) {
  if crate::opts::opts().mech
    && !*done
    && let Some((player, avatar)) = players.iter().next()
  {
    *done = true;
    let at = crate::opts::opts().inside.map_or(avatar.at, Vec3::from);
    spawn(&mut commands, Mech {
      modules: vec![
        Block::DiamondDrill,
        Block::MechLaser,
        Block::Jumpjets,
        Block::Floodlight,
        Block::HeatShield,
        Block::ResonantPlating,
      ],
      charge: CHARGE,
      rider: Some(player),
      ..Mech::deployed(at, avatar.yaw)
    })
  }
}

#[derive(Resource)]
struct Kit {
  slab: Handle<Mesh>,
  rod: Handle<Mesh>,
  ball: Handle<Mesh>,
  cone: Handle<Mesh>,
  paint: Handle<StandardMaterial>,
  plated: Handle<StandardMaterial>,
  scorched: Handle<StandardMaterial>,
  dark: Handle<StandardMaterial>,
  steel: Handle<StandardMaterial>,
  cabin: Handle<StandardMaterial>,
  crate_: Handle<StandardMaterial>,
  glass: Handle<StandardMaterial>,
  glow: Handle<StandardMaterial>,
  amber: Handle<StandardMaterial>,
  ember: Handle<StandardMaterial>,
  screen: Handle<StandardMaterial>,
  beam: Handle<StandardMaterial>,
  flame: Handle<StandardMaterial>,
  green: Handle<StandardMaterial>
}

fn build(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let colour = |r: f32, g: f32, b: f32, rough: f32, metal: f32| StandardMaterial {
    base_color: Color::srgb(r, g, b),
    perceptual_roughness: rough,
    metallic: metal,
    ..default()
  };
  let shining = |[r, g, b]: [f32; 3], strength: f32| StandardMaterial {
    base_color: Color::srgb(r, g, b),
    emissive: LinearRgba::rgb(r, g, b) * strength,
    unlit: true,
    ..default()
  };
  commands.insert_resource(Kit {
    slab: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
    rod: meshes.add(Cylinder::new(0.5, 1.0).mesh().resolution(12)),
    ball: meshes.add(Sphere::new(0.5).mesh().uv(14, 8)),
    cone: meshes.add(Cone { radius: 0.5, height: 1.0 }),
    paint: materials.add(colour(0.9, 0.55, 0.12, 0.5, 0.25)),
    plated: materials.add(colour(0.45, 0.48, 0.52, 0.35, 0.8)),
    scorched: materials.add(colour(0.42, 0.16, 0.08, 0.6, 0.5)),
    dark: materials.add(colour(0.13, 0.13, 0.15, 0.6, 0.5)),
    steel: materials.add(colour(0.32, 0.34, 0.37, 0.4, 0.85)),
    cabin: materials.add(StandardMaterial {
      double_sided: true,
      cull_mode: None,
      ..colour(0.5, 0.52, 0.5, 0.7, 0.2)
    }),
    crate_: materials.add(colour(0.5, 0.36, 0.2, 0.85, 0.05)),
    glass: materials.add(StandardMaterial {
      base_color: Color::srgba(0.72, 0.9, 1.0, 0.05),
      alpha_mode: AlphaMode::Blend,
      perceptual_roughness: 0.06,
      reflectance: 0.3,
      double_sided: true,
      cull_mode: None,
      ..default()
    }),
    glow: materials.add(shining([0.3, 0.95, 1.0], 6.0)),
    amber: materials.add(shining([1.0, 0.7, 0.2], 3.0)),
    ember: materials.add(shining([1.0, 0.25, 0.15], 6.0)),
    screen: materials.add(shining([0.2, 0.9, 0.5], 2.0)),
    beam: materials.add(shining([1.0, 0.95, 0.8], 12.0)),
    flame: materials.add(StandardMaterial {
      alpha_mode: AlphaMode::Add,
      ..shining([1.0, 0.55, 0.15], 10.0)
    }),
    green: materials.add(shining([0.3, 1.0, 0.4], 3.0))
  })
}

#[derive(Component)]
struct Shown(Vec<Block>);

#[derive(Component)]
struct Torso(Entity);

#[derive(Component)]
struct Fittings;

#[derive(Component)]
struct Beam(Entity);

#[derive(Component)]
struct Flame(Entity);

#[derive(Component)]
struct Drone(Entity);

#[derive(Clone, Copy, PartialEq)]
enum Segment {
  Thigh,
  Knee,
  Shin,
  Foot
}

#[derive(Component)]
struct Limb {
  mech: Entity,
  side: usize,
  segment: Segment
}

#[derive(Component)]
struct Strut {
  last: Vec3,
  phase: f32,
  amp: f32,
  heading: Vec3,
  speed: f32
}

fn piece(
  mesh: &Handle<Mesh>,
  material: &Handle<StandardMaterial>,
  at: Vec3,
  size: Vec3
) -> impl Bundle {
  (
    Mesh3d(mesh.clone()),
    MeshMaterial3d(material.clone()),
    Transform::from_translation(at).with_scale(size)
  )
}

fn spanning(from: Vec3, to: Vec3, thick: f32) -> Transform {
  Transform::from_translation((from + to) / 2.0)
    .with_rotation(Quat::from_rotation_arc(Vec3::Y, (to - from).normalize_or(Vec3::Y)))
    .with_scale(Vec3::new(thick, from.distance(to), thick))
}

fn bar(
  kit: &Kit,
  material: &Handle<StandardMaterial>,
  from: Vec3,
  to: Vec3,
  thick: f32
) -> impl Bundle {
  (Mesh3d(kit.slab.clone()), MeshMaterial3d(material.clone()), spanning(from, to, thick))
}

fn cockpit(kit: &Kit, mech: Entity, torso: &mut ChildSpawnerCommands) {
  let middle = (FLOOR + ROOF) / 2.0;
  let tall = ROOF - FLOOR;
  torso.spawn(piece(
    &kit.slab,
    &kit.glass,
    Vec3::new(0.0, middle, (FRONT + MID) / 2.0),
    Vec3::new(HALF * 2.0, tall, MID - FRONT)
  ));
  for (x, z) in [(-HALF, FRONT), (HALF, FRONT), (-HALF, MID), (HALF, MID)] {
    torso.spawn(piece(
      &kit.slab,
      &kit.steel,
      Vec3::new(x, middle, z),
      Vec3::new(0.07, tall, 0.07)
    ));
  }
  for y in [FLOOR + 0.03, ROOF - 0.03] {
    torso.spawn(piece(
      &kit.slab,
      &kit.steel,
      Vec3::new(0.0, y, FRONT),
      Vec3::new(HALF * 2.0, 0.07, 0.07)
    ));
  }
  torso.spawn(piece(
    &kit.slab,
    &kit.dark,
    Vec3::new(0.0, FLOOR, (FRONT + BACK) / 2.0),
    Vec3::new(HALF * 2.0, 0.06, BACK - FRONT)
  ));
  torso.spawn(piece(
    &kit.slab,
    &kit.cabin,
    Vec3::new(0.0, middle, MID + 0.02),
    Vec3::new(HALF * 2.0, tall, 0.04)
  ));
  torso.spawn(piece(
    &kit.slab,
    &kit.dark,
    Vec3::new(0.0, 1.92, -0.62),
    Vec3::new(1.0, 0.5, 0.3)
  ));
  let console = Quat::from_rotation_x(0.5);
  torso.spawn((
    Mesh3d(kit.slab.clone()),
    MeshMaterial3d(kit.screen.clone()),
    Transform::from_xyz(0.0, 2.18, -0.58)
      .with_rotation(console)
      .with_scale(Vec3::new(0.42, 0.01, 0.2))
  ));
  for (x, light) in
    [(-0.38, &kit.amber), (-0.28, &kit.glow), (0.28, &kit.amber), (0.38, &kit.glow)]
  {
    torso.spawn((
      Mesh3d(kit.rod.clone()),
      MeshMaterial3d(light.clone()),
      Transform::from_xyz(x, 2.17, -0.56)
        .with_rotation(console)
        .with_scale(Vec3::new(0.06, 0.012, 0.06))
    ));
  }
  for side in [-1.0f32, 1.0] {
    torso.spawn((
      Mesh3d(kit.rod.clone()),
      MeshMaterial3d(kit.steel.clone()),
      Transform::from_xyz(side * 0.26, 2.15, -0.36)
        .with_rotation(Quat::from_rotation_x(-0.4))
        .with_scale(Vec3::new(0.04, 0.2, 0.04))
    ));
  }
  torso.spawn((
    PointLight {
      range: 3.0,
      intensity: crate::sky::torchlight().intensity * 0.6,
      ..crate::sky::torchlight()
    },
    Transform::from_xyz(0.0, ROOF - 0.25, -0.2)
  ));
  torso.spawn((
    Beam(mech),
    SpotLight {
      intensity: 0.0,
      range: 48.0,
      outer_angle: 0.6,
      inner_angle: 0.3,
      color: Color::srgb(1.0, 0.95, 0.85),
      shadow_maps_enabled: false,
      ..default()
    },
    Transform::from_xyz(0.0, ROOF - 0.1, FRONT - 0.15)
  ));
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
  Fist,
  Drill,
  Diamond,
  Clamp,
  Laser
}

fn arm(
  kit: &Kit,
  body: &Handle<StandardMaterial>,
  side: f32,
  tool: Tool,
  fitted: &mut ChildSpawnerCommands
) {
  let shoulder = Vec3::new(side * (HALF + 0.17), 2.4, 0.35);
  let elbow = Vec3::new(side * 0.95, 1.85, 0.15);
  let wrist = Vec3::new(side * 0.95, 1.8, -0.55);
  fitted.spawn(piece(&kit.slab, body, shoulder, Vec3::new(0.34, 0.42, 0.5)));
  fitted.spawn(bar(kit, &kit.steel, shoulder, elbow, 0.16));
  fitted.spawn(piece(&kit.ball, &kit.dark, elbow, Vec3::splat(0.22)));
  fitted.spawn(bar(kit, body, elbow, wrist, 0.22));
  let ahead = Quat::from_rotation_arc(Vec3::Y, Vec3::NEG_Z);
  match tool {
    Tool::Fist => {
      fitted.spawn(piece(
        &kit.slab,
        &kit.dark,
        wrist + Vec3::Z * -0.1,
        Vec3::splat(0.26)
      ));
    }
    Tool::Drill | Tool::Diamond => {
      let bit = if tool == Tool::Diamond { &kit.glow } else { &kit.steel };
      fitted.spawn(piece(&kit.rod, &kit.dark, wrist, Vec3::new(0.3, 0.12, 0.3)));
      fitted.spawn((
        Mesh3d(kit.cone.clone()),
        MeshMaterial3d(bit.clone()),
        Transform::from_translation(wrist + Vec3::Z * -0.36)
          .with_rotation(ahead)
          .with_scale(Vec3::new(0.3, 0.62, 0.3))
      ));
    }
    Tool::Clamp => {
      for finger in [-1.0f32, 1.0] {
        let root = wrist + Vec3::new(finger * 0.09, 0.0, -0.08);
        let knuckle = root + Vec3::new(finger * 0.12, 0.0, -0.22);
        fitted.spawn(bar(kit, &kit.steel, root, knuckle, 0.08));
        fitted.spawn(bar(
          kit,
          &kit.steel,
          knuckle,
          knuckle + Vec3::new(-finger * 0.1, 0.0, -0.18),
          0.07
        ));
      }
    }
    Tool::Laser => {
      fitted.spawn(bar(kit, &kit.dark, wrist, wrist + Vec3::Z * -0.75, 0.13));
      fitted.spawn(piece(
        &kit.slab,
        &kit.steel,
        wrist + Vec3::Z * -0.25,
        Vec3::new(0.2, 0.2, 0.3)
      ));
      fitted.spawn(piece(
        &kit.ball,
        &kit.ember,
        wrist + Vec3::Z * -0.78,
        Vec3::splat(0.1)
      ));
    }
  }
}

fn outfit(kit: &Kit, mech: Entity, rig: &Mech, fitted: &mut ChildSpawnerCommands) {
  let body = match (
    rig.has(Block::HullPlating) || rig.has(Block::ResonantPlating),
    rig.has(Block::HeatShield)
  ) {
    (true, _) => &kit.plated,
    (_, true) => &kit.scorched,
    _ => &kit.paint
  };
  let middle = (FLOOR + ROOF) / 2.0;
  fitted.spawn(piece(
    &kit.slab,
    body,
    Vec3::new(0.0, middle, (MID + BACK) / 2.0 + 0.02),
    Vec3::new(HALF * 2.0 + 0.06, ROOF - FLOOR + 0.06, BACK - MID)
  ));
  fitted.spawn(piece(
    &kit.slab,
    body,
    Vec3::new(0.0, ROOF + 0.04, (FRONT + BACK) / 2.0),
    Vec3::new(HALF * 2.0 + 0.1, 0.08, BACK - FRONT + 0.1)
  ));
  fitted.spawn(piece(
    &kit.slab,
    body,
    Vec3::new(0.0, FLOOR - 0.12, (FRONT + MID) / 2.0 - 0.02),
    Vec3::new(HALF * 2.0, 0.22, MID - FRONT + 0.04)
  ));
  fitted.spawn(piece(
    &kit.slab,
    &kit.dark,
    Vec3::new(0.0, 1.2, 0.1),
    Vec3::new(0.85, 0.8, 0.8)
  ));
  for side in [-1.0f32, 1.0] {
    fitted.spawn(piece(
      &kit.ball,
      &kit.dark,
      HIP * Vec3::new(side, 1.0, 1.0),
      Vec3::splat(0.32)
    ));
    fitted.spawn(piece(
      &kit.slab,
      &kit.steel,
      Vec3::new(side * 0.25, 1.75, BACK + 0.02),
      Vec3::new(0.12, 0.2, 0.04)
    ));
  }
  let drill = match (rig.has(Block::DiamondDrill), rig.has(Block::MechDrill)) {
    (true, _) => Some(Tool::Diamond),
    (_, true) => Some(Tool::Drill),
    _ => None
  };
  let clamp = rig.has(Block::HydraulicClamp).then_some(Tool::Clamp);
  let laser = rig.has(Block::MechLaser).then_some(Tool::Laser);
  let left = drill.or(clamp).unwrap_or(Tool::Fist);
  let right = laser.or(clamp.filter(|_| left != Tool::Clamp)).unwrap_or(Tool::Fist);
  arm(kit, body, -1.0, left, fitted);
  arm(kit, body, 1.0, right, fitted);
  let cells = rig.modules.iter().filter(|&&module| module == Block::PowerCell).count();
  for cell in 0..cells {
    fitted.spawn(piece(
      &kit.slab,
      &kit.glow,
      Vec3::new(-0.45 + cell as f32 * 0.18, 2.75, BACK + 0.02),
      Vec3::new(0.1, 0.2, 0.03)
    ));
  }
  for &module in rig.modules.iter() {
    match module {
      Block::Floodlight => {
        for side in [-1.0, 1.0] {
          fitted.spawn((
            Mesh3d(kit.rod.clone()),
            MeshMaterial3d(kit.beam.clone()),
            Transform::from_xyz(side * 0.45, ROOF + 0.15, FRONT + 0.12)
              .with_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::NEG_Z))
              .with_scale(Vec3::new(0.2, 0.18, 0.2))
          ));
        }
      }
      Block::MissileRack => {
        let pod = Vec3::new(0.48, ROOF + 0.24, 0.42);
        fitted.spawn(piece(&kit.slab, &kit.dark, pod, Vec3::new(0.46, 0.34, 0.6)));
        for (x, y) in [(-0.1, -0.08), (0.1, -0.08), (-0.1, 0.08), (0.1, 0.08)] {
          fitted.spawn(piece(
            &kit.ball,
            &kit.amber,
            pod + Vec3::new(x, y, -0.3),
            Vec3::splat(0.09)
          ));
        }
      }
      Block::Jumpjets => {
        for side in [-1.0f32, 1.0] {
          let nozzle = Vec3::new(side * 0.42, 1.8, BACK + 0.14);
          fitted.spawn((
            Mesh3d(kit.cone.clone()),
            MeshMaterial3d(kit.dark.clone()),
            Transform::from_translation(nozzle).with_scale(Vec3::new(0.26, 0.32, 0.26))
          ));
          fitted.spawn((
            Flame(mech),
            Mesh3d(kit.cone.clone()),
            MeshMaterial3d(kit.flame.clone()),
            Transform::from_translation(nozzle - Vec3::Y * 0.45)
              .with_rotation(Quat::from_rotation_x(PI))
              .with_scale(Vec3::new(0.2, 0.6, 0.2)),
            Visibility::Hidden
          ));
        }
      }
      Block::OreBox => {
        fitted.spawn(piece(
          &kit.slab,
          &kit.crate_,
          Vec3::new(0.0, 2.25, BACK + 0.24),
          Vec3::new(1.0, 0.7, 0.42)
        ));
        fitted.spawn(piece(
          &kit.slab,
          &kit.steel,
          Vec3::new(0.0, 2.25, BACK + 0.46),
          Vec3::new(1.04, 0.08, 0.03)
        ));
      }
      Block::Dynamo => {
        fitted.spawn(piece(
          &kit.rod,
          &kit.amber,
          Vec3::new(-0.42, ROOF + 0.2, 0.45),
          Vec3::new(0.26, 0.3, 0.26)
        ));
      }
      Block::Sleeper => {
        for side in [-1.0f32, 1.0] {
          let at = Vec3::new(side * (HALF + 0.04), 2.5, 0.45);
          fitted.spawn(piece(&kit.slab, &kit.green, at, Vec3::new(0.02, 0.32, 0.1)));
          fitted.spawn(piece(&kit.slab, &kit.green, at, Vec3::new(0.02, 0.1, 0.32)));
        }
      }
      Block::RepairDroid => {
        fitted
          .spawn((Drone(mech), Transform::default(), Visibility::default()))
          .with_children(|drone| {
            drone.spawn(piece(&kit.ball, &kit.steel, Vec3::ZERO, Vec3::splat(0.24)));
            drone.spawn(piece(&kit.ball, &kit.glow, Vec3::Z * -0.1, Vec3::splat(0.08)));
          });
      }
      Block::HeatShield => {
        for side in [-1.0f32, 1.0] {
          fitted.spawn(piece(
            &kit.slab,
            &kit.scorched,
            Vec3::new(side * (HALF + 0.06), 1.45, 0.0),
            Vec3::new(0.06, 0.5, 1.5)
          ));
        }
      }
      Block::BuildArm => {
        let base = Vec3::new(-0.5, ROOF + 0.08, 0.5);
        let tip = Vec3::new(-0.95, ROOF + 0.6, -0.35);
        fitted.spawn(bar(kit, &kit.paint, base, tip, 0.1));
        fitted.spawn(piece(&kit.ball, &kit.amber, tip, Vec3::splat(0.14)));
      }
      _ => ()
    }
  }
}

fn assemble(kit: &Kit, commands: &mut Commands, entity: Entity, placed: Transform) {
  commands.entity(entity).insert((placed, Visibility::default())).with_children(|root| {
    root
      .spawn((Torso(entity), Transform::default(), Visibility::default()))
      .with_children(|torso| cockpit(kit, entity, torso));
    for side in 0..2 {
      for (segment, mesh, material, size) in [
        (Segment::Thigh, &kit.slab, &kit.dark, Vec3::new(0.24, 1.0, 0.28)),
        (Segment::Knee, &kit.ball, &kit.steel, Vec3::splat(0.24)),
        (Segment::Shin, &kit.slab, &kit.paint, Vec3::new(0.22, 1.0, 0.26)),
        (Segment::Foot, &kit.slab, &kit.dark, Vec3::new(0.4, 0.14, 0.7))
      ] {
        root.spawn((
          Limb { mech: entity, side, segment },
          Mesh3d(mesh.clone()),
          MeshMaterial3d(material.clone()),
          Transform::from_scale(size)
        ));
      }
    }
  });
  commands.entity(entity).insert(Strut {
    last: placed.translation,
    phase: 0.0,
    amp: 0.0,
    heading: Vec3::NEG_Z,
    speed: 0.0
  });
}

fn ridden(pilot: &Option<Res<Pilot>>, entity: Entity) -> Option<(Vec3, f32)> {
  pilot.as_ref().and_then(|pilot| {
    pilot
      .riding
      .filter(|riding| riding.craft == entity && riding.kind == Craft::Mech)
      .map(|_| (pilot.at - Vec3::Y * SEAT, pilot.yaw))
  })
}

fn moor(
  time: Res<Time>,
  kit: Res<Kit>,
  pilot: Option<Res<Pilot>>,
  mut commands: Commands,
  mut mechs: Query<
    (Entity, &Mech, Option<&mut Transform>, Option<&Shown>),
    Without<Torso>
  >,
  torsos: Query<(Entity, &Torso, Option<&Children>)>,
  fittings: Query<(), With<Fittings>>,
  mut beams: Query<(&Beam, &mut SpotLight)>,
  mut flames: Query<(&Flame, &mut Visibility)>
) {
  let blend = (time.delta_secs() * 12.0).min(1.0);
  for (entity, mech, transform, shown) in mechs.iter_mut() {
    let mine = ridden(&pilot, entity);
    let (at, yaw) = mine.unwrap_or((mech.at, mech.yaw));
    let wanted =
      Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw));
    match transform {
      Some(mut transform) if mine.is_some() => *transform = wanted,
      Some(mut transform) => {
        transform.translation = transform.translation.lerp(wanted.translation, blend);
        transform.rotation = transform.rotation.slerp(wanted.rotation, blend)
      }
      None => assemble(&kit, &mut commands, entity, wanted)
    }
    if shown.is_none_or(|shown| shown.0 != mech.modules)
      && let Some((torso, _, children)) =
        torsos.iter().find(|(_, torso, _)| torso.0 == entity)
    {
      for child in
        children.into_iter().flatten().filter(|child| fittings.contains(**child))
      {
        commands.entity(*child).despawn()
      }
      commands.entity(entity).insert(Shown(mech.modules.clone()));
      commands.entity(torso).with_children(|torso| {
        torso
          .spawn((Fittings, Transform::default(), Visibility::default()))
          .with_children(|fitted| outfit(&kit, entity, mech, fitted));
      });
    }
  }
  for (beam, mut light) in beams.iter_mut() {
    let lit = mechs.get(beam.0).is_ok_and(|(_, mech, ..)| mech.lit);
    light.intensity = if lit { 9.0e6 } else { 0.0 }
  }
  for (flame, mut visibility) in flames.iter_mut() {
    let burning = mechs.get(flame.0).is_ok_and(|(_, mech, ..)| mech.jets);
    visibility.set_if_neq(if burning {
      Visibility::Inherited
    } else {
      Visibility::Hidden
    });
  }
}

fn knee(hip: Vec3, foot: Vec3, ahead: Vec3) -> Vec3 {
  let span = foot - hip;
  let reach = span.length().clamp(0.05, (THIGH + SHIN) * 0.999);
  let along = span.normalize_or(Vec3::NEG_Y);
  let out = (ahead - along * along.dot(ahead)).normalize_or(ahead);
  let toward = (THIGH * THIGH - SHIN * SHIN + reach * reach) / (2.0 * reach);
  hip + along * toward + out * (THIGH * THIGH - toward * toward).max(0.0).sqrt()
}

fn feet(strut: &Strut) -> [(Vec3, f32); 2] {
  [0, 1].map(|side| {
    let swing = strut.phase + side as f32 * PI;
    let lifted = swing.sin().max(0.0) * strut.amp;
    let x = if side == 0 { -STANCE } else { STANCE };
    (
      Vec3::new(x, ANKLE + lifted * LIFT, 0.0)
        - strut.heading * swing.cos() * STRIDE * strut.amp,
      lifted
    )
  })
}

fn bob(strut: &Strut) -> f32 { -0.06 * strut.amp * strut.phase.sin().abs() }

fn strut(
  time: Res<Time>,
  mut mechs: Query<(&Transform, &mut Strut), With<Mech>>,
  mut sounds: MessageWriter<Sound>
) {
  let dt = time.delta_secs().max(1e-4);
  for (transform, mut strut) in mechs.iter_mut() {
    let moved = (transform.translation - strut.last).with_y(0.0);
    strut.last = transform.translation;
    let local = transform.rotation.inverse() * moved;
    let pace = (local.length() / dt).min(12.0);
    strut.speed += (pace - strut.speed) * (dt * 8.0).min(1.0);
    if local.length() > 1e-4 {
      strut.heading = local.normalize()
    }
    let wanted = (strut.speed / 1.2).clamp(0.0, 1.0);
    strut.amp += (wanted - strut.amp) * (dt * 6.0).min(1.0);
    let before = (strut.phase / PI).floor();
    strut.phase =
      (strut.phase + local.length() * PI / (2.0 * STRIDE)).rem_euclid(TAU * 64.0);
    let landed = (strut.phase / PI).floor() != before;
    if landed && strut.amp > 0.3 {
      let side = ((strut.phase / PI).floor() as i32).rem_euclid(2) as usize;
      let (foot, _) = feet(&strut)[1 - side];
      sounds.write(Sound::at(Cue::Stomp, transform.transform_point(foot)));
    }
  }
}

fn pose(
  time: Res<Time>,
  mechs: Query<&Strut, With<Mech>>,
  mut limbs: Query<(&Limb, &mut Transform), (Without<Torso>, Without<Drone>)>,
  mut torsos: Query<(&Torso, &mut Transform), (Without<Limb>, Without<Drone>)>,
  mut drones: Query<(&Drone, &mut Transform), (Without<Limb>, Without<Torso>)>
) {
  for (torso, mut transform) in torsos.iter_mut() {
    if let Ok(strut) = mechs.get(torso.0) {
      transform.translation = Vec3::Y * bob(strut)
    }
  }
  for (limb, mut transform) in limbs.iter_mut() {
    if let Ok(strut) = mechs.get(limb.mech) {
      let (foot, lifted) = feet(strut)[limb.side];
      let side = if limb.side == 0 { -1.0 } else { 1.0 };
      let hip = HIP * Vec3::new(side, 1.0, 1.0) + Vec3::Y * bob(strut);
      let bend = knee(hip, foot, Vec3::NEG_Z);
      let scale = transform.scale;
      *transform = match limb.segment {
        Segment::Thigh => spanning(hip, bend, 1.0).with_scale(scale.with_y(THIGH)),
        Segment::Knee => Transform::from_translation(bend).with_scale(scale),
        Segment::Shin => spanning(bend, foot, 1.0).with_scale(scale.with_y(SHIN)),
        Segment::Foot => Transform::from_translation(foot - Vec3::Y * (ANKLE - 0.07))
          .with_rotation(Quat::from_rotation_x(lifted * 0.35))
          .with_scale(scale)
      }
    }
  }
  let now = time.elapsed_secs();
  for (drone, mut transform) in drones.iter_mut() {
    let spin = now * 1.4 + drone.0.index_u32() as f32;
    transform.translation = Vec3::new(
      spin.cos() * 1.2,
      ROOF + 0.45 + (now * 3.0).sin() * 0.08,
      spin.sin() * 1.2
    );
    transform.rotation = Quat::from_rotation_y(-spin)
  }
}

fn embark(
  pilot: Option<ResMut<Pilot>>,
  mechs: Query<(Entity, &Mech)>,
  mut seated: Local<Option<Entity>>
) {
  if let Some(mut pilot) = pilot {
    let me = pilot.me;
    let now =
      mechs.iter().find(|(_, mech)| mech.rider == Some(me)).map(|(entity, _)| entity);
    match (now, *seated) {
      (Some(craft), before) if before != Some(craft) => {
        if let Ok((_, mech)) = mechs.get(craft) {
          pilot.riding = Some(Riding {
            craft,
            kind: Craft::Mech,
            yaw: mech.yaw,
            speed: 0.0,
            heat: 0.0
          });
          pilot.at = mech.at + Vec3::Y * SEAT;
          pilot.velocity = Vec3::ZERO;
          pilot.grounded = false
        }
      }
      (None, Some(before))
        if pilot.riding.is_some_and(|riding| riding.craft == before) =>
      {
        pilot.riding = None
      }
      _ => ()
    }
    *seated = now
  }
}

#[derive(Resource)]
pub struct Floodlit(pub bool);

fn alight(voxels: &Voxels, mech: &Mech, yaw: f32) -> Vec3 {
  let right = Quat::from_rotation_y(yaw) * Vec3::X;
  [right * 1.4, -right * 1.4, Vec3::Z * 0.0]
    .into_iter()
    .map(|offset| match offset == Vec3::ZERO {
      true => mech.at + Vec3::Y * (BULK.tall + 0.05),
      false => mech.at + offset + Vec3::Y * 0.1
    })
    .find(|&spot| {
      let (low, high) = Bulk::PERSON.body(spot);
      cells(low, high).all(|cell| !voxels.solid(cell))
    })
    .unwrap_or(mech.at + Vec3::Y * (BULK.tall + 0.05))
}

fn walk(
  time: Res<Time>,
  keys: Res<ButtonInput<KeyCode>>,
  menu: Res<Menu>,
  voxels: Option<Res<Voxels>>,
  mechs: Query<&Mech>,
  mut pilot: ResMut<Pilot>,
  mut lamp: ResMut<Floodlit>,
  bodies: Res<crate::bump::Bodies>,
  (mut leaves, mut strides, mut shots, mut sounds): (
    MessageWriter<Disembark>,
    MessageWriter<Stride>,
    MessageWriter<Shoot>,
    MessageWriter<Sound>
  ),
  mut since: Local<f32>,
  mut spare: Local<f32>,
  mut cooling: Local<f32>
) {
  if let Some(voxels) = voxels
    && let Some(mut riding) = pilot.riding
    && riding.kind == Craft::Mech
    && let Ok(mech) = mechs.get(riding.craft)
    && ready_around(&voxels, pilot.at)
  {
    let idle = menu.idle();
    let pressed = |key: KeyCode| idle && keys.pressed(key);
    let tapped = |key: KeyCode| idle && keys.just_pressed(key);
    let held = |key: KeyCode| f32::from(u8::from(pressed(key)));
    *cooling -= time.delta_secs();
    if tapped(KeyCode::KeyL) {
      lamp.0 = !lamp.0
    }
    let toward = pilot.facing() * Vec3::NEG_Z;
    if pressed(KeyCode::KeyF) && mech.has(Block::MechLaser) && *cooling <= 0.0 {
      *cooling = LASER_EVERY;
      shots.write(Shoot { weapon: Weapon::Laser, toward });
      sounds.write(Sound::here(Cue::Zap));
    }
    if tapped(KeyCode::KeyX) && mech.has(Block::MissileRack) {
      shots.write(Shoot { weapon: Weapon::Missile, toward });
      sounds.write(Sound::here(Cue::Cast));
    }
    match tapped(KeyCode::ShiftLeft) {
      true => {
        leaves.write(Disembark);
        pilot.riding = None;
        pilot.at = alight(
          &voxels,
          &Mech { at: pilot.at - Vec3::Y * SEAT, ..mech.clone() },
          pilot.yaw
        );
        pilot.velocity = Vec3::ZERO;
        pilot.lift = 0.0
      }
      false => {
        let level = Quat::from_rotation_y(pilot.yaw);
        let wish = (level
          * Vec3::new(
            held(KeyCode::KeyD) - held(KeyCode::KeyA),
            0.0,
            held(KeyCode::KeyS) - held(KeyCode::KeyW)
          ))
        .normalize_or_zero();
        let jets =
          pressed(KeyCode::Space) && mech.has(Block::Jumpjets) && mech.charge > 0.0;
        let hopping = pressed(KeyCode::Space) && !mech.has(Block::Jumpjets);
        let banked = (*spare + time.delta_secs()).min(0.25);
        let steps = (banked / STEP).floor();
        *spare = banked - steps * STEP;
        for _ in 0..steps as u32 {
          let base = pilot.at - Vec3::Y * SEAT;
          let wading = wet(&voxels, base + Vec3::Y * 1.2);
          let footed = pilot.grounded;
          let pace = mech.speed() * if wading { WADE } else { 1.0 };
          let blend = match (footed, wading) {
            (true, _) => 0.25,
            (false, true) => 0.08,
            (false, false) => 0.04
          };
          let flat = pilot.velocity.xz().lerp((wish * pace).xz(), blend);
          let rise = pilot.velocity.y;
          let rise = match () {
            () if jets => (rise + (THRUST - GRAVITY) * STEP).min(CLIMB),
            () if hopping && footed => HOP,
            () if wading => (rise - SINKING * STEP).max(-3.0),
            () => (rise - GRAVITY * STEP).max(-40.0)
          };
          let mut velocity = Vec3::new(flat.x, rise, flat.y);
          let (mut at, hit) = slide(&voxels, BULK, base, 1, velocity.y * STEP);
          let landed = hit && velocity.y < 0.0;
          if hit {
            velocity.y = 0.0
          }
          for axis in [0, 2] {
            let (moved, blocked) = slide(&voxels, BULK, at, axis, velocity[axis] * STEP);
            let climbed = (blocked && (landed || footed))
              .then(|| {
                let (up, roofed) = slide(&voxels, BULK, at, 1, STEP_UP);
                let (over, stuck) = slide(&voxels, BULK, up, axis, velocity[axis] * STEP);
                (!roofed && !stuck).then_some(over)
              })
              .flatten();
            match climbed {
              Some(over) => {
                at = over;
                pilot.lift -= STEP_UP
              }
              None => {
                at = moved;
                if blocked {
                  velocity[axis] = 0.0
                }
              }
            }
          }
          let (at, decked) = bodies.0.iter().filter(|body| !body.soft).fold(
            (at, false),
            |(at, decked), body| match body
              .deck_under(at, BULK)
              .filter(|_| velocity.y <= 0.0)
            {
              Some(top) => (at.with_y(top), true),
              None => (
                crate::bump::pushed(&voxels, BULK, at, body.shove(at, BULK, STEP)),
                decked
              )
            }
          );
          if decked {
            velocity.y = 0.0
          }
          pilot.at = at + Vec3::Y * SEAT;
          pilot.velocity = velocity;
          pilot.grounded =
            landed || decked || (footed && slide(&voxels, BULK, at, 1, -0.05).1);
          pilot.lift *= 1.0 - STEP * 10.0
        }
        riding.yaw = pilot.yaw;
        pilot.riding = Some(riding);
        *since += time.delta_secs();
        if *since >= STRIDE_EVERY {
          *since = 0.0;
          strides.write(Stride {
            at: pilot.at - Vec3::Y * SEAT,
            yaw: pilot.yaw,
            jets,
            lit: lamp.0,
            striding: wish != Vec3::ZERO
          });
        }
      }
    }
  }
}

fn handle(
  buttons: Res<ButtonInput<MouseButton>>,
  keys: Res<ButtonInput<KeyCode>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  pilot: Res<Pilot>,
  selected: Res<Selected>,
  inventories: Query<&Inventory>,
  voxels: Option<Res<Voxels>>,
  mechs: Query<(Entity, &Mech)>,
  (mut deploys, mut boards, mut packs, mut refits, mut unfits): (
    MessageWriter<Deploy>,
    MessageWriter<Board>,
    MessageWriter<Pack>,
    MessageWriter<Refit>,
    MessageWriter<Unfit>
  )
) {
  if let Some(voxels) = voxels
    && captured(&cursor, &menu)
  {
    let (from, toward) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
    let wall = voxels.cast(from, toward, REACH);
    let blocked =
      wall.as_ref().map_or(REACH, |hit| (hit.at.as_vec3() + 0.5).distance(from));
    let riding =
      pilot.riding.filter(|riding| riding.kind == Craft::Mech).map(|riding| riding.craft);
    let holding = inventories
      .get(pilot.me)
      .ok()
      .and_then(|inventory| inventory.slots[selected.0])
      .map(|stack| stack.block);
    let target = mechs
      .iter()
      .filter(|&(entity, _)| riding.is_none_or(|craft| craft != entity))
      .filter_map(|(entity, mech)| {
        struck(mech, from, toward).map(|near| (entity, mech, near))
      })
      .filter(|&(.., near)| near <= REACH.min(blocked + 0.5))
      .min_by(|a, b| a.2.total_cmp(&b.2));
    let feeds = holding.is_some_and(|held| {
      fitting(held)
        || matches!(held, Block::IronOre | Block::AetherCell)
        || crate::balloon::burns(held).is_some()
    });
    let shifted = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if let Some(own) = riding
      && feeds
      && menu.idle()
      && keys.just_pressed(KeyCode::KeyR)
    {
      refits.write(Refit(own));
    }
    match (
      target,
      buttons.just_pressed(MouseButton::Right),
      buttons.just_pressed(MouseButton::Left)
    ) {
      (Some((mech, ..)), true, _) if shifted => {
        unfits.write(Unfit(mech));
      }
      (Some((mech, ..)), true, _) if feeds => {
        refits.write(Refit(mech));
      }
      (Some((mech, found, _)), true, _) if riding.is_none() && found.rider.is_none() => {
        boards.write(Board(mech));
      }
      (Some((mech, found, _)), _, true) if riding.is_none() && found.rider.is_none() => {
        packs.write(Pack(mech));
      }
      (None, true, _) if riding.is_none() && holding == Some(Block::Mech) => {
        if let Some(hit) = wall {
          deploys.write(Deploy(hit.at + hit.normal));
        }
      }
      _ => ()
    }
  }
}

#[derive(Component)]
struct Gauge;

fn panel(mut commands: Commands) {
  commands.spawn((
    Gauge,
    Text::new(""),
    TextFont { font_size: FontSize::Px(15.0), ..default() },
    crate::hud::SHADE,
    Node {
      position_type: PositionType::Absolute,
      right: px(12),
      bottom: px(96),
      ..default()
    },
    Visibility::Hidden
  ));
}

fn read(
  pilot: Res<Pilot>,
  lamp: Res<Floodlit>,
  mechs: Query<&Mech>,
  mut gauges: Query<(&mut Text, &mut Visibility), With<Gauge>>
) {
  let riding = pilot
    .riding
    .filter(|riding| riding.kind == Craft::Mech)
    .and_then(|riding| mechs.get(riding.craft).ok());
  for (mut text, mut visibility) in gauges.iter_mut() {
    visibility.set_if_neq(if riding.is_some() {
      Visibility::Inherited
    } else {
      Visibility::Hidden
    });
    if let Some(mech) = riding {
      let rated =
        mech.rated().map_or("any depth".into(), |rated| format!("{rated:.0} m"));
      let modules: Vec<&str> = mech.modules.iter().map(|module| module.name()).collect();
      let keys: Vec<&str> = [
        (true, "WASD walk"),
        (true, if mech.has(Block::Jumpjets) { "Space jets" } else { "Space hop" }),
        (mech.drill().is_some(), "hold LMB drill"),
        (mech.has(Block::MechLaser), "F laser"),
        (mech.has(Block::MissileRack), "X missile"),
        (mech.has(Block::Floodlight), "L light"),
        (true, "R refit"),
        (true, "Shift leave")
      ]
      .into_iter()
      .filter_map(|(shown, key)| shown.then_some(key))
      .collect();
      let reading = format!(
        "Frame {:.0}%   Charge {:.0}/{:.0} s{}   Depth {:.0} m (rated {rated}){}\nHardpoints {}/{HARDPOINTS}: {}\n{}",
        mech.hull / HULL * 100.0,
        mech.charge,
        mech.capacity(),
        if mech.sealed() { "   Sealed" } else { "   UNSEALED: no power" },
        mech.depth().max(0.0),
        match (mech.has(Block::Floodlight), lamp.0) {
          (true, true) => "   Light on",
          (true, false) => "   Light off",
          _ => ""
        },
        mech.modules.len(),
        if modules.is_empty() { "empty".into() } else { modules.join(", ") },
        keys.join(", ")
      );
      text.set_if_neq(Text::new(reading));
    }
  }
}

pub struct Mechs;

impl Plugin for Mechs {
  fn build(&self, app: &mut App) {
    app
      .insert_resource(Floodlit(true))
      .add_systems(Startup, (build, panel).run_if(plays))
      .add_systems(
        PreUpdate,
        (serve, unseat)
          .chain()
          .after(ServerSystems::Receive)
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
      )
      .add_systems(
        Update,
        (endure, volley, muster).run_if(authority).run_if(resource_exists::<Voxels>)
      )
      .add_systems(
        Update,
        (embark, walk, handle, moor, strut, pose, read)
          .chain()
          .run_if(plays)
          .run_if(resource_exists::<Pilot>)
      );
  }
}
