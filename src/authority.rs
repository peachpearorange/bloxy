use {crate::{account::{Account, Accounts, Kept, LONGEST_NAME, tidy},
             block::{Block, Fluid},
             claim::{self, Claims, barred},
             flow::Flows,
             folk, generate,
             generate::Waystone,
             island::FACING_STONE,
             opts::opts,
             protocol::*,
             recipe::RECIPES,
             save::{self, World},
             shroomling, sign,
             skill::{Earned, Skills},
             voxels::{Voxels, unpack}},
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
    Block::GreywackeCobble,
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
    Block::Greywacke,
    Block::Reefstone,
    Block::Mech,
    Block::GreywackeBricks,
    Block::Dirt,
    Block::Sand,
    Block::Submarine,
    Block::SonarModule,
    Block::DrillModule,
    Block::Floodlight,
    Block::TorpedoBay,
    Block::Torpedo,
    Block::BioLab,
    Block::PrecursorAlloy,
    Block::ArcaneDust,
    Block::BoneDust,
    Block::Candle,
    Block::Nacre,
    Block::SpellFirebolt,
    Block::SpellMissile,
    Block::SpellThunder,
    Block::SpellLight,
    Block::Brazier,
    Block::Pickaxe,
    Block::Axe,
    Block::Shovel,
    Block::Sword
  ];
  let mut inventory = Inventory::default();
  if creative {
    for (&block, slot) in kit.iter().zip(&mut inventory.slots) {
      *slot = Some(Stack {
        gear: block
          .forged()
          .then(|| crate::gear::Gear::forged(crate::gear::Material::Iron)),
        ..Stack::of(block, block.stack())
      })
    }
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
  let hue = accounts.0[account].hue();
  let Account {
    name,
    avatar,
    inventory,
    skin,
    visited,
    bookmarks,
    bedside,
    skills,
    deeds,
    ..
  } = accounts.0[account].clone();
  let most = skills.most();
  let at = generate::spawn_point(seed);
  let player = commands
    .spawn((
      Replicated,
      Controller { client, account },
      Player { name, hue },
      avatar.unwrap_or(Avatar { at, yaw: FACING_STONE, pitch: 0.0, ..default() }),
      inventory,
      skin,
      visited,
      bookmarks,
      bedside,
      (
        skills,
        deeds,
        crate::condition::Conditions::default(),
        crate::spell::Spellbook::default(),
        Health(most),
        folk::Vigour::default()
      )
    ))
    .id();
  commands.write_message(ToClients {
    targets: SendTargets::Single(client),
    message: Possess(player)
  });
  player
}

const STOCKED_PER_CHEST: usize = 27;

#[derive(Resource, Default)]
pub struct Stock(pub Vec<(IVec3, Vec<Stack>)>);

fn stock(voxels: &mut Voxels) -> Stock {
  let items: Vec<Stack> = Block::ALL
    .into_iter()
    .filter(|block| block.item())
    .map(|block| Stack {
      gear: block
        .forged()
        .then(|| crate::gear::Gear::forged(crate::gear::Material::Iron)),
      ..Stack::of(block, block.stack())
    })
    .collect();
  let stone =
    Waystone::at(voxels.seed, IVec2::ZERO).map_or(IVec3::new(0, 100, 0), |home| home.at);
  Stock(
    items
      .chunks(STOCKED_PER_CHEST)
      .zip(-2..)
      .map(|(loot, along)| {
        let column = stone.xz() + IVec2::new(-4, along);
        let ground = (0..stone.y + 12)
          .rev()
          .map(|y| column.extend(y).xzy())
          .find(|&at| {
            let block = voxels.ensure(at);
            block.solid() && !block.leafy()
              || matches!(block, Block::Chest | Block::EmptyChest)
          })
          .unwrap_or(stone);
        let at = match voxels.ensure(ground) {
          Block::Chest | Block::EmptyChest => ground,
          _ => ground + IVec3::Y
        };
        if !matches!(voxels.ensure(at), Block::Chest | Block::EmptyChest) {
          voxels.set(at, Block::Chest)
        }
        (at, loot.to_vec())
      })
      .collect()
  )
}

