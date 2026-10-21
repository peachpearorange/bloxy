use {crate::{block::{Block, Fluid},
             gear::{Gear, Wear},
             skin::Skin},
     bevy::{ecs::entity::MapEntities, prelude::*},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

pub const HOTBAR: usize = 10;
pub const BACKPACK: usize = 30;
pub const SLOTS: usize = HOTBAR + BACKPACK;
pub const CURSOR: usize = SLOTS;
pub const WORN: usize = Wear::ALL.len();
pub const WEARING: usize = CURSOR + 1;
pub const HELD: usize = WEARING + WORN;
pub const REACH: f32 = 5.0;
pub const EYE: f32 = 1.8;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
  Solo,
  Host,
  Dedicated,
  Guest
}

impl Role {
  pub fn authority(self) -> bool { self != Role::Guest }
  pub fn plays(self) -> bool { self != Role::Dedicated }
}

pub fn authority(role: Res<Role>) -> bool { role.authority() }

pub fn plays(role: Res<Role>) -> bool { role.plays() }

pub const HUES: [[u8; 3]; 16] = [
  [255, 255, 255],
  [255, 85, 85],
  [255, 150, 60],
  [255, 215, 70],
  [190, 240, 80],
  [85, 220, 110],
  [70, 225, 200],
  [90, 200, 255],
  [100, 140, 255],
  [165, 115, 255],
  [225, 110, 255],
  [255, 120, 185],
  [205, 165, 120],
  [170, 170, 175],
  [255, 190, 160],
  [160, 255, 220]
];

