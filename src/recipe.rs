use crate::{block::Block,
            gear::{Gear, Material, heads},
            protocol::{Inventory, Stack}};

#[derive(Clone, Copy)]
pub struct Recipe {
  pub inputs: &'static [(Block, u16)],
  pub output: (Block, u16),
  pub station: Option<Block>,
  pub material: Option<Material>
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

  pub fn learned(&self, spells: &[Block]) -> bool {
    let (output, _) = self.output;
    !output.spell() || spells.contains(&output)
  }

  pub fn ready(
    &self,
    inventory: &Inventory,
    stations: &[Block],
    spells: &[Block]
  ) -> bool {
    self.housed(stations) && self.learned(spells) && self.missing(inventory).is_empty()
  }

  pub fn made(
    &self,
    inventory: &Inventory,
    stations: &[Block],
    spells: &[Block]
  ) -> Option<Inventory> {
    let mut after = inventory.clone();
    let (output, count) = self.output;
    let gear = self.material.map(Gear::forged);
    (self.housed(stations)
      && self.learned(spells)
      && self.inputs.iter().all(|&(block, need)| (0..need).all(|_| after.take(block)))
      && after.gain(Stack { block: output, count, gear }) == 0)
      .then_some(after)
  }
}

const fn hand(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe { inputs, output, station: None, material: None }
}

const fn table(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe { inputs, output, station: Some(Block::CraftingTable), material: None }
}

const fn furnace(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe { inputs, output, station: Some(Block::Furnace), material: None }
}

const BASICS: &[Recipe] = &[
  hand(&[(Block::Log, 1)], (Block::Planks, 4)),
  hand(&[(Block::BirchLog, 1)], (Block::Planks, 4)),
  hand(&[(Block::SpruceLog, 1)], (Block::Planks, 4)),
  hand(&[(Block::PalmLog, 1)], (Block::Planks, 4)),
  hand(&[(Block::Planks, 4)], (Block::CraftingTable, 1)),
  hand(&[(Block::Planks, 1), (Block::CoalOre, 1)], (Block::Torch, 4)),
  hand(&[(Block::Planks, 1), (Block::Charcoal, 1)], (Block::Torch, 4)),
  table(&[(Block::Cobblestone, 8)], (Block::Furnace, 1)),
  table(&[(Block::Planks, 5)], (Block::Boat, 1)),
  table(
    &[(Block::Wool, 12), (Block::Planks, 6), (Block::Bucket, 1)],
    (Block::Balloon, 1)
  ),
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
  furnace(&[(Block::Log, 2)], (Block::Charcoal, 1)),
  furnace(&[(Block::BirchLog, 2)], (Block::Charcoal, 1)),
  furnace(&[(Block::SpruceLog, 2)], (Block::Charcoal, 1)),
  furnace(&[(Block::PalmLog, 2)], (Block::Charcoal, 1)),
  hand(&[(Block::Cobblestone, 1)], (Block::Gravel, 1)),
  hand(&[(Block::Gravel, 1)], (Block::Sand, 1)),
  hand(&[(Block::Snow, 4)], (Block::Ice, 1)),
  hand(&[(Block::RedMushroom, 4)], (Block::RedCap, 1)),
  hand(&[(Block::BrownMushroom, 4)], (Block::BrownCap, 1)),
  hand(&[(Block::RedMushroom, 1), (Block::BrownMushroom, 1)], (Block::MushroomStem, 1)),
  table(&[(Block::Log, 2)], (Block::OakSign, 3)),
  table(&[(Block::BirchLog, 2)], (Block::BirchSign, 3)),
  table(&[(Block::SpruceLog, 2)], (Block::SpruceSign, 3)),
  table(&[(Block::PalmLog, 2)], (Block::PalmSign, 3)),
  table(&[(Block::Planks, 7)], (Block::Ladder, 3)),
  table(&[(Block::Planks, 3), (Block::Wool, 3)], (Block::Bed, 1)),
  table(&[(Block::Planks, 3), (Block::Feather, 6)], (Block::Bed, 1)),
  table(&[(Block::Planks, 3), (Block::Wool, 1)], (Block::FishingRod, 1)),
  hand(&[(Block::Lumen, 1)], (Block::ArcaneDust, 4)),
  hand(&[(Block::BonePile, 1)], (Block::BoneDust, 3)),
  table(&[(Block::Wool, 1), (Block::Cod, 1)], (Block::Candle, 3)),
  table(&[(Block::Wool, 1), (Block::Salmon, 1)], (Block::Candle, 3)),
  furnace(&[(Block::IronOre, 2), (Block::CoalOre, 1)], (Block::Brazier, 1)),
  furnace(&[(Block::Cobblestone, 4), (Block::Clay, 1)], (Block::AncientBrick, 4)),
  hand(&[(Block::ArcaneDust, 1)], (Block::SpellFirebolt, 2)),
  hand(&[(Block::ArcaneDust, 1)], (Block::SpellFrost, 3)),
  hand(&[(Block::ArcaneDust, 1)], (Block::SpellMissile, 2)),
  hand(&[(Block::Candle, 1)], (Block::SpellHeal, 1)),
  hand(&[(Block::LizardClaw, 1)], (Block::SpellHaste, 1)),
  hand(&[(Block::BoneDust, 1)], (Block::SpellStoneskin, 1)),
  hand(&[(Block::Feather, 1)], (Block::SpellLeap, 1)),
  hand(&[(Block::Candle, 1)], (Block::SpellLight, 2)),
  hand(&[(Block::BoneDust, 1)], (Block::SpellThunder, 1)),
  hand(&[(Block::ArcaneDust, 2)], (Block::SpellFireball, 1)),
  hand(&[(Block::ArcaneDust, 3)], (Block::SpellIceStorm, 1)),
  hand(&[(Block::BoneDust, 2)], (Block::SpellAnimate, 1)),
  hand(&[(Block::Candle, 2)], (Block::SpellGuardian, 1)),
  hand(&[(Block::LizardClaw, 2)], (Block::SpellInvisibility, 1))
];