fn found_world(mut commands: Commands) {
  let World {
    seed,
    edits,
    accounts,
    shroomlings,
    boats,
    signs,
    claims,
    kept,
    generation,
    coffers,
    balloons,
    works,
    packs,
    pockets,
    subs,
    mechs
  } = save::load().unwrap_or_else(|| World {
    seed: opts().seed,
    edits: default(),
    accounts: default(),
    shroomlings: default(),
    boats: default(),
    signs: default(),
    claims: default(),
    kept: default(),
    generation: Some(generate::GENERATION),
    coffers: default(),
    balloons: default(),
    works: default(),
    packs: default(),
    pockets: default(),
    subs: default(),
    mechs: default()
  });
  let migrating = generation.is_some_and(|generation| generation != generate::GENERATION);
  if migrating {
    info!(
      "world generation changed: keeping {} claimed chunks, dropping other edits",
      claims.len()
    )
  }
  let accounts = Accounts(accounts);
  claim::settle(&mut commands, &accounts, &claims);
  let (edits, shroomlings) = match migrating {
    true => (Vec::new(), Vec::new()),
    false => (edits, shroomlings)
  };
  for sign in signs.into_iter() {
    commands.spawn((Replicated, sign));
  }
  for (at, yaw) in boats.into_iter() {
    commands.spawn((Replicated, Vessel { at, yaw, rider: None }));
  }
  for sub in subs.into_iter() {
    crate::submarine::spawn(&mut commands, sub)
  }
  for mech in mechs.into_iter() {
    crate::mech::spawn(&mut commands, mech)
  }
  for (at, yaw, fuel, pattern) in balloons.into_iter() {
    crate::balloon::spawn(
      &mut commands,
      crate::balloon::Aerostat::landed(at, yaw, fuel, pattern)
    )
  }
  let colonies = shroomlings.iter().map(|&(home, _)| home).collect();
  for (index, (home, hopper)) in shroomlings.into_iter().enumerate() {
    shroomling::lodge(&mut commands, home, hopper, index as u32 * 0x9E37 + 1)
  }
  commands.insert_resource(shroomling::Colonies(colonies));
  let mut voxels = Voxels::new(seed);
  voxels.kept = match migrating {
    true => claim::kept_from(&claims),
    false => std::sync::Arc::new(
      kept
        .iter()
        .filter_map(|(column, text)| {
          unpack(&claim::decode(text))
            .map(|blocks| (*column, std::sync::Arc::new(blocks)))
        })
        .collect()
    )
  };
  for &(at, block) in edits.iter() {
    voxels.set(at, block)
  }
  voxels.ensure(generate::spawn_point(seed).floor().as_ivec3());
  commands.insert_resource(crate::chest::Coffers(
    coffers
      .into_iter()
      .filter(|&(at, _)| matches!(voxels.ensure(at), Block::Chest | Block::EmptyChest))
      .collect()
  ));
  commands.insert_resource(stock(&mut voxels));
  commands.insert_resource(crate::chest::Hoards {
    packs: packs.into_iter().collect(),
    pockets: pockets.into_iter().collect()
  });
  commands.insert_resource(crate::machine::settle(&mut voxels, match migrating {
    true => Vec::new(),
    false => works
  }));
  commands.insert_resource(voxels);
  commands.insert_resource(accounts)
}

