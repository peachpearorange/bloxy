use crate::{arcana::Aspect,
            block::Block,
            gear::{Gear, Material, heads},
            protocol::{Inventory, Stack}};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cost {
  Free,
  Mana(u32),
  Lifeblood(u32),
  Essence(&'static [(Aspect, u16)]),
  Chance(u8)
}

#[derive(Clone, Copy)]
pub struct Recipe {
  pub inputs: &'static [(Block, u16)],
  pub output: (Block, u16),
  pub station: Option<Block>,
  pub material: Option<Material>,
  pub cost: Cost
}

impl Recipe {
  pub fn rite(&self) -> bool {
    self.station.is_some_and(|station| station.arcane() || station == Block::Crusher)
  }

  pub fn how(&self) -> Option<String> {
    let first = self.inputs.first().map_or("it", |&(block, _)| block.name());
    let all: Vec<&str> = self.inputs.iter().map(|&(block, _)| block.name()).collect();
    match (self.station, self.cost) {
      (Some(Block::ManaCistern), Cost::Mana(mana)) => {
        Some(format!("Toss {} into a Mana Cistern: {mana} mana each", all.join(" and ")))
      }
      (Some(Block::Crusher), Cost::Free) => {
        Some(format!("Feed {first} to a powered Crusher"))
      }
      (Some(Block::Crusher), Cost::Chance(percent)) => Some(format!(
        "A powered Crusher sometimes sifts this out of {first} ({percent}% each)"
      )),
      (Some(Block::CrimsonAltar), Cost::Lifeblood(blood)) => {
        Some(format!("Set {first} on a Crimson Altar: {blood} lifeblood"))
      }
      (Some(Block::Cauldron), Cost::Essence(essence)) => Some(format!(
        "Toss {first} into a heated Steeping Cauldron holding {}",
        essence
          .iter()
          .map(|(aspect, count)| format!("{count} {}", aspect.name()))
          .collect::<Vec<_>>()
          .join(", ")
      )),
      (Some(Block::ConfluenceAltar), Cost::Mana(mana)) => Some(format!(
        "Set {first} on a Confluence Altar, the rest on plinths around it: {mana} mana \
         from cisterns nearby"
      )),
      _ => None
    }
  }

  pub fn electric(&self, stations: &[Block]) -> bool {
    self.station == Some(Block::Furnace) && stations.contains(&Block::ElectricFurnace)
  }

  pub fn needs(&self, stations: &[Block]) -> Vec<(Block, u16)> {
    let electric = self.electric(stations);
    self
      .inputs
      .iter()
      .copied()
      .filter(|&(block, _)| !(electric && block == Block::CoalOre))
      .collect()
  }

  pub fn missing(&self, inventory: &Inventory, stations: &[Block]) -> Vec<(Block, u32)> {
    self
      .needs(stations)
      .into_iter()
      .map(|(block, need)| {
        (block, u32::from(need).saturating_sub(inventory.count(block)))
      })
      .filter(|&(_, short)| short > 0)
      .collect()
  }

  pub fn housed(&self, stations: &[Block]) -> bool {
    !self.rite()
      && (self.station.is_none_or(|station| stations.contains(&station))
        || self.electric(stations))
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
    self.housed(stations)
      && self.learned(spells)
      && self.missing(inventory, stations).is_empty()
  }

  pub fn made(
    &self,
    inventory: &Inventory,
    stations: &[Block],
    spells: &[Block]
  ) -> Option<Inventory> {
    let mut after = inventory.clone();
    let (output, count) = self.output;
    let carried = self
      .inputs
      .iter()
      .filter(|&&(block, _)| output.pack() && block.pack())
      .find_map(|&(block, _)| {
        inventory.slots[..crate::protocol::SLOTS]
          .iter()
          .flatten()
          .find(|stack| stack.block == block)
      })
      .and_then(|stack| stack.gear);
    let gear = self.material.map(Gear::forged).or(carried);
    (self.housed(stations)
      && self.learned(spells)
      && self
        .needs(stations)
        .into_iter()
        .all(|(block, need)| (0..need).all(|_| after.take(block)))
      && after.gain(Stack { block: output, count, gear }) == 0)
      .then_some(after)
  }
}

const fn hand(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe { inputs, output, station: None, material: None, cost: Cost::Free }
}

const fn table(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe {
    inputs,
    output,
    station: Some(Block::CraftingTable),
    material: None,
    cost: Cost::Free
  }
}

const fn furnace(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe {
    inputs,
    output,
    station: Some(Block::Furnace),
    material: None,
    cost: Cost::Free
  }
}

const fn bloom_font(inputs: &'static [(Block, u16)], output: (Block, u16)) -> Recipe {
  Recipe {
    inputs,
    output,
    station: Some(Block::BloomFont),
    material: None,
    cost: Cost::Free
  }
}

const fn rite(
  inputs: &'static [(Block, u16)],
  output: (Block, u16),
  station: Block,
  cost: Cost
) -> Recipe {
  Recipe { inputs, output, station: Some(station), material: None, cost }
}

const ARCANA: &[Recipe] = &[
  hand(&[(Block::GlimmerPink, 1), (Block::BoneDust, 1)], (Block::GlimmerPink, 2)),
  hand(&[(Block::GlimmerCyan, 1), (Block::BoneDust, 1)], (Block::GlimmerCyan, 2)),
  hand(&[(Block::GlimmerViolet, 1), (Block::BoneDust, 1)], (Block::GlimmerViolet, 2)),
  hand(&[(Block::GlimmerGold, 1), (Block::BoneDust, 1)], (Block::GlimmerGold, 2)),
  table(&[(Block::Cobblestone, 6), (Block::GlimmerViolet, 1)], (Block::BloomFont, 1)),
  bloom_font(
    &[(Block::GlimmerGold, 2), (Block::Dandelion, 1), (Block::Daisy, 1)],
    (Block::Sunsip, 1)
  ),
  bloom_font(
    &[(Block::GlimmerCyan, 2), (Block::Cornflower, 1), (Block::Cod, 1)],
    (Block::Dewcup, 1)
  ),
  bloom_font(
    &[(Block::GlimmerPink, 2), (Block::Poppy, 1), (Block::CoalOre, 1)],
    (Block::Emberbell, 1)
  ),
  bloom_font(
    &[(Block::Stone, 6), (Block::GlimmerViolet, 2), (Block::Glass, 1)],
    (Block::ManaCistern, 1)
  ),
  bloom_font(&[(Block::Stone, 4), (Block::GlimmerCyan, 1)], (Block::Plinth, 1)),
  table(
    &[(Block::Lumen, 2), (Block::Stone, 4), (Block::Starheart, 1)],
    (Block::ConfluenceAltar, 1)
  ),
  furnace(&[(Block::IronOre, 6), (Block::CoalOre, 1)], (Block::Cauldron, 1)),
  table(
    &[
      (Block::Stone, 6),
      (Block::Furnace, 1),
      (Block::GoldOre, 1),
      (Block::DiamondOre, 1)
    ],
    (Block::CrimsonAltar, 1)
  ),
  table(&[(Block::IronOre, 2), (Block::Planks, 1)], (Block::RitualKnife, 1)),
  table(&[(Block::BareTablet, 1), (Block::Stone, 4)], (Block::CrimsonRune, 2)),
  table(
    &[(Block::BoundTablet, 1), (Block::WaterBucket, 1), (Block::Glass, 2)],
    (Block::WellspringSigil, 1)
  ),
  rite(
    &[(Block::IronOre, 1)],
    (Block::Glimsteel, 1),
    Block::ManaCistern,
    Cost::Mana(1000)
  ),
  rite(
    &[(Block::DiamondOre, 1)],
    (Block::Starheart, 1),
    Block::ManaCistern,
    Cost::Mana(4000)
  ),
  rite(&[(Block::Lumen, 1)], (Block::Tidepearl, 1), Block::ManaCistern, Cost::Mana(2500)),
  rite(
    &[(Block::BoneDust, 1)],
    (Block::ArcaneDust, 1),
    Block::ManaCistern,
    Cost::Mana(800)
  ),
  rite(
    &[(Block::Stone, 1)],
    (Block::BareTablet, 1),
    Block::CrimsonAltar,
    Cost::Lifeblood(1000)
  ),
  rite(
    &[(Block::BareTablet, 1)],
    (Block::BoundTablet, 1),
    Block::CrimsonAltar,
    Cost::Lifeblood(2500)
  ),
  rite(
    &[(Block::DiamondOre, 1)],
    (Block::Heartstone, 1),
    Block::CrimsonAltar,
    Cost::Lifeblood(5000)
  ),
  rite(
    &[(Block::IronOre, 1)],
    (Block::Hexbrass, 1),
    Block::Cauldron,
    Cost::Essence(&[(Aspect::Metal, 4), (Aspect::Magic, 4)])
  ),
  rite(
    &[(Block::Glass, 1)],
    (Block::Lumen, 1),
    Block::Cauldron,
    Cost::Essence(&[(Aspect::Magic, 6), (Aspect::Order, 4)])
  ),
  rite(
    &[(Block::BoneDust, 1)],
    (Block::OakSapling, 1),
    Block::Cauldron,
    Cost::Essence(&[(Aspect::Growth, 8)])
  ),
  rite(
    &[(Block::Charcoal, 1)],
    (Block::CoalOre, 1),
    Block::Cauldron,
    Cost::Essence(&[(Aspect::Fire, 4), (Aspect::Earth, 4)])
  ),
  rite(
    &[(Block::Wool, 1)],
    (Block::Feather, 4),
    Block::Cauldron,
    Cost::Essence(&[(Aspect::Air, 6), (Aspect::Magic, 2)])
  ),
  hand(&[(Block::Moonlace, 1), (Block::BoneDust, 1)], (Block::Moonlace, 2)),
  furnace(&[(Block::PlatinumOre, 1), (Block::CoalOre, 1)], (Block::Platinum, 1)),
  furnace(&[(Block::PlatinumDust, 2), (Block::CoalOre, 1)], (Block::Platinum, 2)),
  table(
    &[(Block::IronOre, 6), (Block::CopperCable, 2), (Block::DiamondOre, 1)],
    (Block::Crusher, 1)
  ),
  rite(&[(Block::PlatinumOre, 1)], (Block::PlatinumDust, 2), Block::Crusher, Cost::Free),
  rite(&[(Block::Cobblestone, 1)], (Block::Gravel, 1), Block::Crusher, Cost::Free),
  rite(&[(Block::Gravel, 1)], (Block::Sand, 1), Block::Crusher, Cost::Free),
  rite(&[(Block::Gravel, 1)], (Block::PlatinumDust, 1), Block::Crusher, Cost::Chance(6)),
  rite(
    &[(Block::GoldOre, 1), (Block::Moonlace, 1)],
    (Block::Platinum, 1),
    Block::ManaCistern,
    Cost::Mana(5000)
  ),
  rite(
    &[(Block::GoldOre, 1)],
    (Block::Platinum, 1),
    Block::CrimsonAltar,
    Cost::Lifeblood(8000)
  ),
  rite(
    &[(Block::GoldOre, 1)],
    (Block::Platinum, 1),
    Block::Cauldron,
    Cost::Essence(&[(Aspect::Metal, 8), (Aspect::Order, 6)])
  ),
  table(
    &[(Block::Wool, 4), (Block::Planks, 2), (Block::IronOre, 1)],
    (Block::Satchel, 1)
  ),
  table(
    &[(Block::Satchel, 1), (Block::IronOre, 4), (Block::Conveyor, 2)],
    (Block::Rucksack, 1)
  ),
  table(
    &[
      (Block::Rucksack, 1),
      (Block::Platinum, 3),
      (Block::ElectricLamp, 1),
      (Block::Conveyor, 2)
    ],
    (Block::ExpeditionPack, 1)
  ),
  table(
    &[
      (Block::Rucksack, 1),
      (Block::Platinum, 1),
      (Block::Hexbrass, 1),
      (Block::ElectricLamp, 1)
    ],
    (Block::ExpeditionPack, 1)
  ),
  rite(
    &[(Block::Glimsteel, 1), (Block::Tidepearl, 3)],
    (Block::LesserRift, 1),
    Block::ConfluenceAltar,
    Cost::Mana(6000)
  ),
  rite(
    &[(Block::Glimsteel, 1), (Block::Tidepearl, 1), (Block::BoundTablet, 1)],
    (Block::LesserRift, 1),
    Block::ConfluenceAltar,
    Cost::Mana(3000)
  ),
  rite(
    &[(Block::LesserRift, 1), (Block::Platinum, 2), (Block::Starheart, 1)],
    (Block::GreaterRift, 1),
    Block::ConfluenceAltar,
    Cost::Mana(10000)
  ),
  rite(
    &[
      (Block::LesserRift, 1),
      (Block::Platinum, 1),
      (Block::Starheart, 1),
      (Block::Hexbrass, 1)
    ],
    (Block::GreaterRift, 1),
    Block::ConfluenceAltar,
    Cost::Mana(6000)
  ),
  rite(
    &[(Block::Glimsteel, 1), (Block::Starheart, 1), (Block::Tidepearl, 1)],
    (Block::Verdantine, 1),
    Block::ConfluenceAltar,
    Cost::Mana(6000)
  ),
  rite(
    &[
      (Block::Verdantine, 1),
      (Block::Hexbrass, 1),
      (Block::BoundTablet, 1),
      (Block::Heartstone, 1)
    ],
    (Block::MithrilIngot, 1),
    Block::ConfluenceAltar,
    Cost::Mana(12000)
  )
];

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
  hand(&[(Block::LizardClaw, 2)], (Block::SpellInvisibility, 1)),
  furnace(
    &[(Block::CopperOre, 1), (Block::Glass, 2), (Block::CoalOre, 1)],
    (Block::CopperPipe, 4)
  ),
  furnace(
    &[(Block::CopperPipe, 4), (Block::IronOre, 1), (Block::CoalOre, 1)],
    (Block::IronPipe, 4)
  ),
  furnace(
    &[(Block::IronPipe, 4), (Block::GoldOre, 1), (Block::CoalOre, 1)],
    (Block::GoldPipe, 4)
  ),
  table(&[(Block::GoldPipe, 4), (Block::DiamondOre, 1)], (Block::DiamondPipe, 4)),
  table(
    &[(Block::CopperPipe, 2), (Block::Glass, 4), (Block::CopperOre, 2)],
    (Block::Tank, 1)
  ),
  table(&[(Block::CopperOre, 1), (Block::Wool, 1)], (Block::CopperCable, 8)),
  table(
    &[(Block::CopperCable, 2), (Block::IronOre, 2), (Block::TinOre, 2)],
    (Block::Battery, 1)
  ),
  table(
    &[(Block::CopperPipe, 2), (Block::IronOre, 3), (Block::CopperCable, 2)],
    (Block::ElectricPump, 1)
  ),
  table(
    &[
      (Block::IronOre, 8),
      (Block::CopperPipe, 4),
      (Block::CopperCable, 2),
      (Block::Planks, 4)
    ],
    (Block::Pumpjack, 1)
  ),
  table(
    &[
      (Block::CopperPipe, 4),
      (Block::IronOre, 6),
      (Block::Bricks, 4),
      (Block::Glass, 2),
      (Block::CopperCable, 2)
    ],
    (Block::Refinery, 1)
  ),
  table(
    &[(Block::Planks, 8), (Block::CopperPipe, 1), (Block::IronOre, 1)],
    (Block::Fermenter, 1)
  ),
  table(
    &[
      (Block::Bricks, 6),
      (Block::CopperOre, 4),
      (Block::CopperPipe, 2),
      (Block::IronOre, 2)
    ],
    (Block::SteamEngine, 1)
  ),
  table(
    &[
      (Block::IronOre, 4),
      (Block::CopperCable, 4),
      (Block::CopperPipe, 2),
      (Block::Glass, 1)
    ],
    (Block::BiofuelGenerator, 1)
  ),
  table(
    &[
      (Block::IronOre, 8),
      (Block::CopperCable, 4),
      (Block::CopperPipe, 2),
      (Block::GoldOre, 1)
    ],
    (Block::DieselGenerator, 1)
  ),
  table(
    &[
      (Block::Basalt, 6),
      (Block::IronOre, 4),
      (Block::CopperPipe, 2),
      (Block::CopperCable, 4)
    ],
    (Block::GeothermalGenerator, 1)
  ),
  table(
    &[(Block::Glass, 4), (Block::TinOre, 2), (Block::CopperCable, 2)],
    (Block::SolarPanel, 1)
  ),
  table(
    &[(Block::IronOre, 6), (Block::Planks, 6), (Block::Wool, 4), (Block::CopperCable, 2)],
    (Block::WindTurbine, 1)
  ),
  table(
    &[(Block::Furnace, 1), (Block::CopperCable, 4), (Block::IronOre, 2)],
    (Block::ElectricFurnace, 1)
  ),
  table(
    &[(Block::Glass, 2), (Block::CopperCable, 1), (Block::IronOre, 1)],
    (Block::ElectricLamp, 2)
  ),
  table(&[(Block::CopperPipe, 1), (Block::IronOre, 1)], (Block::FluidDrain, 1)),
  table(&[(Block::IronOre, 2), (Block::Planks, 1)], (Block::ItemDrain, 1)),
  table(&[(Block::Glass, 2), (Block::IronOre, 1)], (Block::ItemPipe, 4)),
  table(
    &[(Block::IronOre, 2), (Block::Wool, 2), (Block::Planks, 1)],
    (Block::Conveyor, 8)
  )
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
      material: Some(material),
      cost: Cost::Free
    }
  })
}

