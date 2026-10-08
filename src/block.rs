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
  Bricks
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
  Bricks
}

impl Tile {
  pub const ALL: [Tile; 25] = [
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
    Tile::Bricks
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
      Block::Leaves | Block::Glass => Look::Cutout,
      Block::Water => Look::Liquid,
      _ => Look::Opaque
    }
  }

  pub fn opaque(self) -> bool { self.look() == Look::Opaque }

  pub fn solid(self) -> bool { !matches!(self, Block::Air | Block::Water) }

  pub fn targetable(self) -> bool { self.solid() }

  pub fn breakable(self) -> bool { self.solid() && self != Block::Bedrock }

  pub fn seconds_to_break(self) -> f32 {
    match self {
      Block::Leaves | Block::Glass => 0.3,
      Block::Dirt
      | Block::Grass
      | Block::Sand
      | Block::Gravel
      | Block::Clay
      | Block::Snow => 0.6,
      Block::Log | Block::Planks => 1.5,
      Block::Lamp => 0.5,
      Block::DiamondOre | Block::GoldOre => 3.0,
      _ => 2.0
    }
  }

  pub fn drop(self) -> Block {
    match self {
      Block::Stone => Block::Cobblestone,
      Block::Grass => Block::Dirt,
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
      Block::Bricks => "Bricks"
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
      Block::Bricks => all(Tile::Bricks)
    }
  }
}