fn welcome(
  joined: On<Add, AuthorizedClient>,
  mut commands: Commands,
  voxels: Res<Voxels>
) {
  commands.write_message(ToClients {
    targets: SendTargets::Single(ClientId::Client(joined.entity)),
    message: Welcome {
      seed: voxels.seed,
      edits: voxels.all_edits(),
      kept: voxels.all_kept()
    }
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
  for FromClient { client_id, message: Hello { name, password } } in hellos.read() {
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
          commands
            .entity(entity)
            .insert(Player { name: name.clone(), hue: accounts.0[mine].hue() });
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
          accounts.0.push(Account {
            skills: match opts().creative {
              true => {
                let learned: Vec<u16> = (1..crate::skill::TREE.len() as u16).collect();
                let level =
                  Skills { learned: learned.clone(), ..default() }.spent() as u32;
                Skills { learned, level, xp: 0, stance: None }
              }
              false => default()
            },
            ..Account::open(name.clone(), password, starter(opts().creative))
          });
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
  }
}

fn paint(
  mut paints: MessageReader<FromClient<Paint>>,
  mut commands: Commands,
  players: Query<(Entity, &Controller)>
) {
  for paint in paints.read().filter(|paint| paint.message.0.valid()) {
    for (entity, _) in
      players.iter().filter(|(_, controller)| controller.client == paint.client_id)
    {
      commands.entity(entity).insert(paint.message.0.clone());
    }
  }
}

fn tint(
  mut tints: MessageReader<FromClient<Tint>>,
  mut accounts: ResMut<Accounts>,
  mut players: Query<(&Controller, &mut Player)>
) {
  for tint in tints.read().filter(|tint| usize::from(tint.message.0) < HUES.len()) {
    for (controller, mut player) in
      players.iter_mut().filter(|(controller, _)| controller.client == tint.client_id)
    {
      accounts.0[controller.account].hue = Some(tint.message.0);
      player.hue = tint.message.0
    }
  }
}

fn chat(
  mut sayings: MessageReader<FromClient<Say>>,
  players: Query<(&Controller, &Player)>,
  mut said: MessageWriter<ToClients<Said>>
) {
  for FromClient { client_id, message: Say(text) } in sayings.read() {
    let text: String =
      text.chars().filter(|c| !c.is_control()).take(LONGEST_SAYING).collect();
    if let Some((_, player)) =
      players.iter().find(|(controller, _)| controller.client == *client_id)
      && !text.trim().is_empty()
    {
      info!("{}: {}", player.name, text.trim());
      said.write(ToClients {
        targets: SendTargets::All,
        message: Said {
          name: player.name.clone(),
          hue: player.hue,
          text: text.trim().into()
        }
      });
    }
  }
}

fn farewell(
  left: On<Remove, ConnectedClient>,
  mut commands: Commands,
  mut accounts: ResMut<Accounts>,
  players: Query<(Entity, &Controller, Kept)>
) {
  for (entity, controller, kept) in players
    .iter()
    .filter(|(_, controller, _)| controller.client == ClientId::Client(left.entity))
  {
    accounts.0[controller.account].keep(kept);
    commands.entity(entity).despawn()
  }
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
  for moved in moves.read() {
    if let Some(mut avatar) = player_of(players.iter_mut(), moved.client_id)
      && moved.message.0.at.is_finite()
    {
      avatar.set_if_neq(moved.message.0);
    }
  }
}

pub fn within_reach(avatar: &Avatar, at: IVec3) -> bool {
  (avatar.at + Vec3::Y * EYE).distance(at.as_vec3() + Vec3::splat(0.5)) <= REACH + 2.0
}

fn account_of<T: bevy::ecs::query::QueryData>(
  players: &Query<(&Controller, T)>,
  client: ClientId
) -> Option<usize> {
  players
    .iter()
    .find(|(controller, _)| controller.client == client)
    .map(|(controller, _)| controller.account)
}

fn guarded(
  claims: &Claims,
  notices: &mut MessageWriter<ToClients<Notice>>,
  client: ClientId,
  account: usize,
  at: IVec3
) -> bool {
  barred(claims, account, at)
    .map(|word| {
      notices
        .write(ToClients { targets: SendTargets::Single(client), message: Notice(word) });
    })
    .is_none()
}

fn ore(block: Block) -> u32 {
  match block {
    Block::CoalOre => 1,
    Block::CopperOre | Block::TinOre | Block::IronOre => 2,
    Block::GoldOre => 5,
    Block::DiamondOre => 12,
    Block::PlatinumOre => 8,
    Block::Lumen => 4,
    _ => 0
  }
}

fn dig(
  mut digs: MessageReader<FromClient<Dig>>,
  time: Res<Time>,
  mut flows: ResMut<Flows>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Inventory, &Skills))>,
  (claims, mut notices): (Claims, MessageWriter<ToClients<Notice>>),
  mut changes: MessageWriter<ToClients<Altered>>,
  mut earned: MessageWriter<Earned>,
  (subs, mechs): (Query<&crate::submarine::Sub>, Query<&crate::mech::Mech>),
  mut commands: Commands
) {
  for &FromClient { client_id, message: Dig { at } } in digs.read() {
    let block = voxels.ensure(at);
    let above = at + IVec3::Y;
    let perched: Vec<(IVec3, Block)> = Some((above, voxels.ensure(above)))
      .filter(|(_, block)| block.modelled() && block.wall().is_none() && !block.bed())
      .into_iter()
      .chain(Block::WALL_TORCHES.iter().filter_map(|&(wall, torch)| {
        let cell = at - wall;
        (voxels.ensure(cell) == torch).then_some((cell, torch))
      }))
      .collect();
    let partner = block
      .partner()
      .map(|toward| at + toward)
      .filter(|&cell| voxels.ensure(cell).partner() == Some(at - cell));
    let account = account_of(&players, client_id);
    let allowed = account
      .is_some_and(|account| guarded(&claims, &mut notices, client_id, account, at));
    let drilling = |player: Entity| {
      subs.iter().find(|sub| sub.rider == Some(player)).and_then(|sub| sub.drill()).or(
        mechs
          .iter()
          .find(|mech| mech.rider == Some(player))
          .and_then(|mech| mech.drill())
      )
    };
    let boxed = |player: Entity| {
      mechs.iter().any(|mech| mech.rider == Some(player) && mech.has(Block::OreBox))
    };
    let digger = player_of(players.iter_mut(), client_id)
      .filter(|(player, avatar, ..)| {
        let bored = drilling(*player).is_some_and(|drill| {
          (avatar.at + Vec3::Y * EYE).distance(at.as_vec3() + 0.5) <= drill.reach + 2.0
        });
        allowed && block.breakable() && (within_reach(avatar, at) || bored)
      })
      .map(|(player, avatar, inventory, skills)| {
        (player, avatar.at, skills.geared(Some(&inventory), avatar.hand).fortune)
      });
    let hoarded = digger.is_some_and(|(player, ..)| boxed(player));
    match digger {
      Some((player, from, fortune)) => {
        let along = (at.as_vec3() + 0.5 - from - Vec3::Y * EYE).abs();
        let axis = match (along.x >= along.y && along.x >= along.z, along.y >= along.z) {
          (true, _) => 0,
          (_, true) => 1,
          _ => 2
        };
        let flank: Vec<IVec3> = match drilling(player).is_some_and(|drill| drill.wide) {
          true => (-1..=1)
            .flat_map(|a| (-1..=1).map(move |b| (a, b)))
            .filter(|&pair| pair != (0, 0))
            .map(|(a, b)| {
              let mut offset = IVec3::ZERO;
              offset[(axis + 1) % 3] = a;
              offset[(axis + 2) % 3] = b;
              at + offset
            })
            .collect(),
          false => vec![]
        };
        for cell in flank {
          let beside = voxels.ensure(cell);
          if beside.breakable()
            && beside.solid()
            && account.is_some_and(|account| barred(&claims, account, cell).is_none())
          {
            crate::loose::fall(&mut commands, beside.drop(), cell);
            voxels.set(cell, Block::Air);
            flows.stir(cell, time.elapsed_secs() + Fluid::Water.delay());
            changes.write(ToClients {
              targets: SendTargets::All,
              message: Altered { at: cell, block: Block::Air }
            });
          }
        }
        let lucky = ore(block) > 0
          && crate::noise::unit(voxels.seed ^ 0xF0F, at.x, at.y, at.z) < fortune;
        let found = (block == Block::Urn)
          .then(|| {
            crate::dungeon::urn(voxels.seed, at, (time.elapsed_secs() * 10.0) as u32)
          })
          .flatten();
        let rig = subs.iter().find(|sub| sub.rider == Some(player));
        let clawed = rig.is_some_and(|sub| sub.has(Block::SampleClaw));
        let shredded = block.organism() && rig.is_some() && !clawed;
        for dropped in std::iter::once(block.drop())
          .filter(|_| !shredded)
          .chain((block.organism() && clawed).then_some(block.drop()))
          .chain(perched.iter().map(|&(_, block)| block.drop()))
          .chain(lucky.then_some(block.drop()))
          .chain((block == Block::PipedDrain).then_some(Block::CopperPipe))
          .chain(found)
          .filter(|&dropped| dropped != Block::Air)
        {
          let kept = hoarded
            && player_of(players.iter_mut(), client_id)
              .is_some_and(|(_, _, mut inventory, _)| inventory.add(dropped));
          if !kept {
            crate::loose::fall(&mut commands, dropped, at)
          }
        }
        if ore(block) > 0 {
          earned.write(Earned { player, xp: ore(block) });
        }
        let left = match block.waterlogged() {
          true => Block::Water,
          false => Block::Air
        };
        voxels.set(at, left);
        flows.stir(at, time.elapsed_secs() + Fluid::Water.delay());
        for cell in partner.into_iter() {
          voxels.set(cell, Block::Air);
          changes.write(ToClients {
            targets: SendTargets::All,
            message: Altered { at: cell, block: Block::Air }
          });
        }
        for &(cell, _) in perched.iter() {
          voxels.set(cell, Block::Air);
          changes.write(ToClients {
            targets: SendTargets::All,
            message: Altered { at: cell, block: Block::Air }
          });
        }
        changes.write(ToClients {
          targets: SendTargets::All,
          message: Altered { at, block: left }
        })
      }
      None => changes.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Altered { at, block }
      })
    };
  }
}

