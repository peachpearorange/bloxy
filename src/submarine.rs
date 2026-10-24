use {crate::{audio::{Cue, Sound},
             authority::{Controller, player_of, within_reach},
             block::Block,
             death::Dying,
             dungeon::{Dungeon, Style},
             folk::{Vigour, hurt},
             haunt::Blamed,
             island::SEA,
             menu::Menu,
             player::{Bulk, Craft, Pilot, Riding, Selected, captured, slide},
             protocol::*,
             spell::{Bolt, Flourish},
             stream::ready_around,
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            ecs::entity::MapEntities,
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize},
     std::f32::consts::{PI, TAU}};

pub const BAYS: usize = 5;
const HULL: f32 = 100.0;
const BULK: Bulk = Bulk { half: 0.9, tall: 1.8 };
const KEEL: f32 = 0.75;
const MIDDLE: f32 = 1.65;
const RADIUS: f32 = 0.9;
const BUBBLE: f32 = 0.25;
const STERN: f32 = 2.85;
const SPEED: f32 = 5.0;
const CHARGE: f32 = 900.0;
const RATED: f32 = 26.0;
const PLATED: f32 = 24.0;
const CRUSH: f32 = 4.0;
const MEND: f32 = 15.0;
const SPARK: f32 = 4.0;
const DRAW_IDLE: f32 = 0.08;
const DRAW_THRUST: f32 = 1.0;
const DRAW_LIGHT: f32 = 0.15;
const GRAVITY: f32 = 9.8;
const STEP: f32 = 1.0 / 60.0;
const DIVE_EVERY: f32 = 0.05;
const TORPEDO_SPEED: f32 = 18.0;
const TORPEDO_LIFE: f32 = 3.0;
const TORPEDO_BLAST: f32 = 3.0;
const TORPEDO_BLOW: u8 = 14;
const RELOAD: f32 = 1.2;
const PING_EVERY: f32 = 2.5;
const SCOPE: u32 = 128;
const BLOWN: f32 = 4.0;

#[derive(Clone, Copy, PartialEq)]
pub struct Drill {
  pub speed: f32,
  pub reach: f32,
  pub wide: bool
}

#[derive(Component, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Sub {
  pub at: Vec3,
  pub yaw: f32,
  pub pitch: f32,
  pub hull: f32,
  pub charge: f32,
  pub modules: Vec<Block>,
  pub lit: bool,
  pub thrust: bool,
  #[entities]
  pub rider: Option<Entity>
}

