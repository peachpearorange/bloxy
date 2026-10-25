use {crate::{authority::Controller,
             block::Block,
             death::Dying,
             gear::Material,
             haunt::Blamed,
             hud::{SHADE, forged_rect},
             menu::{EDGE, FAINT, INK, LIT, Menu, Tab, words},
             player::Pilot,
             protocol::{Band, Beast, Breed, Fleece, Folk, Inventory, Notice, Visited,
                        authority, plays},
             skill::{Earned, Skills},
             stream::Palette},
     bevy::{input::mouse::AccumulatedMouseScroll, prelude::*,
            ui::RelativeCursorPosition, window::PrimaryWindow},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

const SPACING: f32 = 92.0;
const NODE: f32 = 48.0;
const DRAG: f32 = 4.0;
const ZOOMS: (f32, f32) = (0.3, 1.8);
const REVIEW_EVERY: f32 = 1.0;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Prey {
  Beast(Breed),
  Folk(Band),
  Bird
}

impl Prey {
  fn name(self) -> &'static str {
    match self {
      Prey::Beast(Breed::Sheep(_)) => "sheep",
      Prey::Beast(Breed::Lizard) => "water lizards",
      Prey::Beast(Breed::Spider) => "cave spiders",
      Prey::Beast(Breed::Glowray) => "glowrays",
      Prey::Beast(Breed::Lanternjelly) => "lanternjellies",
      Prey::Beast(Breed::Snapjaw) => "snapjaws",
      Prey::Beast(Breed::Brinewyrm) => "brinewyrms",
      Prey::Beast(Breed::Shellback) => "shellbacks",
      Prey::Folk(Band::Pirate) => "pirates",
      Prey::Folk(Band::Viking) => "vikings",
      Prey::Folk(Band::Skeleton) => "skeletons",
      Prey::Folk(Band::Shade) => "shades",
      Prey::Folk(Band::Imp) => "imps",
      Prey::Folk(Band::Warden) => "Bone Wardens",
      Prey::Folk(Band::Sentinel) => "sentinels",
      Prey::Folk(Band::Custodian) => "custodians",
      Prey::Folk(_) => "folk",
      Prey::Bird => "birds"
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub enum Task {
  Have(Block, u32),
  Forge(Block, Material),
  Slay(&'static [Prey], u16),
  Attune(usize),
  Level(u32)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Chapter {
  Basics,
  Tools,
  Voyages,
  Hunts,
  Arcana,
  Industry,
  Depths
}

impl Chapter {
  fn colour(self) -> Color {
    match self {
      Chapter::Basics => Color::srgb(0.95, 0.8, 0.4),
      Chapter::Tools => Color::srgb(0.75, 0.78, 0.82),
      Chapter::Voyages => Color::srgb(0.35, 0.8, 0.95),
      Chapter::Hunts => Color::srgb(0.95, 0.38, 0.32),
      Chapter::Arcana => Color::srgb(0.75, 0.5, 1.0),
      Chapter::Industry => Color::srgb(0.9, 0.55, 0.25),
      Chapter::Depths => Color::srgb(0.25, 0.95, 0.8)
    }
  }

  fn title(self) -> &'static str {
    match self {
      Chapter::Basics => "Basics",
      Chapter::Tools => "Tools & Ores",
      Chapter::Voyages => "Voyages",
      Chapter::Hunts => "Hunts",
      Chapter::Arcana => "Arcana",
      Chapter::Industry => "Industry",
      Chapter::Depths => "Depths"
    }
  }
}

pub struct Goal {
  pub title: &'static str,
  pub about: &'static str,
  pub task: Task,
  pub after: &'static [u16],
  pub at: (f32, f32),
  pub xp: u32,
  pub chapter: Chapter
}

const SEA_MONSTERS: &[Prey] = &[Prey::Beast(Breed::Snapjaw)];
const RAIDERS: &[Prey] = &[Prey::Folk(Band::Pirate), Prey::Folk(Band::Viking)];
const SENTINELS: &[Prey] = &[Prey::Folk(Band::Sentinel)];
const CUSTODIANS: &[Prey] = &[Prey::Folk(Band::Custodian)];

const fn goal(
  title: &'static str,
  about: &'static str,
  task: Task,
  after: &'static [u16],
  at: (f32, f32),
  xp: u32,
  chapter: Chapter
) -> Goal {
  Goal { title, about, task, after, at, xp, chapter }
}

pub const GOALS: [Goal; 77] = [
  goal(
    "Timber",
    "Punch a tree for its log.",
    Task::Have(Block::Log, 1),
    &[],
    (0.0, 0.0),
    10,
    Chapter::Basics
  ),
  goal(
    "Woodworking",
    "Turn logs into planks.",
    Task::Have(Block::Planks, 8),
    &[0],
    (1.0, 0.0),
    10,
    Chapter::Basics
  ),
  goal(
    "Workbench",
    "Build a crafting table: many recipes need one nearby.",
    Task::Have(Block::CraftingTable, 1),
    &[1],
    (2.0, 0.0),
    15,
    Chapter::Basics
  ),
  goal(
    "Light the Dark",
    "Make torches from planks and coal or charcoal.",
    Task::Have(Block::Torch, 8),
    &[2],
    (3.0, -1.0),
    15,
    Chapter::Basics
  ),
  goal(
    "Stone Age",
    "Gather greywacke cobble.",
    Task::Have(Block::GreywackeCobble, 16),
    &[2],
    (3.0, 1.0),
    15,
    Chapter::Basics
  ),
  goal(
    "Furnace",
    "Build a furnace from greywacke cobble at the table.",
    Task::Have(Block::Furnace, 1),
    &[4],
    (4.0, 1.0),
    20,
    Chapter::Basics
  ),
  goal(
    "Glazier",
    "Smelt sand into glass.",
    Task::Have(Block::Glass, 8),
    &[5],
    (5.0, 0.0),
    20,
    Chapter::Basics
  ),
  goal(
    "Smooth Rock",
    "Smelt greywacke cobble back into greywacke.",
    Task::Have(Block::Greywacke, 16),
    &[5],
    (5.0, 1.0),
    20,
    Chapter::Basics
  ),
  goal(
    "Mason",
    "Cut greywacke into bricks at the table.",
    Task::Have(Block::GreywackeBricks, 16),
    &[7],
    (6.0, 1.0),
    25,
    Chapter::Basics
  ),
  goal(
    "Restful Night",
    "Make a bed: sleep there to wake there.",
    Task::Have(Block::Bed, 1),
    &[3],
    (4.0, -1.0),
    20,
    Chapter::Basics
  ),
  goal(
    "Greywacke Pickaxe",
    "Forge a pickaxe with a greywacke head.",
    Task::Forge(Block::Pickaxe, Material::Greywacke),
    &[2],
    (3.0, 3.0),
    20,
    Chapter::Tools
  ),
  goal(
    "Copper Prospector",
    "Mine copper ore.",
    Task::Have(Block::CopperOre, 8),
    &[10],
    (4.0, 3.0),
    20,
    Chapter::Tools
  ),
  goal(
    "Iron Age",
    "Mine iron ore.",
    Task::Have(Block::IronOre, 8),
    &[11],
    (5.0, 3.0),
    25,
    Chapter::Tools
  ),
  goal(
    "Iron Pickaxe",
    "Forge an iron pickaxe at the furnace.",
    Task::Forge(Block::Pickaxe, Material::Iron),
    &[12, 5],
    (6.0, 3.0),
    35,
    Chapter::Tools
  ),
  goal(
    "Gold Fever",
    "Dig deeper for gold.",
    Task::Have(Block::GoldOre, 4),
    &[13],
    (7.0, 2.5),
    35,
    Chapter::Tools
  ),
  goal(
    "Diamonds!",
    "Find diamond ore near the bottom of the world.",
    Task::Have(Block::DiamondOre, 1),
    &[13],
    (7.0, 3.5),
    50,
    Chapter::Tools
  ),
  goal(
    "Diamond Pickaxe",
    "Forge a diamond pickaxe.",
    Task::Forge(Block::Pickaxe, Material::Diamond),
    &[15],
    (8.0, 3.5),
    80,
    Chapter::Tools
  ),
  goal(
    "Bucket",
    "Smelt iron ore into a bucket.",
    Task::Have(Block::Bucket, 1),
    &[12],
    (6.0, 4.0),
    20,
    Chapter::Tools
  ),
  goal(
    "Platinum",
    "Platinum lies deep under cold peaks.",
    Task::Have(Block::PlatinumOre, 2),
    &[15],
    (8.0, 4.5),
    70,
    Chapter::Tools
  ),
  goal(
    "Attuned",
    "Right-click a waystone to remember it.",
    Task::Attune(1),
    &[0],
    (1.0, -3.0),
    20,
    Chapter::Voyages
  ),
  goal(
    "Wayfarer",
    "Attune to four waystones.",
    Task::Attune(4),
    &[19],
    (2.0, -3.0),
    40,
    Chapter::Voyages
  ),
  goal(
    "Cartographer",
    "Attune to ten waystones across the sea.",
    Task::Attune(10),
    &[20],
    (3.0, -3.0),
    90,
    Chapter::Voyages
  ),
  goal(
    "Seafarer",
    "Build a boat.",
    Task::Have(Block::Boat, 1),
    &[1],
    (2.0, -4.0),
    20,
    Chapter::Voyages
  ),
  goal(
    "Angler",
    "Make a fishing rod.",
    Task::Have(Block::FishingRod, 1),
    &[22],
    (3.0, -4.0),
    15,
    Chapter::Voyages
  ),
  goal(
    "Catch of the Day",
    "Catch some cod.",
    Task::Have(Block::Cod, 3),
    &[23],
    (4.0, -4.0),
    20,
    Chapter::Voyages
  ),
  goal(
    "Exotic Catch",
    "Catch a pufferfish.",
    Task::Have(Block::Pufferfish, 1),
    &[24],
    (5.0, -4.0),
    35,
    Chapter::Voyages
  ),
  goal(
    "Skyward",
    "Build a hot air balloon.",
    Task::Have(Block::Balloon, 1),
    &[22],
    (4.0, -5.0),
    50,
    Chapter::Voyages
  ),
  goal(
    "Shepherd",
    "Gather wool from sheep.",
    Task::Have(Block::Wool, 8),
    &[0],
    (1.0, 6.0),
    15,
    Chapter::Hunts
  ),
  goal(
    "Plucked",
    "Gather feathers from birds.",
    Task::Have(Block::Feather, 4),
    &[27],
    (2.0, 6.0),
    15,
    Chapter::Hunts
  ),
  goal(
    "Jelly Wrangler",
    "Slay lanternjellies: mind the sting.",
    Task::Slay(&[Prey::Beast(Breed::Lanternjelly)], 3),
    &[22],
    (3.0, 6.5),
    25,
    Chapter::Hunts
  ),
  goal(
    "Snapjaw Hunter",
    "Slay snapjaws, the fins that circle swimmers.",
    Task::Slay(SEA_MONSTERS, 2),
    &[29],
    (4.0, 6.5),
    50,
    Chapter::Hunts
  ),
  goal(
    "Leviathan",
    "Slay a brinewyrm in the deep sea.",
    Task::Slay(&[Prey::Beast(Breed::Brinewyrm)], 1),
    &[30],
    (5.0, 6.5),
    150,
    Chapter::Hunts
  ),
  goal(
    "Repel Boarders",
    "Defeat pirates or vikings who come ashore.",
    Task::Slay(RAIDERS, 3),
    &[28],
    (3.0, 5.5),
    40,
    Chapter::Hunts
  ),
  goal(
    "Into the Deep",
    "Defeat skeletons in a dungeon.",
    Task::Slay(&[Prey::Folk(Band::Skeleton)], 5),
    &[3],
    (2.0, 8.0),
    30,
    Chapter::Hunts
  ),
  goal(
    "Web Cutter",
    "Defeat cave spiders.",
    Task::Slay(&[Prey::Beast(Breed::Spider)], 3),
    &[33],
    (3.0, 8.0),
    30,
    Chapter::Hunts
  ),
  goal(
    "Imp Hunter",
    "Defeat imps over the lava.",
    Task::Slay(&[Prey::Folk(Band::Imp)], 3),
    &[34],
    (4.0, 8.0),
    45,
    Chapter::Hunts
  ),
  goal(
    "Warden's End",
    "Defeat the Bone Warden in a vault.",
    Task::Slay(&[Prey::Folk(Band::Warden)], 1),
    &[35, 13],
    (5.0, 8.0),
    150,
    Chapter::Hunts
  ),
  goal(
    "Arcane Dust",
    "Grind lumen crystals or defeat shades.",
    Task::Have(Block::ArcaneDust, 4),
    &[33],
    (2.0, 10.0),
    25,
    Chapter::Arcana
  ),
  goal(
    "Apprentice",
    "Reach level 5.",
    Task::Level(5),
    &[0],
    (1.0, 11.0),
    30,
    Chapter::Arcana
  ),
  goal(
    "Seasoned",
    "Reach level 15.",
    Task::Level(15),
    &[38],
    (2.0, 11.0),
    80,
    Chapter::Arcana
  ),
  goal(
    "Spellcraft",
    "Craft a Fire Bolt once you know it.",
    Task::Have(Block::SpellFirebolt, 1),
    &[37],
    (3.0, 10.0),
    30,
    Chapter::Arcana
  ),
  goal(
    "Bloom Font",
    "Build a bloom font from glimmer flowers.",
    Task::Have(Block::BloomFont, 1),
    &[40],
    (4.0, 10.0),
    35,
    Chapter::Arcana
  ),
  goal(
    "Mana Cistern",
    "Make a pool to gather mana.",
    Task::Have(Block::ManaCistern, 1),
    &[41],
    (5.0, 10.0),
    40,
    Chapter::Arcana
  ),
  goal(
    "Glimsteel",
    "Soak iron ore in a mana pool.",
    Task::Have(Block::Glimsteel, 1),
    &[42],
    (6.0, 10.0),
    60,
    Chapter::Arcana
  ),
  goal(
    "Mithril",
    "Infuse a mithril ingot on the confluence altar.",
    Task::Have(Block::MithrilIngot, 1),
    &[43],
    (7.0, 10.0),
    120,
    Chapter::Arcana
  ),
  goal(
    "Pipework",
    "Smelt copper pipes.",
    Task::Have(Block::CopperPipe, 8),
    &[11, 6],
    (5.0, 13.0),
    30,
    Chapter::Industry
  ),
  goal(
    "Steam Power",
    "Build a steam engine.",
    Task::Have(Block::SteamEngine, 1),
    &[45],
    (6.0, 13.0),
    50,
    Chapter::Industry
  ),
  goal(
    "Stored Charge",
    "Build a battery.",
    Task::Have(Block::Battery, 1),
    &[46],
    (7.0, 13.0),
    40,
    Chapter::Industry
  ),
  goal(
    "Conveyance",
    "Make conveyor belts.",
    Task::Have(Block::Conveyor, 4),
    &[46],
    (7.0, 14.0),
    35,
    Chapter::Industry
  ),
  goal(
    "Black Gold",
    "Build a pumpjack for an oil seep.",
    Task::Have(Block::Pumpjack, 1),
    &[47],
    (8.0, 13.0),
    60,
    Chapter::Industry
  ),
  goal(
    "Refined",
    "Build a refinery to make diesel.",
    Task::Have(Block::Refinery, 1),
    &[49],
    (9.0, 13.0),
    80,
    Chapter::Industry
  ),
  goal(
    "Kelp Forest",
    "Swim down and gather kelp from a cool-water forest.",
    Task::Have(Block::Kelp, 1),
    &[22],
    (6.0, -5.0),
    20,
    Chapter::Depths
  ),
  goal(
    "Coral Reef",
    "Collect brain coral from a warm reef.",
    Task::Have(Block::BrainCoral, 1),
    &[51],
    (7.0, -5.0),
    25,
    Chapter::Depths
  ),
  goal(
    "Fan Coral",
    "Catalogue a red fan coral.",
    Task::Have(Block::FanCoral, 1),
    &[52],
    (8.0, -5.0),
    20,
    Chapter::Depths
  ),
  goal(
    "Tube Coral",
    "Catalogue violet tube coral.",
    Task::Have(Block::TubeCoral, 1),
    &[52],
    (8.0, -4.0),
    20,
    Chapter::Depths
  ),
  goal(
    "Sea Anemone",
    "Catalogue a sea anemone.",
    Task::Have(Block::Anemone, 1),
    &[53],
    (9.0, -5.0),
    25,
    Chapter::Depths
  ),
  goal(
    "Sea Sponge",
    "Catalogue a sea sponge.",
    Task::Have(Block::Sponge, 1),
    &[54],
    (9.0, -4.0),
    25,
    Chapter::Depths
  ),
  goal(
    "Giant Clam",
    "Find a giant clam on the sea floor.",
    Task::Have(Block::Clam, 1),
    &[55],
    (10.0, -5.0),
    40,
    Chapter::Depths
  ),
  goal(
    "Glow Polyp",
    "Glow polyps light the deep sea. Bring one up.",
    Task::Have(Block::GlowPolyp, 1),
    &[56],
    (10.0, -4.0),
    40,
    Chapter::Depths
  ),
  goal(
    "Vent Worms",
    "Tube worms crowd the hot vents in the trenches.",
    Task::Have(Block::VentWorms, 1),
    &[58],
    (8.0, -8.0),
    60,
    Chapter::Depths
  ),
  goal(
    "Bio Lab",
    "Build a bio lab to culture what you collect.",
    Task::Have(Block::BioLab, 1),
    &[51],
    (6.0, -6.0),
    40,
    Chapter::Depths
  ),
  goal(
    "Cultivator",
    "Culture kelp at the bio lab.",
    Task::Have(Block::Kelp, 16),
    &[60],
    (7.0, -6.0),
    30,
    Chapter::Depths
  ),
  goal(
    "Living Stone",
    "Grow reefstone from coral and sponge.",
    Task::Have(Block::Reefstone, 8),
    &[60, 52],
    (8.0, -6.0),
    40,
    Chapter::Depths
  ),
  goal(
    "Pearl Diver",
    "Culture a clam until it yields pearls.",
    Task::Have(Block::Pearl, 1),
    &[57, 60],
    (9.0, -6.0),
    50,
    Chapter::Depths
  ),
  goal(
    "Mother of Pearl",
    "Turn pearls into nacre.",
    Task::Have(Block::Nacre, 8),
    &[63],
    (10.0, -6.0),
    50,
    Chapter::Depths
  ),
  goal(
    "Splicer",
    "Splice kelp with glow polyps into glowkelp.",
    Task::Have(Block::Glowkelp, 1),
    &[60, 58],
    (11.0, -6.0),
    60,
    Chapter::Depths
  ),
  goal(
    "Submersible",
    "Build a submarine: launch it into water and fit modules.",
    Task::Have(Block::Submarine, 1),
    &[22, 12],
    (6.0, -7.0),
    80,
    Chapter::Depths
  ),
  goal(
    "Echoes",
    "Make a sonar module.",
    Task::Have(Block::SonarModule, 1),
    &[66],
    (7.0, -7.0),
    50,
    Chapter::Depths
  ),
  goal(
    "Pressure Hull",
    "Smelt hull plating from iron and manganese nodules.",
    Task::Have(Block::HullPlating, 1),
    &[66],
    (8.0, -7.0),
    60,
    Chapter::Depths
  ),
  goal(
    "Drill Arm",
    "Build a drill arm for the submarine.",
    Task::Have(Block::DrillModule, 1),
    &[66],
    (7.0, -8.0),
    50,
    Chapter::Depths
  ),
  goal(
    "Sunken Ruins",
    "Salvage alloy from a ruin on the sea floor.",
    Task::Have(Block::AlloyScrap, 1),
    &[67],
    (9.0, -7.0),
    80,
    Chapter::Depths
  ),
  goal(
    "Silence the Watch",
    "Destroy the sentinels that keep the sunken ruins.",
    Task::Slay(SENTINELS, 3),
    &[70],
    (10.0, -7.0),
    100,
    Chapter::Depths
  ),
  goal(
    "Custodian",
    "Bring down a custodian.",
    Task::Slay(CUSTODIANS, 1),
    &[71],
    (11.0, -7.0),
    200,
    Chapter::Depths
  ),
  goal(
    "Aether Cell",
    "Recover an aether cell.",
    Task::Have(Block::AetherCell, 1),
    &[70],
    (10.0, -8.0),
    100,
    Chapter::Depths
  ),
  goal(
    "Abyssal Hull",
    "Fit resonant plating: no depth can crush it.",
    Task::Have(Block::ResonantPlating, 1),
    &[72],
    (12.0, -7.0),
    150,
    Chapter::Depths
  ),
  goal(
    "Aether Lance",
    "Rebuild the drill the sunken ruins were cut with.",
    Task::Have(Block::PlasmaDrill, 1),
    &[73],
    (11.0, -8.0),
    150,
    Chapter::Depths
  ),
  goal(
    "Deep Resonance",
    "Build the deep resonance sonar.",
    Task::Have(Block::DeepSonar, 1),
    &[73],
    (12.0, -8.0),
    150,
    Chapter::Depths
  )
];

#[derive(Component, Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct Deeds {
  pub done: Vec<u16>,
  pub tallies: Vec<(Prey, u16)>
}

impl Deeds {
  pub fn finished(&self, goal: u16) -> bool { self.done.contains(&goal) }

  pub fn open(&self, goal: u16) -> bool {
    GOALS[usize::from(goal)].after.iter().all(|&before| self.finished(before))
  }

  fn slain(&self, prey: &[Prey]) -> u16 {
    self
      .tallies
      .iter()
      .filter(|(kind, _)| prey.contains(kind))
      .map(|&(_, count)| count)
      .sum()
  }

  fn tally(&mut self, prey: Prey) {
    match self.tallies.iter_mut().find(|(kind, _)| *kind == prey) {
      Some((_, count)) => *count = count.saturating_add(1),
      None => self.tallies.push((prey, 1))
    }
  }
}

pub struct Standing {
  have: u32,
  need: u32
}

fn standing(
  task: Task,
  deeds: &Deeds,
  inventory: &Inventory,
  visited: &Visited,
  skills: &Skills
) -> Standing {
  match task {
    Task::Have(block, need) => Standing { have: inventory.count(block), need },
    Task::Forge(block, material) => Standing {
      have: u32::from(inventory.slots.iter().flatten().any(|stack| {
        stack.block == block
          && stack.gear.and_then(|gear| gear.material) == Some(material)
      })),
      need: 1
    },
    Task::Slay(prey, need) => {
      Standing { have: u32::from(deeds.slain(prey)), need: u32::from(need) }
    }
    Task::Attune(need) => Standing { have: visited.0.len() as u32, need: need as u32 },
    Task::Level(need) => Standing { have: skills.level, need }
  }
}

fn describe(task: Task) -> String {
  match task {
    Task::Have(block, need) => format!("Have {need} {}", block.name()),
    Task::Forge(block, material) => format!("Have a {material:?} {}", block.name()),
    Task::Slay(prey, need) => format!(
      "Defeat {need} {}",
      prey.iter().map(|prey| prey.name()).collect::<Vec<_>>().join(" or ")
    ),
    Task::Attune(need) => {
      format!("Attune to {need} waystone{}", if need == 1 { "" } else { "s" })
    }
    Task::Level(need) => format!("Reach level {need}")
  }
}

fn emblem(task: Task) -> (Block, Option<Material>) {
  match task {
    Task::Have(block, _) => (block, None),
    Task::Forge(block, material) => (block, Some(material)),
    Task::Slay(prey, _) => (
      match prey.first() {
        Some(Prey::Beast(Breed::Spider)) => Block::Cobweb,
        Some(Prey::Beast(Breed::Lanternjelly)) => Block::ArcaneDust,
        Some(Prey::Beast(Breed::Brinewyrm)) => Block::Tidepearl,
        Some(Prey::Beast(_)) => Block::Cod,
        Some(Prey::Folk(Band::Skeleton)) => Block::BoneDust,
        Some(Prey::Folk(Band::Imp)) => Block::Charcoal,
        Some(Prey::Folk(Band::Warden)) => Block::BonePile,
        _ => Block::Sword
      },
      None
    ),
    Task::Attune(_) => (Block::Waystone, None),
    Task::Level(_) => (Block::Lumen, None)
  }
}

fn tally(
  slain: On<Add, Dying>,
  victims: Query<(Option<&Folk>, Option<&Beast>, Option<&crate::bird::Bird>, &Blamed)>,
  mut players: Query<&mut Deeds>
) {
  if let Ok((folk, beast, bird, &Blamed(player))) = victims.get(slain.entity)
    && let Ok(mut deeds) = players.get_mut(player)
  {
    let prey = folk
      .map(|folk| Prey::Folk(folk.band))
      .or(beast.map(|beast| match beast.breed {
        Breed::Sheep(_) => Prey::Beast(Breed::Sheep(Fleece::White)),
        breed => Prey::Beast(breed)
      }))
      .or(bird.map(|_| Prey::Bird));
    if let Some(prey) = prey {
      deeds.tally(prey)
    }
  }
}

fn review(
  time: Res<Time>,
  mut since: Local<f32>,
  mut players: Query<(Entity, &Controller, &mut Deeds, &Inventory, &Visited, &Skills)>,
  mut earned: MessageWriter<Earned>,
  mut notices: MessageWriter<ToClients<Notice>>
) {
  *since += time.delta_secs();
  if *since >= REVIEW_EVERY {
    *since = 0.0;
    for (player, controller, mut deeds, inventory, visited, skills) in players.iter_mut()
    {
      let reached: Vec<u16> = (0..GOALS.len() as u16)
        .filter(|&index| !deeds.finished(index) && deeds.open(index))
        .filter(|&index| {
          let Standing { have, need } =
            standing(GOALS[usize::from(index)].task, &deeds, inventory, visited, skills);
          have >= need
        })
        .collect();
      if !reached.is_empty() {
        deeds.done.extend(reached.iter().copied());
        for index in reached {
          let found = &GOALS[usize::from(index)];
          earned.write(Earned { player, xp: found.xp });
          notices.write(ToClients {
            targets: SendTargets::Single(controller.client),
            message: Notice(format!("Goal complete: {} (+{} xp)", found.title, found.xp))
          });
        }
      }
    }
  }
}

#[derive(Component)]
struct Canvas;

#[derive(Component)]
struct Header;

#[derive(Component)]
struct Spot(u16);

#[derive(Component)]
struct Mark(u16);

#[derive(Component)]
struct Link(u16, u16);

#[derive(Component)]
struct Label(Chapter);

#[derive(Component)]
struct Hint;

#[derive(Resource, Default)]
struct View {
  pan: Vec2,
  zoom: f32,
  grip: Option<(Vec2, Vec2)>,
  fitted: bool
}

fn bounds() -> (Vec2, Vec2) {
  GOALS
    .iter()
    .map(|found| Vec2::new(found.at.0, found.at.1))
    .fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(low, high), at| {
      (low.min(at), high.max(at))
    })
}

fn clipped(from: Vec2, to: Vec2, size: Vec2) -> Option<(Vec2, Vec2)> {
  let along = to - from;
  let (enter, leave) = [
    (-along.x, from.x),
    (along.x, size.x - from.x),
    (-along.y, from.y),
    (along.y, size.y - from.y)
  ]
  .into_iter()
  .fold(Some((0.0f32, 1.0f32)), |span, (p, q)| {
    span.and_then(|(enter, leave)| match () {
      () if p == 0.0 => (q >= 0.0).then_some((enter, leave)),
      () if p < 0.0 => Some((enter.max(q / p), leave)),
      () => Some((enter, leave.min(q / p)))
    })
  })?;
  (enter < leave).then(|| (from + along * enter, from + along * leave))
}

const CHAPTERS: [Chapter; 6] = [
  Chapter::Basics,
  Chapter::Tools,
  Chapter::Voyages,
  Chapter::Hunts,
  Chapter::Arcana,
  Chapter::Industry
];

pub fn page(page: &mut ChildSpawnerCommands) {
  page.spawn((Header, words("", 18.0, INK)));
  page.spawn((
    Canvas,
    Button,
    RelativeCursorPosition::default(),
    Node {
      flex_grow: 1.0,
      min_height: px(0),
      width: percent(100),
      overflow: Overflow::clip(),
      border: UiRect::all(px(2)),
      ..default()
    },
    BorderColor::all(EDGE),
    BackgroundColor(Color::srgba(0.03, 0.03, 0.035, 0.9))
  ));
  page.spawn(words(
    "Drag to pan and scroll to zoom. Goals unlock once the goals before them are done; finishing one grants experience. Hover a goal for details.",
    13.0,
    FAINT
  ));
}

fn plant(
  canvases: Query<Entity, (With<Canvas>, Without<Children>)>,
  palette: Option<Res<Palette>>,
  mut commands: Commands
) {
  if let Some(palette) = palette {
    for canvas in canvases.iter() {
      commands.entity(canvas).with_children(|canvas| {
        for (index, found) in GOALS.iter().enumerate() {
          for &before in found.after {
            canvas.spawn((
              Link(before, index as u16),
              Node { position_type: PositionType::Absolute, ..default() },
              BackgroundColor(EDGE)
            ));
          }
        }
        for chapter in CHAPTERS {
          canvas.spawn((
            Label(chapter),
            words(chapter.title().to_uppercase(), 20.0, chapter.colour().with_alpha(0.8)),
            SHADE,
            Node { position_type: PositionType::Absolute, ..default() }
          ));
        }
        for (index, found) in GOALS.iter().enumerate() {
          let (block, material) = emblem(found.task);
          canvas
            .spawn((
              Spot(index as u16),
              Button,
              Node {
                position_type: PositionType::Absolute,
                border: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(8)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
              },
              BorderColor::all(EDGE),
              BackgroundColor(Color::srgb(0.07, 0.07, 0.08))
            ))
            .with_child((
              Mark(index as u16),
              ImageNode {
                rect: Some(forged_rect(block, material)),
                ..ImageNode::new(palette.icons.clone())
              },
              Node { width: px(32), height: px(32), ..default() }
            ));
        }
      });
    }
  }
}

fn steer(
  menu: Res<Menu>,
  scroll: Res<AccumulatedMouseScroll>,
  buttons: Res<ButtonInput<MouseButton>>,
  canvas: Query<(&RelativeCursorPosition, &ComputedNode), With<Canvas>>,
  mut view: ResMut<View>
) {
  if menu.showing(Tab::Goals)
    && let Ok((_, computed)) = canvas.single()
    && let size = computed.size * computed.inverse_scale_factor
    && size.min_element() > 50.0
    && !view.fitted
  {
    let (low, high) = bounds();
    let extent = (high - low + 1.5) * SPACING;
    let zoom = (size / extent).min_element().clamp(0.45, 1.0);
    let middle = (low + high) / 2.0;
    let top = low.y + size.y / (2.0 * SPACING * zoom) - 0.8;
    *view = View {
      zoom,
      pan: -Vec2::new(middle.x, middle.y.min(top)) * SPACING * zoom,
      grip: None,
      fitted: true
    }
  }
  if menu.showing(Tab::Goals)
    && let Ok((cursor, computed)) = canvas.single()
    && let Some(spot) = cursor.normalized
  {
    let size = computed.size * computed.inverse_scale_factor;
    let pixel = spot * size;
    let over = cursor.cursor_over;
    if over && scroll.delta.y != 0.0 {
      let zoom = (view.zoom * if scroll.delta.y > 0.0 { 1.15 } else { 1.0 / 1.15 })
        .clamp(ZOOMS.0, ZOOMS.1);
      let anchor = (pixel - view.pan) / view.zoom;
      view.pan = pixel - anchor * zoom;
      view.zoom = zoom
    }
    if over && buttons.just_pressed(MouseButton::Left) {
      view.grip = Some((pixel, view.pan))
    }
    if let Some((from, pan)) = view.grip
      && buttons.pressed(MouseButton::Left)
      && (pixel - from).length() > DRAG
    {
      view.pan = pan + pixel - from
    }
    if !buttons.pressed(MouseButton::Left) {
      view.grip = None
    }
  }
}

fn spot_of(index: u16) -> Vec2 {
  let (x, y) = GOALS[usize::from(index)].at;
  Vec2::new(x, y)
}

fn arrange(
  menu: Res<Menu>,
  view: Res<View>,
  canvas: Query<&ComputedNode, With<Canvas>>,
  mut spots: Query<(&Spot, &mut Node), (Without<Link>, Without<Label>, Without<Mark>)>,
  mut links: Query<
    (&Link, &mut Node, &mut UiTransform, &mut Visibility),
    (Without<Spot>, Without<Mark>)
  >,
  mut labels: Query<(&Label, &mut Node), (Without<Spot>, Without<Link>, Without<Mark>)>,
  mut marks: Query<&mut Node, (With<Mark>, Without<Spot>, Without<Link>, Without<Label>)>
) {
  if menu.showing(Tab::Goals)
    && let Ok(computed) = canvas.single()
  {
    let size = computed.size * computed.inverse_scale_factor;
    let place = |at: Vec2| size / 2.0 + view.pan + at * SPACING * view.zoom;
    let wide = NODE * view.zoom.max(0.5);
    for (spot, mut node) in spots.iter_mut() {
      let centre = place(spot_of(spot.0));
      node.left = px(centre.x - wide / 2.0);
      node.top = px(centre.y - wide / 2.0);
      node.width = px(wide);
      node.height = px(wide);
    }
    for mut node in marks.iter_mut() {
      node.width = px((wide - 12.0).max(10.0));
      node.height = px((wide - 12.0).max(10.0));
    }
    for (link, mut node, mut transform, mut visibility) in links.iter_mut() {
      let shown = clipped(place(spot_of(link.0)), place(spot_of(link.1)), size - 4.0);
      visibility.set_if_neq(match shown {
        Some(_) => Visibility::Inherited,
        None => Visibility::Hidden
      });
      let (from, to) = shown.unwrap_or_default();
      let length = from.distance(to);
      let middle = (from + to) / 2.0;
      node.left = px(middle.x - length / 2.0);
      node.top = px(middle.y - 2.0);
      node.width = px(length);
      node.height = px(4.0);
      transform.rotation = Rot2::radians((to - from).to_angle());
    }
    for (label, mut node) in labels.iter_mut() {
      let first = GOALS
        .iter()
        .filter(|found| found.chapter == label.0)
        .map(|found| Vec2::new(found.at.0, found.at.1))
        .fold(Vec2::new(f32::MAX, f32::MAX), Vec2::min);
      let at = place(first - Vec2::new(0.4, 0.75));
      node.left = px(at.x);
      node.top = px(at.y - 12.0);
    }
  }
}

fn mine(pilot: Option<Res<Pilot>>, deeds: &Query<&Deeds>) -> Deeds {
  pilot.and_then(|pilot| deeds.get(pilot.me).ok().cloned()).unwrap_or_default()
}

fn paint(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  deeds: Query<&Deeds>,
  time: Res<Time>,
  mut spots: Query<(&Spot, &Interaction, &mut BorderColor, &mut BackgroundColor)>,
  mut marks: Query<(&Mark, &mut ImageNode)>,
  mut links: Query<(&Link, &mut BackgroundColor), Without<Spot>>,
  mut headers: Query<&mut Text, With<Header>>
) {
  if menu.showing(Tab::Goals) {
    let deeds = mine(pilot, &deeds);
    let pulse = 0.6 + (time.elapsed_secs() * 4.0).sin() * 0.4;
    for (spot, interaction, mut border, mut background) in spots.iter_mut() {
      let colour = GOALS[usize::from(spot.0)].chapter.colour();
      let (done, open) = (deeds.finished(spot.0), deeds.open(spot.0));
      let edge = match (done, open) {
        (true, _) => colour,
        (false, true) => Color::WHITE.mix(&colour, 0.4).with_alpha(pulse),
        _ => EDGE
      };
      border.set_if_neq(BorderColor::all(match *interaction != Interaction::None {
        true => LIT,
        false => edge
      }));
      background.set_if_neq(BackgroundColor(match done {
        true => colour.darker(0.6),
        false => Color::srgb(0.07, 0.07, 0.08)
      }));
    }
    for (mark, mut image) in marks.iter_mut() {
      let tint = match (deeds.finished(mark.0), deeds.open(mark.0)) {
        (true, _) | (_, true) => Color::WHITE,
        _ => Color::srgba(0.6, 0.6, 0.6, 0.75)
      };
      if image.color != tint {
        image.color = tint
      }
    }
    for (link, mut background) in links.iter_mut() {
      let colour = GOALS[usize::from(link.1)].chapter.colour();
      background.set_if_neq(BackgroundColor(match deeds.finished(link.0) {
        true => colour.with_alpha(0.85),
        false => Color::srgba(0.3, 0.33, 0.36, 0.6)
      }));
    }
    let line = format!("{} of {} goals complete", deeds.done.len(), GOALS.len());
    for mut text in headers.iter_mut() {
      if text.0 != line {
        text.0 = line.clone()
      }
    }
  }
}

fn tooltip(mut commands: Commands) {
  commands
    .spawn((
      Hint,
      Node {
        position_type: PositionType::Absolute,
        flex_direction: FlexDirection::Column,
        padding: UiRect::axes(px(8), px(5)),
        border: UiRect::all(px(2)),
        max_width: px(340),
        row_gap: px(2),
        ..default()
      },
      BorderColor::all(Color::srgb(0.45, 0.4, 0.25)),
      BackgroundColor(Color::srgba(0.06, 0.05, 0.03, 0.96)),
      GlobalZIndex(40),
      Visibility::Hidden
    ))
    .with_children(|tip| {
      tip.spawn(words("", 16.0, INK));
      tip.spawn(words("", 14.0, FAINT));
      tip.spawn(words("", 14.0, INK));
      tip.spawn(words("", 13.0, LIT));
    });
}

fn explain(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  deeds: Query<&Deeds>,
  owned: Query<(&Inventory, &Visited, &Skills)>,
  spots: Query<(&Spot, &Interaction)>,
  windows: Query<&Window, With<PrimaryWindow>>,
  scale: Res<UiScale>,
  mut tips: Query<(&mut Node, &mut Visibility, &Children), With<Hint>>,
  mut texts: Query<(&mut Text, &mut TextColor)>
) {
  let me = pilot.as_ref().map(|pilot| pilot.me);
  let deeds = mine(pilot, &deeds);
  let pointed = spots
    .iter()
    .find(|(_, interaction)| **interaction != Interaction::None)
    .map(|(spot, _)| spot.0)
    .filter(|_| menu.showing(Tab::Goals));
  let cursor =
    windows.single().ok().and_then(Window::cursor_position).map(|at| at / scale.0);
  for (mut node, mut visibility, children) in tips.iter_mut() {
    match pointed.zip(cursor) {
      Some((index, at)) => {
        let found = &GOALS[usize::from(index)];
        let progress = me.and_then(|me| owned.get(me).ok()).map_or(
          String::new(),
          |(inventory, visited, skills)| {
            let Standing { have, need } =
              standing(found.task, &deeds, inventory, visited, skills);
            format!("{}: {}/{}", describe(found.task), have.min(need), need)
          }
        );
        let state = match (deeds.finished(index), deeds.open(index)) {
          (true, _) => format!("Complete! (+{} xp)", found.xp),
          (false, true) => format!("Reward: {} xp", found.xp),
          _ => format!(
            "Locked: first finish {}",
            found
              .after
              .iter()
              .filter(|&&before| !deeds.finished(before))
              .map(|&before| GOALS[usize::from(before)].title)
              .collect::<Vec<_>>()
              .join(", ")
          )
        };
        let lines = [found.title.to_string(), found.about.to_string(), progress, state];
        node.left = px(at.x + 18.0);
        node.top = px(at.y + 14.0);
        visibility.set_if_neq(Visibility::Inherited);
        for (child, line) in children.iter().zip(lines) {
          if let Ok((mut text, mut colour)) = texts.get_mut(child) {
            if text.0 != line {
              text.0 = line
            }
            if child == children[0] {
              colour.0 = found.chapter.colour()
            }
          }
        }
      }
      None => {
        visibility.set_if_neq(Visibility::Hidden);
      }
    }
  }
}

pub struct Goals;

impl Plugin for Goals {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<View>()
      .add_observer(tally)
      .add_systems(Update, review.run_if(authority))
      .add_systems(Startup, tooltip.run_if(plays))
      .add_systems(Update, (plant, steer, arrange, paint, explain).chain().run_if(plays));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn goals_form_a_tree_of_items() {
    for (index, found) in GOALS.iter().enumerate() {
      assert!(
        found.after.iter().all(|&before| usize::from(before) < index),
        "{}",
        found.title
      );
      if let Task::Have(block, _) = found.task {
        assert!(block.item(), "{} wants {block:?}", found.title)
      }
      for other in GOALS.iter().skip(index + 1) {
        assert!(found.at != other.at, "{} and {} overlap", found.title, other.title)
      }
    }
  }
}