fn put(
  mut puts: MessageReader<FromClient<Put>>,
  time: Res<Time>,
  mut flows: ResMut<Flows>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Inventory))>,
  everyone: Query<&Avatar>,
  mechs: Query<&crate::mech::Mech>,
  (claims, mut notices): (Claims, MessageWriter<ToClients<Notice>>),
  mut changes: MessageWriter<ToClients<Altered>>,
  mut commands: Commands
) {
  for &FromClient { client_id, message: Put { at, block } } in puts.read() {
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
    let head = block
      .partner()
      .filter(|_| !block.pillow())
      .and_then(|toward| Block::laid(toward).map(|(_, pillow)| (at + toward, pillow)));
    let roomy = head.is_none_or(|(cell, _)| {
      !voxels.ensure(cell).solid()
        && voxels.ensure(cell - IVec3::Y).solid()
        && !everyone.iter().any(|avatar| avatar.at.floor().as_ivec3() == cell)
    });
    if let Some(account) = account_of(&players, client_id)
      && guarded(&claims, &mut notices, client_id, account, at)
      && head
        .is_none_or(|(cell, _)| guarded(&claims, &mut notices, client_id, account, cell))
      && block.held().placeable()
      && !block.pillow()
      && roomy
      && !(crowded && block.solid())
      && footed
      && !present.solid()
      && let Some((player, avatar, mut inventory)) =
        player_of(players.iter_mut(), client_id)
      && (within_reach(avatar, at)
        || mechs
          .iter()
          .find(|mech| mech.rider == Some(player))
          .and_then(crate::mech::Mech::reach)
          .is_some_and(|reach| {
            (avatar.at + Vec3::Y * EYE).distance(at.as_vec3() + 0.5) <= reach + 1.0
          }))
      && inventory.take(block.held())
    {
      voxels.set(at, block);
      for (cell, pillow) in head.into_iter() {
        voxels.set(cell, pillow);
        changes.write(ToClients {
          targets: SendTargets::All,
          message: Altered { at: cell, block: pillow }
        });
      }
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
      for (cell, _) in head.into_iter() {
        changes.write(ToClients {
          targets: SendTargets::Single(client_id),
          message: Altered { at: cell, block: voxels.ensure(cell) }
        });
      }
    }
  }
}

