use crate::{block::Block,
            glyph::Glyph,
            machine::{self, Design, Liquid},
            recipe::{self, Cost}};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Chip {
  Liquid(Liquid),
  Made(f32),
  Drawn(f32),
  Stored(f32),
  Fuel(Option<f32>),
  Item(Block),
  Sign(Glyph)
}

fn poured(block: Block) -> Option<Liquid> {
  match block {
    Block::ElectricPump => Some(Liquid::Water),
    Block::Pumpjack => Some(Liquid::Crude),
    Block::Refinery => Some(Liquid::Diesel),
    Block::Fermenter => Some(Liquid::Biofuel),
    Block::CrimsonAltar => Some(Liquid::Lifeblood),
    bloomer if bloomer.bloomer() => Some(Liquid::Mana),
    _ => None
  }
}

fn fed(block: Block) -> Vec<Chip> {
  match block {
    Block::SteamEngine | Block::Emberbell => vec![Chip::Fuel(None)],
    Block::Fermenter => vec![Chip::Item(Block::Leaves)],
    Block::Crusher => vec![Chip::Item(Block::Cobblestone)],
    Block::Pumpjack => vec![Chip::Item(Block::OilSeep)],
    Block::ElectricPump | Block::Dewcup => vec![Chip::Liquid(Liquid::Water)],
    Block::SolarPanel | Block::Sunsip => vec![Chip::Sign(Glyph::Sun)],
    Block::WindTurbine => vec![Chip::Sign(Glyph::Wing)],
    Block::CrimsonAltar => vec![Chip::Sign(Glyph::Heart)],
    Block::Cauldron => vec![Chip::Fuel(None)],
    _ => vec![]
  }
}

pub fn flow(block: Block) -> (Vec<Chip>, Vec<Chip>) {
  let Design { intake, draws, makes, stores, .. } = machine::design(block);
  let inputs: Vec<Chip> = intake
    .into_iter()
    .flat_map(|(liquids, _)| liquids.iter().map(|&liquid| Chip::Liquid(liquid)))
    .filter(|_| poured(block).is_none() || block == Block::Refinery)
    .chain(fed(block))
    .chain((draws > 0.0).then_some(Chip::Drawn(draws)))
    .fold(vec![], |mut chips, chip| {
      if !chips.contains(&chip) {
        chips.push(chip)
      }
      chips
    });
  let outputs: Vec<Chip> = poured(block)
    .map(Chip::Liquid)
    .into_iter()
    .chain((makes > 0.0).then_some(Chip::Made(makes)))
    .chain((stores > 0.0 && draws == 0.0).then_some(Chip::Stored(stores)))
    .chain(
      matches!(block, Block::ElectricLamp | Block::ElectricLampLit)
        .then_some(Chip::Sign(Glyph::Sun))
    )
    .chain((block == Block::ElectricFurnace).then_some(Chip::Item(Block::Furnace)))
    .collect();
  (inputs, outputs)
}

