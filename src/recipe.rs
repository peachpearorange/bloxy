use crate::{block::Block, protocol::Inventory};

#[derive(Clone, Copy)]
pub struct Recipe {
  pub inputs: &'static [(Block, u16)],
  pub output: (Block, u16),
  pub station: Option<Block>
}

impl Recipe {
  pub fn missing(&self, inventory: &Inventory) -> Vec<(Block, u32)> {
    self
      .inputs
      .iter()
      .map(|&(block, need)| {
        (block, u32::from(need).saturating_sub(inventory.count(block)))
      })
      .filter(|&(_, short)| short > 0)
      .collect()
  }

  pub fn housed(&self, stations: &[Block]) -> bool {
    self.station.is_none_or(|station| stations.contains(&station))
  }

  pub fn ready(&self, inventory: &Inventory, stations: &[Block]) -> bool {
    self.housed(stations) && self.missing(inventory).is_empty()
  }

  pub fn made(&self, inventory: &Inventory, stations: &[Block]) -> Option<Inventory> {
    let mut after = inventory.clone();
    let (output, count) = self.output;
    (self.housed(stations)
      && self.inputs.iter().all(|&(block, need)| (0..need).all(|_| after.take(block)))
      && (0..count).all(|_| after.add(output)))
    .then_some(after)
  }
}

const fn hand(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe { inputs, output, station: None }
}

const fn table(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe { inputs, output, station: Some(Block::CraftingTable) }
}

const fn furnace(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe { inputs, output, station: Some(Block::Furnace) }
}

pub const RECIPES: &[Recipe] = &[
  hand(&[(Block::Log, 1)], (Block::Planks, 4)),
  hand(&[(Block::BirchLog, 1)], (Block::Planks, 4)),
  hand(&[(Block::SpruceLog, 1)], (Block::Planks, 4)),
  hand(&[(Block::PalmLog, 1)], (Block::Planks, 4)),
  hand(&[(Block::Planks, 4)], (Block::CraftingTable, 1)),
  hand(&[(Block::Planks, 1), (Block::CoalOre, 1)], (Block::Torch, 4)),
  table(&[(Block::Cobblestone, 8)], (Block::Furnace, 1)),
  table(&[(Block::Planks, 5)], (Block::Boat, 1)),
  table(&[(Block::Glass, 2), (Block::CopperOre, 1)], (Block::Lamp, 2)),
  furnace(&[(Block::Cobblestone, 8), (Block::CoalOre, 1)], (Block::Stone, 8)),
  furnace(&[(Block::GraniteCobble, 8), (Block::CoalOre, 1)], (Block::Granite, 8)),
  furnace(&[(Block::DioriteCobble, 8), (Block::CoalOre, 1)], (Block::Diorite, 8)),
  furnace(&[(Block::AndesiteCobble, 8), (Block::CoalOre, 1)], (Block::Andesite, 8)),
  furnace(&[(Block::LimestoneCobble, 8), (Block::CoalOre, 1)], (Block::Limestone, 8)),
  furnace(&[(Block::SlateCobble, 8), (Block::CoalOre, 1)], (Block::Slate, 8)),
  furnace(&[(Block::Sand, 4), (Block::CoalOre, 1)], (Block::Glass, 4)),
  furnace(&[(Block::IronOre, 3), (Block::CoalOre, 1)], (Block::Bucket, 1)),
  furnace(&[(Block::Clay, 4), (Block::CoalOre, 1)], (Block::Bricks, 4)),
  hand(&[(Block::Cobblestone, 1)], (Block::Gravel, 1)),
  hand(&[(Block::Gravel, 1)], (Block::Sand, 1)),
  hand(&[(Block::Snow, 4)], (Block::Ice, 1)),
  hand(&[(Block::RedMushroom, 4)], (Block::RedCap, 1)),
  hand(&[(Block::BrownMushroom, 4)], (Block::BrownCap, 1)),
  hand(&[(Block::RedMushroom, 1), (Block::BrownMushroom, 1)], (Block::MushroomStem, 1))
];

pub fn making(block: Block) -> impl Iterator<Item = (usize, &'static Recipe)> {
  RECIPES.iter().enumerate().filter(move |(_, recipe)| recipe.output.0 == block)
}

pub fn using(block: Block) -> impl Iterator<Item = (usize, &'static Recipe)> {
  RECIPES.iter().enumerate().filter(move |(_, recipe)| {
    recipe.inputs.iter().any(|&(input, _)| input == block)
      || recipe.station == Some(block)
  })
}

pub fn craftable(block: Block, inventory: &Inventory, stations: &[Block]) -> bool {
  making(block).any(|(_, recipe)| recipe.ready(inventory, stations))
}