fn attune(
  mut touches: MessageReader<FromClient<Attune>>,
  voxels: Res<Voxels>,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Visited))>,
  mut earned: MessageWriter<Earned>
) {
  for &FromClient { client_id, message: Attune(at) } in touches.read() {
    if let Some((player, avatar, mut visited)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
      && let Some(stone) = Waystone::beside(voxels.seed, at.as_vec3())
        .filter(|stone| at == stone.at || at == stone.at + IVec3::Y)
      && !visited.0.contains(&stone.cell)
    {
      visited.0.push(stone.cell);
      earned.write(Earned { player, xp: 30 });
    }
  }
}

fn travel(
  mut travels: MessageReader<FromClient<Travel>>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&mut Avatar, &Visited))>,
  mut teleports: MessageWriter<ToClients<Teleport>>
) {
  for &FromClient { client_id, message: Travel(cell) } in travels.read() {
    let seed = voxels.seed;
    if let Some((mut avatar, visited)) = player_of(players.iter_mut(), client_id)
      && visited.0.contains(&cell)
      && Waystone::beside(seed, avatar.at).is_some()
      && let Some(stone) = Waystone::at(seed, cell)
    {
      let arrival = stone.arrival();
      let feet = (0..32)
        .map(|lift| arrival.floor().as_ivec3() + IVec3::Y * lift)
        .find(|&at| !voxels.ensure(at).solid() && !voxels.ensure(at + IVec3::Y).solid())
        .map_or(arrival.y, |at| at.y as f32 + 0.05);
      let moved =
        Avatar { at: arrival.with_y(feet), yaw: FACING_STONE, pitch: 0.0, ..*avatar };
      *avatar = moved;
      teleports.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Teleport(moved)
      });
    }
  }
}

fn shuffle(
  mut shuffles: MessageReader<FromClient<Shuffle>>,
  mut players: Query<(&Controller, &mut Inventory)>
) {
  for &FromClient { client_id, message: Shuffle { from, to } } in shuffles.read() {
    if let Some(mut inventory) = player_of(players.iter_mut(), client_id) {
      inventory.shuffle(from.into(), to.into())
    }
  }
}

