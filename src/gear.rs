use strum::VariantArray;
use {crate::{block::Block,
             noise::unit,
             protocol::{Inventory, Stack}},
     serde::{Deserialize, Serialize}};

#[derive(
  Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, VariantArray,
)]
pub enum Material {
  Wood,
  #[serde(alias = "Stone")]
  Greywacke,
  Granite,
  Diorite,
  Andesite,
  Limestone,
  Slate,
  Basalt,
  Copper,
  Tin,
  Iron,
  Gold,
  Diamond,
  Lumen,
  Steel,
  Mithril,
  Glimsteel,
  Verdantine,
  Hexbrass,
  Platinum
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Grit {
  pub hardness: u8,
  pub toughness: u8
}

impl Material {
  pub const ALL: [Material; Material::VARIANTS.len()] =
    *Material::VARIANTS.first_chunk().unwrap();

  pub fn index(self) -> usize { self as usize }

  pub fn name(self) -> &'static str {
    match self {
      Material::Wood => "Wooden",
      Material::Greywacke => "Greywacke",
      Material::Granite => "Granite",
      Material::Diorite => "Diorite",
      Material::Andesite => "Andesite",
      Material::Limestone => "Limestone",
      Material::Slate => "Slate",
      Material::Basalt => "Basalt",
      Material::Copper => "Copper",
      Material::Tin => "Tin",
      Material::Iron => "Iron",
      Material::Gold => "Golden",
      Material::Diamond => "Diamond",
      Material::Lumen => "Lumen",
      Material::Steel => "Ancient Steel",
      Material::Mithril => "Mithril",
      Material::Glimsteel => "Glimsteel",
      Material::Verdantine => "Verdantine",
      Material::Hexbrass => "Hexbrass",
      Material::Platinum => "Platinum"
    }
  }

  pub fn grit(self) -> Grit {
    let (hardness, toughness) = match self {
      Material::Wood => (2, 3),
      Material::Greywacke => (4, 2),
      Material::Granite => (5, 3),
      Material::Diorite => (5, 2),
      Material::Andesite => (4, 3),
      Material::Limestone => (2, 1),
      Material::Slate => (3, 1),
      Material::Basalt => (5, 4),
      Material::Copper => (4, 5),
      Material::Tin => (2, 3),
      Material::Iron => (6, 7),
      Material::Gold => (2, 4),
      Material::Diamond => (9, 4),
      Material::Lumen => (7, 1),
      Material::Steel => (7, 8),
      Material::Mithril => (8, 9),
      Material::Glimsteel => (7, 7),
      Material::Verdantine => (10, 9),
      Material::Hexbrass => (8, 7),
      Material::Platinum => (8, 6)
    };
    Grit { hardness, toughness }
  }

  pub fn ingredient(self) -> Option<Block> {
    match self {
      Material::Wood => Some(Block::Planks),
      Material::Greywacke => Some(Block::GreywackeCobble),
      Material::Granite => Some(Block::GraniteCobble),
      Material::Diorite => Some(Block::DioriteCobble),
      Material::Andesite => Some(Block::AndesiteCobble),
      Material::Limestone => Some(Block::LimestoneCobble),
      Material::Slate => Some(Block::SlateCobble),
      Material::Basalt => Some(Block::Basalt),
      Material::Copper => Some(Block::CopperOre),
      Material::Tin => Some(Block::TinOre),
      Material::Iron => Some(Block::IronOre),
      Material::Gold => Some(Block::GoldOre),
      Material::Diamond => Some(Block::DiamondOre),
      Material::Lumen => Some(Block::Lumen),
      Material::Mithril => Some(Block::MithrilIngot),
      Material::Glimsteel => Some(Block::Glimsteel),
      Material::Verdantine => Some(Block::Verdantine),
      Material::Hexbrass => Some(Block::Hexbrass),
      Material::Platinum => Some(Block::Platinum),
      Material::Steel => None
    }
  }

  pub fn of(ingredient: Block) -> Option<Material> {
    Material::ALL.into_iter().find(|material| material.ingredient() == Some(ingredient))
  }

  pub fn smelted(self) -> bool {
    matches!(self, Material::Copper | Material::Tin | Material::Iron | Material::Gold)
  }