pub static RECIPES: std::sync::LazyLock<Vec<Recipe>> = std::sync::LazyLock::new(|| {
  BASICS
    .iter()
    .chain(ARCANA)
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn platinum_has_a_route_through_every_school() {
    let stations: Vec<Option<Block>> =
      making(Block::Platinum).map(|(_, recipe)| recipe.station).collect();
    for station in
      [Block::Furnace, Block::ManaCistern, Block::CrimsonAltar, Block::Cauldron]
    {
      assert!(stations.contains(&Some(station)), "no platinum from {station:?}")
    }
    assert!(
      making(Block::PlatinumDust)
        .any(|(_, recipe)| recipe.station == Some(Block::Crusher))
    )
  }

  #[test]
  fn an_upgraded_pack_keeps_its_contents() {
    let mut inventory = Inventory::default();
    inventory.slots[0] = Some(Stack {
      block: Block::Satchel,
      count: 1,
      gear: Some(Gear { pack: Some(7), ..Gear::default() })
    });
    inventory.slots[1] = Some(Stack::of(Block::IronOre, 4));
    inventory.slots[2] = Some(Stack::of(Block::Conveyor, 2));
    let (_, recipe) = making(Block::Rucksack).next().unwrap();
    let after = recipe.made(&inventory, &[Block::CraftingTable], &[]).unwrap();
    let pack = after.slots.iter().flatten().find(|stack| stack.block == Block::Rucksack);
    assert_eq!(pack.and_then(|stack| stack.gear).and_then(|gear| gear.pack), Some(7));
    assert_eq!(after.count(Block::Satchel), 0)
  }

  #[test]
  fn rituals_cannot_be_crafted_by_hand() {
    let mut inventory = Inventory::default();
    inventory.slots[0] = Some(Stack::of(Block::GoldOre, 4));
    let everywhere =
      [Block::ManaCistern, Block::CrimsonAltar, Block::Cauldron, Block::Crusher];
    assert!(
      making(Block::Platinum)
        .filter(|(_, recipe)| recipe.rite())
        .all(|(_, recipe)| recipe.made(&inventory, &everywhere, &[]).is_none())
    )
  }
}
