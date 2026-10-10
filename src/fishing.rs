use {crate::{authority::{Controller, player_of},
             block::{Block, Fluid},
             hand::RodTip,
             hud::icon_rect,
             island::{Island, Kind},
             menu::Menu,
             noise::{hash, unit},
             player::{Pilot, Selected, captured},
             protocol::*,
             stream::Palette,
             voxels::Voxels},
     bevy::{light::NotShadowCaster,
            prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

const CAST_REACH: f32 = 14.0;
const HOOK_WINDOW: f32 = 1.2;
const SNAP: f32 = 24.0;
const BAR: f32 = 300.0;

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Cast(pub IVec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Bite {
  pub catch: Block,
  pub after: f32
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Reel(pub bool);

#[derive(Clone, Copy)]
struct Temper {
  speed: f32,
  jumpiness: f32,
  zone: f32
}

impl Temper {
  fn of(catch: Block) -> Temper {
    match catch {
      Block::Cod => Temper { speed: 0.35, jumpiness: 0.5, zone: 0.34 },
      Block::Salmon => Temper { speed: 0.6, jumpiness: 0.9, zone: 0.28 },
      Block::TropicalFish => Temper { speed: 0.8, jumpiness: 1.4, zone: 0.26 },
      Block::Pufferfish => Temper { speed: 1.1, jumpiness: 2.2, zone: 0.22 },
      junk if !junk.fish() && junk.tool() => {
        Temper { speed: 0.15, jumpiness: 0.2, zone: 0.4 }
      }
      _ => Temper { speed: 0.45, jumpiness: 0.3, zone: 0.24 }
    }
  }
}

fn catch(seed: u32, at: IVec3, roll: f32, pick: f32) -> Block {
  let climate = Island::near(seed, at.as_vec3()).first().map(|island| island.kind);
  let open_sea = climate.is_none();
  let (cold, warm) = match climate {
    Some(Kind::Frost) => (0.35, 0.0),
    Some(Kind::Dunes | Kind::Volcano) => (0.0, 0.3),
    _ => (0.12, 0.08)
  };
  let treasure = if open_sea { 0.1 } else { 0.05 };
  let table = [
    (0.1, if pick < 0.5 { Block::Log } else { Block::Bucket }),
    (
      treasure,
      [Block::GoldOre, Block::DiamondOre, Block::CopperOre][(pick * 2.99) as usize]
    ),
    (0.07, Block::Pufferfish),
    (warm, Block::TropicalFish),
    (cold, Block::Salmon)
  ];
  table
    .into_iter()
    .scan(0.0, |below, (chance, block)| {
      *below += chance;
      Some((*below, block))
    })
    .find(|&(below, _)| roll < below)
    .map_or(Block::Cod, |(_, block)| block)
}

#[derive(Component)]
struct Line {
  catch: Block,
  due: f32
}

fn cast(
  mut casts: MessageReader<FromClient<Cast>>,
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  players: Query<(&Controller, (Entity, &Avatar, &Inventory))>,
  mut bites: MessageWriter<ToClients<Bite>>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  casts.read().for_each(|&FromClient { client_id, message: Cast(at) }| {
    if let Some((entity, avatar, inventory)) = player_of(players.iter(), client_id)
      && inventory.count(Block::FishingRod) > 0
      && voxels.ensure(at).liquid().is_some_and(|(fluid, _)| fluid == Fluid::Water)
      && (avatar.at + Vec3::Y * EYE).distance(at.as_vec3() + 0.5) <= CAST_REACH + 2.0
    {
      let luck = hash(voxels.seed ^ 0xF15, at.x, at.z, (now * 1000.0) as i32);
      let catch = catch(voxels.seed, at, unit(luck, 1, 0, 0), unit(luck, 2, 0, 0));
      let after = 2.0 + unit(luck, 3, 0, 0) * 6.0;
      commands.entity(entity).insert(Line { catch, due: now + after });
      bites.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Bite { catch, after }
      });
    }
  })
}

fn reel(
  mut reels: MessageReader<FromClient<Reel>>,
  time: Res<Time>,
  mut players: Query<(&Controller, (Entity, &mut Inventory, Option<&Line>))>,
  mut notices: MessageWriter<ToClients<Notice>>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  reels.read().for_each(|&FromClient { client_id, message: Reel(landed) }| {
    if let Some((entity, mut inventory, line)) = player_of(players.iter_mut(), client_id)
    {
      commands.entity(entity).remove::<Line>();
      let caught =
        line.filter(|line| landed && now >= line.due - 0.5).map(|line| line.catch);
      let word = match caught {
        Some(catch) if inventory.add(catch) => Some(format!("Caught: {}!", catch.name())),
        Some(_) => Some("Your pack is full".into()),
        None if landed || line.is_some_and(|line| now >= line.due) => {
          Some("It got away".into())
        }
        None => None
      };
      word.into_iter().for_each(|word| {
        notices.write(ToClients {
          targets: SendTargets::Single(client_id),
          message: Notice(word)
        });
      })
    }
  })
}

#[derive(Clone, Copy)]
enum Angling {
  Idle,
  Waiting { bobber: Vec3, since: f32, bite: Option<Bite> },
  Fighting(Fight)
}

#[derive(Clone, Copy)]
struct Fight {
  catch: Block,
  bobber: Vec3,
  zone: f32,
  lift: f32,
  fish: f32,
  goal: f32,
  progress: f32
}

#[derive(Resource)]
struct Rod {
  angling: Angling,
  luck: u32
}

impl Rod {
  fn dice(&mut self) -> f32 {
    self.luck = hash(self.luck, 0x71, 0x13, 0x5F);
    unit(self.luck, 0, 0, 0)
  }
}

#[derive(Component)]
struct Bobber;

#[derive(Component)]
struct Tether;

#[derive(Component)]
struct Panel;

#[derive(Component)]
struct Zone;

#[derive(Component)]
struct Quarry;

#[derive(Component)]
struct Gauge;

fn tackle(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  palette: Res<Palette>
) {
  let float = materials.add(StandardMaterial {
    base_color: Color::srgb(0.85, 0.12, 0.1),
    perceptual_roughness: 0.6,
    ..default()
  });
  let cap = materials.add(Color::srgb(0.95, 0.95, 0.92));
  commands.spawn((Bobber, Transform::default(), Visibility::Hidden)).with_children(
    |bobber| {
      bobber.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.14, 0.12, 0.14))),
        MeshMaterial3d(float),
        Transform::default()
      ));
      bobber.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.1, 0.06, 0.1))),
        MeshMaterial3d(cap),
        Transform::from_xyz(0.0, 0.09, 0.0)
      ));
    }
  );
  commands.spawn((
    Tether,
    NotShadowCaster,
    Mesh3d(meshes.add(Cuboid::new(0.012, 0.012, 1.0))),
    MeshMaterial3d(materials.add(StandardMaterial {
      base_color: Color::srgb(0.9, 0.9, 0.88),
      unlit: true,
      ..default()
    })),
    Transform::default(),
    Visibility::Hidden
  ));
  commands
    .spawn((
      Panel,
      Node {
        position_type: PositionType::Absolute,
        right: percent(28),
        top: percent(50),
        margin: UiRect::top(px(-BAR / 2.0)),
        column_gap: px(6),
        ..default()
      },
      Visibility::Hidden
    ))
    .with_children(|panel| {
      panel
        .spawn((
          Node {
            width: px(44),
            height: px(BAR),
            border: UiRect::all(px(3)),
            ..default()
          },
          BorderColor::all(Color::srgb(0.35, 0.24, 0.12)),
          BackgroundColor(Color::srgba(0.08, 0.2, 0.35, 0.85))
        ))
        .with_children(|bar| {
          bar.spawn((
            Zone,
            Node {
              position_type: PositionType::Absolute,
              left: px(0),
              width: percent(100),
              bottom: px(0),
              height: px(60),
              ..default()
            },
            BackgroundColor(Color::srgba(0.35, 0.95, 0.4, 0.55))
          ));
          bar.spawn((Quarry, ImageNode::new(palette.icons.clone()), Node {
            position_type: PositionType::Absolute,
            left: px(3),
            width: px(32),
            height: px(32),
            bottom: px(0),
            ..default()
          }));
        });
      panel
        .spawn((
          Node {
            width: px(12),
            height: px(BAR),
            align_items: AlignItems::FlexEnd,
            ..default()
          },
          BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.7))
        ))
        .with_children(|gauge| {
          gauge.spawn((
            Gauge,
            Node { width: percent(100), height: percent(30), ..default() },
            BackgroundColor(Color::srgb(0.95, 0.8, 0.2))
          ));
        });
    });
}