pub fn feeds(block: Block) -> Vec<Chip> {
  crate::balloon::burns(block)
    .map(|seconds| Chip::Fuel(Some(seconds)))
    .into_iter()
    .chain(machine::biomass(block).map(|_| Chip::Item(Block::Fermenter)))
    .chain(machine::crushable(block).then_some(Chip::Item(Block::Crusher)))
    .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Price {
  Free,
  Mana,
  Lifeblood,
  Essence,
  Chance
}

impl Price {
  fn of(cost: Cost) -> Price {
    match cost {
      Cost::Free => Price::Free,
      Cost::Mana(_) => Price::Mana,
      Cost::Lifeblood(_) => Price::Lifeblood,
      Cost::Essence(_) => Price::Essence,
      Cost::Chance(_) => Price::Chance
    }
  }
}

pub struct Making {
  pub station: Option<Block>,
  pub price: Price,
  pub products: Vec<Block>
}

pub fn makes(block: Block, known: &[Block]) -> Vec<Making> {
  recipe::using(block)
    .map(|(_, recipe)| recipe)
    .filter(|recipe| !recipe.output.0.spell() || known.contains(&recipe.output.0))
    .fold(Vec::<Making>::new(), |mut groups, recipe| {
      let price = Price::of(recipe.cost);
      let product = recipe.output.0;
      match groups
        .iter_mut()
        .find(|group| group.station == recipe.station && group.price == price)
      {
        Some(group) if !group.products.contains(&product) => group.products.push(product),
        Some(_) => (),
        None => {
          groups.push(Making { station: recipe.station, price, products: vec![product] })
        }
      }
      groups
    })
}

pub fn tagline(block: Block) -> Option<&'static str> {
  match block {
    Block::Waystone | Block::WaystoneTop => Some("Remembers you, and carries you back."),
    Block::CraftingTable => Some("More can be made at a bench."),
    Block::Furnace => Some("Fire changes what it holds."),
    Block::Torch => Some("Keeps the dark back."),
    Block::Boat => Some("Rows across open water."),
    Block::Bucket => Some("Holds one source of water or lava."),
    Block::WaterBucket | Block::LavaBucket => Some("Pour it where it is needed."),
    Block::OakSign | Block::BirchSign | Block::SpruceSign | Block::PalmSign => {
      Some("Leaves words for whoever comes next.")
    }
    Block::Ladder => Some("Climb straight up."),
    Block::Bed => Some("Wake here after you fall."),
    Block::FishingRod => Some("Cast, wait for the dip, then fight."),
    Block::Lumen => Some("Light grown in the dark."),
    Block::Brazier => Some("A fire that needs no feeding."),
    Block::Urn => Some("Something rattles inside."),
    Block::Chest | Block::EmptyChest => Some("Holds what you cannot carry."),
    Block::Balloon => Some("Warm air rises. Feed the flame."),
    Block::OilSeep => Some("Black wealth seeps up from below."),
    pipe if pipe.pipe() => Some("Carries liquid. Better metal flows faster."),
    Block::Tank => Some("Keeps liquid."),
    Block::CopperCable => Some("Joins what makes power to what uses it."),
    Block::Battery => Some("Keeps power for later."),
    Block::ElectricPump => Some("Draws from a source beside it."),
    Block::Pumpjack => Some("Set it on an oil seep."),
    Block::Refinery => Some("Crude in, diesel out."),
    Block::Fermenter => Some("Rots green things into fuel."),
    Block::SteamEngine => Some("Burns, boils, turns."),
    Block::BiofuelGenerator | Block::DieselGenerator => {
      Some("Burns what it is fed for power.")
    }
    Block::GeothermalGenerator => Some("Power from lava beside it."),
    Block::SolarPanel => Some("Power from open sky by day."),
    Block::WindTurbine => Some("Higher is windier."),
    Block::ElectricFurnace => Some("Smelts with power instead of coal."),
    Block::ElectricLamp | Block::ElectricLampLit => Some("Light from power."),
    Block::FluidDrain => Some("Lets liquid out of what it touches."),
    Block::PipedDrain => Some("Pushes what it drains into pipes."),
    Block::ItemDrain => Some("Lets things out of a chest."),
    Block::ItemPipe => Some("Carries things one at a time."),
    belt if belt.conveyor() => Some("Moves what lies on it."),
    glimmer if glimmer.glimmer() => Some("Hums with something waiting."),
    Block::BloomFont => Some("Wakes flowers that give mana."),
    Block::Sunsip => Some("Drinks sunlight."),
    Block::Dewcup => Some("Sips from water beside it."),
    Block::Emberbell => Some("Eats what burns."),
    Block::ManaCistern => Some("Gathers mana. What falls in may change."),
    Block::Cauldron => Some("Over fire, water draws out what things are."),
    Block::CrimsonAltar => Some("It asks for blood."),
    Block::Plinth => Some("Set offerings around an altar."),
    Block::ConfluenceAltar => Some("Many become one."),
    Block::CrimsonRune => Some("Strengthens an altar nearby."),
    Block::Glimsteel => Some("Iron that drank mana."),
    Block::Starheart => Some("A diamond full of night."),
    Block::Tidepearl => Some("Light that remembers the sea."),
    Block::Verdantine => Some("Metal that grew."),
    Block::Hexbrass => Some("Brewed, not mined."),
    Block::MithrilIngot => Some("Light, and nearly unbreakable."),
    Block::BareTablet => Some("Waiting to be bound."),
    Block::BoundTablet => Some("Holds a working."),
    Block::Heartstone => Some("It beats."),
    Block::RitualKnife => Some("It costs you. It pays the altar."),
    Block::WellspringSigil => Some("Water without end."),
    Block::Moonlace => Some("Blooms only in cold heights."),
    Block::Crusher => Some("Grinds things finer."),
    pack if pack.pack() => Some("A bag of your own."),
    rift if rift.rift() => Some("The same pocket, wherever you are."),
    Block::Kelp => Some("Climbs toward the light."),
    Block::GlowPolyp => Some("A small light in the deep."),
    Block::VentWorms => Some("Live where the sea floor boils."),
    Block::Clam => Some("Sometimes grows a pearl."),
    Block::Abyssite => Some("Stone from where light never reaches."),
    Block::VentStone => Some("Warm from below."),
    Block::NoduleOre => Some("Grown on the deep floor over ages."),
    Block::PrecursorAlloy => Some("Whoever made this knew more than we do."),
    Block::PrecursorGlass => Some("Still holding back the sea."),
    Block::AetherConduit => Some("Something still runs through it."),
    Block::AetherLamp => Some("It has been burning all this time."),
    Block::AetherCell => Some("A power that never faded."),
    Block::AlloyScrap => Some("Pieces of something made."),
    Block::ResonantCrystal => Some("It rings when the deep speaks."),
    Block::Datacore => Some("It remembers. You cannot read it."),
    Block::JellyGel => Some("Faintly alive."),
    Block::BioLab => Some("Cultures life from the sea."),
    Block::Glowkelp => Some("Kelp taught to live on land."),
    Block::Submarine => Some("A window under the sea."),
    Block::SonarModule => Some("Hears shapes in the dark."),
    Block::DeepSonar => Some("Hears farther. Hears things calling."),
    Block::DrillModule => Some("Bores through rock."),
    Block::PlasmaDrill => Some("Bores through anything."),
    Block::Floodlight => Some("Light ahead."),
    Block::Thruster => Some("Faster."),
    Block::Hydrojet => Some("Much faster."),
    Block::HullPlating => Some("Deeper."),
    Block::ResonantPlating => Some("As deep as there is."),
    Block::SampleClaw => Some("Gathers life instead of shredding it."),
    Block::TorpedoBay => Some("Fires torpedoes."),
    Block::PowerCell => Some("Lasts longer."),
    _ => None
  }
}
