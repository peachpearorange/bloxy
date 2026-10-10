use crate::{block::Block, protocol::Inventory};

#[derive(Clone, Copy)]
pub struct Recipe {
  pub inputs: &'static [(Block, u16)],
  pub output: (Block, u16)
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

  pub fn made(&self, inventory: &Inventory) -> Option<Inventory> {
    let mut after = inventory.clone();
    let (output, count) = self.output;
    (self.inputs.iter().all(|&(block, need)| (0..need).all(|_| after.take(block)))
      && (0..count).all(|_| after.add(output)))
    .then_some(after)
  }
}

const fn recipe(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe { inputs, output }
}

pub const RECIPES: &[Recipe] = &[
  recipe(&[(Block::Log, 1)], (Block::Planks, 4)),
  recipe(&[(Block::BirchLog, 1)], (Block::Planks, 4)),
  recipe(&[(Block::SpruceLog, 1)], (Block::Planks, 4)),
  recipe(&[(Block::PalmLog, 1)], (Block::Planks, 4)),
  recipe(&[(Block::Cobblestone, 8), (Block::CoalOre, 1)], (Block::Stone, 8)),
  recipe(&[(Block::GraniteCobble, 8), (Block::CoalOre, 1)], (Block::Granite, 8)),
  recipe(&[(Block::DioriteCobble, 8), (Block::CoalOre, 1)], (Block::Diorite, 8)),
  recipe(&[(Block::AndesiteCobble, 8), (Block::CoalOre, 1)], (Block::Andesite, 8)),
  recipe(&[(Block::LimestoneCobble, 8), (Block::CoalOre, 1)], (Block::Limestone, 8)),
  recipe(&[(Block::SlateCobble, 8), (Block::CoalOre, 1)], (Block::Slate, 8)),
  recipe(&[(Block::Cobblestone, 1)], (Block::Gravel, 1)),
  recipe(&[(Block::Gravel, 1)], (Block::Sand, 1)),
  recipe(&[(Block::Sand, 4), (Block::CoalOre, 1)], (Block::Glass, 4)),
  recipe(&[(Block::Clay, 4), (Block::CoalOre, 1)], (Block::Bricks, 4)),
  recipe(&[(Block::Glass, 2), (Block::CopperOre, 1)], (Block::Lamp, 2)),
  recipe(&[(Block::Snow, 4)], (Block::Ice, 1)),
  recipe(&[(Block::RedMushroom, 4)], (Block::RedCap, 1)),
  recipe(&[(Block::BrownMushroom, 4)], (Block::BrownCap, 1)),
  recipe(&[(Block::RedMushroom, 1), (Block::BrownMushroom, 1)], (Block::MushroomStem, 1))
];

pub fn making(block: Block) -> impl Iterator<Item = (usize, &'static Recipe)> {
  RECIPES.iter().enumerate().filter(move |(_, recipe)| recipe.output.0 == block)
}

pub fn using(block: Block) -> impl Iterator<Item = (usize, &'static Recipe)> {
  RECIPES
    .iter()
    .enumerate()
    .filter(move |(_, recipe)| recipe.inputs.iter().any(|&(input, _)| input == block))
}

pub fn craftable(block: Block, inventory: &Inventory) -> bool {
  making(block).any(|(_, recipe)| recipe.missing(inventory).is_empty())
}