impl Sub {
  pub fn launched(at: Vec3, yaw: f32) -> Sub {
    Sub {
      at,
      yaw,
      pitch: 0.0,
      hull: HULL,
      charge: CHARGE * 0.5,
      modules: vec![],
      lit: false,
      thrust: false,
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

  pub fn depth(&self) -> f32 { SEA as f32 - (self.at.y + MIDDLE) }

  pub fn capacity(&self) -> f32 {
    CHARGE
      * (1.0
        + self.modules.iter().filter(|&&module| module == Block::PowerCell).count()
          as f32)
  }

  pub fn speed(&self) -> f32 {
    let boost = match (self.has(Block::Hydrojet), self.has(Block::Thruster)) {
      (true, _) => 2.1,
      (_, true) => 1.45,
      _ => 1.0
    };
    let starved = if self.charge > 0.0 { 1.0 } else { 0.25 };
    SPEED * boost * starved
  }

  pub fn drill(&self) -> Option<Drill> {
    match (self.has(Block::PlasmaDrill), self.has(Block::DrillModule)) {
      (true, _) => Some(Drill { speed: 7.0, reach: 8.0, wide: true }),
      (_, true) => Some(Drill { speed: 2.6, reach: 6.0, wide: false }),
      _ => None
    }
  }

  pub fn sonar(&self) -> Option<f32> {
    match (self.has(Block::DeepSonar), self.has(Block::SonarModule)) {
      (true, _) => Some(160.0),
      (_, true) => Some(64.0),
      _ => None
    }
  }
}

fn facing(yaw: f32, pitch: f32) -> Quat {
  Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)
}

fn wet(voxels: &Voxels, at: Vec3) -> bool {
  voxels.block(at.floor().as_ivec3()).is_some_and(Block::wet)
}

pub fn struck(sub: &Sub, from: Vec3, toward: Vec3) -> Option<f32> {
  let turn = Quat::from_rotation_y(-sub.yaw);
  crate::boat::pierce(
    Vec3::new(-1.0, KEEL - 0.1, BUBBLE - RADIUS - 0.1),
    Vec3::new(1.0, MIDDLE + RADIUS + 0.7, STERN + RADIUS + 0.4),
    turn * (from - sub.at),
    turn * toward
  )
}

pub fn shield(subs: &mut Query<&mut Sub>, player: Entity, blow: u8) -> bool {
  subs
    .iter_mut()
    .find(|sub| sub.rider == Some(player))
    .map(|mut sub| sub.hull = (sub.hull - f32::from(blow) * BLOWN).max(0.0))
    .is_some()
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Submerge(pub IVec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Hatch(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Dock(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Fit(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Strip(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Dive {
  pub at: Vec3,
  pub yaw: f32,
  pub pitch: f32,
  pub thrust: bool,
  pub lit: bool
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Fire(pub Vec3);

#[derive(Component)]
struct Torpedo {
  velocity: Vec3,
  life: f32,
  owner: Entity
}

#[derive(Component, Default)]
struct Reload(f32);

fn fits(sub: &Sub, module: Block) -> bool {
  module.module()
    && sub.modules.len() < BAYS
    && (module == Block::PowerCell || !sub.has(module))
}

fn serve(
  (
    mut submerges,
    mut hatches,
    mut docks,
    mut fits_,
    mut strips,
    mut leaves,
    mut dives,
    mut fires
  ): (
    MessageReader<FromClient<Submerge>>,
    MessageReader<FromClient<Hatch>>,
    MessageReader<FromClient<Dock>>,
    MessageReader<FromClient<Fit>>,
    MessageReader<FromClient<Strip>>,
    MessageReader<FromClient<Disembark>>,
    MessageReader<FromClient<Dive>>,
    MessageReader<FromClient<Fire>>
  ),
  mut voxels: ResMut<Voxels>,
  mut commands: Commands,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Inventory))>,
  mut subs: Query<(Entity, &mut Sub, &mut Reload)>,
  mut notices: MessageWriter<ToClients<Notice>>
) {
  for &FromClient { client_id, message: Submerge(cell) } in submerges.read() {
    if voxels.ensure(cell).wet()
      && let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, cell)
      && inventory.take(Block::Submarine)
    {
      spawn(
        &mut commands,
        Sub::launched(cell.as_vec3() + Vec3::new(0.5, 0.5 - MIDDLE, 0.5), avatar.yaw)
      );
    }
  }
  for &FromClient { client_id, message: Hatch(sub) } in hatches.read() {
    if let Some((player, avatar, _)) = player_of(players.iter_mut(), client_id)
      && subs.iter().all(|(_, sub, _)| sub.rider != Some(player))
      && let Ok((_, mut sub, _)) = subs.get_mut(sub)
      && sub.rider.is_none()
      && within_reach(avatar, (sub.at + Vec3::Y * MIDDLE).floor().as_ivec3())
    {
      sub.rider = Some(player)
    }
  }
  for &FromClient { client_id, message: Dock(entity) } in docks.read() {
    if let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Ok((_, sub, _)) = subs.get(entity)
      && sub.rider.is_none()
      && within_reach(avatar, (sub.at + Vec3::Y * MIDDLE).floor().as_ivec3())
      && inventory.add(Block::Submarine)
    {
      for &module in sub.modules.iter() {
        if !inventory.add(module) {
          crate::loose::scatter(
            &mut commands,
            module,
            1,
            avatar.at + Vec3::Y,
            Vec3::Y * 2.0,
            0.5
          )
        }
      }
      commands.entity(entity).despawn()
    }
  }
  for &FromClient { client_id, message: Fit(entity) } in fits_.read() {
    if let Some((player, avatar, mut inventory)) =
      player_of(players.iter_mut(), client_id)
      && let Ok((_, mut sub, _)) = subs.get_mut(entity)
      && (sub.rider == Some(player)
        || within_reach(avatar, (sub.at + Vec3::Y * MIDDLE).floor().as_ivec3()))
      && let Some(stack) = inventory.slots[usize::from(avatar.hand)]
    {
      let held = stack.block;
      let fuel = crate::balloon::burns(held).map(|seconds| seconds * SPARK);
      let done = match () {
        () if fits(&sub, held) && inventory.take(held) => {
          sub.modules.push(held);
          Some(format!("Fitted the {}", held.name()))
        }
        () if held.module() => Some(match sub.modules.len() >= BAYS {
          true => format!("All {BAYS} bays are full; Shift+right-click to take one out"),
          false => format!("A {} is already fitted", held.name())
        }),
        () if held == Block::IronOre && sub.hull < HULL && inventory.take(held) => {
          sub.hull = (sub.hull + MEND).min(HULL);
          Some(format!("Hull patched: {:.0}%", sub.hull))
        }
        () if held == Block::AetherCell && inventory.take(held) => {
          sub.charge = sub.capacity();
          Some("Aether cell slotted: fully charged".into())
        }
        () if let Some(spark) = fuel
          && sub.charge < sub.capacity()
          && inventory.take(held) =>
        {
          sub.charge = (sub.charge + spark).min(sub.capacity());
          Some(format!("Charge {:.0} s", sub.charge))
        }
        () => None
      };
      if let Some(word) = done {
        notices.write(ToClients {
          targets: SendTargets::Single(client_id),
          message: Notice(word)
        });
      }
    }
  }
  for &FromClient { client_id, message: Strip(entity) } in strips.read() {
    if let Some((_, avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Ok((_, mut sub, _)) = subs.get_mut(entity)
      && within_reach(avatar, (sub.at + Vec3::Y * MIDDLE).floor().as_ivec3())
      && let Some(&module) = sub.modules.last()
      && inventory.add(module)
    {
      sub.modules.pop();
      sub.charge = sub.charge.min(sub.capacity())
    }
  }
  for &FromClient { client_id, .. } in leaves.read() {
    if let Some((player, ..)) = player_of(players.iter_mut(), client_id) {
      for (_, mut sub, _) in
        subs.iter_mut().filter(|(_, sub, _)| sub.rider == Some(player))
      {
        sub.rider = None;
        sub.thrust = false
      }
    }
  }
  for &FromClient { client_id, message: Dive { at, yaw, pitch, thrust, lit } } in
    dives.read()
  {
    if let Some((player, ..)) = player_of(players.iter_mut(), client_id)
      && at.is_finite()
      && yaw.is_finite()
      && pitch.is_finite()
    {
      for (_, mut sub, _) in
        subs.iter_mut().filter(|(_, sub, _)| sub.rider == Some(player))
      {
        let lit = lit && sub.has(Block::Floodlight);
        let moved = Sub { at, yaw, pitch, thrust, lit, ..sub.clone() };
        sub.set_if_neq(moved);
      }
    }
  }
  for &FromClient { client_id, message: Fire(toward) } in fires.read() {
    if let Some((player, _, mut inventory)) = player_of(players.iter_mut(), client_id)
      && toward.is_finite()
      && let Some((_, sub, mut reload)) =
        subs.iter_mut().find(|(_, sub, _)| sub.rider == Some(player))
      && sub.has(Block::TorpedoBay)
      && reload.0 <= 0.0
      && inventory.take(Block::Torpedo)
    {
      reload.0 = RELOAD;
      let toward = toward.normalize_or(Vec3::NEG_Z);
      let at = sub.at + Vec3::Y * (MIDDLE - 0.3) + toward * 1.8;
      commands.spawn((Replicated, Bolt { spell: Block::SpellFireball, at }, Torpedo {
        velocity: toward * TORPEDO_SPEED,
        life: TORPEDO_LIFE,
        owner: player
      }));
    }
  }
}

pub fn spawn(commands: &mut Commands, sub: Sub) {
  commands.spawn((Replicated, sub, Reload::default()));
}

fn strain(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut subs: Query<(Entity, &mut Sub, &mut Reload)>,
  players: Query<&Controller>,
  mut notices: MessageWriter<ToClients<Notice>>,
  mut warned: Local<f32>,
  mut commands: Commands
) {
  let dt = time.delta_secs();
  *warned -= dt;
  for (entity, mut sub, mut reload) in subs.iter_mut() {
    reload.0 -= dt;
    let crushed = sub.rated().is_some_and(|rated| sub.depth() > rated);
    if sub.rider.is_some() {
      let draw = DRAW_IDLE
        + DRAW_THRUST * f32::from(u8::from(sub.thrust))
        + DRAW_LIGHT * f32::from(u8::from(sub.lit));
      let charge = (sub.charge - draw * dt).max(0.0);
      if (charge - sub.charge).abs() > 0.0 {
        sub.charge = charge
      }
    }
    if crushed {
      sub.hull = (sub.hull - CRUSH * dt).max(0.0);
      if let Some(rider) = sub.rider
        && let Ok(controller) = players.get(rider)
        && *warned <= 0.0
      {
        *warned = 3.0;
        notices.write(ToClients {
          targets: SendTargets::Single(controller.client),
          message: Notice(format!(
            "Hull groaning: rated to {:.0} m, at {:.0} m ({:.0}%)",
            sub.rated().unwrap_or_default(),
            sub.depth(),
            sub.hull
          ))
        });
      }
    }
    if sub.has(Block::ResonantPlating) && sub.hull < HULL {
      sub.hull = (sub.hull + dt * 0.5).min(HULL)
    }
    if sub.hull <= 0.0 {
      if let Some(rider) = sub.rider
        && let Ok(controller) = players.get(rider)
      {
        notices.write(ToClients {
          targets: SendTargets::Single(controller.client),
          message: Notice("The hull gave way! The submarine is wrecked".into())
        });
      }
      let at = sub.at + Vec3::Y * MIDDLE;
      for &module in sub.modules.iter() {
        crate::loose::scatter(&mut commands, module, 1, at, Vec3::Y * 3.0, 0.5)
      }
      crate::loose::scatter(&mut commands, Block::AlloyScrap, 2, at, Vec3::Y * 3.0, 0.5);
      commands.entity(entity).despawn()
    }
    if sub.rider.is_none() {
      voxels.ensure(sub.at.floor().as_ivec3());
    }
  }
}

fn unseat(players: Query<(), With<Controller>>, mut subs: Query<&mut Sub>) {
  for mut sub in subs.iter_mut() {
    if sub.rider.is_some_and(|rider| players.get(rider).is_err()) {
      sub.rider = None;
      sub.thrust = false
    }
  }
}

fn detonate(
  time: Res<Time>,
  voxels: Res<Voxels>,
  mut torpedoes: Query<(Entity, &mut Bolt, &mut Torpedo)>,
  mut victims: Query<
    (Entity, Option<&Folk>, Option<&Beast>, &mut Health),
    (Without<Controller>, Without<Dying>)
  >,
  mut flourishes: MessageWriter<ToClients<Flourish>>,
  mut commands: Commands
) {
  let dt = time.delta_secs().min(0.1);
  for (entity, mut bolt, mut torpedo) in torpedoes.iter_mut() {
    let (from, step) = (bolt.at, torpedo.velocity * dt);
    let (direction, length) = (step.normalize_or_zero(), step.length());
    let wall = voxels.cast_for(from, direction, length, Block::solid).is_some();
    let place = |folk: Option<&Folk>, beast: Option<&Beast>| {
      folk.map(|folk| folk.at).or(beast.map(|beast| beast.at))
    };
    let hit = victims.iter().any(|(_, folk, beast, health)| {
      health.0 > 0
        && place(folk, beast)
          .is_some_and(|at| (at + Vec3::Y * 0.8).distance(from + step) < 1.4)
    });
    torpedo.life -= dt;
    match wall || hit || torpedo.life <= 0.0 {
      true => {
        let at = from + step * 0.5;
        for (victim, folk, beast, mut health) in victims.iter_mut() {
          if let Some(spot) = place(folk, beast)
            && (spot + Vec3::Y * 0.8).distance(at) < TORPEDO_BLAST
            && health.0 > 0
          {
            health.0 = health.0.saturating_sub(TORPEDO_BLOW);
            commands.entity(victim).insert(Blamed(torpedo.owner));
            if health.0 == 0 {
              crate::haunt::fell(&mut commands, victim)
            }
          }
        }
        flourishes.write(ToClients {
          targets: SendTargets::All,
          message: Flourish { spell: Block::SpellFireball, at, reach: TORPEDO_BLAST }
        });
        commands.entity(entity).despawn()
      }
      false => bolt.at = from + step
    }
  }
}

#[derive(Component)]
pub struct Breath(pub f32);

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
pub struct Air(pub u8);

pub const LUNGS: f32 = 20.0;
const GASP_EVERY: f32 = 1.0;

fn breathe(
  time: Res<Time>,
  voxels: Res<Voxels>,
  subs: Query<&Sub>,
  mut players: Query<
    (
      Entity,
      &Avatar,
      &mut Health,
      &mut Vigour,
      Option<&mut Air>,
      Option<&mut Breath>,
      Option<&crate::skill::Skills>
    ),
    With<Controller>
  >,
  mut commands: Commands
) {
  let dt = time.delta_secs();
  for (player, avatar, mut health, mut vigour, air, breath, skills) in players.iter_mut()
  {
    let aboard = subs.iter().any(|sub| sub.rider == Some(player));
    let lungs = LUNGS * skills.map_or(1.0, |skills| skills.stats().swim);
    let under = !aboard
      && !crate::opts::opts().creative
      && wet(&voxels, avatar.at + Vec3::Y * (EYE - 0.15));
    match (breath, air) {
      (Some(mut breath), Some(mut air)) => {
        breath.0 = match under {
          true => breath.0 - dt,
          false => (breath.0 + dt * 5.0).min(lungs)
        };
        if breath.0 <= -GASP_EVERY {
          breath.0 = 0.0;
          if health.0 > 0 {
            hurt(&mut health, &mut vigour, 2)
          }
        }
        let shown = Air(breath.0.max(0.0).ceil() as u8);
        air.set_if_neq(shown);
      }
      _ => {
        commands.entity(player).insert((Breath(LUNGS), Air(LUNGS as u8)));
      }
    }
  }
}

#[derive(Resource)]
struct Kit {
  hull: Handle<Mesh>,
  liner: Handle<Mesh>,
  stern: Handle<Mesh>,
  bubble: Handle<Mesh>,
  slab: Handle<Mesh>,
  rib: Handle<Mesh>,
  frame: Handle<Mesh>,
  wheel: Handle<Mesh>,
  pipe: Handle<Mesh>,
  dial: Handle<Mesh>,
  grip: Handle<Mesh>,
  blade: Handle<Mesh>,
  lamp: Handle<Mesh>,
  cone: Handle<Mesh>,
  tube: Handle<Mesh>,
  ring: Handle<Mesh>,
  pod: Handle<Mesh>,
  paint: Handle<StandardMaterial>,
  plated: Handle<StandardMaterial>,
  steel: Handle<StandardMaterial>,
  cabin: Handle<StandardMaterial>,
  deck: Handle<StandardMaterial>,
  cushion: Handle<StandardMaterial>,
  glass: Handle<StandardMaterial>,
  glow: Handle<StandardMaterial>,
  amber: Handle<StandardMaterial>,
  screen: Handle<StandardMaterial>,
  beam: Handle<StandardMaterial>
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
  let inside = |r: f32, g: f32, b: f32| StandardMaterial {
    double_sided: true,
    cull_mode: None,
    ..colour(r, g, b, 0.7, 0.2)
  };
  let shining = |[r, g, b]: [f32; 3], strength: f32| StandardMaterial {
    base_color: Color::srgb(r, g, b),
    emissive: LinearRgba::rgb(r, g, b) * strength,
    unlit: true,
    ..default()
  };
  let length = STERN - BUBBLE;
  commands.insert_resource(Kit {
    hull: meshes.add(Cylinder::new(RADIUS, length).mesh().resolution(20)),
    liner: meshes.add(Cylinder::new(RADIUS - 0.06, length).mesh().resolution(20)),
    stern: meshes.add(Sphere::new(RADIUS).mesh().uv(20, 12)),
    bubble: meshes.add(Sphere::new(RADIUS + 0.02).mesh().uv(24, 14)),
    slab: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
    rib: meshes.add(Torus::new(RADIUS - 0.1, RADIUS - 0.04)),
    frame: meshes.add(Torus::new(RADIUS - 0.005, RADIUS + 0.025)),
    wheel: meshes.add(Torus::new(0.13, 0.17)),
    pipe: meshes.add(Cylinder::new(0.035, length - 0.2)),
    dial: meshes.add(Cylinder::new(0.028, 0.012)),
    grip: meshes.add(Cylinder::new(0.018, 0.18)),
    blade: meshes.add(Cuboid::new(0.12, 0.55, 0.04)),
    lamp: meshes.add(Cylinder::new(0.14, 0.25)),
    cone: meshes.add(Cone { radius: 0.3, height: 0.8 }),
    tube: meshes.add(Cylinder::new(0.13, 1.6)),
    ring: meshes.add(Torus::new(0.22, 0.3)),
    pod: meshes.add(Capsule3d::new(0.2, 0.9)),
    paint: materials.add(colour(0.95, 0.72, 0.12, 0.45, 0.2)),
    plated: materials.add(colour(0.42, 0.46, 0.5, 0.35, 0.8)),
    steel: materials.add(colour(0.3, 0.32, 0.35, 0.4, 0.85)),
    cabin: materials.add(inside(0.55, 0.58, 0.55)),
    deck: materials.add(colour(0.22, 0.2, 0.18, 0.8, 0.4)),
    cushion: materials.add(colour(0.45, 0.12, 0.1, 0.9, 0.0)),
    glass: materials.add(StandardMaterial {
      base_color: Color::srgba(0.6, 0.9, 1.0, 0.12),
      alpha_mode: AlphaMode::Blend,
      perceptual_roughness: 0.03,
      reflectance: 0.9,
      double_sided: true,
      cull_mode: None,
      ..default()
    }),
    glow: materials.add(shining([0.3, 0.95, 1.0], 6.0)),
    amber: materials.add(shining([1.0, 0.7, 0.2], 3.0)),
    screen: materials.add(shining([0.2, 0.9, 0.5], 2.0)),
    beam: materials.add(shining([1.0, 0.95, 0.8], 12.0))
  })
}

#[derive(Component)]
struct Shown(Vec<Block>);

#[derive(Component)]
struct Screw;

#[derive(Component)]
struct Beam;

#[derive(Component)]
struct Fittings;

fn spin(toward: Vec3) -> Quat { Quat::from_rotation_arc(Vec3::Y, toward) }

fn part(
  mesh: &Handle<Mesh>,
  material: &Handle<StandardMaterial>,
  transform: Transform
) -> impl Bundle {
  (Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), transform)
}

fn slab(
  kit: &Kit,
  material: &Handle<StandardMaterial>,
  middle: Vec3,
  size: Vec3
) -> impl Bundle {
  part(&kit.slab, material, Transform::from_translation(middle).with_scale(size))
}

fn hollow(kit: &Kit, ship: &mut ChildSpawnerCommands) {
  let along = (BUBBLE + STERN) / 2.0;
  let eye = Vec3::Y * EYE;
  ship.spawn(part(
    &kit.liner,
    &kit.cabin,
    Transform::from_xyz(0.0, MIDDLE, along).with_rotation(spin(Vec3::Z))
  ));
  ship.spawn(part(&kit.bubble, &kit.glass, Transform::from_xyz(0.0, MIDDLE, BUBBLE)));
  ship.spawn(part(
    &kit.frame,
    &kit.steel,
    Transform::from_xyz(0.0, MIDDLE, BUBBLE).with_rotation(spin(Vec3::Z))
  ));
  for turn in [-0.75f32, 0.75] {
    ship.spawn(part(
      &kit.frame,
      &kit.steel,
      Transform::from_xyz(0.0, MIDDLE, BUBBLE)
        .with_rotation(Quat::from_rotation_y(turn) * spin(Vec3::X))
    ));
  }
  for z in [0.9, 1.6, 2.3] {
    ship.spawn(part(
      &kit.rib,
      &kit.steel,
      Transform::from_xyz(0.0, MIDDLE, z).with_rotation(spin(Vec3::Z))
    ));
  }
  ship.spawn(slab(
    kit,
    &kit.deck,
    Vec3::new(0.0, KEEL + 0.22, 1.2),
    Vec3::new(1.2, 0.06, 3.0)
  ));
  ship.spawn(slab(
    kit,
    &kit.cabin,
    Vec3::new(0.0, MIDDLE, STERN - 0.2),
    Vec3::new(1.6, 1.6, 0.06)
  ));
  ship.spawn(part(
    &kit.wheel,
    &kit.steel,
    Transform::from_xyz(0.0, MIDDLE, STERN - 0.25).with_rotation(spin(Vec3::Z))
  ));
  ship.spawn(slab(
    kit,
    &kit.steel,
    Vec3::new(0.0, MIDDLE, STERN - 0.24),
    Vec3::new(0.3, 0.03, 0.02)
  ));
  for side in [-1.0f32, 1.0] {
    ship.spawn(part(
      &kit.pipe,
      &kit.steel,
      Transform::from_xyz(side * 0.4, MIDDLE + 0.68, along + 0.1)
        .with_rotation(spin(Vec3::Z))
    ));
    ship.spawn(slab(
      kit,
      &kit.steel,
      Vec3::new(side * 0.68, MIDDLE - 0.15, 1.7),
      Vec3::new(0.12, 0.7, 1.1)
    ));
    for (index, light) in [&kit.amber, &kit.glow, &kit.screen].into_iter().enumerate() {
      ship.spawn(slab(
        kit,
        light,
        Vec3::new(
          side * 0.61,
          MIDDLE + 0.05 - index as f32 * 0.12,
          1.4 + index as f32 * 0.15
        ),
        Vec3::new(0.02, 0.05, 0.08)
      ));
    }
    ship.spawn(part(
      &kit.grip,
      &kit.steel,
      Transform::from_translation(eye + Vec3::new(side * 0.22, -0.5, -0.42))
        .with_rotation(Quat::from_rotation_x(-0.4))
    ));
  }
  let console = Quat::from_rotation_x(0.5);
  ship.spawn((
    Mesh3d(kit.slab.clone()),
    MeshMaterial3d(kit.steel.clone()),
    Transform::from_translation(eye + Vec3::new(0.0, -0.62, -0.6))
      .with_rotation(console)
      .with_scale(Vec3::new(0.95, 0.05, 0.36))
  ));
  ship.spawn(slab(
    kit,
    &kit.deck,
    eye + Vec3::new(0.0, -0.88, -0.5),
    Vec3::new(1.0, 0.5, 0.3)
  ));
  ship.spawn((
    Mesh3d(kit.slab.clone()),
    MeshMaterial3d(kit.screen.clone()),
    Transform::from_translation(eye + Vec3::new(0.0, -0.585, -0.62))
      .with_rotation(console)
      .with_scale(Vec3::new(0.24, 0.01, 0.15))
  ));
  for (x, light) in
    [(-0.34, &kit.amber), (-0.24, &kit.glow), (0.24, &kit.amber), (0.34, &kit.glow)]
  {
    ship.spawn(part(
      &kit.dial,
      light,
      Transform::from_translation(eye + Vec3::new(x, -0.59, -0.58))
        .with_rotation(console)
    ));
  }
  ship.spawn(slab(
    kit,
    &kit.cushion,
    Vec3::new(0.0, KEEL + 0.5, 0.35),
    Vec3::new(0.55, 0.1, 0.5)
  ));
  ship.spawn(slab(
    kit,
    &kit.cushion,
    Vec3::new(0.0, KEEL + 0.95, 0.62),
    Vec3::new(0.55, 0.8, 0.1)
  ));
  ship.spawn(slab(
    kit,
    &kit.steel,
    Vec3::new(0.0, KEEL + 0.36, 0.35),
    Vec3::new(0.12, 0.3, 0.12)
  ));
  ship.spawn((
    PointLight {
      range: 4.0,
      intensity: crate::sky::torchlight().intensity,
      ..crate::sky::torchlight()
    },
    Transform::from_xyz(0.0, MIDDLE + 0.6, 1.3)
  ));
}

fn outfit(kit: &Kit, sub: &Sub, ship: &mut ChildSpawnerCommands) {
  let body = match sub.has(Block::HullPlating) || sub.has(Block::ResonantPlating) {
    true => &kit.plated,
    false => &kit.paint
  };
  let along = (BUBBLE + STERN) / 2.0;
  ship.spawn(part(
    &kit.hull,
    body,
    Transform::from_xyz(0.0, MIDDLE, along).with_rotation(spin(Vec3::Z))
  ));
  ship.spawn(part(&kit.stern, body, Transform::from_xyz(0.0, MIDDLE, STERN)));
  ship.spawn(slab(
    kit,
    body,
    Vec3::new(0.0, MIDDLE + RADIUS + 0.25, 1.4),
    Vec3::new(0.55, 0.6, 1.0)
  ));
  for (offset, turn) in
    [(Vec3::X, 0.0), (Vec3::NEG_X, 0.0), (Vec3::Y, PI / 2.0), (Vec3::NEG_Y, PI / 2.0)]
  {
    ship.spawn(part(
      &kit.slab,
      &kit.steel,
      Transform::from_translation(Vec3::new(0.0, MIDDLE, STERN + 0.4) + offset * 0.75)
        .with_rotation(Quat::from_rotation_z(turn))
        .with_scale(Vec3::new(0.08, 0.9, 0.6))
    ));
  }
  ship
    .spawn((
      Screw,
      Transform::from_xyz(0.0, MIDDLE, STERN + RADIUS + 0.1),
      Visibility::default()
    ))
    .with_children(|screw| {
      for blade in 0..4 {
        let turn = Quat::from_rotation_z(blade as f32 * TAU / 4.0);
        screw.spawn(part(
          &kit.blade,
          &kit.steel,
          Transform::from_rotation(turn).with_translation(turn * Vec3::Y * 0.25)
        ));
      }
    });
  for &module in sub.modules.iter() {
    match module {
      Block::Floodlight => {
        for side in [-1.0, 1.0] {
          ship.spawn(part(
            &kit.lamp,
            &kit.beam,
            Transform::from_xyz(side * 0.72, MIDDLE - 0.55, BUBBLE - 0.3)
              .with_rotation(spin(Vec3::NEG_Z))
          ));
        }
      }
      Block::DrillModule | Block::PlasmaDrill => {
        let tip = if module == Block::PlasmaDrill { &kit.glow } else { &kit.steel };
        ship.spawn(part(
          &kit.cone,
          tip,
          Transform::from_xyz(0.0, KEEL + 0.15, BUBBLE - 0.75)
            .with_rotation(spin(Vec3::NEG_Z))
        ));
      }
      Block::SonarModule | Block::DeepSonar => {
        let dish = if module == Block::DeepSonar { &kit.glow } else { &kit.steel };
        ship.spawn(part(
          &kit.ring,
          dish,
          Transform::from_xyz(0.0, MIDDLE + RADIUS + 0.65, 1.4)
        ));
      }
      Block::TorpedoBay => {
        for side in [-1.0, 1.0] {
          ship.spawn(part(
            &kit.tube,
            &kit.steel,
            Transform::from_xyz(side * 0.98, MIDDLE - 0.3, 1.4)
              .with_rotation(spin(Vec3::Z))
          ));
        }
      }
      Block::Thruster | Block::Hydrojet => {
        let jet = if module == Block::Hydrojet { &kit.glow } else { &kit.steel };
        for side in [-1.0, 1.0] {
          ship.spawn(part(
            &kit.pod,
            jet,
            Transform::from_xyz(side * 1.1, MIDDLE + 0.15, 2.4)
              .with_rotation(spin(Vec3::Z))
          ));
        }
      }
      Block::SampleClaw => {
        ship.spawn(part(
          &kit.tube,
          &kit.steel,
          Transform::from_xyz(0.45, KEEL, BUBBLE - 0.2)
            .with_rotation(Quat::from_rotation_x(1.1))
            .with_scale(Vec3::new(0.6, 0.7, 0.6))
        ));
      }
      _ => ()
    }
  }
}

fn moor(
  time: Res<Time>,
  kit: Res<Kit>,
  pilot: Option<Res<Pilot>>,
  mut commands: Commands,
  mut subs: Query<
    (Entity, &Sub, Option<&mut Transform>, Option<&Shown>, Option<&Children>),
    Without<Screw>
  >,
  fittings: Query<(), With<Fittings>>,
  mut screws: Query<&mut Transform, With<Screw>>,
  mut beams: Query<(&ChildOf, &mut SpotLight), With<Beam>>
) {
  let blend = (time.delta_secs() * 12.0).min(1.0);
  let mine = |entity: Entity| {
    pilot.as_ref().and_then(|pilot| {
      pilot
        .riding
        .filter(|riding| riding.craft == entity && riding.kind == Craft::Submarine)
        .map(|_| (pilot.at, pilot.yaw, pilot.pitch))
    })
  };
  for (entity, sub, transform, shown, children) in subs.iter_mut() {
    let ridden = mine(entity);
    let (at, yaw, pitch) = ridden.unwrap_or((sub.at, sub.yaw, sub.pitch));
    let tilt = Quat::from_euler(EulerRot::YXZ, yaw, pitch * 0.2, 0.0);
    let eye = Vec3::Y * EYE;
    let wanted = Transform::from_translation(at + eye - tilt * eye).with_rotation(tilt);
    match transform {
      Some(mut transform) if ridden.is_some() => *transform = wanted,
      Some(mut transform) => {
        transform.translation = transform.translation.lerp(wanted.translation, blend);
        transform.rotation = transform.rotation.slerp(wanted.rotation, blend)
      }
      None => {
        commands.entity(entity).insert((wanted, Visibility::default())).with_children(
          |ship| {
            ship.spawn((
              Beam,
              SpotLight {
                intensity: 0.0,
                range: 48.0,
                outer_angle: 0.62,
                inner_angle: 0.3,
                color: Color::srgb(1.0, 0.95, 0.85),
                shadow_maps_enabled: false,
                ..default()
              },
              Transform::from_xyz(0.0, MIDDLE - 0.55, BUBBLE - 0.5)
            ));
            hollow(&kit, ship)
          }
        );
      }
    }
    if shown.is_none_or(|shown| shown.0 != sub.modules) {
      for child in
        children.into_iter().flatten().filter(|child| fittings.contains(**child))
      {
        commands.entity(*child).despawn()
      }
      commands.entity(entity).insert(Shown(sub.modules.clone())).with_children(|ship| {
        ship
          .spawn((Fittings, Transform::default(), Visibility::default()))
          .with_children(|fitted| outfit(&kit, sub, fitted));
      });
    }
  }
  let turning = time.elapsed_secs() * 14.0;
  for mut screw in screws.iter_mut() {
    screw.rotation = Quat::from_rotation_z(turning)
  }
  for (&ChildOf(parent), mut light) in beams.iter_mut() {
    let lit = subs.get(parent).is_ok_and(|(_, sub, ..)| sub.lit);
    light.intensity = if lit { 9.0e6 } else { 0.0 }
  }
}

fn embark(
  pilot: Option<ResMut<Pilot>>,
  subs: Query<(Entity, &Sub)>,
  mut seated: Local<Option<Entity>>
) {
  if let Some(mut pilot) = pilot {
    let me = pilot.me;
    let now =
      subs.iter().find(|(_, sub)| sub.rider == Some(me)).map(|(entity, _)| entity);
    match (now, *seated) {
      (Some(craft), before) if before != Some(craft) => {
        if let Ok((_, sub)) = subs.get(craft) {
          pilot.riding = Some(Riding {
            craft,
            kind: Craft::Submarine,
            yaw: sub.yaw,
            speed: 0.0,
            heat: 0.0
          });
          pilot.at = sub.at;
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
pub struct Lamp(pub bool);

fn cruise(
  time: Res<Time>,
  keys: Res<ButtonInput<KeyCode>>,
  menu: Res<Menu>,
  voxels: Option<Res<Voxels>>,
  subs: Query<&Sub>,
  mut pilot: ResMut<Pilot>,
  mut lamp: ResMut<Lamp>,
  (mut leaves, mut dives, mut fires, mut sounds): (
    MessageWriter<Disembark>,
    MessageWriter<Dive>,
    MessageWriter<Fire>,
    MessageWriter<Sound>
  ),
  mut since: Local<f32>,
  mut spare: Local<f32>
) {
  if let Some(voxels) = voxels
    && let Some(mut riding) = pilot.riding
    && riding.kind == Craft::Submarine
    && let Ok(sub) = subs.get(riding.craft)
    && ready_around(&voxels, pilot.at)
  {
    let pressed = |key: KeyCode| menu.idle() && keys.pressed(key);
    let held = |key: KeyCode| f32::from(u8::from(pressed(key)));
    if menu.idle() && keys.just_pressed(KeyCode::KeyL) {
      lamp.0 = !lamp.0
    }
    if menu.idle() && keys.just_pressed(KeyCode::KeyF) && sub.has(Block::TorpedoBay) {
      fires.write(Fire(pilot.facing() * Vec3::NEG_Z));
      sounds.write(Sound::here(Cue::Cast));
    }
    match menu.idle() && keys.just_pressed(KeyCode::ShiftLeft) {
      true => {
        leaves.write(Disembark);
        pilot.riding = None;
        pilot.at += Vec3::Y * 0.4;
        pilot.velocity = Vec3::Y * 2.0
      }
      false => {
        let look = facing(pilot.yaw, pilot.pitch);
        let level = Quat::from_rotation_y(pilot.yaw);
        let wish = look * Vec3::new(0.0, 0.0, held(KeyCode::KeyS) - held(KeyCode::KeyW))
          + level * Vec3::X * (held(KeyCode::KeyD) - held(KeyCode::KeyA))
          + Vec3::Y * (held(KeyCode::Space) - held(KeyCode::KeyC));
        let thrust = wish.length_squared() > 0.01;
        let banked = (*spare + time.delta_secs()).min(0.25);
        let steps = (banked / STEP).floor();
        *spare = banked - steps * STEP;
        for _ in 0..steps as u32 {
          let middle = pilot.at + Vec3::Y * MIDDLE;
          let afloat = wet(&voxels, middle);
          let breaching = !wet(&voxels, middle + Vec3::Y * 0.7);
          let target = wish.normalize_or_zero() * sub.speed();
          let mut velocity = match afloat {
            true => pilot.velocity.lerp(target, (STEP * 2.5).min(1.0)),
            false => pilot.velocity.with_y(pilot.velocity.y - GRAVITY * STEP) * 0.995
          };
          if afloat && breaching && velocity.y > 0.0 {
            velocity.y = 0.0
          }
          let at = [1, 0, 2].into_iter().fold(pilot.at, |at, axis| {
            let keel = Vec3::Y * KEEL;
            let (moved, hit) =
              slide(&voxels, BULK, at + keel, axis, velocity[axis] * STEP);
            if hit {
              velocity[axis] = 0.0
            }
            moved - keel
          });
          pilot.at = at;
          pilot.velocity = velocity
        }
        riding.yaw = pilot.yaw;
        pilot.riding = Some(riding);
        *since += time.delta_secs();
        if *since >= DIVE_EVERY {
          *since = 0.0;
          dives.write(Dive {
            at: pilot.at,
            yaw: pilot.yaw,
            pitch: pilot.pitch,
            thrust,
            lit: lamp.0
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
  subs: Query<(Entity, &Sub)>,
  (mut submerges, mut hatches, mut docks, mut fits_, mut strips): (
    MessageWriter<Submerge>,
    MessageWriter<Hatch>,
    MessageWriter<Dock>,
    MessageWriter<Fit>,
    MessageWriter<Strip>
  )
) {
  if let Some(voxels) = voxels
    && captured(&cursor, &menu)
  {
    let (from, toward) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
    let blocked = voxels
      .cast(from, toward, REACH)
      .map_or(REACH, |hit| (hit.at.as_vec3() + 0.5).distance(from));
    let riding = pilot
      .riding
      .filter(|riding| riding.kind == Craft::Submarine)
      .map(|riding| riding.craft);
    let holding = inventories
      .get(pilot.me)
      .ok()
      .and_then(|inventory| inventory.slots[selected.0])
      .map(|stack| stack.block);
    let target = subs
      .iter()
      .filter(|&(entity, _)| riding.is_none_or(|craft| craft != entity))
      .filter_map(|(entity, sub)| {
        struck(sub, from, toward).map(|near| (entity, sub, near))
      })
      .filter(|&(.., near)| near <= REACH.min(blocked + 0.5))
      .min_by(|a, b| a.2.total_cmp(&b.2));
    let feeds = holding.is_some_and(|held| {
      held.module()
        || matches!(held, Block::IronOre | Block::AetherCell)
        || crate::balloon::burns(held).is_some()
    });
    let shifted = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    if let Some(own) = riding
      && feeds
      && keys.just_pressed(KeyCode::KeyR)
    {
      fits_.write(Fit(own));
    }
    match (
      target,
      buttons.just_pressed(MouseButton::Right),
      buttons.just_pressed(MouseButton::Left)
    ) {
      (Some((sub, ..)), true, _) if shifted => {
        strips.write(Strip(sub));
      }
      (Some((sub, ..)), true, _) if feeds => {
        fits_.write(Fit(sub));
      }
      (Some((sub, found, _)), true, _) if riding.is_none() && found.rider.is_none() => {
        hatches.write(Hatch(sub));
      }
      (Some((sub, found, _)), _, true) if riding.is_none() && found.rider.is_none() => {
        docks.write(Dock(sub));
      }
      (None, true, _) if riding.is_none() && holding == Some(Block::Submarine) => {
        let water = (1..=(REACH * 10.0) as i32)
          .map(|step| (from + toward * step as f32 * 0.1).floor().as_ivec3())
          .take_while(|&cell| !voxels.solid(cell))
          .find(|&cell| voxels.block(cell).is_some_and(Block::wet));
        if let Some(cell) = water {
          submerges.write(Submerge(cell));
        }
      }
      _ => ()
    }
  }
}

#[derive(Clone, Copy, PartialEq)]
enum Contact {
  Creature,
  Hostile,
  Life,
  Ore,
  Ruin
}

impl Contact {
  fn colour(self) -> [u8; 3] {
    match self {
      Contact::Creature => [235, 235, 120],
      Contact::Hostile => [255, 70, 60],
      Contact::Life => [90, 255, 200],
      Contact::Ore => [255, 190, 60],
      Contact::Ruin => [255, 80, 255]
    }
  }
}

#[derive(Resource, Default)]
struct Sweep {
  since: f32,
  range: f32,
  centre: Vec3,
  terrain: Vec<u8>,
  contacts: Vec<(Vec3, Contact)>,
  signals: Vec<(Vec3, &'static str)>
}

#[derive(Component)]
struct Scope;

#[derive(Component)]
struct Readout;

#[derive(Component)]
struct Gauge;

#[derive(Resource)]
struct Screen(Handle<Image>);

fn panel(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
  let image = images.add(Image::new_fill(
    Extent3d { width: SCOPE, height: SCOPE, depth_or_array_layers: 1 },
    TextureDimension::D2,
    &[0, 0, 0, 0],
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  ));
  commands.insert_resource(Screen(image.clone()));
  commands
    .spawn((
      Node {
        position_type: PositionType::Absolute,
        left: px(12),
        top: percent(24),
        flex_direction: FlexDirection::Column,
        row_gap: px(4),
        ..default()
      },
      Scope,
      Visibility::Hidden
    ))
    .with_children(|scope| {
      scope.spawn((ImageNode::new(image), Node {
        width: px(176),
        height: px(176),
        ..default()
      }));
      scope.spawn((
        Readout,
        Text::new(""),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(Color::srgb(0.6, 1.0, 0.85)),
        crate::hud::SHADE
      ));
    });
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

fn compass(offset: Vec3) -> &'static str {
  let turn = (offset.x.atan2(-offset.z) / TAU * 8.0).round().rem_euclid(8.0) as usize;
  ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][turn]
}

fn ping(
  time: Res<Time>,
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  subs: Query<&Sub>,
  beasts: Query<&Beast>,
  folk: Query<&Folk>,
  mut sweep: ResMut<Sweep>,
  mut sounds: MessageWriter<Sound>
) {
  sweep.since += time.delta_secs();
  if let Some(voxels) = voxels
    && let Some(riding) = pilot.riding.filter(|riding| riding.kind == Craft::Submarine)
    && let Ok(sub) = subs.get(riding.craft)
    && let Some(range) = sub.sonar()
    && sweep.since >= PING_EVERY
  {
    let voxels: &Voxels = &voxels;
    let deep = sub.has(Block::DeepSonar);
    let centre = pilot.at + Vec3::Y * MIDDLE;
    let level = centre.y.floor() as i32;
    let terrain: Vec<u8> = (0..SCOPE * SCOPE)
      .map(|index| {
        let (x, y) = (index % SCOPE, index / SCOPE);
        let offset = (Vec2::new(x as f32, y as f32) / SCOPE as f32 * 2.0 - 1.0) * range;
        let cell = (centre + Vec3::new(offset.x, 0.0, offset.y)).floor().as_ivec3();
        let solid = |rise: i32| voxels.block(cell.with_y(level + rise)).map(Block::solid);
        match (solid(0), solid(-3), solid(-8)) {
          (Some(true), ..) => 3,
          (_, Some(true), _) => 2,
          (.., Some(true)) => 1,
          _ => 0
        }
      })
      .collect();
    let reach = range * range;
    let step = if deep { 3 } else { 2 };
    let around = (range / step as f32) as i32;
    let found: Vec<(Vec3, Contact)> = (-around..=around)
      .flat_map(|x| (-around..=around).map(move |z| IVec2::new(x, z) * step))
      .filter(|column| (column.as_vec2().length_squared()) <= reach)
      .flat_map(|column| {
        (-14..=10).filter_map(move |rise| {
          let cell = centre.floor().as_ivec3() + IVec3::new(column.x, rise, column.y);
          voxels.block(cell).and_then(|block| {
            let ore = deep
              && matches!(
                block,
                Block::GoldOre
                  | Block::DiamondOre
                  | Block::PlatinumOre
                  | Block::NoduleOre
                  | Block::IronOre
              );
            match () {
              () if block.organism()
                && !matches!(
                  block,
                  Block::Kelp | Block::Seagrass | Block::BrainCoral
                )
                && cell.x.rem_euclid(3) == 0 =>
              {
                Some((cell.as_vec3() + 0.5, Contact::Life))
              }
              () if ore => Some((cell.as_vec3() + 0.5, Contact::Ore)),
              () => None
            }
          })
        })
      })
      .collect();
    let creatures = beasts
      .iter()
      .map(|beast| (beast.at, Contact::Creature))
      .chain(folk.iter().map(|folk| {
        (folk.at, if folk.band.hostile() { Contact::Hostile } else { Contact::Creature })
      }))
      .filter(|(at, _)| at.distance_squared(centre) <= reach);
    let listen = if deep { 600 } else { 220 };
    let spot = centre.xz().floor().as_ivec2();
    let ruins: Vec<Vec3> = Dungeon::within(voxels.seed, spot - listen, spot + listen)
      .iter()
      .filter(|dungeon| dungeon.style == Style::Precursor)
      .map(|dungeon| {
        ((dungeon.low + dungeon.high).as_vec3() / 2.0).with_y(dungeon.entry.y as f32)
      })
      .collect();
    let mut signals: Vec<(Vec3, &'static str)> =
      ruins.iter().map(|&at| (at, "Unknown signal")).collect();
    if let Some(&(at, _)) = found
      .iter()
      .filter(|(_, contact)| *contact == Contact::Ore)
      .min_by(|a, b| a.0.distance(centre).total_cmp(&b.0.distance(centre)))
    {
      signals.push((at, "Dense ore"))
    }
    signals.sort_by(|a, b| a.0.distance(centre).total_cmp(&b.0.distance(centre)));
    signals.truncate(3);
    *sweep = Sweep {
      since: 0.0,
      range,
      centre,
      terrain,
      contacts: found
        .into_iter()
        .chain(creatures)
        .chain(ruins.into_iter().map(|at| (at, Contact::Ruin)))
        .collect(),
      signals
    };
    sounds.write(Sound::here(Cue::Chime).louder(0.35));
  }
}

fn scope(
  pilot: Res<Pilot>,
  subs: Query<&Sub>,
  sweep: Res<Sweep>,
  screen: Res<Screen>,
  mut images: ResMut<Assets<Image>>,
  mut panels: Query<&mut Visibility, With<Scope>>,
  mut readouts: Query<&mut Text, With<Readout>>
) {
  let sonar = pilot
    .riding
    .filter(|riding| riding.kind == Craft::Submarine)
    .and_then(|riding| subs.get(riding.craft).ok())
    .and_then(Sub::sonar);
  for mut visibility in panels.iter_mut() {
    visibility.set_if_neq(match sonar.is_some() && !sweep.terrain.is_empty() {
      true => Visibility::Inherited,
      false => Visibility::Hidden
    });
  }
  if sonar.is_some()
    && !sweep.terrain.is_empty()
    && let Some(mut image) = images.get_mut(&screen.0)
    && let Some(data) = image.data.as_mut()
  {
    let half = SCOPE as f32 / 2.0;
    let sweep_angle = (sweep.since / PING_EVERY * TAU).rem_euclid(TAU);
    let turn = Quat::from_rotation_y(-pilot.yaw);
    for index in 0..(SCOPE * SCOPE) as usize {
      let (x, y) =
        ((index as u32 % SCOPE) as f32 + 0.5, (index as u32 / SCOPE) as f32 + 0.5);
      let offset = Vec2::new(x - half, y - half);
      let radius = offset.length() / half;
      let angle = offset.x.atan2(-offset.y).rem_euclid(TAU);
      let behind = (sweep_angle - angle).rem_euclid(TAU);
      let trail = (1.0 - behind / 1.2).max(0.0);
      let ring = ((radius * 4.0).fract() < 0.03) as u8 as f32;
      let rgba: [u8; 4] = match radius > 1.0 {
        true => [0, 0, 0, 0],
        false => {
          let world = turn.inverse()
            * Vec3::new(
              offset.x / half * sweep.range,
              0.0,
              offset.y / half * sweep.range
            );
          let sample = (sweep.centre.xz() - sweep.centre.xz() + world.xz()) / sweep.range;
          let (u, v) = (
            (sample.x * 0.5 + 0.5) * SCOPE as f32,
            (sample.y * 0.5 + 0.5) * SCOPE as f32
          );
          let land = sweep
            .terrain
            .get(
              v.clamp(0.0, SCOPE as f32 - 1.0) as usize * SCOPE as usize
                + u.clamp(0.0, SCOPE as f32 - 1.0) as usize
            )
            .copied()
            .unwrap_or(0);
          let green = 18.0 + f32::from(land) * 34.0 + trail * 70.0 + ring * 40.0;
          [6, green.min(255.0) as u8, (green * 0.6) as u8, 225]
        }
      };
      data[index * 4..index * 4 + 4].copy_from_slice(&rgba);
    }
    let mut dot = |at: Vec3, size: i32, [r, g, b]: [u8; 3]| {
      let local = turn * (at - pilot.at) / sweep.range;
      if local.xz().length() <= 1.0 {
        let (px, py) = ((local.x * half + half) as i32, (local.z * half + half) as i32);
        for dx in -size..=size {
          for dy in -size..=size {
            let (x, y) = (px + dx, py + dy);
            if (0..SCOPE as i32).contains(&x) && (0..SCOPE as i32).contains(&y) {
              let at = ((y as u32 * SCOPE + x as u32) * 4) as usize;
              data[at..at + 4].copy_from_slice(&[r, g, b, 255]);
            }
          }
        }
      }
    };
    for &(at, contact) in sweep.contacts.iter() {
      let size = match contact {
        Contact::Ruin => 3,
        Contact::Hostile | Contact::Creature => 1,
        Contact::Life | Contact::Ore => 0
      };
      dot(at, size, contact.colour())
    }
    dot(pilot.at, 1, [255, 255, 255]);
    let lines: Vec<String> = sweep
      .signals
      .iter()
      .map(|&(at, what)| {
        let offset = at - (pilot.at + Vec3::Y * MIDDLE);
        format!(
          "{what}: {:.0} m {}, {:.0} m {}",
          offset.xz().length(),
          compass(offset),
          offset.y.abs(),
          if offset.y < 0.0 { "below" } else { "above" }
        )
      })
      .collect();
    for mut text in readouts.iter_mut() {
      text.set_if_neq(Text::new(match lines.is_empty() {
        true => "No signals".into(),
        false => lines.join("\n")
      }));
    }
  }
}

fn read(
  pilot: Res<Pilot>,
  lamp: Res<Lamp>,
  subs: Query<&Sub>,
  airs: Query<&Air>,
  mut gauges: Query<(&mut Text, &mut Visibility), With<Gauge>>
) {
  let riding = pilot
    .riding
    .filter(|riding| riding.kind == Craft::Submarine)
    .and_then(|riding| subs.get(riding.craft).ok());
  let air = airs.get(pilot.me).map_or(LUNGS as u8, |air| air.0);
  for (mut text, mut visibility) in gauges.iter_mut() {
    let shown = riding.is_some() || air < LUNGS as u8;
    visibility.set_if_neq(if shown { Visibility::Inherited } else { Visibility::Hidden });
    let reading = match riding {
      Some(sub) => {
        let depth = SEA as f32 - (pilot.at.y + MIDDLE);
        let rated =
          sub.rated().map_or("any depth".into(), |rated| format!("{rated:.0} m"));
        let modules: Vec<&str> = sub.modules.iter().map(|module| module.name()).collect();
        format!(
          "Depth {:.0} m (rated {rated})   Hull {:.0}%   Charge {:.0}/{:.0} s{}\nBays {}/{BAYS}: {}\nWASD move, Space rise, C dive, R refit, Shift leave{}{}{}",
          depth.max(0.0),
          sub.hull,
          sub.charge,
          sub.capacity(),
          if sub.has(Block::Floodlight) {
            if lamp.0 { "   Light on" } else { "   Light off" }
          } else {
            ""
          },
          sub.modules.len(),
          if modules.is_empty() { "empty".into() } else { modules.join(", ") },
          if sub.has(Block::Floodlight) { ", L light" } else { "" },
          if sub.has(Block::TorpedoBay) { ", F torpedo" } else { "" },
          if sub.drill().is_some() { ", hold LMB to drill" } else { "" }
        )
      }
      None => format!("Air {air} s")
    };
    text.set_if_neq(Text::new(reading));
  }
}

fn dive(
  players: Query<(Entity, &Avatar), With<Controller>>,
  mut done: Local<bool>,
  mut commands: Commands
) {
  if crate::opts::opts().dive
    && !*done
    && let Some((player, avatar)) = players.iter().next()
  {
    *done = true;
    let at = crate::opts::opts().inside.map_or(avatar.at, Vec3::from);
    spawn(&mut commands, Sub {
      modules: vec![
        Block::DeepSonar,
        Block::PlasmaDrill,
        Block::Floodlight,
        Block::TorpedoBay,
        Block::ResonantPlating,
      ],
      charge: CHARGE,
      rider: Some(player),
      ..Sub::launched(at, avatar.yaw)
    })
  }
}

pub struct Submarines;

impl Plugin for Submarines {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Sweep>()
      .insert_resource(Lamp(true))
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
        (strain, detonate, breathe, dive)
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
      )
      .add_systems(
        Update,
        (embark, cruise, handle, moor, ping, scope, read)
          .chain()
          .run_if(plays)
          .run_if(resource_exists::<Pilot>)
      );
  }
}
