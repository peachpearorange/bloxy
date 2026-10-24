use {crate::{authority::{Controller, Stock, player_of, within_reach},
             block::Block,
             dungeon::{Dungeon, loot},
             menu::{Menu, Tab},
             protocol::{Altered, Avatar, CURSOR, HELD, Inventory, Notice, Open, Stack,
                        authority, plays},
             skill::Earned,
             voxels::Voxels},
     bevy::{platform::collections::HashMap, prelude::*},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

pub const ROOM: usize = 30;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Handling {
  Swap(u8),
  Deposit(u8),
  Withdraw(u8),
  Half(u8),
  One(u8)
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Hoard {
  Chest(IVec3),
  Pack(u32),
  Pocket(u32)
}

impl Default for Hoard {
  fn default() -> Hoard { Hoard::Chest(IVec3::ZERO) }
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Rummage {
  pub at: Hoard,
  pub handling: Handling
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Unpack(pub u8);

#[derive(Message, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Opened {
  pub at: Hoard,
  pub kind: Block,
  pub slots: Vec<Option<Stack>>
}

#[derive(Resource, Default)]
pub struct Coffers(pub HashMap<IVec3, Vec<Option<Stack>>>);

#[derive(Resource, Default)]
pub struct Hoards {
  pub packs: HashMap<u32, Vec<Option<Stack>>>,
  pub pockets: HashMap<u32, Vec<Option<Stack>>>
}

pub const POCKET: usize = 30;

pub fn room(block: Block) -> usize {
  match block {
    Block::Satchel => 9,
    Block::Rucksack => 18,
    Block::ExpeditionPack => 27,
    Block::LesserRift => 12,
    Block::GreaterRift => POCKET,
    _ => ROOM
  }
}

fn rift_room(inventory: &Inventory) -> Option<(Block, usize)> {
  inventory
    .slots
    .iter()
    .flatten()
    .filter(|stack| stack.block.rift())
    .map(|stack| (stack.block, room(stack.block)))
    .max_by_key(|&(_, room)| room)
}

fn carries_pack(inventory: &Inventory, id: u32) -> Option<Block> {
  inventory
    .slots
    .iter()
    .flatten()
    .find(|stack| stack.block.pack() && stack.gear.and_then(|gear| gear.pack) == Some(id))
    .map(|stack| stack.block)
}

fn fitted(slots: &mut Vec<Option<Stack>>, room: usize) {
  if slots.len() < room {
    slots.resize(room, None)
  }
}

#[derive(Component)]
struct Peeking(Hoard);

#[derive(Resource, Default)]
pub struct Coffer(pub Opened);

pub fn stash(slots: &mut [Option<Stack>], stack: Stack) -> Option<Stack> {
  let left = match stack.gear {
    Some(_) => stack,
    None => slots.iter_mut().fold(stack, |left, slot| match slot {
      Some(kept) if kept.block == left.block && kept.gear.is_none() => {
        let moved = left.count.min(kept.block.stack().saturating_sub(kept.count));
        kept.count += moved;
        Stack { count: left.count - moved, ..left }
      }
      _ => left
    })
  };
  match (left.count, slots.iter().position(Option::is_none)) {
    (0, _) => None,
    (_, Some(free)) => {
      slots[free] = Some(left);
      None
    }
    (_, None) => Some(left)
  }
}

fn moved(inventory: &Inventory, handling: Handling) -> Option<Stack> {
  match handling {
    Handling::Swap(_) | Handling::One(_) => inventory.slots[CURSOR],
    Handling::Deposit(slot) => inventory.slots.get(usize::from(slot)).copied().flatten(),
    Handling::Withdraw(_) | Handling::Half(_) => None
  }
}

fn handle(chest: &mut [Option<Stack>], inventory: &mut Inventory, handling: Handling) {
  match handling {
    Handling::Swap(index) => {
      if let Some(&kept) = chest.get(usize::from(index)) {
        let index = usize::from(index);
        match (inventory.slots[CURSOR], kept) {
          (Some(held), Some(kept))
            if held.block == kept.block
              && held.gear == kept.gear
              && kept.count < kept.block.stack() =>
          {
            let moved = held.count.min(kept.block.stack() - kept.count);
            chest[index] = Some(Stack { count: kept.count + moved, ..kept });
            inventory.slots[CURSOR] =
              (held.count > moved).then_some(Stack { count: held.count - moved, ..held })
          }
          (held, kept) => {
            chest[index] = held;
            inventory.slots[CURSOR] = kept
          }
        }
      }
    }
    Handling::Deposit(slot) => {
      let slot = usize::from(slot);
      if slot < HELD
        && slot != CURSOR
        && let Some(stack) = inventory.slots[slot]
      {
        inventory.slots[slot] = stash(chest, stack)
      }
    }
    Handling::Withdraw(index) => {
      if let Some(slot) = chest.get_mut(usize::from(index))
        && let Some(stack) = *slot
      {
        let left = inventory.gain(stack);
        *slot = (left > 0).then_some(Stack { count: left, ..stack })
      }
    }
    Handling::Half(index) => {
      if let Some(slot) = chest.get_mut(usize::from(index))
        && inventory.slots[CURSOR].is_none()
        && let Some(stack) = *slot
      {
        let taken = stack.count.div_ceil(2);
        inventory.slots[CURSOR] = Some(Stack { count: taken, ..stack });
        *slot =
          (stack.count > taken).then_some(Stack { count: stack.count - taken, ..stack })
      }
    }
    Handling::One(index) => {
      if let Some(slot) = chest.get_mut(usize::from(index))
        && let Some(held) = inventory.slots[CURSOR]
        && slot.is_none_or(|stack| {
          stack.block == held.block
            && stack.gear == held.gear
            && stack.count < held.block.stack()
        })
      {
        *slot = Some(Stack { count: slot.map_or(0, |stack| stack.count) + 1, ..held });
        inventory.slots[CURSOR] =
          (held.count > 1).then_some(Stack { count: held.count - 1, ..held })
      }
    }
  }
}

fn chest(block: Block) -> bool { matches!(block, Block::Chest | Block::EmptyChest) }

fn shown(chest: &[Option<Stack>]) -> Block {
  match chest.iter().any(Option::is_some) {
    true => Block::Chest,
    false => Block::EmptyChest
  }
}

fn open(
  mut opens: MessageReader<FromClient<Open>>,
  mut voxels: ResMut<Voxels>,
  mut coffers: ResMut<Coffers>,
  players: Query<(&Controller, (Entity, &Avatar))>,
  mut shown_to: MessageWriter<ToClients<Opened>>,
  mut notices: MessageWriter<ToClients<Notice>>,
  mut earned: MessageWriter<Earned>,
  stock: Res<Stock>,
  mut commands: Commands
) {
  for &FromClient { client_id, message: Open(at) } in opens.read() {
    let seed = voxels.seed;
    let block = voxels.ensure(at);
    if chest(block)
      && let Some((player, avatar)) = player_of(players.iter(), client_id)
      && within_reach(avatar, at)
    {
      let slots = coffers
        .0
        .entry(at)
        .or_insert_with(|| {
          let found = match block {
            Block::Chest => {
              let tier = Dungeon::within(seed, at.xz(), at.xz())
                .iter()
                .find_map(|dungeon| dungeon.chest(at))
                .unwrap_or(1);
              earned.write(Earned { player, xp: 20 + u32::from(tier) * 15 });
              notices.write(ToClients {
                targets: SendTargets::Single(client_id),
                message: Notice("You found a chest".into())
              });
              stock
                .0
                .iter()
                .find(|(chest, _)| *chest == at)
                .map_or_else(|| loot(seed, at, tier), |(_, loot)| loot.clone())
            }
            _ => Vec::new()
          };
          let mut slots = vec![None; ROOM.max(found.len())];
          for (slot, stack) in slots.iter_mut().zip(found) {
            *slot = Some(stack)
          }
          slots
        })
        .clone();
      commands.entity(player).insert(Peeking(Hoard::Chest(at)));
      shown_to.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Opened { at: Hoard::Chest(at), kind: Block::Chest, slots }
      });
    }
  }
}

fn unpack(
  mut unpacks: MessageReader<FromClient<Unpack>>,
  mut hoards: ResMut<Hoards>,
  mut players: Query<(&Controller, (Entity, &mut Inventory))>,
  mut shown_to: MessageWriter<ToClients<Opened>>,
  mut commands: Commands
) {
  for &FromClient { client_id, message: Unpack(slot) } in unpacks.read() {
    let account = players
      .iter()
      .find(|(controller, _)| controller.client == client_id)
      .map(|(controller, _)| controller.account as u32);
    let next = hoards.packs.keys().max().map_or(1, |most| most + 1);
    if let Some(account) = account
      && let Some((player, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Some(stack) = inventory.slots.get(usize::from(slot)).copied().flatten()
      && stack.block.portable()
    {
      let (at, kind, slots) = match stack.block.pack() {
        true => {
          let gear = stack.gear.unwrap_or_default();
          let id = gear.pack.unwrap_or(next);
          inventory.slots[usize::from(slot)] = Some(Stack {
            gear: Some(crate::gear::Gear { pack: Some(id), ..gear }),
            ..stack
          });
          let slots = hoards.packs.entry(id).or_default();
          fitted(slots, room(stack.block));
          (Hoard::Pack(id), stack.block, slots[..room(stack.block)].to_vec())
        }
        false => {
          let (kind, room) = rift_room(&inventory).unwrap_or((stack.block, 0));
          let slots = hoards.pockets.entry(account).or_default();
          fitted(slots, POCKET);
          (Hoard::Pocket(account), kind, slots[..room].to_vec())
        }
      };
      commands.entity(player).insert(Peeking(at));
      shown_to.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Opened { at, kind, slots }
      });
    }
  }
}

fn rummage(
  mut rummages: MessageReader<FromClient<Rummage>>,
  mut voxels: ResMut<Voxels>,
  mut coffers: ResMut<Coffers>,
  mut hoards: ResMut<Hoards>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  peeking: Query<(&Controller, &Peeking)>,
  mut shown_to: MessageWriter<ToClients<Opened>>,
  mut changes: MessageWriter<ToClients<Altered>>
) {
  for &FromClient { client_id, message: Rummage { at, handling } } in rummages.read() {
    let account = players
      .iter()
      .find(|(controller, _)| controller.client == client_id)
      .map(|(controller, _)| controller.account as u32);
    if let Some(account) = account
      && let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
    {
      let carried =
        moved(&inventory, handling).is_some_and(|stack| stack.block.portable());
      let opened = match at {
        Hoard::Chest(spot) => (chest(voxels.ensure(spot)) && within_reach(avatar, spot))
          .then(|| {
            coffers
              .0
              .get_mut(&spot)
              .map(|slots| (Hoard::Chest(spot), Block::Chest, slots.len(), slots))
          })
          .flatten(),
        Hoard::Pack(id) => {
          carries_pack(&inventory, id).filter(|_| !carried).and_then(|kind| {
            hoards
              .packs
              .get_mut(&id)
              .map(|slots| (at, kind, room(kind).min(slots.len()), slots))
          })
        }
        Hoard::Pocket(_) => {
          rift_room(&inventory).filter(|_| !carried).and_then(|(kind, room)| {
            hoards
              .pockets
              .get_mut(&account)
              .map(|slots| (Hoard::Pocket(account), kind, room.min(slots.len()), slots))
          })
        }
      };
      if let Some((hoard, kind, room, slots)) = opened {
        handle(&mut slots[..room], &mut inventory, handling);
        if let Hoard::Chest(spot) = hoard {
          let block = shown(slots);
          if voxels.ensure(spot) != block {
            voxels.set(spot, block);
            changes.write(ToClients {
              targets: SendTargets::All,
              message: Altered { at: spot, block }
            });
          }
        }
        for (controller, _) in peeking.iter().filter(|(_, peeking)| peeking.0 == hoard) {
          shown_to.write(ToClients {
            targets: SendTargets::Single(controller.client),
            message: Opened { at: hoard, kind, slots: slots[..room].to_vec() }
          });
        }
      }
    }
  }
}

fn peek(
  time: Res<Time>,
  mut opened: MessageReader<Opened>,
  mut coffer: ResMut<Coffer>,
  mut menu: ResMut<Menu>
) {
  for shown in opened.read() {
    if !menu.showing(Tab::Chest) || coffer.0.at != shown.at {
      menu.show(Tab::Chest, time.elapsed_secs())
    }
    coffer.0 = shown.clone()
  }
}

pub struct Chests;

impl Plugin for Chests {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Coffers>()
      .init_resource::<Hoards>()
      .init_resource::<Coffer>()
      .add_systems(
        PreUpdate,
        (open, unpack, rummage).chain().after(ServerSystems::Receive).run_if(authority)
      )
      .add_systems(Update, peek.run_if(plays).run_if(resource_exists::<Menu>));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn stacks_move_between_chest_and_inventory() {
    let mut chest = vec![None; ROOM];
    chest[0] = Some(Stack::of(Block::Planks, 60));
    let mut inventory = Inventory::default();
    inventory.slots[2] = Some(Stack::of(Block::Planks, 10));
    handle(&mut chest, &mut inventory, Handling::Deposit(2));
    assert_eq!(chest[0], Some(Stack::of(Block::Planks, 64)));
    assert_eq!(chest[1], Some(Stack::of(Block::Planks, 6)));
    assert_eq!(inventory.slots[2], None);
    handle(&mut chest, &mut inventory, Handling::Withdraw(0));
    assert_eq!(inventory.count(Block::Planks), 64);
    assert_eq!(chest[0], None);
    handle(&mut chest, &mut inventory, Handling::Swap(1));
    assert_eq!(inventory.slots[CURSOR], Some(Stack::of(Block::Planks, 6)));
    assert_eq!(shown(&chest), Block::EmptyChest)
  }
}