fn forge(tool: Block, material: Material) -> Option<Recipe> {
  let (head, haft) = heads(tool);
  material.ingredient().filter(|_| material.suits(tool)).map(|ingredient| {
    let inputs: Vec<(Block, u16)> = match ingredient {
      Block::Planks => vec![(Block::Planks, head + haft)],
      _ if material.smelted() => {
        vec![(ingredient, head), (Block::Planks, haft), (Block::CoalOre, 1)]
      }
      _ => vec![(ingredient, head), (Block::Planks, haft)]
    };
    Recipe {
      inputs: inputs.leak(),
      output: (tool, 1),
      station: Some(match material.smelted() {
        true => Block::Furnace,
        false => Block::CraftingTable
      }),
      material: Some(material)
    }
  })
}

pub static RECIPES: std::sync::LazyLock<Vec<Recipe>> = std::sync::LazyLock::new(|| {
  BASICS
    .iter()
    .copied()
    .chain(Block::TOOLS.into_iter().flat_map(|tool| {
      Material::ALL.into_iter().filter_map(move |material| forge(tool, material))
    }))
    .collect()
});

pub fn making(block: Block) -> impl Iterator<Item = (usize, &'static Recipe)> {
  RECIPES.iter().enumerate().filter(move |(_, recipe)| recipe.output.0 == block)
}

pub fn using(block: Block) -> impl Iterator<Item = (usize, &'static Recipe)> {
  RECIPES.iter().enumerate().filter(move |(_, recipe)| {
    recipe.inputs.iter().any(|&(input, _)| input == block)
      || recipe.station == Some(block)
  })
}

pub fn craftable(
  block: Block,
  inventory: &Inventory,
  stations: &[Block],
  spells: &[Block]
) -> bool {
  making(block).any(|(_, recipe)| recipe.ready(inventory, stations, spells))
}
