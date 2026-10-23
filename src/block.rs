use strum::VariantArray;
use {bevy::math::IVec3,
     serde::{Deserialize, Serialize}};

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
  VariantArray,
)]
#[repr(u16)]
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
  PalmSign,
  TallGrass,
  Ladder,
  LadderEast,
  LadderSouth,
  LadderWest,
  Bed,
  Wool,
  Feather,
  FishingRod,
  Cod,
  Salmon,
  TropicalFish,
  Pufferfish,
  BedEast,
  BedSouth,
  BedWest,
  BedHead,
  BedHeadEast,
  BedHeadSouth,
  BedHeadWest,
  Charcoal,
  AncientBrick,
  MossyBrick,
  CrackedBrick,
  CarvedStone,
  #[serde(alias = "Adra")]
  Lumen,
  BonePile,
  Cobweb,
  Brazier,
  Urn,
  Chest,
  EmptyChest,
  ArcaneDust,
  BoneDust,
  LizardClaw,
  Candle,
  SpellFirebolt,
  SpellFrost,
  SpellMissile,
  SpellHeal,
  SpellHaste,
  SpellStoneskin,
  SpellLeap,
  SpellLight,
  SpellThunder,
  SpellFireball,
  SpellIceStorm,
  SpellAnimate,
  SpellGuardian,
  SpellInvisibility,
  Pickaxe,
  Axe,
  Shovel,
  Sword,
  Hat,
  Amulet,
  Mail,
  Belt,
  Boots,
  Ring,
  WallTorch,
  WallTorchEast,
  WallTorchSouth,
  WallTorchWest,
  Balloon,
  OilSeep,
  CopperPipe,
  IronPipe,
  GoldPipe,
  DiamondPipe,
  Tank,
  CopperCable,
  Battery,
  ElectricPump,
  Pumpjack,
  Refinery,
  Fermenter,
  SteamEngine,
  BiofuelGenerator,
  DieselGenerator,
  GeothermalGenerator,
  SolarPanel,
  WindTurbine,
  ElectricFurnace,
  ElectricLamp,
  ElectricLampLit,
  FluidDrain,
  PipedDrain,
  ItemDrain,
  ItemPipe,
  Conveyor,
  ConveyorEast,
  ConveyorSouth,
  ConveyorWest,
  ConveyorUp,
  ConveyorUpEast,
  ConveyorUpSouth,
  ConveyorUpWest,
  ConveyorDown,
  ConveyorDownEast,
  ConveyorDownSouth,
  ConveyorDownWest,
  GlimmerPink,
  GlimmerCyan,
  GlimmerViolet,
  GlimmerGold,
  BloomFont,
  Sunsip,
  Dewcup,
  Emberbell,
  ManaCistern,
  Cauldron,
  CrimsonAltar,
  Plinth,
  ConfluenceAltar,
  CrimsonRune,
  Glimsteel,
  Starheart,
  Tidepearl,
  Verdantine,
  Hexbrass,
  MithrilIngot,
  BareTablet,
  BoundTablet,
  Heartstone,
  RitualKnife,
  WellspringSigil,
  PlatinumOre,
  Platinum,
  PlatinumDust,
  Moonlace,
  Crusher,
  Satchel,
  Rucksack,
  ExpeditionPack,
  LesserRift,
  GreaterRift,
  StoneBricks,
  Kelp,
  Seagrass,
  BrainCoral,
  FanCoral,
  TubeCoral,
  Anemone,
  GlowPolyp,
  VentWorms,
  Sponge,
  Clam,
  Pearl,
  Abyssite,
  VentStone,
  NoduleOre,
  PrecursorAlloy,
  PrecursorGlass,
  AetherConduit,
  AetherLamp,
  AetherCell,
  AlloyScrap,
  ResonantCrystal,
  Datacore,
  JellyGel,
  BioLab,
  Reefstone,
  Nacre,
  Biolume,
  Glowkelp,
  Submarine,
  SonarModule,
  DeepSonar,
  DrillModule,
  PlasmaDrill,
  Floodlight,
  Thruster,
  Hydrojet,
  HullPlating,
  ResonantPlating,
  SampleClaw,
  TorpedoBay,
  PowerCell,
  Torpedo
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, VariantArray)]
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
  PalmSign,
  TallGrass,
  Ladder,
  Bed,
  Wool,
  Feather,
  FishingRod,
  Cod,
  Salmon,
  TropicalFish,
  Pufferfish,
  Charcoal,
  AncientBrick,
  MossyBrick,
  CrackedBrick,
  CarvedStone,
  Lumen,
  BonePile,
  Cobweb,
  Terracotta,
  ArcaneDust,
  BoneDust,
  LizardClaw,
  Candle,
  SpellFirebolt,
  SpellFrost,
  SpellMissile,
  SpellHeal,
  SpellHaste,
  SpellStoneskin,
  SpellLeap,
  SpellLight,
  SpellThunder,
  SpellFireball,
  SpellIceStorm,
  SpellAnimate,
  SpellGuardian,
  SpellInvisibility,
  Pickaxe,
  Axe,
  Shovel,
  Sword,
  Hat,
  Amulet,
  Mail,
  Belt,
  Boots,
  Ring,
  Balloon,
  Copper,
  Steel,
  PipeGlass,
  Casing,
  Grille,
  Solar,
  Cable,
  OilSeep,
  Cells,
  Coil,
  Bulb,
  BulbLit,
  Hazard,
  Tar,
  Rubber,
  GlimmerPink,
  GlimmerCyan,
  GlimmerViolet,
  GlimmerGold,
  Sunsip,
  Dewcup,
  Emberbell,
  Moonstone,
  Runework,
  CrimsonRune,
  Glimsteel,
  Starheart,
  Tidepearl,
  Verdantine,
  Hexbrass,
  MithrilIngot,
  BareTablet,
  BoundTablet,
  Heartstone,
  RitualKnife,
  WellspringSigil,
  PlatinumOre,
  Platinum,
  PlatinumDust,
  Moonlace,
  Crusher,
  Satchel,
  Rucksack,
  ExpeditionPack,
  LesserRift,
  GreaterRift,
  StoneBricks,
  Kelp,
  Seagrass,
  BrainCoral,
  FanCoral,
  TubeCoral,
  Anemone,
  GlowPolyp,
  VentWorms,
  Sponge,
  Clam,
  Pearl,
  Abyssite,
  VentStone,
  NoduleOre,
  PrecursorAlloy,
  PrecursorGlass,
  AetherConduit,
  AetherLamp,
  AetherCell,
  AlloyScrap,
  ResonantCrystal,
  Datacore,
  JellyGel,
  BioLabTop,
  BioLabSide,
  Reefstone,
  Nacre,
  Biolume,
  Glowkelp,
  Submarine,
  SonarModule,
  DeepSonar,
  DrillModule,
  PlasmaDrill,
  Floodlight,
  Thruster,
  Hydrojet,
  HullPlating,
  ResonantPlating,
  SampleClaw,
  TorpedoBay,
  PowerCell,
  Torpedo
}

