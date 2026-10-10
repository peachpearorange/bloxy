use serde::{Deserialize, Serialize};

#[derive(
  Clone,
  Copy,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  Hash,
  Debug,
  Default,
  Serialize,
  Deserialize,
)]
#[repr(u8)]
pub enum Block {
  #[default]
  Air,
  Stone,
  Cobblestone,
  Dirt,
  Grass,
  Sand,
  Gravel,
  Clay,
  Snow,
  Bedrock,
  Log,
  Leaves,
  Planks,
  Glass,
  Water,
  CoalOre,
  IronOre,
  CopperOre,
  TinOre,
  GoldOre,
  DiamondOre,
  Lamp,
  Bricks,
  BirchLog,
  BirchLeaves,
  SpruceLog,
  SpruceLeaves,
  PalmLog,
  PalmLeaves,
  Mycelium,
  MushroomStem,
  RedCap,
  BrownCap,
  Basalt,
  Lava,
  Ice,
  Waystone,
  Poppy,
  Dandelion,
  Cornflower,
  Daisy,
  OakSapling,
  BirchSapling,
  SpruceSapling,
  PalmSapling,
  RedMushroom,
  BrownMushroom,
  Granite,
  GraniteCobble,
  Diorite,
  DioriteCobble,
  Andesite,
  AndesiteCobble,
  Limestone,
  LimestoneCobble,
  Slate,
  SlateCobble,
  WaystoneTop,
  Torch,
  CraftingTable,
  Furnace,
  Boat,
  WaterFlow1,
  WaterFlow2,
  WaterFlow3,
  WaterFlow4,
  WaterFlow5,
  WaterFlow6,
  WaterFlow7,
  LavaFlow1,
  LavaFlow2,
  LavaFlow3,
  Bucket,
  WaterBucket,
  LavaBucket,
  OakSign,
  BirchSign,
  SpruceSign,
  PalmSign
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
  Stone,
  Cobblestone,
  Dirt,
  GrassTop,
  GrassSide,
  Sand,
  Gravel,
  Clay,
  Snow,
  SnowSide,
  Bedrock,
  LogSide,
  LogTop,
  Leaves,
  Planks,
  Glass,
  Water,
  CoalOre,
  IronOre,
  CopperOre,
  TinOre,
  GoldOre,
  DiamondOre,
  Lamp,
  Bricks,
  BirchSide,
  BirchLeaves,
  SpruceSide,
  SpruceLeaves,
  PalmSide,
  PalmLeaves,
  MyceliumTop,
  MyceliumSide,
  Stem,
  RedCap,
  BrownCap,
  Pores,
  Basalt,
  Lava,
  Ice,
  WaystoneSide,
  WaystoneTop,
  Grain,
  Poppy,
  Dandelion,
  Cornflower,
  Daisy,
  OakSapling,
  BirchSapling,
  SpruceSapling,
  PalmSapling,
  RedMushroom,
  BrownMushroom,
  Granite,
  GraniteCobble,
  Diorite,
  DioriteCobble,
  Andesite,
  AndesiteCobble,
  Limestone,
  LimestoneCobble,
  Slate,
  SlateCobble,
  WaystoneStone,
  Flame,
  Torch,
  TableTop,
  TableSide,
  FurnaceTop,
  FurnaceSide,
  Boat,
  Bucket,
  WaterBucket,
  LavaBucket,
  OakSign,
  BirchSign,
  SpruceSign,
  PalmSign
}