pub fn hue(index: u8) -> Color {
  let [r, g, b] = HUES[usize::from(index) % HUES.len()];
  Color::srgb_u8(r, g, b)
}

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct Player {
  pub name: String,
  pub hue: u8
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Tint(pub u8);

#[derive(Message, Serialize, Deserialize, Clone)]
pub struct Say(pub String);

#[derive(Message, Serialize, Deserialize, Clone)]
pub struct Said {
  pub name: String,
  pub hue: u8,
  pub text: String
}

pub const LONGEST_SAYING: usize = 200;

#[derive(Component, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
pub struct Avatar {
  pub at: Vec3,
  pub yaw: f32,
  pub pitch: f32,
  #[serde(default)]
  pub held: Option<Block>,
  #[serde(default)]
  pub charging: bool,
  #[serde(default)]
  pub hand: u8
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
pub struct Hopper {
  pub at: Vec3,
  pub yaw: f32,
  pub aloft: bool
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Stack {
  pub block: Block,
  pub count: u16,
  #[serde(default)]
  pub gear: Option<Gear>
}

impl Stack {
  pub fn of(block: Block, count: u16) -> Stack { Stack { block, count, gear: None } }

  pub fn fits(&self, slot: usize) -> bool {
    slot < WEARING || Wear::of(self.block) == Wear::ALL.get(slot - WEARING).copied()
  }
}

#[derive(Component, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Inventory {
  #[serde(deserialize_with = "fitted")]
  pub slots: Vec<Option<Stack>>
}

fn fitted<'de, D: serde::Deserializer<'de>>(
  slots: D
) -> Result<Vec<Option<Stack>>, D::Error> {
  Vec::<Option<Stack>>::deserialize(slots)
    .map(|slots| slots.into_iter().chain(std::iter::repeat(None)).take(HELD).collect())
}

impl Default for Inventory {
  fn default() -> Self { Inventory { slots: vec![None; HELD] } }
}

impl Inventory {
  pub fn count(&self, block: Block) -> u32 {
    self.slots[..SLOTS]
      .iter()
      .flatten()
      .filter(|stack| stack.block == block)
      .map(|stack| u32::from(stack.count))
      .sum()
  }

  pub fn worn(&self) -> &[Option<Stack>] { &self.slots[WEARING..HELD] }

  pub fn shuffle(&mut self, from: usize, to: usize) {
    if from != to
      && from < HELD
      && to < HELD
      && self.slots[from].is_none_or(|stack| stack.fits(to))
      && self.slots[to].is_none_or(|stack| stack.fits(from))
    {
      match (self.slots[from], self.slots[to]) {
        (Some(moved), Some(kept))
          if moved.block == kept.block
            && moved.gear == kept.gear
            && kept.count < kept.block.stack() =>
        {
          let shifted = moved.count.min(kept.block.stack().saturating_sub(kept.count));
          self.slots[to] = Some(Stack { count: kept.count + shifted, ..kept });
          self.slots[from] = (moved.count > shifted)
            .then_some(Stack { count: moved.count - shifted, ..moved })
        }
        _ => self.slots.swap(from, to)
      }
    }
  }

  pub fn add(&mut self, block: Block) -> bool {
    let fits = |slot: &Option<Stack>| {
      slot.is_some_and(|stack| stack.block == block && stack.count < block.stack())
    };
    match self.slots[..SLOTS]
      .iter()
      .position(fits)
      .or_else(|| self.slots[..SLOTS].iter().position(Option::is_none))
    {
      Some(index) => {
        let slot = &mut self.slots[index];
        *slot = Some(Stack::of(block, slot.map_or(1, |stack| stack.count + 1)));
        true
      }
      None => false
    }
  }

  pub fn gain(&mut self, stack: Stack) -> u16 {
    match stack.gear {
      Some(_) => match self.slots[..SLOTS].iter().position(Option::is_none) {
        Some(index) => {
          self.slots[index] = Some(stack);
          0
        }
        None => stack.count
      },
      None => (0..stack.count).filter(|_| !self.add(stack.block)).count() as u16
    }
  }

  pub fn take(&mut self, block: Block) -> bool {
    match self.slots[..SLOTS]
      .iter()
      .position(|slot| slot.is_some_and(|stack| stack.block == block))
    {
      Some(index) => {
        let slot = &mut self.slots[index];
        *slot = slot
          .filter(|stack| stack.count > 1)
          .map(|stack| Stack { count: stack.count - 1, ..stack });
        true
      }
      None => false
    }
  }
}

impl Inventory {
  pub fn halve(&mut self, slot: usize) {
    if slot < SLOTS
      && self.slots[CURSOR].is_none()
      && let Some(stack) = self.slots[slot]
    {
      let taken = stack.count.div_ceil(2);
      self.slots[CURSOR] = Some(Stack { count: taken, ..stack });
      self.slots[slot] =
        (stack.count > taken).then_some(Stack { count: stack.count - taken, ..stack })
    }
  }

  pub fn drop_one(&mut self, slot: usize) {
    if slot < SLOTS
      && let Some(held) = self.slots[CURSOR]
      && self.slots[slot].is_none_or(|stack| {
        stack.block == held.block
          && stack.gear == held.gear
          && stack.count < held.block.stack()
      })
    {
      self.slots[slot] = Some(Stack {
        count: self.slots[slot].map_or(0, |stack| stack.count) + 1,
        ..held
      });
      self.slots[CURSOR] =
        (held.count > 1).then_some(Stack { count: held.count - 1, ..held })
    }
  }

  pub fn stow(&mut self) {
    if let Some(held) = self.slots[CURSOR].take() {
      let left = self.gain(held);
      self.slots[CURSOR] = (left > 0).then_some(Stack { count: left, ..held })
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pick {
  Half(u8),
  One(u8),
  Stow
}

#[derive(Component, Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct Visited(pub Vec<IVec2>);

#[derive(Component, Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct Bookmarks(pub Vec<Block>);

impl Bookmarks {
  pub fn toggle(&mut self, block: Block) {
    match self.0.contains(&block) {
      true => self.0.retain(|&marked| marked != block),
      false => self.0.push(block)
    }
  }
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Mark(pub Block);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Scoop(pub IVec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Pour {
  pub at: IVec3,
  pub fluid: Fluid
}

#[derive(Message, Serialize, Deserialize, Clone)]
pub struct Hello {
  pub name: String,
  pub password: String
}

#[derive(Message, Serialize, Deserialize, Clone)]
pub struct Paint(pub Skin);

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
pub enum Verdict {
  Accepted { name: String },
  Refused { reason: String }
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Moved(pub Avatar);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Dig {
  pub at: IVec3
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Put {
  pub at: IVec3,
  pub block: Block
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Travel(pub IVec2);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Attune(pub IVec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Shuffle {
  pub from: u8,
  pub to: u8
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Craft(pub u16);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Picked(pub Pick);

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Vessel {
  pub at: Vec3,
  pub yaw: f32,
  #[entities]
  pub rider: Option<Entity>
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Launch(pub IVec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Board(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Wreck(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Disembark;

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Steer {
  pub at: Vec3,
  pub yaw: f32
}

#[derive(Component, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Sign {
  pub at: IVec3,
  pub yaw: f32,
  pub wood: Block,
  pub text: String
}

#[derive(Message, Serialize, Deserialize, Clone)]
pub struct Inscribe {
  pub at: IVec3,
  pub text: String
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Fleece {
  White,
  Silver,
  Grey,
  Black,
  Brown,
  Pink
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Breed {
  Sheep(Fleece),
  Lizard,
  Spider
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pose {
  Still,
  Walk,
  Run,
  Swim,
  Graze
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Beast {
  pub breed: Breed,
  pub at: Vec3,
  pub yaw: f32,
  pub pose: Pose
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Band {
  Trader,
  Pirate,
  Viking,
  Skeleton,
  Shade,
  Warden,
  Thrall,
  Phantom,
  Imp
}

impl Band {
  pub fn hostile(self) -> bool {
    !matches!(self, Band::Trader | Band::Thrall | Band::Phantom)
  }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Ship {
  pub band: Band,
  pub at: Vec3,
  pub yaw: f32,
  pub aim: f32
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Folk {
  pub band: Band,
  pub at: Vec3,
  pub yaw: f32,
  pub luck: u32,
  pub blows: u8,
  pub aboard: bool
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Health(pub u8);

impl Health {
  pub const FULL: u8 = 20;
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Cannonball(pub Vec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Strike(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Trade {
  #[entities]
  pub trader: Entity,
  pub offer: u8
}

#[derive(Message, Serialize, Deserialize, Clone)]
pub struct Welcome {
  pub seed: u32,
  pub edits: Vec<(IVec3, Block)>,
  pub kept: Vec<(IVec2, Vec<u8>)>
}

#[derive(Message, Serialize, Deserialize, Clone, Copy, MapEntities)]
pub struct Possess(#[entities] pub Entity);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Altered {
  pub at: IVec3,
  pub block: Block
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Teleport(pub Avatar);

#[derive(Message, Serialize, Deserialize, Clone)]
pub struct Notice(pub String);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Rest(pub IVec3);

#[derive(Component, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Debug)]
pub struct Bedside(pub Option<IVec3>);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Knock(pub Vec3);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Toss {
  pub slot: u8,
  pub all: bool
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Loose {
  pub block: Block,
  pub count: u16,
  pub at: Vec3,
  #[serde(default)]
  pub gear: Option<Gear>
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Gathered;

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Open(pub IVec3);

pub struct Protocol;

impl Plugin for Protocol {
  fn build(&self, app: &mut App) {
    app
      .replicate::<Player>()
      .replicate::<Avatar>()
      .replicate::<Inventory>()
      .replicate::<Skin>()
      .add_client_message::<Hello>(Channel::Ordered)
      .add_client_message::<Paint>(Channel::Ordered)
      .add_client_message::<Moved>(Channel::Unreliable)
      .add_client_message::<Dig>(Channel::Ordered)
      .add_client_message::<Put>(Channel::Ordered)
      .add_server_message::<Welcome>(Channel::Ordered)
      .add_mapped_server_message::<Possess>(Channel::Ordered)
      .add_server_message::<Altered>(Channel::Ordered)
      .add_server_message::<Verdict>(Channel::Ordered)
      .replicate::<Visited>()
      .add_client_message::<Travel>(Channel::Ordered)
      .add_server_message::<Teleport>(Channel::Ordered)
      .replicate::<Hopper>()
      .add_client_message::<Attune>(Channel::Ordered)
      .add_client_message::<Shuffle>(Channel::Ordered)
      .add_client_message::<Craft>(Channel::Ordered)
      .replicate::<Vessel>()
      .add_client_message::<Launch>(Channel::Ordered)
      .add_mapped_client_message::<Board>(Channel::Ordered)
      .add_mapped_client_message::<Wreck>(Channel::Ordered)
      .add_client_message::<Disembark>(Channel::Ordered)
      .add_client_message::<Steer>(Channel::Unreliable)
      .replicate::<Bookmarks>()
      .add_client_message::<Mark>(Channel::Ordered)
      .add_client_message::<Scoop>(Channel::Ordered)
      .add_client_message::<Pour>(Channel::Ordered)
      .replicate::<Sign>()
      .add_client_message::<Inscribe>(Channel::Ordered)
      .replicate::<Beast>()
      .replicate::<Ship>()
      .replicate::<Folk>()
      .replicate::<Health>()
      .replicate::<Cannonball>()
      .add_mapped_client_message::<Strike>(Channel::Ordered)
      .add_mapped_client_message::<Trade>(Channel::Ordered)
      .add_server_message::<Notice>(Channel::Ordered)
      .add_client_message::<Rest>(Channel::Ordered)
      .add_server_message::<Knock>(Channel::Ordered)
      .add_client_message::<Toss>(Channel::Ordered)
      .replicate::<Loose>()
      .add_client_message::<Tint>(Channel::Ordered)
      .add_client_message::<Say>(Channel::Ordered)
      .add_server_message::<Said>(Channel::Ordered)
      .add_client_message::<Picked>(Channel::Ordered)
      .replicate::<Gathered>()
      .replicate::<crate::skill::Skills>()
      .replicate::<crate::condition::Conditions>()
      .replicate::<crate::spell::Bolt>()
      .add_client_message::<crate::skill::Learn>(Channel::Ordered)
      .add_client_message::<crate::spell::Invoke>(Channel::Ordered)
      .add_server_message::<crate::spell::Flourish>(Channel::Ordered)
      .add_client_message::<Open>(Channel::Ordered)
      .add_client_message::<crate::stance::Adopt>(Channel::Ordered)
      .add_client_message::<crate::stance::Lunge>(Channel::Ordered)
      .add_client_message::<crate::chest::Rummage>(Channel::Ordered)
      .add_client_message::<crate::chest::Unpack>(Channel::Ordered)
      .add_server_message::<crate::chest::Opened>(Channel::Ordered)
      .replicate::<crate::balloon::Aerostat>()
      .add_client_message::<crate::balloon::Inflate>(Channel::Ordered)
      .add_mapped_client_message::<crate::balloon::Ascend>(Channel::Ordered)
      .add_mapped_client_message::<crate::balloon::Deflate>(Channel::Ordered)
      .add_mapped_client_message::<crate::balloon::Stoke>(Channel::Ordered)
      .add_client_message::<crate::balloon::Drift>(Channel::Unreliable);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_cursor_splits_and_drops_stacks() {
    let mut inventory = Inventory::default();
    inventory.slots[3] = Some(Stack::of(Block::Planks, 9));
    inventory.halve(3);
    assert_eq!(inventory.slots[CURSOR], Some(Stack::of(Block::Planks, 5)));
    assert_eq!(inventory.slots[3], Some(Stack::of(Block::Planks, 4)));
    inventory.drop_one(7);
    inventory.drop_one(3);
    assert_eq!(inventory.slots[7], Some(Stack::of(Block::Planks, 1)));
    assert_eq!(inventory.slots[3], Some(Stack::of(Block::Planks, 5)));
    assert_eq!(inventory.count(Block::Planks), 6);
    inventory.stow();
    assert_eq!(inventory.slots[CURSOR], None);
    assert_eq!(inventory.count(Block::Planks), 9)
  }

  #[test]
  fn only_fitting_gear_is_worn() {
    let mut inventory = Inventory::default();
    inventory.slots[0] = Some(Stack::of(Block::Planks, 3));
    inventory.slots[1] = Some(Stack::of(Block::Boots, 1));
    let feet = WEARING + Wear::ALL.iter().position(|&wear| wear == Wear::Feet).unwrap();
    inventory.shuffle(0, feet);
    assert_eq!(inventory.slots[feet], None);
    inventory.shuffle(1, feet);
    assert_eq!(inventory.slots[feet], Some(Stack::of(Block::Boots, 1)));
    inventory.shuffle(feet, 0);
    assert_eq!(inventory.slots[feet], Some(Stack::of(Block::Boots, 1)));
    inventory.shuffle(feet, 2);
    assert_eq!(inventory.slots[feet], None);
    assert_eq!(inventory.slots[2], Some(Stack::of(Block::Boots, 1)))
  }
}
