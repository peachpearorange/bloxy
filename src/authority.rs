use {crate::{account::{Account, Accounts, LONGEST_NAME, tidy},
             block::Block,
             generate,
             opts::opts,
             protocol::*,
             save::{self, World},
             skin::Skin,
             voxels::Voxels},
     bevy::prelude::*,
     bevy_replicon::prelude::*};

#[derive(Component)]
pub struct Controller {
  pub client: ClientId,
  pub account: usize
}

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
  accounts: &Accounts,
  seed: u32,
  client: ClientId,
  account: usize
) -> Entity {
  let Account { name, avatar, inventory, skin, .. } = accounts.0[account].clone();
  let at = generate::spawn_point(seed);
  let player = commands
    .spawn((
      Replicated,
      Controller { client, account },
      Player { name },
      avatar.unwrap_or(Avatar { at, yaw: 0.0, pitch: 0.0 }),
      inventory,
      skin
    ))
    .id();
  commands.write_message(ToClients {
    targets: SendTargets::Single(client),
    message: Possess(player)
  });
  player
}

fn found_world(mut commands: Commands) {
  let World { seed, edits, accounts } = save::load().unwrap_or_else(|| World {
    seed: opts().seed,
    edits: default(),
    accounts: default()
  });
  let mut voxels = Voxels::new(seed);
  edits.iter().for_each(|&(at, block)| voxels.set(at, block));
  voxels.ensure(generate::spawn_point(seed).floor().as_ivec3());
  commands.insert_resource(voxels);
  commands.insert_resource(Accounts(accounts))
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

fn sign_in(
  mut hellos: MessageReader<FromClient<Hello>>,
  mut commands: Commands,
  mut accounts: ResMut<Accounts>,
  voxels: Res<Voxels>,
  players: Query<(Entity, &Controller, &Avatar, &Inventory, &Skin)>,
  mut verdicts: MessageWriter<ToClients<Verdict>>
) {
  let mut online: Vec<(ClientId, usize, Entity)> = players
    .iter()
    .map(|(entity, controller, ..)| (controller.client, controller.account, entity))
    .collect();
  hellos.read().for_each(|FromClient { client_id, message: Hello { name, password } }| {
    let client = *client_id;
    let current = online
      .iter()
      .find(|(other, ..)| *other == client)
      .map(|&(_, id, entity)| (id, entity));
    let playing = online.iter().map(|&(_, id, _)| id).collect::<Vec<_>>();
    let answer = match tidy(name) {
      None => Err(format!("Names need 1 to {LONGEST_NAME} characters")),
      Some(name) => match (accounts.named(&name), current) {
        (owner, Some((mine, entity))) if owner.is_none_or(|owner| owner == mine) => {
          accounts.0[mine].name = name.clone();
          accounts.0[mine].lock(password);
          commands.entity(entity).insert(Player { name: name.clone() });
          Ok(name)
        }
        (Some(owner), _) if playing.contains(&owner) => {
          Err(format!("{} is already playing", accounts.0[owner].name))
        }
        (Some(owner), current) if accounts.0[owner].admits(password) => {
          if let Some((mine, entity)) = current
            && let Ok((_, _, avatar, inventory, skin)) = players.get(entity)
          {
            accounts.0[mine].keep(avatar, inventory, skin);
            commands.entity(entity).despawn();
            online.retain(|&(other, ..)| other != client)
          }
          let player = embody(&mut commands, &accounts, voxels.seed, client, owner);
          online.push((client, owner, player));
          Ok(accounts.0[owner].name.clone())
        }
        (Some(owner), _) => Err(format!("Wrong password for {}", accounts.0[owner].name)),
        (None, _) => {
          accounts.0.push(Account::open(
            name.clone(),
            password,
            starter(opts().creative)
          ));
          let owner = accounts.0.len() - 1;
          let player = embody(&mut commands, &accounts, voxels.seed, client, owner);
          online.push((client, owner, player));
          Ok(name)
        }
      }
    };
    verdicts.write(ToClients {
      targets: SendTargets::Single(client),
      message: match answer {
        Ok(name) => Verdict::Accepted { name },
        Err(reason) => Verdict::Refused { reason }
      }
    });
  })
}

fn paint(
  mut paints: MessageReader<FromClient<Paint>>,
  mut commands: Commands,
  players: Query<(Entity, &Controller)>
) {
  paints.read().filter(|paint| paint.message.0.valid()).for_each(|paint| {
    players
      .iter()
      .filter(|(_, controller)| controller.client == paint.client_id)
      .for_each(|(entity, _)| {
        commands.entity(entity).insert(paint.message.0.clone());
      })
  })
}

fn farewell(
  left: On<Remove, ConnectedClient>,
  mut commands: Commands,
  mut accounts: ResMut<Accounts>,
  players: Query<(Entity, &Controller, &Avatar, &Inventory, &Skin)>
) {
  players
    .iter()
    .filter(|(_, controller, ..)| controller.client == ClientId::Client(left.entity))
    .for_each(|(entity, controller, avatar, inventory, skin)| {
      accounts.0[controller.account].keep(avatar, inventory, skin);
      commands.entity(entity).despawn()
    })
}

fn player_of<'a, T>(
  players: impl IntoIterator<Item = (&'a Controller, T)>,
  client: ClientId
) -> Option<T> {
  players
    .into_iter()
    .find(|(controller, _)| controller.client == client)
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
      .add_systems(Startup, found_world.run_if(authority))
      .add_systems(
        PreUpdate,
        (sign_in, paint, follow, dig, put)
          .after(ServerSystems::Receive)
          .run_if(authority)
      )
      .add_observer(welcome)
      .add_observer(farewell);
  }
}