impl Tile {
  pub const ALL: [Tile; Tile::VARIANTS.len()] = *Tile::VARIANTS.first_chunk().unwrap();

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Surface {
  Alloy,
  Conduit,
  Abyssal,
  Vent,
  Reef,
  Nacre,
  Brain,
  Sponge,
  Biolume
}

impl Surface {
  pub fn code(self) -> u32 { self as u32 }

  pub fn joins(self, other: Surface) -> bool {
    self == other
      || matches!(
        (self, other),
        (Surface::Alloy, Surface::Conduit) | (Surface::Conduit, Surface::Alloy)
      )
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
  pub const ALL: [Block; Block::VARIANTS.len()] = *Block::VARIANTS.first_chunk().unwrap();

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
    self != Block::Air
      && !self.fluid()
      && self != Block::WaystoneTop
      && self.held() == self
  }

  pub fn tool(self) -> bool {
    matches!(
      self,
      Block::Boat
        | Block::Balloon
        | Block::Bucket
        | Block::WaterBucket
        | Block::LavaBucket
        | Block::Feather
        | Block::FishingRod
        | Block::Charcoal
        | Block::ArcaneDust
        | Block::BoneDust
        | Block::LizardClaw
        | Block::Candle
    ) || self.fish()
      || self.salvage()
      || self.module()
      || self == Block::Submarine
      || self.trinket()
      || self.spell()
      || self.forged()
      || self.worn()
  }

  pub fn salvage(self) -> bool {
    matches!(
      self,
      Block::Pearl
        | Block::AetherCell
        | Block::AlloyScrap
        | Block::ResonantCrystal
        | Block::Datacore
        | Block::JellyGel
        | Block::Torpedo
    )
  }

  pub fn module(self) -> bool { (Block::SonarModule..=Block::PowerCell).contains(&self) }

  pub fn organism(self) -> bool { (Block::Kelp..=Block::Clam).contains(&self) }

  pub fn waterlogged(self) -> bool {
    self.organism() && !matches!(self, Block::BrainCoral | Block::Sponge)
  }

  pub fn wet(self) -> bool {
    self.waterlogged() || self.liquid().is_some_and(|(fluid, _)| fluid == Fluid::Water)
  }

  pub fn sunk(self) -> bool { self.fluid() || self.waterlogged() }

  pub const TOOLS: [Block; 4] = [Block::Pickaxe, Block::Axe, Block::Shovel, Block::Sword];

  pub fn trinket(self) -> bool {
    (Block::Glimsteel..=Block::WellspringSigil).contains(&self)
      || matches!(self, Block::Platinum | Block::PlatinumDust)
      || self.pack()
      || self.rift()
  }

  pub fn pack(self) -> bool {
    matches!(self, Block::Satchel | Block::Rucksack | Block::ExpeditionPack)
  }

  pub fn rift(self) -> bool { matches!(self, Block::LesserRift | Block::GreaterRift) }

  pub fn portable(self) -> bool { self.pack() || self.rift() }

  pub fn glimmer(self) -> bool {
    (Block::GlimmerPink..=Block::GlimmerGold).contains(&self)
  }

  pub fn bloomer(self) -> bool { (Block::Sunsip..=Block::Emberbell).contains(&self) }

  pub fn arcane(self) -> bool { (Block::Sunsip..=Block::ConfluenceAltar).contains(&self) }

  pub fn vessel(self) -> bool { matches!(self, Block::ManaCistern | Block::Cauldron) }

