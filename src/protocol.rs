use {crate::{block::Block, skin::Skin},
     bevy::{ecs::entity::MapEntities, prelude::*},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

pub const HOTBAR: usize = 9;
pub const STACK: u16 = 64;
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

#[derive(Component, Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct Inventory {
  pub slots: [Option<Stack>; HOTBAR]
}

impl Inventory {
  pub fn add(&mut self, block: Block) -> bool {
    let fits = |slot: &Option<Stack>| {
      slot.is_some_and(|stack| stack.block == block && stack.count < STACK)
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
      .replicate::<Hopper>();
  }
}
