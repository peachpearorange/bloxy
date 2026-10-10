use {crate::{account::{Account, Accounts, Kept, LONGEST_NAME, tidy},
             block::{Block, Fluid},
             flow::Flows,
             folk, generate,
             island::{FACING_STONE, Island},
             opts::opts,
             protocol::*,
             recipe::RECIPES,
             save::{self, World},
             shroomling, sign,
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
    Block::Torch,
    Block::CraftingTable,
    Block::Furnace,
    Block::Boat,
    Block::Bucket,
    Block::FishingRod,
    Block::Ladder,
    Block::Bed,
    Block::Wool,
    Block::OakSign,
    Block::Log,
    Block::Stone,
    Block::Lamp,
    Block::Bricks,
    Block::Dirt,
    Block::Sand,
    Block::Granite,
    Block::Diorite,
    Block::Andesite,
    Block::Limestone,
    Block::Slate,
    Block::BirchSign,
    Block::SpruceSign,
    Block::PalmSign
  ];
  let mut inventory = Inventory::default();
  if creative {
    kit
      .iter()
      .zip(&mut inventory.slots)
      .for_each(|(&block, slot)| *slot = Some(Stack { block, count: block.stack() }))
  }
  inventory
}

fn embody(
  commands: &mut Commands,
  accounts: &Accounts,
  seed: u32,
  client: ClientId,
  account: usize
) -> Entity {
  let Account { name, avatar, inventory, skin, visited, bookmarks, bedside, .. } =
    accounts.0[account].clone();
  let at = generate::spawn_point(seed);
  let player = commands
    .spawn((
      Replicated,
      Controller { client, account },
      Player { name },
      avatar.unwrap_or(Avatar { at, yaw: FACING_STONE, pitch: 0.0 }),
      inventory,
      skin,
      visited,
      bookmarks,
      bedside,
      Health(Health::FULL),
      folk::Vigour::default()
    ))
    .id();
  commands.write_message(ToClients {
    targets: SendTargets::Single(client),
    message: Possess(player)
  });
  player
}

