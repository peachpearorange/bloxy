use {crate::{block::{Block, Fluid},
             skin::Skin},
     bevy::{ecs::entity::MapEntities, prelude::*},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

pub const HOTBAR: usize = 9;
pub const BACKPACK: usize = 27;
pub const SLOTS: usize = HOTBAR + BACKPACK;
pub const REACH: f32 = 5.0;
pub const EYE: f32 = 1.62;

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

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct Player {
  pub name: String
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
pub struct Avatar {
  pub at: Vec3,
  pub yaw: f32,
  pub pitch: f32
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
  pub count: u16
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
    .map(|slots| slots.into_iter().chain(std::iter::repeat(None)).take(SLOTS).collect())
}

impl Default for Inventory {
  fn default() -> Self { Inventory { slots: vec![None; SLOTS] } }
}

impl Inventory {
  pub fn count(&self, block: Block) -> u32 {
    self
      .slots
      .iter()
      .flatten()
      .filter(|stack| stack.block == block)
      .map(|stack| u32::from(stack.count))
      .sum()
  }

  pub fn shuffle(&mut self, from: usize, to: usize) {
    if from != to && from < SLOTS && to < SLOTS {
      match (self.slots[from], self.slots[to]) {
        (Some(moved), Some(kept)) if moved.block == kept.block => {
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
    match self
      .slots
      .iter()
      .position(fits)
      .or_else(|| self.slots.iter().position(Option::is_none))
    {
      Some(index) => {
        let slot = &mut self.slots[index];
        *slot = Some(Stack { block, count: slot.map_or(1, |stack| stack.count + 1) });
        true
      }
      None => false
    }
  }

  pub fn take(&mut self, block: Block) -> bool {
    match self
      .slots
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fleece {
  White,
  Silver,
  Grey,
  Black,
  Brown,
  Pink
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Breed {
  Sheep(Fleece),
  Lizard
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Band {
  Trader,
  Pirate,
  Viking
}

impl Band {
  pub fn hostile(self) -> bool { self != Band::Trader }
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
  pub edits: Vec<(IVec3, Block)>
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
      .add_client_message::<Rest>(Channel::Ordered);
  }
}