impl Tile {
  pub const ALL: [Tile; 78] = [
    Tile::Stone,
    Tile::Cobblestone,
    Tile::Dirt,
    Tile::GrassTop,
    Tile::GrassSide,
    Tile::Sand,
    Tile::Gravel,
    Tile::Clay,
    Tile::Snow,
    Tile::SnowSide,
    Tile::Bedrock,
    Tile::LogSide,
    Tile::LogTop,
    Tile::Leaves,
    Tile::Planks,
    Tile::Glass,
    Tile::Water,
    Tile::CoalOre,
    Tile::IronOre,
    Tile::CopperOre,
    Tile::TinOre,
    Tile::GoldOre,
    Tile::DiamondOre,
    Tile::Lamp,
    Tile::Bricks,
    Tile::BirchSide,
    Tile::BirchLeaves,
    Tile::SpruceSide,
    Tile::SpruceLeaves,
    Tile::PalmSide,
    Tile::PalmLeaves,
    Tile::MyceliumTop,
    Tile::MyceliumSide,
    Tile::Stem,
    Tile::RedCap,
    Tile::BrownCap,
    Tile::Pores,
    Tile::Basalt,
    Tile::Lava,
    Tile::Ice,
    Tile::WaystoneSide,
    Tile::WaystoneTop,
    Tile::Grain,
    Tile::Poppy,
    Tile::Dandelion,
    Tile::Cornflower,
    Tile::Daisy,
    Tile::OakSapling,
    Tile::BirchSapling,
    Tile::SpruceSapling,
    Tile::PalmSapling,
    Tile::RedMushroom,
    Tile::BrownMushroom,
    Tile::Granite,
    Tile::GraniteCobble,
    Tile::Diorite,
    Tile::DioriteCobble,
    Tile::Andesite,
    Tile::AndesiteCobble,
    Tile::Limestone,
    Tile::LimestoneCobble,
    Tile::Slate,
    Tile::SlateCobble,
    Tile::WaystoneStone,
    Tile::Flame,
    Tile::Torch,
    Tile::TableTop,
    Tile::TableSide,
    Tile::FurnaceTop,
    Tile::FurnaceSide,
    Tile::Boat,
    Tile::Bucket,
    Tile::WaterBucket,
    Tile::LavaBucket,
    Tile::OakSign,
    Tile::BirchSign,
    Tile::SpruceSign,
    Tile::PalmSign
  ];

  pub fn index(self) -> u32 { self as u32 }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Fluid {
  Water,
  Lava
}

impl Fluid {
  pub fn flows(self) -> &'static [Block] {
    match self {
      Fluid::Water => &[
        Block::WaterFlow1,
        Block::WaterFlow2,
        Block::WaterFlow3,
        Block::WaterFlow4,
        Block::WaterFlow5,
        Block::WaterFlow6,
        Block::WaterFlow7
      ],
      Fluid::Lava => &[Block::LavaFlow1, Block::LavaFlow2, Block::LavaFlow3]
    }
  }

  pub fn bucket(self) -> Block {
    match self {
      Fluid::Water => Block::WaterBucket,
      Fluid::Lava => Block::LavaBucket
    }
  }

  pub fn source(self) -> Block {
    match self {
      Fluid::Water => Block::Water,
      Fluid::Lava => Block::Lava
    }
  }

  pub fn reach(self) -> u8 { self.flows().len() as u8 }

  pub fn flowing(self, level: u8) -> Block { self.flows()[usize::from(level.max(1)) - 1] }