  pub fn palette(self) -> [[f32; 3]; 4] {
    match self {
      Material::Wood => {
        [[0.3, 0.2, 0.11], [0.46, 0.33, 0.19], [0.6, 0.45, 0.27], [0.72, 0.57, 0.36]]
      }
      Material::Greywacke => {
        [[0.27, 0.27, 0.28], [0.42, 0.42, 0.44], [0.55, 0.55, 0.57], [0.68, 0.68, 0.7]]
      }
      Material::Granite => {
        [[0.36, 0.22, 0.19], [0.55, 0.36, 0.31], [0.68, 0.47, 0.41], [0.8, 0.6, 0.53]]
      }
      Material::Diorite => {
        [[0.48, 0.48, 0.47], [0.7, 0.7, 0.69], [0.82, 0.82, 0.81], [0.93, 0.93, 0.92]]
      }
      Material::Andesite => {
        [[0.3, 0.31, 0.31], [0.46, 0.47, 0.47], [0.57, 0.58, 0.58], [0.68, 0.69, 0.69]]
      }
      Material::Limestone => {
        [[0.5, 0.47, 0.38], [0.7, 0.67, 0.56], [0.8, 0.77, 0.66], [0.89, 0.87, 0.78]]
      }
      Material::Slate => {
        [[0.16, 0.18, 0.22], [0.26, 0.29, 0.34], [0.35, 0.38, 0.44], [0.46, 0.5, 0.56]]
      }
      Material::Basalt => {
        [[0.09, 0.09, 0.1], [0.17, 0.17, 0.19], [0.25, 0.25, 0.28], [0.36, 0.36, 0.4]]
      }
      Material::Copper => {
        [[0.4, 0.18, 0.08], [0.66, 0.33, 0.15], [0.84, 0.5, 0.26], [0.97, 0.72, 0.48]]
      }
      Material::Tin => {
        [[0.42, 0.44, 0.47], [0.62, 0.64, 0.67], [0.77, 0.79, 0.82], [0.9, 0.92, 0.94]]
      }
      Material::Iron => {
        [[0.26, 0.26, 0.28], [0.45, 0.45, 0.48], [0.66, 0.67, 0.7], [0.86, 0.87, 0.89]]
      }
      Material::Gold => {
        [[0.55, 0.36, 0.06], [0.82, 0.62, 0.14], [0.96, 0.82, 0.3], [1.0, 0.95, 0.62]]
      }
      Material::Diamond => {
        [[0.1, 0.42, 0.46], [0.24, 0.7, 0.74], [0.48, 0.9, 0.92], [0.82, 1.0, 1.0]]
      }
      Material::Lumen => {
        [[0.04, 0.34, 0.29], [0.1, 0.56, 0.47], [0.24, 0.8, 0.66], [0.6, 1.0, 0.88]]
      }
      Material::Steel => {
        [[0.18, 0.21, 0.27], [0.35, 0.4, 0.48], [0.55, 0.61, 0.7], [0.8, 0.85, 0.92]]
      }
      Material::Mithril => {
        [[0.3, 0.36, 0.5], [0.52, 0.6, 0.78], [0.74, 0.8, 0.95], [0.94, 0.97, 1.0]]
      }
      Material::Glimsteel => {
        [[0.1, 0.24, 0.5], [0.22, 0.46, 0.82], [0.42, 0.7, 0.98], [0.78, 0.92, 1.0]]
      }
      Material::Verdantine => {
        [[0.08, 0.3, 0.08], [0.2, 0.55, 0.16], [0.4, 0.82, 0.3], [0.78, 1.0, 0.62]]
      }
      Material::Hexbrass => {
        [[0.18, 0.08, 0.32], [0.34, 0.18, 0.56], [0.52, 0.34, 0.78], [0.8, 0.66, 0.98]]
      }
      Material::Platinum => {
        [[0.42, 0.44, 0.48], [0.66, 0.69, 0.72], [0.83, 0.85, 0.87], [0.97, 0.98, 1.0]]
      }
    }
  }

  pub fn glows(self) -> bool {
    matches!(
      self,
      Material::Lumen | Material::Mithril | Material::Glimsteel | Material::Verdantine
    )
  }

