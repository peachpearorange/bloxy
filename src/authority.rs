use {crate::{block::Block,
             generate,
             opts::opts,
             protocol::*,
             save::{self, Roster, World},
             voxels::Voxels},
     bevy::prelude::*,
     bevy_replicon::prelude::*};

#[derive(Component)]
struct Controller(ClientId);

const TINTS: [[f32; 3]; 8] = [
  [0.85, 0.3, 0.25],
  [0.25, 0.5, 0.9],
  [0.3, 0.75, 0.35],
  [0.9, 0.75, 0.2],
  [0.65, 0.35, 0.8],
  [0.2, 0.75, 0.75],
  [0.9, 0.5, 0.2],
  [0.85, 0.85, 0.85]
];

fn starter(creative: bool) -> Inventory {
  let kit = [
    Block::Planks,
    Block::Cobblestone,
    Block::Glass,
    Block::Lamp,
    Block::Bricks,
    Block::Log,
    Block::Dirt,
    Block::Sand,
    Block::Stone
  ];
  Inventory {
    slots: match creative {
      true => kit.map(|block| Some(Stack { block, count: STACK })),
      false => [None; HOTBAR]
    }
  }
}

fn embody(
  commands: &mut Commands,
  roster: &mut Roster,
  seed: u32,
  client: ClientId,
  name: String,
  count: usize
) {
  let (avatar, inventory) = roster.0.remove(&name).unwrap_or_else(|| {
    (
      Avatar { at: generate::spawn_point(seed), yaw: 0.0, pitch: 0.0 },
      starter(opts().creative)
    )
  });
  let player = commands
    .spawn((
      Replicated,
      Controller(client),
      Player { name, tint: TINTS[count % TINTS.len()] },
      avatar,
      inventory
    ))
    .id();
  commands.write_message(ToClients {
    targets: SendTargets::Single(client),
    message: Possess(player)
  });
}

fn found_world(mut commands: Commands) {
  let World { seed, edits, players } = save::load().unwrap_or_else(|| World {
    seed: opts().seed,
    edits: default(),
    players: default()
  });
  let mut voxels = Voxels::new(seed);
  edits.iter().for_each(|&(at, block)| voxels.set(at, block));
  voxels.ensure(generate::spawn_point(seed).floor().as_ivec3());
  commands.insert_resource(voxels);
  commands.insert_resource(Roster(players))
}

fn embody_host(
  mut commands: Commands,
  mut roster: ResMut<Roster>,
  voxels: Res<Voxels>,
  role: Res<Role>
) {
  if role.plays() {
    embody(&mut commands, &mut roster, voxels.seed, ClientId::Server, opts().called(), 0)
  }
}

fn welcome(
  joined: On<Add, AuthorizedClient>,
  mut commands: Commands,
  voxels: Res<Voxels>
) {
  commands.write_message(ToClients {
    targets: SendTargets::Single(ClientId::Client(joined.entity)),
    message: Welcome { seed: voxels.seed, edits: voxels.all_edits() }
  });
}

fn introduce(
  mut hellos: MessageReader<FromClient<Hello>>,
  mut commands: Commands,
  mut roster: ResMut<Roster>,
  voxels: Res<Voxels>,
  players: Query<(&Controller, &Player)>
) {
  let mut present: Vec<(ClientId, String)> = players
    .iter()
    .map(|(controller, player)| (controller.0, player.name.clone()))
    .collect();
  hellos.read().for_each(|hello| {
    let given: String =
      hello.message.name.chars().filter(|c| !c.is_control()).take(24).collect();
    let given = match given.trim() {
      "" => "Wanderer".to_string(),
      trimmed => trimmed.to_string()
    };
    let taken = |name: &String| present.iter().any(|(_, other)| other == name);
    let name = (1..)
      .map(|n| match n {
        1 => given.clone(),
        n => format!("{given} {n}")
      })
      .find(|name| !taken(name))
      .unwrap_or(given);
    if !present.iter().any(|(client, _)| *client == hello.client_id) {
      embody(
        &mut commands,
        &mut roster,
        voxels.seed,
        hello.client_id,
        name.clone(),
        present.len()
      );
      present.push((hello.client_id, name))
    }
  })
}