  pub fn holder(self) -> bool {
    matches!(self, Block::CrimsonAltar | Block::Plinth | Block::ConfluenceAltar)
  }

  pub fn forged(self) -> bool { Block::TOOLS.contains(&self) }

  pub fn worn(self) -> bool { crate::gear::Wear::of(self).is_some() }

  pub const SPELLS: [Block; 14] = [
    Block::SpellFirebolt,
    Block::SpellFrost,
    Block::SpellMissile,
    Block::SpellHeal,
    Block::SpellHaste,
    Block::SpellStoneskin,
    Block::SpellLeap,
    Block::SpellLight,
    Block::SpellThunder,
    Block::SpellFireball,
    Block::SpellIceStorm,
    Block::SpellAnimate,
    Block::SpellGuardian,
    Block::SpellInvisibility
  ];

  pub fn spell(self) -> bool { Block::SPELLS.contains(&self) }

  pub fn furnished(self) -> bool {
    matches!(
      self,
      Block::Brazier
        | Block::Urn
        | Block::Chest
        | Block::EmptyChest
        | Block::CraftingTable
        | Block::Furnace
        | Block::BloomFont
        | Block::BioLab
        | Block::AetherLamp
    ) || self.machine()
  }

  pub const PIPES: [Block; 4] =
    [Block::CopperPipe, Block::IronPipe, Block::GoldPipe, Block::DiamondPipe];

  pub fn pipe(self) -> bool { Block::PIPES.contains(&self) }

  pub fn machine(self) -> bool {
    (Block::CopperPipe..=Block::ItemDrain).contains(&self)
      || self == Block::ItemPipe
      || self == Block::Crusher
      || self.arcane()
  }

  pub fn conveyor(self) -> bool {
    (Block::Conveyor..=Block::ConveyorDownWest).contains(&self)
  }

  pub fn drain(self) -> bool {
    matches!(self, Block::FluidDrain | Block::PipedDrain | Block::ItemDrain)
  }

  pub const WALL_TORCHES: [(IVec3, Block); 4] = [
    (IVec3::NEG_Z, Block::WallTorch),
    (IVec3::X, Block::WallTorchEast),
    (IVec3::Z, Block::WallTorchSouth),
    (IVec3::NEG_X, Block::WallTorchWest)
  ];

  pub fn torch(self) -> bool {
    self == Block::Torch || Block::WALL_TORCHES.iter().any(|&(_, torch)| torch == self)
  }

  pub fn lit(self) -> bool {
    self.torch()
      || matches!(
        self,
        Block::Brazier
          | Block::ElectricLampLit
          | Block::AetherLamp
          | Block::Biolume
          | Block::Glowkelp
      )
  }

  pub const FISH: [Block; 4] =
    [Block::Cod, Block::Salmon, Block::TropicalFish, Block::Pufferfish];

  pub fn fish(self) -> bool { Block::FISH.contains(&self) }

  pub const LADDERS: [(IVec3, Block); 4] = [
    (IVec3::NEG_Z, Block::Ladder),
    (IVec3::X, Block::LadderEast),
    (IVec3::Z, Block::LadderSouth),
    (IVec3::NEG_X, Block::LadderWest)
  ];

  pub fn ladder(self) -> bool { Block::LADDERS.iter().any(|&(_, ladder)| ladder == self) }

  fn hung(self) -> &'static [(IVec3, Block)] {
    match (self.ladder(), self.torch()) {
      (true, _) => &Block::LADDERS,
      (_, true) => &Block::WALL_TORCHES,
      _ => &[]
    }
  }

  pub fn wall(self) -> Option<IVec3> {
    self.hung().iter().find(|&&(_, hung)| hung == self).map(|&(wall, _)| wall)
  }

  pub fn against(self, wall: IVec3) -> Block {
    self.hung().iter().find(|&&(side, _)| side == wall).map_or(self, |&(_, hung)| hung)
  }

  pub const BEDS: [(IVec3, Block, Block); 4] = [
    (IVec3::NEG_Z, Block::Bed, Block::BedHead),
    (IVec3::X, Block::BedEast, Block::BedHeadEast),
    (IVec3::Z, Block::BedSouth, Block::BedHeadSouth),
    (IVec3::NEG_X, Block::BedWest, Block::BedHeadWest)
  ];

  pub fn bed(self) -> bool {
    Block::BEDS.iter().any(|&(_, foot, head)| foot == self || head == self)
  }

  pub fn pillow(self) -> bool { Block::BEDS.iter().any(|&(_, _, head)| head == self) }

  pub fn partner(self) -> Option<IVec3> {
    Block::BEDS.iter().find_map(|&(toward, foot, head)| {
      (foot == self).then_some(toward).or((head == self).then_some(-toward))
    })
  }

  pub fn laid(toward: IVec3) -> Option<(Block, Block)> {
    Block::BEDS
      .iter()
      .find(|&&(way, ..)| way == toward)
      .map(|&(_, foot, head)| (foot, head))
  }

  pub fn held(self) -> Block {
    match (self.ladder(), self.bed(), self.torch()) {
      (true, ..) => Block::Ladder,
      (_, true, _) => Block::Bed,
      (.., true) => Block::Torch,
      _ if self == Block::ElectricLampLit => Block::ElectricLamp,
      _ if self == Block::PipedDrain => Block::FluidDrain,
      _ if self.conveyor() => Block::Conveyor,
      _ => self
    }
  }

