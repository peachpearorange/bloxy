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
)]
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
  ItemDrain
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
  Tar
}

impl Tile {
  pub const ALL: [Tile; 140] = [
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
    Tile::WaystoneTop,
    Tile::Grain,
    Tile::Poppy,
    Tile::Dandelion,
    Tile::Cornflower,
    Tile::Daisy,
    Tile::OakSapling,
    Tile::BirchSapling,
    Tile::SpruceSapling,
    Tile::PalmSapling,
    Tile::RedMushroom,
    Tile::BrownMushroom,
    Tile::Granite,
    Tile::GraniteCobble,
    Tile::Diorite,
    Tile::DioriteCobble,
    Tile::Andesite,
    Tile::AndesiteCobble,
    Tile::Limestone,
    Tile::LimestoneCobble,
    Tile::Slate,
    Tile::SlateCobble,
    Tile::WaystoneStone,
    Tile::Flame,
    Tile::Torch,
    Tile::TableTop,
    Tile::TableSide,
    Tile::FurnaceTop,
    Tile::FurnaceSide,
    Tile::Boat,
    Tile::Bucket,
    Tile::WaterBucket,
    Tile::LavaBucket,
    Tile::OakSign,
    Tile::BirchSign,
    Tile::SpruceSign,
    Tile::PalmSign,
    Tile::TallGrass,
    Tile::Ladder,
    Tile::Bed,
    Tile::Wool,
    Tile::Feather,
    Tile::FishingRod,
    Tile::Cod,
    Tile::Salmon,
    Tile::TropicalFish,
    Tile::Pufferfish,
    Tile::Charcoal,
    Tile::AncientBrick,
    Tile::MossyBrick,
    Tile::CrackedBrick,
    Tile::CarvedStone,
    Tile::Lumen,
    Tile::BonePile,
    Tile::Cobweb,
    Tile::Terracotta,
    Tile::ArcaneDust,
    Tile::BoneDust,
    Tile::LizardClaw,
    Tile::Candle,
    Tile::SpellFirebolt,
    Tile::SpellFrost,
    Tile::SpellMissile,
    Tile::SpellHeal,
    Tile::SpellHaste,
    Tile::SpellStoneskin,
    Tile::SpellLeap,
    Tile::SpellLight,
    Tile::SpellThunder,
    Tile::SpellFireball,
    Tile::SpellIceStorm,
    Tile::SpellAnimate,
    Tile::SpellGuardian,
    Tile::SpellInvisibility,
    Tile::Pickaxe,
    Tile::Axe,
    Tile::Shovel,
    Tile::Sword,
    Tile::Hat,
    Tile::Amulet,
    Tile::Mail,
    Tile::Belt,
    Tile::Boots,
    Tile::Ring,
    Tile::Balloon,
    Tile::Copper,
    Tile::Steel,
    Tile::PipeGlass,
    Tile::Casing,
    Tile::Grille,
    Tile::Solar,
    Tile::Cable,
    Tile::OilSeep,
    Tile::Cells,
    Tile::Coil,
    Tile::Bulb,
    Tile::BulbLit,
    Tile::Hazard,
    Tile::Tar
  ];

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
  pub const ALL: [Block; 168] = [
    Block::Air,
    Block::Stone,
    Block::Cobblestone,
    Block::Dirt,
    Block::Grass,
    Block::Sand,
    Block::Gravel,
    Block::Clay,
    Block::Snow,
    Block::Bedrock,
    Block::Log,
    Block::Leaves,
    Block::Planks,
    Block::Glass,
    Block::Water,
    Block::CoalOre,
    Block::IronOre,
    Block::CopperOre,
    Block::TinOre,
    Block::GoldOre,
    Block::DiamondOre,
    Block::Lamp,
    Block::Bricks,
    Block::BirchLog,
    Block::BirchLeaves,
    Block::SpruceLog,
    Block::SpruceLeaves,
    Block::PalmLog,
    Block::PalmLeaves,
    Block::Mycelium,
    Block::MushroomStem,
    Block::RedCap,
    Block::BrownCap,
    Block::Basalt,
    Block::Lava,
    Block::Ice,
    Block::Waystone,
    Block::Poppy,
    Block::Dandelion,
    Block::Cornflower,
    Block::Daisy,
    Block::OakSapling,
    Block::BirchSapling,
    Block::SpruceSapling,
    Block::PalmSapling,
    Block::RedMushroom,
    Block::BrownMushroom,
    Block::Granite,
    Block::GraniteCobble,
    Block::Diorite,
    Block::DioriteCobble,
    Block::Andesite,
    Block::AndesiteCobble,
    Block::Limestone,
    Block::LimestoneCobble,
    Block::Slate,
    Block::SlateCobble,
    Block::WaystoneTop,
    Block::Torch,
    Block::CraftingTable,
    Block::Furnace,
    Block::Boat,
    Block::WaterFlow1,
    Block::WaterFlow2,
    Block::WaterFlow3,
    Block::WaterFlow4,
    Block::WaterFlow5,
    Block::WaterFlow6,
    Block::WaterFlow7,
    Block::LavaFlow1,
    Block::LavaFlow2,
    Block::LavaFlow3,
    Block::Bucket,
    Block::WaterBucket,
    Block::LavaBucket,
    Block::OakSign,
    Block::BirchSign,
    Block::SpruceSign,
    Block::PalmSign,
    Block::TallGrass,
    Block::Ladder,
    Block::LadderEast,
    Block::LadderSouth,
    Block::LadderWest,
    Block::Bed,
    Block::Wool,
    Block::Feather,
    Block::FishingRod,
    Block::Cod,
    Block::Salmon,
    Block::TropicalFish,
    Block::Pufferfish,
    Block::BedEast,
    Block::BedSouth,
    Block::BedWest,
    Block::BedHead,
    Block::BedHeadEast,
    Block::BedHeadSouth,
    Block::BedHeadWest,
    Block::Charcoal,
    Block::AncientBrick,
    Block::MossyBrick,
    Block::CrackedBrick,
    Block::CarvedStone,
    Block::Lumen,
    Block::BonePile,
    Block::Cobweb,
    Block::Brazier,
    Block::Urn,
    Block::Chest,
    Block::EmptyChest,
    Block::ArcaneDust,
    Block::BoneDust,
    Block::LizardClaw,
    Block::Candle,
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
    Block::SpellInvisibility,
    Block::Pickaxe,
    Block::Axe,
    Block::Shovel,
    Block::Sword,
    Block::Hat,
    Block::Amulet,
    Block::Mail,
    Block::Belt,
    Block::Boots,
    Block::Ring,
    Block::WallTorch,
    Block::WallTorchEast,
    Block::WallTorchSouth,
    Block::WallTorchWest,
    Block::Balloon,
    Block::OilSeep,
    Block::CopperPipe,
    Block::IronPipe,
    Block::GoldPipe,
    Block::DiamondPipe,
    Block::Tank,
    Block::CopperCable,
    Block::Battery,
    Block::ElectricPump,
    Block::Pumpjack,
    Block::Refinery,
    Block::Fermenter,
    Block::SteamEngine,
    Block::BiofuelGenerator,
    Block::DieselGenerator,
    Block::GeothermalGenerator,
    Block::SolarPanel,
    Block::WindTurbine,
    Block::ElectricFurnace,
    Block::ElectricLamp,
    Block::ElectricLampLit,
    Block::FluidDrain,
    Block::PipedDrain,
    Block::ItemDrain
  ];

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
      || self.spell()
      || self.forged()
      || self.worn()
  }

  pub const TOOLS: [Block; 4] = [Block::Pickaxe, Block::Axe, Block::Shovel, Block::Sword];

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
    ) || self.machine()
  }

  pub const PIPES: [Block; 4] =
    [Block::CopperPipe, Block::IronPipe, Block::GoldPipe, Block::DiamondPipe];

  pub fn pipe(self) -> bool { Block::PIPES.contains(&self) }

  pub fn machine(self) -> bool { (Block::CopperPipe..=Block::ItemDrain).contains(&self) }

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
    self.torch() || self == Block::Brazier || self == Block::ElectricLampLit
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
      _ => self
    }
  }

  pub fn placeable(self) -> bool { self.item() && !self.tool() }

  pub fn stack(self) -> u16 {
    match self {
      Block::WaterBucket | Block::LavaBucket | Block::Boat | Block::FishingRod => 1,
      Block::Balloon => 1,
      Block::Bed => 1,
      gear if gear.forged() || gear.worn() => 1,
      Block::Bucket => 16,
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
    matches!(self, Block::CraftingTable | Block::Furnace | Block::ElectricFurnace)
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
      Block::Glass | Block::Cobweb => Look::Cutout,
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
    self.plant() || self.torch() || self.sign() || self.ladder() || self.bed()
  }

  pub fn plant(self) -> bool {
    (Block::Poppy..=Block::BrownMushroom).contains(&self) || self == Block::TallGrass
  }

  pub fn opaque(self) -> bool { self.look() == Look::Opaque }

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
      | Block::PalmLeaves => 1.0,
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
      | Block::DiamondOre => 0.6,
      Block::Cobblestone
      | Block::GraniteCobble
      | Block::DioriteCobble
      | Block::AndesiteCobble
      | Block::LimestoneCobble
      | Block::SlateCobble
      | Block::MossyBrick
      | Block::CrackedBrick
      | Block::Wool => 0.4,
      Block::Ice | Block::Lumen | Block::AncientBrick | Block::CarvedStone => 0.25,
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
      sign if sign.sign() => 0.6,
      ladder if ladder.ladder() => 0.5,
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
      Block::DiamondOre | Block::GoldOre => 3.0,
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
      Block::ItemDrain => "Item Drain"
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
      Block::ItemDrain => all(Tile::Steel),
      Block::ElectricPump
      | Block::Refinery
      | Block::BiofuelGenerator
      | Block::WindTurbine => all(Tile::Casing)
    }
  }
}