fn farewell(
  left: On<Remove, ConnectedClient>,
  mut commands: Commands,
  mut roster: ResMut<Roster>,
  players: Query<(Entity, &Controller, &Player, &Avatar, &Inventory)>
) {
  players
    .iter()
    .filter(|(_, controller, ..)| controller.0 == ClientId::Client(left.entity))
    .for_each(|(entity, _, player, &avatar, inventory)| {
      roster.0.insert(player.name.clone(), (avatar, inventory.clone()));
      commands.entity(entity).despawn()
    })
}

fn player_of<'a, T>(
  players: impl IntoIterator<Item = (&'a Controller, T)>,
  client: ClientId
) -> Option<T> {
  players
    .into_iter()
    .find(|(controller, _)| controller.0 == client)
    .map(|(_, found)| found)
}

fn follow(
  mut moves: MessageReader<FromClient<Moved>>,
  mut players: Query<(&Controller, &mut Avatar)>
) {
  moves.read().for_each(|moved| {
    if let Some(mut avatar) = player_of(players.iter_mut(), moved.client_id)
      && moved.message.0.at.is_finite()
    {
      avatar.set_if_neq(moved.message.0);
    }
  })
}

fn within_reach(avatar: &Avatar, at: IVec3) -> bool {
  (avatar.at + Vec3::Y * EYE).distance(at.as_vec3() + Vec3::splat(0.5)) <= REACH + 2.0
}

fn dig(
  mut digs: MessageReader<FromClient<Dig>>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  mut changes: MessageWriter<ToClients<Altered>>
) {
  digs.read().for_each(|&FromClient { client_id, message: Dig { at } }| {
    let block = voxels.ensure(at);
    let granted =
      player_of(players.iter_mut(), client_id).is_some_and(|(avatar, mut inventory)| {
        block.breakable() && within_reach(avatar, at) && {
          inventory.add(block.drop());
          true
        }
      });
    match granted {
      true => {
        voxels.set(at, Block::Air);
        changes.write(ToClients {
          targets: SendTargets::All,
          message: Altered { at, block: Block::Air }
        })
      }
      false => changes.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Altered { at, block }
      })
    };
  })
}

fn put(
  mut puts: MessageReader<FromClient<Put>>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  everyone: Query<&Avatar>,
  mut changes: MessageWriter<ToClients<Altered>>
) {
  puts.read().for_each(|&FromClient { client_id, message: Put { at, block } }| {
    let present = voxels.ensure(at);
    let cell = (at.as_vec3(), at.as_vec3() + Vec3::ONE);
    let crowded = everyone.iter().any(|avatar| {
      let (low, high) =
        (avatar.at - Vec3::new(0.3, 0.0, 0.3), avatar.at + Vec3::new(0.3, 1.8, 0.3));
      low.cmplt(cell.1).all() && high.cmpgt(cell.0).all()
    });
    let granted = !crowded
      && block.solid()
      && !present.solid()
      && player_of(players.iter_mut(), client_id).is_some_and(
        |(avatar, mut inventory)| within_reach(avatar, at) && inventory.take(block)
      );
    match granted {
      true => {
        voxels.set(at, block);
        changes
          .write(ToClients { targets: SendTargets::All, message: Altered { at, block } })
      }
      false => changes.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Altered { at, block: present }
      })
    };
  })
}

pub struct Authority;

impl Plugin for Authority {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, (found_world, embody_host).chain().run_if(authority))
      .add_systems(
        PreUpdate,
        (introduce, follow, dig, put).after(ServerSystems::Receive).run_if(authority)
      )
      .add_observer(welcome)
      .add_observer(farewell);
  }
}