fn pick(
  mut picks: MessageReader<FromClient<Picked>>,
  mut players: Query<(&Controller, &mut Inventory)>
) {
  for &FromClient { client_id, message: Picked(pick) } in picks.read() {
    if let Some(mut inventory) = player_of(players.iter_mut(), client_id) {
      match pick {
        Pick::Half(slot) => inventory.halve(slot.into()),
        Pick::One(slot) => inventory.drop_one(slot.into()),
        Pick::Stow => inventory.stow()
      }
    }
  }
}

fn craft(
  mut crafts: MessageReader<FromClient<Craft>>,
  mut voxels: ResMut<Voxels>,
  mut works: ResMut<crate::machine::Works>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory, &Skills))>
) {
  for &FromClient { client_id, message: Craft(index) } in crafts.read() {
    if let Some((avatar, mut inventory, skills)) =
      player_of(players.iter_mut(), client_id)
      && let Some(recipe) = RECIPES.get(usize::from(index))
      && let eye = avatar.at + Vec3::Y * EYE
      && let stations = {
        voxels.ensure(eye.floor().as_ivec3());
        voxels.stations_near(eye, REACH, |at| works.charged(at))
      }
      && let Some(made) = recipe.made(&inventory, &stations, &skills.spells())
    {
      *inventory = made;
      if recipe.electric(&stations) {
        works.smelt(&voxels, eye, REACH)
      }
    }
  }
}

fn scoop(
  mut scoops: MessageReader<FromClient<Scoop>>,
  time: Res<Time>,
  mut flows: ResMut<Flows>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  (claims, mut notices): (Claims, MessageWriter<ToClients<Notice>>),
  mut changes: MessageWriter<ToClients<Altered>>
) {
  for &FromClient { client_id, message: Scoop(at) } in scoops.read() {
    if let Some(account) = account_of(&players, client_id)
      && guarded(&claims, &mut notices, client_id, account, at)
      && let Some((fluid, 0)) = voxels.ensure(at).liquid()
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
  }
}

fn pour(
  mut pours: MessageReader<FromClient<Pour>>,
  time: Res<Time>,
  mut flows: ResMut<Flows>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  (claims, mut notices): (Claims, MessageWriter<ToClients<Notice>>),
  mut changes: MessageWriter<ToClients<Altered>>
) {
  for &FromClient { client_id, message: Pour { at, fluid } } in pours.read() {
    let present = voxels.ensure(at);
    let open = present == Block::Air
      || present.modelled()
      || present.liquid().is_some_and(|(_, level)| level > 0);
    if open
      && let Some(account) = account_of(&players, client_id)
      && guarded(&claims, &mut notices, client_id, account, at)
      && let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
      && let sigil = fluid == crate::block::Fluid::Water
      && inventory
        .slots
        .get(usize::from(avatar.hand))
        .copied()
        .flatten()
        .is_some_and(|stack| stack.block == Block::WellspringSigil)
      && (sigil || inventory.take(fluid.bucket()))
    {
      if !sigil {
        inventory.add(Block::Bucket);
      }
      voxels.set(at, fluid.source());
      flows.stir(at, time.elapsed_secs() + fluid.delay());
      changes.write(ToClients {
        targets: SendTargets::All,
        message: Altered { at, block: fluid.source() }
      });
    }
  }
}

fn mark(
  mut marks: MessageReader<FromClient<Mark>>,
  mut players: Query<(&Controller, &mut Bookmarks)>
) {
  for &FromClient { client_id, message: Mark(block) } in marks.read() {
    if let Some(mut bookmarks) = player_of(players.iter_mut(), client_id)
      && block.item()
    {
      bookmarks.toggle(block)
    }
  }
}

fn rest(
  mut rests: MessageReader<FromClient<Rest>>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(&Controller, (&Avatar, &mut Bedside))>,
  mut notices: MessageWriter<ToClients<Notice>>
) {
  for &FromClient { client_id, message: Rest(at) } in rests.read() {
    if voxels.ensure(at).bed()
      && let Some((avatar, mut bedside)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
    {
      bedside.0 = Some(at);
      notices.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Notice("Spawn point set at your bed".into())
      });
    }
  }
}

pub struct Authority;

impl Plugin for Authority {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, found_world.run_if(authority))
      .add_systems(
        PreUpdate,
        (
          sign_in, paint, tint, chat, follow, attune, travel, dig, put, scoop, pour,
          shuffle, pick, craft, mark, rest
        )
          .chain()
          .after(ServerSystems::Receive)
          .run_if(authority)
      )
      .add_observer(welcome)
      .add_observer(farewell);
  }
}