  pub fn delay(self) -> f32 {
    match self {
      Fluid::Water => 0.25,
      Fluid::Lava => 1.0
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Look {
  Invisible,
  Opaque,
  Cutout,
  Liquid,
  Model,
  Log
}

impl Block {
  pub const ALL: [Block; 79] = [
    Block::Air,
    Block::Stone,
    Block::Cobblestone,
    Block::Dirt,
    Block::Grass,
    Block::Sand,
    Block::Gravel,
    Block::Clay,
    Block::Snow,
    Block::Bedrock,
    Block::Log,
    Block::Leaves,
    Block::Planks,
    Block::Glass,
    Block::Water,
    Block::CoalOre,
    Block::IronOre,
    Block::CopperOre,
    Block::TinOre,
    Block::GoldOre,
    Block::DiamondOre,
    Block::Lamp,
    Block::Bricks,
    Block::BirchLog,
    Block::BirchLeaves,
    Block::SpruceLog,
    Block::SpruceLeaves,
    Block::PalmLog,
    Block::PalmLeaves,
    Block::Mycelium,
    Block::MushroomStem,
    Block::RedCap,
    Block::BrownCap,
    Block::Basalt,
    Block::Lava,
    Block::Ice,
    Block::Waystone,
    Block::Poppy,
    Block::Dandelion,
    Block::Cornflower,
    Block::Daisy,
    Block::OakSapling,
    Block::BirchSapling,
    Block::SpruceSapling,
    Block::PalmSapling,
    Block::RedMushroom,
    Block::BrownMushroom,
    Block::Granite,
    Block::GraniteCobble,
    Block::Diorite,
    Block::DioriteCobble,
    Block::Andesite,
    Block::AndesiteCobble,
    Block::Limestone,
    Block::LimestoneCobble,
    Block::Slate,
    Block::SlateCobble,
    Block::WaystoneTop,
    Block::Torch,
    Block::CraftingTable,
    Block::Furnace,
    Block::Boat,
    Block::WaterFlow1,
    Block::WaterFlow2,
    Block::WaterFlow3,
    Block::WaterFlow4,
    Block::WaterFlow5,
    Block::WaterFlow6,
    Block::WaterFlow7,
    Block::LavaFlow1,
    Block::LavaFlow2,
    Block::LavaFlow3,
    Block::Bucket,
    Block::WaterBucket,
    Block::LavaBucket,
    Block::OakSign,
    Block::BirchSign,
    Block::SpruceSign,
    Block::PalmSign
  ];

  pub const ROCKS: [(Block, Block); 6] = [
    (Block::Stone, Block::Cobblestone),
    (Block::Granite, Block::GraniteCobble),
    (Block::Diorite, Block::DioriteCobble),
    (Block::Andesite, Block::AndesiteCobble),
    (Block::Limestone, Block::LimestoneCobble),
    (Block::Slate, Block::SlateCobble)
  ];

  pub fn rock(self) -> bool { Block::ROCKS.iter().any(|&(stone, _)| stone == self) }

  pub fn item(self) -> bool {
    self != Block::Air && !self.fluid() && self != Block::WaystoneTop
  }

  pub fn tool(self) -> bool {
    matches!(self, Block::Boat | Block::Bucket | Block::WaterBucket | Block::LavaBucket)
  }

  pub fn placeable(self) -> bool { self.item() && !self.tool() }

  pub fn stack(self) -> u16 {
    match self {
      Block::WaterBucket | Block::LavaBucket | Block::Boat => 1,
      Block::Bucket => 16,
      _ => 64
    }
  }

  pub fn carrying(self) -> Option<Fluid> {
    match self {
      Block::WaterBucket => Some(Fluid::Water),
      Block::LavaBucket => Some(Fluid::Lava),
      _ => None
    }
  }

  pub fn waystone(self) -> bool { matches!(self, Block::Waystone | Block::WaystoneTop) }

  pub const SIGNS: [(Block, Block); 4] = [
    (Block::Log, Block::OakSign),
    (Block::BirchLog, Block::BirchSign),
    (Block::SpruceLog, Block::SpruceSign),
    (Block::PalmLog, Block::PalmSign)
  ];

  pub fn sign(self) -> bool { Block::SIGNS.iter().any(|&(_, sign)| sign == self) }

  pub fn station(self) -> bool { matches!(self, Block::CraftingTable | Block::Furnace) }

  pub fn liquid(self) -> Option<(Fluid, u8)> {
    match self {
      Block::Water => Some((Fluid::Water, 0)),
      Block::Lava => Some((Fluid::Lava, 0)),
      flow => [Fluid::Water, Fluid::Lava].into_iter().find_map(|fluid| {
        fluid
          .flows()
          .iter()
          .position(|&level| level == flow)
          .map(|index| (fluid, index as u8 + 1))
      })
    }
  }

  pub fn look(self) -> Look {
    match self {
      Block::Air => Look::Invisible,
      Block::Glass => Look::Cutout,
      leaves if leaves.leafy() => Look::Cutout,
      fluid if fluid.fluid() => Look::Liquid,
      model if model.modelled() || model.waystone() => Look::Model,
      tool if tool.tool() => Look::Invisible,
      Block::Log | Block::BirchLog | Block::SpruceLog | Block::PalmLog => Look::Log,
      _ => Look::Opaque
    }
  }

  pub fn leafy(self) -> bool {
    matches!(
      self,
      Block::Leaves | Block::BirchLeaves | Block::SpruceLeaves | Block::PalmLeaves
    )
  }

  pub const ALL_MODELS: [Block; 10] = [
    Block::Poppy,
    Block::Dandelion,
    Block::Cornflower,
    Block::Daisy,
    Block::OakSapling,
    Block::BirchSapling,
    Block::SpruceSapling,
    Block::PalmSapling,
    Block::RedMushroom,
    Block::BrownMushroom
  ];

  pub fn modelled(self) -> bool { self.plant() || self == Block::Torch || self.sign() }

  pub fn plant(self) -> bool { (Block::Poppy..=Block::BrownMushroom).contains(&self) }

  pub fn opaque(self) -> bool { self.look() == Look::Opaque }

  pub fn fluid(self) -> bool { self.liquid().is_some() }

  pub fn solid(self) -> bool {
    self != Block::Air && !self.tool() && !self.fluid() && !self.modelled()
  }

  pub fn targetable(self) -> bool { self.solid() || self.modelled() }

  pub fn breakable(self) -> bool {
    self.targetable() && self != Block::Bedrock && !self.waystone()
  }

  pub fn seconds_to_break(self) -> f32 {
    match self {
      Block::Glass => 0.3,
      sign if sign.sign() => 0.6,
      model if model.modelled() => 0.05,
      leaves if leaves.leafy() => 0.3,
      Block::Dirt
      | Block::Grass
      | Block::Sand
      | Block::Gravel
      | Block::Clay
      | Block::Snow
      | Block::Mycelium
      | Block::Ice => 0.6,
      Block::MushroomStem | Block::RedCap | Block::BrownCap => 0.8,
      Block::Log
      | Block::BirchLog
      | Block::SpruceLog
      | Block::PalmLog
      | Block::Planks => 1.5,
      Block::Basalt | Block::Granite | Block::Diorite => 2.5,
      Block::Limestone | Block::LimestoneCobble => 1.6,
      Block::Lamp => 0.5,
      Block::CraftingTable => 1.5,
      Block::Furnace => 2.5,
      Block::DiamondOre | Block::GoldOre => 3.0,
      _ => 2.0
    }
  }

  pub fn drop(self) -> Block {
    Block::ROCKS.iter().find(|&&(stone, _)| stone == self).map_or(
      match self {
        Block::Grass | Block::Mycelium => Block::Dirt,
        other => other
      },
      |&(_, cobble)| cobble
    )
  }

  pub fn name(self) -> &'static str {
    match self {
      Block::Air => "Air",
      Block::Stone => "Stone",
      Block::Cobblestone => "Cobblestone",
      Block::Dirt => "Dirt",
      Block::Grass => "Grass",
      Block::Sand => "Sand",
      Block::Gravel => "Gravel",
      Block::Clay => "Clay",
      Block::Snow => "Snow",
      Block::Bedrock => "Bedrock",
      Block::Log => "Log",
      Block::Leaves => "Leaves",
      Block::Planks => "Planks",
      Block::Glass => "Glass",
      Block::Water => "Water",
      Block::CoalOre => "Coal Ore",
      Block::IronOre => "Iron Ore",
      Block::CopperOre => "Copper Ore",
      Block::TinOre => "Tin Ore",
      Block::GoldOre => "Gold Ore",
      Block::DiamondOre => "Diamond Ore",
      Block::Lamp => "Lamp",
      Block::Bricks => "Bricks",
      Block::BirchLog => "Birch Log",
      Block::BirchLeaves => "Birch Leaves",
      Block::SpruceLog => "Spruce Log",
      Block::SpruceLeaves => "Spruce Leaves",
      Block::PalmLog => "Palm Log",
      Block::PalmLeaves => "Palm Leaves",
      Block::Mycelium => "Mycelium",
      Block::MushroomStem => "Mushroom Stem",
      Block::RedCap => "Red Mushroom Cap",
      Block::BrownCap => "Brown Mushroom Cap",
      Block::Basalt => "Basalt",
      Block::Lava => "Lava",
      Block::Ice => "Ice",
      Block::Waystone => "Waystone",
      Block::Poppy => "Poppy",
      Block::Dandelion => "Dandelion",
      Block::Cornflower => "Cornflower",
      Block::Daisy => "Daisy",
      Block::OakSapling => "Oak Sapling",
      Block::BirchSapling => "Birch Sapling",
      Block::SpruceSapling => "Spruce Sapling",
      Block::PalmSapling => "Palm Sapling",
      Block::RedMushroom => "Red Mushroom",
      Block::BrownMushroom => "Brown Mushroom",
      Block::Granite => "Granite",
      Block::GraniteCobble => "Granite Cobblestone",
      Block::Diorite => "Diorite",
      Block::DioriteCobble => "Diorite Cobblestone",
      Block::Andesite => "Andesite",
      Block::AndesiteCobble => "Andesite Cobblestone",
      Block::Limestone => "Limestone",
      Block::LimestoneCobble => "Limestone Cobblestone",
      Block::Slate => "Slate",
      Block::SlateCobble => "Slate Cobblestone",
      Block::WaystoneTop => "Waystone",
      Block::Torch => "Torch",
      Block::CraftingTable => "Crafting Table",
      Block::Furnace => "Furnace",
      Block::Boat => "Boat",
      Block::WaterFlow1 => "Flowing Water",
      Block::WaterFlow2 => "Flowing Water",
      Block::WaterFlow3 => "Flowing Water",
      Block::WaterFlow4 => "Flowing Water",
      Block::WaterFlow5 => "Flowing Water",
      Block::WaterFlow6 => "Flowing Water",
      Block::WaterFlow7 => "Flowing Water",
      Block::LavaFlow1 => "Flowing Lava",
      Block::LavaFlow2 => "Flowing Lava",
      Block::LavaFlow3 => "Flowing Lava",
      Block::Bucket => "Bucket",
      Block::WaterBucket => "Water Bucket",
      Block::LavaBucket => "Lava Bucket",
      Block::OakSign => "Oak Sign",
      Block::BirchSign => "Birch Sign",
      Block::SpruceSign => "Spruce Sign",
      Block::PalmSign => "Palm Sign"
    }
  }

  pub fn tiles(self) -> [Tile; 3] {
    let all = |tile| [tile; 3];
    match self {
      Block::Air | Block::Stone => all(Tile::Stone),
      Block::Cobblestone => all(Tile::Cobblestone),
      Block::Dirt => all(Tile::Dirt),
      Block::Grass => [Tile::GrassTop, Tile::GrassSide, Tile::Dirt],
      Block::Sand => all(Tile::Sand),
      Block::Gravel => all(Tile::Gravel),
      Block::Clay => all(Tile::Clay),
      Block::Snow => [Tile::Snow, Tile::SnowSide, Tile::Dirt],
      Block::Bedrock => all(Tile::Bedrock),
      Block::Log => [Tile::LogTop, Tile::LogSide, Tile::LogTop],
      Block::Leaves => all(Tile::Leaves),
      Block::Planks => all(Tile::Planks),
      Block::Glass => all(Tile::Glass),
      Block::Water => all(Tile::Water),
      Block::CoalOre => all(Tile::CoalOre),
      Block::IronOre => all(Tile::IronOre),
      Block::CopperOre => all(Tile::CopperOre),
      Block::TinOre => all(Tile::TinOre),
      Block::GoldOre => all(Tile::GoldOre),
      Block::DiamondOre => all(Tile::DiamondOre),
      Block::Lamp => all(Tile::Lamp),
      Block::Bricks => all(Tile::Bricks),
      Block::BirchLog => [Tile::LogTop, Tile::BirchSide, Tile::LogTop],
      Block::BirchLeaves => all(Tile::BirchLeaves),
      Block::SpruceLog => [Tile::LogTop, Tile::SpruceSide, Tile::LogTop],
      Block::SpruceLeaves => all(Tile::SpruceLeaves),
      Block::PalmLog => [Tile::LogTop, Tile::PalmSide, Tile::LogTop],
      Block::PalmLeaves => all(Tile::PalmLeaves),
      Block::Mycelium => [Tile::MyceliumTop, Tile::MyceliumSide, Tile::Dirt],
      Block::MushroomStem => all(Tile::Stem),
      Block::RedCap => [Tile::RedCap, Tile::RedCap, Tile::Pores],
      Block::BrownCap => [Tile::BrownCap, Tile::BrownCap, Tile::Pores],
      Block::Basalt => all(Tile::Basalt),
      Block::Lava => all(Tile::Lava),
      Block::Ice => all(Tile::Ice),
      Block::Waystone => [Tile::WaystoneTop, Tile::WaystoneSide, Tile::WaystoneTop],
      Block::Poppy => all(Tile::Poppy),
      Block::Dandelion => all(Tile::Dandelion),
      Block::Cornflower => all(Tile::Cornflower),
      Block::Daisy => all(Tile::Daisy),
      Block::OakSapling => all(Tile::OakSapling),
      Block::BirchSapling => all(Tile::BirchSapling),
      Block::SpruceSapling => all(Tile::SpruceSapling),
      Block::PalmSapling => all(Tile::PalmSapling),
      Block::RedMushroom => all(Tile::RedMushroom),
      Block::BrownMushroom => all(Tile::BrownMushroom),
      Block::Granite => all(Tile::Granite),
      Block::GraniteCobble => all(Tile::GraniteCobble),
      Block::Diorite => all(Tile::Diorite),
      Block::DioriteCobble => all(Tile::DioriteCobble),
      Block::Andesite => all(Tile::Andesite),
      Block::AndesiteCobble => all(Tile::AndesiteCobble),
      Block::Limestone => all(Tile::Limestone),
      Block::LimestoneCobble => all(Tile::LimestoneCobble),
      Block::Slate => all(Tile::Slate),
      Block::SlateCobble => all(Tile::SlateCobble),
      Block::WaystoneTop => {
        [Tile::WaystoneStone, Tile::WaystoneSide, Tile::WaystoneStone]
      }
      Block::Torch => all(Tile::Torch),
      Block::CraftingTable => [Tile::TableTop, Tile::TableSide, Tile::Planks],
      Block::Furnace => [Tile::FurnaceTop, Tile::FurnaceSide, Tile::FurnaceTop],
      Block::Boat => all(Tile::Boat),
      Block::WaterFlow1 => all(Tile::Water),
      Block::WaterFlow2 => all(Tile::Water),
      Block::WaterFlow3 => all(Tile::Water),
      Block::WaterFlow4 => all(Tile::Water),
      Block::WaterFlow5 => all(Tile::Water),
      Block::WaterFlow6 => all(Tile::Water),
      Block::WaterFlow7 => all(Tile::Water),
      Block::LavaFlow1 => all(Tile::Lava),
      Block::LavaFlow2 => all(Tile::Lava),
      Block::LavaFlow3 => all(Tile::Lava),
      Block::Bucket => all(Tile::Bucket),
      Block::WaterBucket => all(Tile::WaterBucket),
      Block::LavaBucket => all(Tile::LavaBucket),
      Block::OakSign => all(Tile::OakSign),
      Block::BirchSign => all(Tile::BirchSign),
      Block::SpruceSign => all(Tile::SpruceSign),
      Block::PalmSign => all(Tile::PalmSign)
    }
  }
}
