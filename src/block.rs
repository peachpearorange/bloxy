use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
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
  Waystone
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
  WaystoneTop
}

impl Tile {
  pub const ALL: [Tile; 42] = [
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
    Tile::WaystoneTop
  ];

  pub fn index(self) -> u32 { self as u32 }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Look {
  Invisible,
  Opaque,
  Cutout,
  Liquid
}

impl Block {
  pub fn look(self) -> Look {
    match self {
      Block::Air => Look::Invisible,
      Block::Glass => Look::Cutout,
      leaves if leaves.leafy() => Look::Cutout,
      Block::Water => Look::Liquid,
      _ => Look::Opaque
    }
  }

  pub fn leafy(self) -> bool {
    matches!(
      self,
      Block::Leaves | Block::BirchLeaves | Block::SpruceLeaves | Block::PalmLeaves
    )
  }

  pub fn opaque(self) -> bool { self.look() == Look::Opaque }

  pub fn fluid(self) -> bool { matches!(self, Block::Water | Block::Lava) }

  pub fn solid(self) -> bool { self != Block::Air && !self.fluid() }

  pub fn targetable(self) -> bool { self.solid() }

  pub fn breakable(self) -> bool {
    self.solid() && !matches!(self, Block::Bedrock | Block::Waystone)
  }

  pub fn seconds_to_break(self) -> f32 {
    match self {
      Block::Glass => 0.3,
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
      Block::Basalt => 2.5,
      Block::Lamp => 0.5,
      Block::DiamondOre | Block::GoldOre => 3.0,
      _ => 2.0
    }
  }

  pub fn drop(self) -> Block {
    match self {
      Block::Stone => Block::Cobblestone,
      Block::Grass | Block::Mycelium => Block::Dirt,
      other => other
    }
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
      Block::Waystone => "Waystone"
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
      Block::Waystone => [Tile::WaystoneTop, Tile::WaystoneSide, Tile::WaystoneTop]
    }
  }
}