  pub fn placeable(self) -> bool { self.item() && !self.tool() }

  pub fn stack(self) -> u16 {
    match self {
      Block::WaterBucket | Block::LavaBucket | Block::Boat | Block::FishingRod => 1,
      Block::Balloon => 1,
      Block::Bed => 1,
      Block::RitualKnife | Block::WellspringSigil | Block::Heartstone => 1,
      portable if portable.portable() => 1,
      gear if gear.forged() || gear.worn() => 1,
      Block::Submarine => 1,
      module if module.module() => 1,
      Block::Bucket | Block::Torpedo => 16,
      spell if spell.spell() => 16,
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

  pub fn station(self) -> bool {
    matches!(
      self,
      Block::CraftingTable
        | Block::Furnace
        | Block::ElectricFurnace
        | Block::BloomFont
        | Block::BioLab
    )
  }

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
      Block::Glass | Block::Cobweb | Block::PrecursorGlass => Look::Cutout,
      leaves if leaves.leafy() => Look::Cutout,
      fluid if fluid.fluid() => Look::Liquid,
      model if model.modelled() || model.waystone() || model.furnished() => Look::Model,
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

  pub fn modelled(self) -> bool {
    self.plant()
      || self.torch()
      || self.sign()
      || self.ladder()
      || self.bed()
      || self.conveyor()
      || self.bloomer()
      || self.vessel()
      || self.waterlogged()
      || self == Block::Glowkelp
  }

  pub fn plant(self) -> bool {
    (Block::Poppy..=Block::BrownMushroom).contains(&self)
      || self == Block::TallGrass
      || self == Block::Moonlace
      || matches!(self, Block::Kelp | Block::Seagrass | Block::Glowkelp)
      || self.glimmer()
  }

  pub fn opaque(self) -> bool { self.look() == Look::Opaque }

  pub fn surface(self) -> Option<Surface> {
    match self {
      Block::PrecursorAlloy => Some(Surface::Alloy),
      Block::AetherConduit => Some(Surface::Conduit),
      Block::Abyssite => Some(Surface::Abyssal),
      Block::VentStone => Some(Surface::Vent),
      Block::Reefstone => Some(Surface::Reef),
      Block::Nacre => Some(Surface::Nacre),
      Block::BrainCoral => Some(Surface::Brain),
      Block::Sponge => Some(Surface::Sponge),
      Block::Biolume => Some(Surface::Biolume),
      _ => None
    }
  }

  pub fn softness(self) -> f32 {
    match self {
      Block::Grass
      | Block::Dirt
      | Block::Sand
      | Block::Snow
      | Block::Mycelium
      | Block::Clay
      | Block::Leaves
      | Block::BirchLeaves
      | Block::SpruceLeaves
      | Block::PalmLeaves
      | Block::Sponge => 1.0,
      Block::BrainCoral => 0.8,
      Block::Abyssite | Block::VentStone | Block::NoduleOre => 0.6,
      Block::Reefstone => 0.4,
      Block::Nacre | Block::Biolume => 0.25,
      Block::Gravel | Block::RedCap | Block::BrownCap | Block::MushroomStem => 0.8,
      Block::Stone
      | Block::Granite
      | Block::Diorite
      | Block::Andesite
      | Block::Limestone
      | Block::Slate
      | Block::Basalt
      | Block::Bedrock
      | Block::CoalOre
      | Block::IronOre
      | Block::CopperOre
      | Block::TinOre
      | Block::GoldOre
      | Block::DiamondOre
      | Block::PlatinumOre => 0.6,
      Block::Cobblestone
      | Block::GraniteCobble
      | Block::DioriteCobble
      | Block::AndesiteCobble
      | Block::LimestoneCobble
      | Block::SlateCobble
      | Block::MossyBrick
      | Block::CrackedBrick
      | Block::Wool => 0.4,
      Block::Ice
      | Block::Lumen
      | Block::AncientBrick
      | Block::CarvedStone
      | Block::CrimsonRune => 0.25,
      _ => 0.0
    }
  }

  pub fn fluid(self) -> bool { self.liquid().is_some() }

  pub fn solid(self) -> bool {
    self != Block::Air
      && !self.tool()
      && !self.fluid()
      && !self.modelled()
      && self != Block::Cobweb
  }

  pub fn targetable(self) -> bool {
    self.solid() || self.modelled() || self == Block::Cobweb
  }

  pub fn breakable(self) -> bool {
    self.targetable()
      && self != Block::Bedrock
      && !self.waystone()
      && self != Block::Chest
      && self != Block::OilSeep
  }

  pub fn seconds_to_break(self) -> f32 {
    match self {
      Block::Glass => 0.3,
      Block::Cobweb | Block::Urn => 0.25,
      Block::BonePile => 0.8,
      Block::Brazier | Block::EmptyChest => 1.0,
      Block::AncientBrick
      | Block::MossyBrick
      | Block::CrackedBrick
      | Block::CarvedStone => 3.0,
      Block::Lumen => 2.2,
      Block::PrecursorAlloy => 12.0,
      Block::AetherConduit => 7.0,
      Block::PrecursorGlass => 2.5,
      Block::AetherLamp => 1.5,
      Block::Abyssite => 3.0,
      Block::VentStone | Block::NoduleOre | Block::Nacre => 2.5,
      Block::Reefstone | Block::BioLab => 2.0,
      Block::BrainCoral | Block::Biolume => 0.8,
      Block::Sponge => 0.5,
      Block::Kelp | Block::Seagrass | Block::Glowkelp => 0.05,
      coral if coral.organism() => 0.4,
      sign if sign.sign() => 0.6,
      vessel if vessel.vessel() => 1.5,
      flower if flower.bloomer() => 0.3,
      ladder if ladder.ladder() => 0.5,
      belt if belt.conveyor() => 0.5,
      bed if bed.bed() => 0.4,
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
      Block::Wool => 0.5,
      Block::CraftingTable => 1.5,
      Block::Furnace => 2.5,
      pipe if pipe.pipe() || pipe == Block::CopperCable => 0.4,
      Block::ElectricLamp | Block::ElectricLampLit | Block::SolarPanel => 0.8,
      machine if machine.machine() => 2.0,
      Block::DiamondOre | Block::GoldOre | Block::PlatinumOre => 3.0,
      _ => 2.0
    }
  }

  pub fn drop(self) -> Block {
    Block::ROCKS.iter().find(|&&(stone, _)| stone == self).map_or(
      match self {
        Block::Grass | Block::Mycelium => Block::Dirt,
        Block::Urn => Block::Air,
        Block::EmptyChest => Block::Planks,
        Block::CrackedBrick => Block::AncientBrick,
        Block::PrecursorAlloy | Block::AetherConduit => Block::AlloyScrap,
        other => other.held()
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
      Block::StoneBricks => "Stone Bricks",
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
      Block::GraniteCobble => "Granite Cobble",
      Block::Diorite => "Diorite",
      Block::DioriteCobble => "Diorite Cobble",
      Block::Andesite => "Andesite",
      Block::AndesiteCobble => "Andesite Cobble",
      Block::Limestone => "Limestone",
      Block::LimestoneCobble => "Limestone Cobble",
      Block::Slate => "Slate",
      Block::SlateCobble => "Slate Cobble",
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
      Block::PalmSign => "Palm Sign",
      Block::TallGrass => "Tall Grass",
      Block::Ladder => "Ladder",
      Block::LadderEast => "Ladder",
      Block::LadderSouth => "Ladder",
      Block::LadderWest => "Ladder",
      Block::Bed => "Bed",
      Block::Wool => "Wool",
      Block::Feather => "Feather",
      Block::Charcoal => "Charcoal",
      Block::FishingRod => "Fishing Rod",
      Block::Cod => "Cod",
      Block::Salmon => "Salmon",
      Block::TropicalFish => "Tropical Fish",
      Block::Pufferfish => "Pufferfish",
      Block::BedEast => "Bed",
      Block::BedSouth => "Bed",
      Block::BedWest => "Bed",
      Block::BedHead => "Bed",
      Block::BedHeadEast => "Bed",
      Block::BedHeadSouth => "Bed",
      Block::BedHeadWest => "Bed",
      Block::AncientBrick => "Ancient Bricks",
      Block::MossyBrick => "Mossy Ancient Bricks",
      Block::CrackedBrick => "Cracked Ancient Bricks",
      Block::CarvedStone => "Carved Runestone",
      Block::Lumen => "Lumen Crystal",
      Block::BonePile => "Bone Pile",
      Block::Cobweb => "Cobweb",
      Block::Brazier => "Brazier",
      Block::Urn => "Burial Urn",
      Block::Chest => "Chest",
      Block::EmptyChest => "Empty Chest",
      Block::ArcaneDust => "Arcane Dust",
      Block::BoneDust => "Bone Dust",
      Block::LizardClaw => "Lizard Claw",
      Block::Candle => "Candle",
      Block::SpellFirebolt => "Fire Bolt",
      Block::SpellFrost => "Ray of Frost",
      Block::SpellMissile => "Magic Missile",
      Block::SpellHeal => "Cure Wounds",
      Block::SpellHaste => "Haste",
      Block::SpellStoneskin => "Stoneskin",
      Block::SpellLeap => "Jump",
      Block::SpellLight => "Dancing Light",
      Block::SpellThunder => "Thunderwave",
      Block::SpellFireball => "Fireball",
      Block::SpellIceStorm => "Ice Storm",
      Block::SpellAnimate => "Animate Dead",
      Block::SpellGuardian => "Spirit Guardian",
      Block::SpellInvisibility => "Invisibility",
      Block::Pickaxe => "Pickaxe",
      Block::Axe => "Axe",
      Block::Shovel => "Shovel",
      Block::Sword => "Sword",
      Block::Hat => "Hat",
      Block::Amulet => "Amulet",
      Block::Mail => "Mail",
      Block::Belt => "Belt",
      Block::Boots => "Boots",
      Block::Ring => "Ring",
      Block::WallTorch
      | Block::WallTorchEast
      | Block::WallTorchSouth
      | Block::WallTorchWest => "Torch",
      Block::Balloon => "Hot Air Balloon",
      Block::OilSeep => "Oil Seep",
      Block::CopperPipe => "Copper Pipe",
      Block::IronPipe => "Iron Pipe",
      Block::GoldPipe => "Gold Pipe",
      Block::DiamondPipe => "Diamond Pipe",
      Block::Tank => "Tank",
      Block::CopperCable => "Copper Cable",
      Block::Battery => "Battery",
      Block::ElectricPump => "Electric Pump",
      Block::Pumpjack => "Pumpjack",
      Block::Refinery => "Refinery",
      Block::Fermenter => "Fermenter",
      Block::SteamEngine => "Steam Engine",
      Block::BiofuelGenerator => "Biofuel Generator",
      Block::DieselGenerator => "Diesel Generator",
      Block::GeothermalGenerator => "Geothermal Generator",
      Block::SolarPanel => "Solar Panel",
      Block::WindTurbine => "Wind Turbine",
      Block::ElectricFurnace => "Electric Furnace",
      Block::ElectricLamp | Block::ElectricLampLit => "Electric Lamp",
      Block::FluidDrain => "Fluid Drain",
      Block::PipedDrain => "Piped Fluid Drain",
      Block::ItemDrain => "Item Drain",
      Block::ItemPipe => "Item Pipe",
      Block::Conveyor
      | Block::ConveyorEast
      | Block::ConveyorSouth
      | Block::ConveyorWest
      | Block::ConveyorUp
      | Block::ConveyorUpEast
      | Block::ConveyorUpSouth
      | Block::ConveyorUpWest
      | Block::ConveyorDown
      | Block::ConveyorDownEast
      | Block::ConveyorDownSouth
      | Block::ConveyorDownWest => "Conveyor Belt",
      Block::GlimmerPink => "Pink Glimmerpetal",
      Block::GlimmerCyan => "Cyan Glimmerpetal",
      Block::GlimmerViolet => "Violet Glimmerpetal",
      Block::GlimmerGold => "Golden Glimmerpetal",
      Block::BloomFont => "Bloom Font",
      Block::Sunsip => "Sunsip",
      Block::Dewcup => "Dewcup",
      Block::Emberbell => "Emberbell",
      Block::ManaCistern => "Mana Cistern",
      Block::Cauldron => "Cauldron",
      Block::CrimsonAltar => "Crimson Altar",
      Block::Plinth => "Offering Plinth",
      Block::ConfluenceAltar => "Confluence Altar",
      Block::CrimsonRune => "Crimson Rune",
      Block::Glimsteel => "Glimsteel",
      Block::Starheart => "Starheart",
      Block::Tidepearl => "Tidepearl",
      Block::Verdantine => "Verdantine",
      Block::Hexbrass => "Hexbrass",
      Block::MithrilIngot => "Mithril Ingot",
      Block::BareTablet => "Bare Tablet",
      Block::BoundTablet => "Bound Tablet",
      Block::Heartstone => "Heartstone",
      Block::RitualKnife => "Ritual Knife",
      Block::WellspringSigil => "Wellspring Sigil",
      Block::PlatinumOre => "Platinum Ore",
      Block::Platinum => "Platinum",
      Block::PlatinumDust => "Platinum Dust",
      Block::Moonlace => "Moonlace",
      Block::Crusher => "Crusher",
      Block::Satchel => "Satchel",
      Block::Rucksack => "Riveted Rucksack",
      Block::ExpeditionPack => "Expedition Pack",
      Block::LesserRift => "Lesser Pocket Rift",
      Block::GreaterRift => "Greater Pocket Rift",
      Block::Kelp => "Kelp",
      Block::Seagrass => "Seagrass",
      Block::BrainCoral => "Brain Coral",
      Block::FanCoral => "Fan Coral",
      Block::TubeCoral => "Tube Coral",
      Block::Anemone => "Sea Anemone",
      Block::GlowPolyp => "Glow Polyp",
      Block::VentWorms => "Vent Worms",
      Block::Sponge => "Sea Sponge",
      Block::Clam => "Giant Clam",
      Block::Pearl => "Pearl",
      Block::Abyssite => "Abyssite",
      Block::VentStone => "Vent Stone",
      Block::NoduleOre => "Manganese Nodules",
      Block::PrecursorAlloy => "Sunken Alloy",
      Block::PrecursorGlass => "Sunken Glass",
      Block::AetherConduit => "Aether Conduit",
      Block::AetherLamp => "Aether Lamp",
      Block::AetherCell => "Aether Cell",
      Block::AlloyScrap => "Alloy Scrap",
      Block::ResonantCrystal => "Resonant Crystal",
      Block::Datacore => "Datacore",
      Block::JellyGel => "Jelly Gel",
      Block::BioLab => "Bio Lab",
      Block::Reefstone => "Reefstone",
      Block::Nacre => "Nacre",
      Block::Biolume => "Biolume",
      Block::Glowkelp => "Glowkelp",
      Block::Submarine => "Submarine",
      Block::SonarModule => "Sonar Module",
      Block::DeepSonar => "Deep Resonance Sonar",
      Block::DrillModule => "Drill Arm",
      Block::PlasmaDrill => "Aether Lance",
      Block::Floodlight => "Floodlight",
      Block::Thruster => "Thruster",
      Block::Hydrojet => "Aether Hydrojet",
      Block::HullPlating => "Hull Plating",
      Block::ResonantPlating => "Resonant Plating",
      Block::SampleClaw => "Sample Claw",
      Block::TorpedoBay => "Torpedo Bay",
      Block::PowerCell => "Power Cell",
      Block::Torpedo => "Torpedo"
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
      Block::PalmSign => all(Tile::PalmSign),
      Block::TallGrass => all(Tile::TallGrass),
      Block::Ladder => all(Tile::Ladder),
      Block::LadderEast => all(Tile::Ladder),
      Block::LadderSouth => all(Tile::Ladder),
      Block::LadderWest => all(Tile::Ladder),
      Block::Bed => all(Tile::Bed),
      Block::Wool => all(Tile::Wool),
      Block::Feather => all(Tile::Feather),
      Block::Charcoal => all(Tile::Charcoal),
      Block::FishingRod => all(Tile::FishingRod),
      Block::Cod => all(Tile::Cod),
      Block::Salmon => all(Tile::Salmon),
      Block::TropicalFish => all(Tile::TropicalFish),
      Block::Pufferfish => all(Tile::Pufferfish),
      Block::BedEast => all(Tile::Bed),
      Block::BedSouth => all(Tile::Bed),
      Block::BedWest => all(Tile::Bed),
      Block::BedHead => all(Tile::Bed),
      Block::BedHeadEast => all(Tile::Bed),
      Block::BedHeadSouth => all(Tile::Bed),
      Block::BedHeadWest => all(Tile::Bed),
      Block::AncientBrick => all(Tile::AncientBrick),
      Block::MossyBrick => [Tile::MossyBrick, Tile::MossyBrick, Tile::AncientBrick],
      Block::CrackedBrick => all(Tile::CrackedBrick),
      Block::CarvedStone => [Tile::AncientBrick, Tile::CarvedStone, Tile::AncientBrick],
      Block::Lumen => all(Tile::Lumen),
      Block::BonePile => all(Tile::BonePile),
      Block::Cobweb => all(Tile::Cobweb),
      Block::Brazier => all(Tile::Flame),
      Block::Urn => all(Tile::Terracotta),
      Block::Chest | Block::EmptyChest => all(Tile::Planks),
      Block::ArcaneDust => all(Tile::ArcaneDust),
      Block::BoneDust => all(Tile::BoneDust),
      Block::LizardClaw => all(Tile::LizardClaw),
      Block::Candle => all(Tile::Candle),
      Block::SpellFirebolt => all(Tile::SpellFirebolt),
      Block::SpellFrost => all(Tile::SpellFrost),
      Block::SpellMissile => all(Tile::SpellMissile),
      Block::SpellHeal => all(Tile::SpellHeal),
      Block::SpellHaste => all(Tile::SpellHaste),
      Block::SpellStoneskin => all(Tile::SpellStoneskin),
      Block::SpellLeap => all(Tile::SpellLeap),
      Block::SpellLight => all(Tile::SpellLight),
      Block::SpellThunder => all(Tile::SpellThunder),
      Block::SpellFireball => all(Tile::SpellFireball),
      Block::SpellIceStorm => all(Tile::SpellIceStorm),
      Block::SpellAnimate => all(Tile::SpellAnimate),
      Block::SpellGuardian => all(Tile::SpellGuardian),
      Block::SpellInvisibility => all(Tile::SpellInvisibility),
      Block::Pickaxe => all(Tile::Pickaxe),
      Block::Axe => all(Tile::Axe),
      Block::Shovel => all(Tile::Shovel),
      Block::Sword => all(Tile::Sword),
      Block::Hat => all(Tile::Hat),
      Block::Amulet => all(Tile::Amulet),
      Block::Mail => all(Tile::Mail),
      Block::Belt => all(Tile::Belt),
      Block::Boots => all(Tile::Boots),
      Block::Ring => all(Tile::Ring),
      Block::WallTorch
      | Block::WallTorchEast
      | Block::WallTorchSouth
      | Block::WallTorchWest => all(Tile::Torch),
      Block::Balloon => all(Tile::Balloon),
      Block::OilSeep => [Tile::OilSeep, Tile::Tar, Tile::Tar],
      Block::CopperPipe | Block::Tank | Block::CopperCable => all(Tile::Copper),
      Block::IronPipe | Block::GoldPipe | Block::DiamondPipe => all(Tile::Steel),
      Block::Battery => all(Tile::Cells),
      Block::SolarPanel => [Tile::Solar, Tile::Steel, Tile::Steel],
      Block::Pumpjack => all(Tile::Hazard),
      Block::Fermenter => all(Tile::Planks),
      Block::SteamEngine => all(Tile::Copper),
      Block::GeothermalGenerator | Block::DieselGenerator => all(Tile::Grille),
      Block::ElectricFurnace => all(Tile::Coil),
      Block::ElectricLamp => all(Tile::Bulb),
      Block::ElectricLampLit => all(Tile::BulbLit),
      Block::FluidDrain | Block::PipedDrain => all(Tile::Copper),
      Block::ItemDrain | Block::ItemPipe => all(Tile::Steel),
      Block::Conveyor
      | Block::ConveyorEast
      | Block::ConveyorSouth
      | Block::ConveyorWest
      | Block::ConveyorUp
      | Block::ConveyorUpEast
      | Block::ConveyorUpSouth
      | Block::ConveyorUpWest
      | Block::ConveyorDown
      | Block::ConveyorDownEast
      | Block::ConveyorDownSouth
      | Block::ConveyorDownWest => all(Tile::Rubber),
      Block::GlimmerPink => all(Tile::GlimmerPink),
      Block::GlimmerCyan => all(Tile::GlimmerCyan),
      Block::GlimmerViolet => all(Tile::GlimmerViolet),
      Block::GlimmerGold => all(Tile::GlimmerGold),
      Block::BloomFont => all(Tile::Moonstone),
      Block::Sunsip => all(Tile::Sunsip),
      Block::Dewcup => all(Tile::Dewcup),
      Block::Emberbell => all(Tile::Emberbell),
      Block::ManaCistern => all(Tile::Moonstone),
      Block::Cauldron => all(Tile::Steel),
      Block::CrimsonAltar => all(Tile::CrimsonRune),
      Block::Plinth => all(Tile::Moonstone),
      Block::ConfluenceAltar => all(Tile::Runework),
      Block::CrimsonRune => [Tile::CrimsonRune, Tile::CrimsonRune, Tile::Slate],
      Block::Glimsteel => all(Tile::Glimsteel),
      Block::Starheart => all(Tile::Starheart),
      Block::Tidepearl => all(Tile::Tidepearl),
      Block::Verdantine => all(Tile::Verdantine),
      Block::Hexbrass => all(Tile::Hexbrass),
      Block::MithrilIngot => all(Tile::MithrilIngot),
      Block::BareTablet => all(Tile::BareTablet),
      Block::BoundTablet => all(Tile::BoundTablet),
      Block::Heartstone => all(Tile::Heartstone),
      Block::RitualKnife => all(Tile::RitualKnife),
      Block::WellspringSigil => all(Tile::WellspringSigil),
      Block::PlatinumOre => all(Tile::PlatinumOre),
      Block::Platinum => all(Tile::Platinum),
      Block::PlatinumDust => all(Tile::PlatinumDust),
      Block::Moonlace => all(Tile::Moonlace),
      Block::Crusher => all(Tile::Crusher),
      Block::Satchel => all(Tile::Satchel),
      Block::Rucksack => all(Tile::Rucksack),
      Block::ExpeditionPack => all(Tile::ExpeditionPack),
      Block::LesserRift => all(Tile::LesserRift),
      Block::GreaterRift => all(Tile::GreaterRift),
      Block::StoneBricks => all(Tile::StoneBricks),
      Block::Kelp => all(Tile::Kelp),
      Block::Seagrass => all(Tile::Seagrass),
      Block::BrainCoral => all(Tile::BrainCoral),
      Block::FanCoral => all(Tile::FanCoral),
      Block::TubeCoral => all(Tile::TubeCoral),
      Block::Anemone => all(Tile::Anemone),
      Block::GlowPolyp => all(Tile::GlowPolyp),
      Block::VentWorms => all(Tile::VentWorms),
      Block::Sponge => all(Tile::Sponge),
      Block::Clam => all(Tile::Clam),
      Block::Pearl => all(Tile::Pearl),
      Block::Abyssite => all(Tile::Abyssite),
      Block::VentStone => all(Tile::VentStone),
      Block::NoduleOre => all(Tile::NoduleOre),
      Block::PrecursorAlloy => all(Tile::PrecursorAlloy),
      Block::PrecursorGlass => all(Tile::PrecursorGlass),
      Block::AetherConduit => all(Tile::AetherConduit),
      Block::AetherLamp => all(Tile::AetherLamp),
      Block::AetherCell => all(Tile::AetherCell),
      Block::AlloyScrap => all(Tile::AlloyScrap),
      Block::ResonantCrystal => all(Tile::ResonantCrystal),
      Block::Datacore => all(Tile::Datacore),
      Block::JellyGel => all(Tile::JellyGel),
      Block::BioLab => [Tile::BioLabTop, Tile::BioLabSide, Tile::BioLabSide],
      Block::Reefstone => all(Tile::Reefstone),
      Block::Nacre => all(Tile::Nacre),
      Block::Biolume => all(Tile::Biolume),
      Block::Glowkelp => all(Tile::Glowkelp),
      Block::Submarine => all(Tile::Submarine),
      Block::SonarModule => all(Tile::SonarModule),
      Block::DeepSonar => all(Tile::DeepSonar),
      Block::DrillModule => all(Tile::DrillModule),
      Block::PlasmaDrill => all(Tile::PlasmaDrill),
      Block::Floodlight => all(Tile::Floodlight),
      Block::Thruster => all(Tile::Thruster),
      Block::Hydrojet => all(Tile::Hydrojet),
      Block::HullPlating => all(Tile::HullPlating),
      Block::ResonantPlating => all(Tile::ResonantPlating),
      Block::SampleClaw => all(Tile::SampleClaw),
      Block::TorpedoBay => all(Tile::TorpedoBay),
      Block::PowerCell => all(Tile::PowerCell),
      Block::Torpedo => all(Tile::Torpedo),
      Block::ElectricPump
      | Block::Refinery
      | Block::BiofuelGenerator
      | Block::WindTurbine => all(Tile::Casing)
    }
  }
}