fn found_world(mut commands: Commands) {
  let World { seed, edits, accounts, shroomlings, boats, signs } = save::load()
    .unwrap_or_else(|| World {
      seed: opts().seed,
      edits: default(),
      accounts: default(),
      shroomlings: default(),
      boats: default(),
      signs: default()
    });
  signs.into_iter().for_each(|sign| {
    commands.spawn((Replicated, sign));
  });
  boats.into_iter().for_each(|(at, yaw)| {
    commands.spawn((Replicated, Vessel { at, yaw, rider: None }));
  });
  let colonies = shroomlings.iter().map(|&(home, _)| home).collect();
  shroomlings.into_iter().enumerate().for_each(|(index, (home, hopper))| {
    shroomling::lodge(&mut commands, home, hopper, index as u32 * 0x9E37 + 1)
  });
  commands.insert_resource(shroomling::Colonies(colonies));
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
  players: Query<(Entity, &Controller, Kept)>,
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
            && let Ok((_, _, kept)) = players.get(entity)
          {
            accounts.0[mine].keep(kept);
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
  players: Query<(Entity, &Controller, Kept)>
) {
  players
    .iter()
    .filter(|(_, controller, _)| controller.client == ClientId::Client(left.entity))
    .for_each(|(entity, controller, kept)| {
      accounts.0[controller.account].keep(kept);
      commands.entity(entity).despawn()
    })
}

pub fn player_of<'a, T>(
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

pub fn within_reach(avatar: &Avatar, at: IVec3) -> bool {
  (avatar.at + Vec3::Y * EYE).distance(at.as_vec3() + Vec3::splat(0.5)) <= REACH + 2.0
}

fn dig(
  mut digs: MessageReader<FromClient<Dig>>,
  time: Res<Time>,
  mut flows: ResMut<Flows>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  mut changes: MessageWriter<ToClients<Altered>>
) {
  digs.read().for_each(|&FromClient { client_id, message: Dig { at } }| {
    let block = voxels.ensure(at);
    let above = at + IVec3::Y;
    let perched =
      Some(voxels.ensure(above)).filter(|block| block.modelled() && !block.ladder());
    let granted =
      player_of(players.iter_mut(), client_id).is_some_and(|(avatar, mut inventory)| {
        block.breakable() && within_reach(avatar, at) && {
          inventory.add(block.drop());
          perched.into_iter().for_each(|plant| {
            inventory.add(plant);
          });
          true
        }
      });
    match granted {
      true => {
        voxels.set(at, Block::Air);
        flows.stir(at, time.elapsed_secs() + Fluid::Water.delay());
        if perched.is_some() {
          voxels.set(above, Block::Air);
          changes.write(ToClients {
            targets: SendTargets::All,
            message: Altered { at: above, block: Block::Air }
          });
        }
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
  time: Res<Time>,
  mut flows: ResMut<Flows>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  everyone: Query<&Avatar>,
  mut changes: MessageWriter<ToClients<Altered>>,
  mut commands: Commands
) {
  puts.read().for_each(|&FromClient { client_id, message: Put { at, block } }| {
    let present = voxels.ensure(at);
    let cell = (at.as_vec3(), at.as_vec3() + Vec3::ONE);
    let crowded = everyone.iter().any(|avatar| {
      let (low, high) =
        (avatar.at - Vec3::new(0.3, 0.0, 0.3), avatar.at + Vec3::new(0.3, 1.8, 0.3));
      low.cmplt(cell.1).all() && high.cmpgt(cell.0).all()
    });
    let footed = match block.wall() {
      Some(wall) => voxels.ensure(at + wall).solid(),
      None => block.solid() || (block.modelled() && voxels.ensure(at - IVec3::Y).solid())
    };
    if block.held().placeable()
      && !(crowded && block.solid())
      && footed
      && !present.solid()
      && let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
      && inventory.take(block.held())
    {
      voxels.set(at, block);
      flows.stir(at, time.elapsed_secs() + Fluid::Water.delay());
      if block.sign() {
        commands.spawn((Replicated, Sign {
          at,
          yaw: sign::facing(avatar.at, at),
          wood: block,
          text: String::new()
        }));
      }
      changes
        .write(ToClients { targets: SendTargets::All, message: Altered { at, block } });
    } else {
      changes.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Altered { at, block: present }
      });
    }
  })
}

fn attune(
  mut touches: MessageReader<FromClient<Attune>>,
  voxels: Res<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Visited))>
) {
  touches.read().for_each(|&FromClient { client_id, message: Attune(at) }| {
    if let Some((avatar, mut visited)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
      && let Some(island) = Island::near(voxels.seed, at.as_vec3())
        .into_iter()
        .find(|island| at == island.stone || at == island.stone + IVec3::Y)
      && !visited.0.contains(&island.cell)
    {
      visited.0.push(island.cell)
    }
  })
}

fn travel(
  mut travels: MessageReader<FromClient<Travel>>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&mut Avatar, &Visited))>,
  mut teleports: MessageWriter<ToClients<Teleport>>
) {
  travels.read().for_each(|&FromClient { client_id, message: Travel(cell) }| {
    let seed = voxels.seed;
    if let Some((mut avatar, visited)) = player_of(players.iter_mut(), client_id)
      && visited.0.contains(&cell)
      && Island::beside(seed, avatar.at).is_some()
      && let Some(island) = Island::at(seed, cell)
    {
      let arrival = island.arrival();
      let feet = (0..32)
        .map(|lift| arrival.floor().as_ivec3() + IVec3::Y * lift)
        .find(|&at| !voxels.ensure(at).solid() && !voxels.ensure(at + IVec3::Y).solid())
        .map_or(arrival.y, |at| at.y as f32 + 0.05);
      let moved = Avatar { at: arrival.with_y(feet), yaw: FACING_STONE, pitch: 0.0 };
      *avatar = moved;
      teleports.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Teleport(moved)
      });
    }
  })
}

