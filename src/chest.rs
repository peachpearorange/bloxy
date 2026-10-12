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
  Withdraw(u8)
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Rummage {
  pub at: IVec3,
  pub handling: Handling
}

#[derive(Message, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct Opened {
  pub at: IVec3,
  pub slots: Vec<Option<Stack>>
}

#[derive(Resource, Default)]
pub struct Coffers(pub HashMap<IVec3, Vec<Option<Stack>>>);

#[derive(Component)]
struct Peeking(IVec3);

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
      commands.entity(player).insert(Peeking(at));
      shown_to.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Opened { at, slots }
      });
    }
  }
}

fn rummage(
  mut rummages: MessageReader<FromClient<Rummage>>,
  mut voxels: ResMut<Voxels>,
  mut coffers: ResMut<Coffers>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  peeking: Query<(&Controller, &Peeking)>,
  mut shown_to: MessageWriter<ToClients<Opened>>,
  mut changes: MessageWriter<ToClients<Altered>>
) {
  for &FromClient { client_id, message: Rummage { at, handling } } in rummages.read() {
    if chest(voxels.ensure(at))
      && let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
      && let Some(slots) = coffers.0.get_mut(&at)
    {
      handle(slots, &mut inventory, handling);
      let block = shown(slots);
      if voxels.ensure(at) != block {
        voxels.set(at, block);
        changes
          .write(ToClients { targets: SendTargets::All, message: Altered { at, block } });
      }
      for (controller, _) in peeking.iter().filter(|(_, peeking)| peeking.0 == at) {
        shown_to.write(ToClients {
          targets: SendTargets::Single(controller.client),
          message: Opened { at, slots: slots.clone() }
        });
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
      .init_resource::<Coffer>()
      .add_systems(
        PreUpdate,
        (open, rummage).chain().after(ServerSystems::Receive).run_if(authority)
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