fn angle(
  time: Res<Time>,
  buttons: Res<ButtonInput<MouseButton>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  menu: Res<Menu>,
  pilot: Res<Pilot>,
  selected: Res<Selected>,
  inventories: Query<&Inventory>,
  voxels: Option<Res<Voxels>>,
  mut bites: MessageReader<Bite>,
  mut rod: ResMut<Rod>,
  (mut casts, mut reels, mut notices): (
    MessageWriter<Cast>,
    MessageWriter<Reel>,
    MessageWriter<Notice>
  )
) {
  let (now, dt) = (time.elapsed_secs(), time.delta_secs().min(0.1));
  let holding = inventories
    .get(pilot.me)
    .ok()
    .and_then(|inventory| inventory.slots[selected.0])
    .is_some_and(|stack| stack.block == Block::FishingRod);
  let active = captured(&cursor, &menu);
  let click = active && buttons.just_pressed(MouseButton::Right);
  let pull = active && buttons.pressed(MouseButton::Right);
  let arrived = bites.read().last().copied();
  let gone = |bobber: Vec3| !holding || pilot.eye().distance(bobber) > SNAP;
  rod.angling = match rod.angling {
    Angling::Idle if click && holding => voxels
      .as_deref()
      .and_then(|voxels| {
        voxels.cast_for(pilot.eye(), pilot.facing() * Vec3::NEG_Z, CAST_REACH, |block| {
          block.solid() || block.fluid()
        })
      })
      .filter(|hit| hit.block.liquid().is_some_and(|(fluid, _)| fluid == Fluid::Water))
      .map_or(Angling::Idle, |hit| {
        casts.write(Cast(hit.at));
        Angling::Waiting {
          bobber: hit.at.as_vec3() + Vec3::new(0.5, 0.9, 0.5),
          since: now,
          bite: None
        }
      }),
    Angling::Waiting { bobber, .. }
      if gone(bobber) || click && !bitten(&rod.angling, now) =>
    {
      reels.write(Reel(false));
      Angling::Idle
    }
    Angling::Waiting { since, bite: Some(bite), .. }
      if now > since + bite.after + HOOK_WINDOW =>
    {
      reels.write(Reel(false));
      Angling::Idle
    }
    Angling::Waiting { bobber, bite: Some(bite), .. } if click => {
      let start = rod.dice() * 0.6 + 0.2;
      Angling::Fighting(Fight {
        catch: bite.catch,
        bobber,
        zone: 0.0,
        lift: 0.0,
        fish: start,
        goal: start,
        progress: 0.3
      })
    }
    Angling::Waiting { bobber, since, bite } => {
      Angling::Waiting { bobber, since, bite: bite.or(arrived) }
    }
    Angling::Fighting(fight) if gone(fight.bobber) => {
      reels.write(Reel(false));
      Angling::Idle
    }
    Angling::Fighting(fight) => {
      let Temper { speed, jumpiness, zone: size } = Temper::of(fight.catch);
      let goal = match rod.dice() < jumpiness * dt {
        true => rod.dice(),
        false => fight.goal
      };
      let fish = fight.fish + (goal - fight.fish).clamp(-speed * dt, speed * dt);
      let lift = (fight.lift + if pull { 2.6 } else { -2.2 } * dt).clamp(-1.5, 1.5);
      let zone = (fight.zone + lift * dt).clamp(0.0, 1.0 - size);
      let lift = if zone <= 0.0 || zone >= 1.0 - size { lift * -0.3 } else { lift };
      let inside = (zone..zone + size).contains(&fish);
      let progress = fight.progress + if inside { 0.32 } else { -0.22 } * dt;
      match progress {
        done if done >= 1.0 => {
          reels.write(Reel(true));
          Angling::Idle
        }
        lost if lost <= 0.0 => {
          reels.write(Reel(false));
          Angling::Idle
        }
        _ => Angling::Fighting(Fight { zone, lift, fish, goal, progress, ..fight })
      }
    }
    idle => idle
  };
  if let Angling::Waiting { since, bite: Some(bite), .. } = rod.angling
    && now - dt <= since + bite.after
    && now > since + bite.after
  {
    notices.write(Notice("Something bites! Right-click".into()));
  }
}