  pub fn suits(self, tool: Block) -> bool {
    let Grit { hardness, toughness } = self.grit();
    let (least_hardness, least_toughness) = needs(tool);
    hardness >= least_hardness && toughness >= least_toughness
  }

  pub fn speed(self) -> f32 { 1.0 + f32::from(self.grit().hardness) * 0.6 }

  pub fn edge(self) -> u8 {
    let Grit { hardness, toughness } = self.grit();
    (hardness + toughness) / 3
  }
}

pub fn needs(tool: Block) -> (u8, u8) {
  match tool {
    Block::Pickaxe => (4, 2),
    Block::Axe => (2, 2),
    Block::Shovel => (1, 2),
    Block::Sword => (2, 3),
    _ => (u8::MAX, u8::MAX)
  }
}

pub fn heads(tool: Block) -> (u16, u16) {
  match tool {
    Block::Pickaxe | Block::Axe => (3, 2),
    Block::Shovel => (1, 2),
    _ => (2, 1)
  }
}

pub fn dug_with(block: Block) -> Option<Block> {
  match block {
    Block::Dirt
    | Block::Grass
    | Block::Sand
    | Block::Gravel
    | Block::Clay
    | Block::Snow
    | Block::Mycelium => Some(Block::Shovel),
    Block::Log
    | Block::BirchLog
    | Block::SpruceLog
    | Block::PalmLog
    | Block::Planks
    | Block::CraftingTable
    | Block::MushroomStem
    | Block::RedCap
    | Block::BrownCap
    | Block::EmptyChest => Some(Block::Axe),
    wooden if wooden.leafy() || wooden.sign() || wooden.ladder() || wooden.bed() => {
      Some(Block::Axe)
    }
    Block::Cobweb => Some(Block::Sword),
    rock if rock.solid() && rock != Block::Wool => Some(Block::Pickaxe),
    _ => None
  }
}

#[derive(
  Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, VariantArray,
)]
pub enum Wear {
  Head,
  Neck,
  Body,
  Waist,
  Feet,
  Finger
}

impl Wear {
  pub const ALL: [Wear; Wear::VARIANTS.len()] = *Wear::VARIANTS.first_chunk().unwrap();