fn shuffle(
  mut shuffles: MessageReader<FromClient<Shuffle>>,
  mut players: Query<(&Controller, &mut Inventory)>
) {
  shuffles.read().for_each(|&FromClient { client_id, message: Shuffle { from, to } }| {
    if let Some(mut inventory) = player_of(players.iter_mut(), client_id) {
      inventory.shuffle(from.into(), to.into())
    }
  })
}

fn craft(
  mut crafts: MessageReader<FromClient<Craft>>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>
) {
  crafts.read().for_each(|&FromClient { client_id, message: Craft(index) }| {
    if let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Some(recipe) = RECIPES.get(usize::from(index))
      && let eye = avatar.at + Vec3::Y * EYE
      && let stations = {
        voxels.ensure(eye.floor().as_ivec3());
        voxels.stations_near(eye, REACH)
      }
      && let Some(made) = recipe.made(&inventory, &stations)
    {
      *inventory = made
    }
  })
}

fn scoop(
  mut scoops: MessageReader<FromClient<Scoop>>,
  time: Res<Time>,
  mut flows: ResMut<Flows>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  mut changes: MessageWriter<ToClients<Altered>>
) {
  scoops.read().for_each(|&FromClient { client_id, message: Scoop(at) }| {
    if let Some((fluid, 0)) = voxels.ensure(at).liquid()
      && let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
      && let Some(filled) = Some(inventory.clone())
        .filter(|filled| {
          filled.slots.iter().flatten().any(|stack| stack.block == Block::Bucket)
        })
        .map(|mut filled| {
          filled.take(Block::Bucket);
          filled
        })
        .filter(|filled| filled.clone().add(fluid.bucket()))
        .map(|mut filled| {
          filled.add(fluid.bucket());
          filled
        })
    {
      *inventory = filled;
      voxels.set(at, Block::Air);
      flows.stir(at, time.elapsed_secs() + fluid.delay());
      changes.write(ToClients {
        targets: SendTargets::All,
        message: Altered { at, block: Block::Air }
      });
    }
  })
}

fn pour(
  mut pours: MessageReader<FromClient<Pour>>,
  time: Res<Time>,
  mut flows: ResMut<Flows>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  mut changes: MessageWriter<ToClients<Altered>>
) {
  pours.read().for_each(|&FromClient { client_id, message: Pour { at, fluid } }| {
    let present = voxels.ensure(at);
    let open = present == Block::Air
      || present.modelled()
      || present.liquid().is_some_and(|(_, level)| level > 0);
    if open
      && let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
      && inventory.take(fluid.bucket())
    {
      inventory.add(Block::Bucket);
      voxels.set(at, fluid.source());
      flows.stir(at, time.elapsed_secs() + fluid.delay());
      changes.write(ToClients {
        targets: SendTargets::All,
        message: Altered { at, block: fluid.source() }
      });
    }
  })
}

fn mark(
  mut marks: MessageReader<FromClient<Mark>>,
  mut players: Query<(&Controller, &mut Bookmarks)>
) {
  marks.read().for_each(|&FromClient { client_id, message: Mark(block) }| {
    if let Some(mut bookmarks) = player_of(players.iter_mut(), client_id)
      && block.item()
    {
      bookmarks.toggle(block)
    }
  })
}

fn rest(
  mut rests: MessageReader<FromClient<Rest>>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Bedside))>,
  mut notices: MessageWriter<ToClients<Notice>>
) {
  rests.read().for_each(|&FromClient { client_id, message: Rest(at) }| {
    if voxels.ensure(at) == Block::Bed
      && let Some((avatar, mut bedside)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
    {
      bedside.0 = Some(at);
      notices.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Notice("Spawn point set at your bed".into())
      });
    }
  })
}

pub struct Authority;

impl Plugin for Authority {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, found_world.run_if(authority))
      .add_systems(
        PreUpdate,
        (
          sign_in, paint, follow, attune, travel, dig, put, scoop, pour, shuffle, craft,
          mark, rest
        )
          .chain()
          .after(ServerSystems::Receive)
          .run_if(authority)
      )
      .add_observer(welcome)
      .add_observer(farewell);
  }
}