fn bitten(angling: &Angling, now: f32) -> bool {
  match *angling {
    Angling::Waiting { since, bite: Some(bite), .. } => now >= since + bite.after,
    _ => false
  }
}

fn show(
  time: Res<Time>,
  rod: Res<Rod>,
  pilot: Res<Pilot>,
  tips: Query<&GlobalTransform, With<RodTip>>,
  mut bobbers: Query<(&mut Transform, &mut Visibility), (With<Bobber>, Without<Tether>)>,
  mut tethers: Query<(&mut Transform, &mut Visibility), (With<Tether>, Without<Bobber>)>,
  mut panels: Query<&mut Visibility, (With<Panel>, Without<Bobber>, Without<Tether>)>,
  mut zones: Query<&mut Node, (With<Zone>, Without<Quarry>, Without<Gauge>)>,
  mut quarries: Query<
    (&mut Node, &mut ImageNode),
    (With<Quarry>, Without<Zone>, Without<Gauge>)
  >,
  mut gauges: Query<&mut Node, (With<Gauge>, Without<Zone>, Without<Quarry>)>
) {
  let now = time.elapsed_secs();
  let bob = match rod.angling {
    Angling::Idle => None,
    Angling::Waiting { bobber, since, bite } => {
      let biting = bite.is_some_and(|bite| now >= since + bite.after);
      let dip = if biting { -0.18 + (now * 30.0).sin() * 0.05 } else { 0.0 };
      Some(bobber + Vec3::Y * ((now * 2.0).sin() * 0.03 + dip))
    }
    Angling::Fighting(fight) => Some(
      fight.bobber + Vec3::new((now * 7.0).sin() * 0.25, -0.15, (now * 5.0).cos() * 0.25)
    )
  };
  let tip = tips
    .iter()
    .next()
    .map_or(pilot.eye() + pilot.facing() * Vec3::new(0.35, 0.25, -1.2), |tip| {
      tip.translation()
    });
  bobbers.iter_mut().for_each(|(mut transform, mut visibility)| {
    *visibility = if bob.is_some() { Visibility::Visible } else { Visibility::Hidden };
    transform.translation = bob.unwrap_or_default()
  });
  tethers.iter_mut().for_each(|(mut transform, mut visibility)| {
    *visibility = if bob.is_some() { Visibility::Visible } else { Visibility::Hidden };
    if let Some(bob) = bob {
      let length = tip.distance(bob).max(0.01);
      *transform = Transform::from_translation((tip + bob) / 2.0)
        .looking_at(bob, Vec3::Y)
        .with_scale(Vec3::new(1.0, 1.0, length))
    }
  });
  let fight = match rod.angling {
    Angling::Fighting(fight) => Some(fight),
    _ => None
  };
  panels.iter_mut().for_each(|mut visibility| {
    *visibility = if fight.is_some() { Visibility::Visible } else { Visibility::Hidden }
  });
  if let Some(fight) = fight {
    let inner = BAR - 6.0;
    let Temper { zone: size, .. } = Temper::of(fight.catch);
    zones.iter_mut().for_each(|mut node| {
      node.bottom = px(fight.zone * inner);
      node.height = px(size * inner)
    });
    quarries.iter_mut().for_each(|(mut node, mut image)| {
      node.bottom = px(fight.fish * inner - 16.0);
      image.rect = Some(icon_rect(fight.catch))
    });
    gauges.iter_mut().for_each(|mut node| node.height = percent(fight.progress * 100.0))
  }
}

pub struct Fishing;

impl Plugin for Fishing {
  fn build(&self, app: &mut App) {
    app
      .add_client_message::<Cast>(Channel::Ordered)
      .add_client_message::<Reel>(Channel::Ordered)
      .add_server_message::<Bite>(Channel::Ordered)
      .insert_resource(Rod { angling: Angling::Idle, luck: 0x5EED })
      .add_systems(
        PreUpdate,
        (cast, reel).chain().after(ServerSystems::Receive).run_if(authority)
      )
      .add_systems(Startup, tackle.after(crate::stream::paint).run_if(plays))
      .add_systems(
        Update,
        (angle, show).chain().run_if(plays).run_if(resource_exists::<Pilot>)
      );
  }
}

#[cfg(test)]
mod tests {
  use {super::*, crate::island::SEA};

  #[test]
  fn catches_vary() {
    let catches: Vec<Block> = (0..200)
      .map(|index| catch(1, IVec3::new(0, SEA, 0), index as f32 / 200.0, 0.5))
      .collect();
    assert!(catches.contains(&Block::Cod));
    assert!(catches.contains(&Block::Pufferfish));
    assert!(catches.iter().any(|block| !block.fish()));
  }
}