  pub fn name(self) -> &'static str {
    match self {
      Wear::Head => "Head",
      Wear::Neck => "Neck",
      Wear::Body => "Body",
      Wear::Waist => "Waist",
      Wear::Feet => "Feet",
      Wear::Finger => "Finger"
    }
  }

  pub fn of(block: Block) -> Option<Wear> {
    match block {
      Block::Hat => Some(Wear::Head),
      Block::Amulet => Some(Wear::Neck),
      Block::Mail => Some(Wear::Body),
      Block::Belt => Some(Wear::Waist),
      Block::Boots => Some(Wear::Feet),
      Block::Ring => Some(Wear::Finger),
      _ => None
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Virtue {
  Swift,
  Springy,
  Swimmer,
  Fireproof,
  Frostward,
  Antidote,
  Warding,
  Might,
  Delving,
  Fortune,
  Quickcast,
  Thrift,
  Vigor,
  Mending
}

impl Virtue {
  fn range(self) -> (u8, u8) {
    match self {
      Virtue::Swift | Virtue::Springy => (5, 15),
      Virtue::Swimmer => (10, 25),
      Virtue::Fireproof | Virtue::Frostward | Virtue::Antidote => (15, 50),
      Virtue::Warding => (4, 12),
      Virtue::Might | Virtue::Delving | Virtue::Mending => (10, 30),
      Virtue::Fortune | Virtue::Thrift => (5, 15),
      Virtue::Quickcast => (8, 25),
      Virtue::Vigor => (2, 4)
    }
  }

  pub fn line(self, by: u8) -> String {
    match self {
      Virtue::Swift => format!("Move {by}% faster"),
      Virtue::Springy => format!("Jump {by}% higher"),
      Virtue::Swimmer => format!("Swim {by}% faster"),
      Virtue::Fireproof => format!("Take {by}% less fire damage"),
      Virtue::Frostward => format!("Chills wear off {by}% sooner"),
      Virtue::Antidote => format!("Take {by}% less poison damage"),
      Virtue::Warding => format!("Take {by}% less damage"),
      Virtue::Might => format!("Strike {by}% harder"),
      Virtue::Delving => format!("Dig {by}% faster"),
      Virtue::Fortune => format!("{by}% chance of double ore"),
      Virtue::Quickcast => format!("Spells charge {by}% faster"),
      Virtue::Thrift => format!("{by}% chance a cast is free"),
      Virtue::Vigor => format!("+{by} health"),
      Virtue::Mending => format!("Mend {by}% faster")
    }
  }

  fn title(self) -> &'static str {
    match self {
      Virtue::Swift => "of Haste",
      Virtue::Springy => "of the Hare",
      Virtue::Swimmer => "of the Otter",
      Virtue::Fireproof => "of the Salamander",
      Virtue::Frostward => "of the Hearth",
      Virtue::Antidote => "of Purity",
      Virtue::Warding => "of Warding",
      Virtue::Might => "of Might",
      Virtue::Delving => "of Delving",
      Virtue::Fortune => "of Fortune",
      Virtue::Quickcast => "of Quickening",
      Virtue::Thrift => "of the Miser",
      Virtue::Vigor => "of Vigor",
      Virtue::Mending => "of Mending"
    }
  }
}

fn pool(item: Block) -> &'static [Virtue] {
  match item {
    Block::Pickaxe => &[Virtue::Delving, Virtue::Fortune],
    Block::Shovel => &[Virtue::Delving, Virtue::Swift],
    Block::Axe => &[Virtue::Delving, Virtue::Might],
    Block::Sword => &[Virtue::Might, Virtue::Warding, Virtue::Mending],
    Block::Hat => &[
      Virtue::Fireproof,
      Virtue::Frostward,
      Virtue::Warding,
      Virtue::Quickcast,
      Virtue::Delving
    ],
    Block::Amulet => &[
      Virtue::Vigor,
      Virtue::Mending,
      Virtue::Antidote,
      Virtue::Fireproof,
      Virtue::Thrift,
      Virtue::Quickcast,
      Virtue::Swimmer
    ],
    Block::Mail => &[
      Virtue::Warding,
      Virtue::Vigor,
      Virtue::Fireproof,
      Virtue::Frostward,
      Virtue::Antidote
    ],
    Block::Belt => {
      &[Virtue::Quickcast, Virtue::Might, Virtue::Vigor, Virtue::Delving, Virtue::Swift]
    }
    Block::Boots => &[Virtue::Swift, Virtue::Springy, Virtue::Swimmer, Virtue::Fireproof],
    Block::Ring => &[
      Virtue::Thrift,
      Virtue::Fortune,
      Virtue::Might,
      Virtue::Warding,
      Virtue::Antidote,
      Virtue::Frostward
    ],
    _ => &[]
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Perk {
  pub virtue: Virtue,
  pub by: u8
}

pub const PERKS: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub struct Gear {
  pub material: Option<Material>,
  pub perks: [Option<Perk>; PERKS],
  #[serde(default)]
  pub pack: Option<u32>
}

impl Gear {
  pub fn forged(material: Material) -> Gear {
    Gear { material: Some(material), perks: [None; PERKS], pack: None }
  }

  pub fn perks(&self) -> impl Iterator<Item = Perk> + '_ {
    self.perks.iter().flatten().copied()
  }
}

pub fn title(stack: &Stack) -> String {
  let material = stack.gear.and_then(|gear| gear.material);
  let first = stack.gear.and_then(|gear| gear.perks().next());
  [
    material.map(Material::name),
    Some(stack.block.name()),
    first.map(|perk| perk.virtue.title())
  ]
  .into_iter()
  .flatten()
  .collect::<Vec<_>>()
  .join(" ")
}

pub fn lines(stack: &Stack) -> Vec<String> {
  let material = stack.gear.and_then(|gear| gear.material).map(|material| {
    let Grit { hardness, toughness } = material.grit();
    match stack.block {
      Block::Sword => {
        format!("Hardness {hardness}, toughness {toughness}: +{} damage", material.edge())
      }
      _ => format!(
        "Hardness {hardness}, toughness {toughness}: digs {:.1}x faster",
        material.speed()
      )
    }
  });
  material
    .into_iter()
    .chain(stack.gear.into_iter().flat_map(|gear| {
      gear.perks().map(|perk| perk.virtue.line(perk.by)).collect::<Vec<_>>()
    }))
    .collect()
}

pub fn worn(inventory: &Inventory, hand: usize) -> impl Iterator<Item = Perk> + '_ {
  inventory
    .worn()
    .iter()
    .flatten()
    .chain(
      inventory
        .slots
        .get(hand)
        .filter(|_| hand < crate::protocol::HOTBAR)
        .and_then(Option::as_ref)
        .filter(|stack| stack.block.forged())
    )
    .filter_map(|stack| stack.gear)
    .flat_map(|gear| gear.perks.into_iter().flatten())
}

pub fn found(seed: u32, salt: u32, tier: u8) -> Stack {
  let roll = |step: u32| unit(seed ^ 0x6EA2, salt as i32, step as i32, i32::from(tier));
  let kinds = [
    Block::Pickaxe,
    Block::Axe,
    Block::Shovel,
    Block::Sword,
    Block::Hat,
    Block::Amulet,
    Block::Mail,
    Block::Belt,
    Block::Boots,
    Block::Ring
  ];
  let pick = |step: u32, length: usize| (roll(step) * length as f32) as usize % length;
  let block = kinds[pick(0, kinds.len())];
  let materials: &[Material] = match tier {
    0 | 1 => &[
      Material::Granite,
      Material::Basalt,
      Material::Copper,
      Material::Iron,
      Material::Steel
    ],
    _ => &[Material::Iron, Material::Diamond, Material::Steel, Material::Mithril]
  };
  let material = Block::TOOLS
    .contains(&block)
    .then(|| materials[pick(1, materials.len())])
    .filter(|material| material.suits(block))
    .or(Block::TOOLS.contains(&block).then_some(Material::Steel));
  let worn = Wear::of(block).is_some();
  let count = match (worn, tier >= 2) {
    (true, false) => 1 + usize::from(roll(2) < 0.35),
    (true, true) => 2 + usize::from(roll(2) < 0.4),
    (false, false) => usize::from(roll(2) < 0.5),
    (false, true) => 1 + usize::from(roll(2) < 0.5)
  };
  let virtues = pool(block);
  let chosen = (0..count).fold(Vec::<Perk>::new(), |mut chosen, index| {
    let step = 3 + index as u32 * 2;
    let virtue = (0..virtues.len())
      .map(|offset| virtues[(pick(step, virtues.len()) + offset) % virtues.len()])
      .find(|virtue| !chosen.iter().any(|perk| perk.virtue == *virtue));
    if let Some(virtue) = virtue {
      let (low, high) = virtue.range();
      let lift = roll(step + 1) * 0.75 + f32::from(u8::from(tier >= 2)) * 0.25;
      chosen.push(Perk { virtue, by: low + (f32::from(high - low) * lift).round() as u8 })
    }
    chosen
  });
  let mut perks = [None; PERKS];
  for (slot, perk) in perks.iter_mut().zip(chosen) {
    *slot = Some(perk)
  }
  Stack { block, count: 1, gear: Some(Gear { material, perks, pack: None }) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_tool_has_some_material_strong_enough_and_some_not() {
    for tool in Block::TOOLS {
      assert!(Material::ALL.iter().any(|material| material.suits(tool)), "{tool:?}");
      assert!(
        Material::ALL
          .iter()
          .filter(|material| material.ingredient().is_some())
          .any(|material| !material.suits(tool)),
        "{tool:?}"
      )
    }
  }

  #[test]
  fn found_gear_varies() {
    let found: Vec<Stack> =
      (0..40).map(|salt| found(7, salt, 1 + (salt % 2) as u8)).collect();
    for stack in found.iter().take(12) {
      println!("{} {:?}", title(stack), lines(stack))
    }
    assert!(found.iter().any(|stack| Block::TOOLS.contains(&stack.block)));
    assert!(found.iter().any(|stack| Wear::of(stack.block).is_some()));
    assert!(found.iter().all(|stack| {
      stack
        .gear
        .is_some_and(|gear| Wear::of(stack.block).is_none() || gear.perks().count() > 0)
    }))
  }
}
