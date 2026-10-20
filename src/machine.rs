use {crate::{authority::{Controller, player_of, within_reach},
             balloon::burns,
             block::{Block, Fluid},
             island::SEA,
             opts::opts,
             protocol::{Altered, Avatar, Inventory, Put, Stack, authority},
             voxels::Voxels,
             weather::{Sky, Weather}},
     bevy::{platform::collections::{HashMap, HashSet},
            prelude::*},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

pub const SIDES: [IVec3; 6] =
  [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Liquid {
  Water,
  Lava,
  Crude,
  Diesel,
  Biofuel,
  Mana,
  Lifeblood
}

impl Liquid {
  pub const ALL: [Liquid; 7] = [
    Liquid::Water,
    Liquid::Lava,
    Liquid::Crude,
    Liquid::Diesel,
    Liquid::Biofuel,
    Liquid::Mana,
    Liquid::Lifeblood
  ];

  pub fn name(self) -> &'static str {
    match self {
      Liquid::Water => "Water",
      Liquid::Lava => "Lava",
      Liquid::Crude => "Crude Oil",
      Liquid::Diesel => "Diesel",
      Liquid::Biofuel => "Biofuel",
      Liquid::Mana => "Mana",
      Liquid::Lifeblood => "Lifeblood"
    }
  }

  pub fn color(self) -> Color {
    match self {
      Liquid::Water => Color::srgb(0.16, 0.42, 0.86),
      Liquid::Lava => Color::srgb(1.0, 0.42, 0.08),
      Liquid::Crude => Color::srgb(0.07, 0.05, 0.04),
      Liquid::Diesel => Color::srgb(0.86, 0.62, 0.16),
      Liquid::Biofuel => Color::srgb(0.42, 0.72, 0.16),
      Liquid::Mana => Color::srgb(0.35, 0.78, 1.0),
      Liquid::Lifeblood => Color::srgb(0.55, 0.02, 0.05)
    }
  }

  pub fn glow(self) -> f32 {
    match self {
      Liquid::Lava => 6.0,
      Liquid::Biofuel => 0.15,
      Liquid::Mana => 1.6,
      Liquid::Lifeblood => 0.4,
      _ => 0.0
    }
  }

  pub fn of(fluid: Fluid) -> Liquid {
    match fluid {
      Fluid::Water => Liquid::Water,
      Fluid::Lava => Liquid::Lava
    }
  }

  pub fn bucket(self) -> Option<Block> {
    match self {
      Liquid::Water => Some(Block::WaterBucket),
      Liquid::Lava => Some(Block::LavaBucket),
      _ => None
    }
  }
}

pub const BUCKET: f32 = 1000.0;
pub const TANK: f32 = 16.0 * BUCKET;
const TANK_RATE: f32 = 2000.0;
const DIRECT_RATE: f32 = 200.0;
const TICK: f32 = 0.25;
const STEAM_WATER: f32 = 10.0;
const BIOFUEL_BURN: f32 = 10.0;
const DIESEL_BURN: f32 = 6.0;
const LAVA_BURN: f32 = 2.0;
const PUMPED: f32 = 100.0;
const CRUDE_FLOW: f32 = 8.0;
const REFINED: f32 = 10.0;
const DIESEL_YIELD: f32 = 0.8;
const FERMENTED: f32 = 5.0;
const BATTERY_RATE: f32 = 200.0;
pub const SMELT: f32 = 1000.0;
const DRAINED: f32 = 50.0;
const PIPED_DRAIN: f32 = 200.0;
const ITEM_EVERY: f32 = 1.0;
const SKY_CLEARANCE: i32 = 24;
const WINDY_HEIGHT: f32 = 48.0;
const LIT_FROM: f32 = 0.5;

pub fn bore(block: Block) -> Option<(f32, f32)> {
  match block {
    Block::CopperPipe => Some((100.0, 100.0)),
    Block::IronPipe => Some((200.0, 400.0)),
    Block::GoldPipe => Some((400.0, 1600.0)),
    Block::DiamondPipe => Some((800.0, 6400.0)),
    _ => None
  }
}

#[derive(Clone, Copy, Default)]
pub struct Design {
  pub intake: Option<(&'static [Liquid], f32)>,
  pub outlet: Option<f32>,
  pub draws: f32,
  pub makes: f32,
  pub stores: f32,
  pub hopper: f32
}

pub fn design(block: Block) -> Design {
  let base = Design::default();
  match block {
    Block::ElectricPump => Design { outlet: Some(4000.0), draws: 20.0, ..base },
    Block::Pumpjack => Design { outlet: Some(4000.0), draws: 30.0, ..base },
    Block::Refinery => Design {
      intake: Some((&[Liquid::Crude], 4000.0)),
      outlet: Some(4000.0),
      draws: 40.0,
      ..base
    },
    Block::Fermenter => Design { outlet: Some(4000.0), hopper: 4000.0, ..base },
    Block::SteamEngine => Design {
      intake: Some((&[Liquid::Water], 4000.0)),
      makes: 40.0,
      hopper: 600.0,
      ..base
    },
    Block::BiofuelGenerator => {
      Design { intake: Some((&[Liquid::Biofuel], 4000.0)), makes: 60.0, ..base }
    }
    Block::DieselGenerator => {
      Design { intake: Some((&[Liquid::Diesel], 4000.0)), makes: 120.0, ..base }
    }
    Block::GeothermalGenerator => {
      Design { intake: Some((&[Liquid::Lava], 4000.0)), makes: 30.0, ..base }
    }
    Block::SolarPanel => Design { makes: 12.0, ..base },
    Block::WindTurbine => Design { makes: 30.0, ..base },
    Block::Battery => Design { stores: 50_000.0, ..base },
    Block::ElectricFurnace => Design { draws: 50.0, stores: 4.0 * SMELT, ..base },
    Block::ElectricLamp | Block::ElectricLampLit => Design { draws: 2.0, ..base },
    Block::PipedDrain => Design { outlet: Some(PIPED_DRAIN), ..base },
    Block::ManaCistern => {
      Design { intake: Some((&[Liquid::Mana], crate::arcana::POOL)), ..base }
    }
    Block::Cauldron => Design { intake: Some((&[Liquid::Water], BUCKET)), ..base },
    Block::CrimsonAltar => {
      Design { intake: Some((&[Liquid::Lifeblood], crate::arcana::ALTAR_MOST)), ..base }
    }
    Block::Emberbell => Design { hopper: 600.0, ..base },
    _ => base
  }
}

pub fn pooled(block: Block) -> bool { block.pipe() || block == Block::Tank }

pub fn plumbed(block: Block) -> bool {
  let Design { intake, outlet, .. } = design(block);
  pooled(block) || intake.is_some() || outlet.is_some()
}

pub fn wired(block: Block) -> bool {
  let Design { draws, makes, stores, .. } = design(block);
  block == Block::CopperCable || draws > 0.0 || makes > 0.0 || stores > 0.0
}

pub fn carries(block: Block) -> bool {
  matches!(
    block,
    Block::ItemPipe
      | Block::Chest
      | Block::EmptyChest
      | Block::ItemDrain
      | Block::SteamEngine
      | Block::Fermenter
  ) || block.conveyor()
}

pub fn side_of(toward: IVec3) -> usize {
  SIDES.iter().position(|&side| side == toward).unwrap_or(0)
}

pub fn joins(block: Block, neighbour: Block) -> bool {
  (block == Block::ItemPipe && carries(neighbour))
    || ((block.pipe() || block == Block::PipedDrain) && plumbed(neighbour))
    || (block == Block::CopperCable && wired(neighbour))
}

pub fn container(drain: Block, near: Block) -> bool {
  let Design { intake, outlet, .. } = design(near);
  match drain {
    Block::ItemDrain => matches!(near, Block::Chest | Block::EmptyChest),
    Block::FluidDrain | Block::PipedDrain => {
      !near.drain() && (near == Block::Tank || intake.is_some() || outlet.is_some())
    }
    _ => false
  }
}

const ATTACHING: [usize; 6] = [2, 5, 0, 4, 1, 3];

pub fn attached(drain: Block, near: impl Fn(IVec3) -> Block) -> Option<usize> {
  ATTACHING.into_iter().find(|&side| drain.drain() && container(drain, near(SIDES[side])))
}

pub fn biomass(block: Block) -> Option<f32> {
  match block {
    leaves if leaves.leafy() => Some(25.0),
    Block::TallGrass => Some(20.0),
    Block::Poppy | Block::Dandelion | Block::Cornflower | Block::Daisy => Some(30.0),
    Block::OakSapling
    | Block::BirchSapling
    | Block::SpruceSapling
    | Block::PalmSapling => Some(40.0),
    Block::RedMushroom | Block::BrownMushroom => Some(50.0),
    Block::RedCap | Block::BrownCap => Some(120.0),
    Block::MushroomStem => Some(80.0),
    fish if fish.fish() => Some(60.0),
    _ => None
  }
}

pub fn fuel(machine: Block, item: Block) -> Option<f32> {
  match machine {
    Block::SteamEngine | Block::Emberbell => burns(item),
    Block::Fermenter => biomass(item),
    _ => None
  }
}

pub fn accepts(machine: Block, item: Block) -> bool {
  let Design { intake, outlet, .. } = design(machine);
  let carried = item.carrying().map(Liquid::of);
  (machine == Block::FluidDrain && item == Block::CopperPipe)
    || fuel(machine, item).is_some()
    || carried.is_some_and(|liquid| {
      machine == Block::Tank || intake.is_some_and(|(taken, _)| taken.contains(&liquid))
    })
    || (item == Block::Bucket && (machine == Block::Tank || outlet.is_some()))
    || (machine == Block::CrimsonAltar && item == Block::RitualKnife)
}

pub fn panelled(block: Block) -> bool {
  block.machine()
    && !block.pipe()
    && block != Block::CopperCable
    && block != Block::ItemPipe
    && !block.station()
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
pub enum Status {
  #[default]
  Idle,
  Running,
  Stocked,
  NoFuel,
  NoWater,
  NoPower,
  NoSeep,
  NoSource,
  Full,
  Dark,
  Unloaded,
  Detached,
  Drained,
  Unbound,
  NoHeat,
  NoMana,
  NoBlood,
  Empty,
  Unknown,
  Finished
}

impl Status {
  pub fn line(self) -> &'static str {
    match self {
      Status::Idle => "Idle",
      Status::Running => "Running",
      Status::Stocked => "Holding",
      Status::NoFuel => "Out of fuel",
      Status::NoWater => "Needs water piped in",
      Status::NoPower => "Needs power",
      Status::NoSeep => "Not standing on an oil seep",
      Status::NoSource => "No water or lava source beside it",
      Status::Full => "Full: pipe the output away",
      Status::Dark => "No sunlight",
      Status::Unloaded => "Nothing to power",
      Status::Detached => "Not against a container",
      Status::Drained => "Nothing left to drain",
      Status::Unbound => "No mana cistern within 6 blocks",
      Status::NoHeat => "Needs fire or lava beneath it",
      Status::NoMana => "Waiting for mana from cisterns nearby",
      Status::NoBlood => "Waiting for lifeblood",
      Status::Empty => "Nothing set on it",
      Status::Unknown => "Nothing to make from this",
      Status::Finished => "Done: right-click to take it"
    }
  }
}

pub type Held = Option<(Liquid, f32)>;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Parcel {
  pub stack: Stack,
  pub from: u8,
  pub to: u8,
  pub progress: f32
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
pub struct State {
  pub block: Block,
  pub intake: Held,
  pub outlet: Held,
  pub stock: f32,
  pub charge: f32,
  #[serde(default)]
  pub parcel: Option<Parcel>,
  #[serde(default)]
  pub item: Option<Stack>,
  #[serde(default)]
  pub essence: [u16; crate::arcana::ASPECTS],
  #[serde(default)]
  pub done: bool,
  #[serde(skip)]
  pub status: Status,
  #[serde(skip)]
  pub link: Option<IVec3>
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Grid {
  pub made: u32,
  pub wanted: u32,
  pub stored: u32,
  pub room: u32
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct Unit {
  pub at: IVec3,
  pub block: Block,
  pub intake: Option<(Liquid, u32)>,
  pub outlet: Option<(Liquid, u32)>,
  pub stock: u32,
  pub charge: u32,
  pub power: i32,
  pub grid: Grid,
  pub status: Status,
  pub parcel: Option<Parcel>,
  pub item: Option<Stack>,
  pub essence: [u16; crate::arcana::ASPECTS],
  pub link: Option<IVec3>
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Tend {
  pub at: IVec3,
  pub slot: u8
}

#[derive(Resource, Default)]
pub struct Works {
  pub states: HashMap<IVec3, State>,
  units: HashMap<IVec3, Entity>
}

impl Works {
  pub fn install(&mut self, at: IVec3, block: Block) {
    if block.machine()
      && self.states.get(&at).is_none_or(|state| state.block.held() != block.held())
    {
      self.states.insert(at, State { block, ..default() });
    }
  }

  pub fn charged(&self, at: IVec3) -> bool {
    self.states.get(&at).is_some_and(|state| state.charge >= SMELT)
  }

  pub fn smelt(&mut self, voxels: &Voxels, eye: Vec3, reach: f32) {
    if let Some(state) = self
      .states
      .iter_mut()
      .filter(|(at, state)| {
        state.block == Block::ElectricFurnace
          && state.charge >= SMELT
          && (at.as_vec3() + 0.5).distance(eye) <= reach
          && voxels.block(**at) == Some(Block::ElectricFurnace)
      })
      .min_by(|a, b| {
        (a.0.as_vec3() + 0.5)
          .distance(eye)
          .total_cmp(&(b.0.as_vec3() + 0.5).distance(eye))
      })
      .map(|(_, state)| state)
    {
      state.charge -= SMELT
    }
  }
}

pub fn amount(held: Held) -> f32 { held.map_or(0.0, |(_, amount)| amount) }

pub fn holds(held: Held, liquid: Liquid) -> f32 {
  held.filter(|&(kept, _)| kept == liquid).map_or(0.0, |(_, amount)| amount)
}

pub fn pour(held: &mut Held, liquid: Liquid, offered: f32, room: f32) -> f32 {
  let moved = match *held {
    Some((kept, _)) if kept != liquid => 0.0,
    _ => offered.min(room - amount(*held)).max(0.0)
  };
  if moved > 0.0 {
    *held = Some((liquid, amount(*held) + moved))
  }
  moved
}

pub fn spill(held: &mut Held, wanted: f32) -> f32 {
  let moved = wanted.min(amount(*held)).max(0.0);
  *held =
    held.map(|(liquid, kept)| (liquid, kept - moved)).filter(|&(_, kept)| kept > 0.01);
  moved
}

pub fn ordered(cells: impl IntoIterator<Item = IVec3>) -> Vec<IVec3> {
  let mut cells: Vec<IVec3> = cells.into_iter().collect();
  cells.sort_by_key(|at| (at.x, at.y, at.z));
  cells
}

fn groups(cells: &HashSet<IVec3>) -> Vec<Vec<IVec3>> {
  ordered(cells.iter().copied())
    .into_iter()
    .fold((Vec::new(), HashSet::<IVec3>::new()), |(mut found, mut seen), start| {
      if seen.insert(start) {
        let mut group = vec![start];
        let mut next = 0;
        while next < group.len() {
          let at = group[next];
          next += 1;
          for side in SIDES {
            let near = at + side;
            if cells.contains(&near) && seen.insert(near) {
              group.push(near)
            }
          }
        }
        found.push(ordered(group))
      }
      (found, seen)
    })
    .0
}

pub fn source(voxels: &mut Voxels, at: IVec3) -> Option<Liquid> {
  SIDES.into_iter().find_map(|side| {
    voxels
      .ensure(at + side)
      .liquid()
      .filter(|&(_, level)| level == 0)
      .map(|(fluid, _)| Liquid::of(fluid))
  })
}

pub fn sunlight(hour: f32) -> f32 {
  let arc = (hour - 6.0) / 12.0 * std::f32::consts::PI;
  arc.sin().max(0.0)
}

#[derive(Clone, Copy, Default)]
struct Load {
  offer: f32,
  want: f32,
  status: Status
}

fn load(voxels: &mut Voxels, sky: Sky, at: IVec3, state: &State) -> Load {
  let Design { makes, draws, outlet, stores, .. } = design(state.block);
  let room = outlet.map_or(0.0, |room| room - amount(state.outlet));
  let making = |ready: bool, missing: Status| match ready {
    true => Load { offer: makes, status: Status::Running, ..default() },
    false => Load { status: missing, ..default() }
  };
  let drawing = |ready: bool, missing: Status| match ready {
    true => Load { want: draws, status: Status::Running, ..default() },
    false => Load { status: missing, ..default() }
  };
  match state.block {
    Block::SteamEngine => match (state.stock > 0.0, holds(state.intake, Liquid::Water)) {
      (false, _) => making(false, Status::NoFuel),
      (true, water) => making(water >= STEAM_WATER * TICK, Status::NoWater)
    },
    Block::BiofuelGenerator => {
      making(holds(state.intake, Liquid::Biofuel) >= BIOFUEL_BURN * TICK, Status::NoFuel)
    }
    Block::DieselGenerator => {
      making(holds(state.intake, Liquid::Diesel) >= DIESEL_BURN * TICK, Status::NoFuel)
    }
    Block::GeothermalGenerator => making(
      source(voxels, at) == Some(Liquid::Lava)
        || holds(state.intake, Liquid::Lava) >= LAVA_BURN * TICK,
      Status::NoFuel
    ),
    Block::SolarPanel => {
      let open =
        (1..=SKY_CLEARANCE).all(|lift| !voxels.ensure(at + IVec3::Y * lift).solid());
      let light = sunlight(opts().hour) * sky.sun() * f32::from(u8::from(open));
      Load {
        offer: makes * light,
        status: if light > 0.0 { Status::Running } else { Status::Dark },
        ..default()
      }
    }
    Block::WindTurbine => {
      let height = ((at.y - SEA) as f32 / WINDY_HEIGHT).clamp(0.15, 1.0);
      Load { offer: makes * height * sky.wind(), status: Status::Running, ..default() }
    }
    Block::ElectricPump => match source(voxels, at) {
      None => drawing(false, Status::NoSource),
      Some(liquid) => drawing(
        room > 0.0 && state.outlet.is_none_or(|(kept, _)| kept == liquid),
        Status::Full
      )
    },
    Block::Pumpjack => match voxels.ensure(at - IVec3::Y) == Block::OilSeep {
      false => drawing(false, Status::NoSeep),
      true => drawing(room > 0.0, Status::Full)
    },
    Block::Refinery => match holds(state.intake, Liquid::Crude) > 0.0 {
      false => drawing(false, Status::NoFuel),
      true => drawing(room > 0.0, Status::Full)
    },
    Block::ElectricFurnace => drawing(state.charge < stores, Status::Full),
    Block::ElectricLamp | Block::ElectricLampLit => drawing(true, Status::Idle),
    _ => Load::default()
  }
}

#[derive(Clone, Copy, Default)]
struct Supply {
  throttle: f32,
  satisfied: f32,
  grid: Grid
}

fn power(
  voxels: &mut Voxels,
  sky: Sky,
  states: &mut HashMap<IVec3, State>
) -> HashMap<IVec3, (Load, Supply)> {
  let wires: HashSet<IVec3> =
    states.iter().filter(|(_, state)| wired(state.block)).map(|(&at, _)| at).collect();
  let loads: HashMap<IVec3, Load> = ordered(wires.iter().copied())
    .into_iter()
    .map(|at| (at, load(voxels, sky, at, &states[&at])))
    .collect();
  groups(&wires)
    .into_iter()
    .flat_map(|group| {
      let batteries: Vec<IVec3> =
        group.iter().copied().filter(|at| states[at].block == Block::Battery).collect();
      let room = |at: &IVec3| {
        let state = &states[at];
        (design(state.block).stores - state.charge).max(0.0)
      };
      let gives: Vec<f32> = batteries
        .iter()
        .map(|at| states[at].charge.min(BATTERY_RATE * TICK) / TICK)
        .collect();
      let takes: Vec<f32> =
        batteries.iter().map(|at| room(at).min(BATTERY_RATE * TICK) / TICK).collect();
      let offered: f32 = group.iter().map(|at| loads[at].offer).sum();
      let wanted: f32 = group.iter().map(|at| loads[at].want).sum();
      let (given, taken) = (gives.iter().sum::<f32>(), takes.iter().sum::<f32>());
      let (throttle, satisfied, flow) = match offered >= wanted {
        true => {
          let into = (offered - wanted).min(taken);
          let throttle = match offered > 0.0 {
            true => (wanted + into) / offered,
            false => 0.0
          };
          (throttle, 1.0, into)
        }
        false => {
          let out = (wanted - offered).min(given);
          (1.0, (offered + out) / wanted, -out)
        }
      };
      for (index, at) in batteries.iter().enumerate() {
        let share = match flow > 0.0 {
          true => takes[index] / taken.max(f32::EPSILON),
          false => gives[index] / given.max(f32::EPSILON)
        };
        if let Some(state) = states.get_mut(at) {
          state.charge = (state.charge + flow * share * TICK).max(0.0)
        }
      }
      let grid = Grid {
        made: (offered * throttle).round() as u32,
        wanted: wanted.round() as u32,
        stored: batteries.iter().map(|at| states[at].charge).sum::<f32>() as u32,
        room: batteries.iter().map(|at| design(states[at].block).stores).sum::<f32>()
          as u32
      };
      group
        .into_iter()
        .map(|at| (at, (loads[&at], Supply { throttle, satisfied, grid })))
        .collect::<Vec<_>>()
    })
    .collect()
}

fn labour(voxels: &mut Voxels, at: IVec3, state: &mut State, load: Load, supply: Supply) {
  let Design { outlet, stores, .. } = design(state.block);
  let (burn, work) = (supply.throttle * TICK, supply.satisfied * TICK);
  let running = load.status == Status::Running;
  let room = outlet.unwrap_or_default();
  match state.block {
    Block::SteamEngine if running => {
      state.stock = (state.stock - burn).max(0.0);
      spill(&mut state.intake, STEAM_WATER * burn);
    }
    Block::BiofuelGenerator if running => {
      spill(&mut state.intake, BIOFUEL_BURN * burn);
    }
    Block::DieselGenerator if running => {
      spill(&mut state.intake, DIESEL_BURN * burn);
    }
    Block::GeothermalGenerator if running && source(voxels, at) != Some(Liquid::Lava) => {
      spill(&mut state.intake, LAVA_BURN * burn);
    }
    Block::ElectricPump if running => {
      if let Some(liquid) = source(voxels, at) {
        pour(&mut state.outlet, liquid, PUMPED * work, room);
      }
    }
    Block::Pumpjack if running => {
      pour(&mut state.outlet, Liquid::Crude, CRUDE_FLOW * work, room);
    }
    Block::Refinery if running => {
      let free = (room - amount(state.outlet)) / DIESEL_YIELD;
      let crude = spill(&mut state.intake, (REFINED * work).min(free));
      pour(&mut state.outlet, Liquid::Diesel, crude * DIESEL_YIELD, room);
    }
    Block::ElectricFurnace if running => {
      state.charge = (state.charge + design(state.block).draws * work).min(stores)
    }
    Block::Fermenter => {
      let made =
        pour(&mut state.outlet, Liquid::Biofuel, state.stock.min(FERMENTED * TICK), room);
      state.stock -= made
    }
    _ => ()
  }
}

fn plumb(states: &mut HashMap<IVec3, State>, attached: &HashMap<IVec3, IVec3>) {
  let pool: HashSet<IVec3> =
    states.iter().filter(|(_, state)| pooled(state.block)).map(|(&at, _)| at).collect();
  let capacity = |block: Block| bore(block).map_or(TANK, |(capacity, _)| capacity);
  for group in groups(&pool) {
    let liquid = Liquid::ALL
      .into_iter()
      .map(|liquid| {
        (liquid, group.iter().map(|at| holds(states[at].intake, liquid)).sum::<f32>())
      })
      .filter(|&(_, total)| total > 0.0)
      .max_by(|a, b| a.1.total_cmp(&b.1))
      .map(|(liquid, _)| liquid);
    let mut held: Held = liquid.map(|liquid| {
      (liquid, group.iter().map(|at| holds(states[at].intake, liquid)).sum::<f32>())
    });
    let room: f32 = group.iter().map(|at| capacity(states[at].block)).sum();
    let rate = group
      .iter()
      .filter_map(|at| bore(states[at].block).map(|(_, rate)| rate))
      .reduce(f32::min)
      .unwrap_or(TANK_RATE);
    let members: HashSet<IVec3> = group.iter().copied().collect();
    let touching = ordered(
      group
        .iter()
        .flat_map(|&at| SIDES.map(|side| at + side))
        .filter(|near| !members.contains(near))
        .filter(|near| {
          attached.get(near).is_none_or(|container| !members.contains(container))
        })
        .filter(|near| states.get(near).is_some_and(|state| plumbed(state.block)))
        .collect::<HashSet<IVec3>>()
    );
    let mut inflow = rate * TICK;
    for at in touching.iter() {
      if let Some(state) = states.get_mut(at)
        && let Some((liquid, offered)) = state.outlet
      {
        let moved = pour(&mut held, liquid, offered.min(inflow), room);
        spill(&mut state.outlet, moved);
        inflow -= moved
      }
    }
    let mut outflow = rate * TICK;
    for at in touching.iter() {
      if let Some(state) = states.get_mut(at)
        && let Some((taken, intake)) = design(state.block).intake
        && let Some((liquid, _)) = held
        && taken.contains(&liquid)
      {
        let moved = pour(&mut state.intake, liquid, amount(held).min(outflow), intake);
        spill(&mut held, moved);
        outflow -= moved
      }
    }
    let (pipes, tanks): (Vec<IVec3>, Vec<IVec3>) =
      group.iter().copied().partition(|at| states[at].block.pipe());
    let tanked = tanks.len() as f32 * TANK;
    let left = pipes.iter().fold(amount(held), |left, at| {
      let kept = left.min(capacity(states[at].block));
      if let Some(state) = states.get_mut(at) {
        state.intake = held.map(|(liquid, _)| (liquid, kept)).filter(|_| kept > 0.0)
      }
      left - kept
    });
    for at in tanks.iter() {
      if let Some(state) = states.get_mut(at) {
        let kept = left * TANK / tanked;
        state.intake = held.map(|(liquid, _)| (liquid, kept)).filter(|_| kept > 0.0)
      }
    }
  }
  for at in ordered(states.keys().copied()) {
    if let Some(&State { outlet: Some((liquid, offered)), .. }) = states.get(&at) {
      let budget = offered.min(DIRECT_RATE * TICK);
      let left = SIDES.into_iter().fold(budget, |left, side| {
        states
          .get_mut(&(at + side))
          .filter(|_| attached.get(&at) != Some(&(at + side)))
          .and_then(|near| {
            design(near.block)
              .intake
              .filter(|(taken, _)| taken.contains(&liquid))
              .map(|(_, room)| left - pour(&mut near.intake, liquid, left, room))
          })
          .unwrap_or(left)
      });
      if let Some(state) = states.get_mut(&at) {
        spill(&mut state.outlet, budget - left);
      }
    }
  }
}

fn shown(state: &State) -> Option<(Liquid, u32)> {
  let steps = |capacity: f32, steps: f32| {
    state.intake.map(|(liquid, kept)| {
      (liquid, ((kept / capacity * steps).ceil() / steps * capacity) as u32)
    })
  };
  match bore(state.block) {
    Some((capacity, _)) => steps(capacity, 8.0),
    None if state.block == Block::Tank => steps(TANK, 48.0),
    None => state.intake.map(|(liquid, kept)| (liquid, kept.round() as u32))
  }
}

fn unit(at: IVec3, state: &State, load: Load, supply: Supply, fixed: bool) -> Unit {
  let power = load.offer * supply.throttle - load.want * supply.satisfied;
  let starved = load.want > 0.0 && supply.satisfied < LIT_FROM;
  let idle_maker = load.offer > 0.0 && supply.throttle <= 0.0;
  let stocked = state.block == Block::Fermenter && state.stock > 0.0;
  Unit {
    at,
    block: state.block,
    intake: shown(state),
    outlet: state.outlet.map(|(liquid, kept)| (liquid, kept.round() as u32)),
    stock: state.stock.ceil() as u32,
    charge: state.charge as u32,
    power: power.round() as i32,
    grid: supply.grid,
    parcel: state.parcel,
    item: state.item,
    essence: state.essence,
    link: state.link,
    status: match () {
      () if state.block.arcane() => state.status,
      () if state.block.drain() && !fixed => Status::Detached,
      () if state.block.drain()
        && state.intake.is_none()
        && (state.block != Block::ItemDrain || state.charge <= 0.0) =>
      {
        Status::Drained
      }
      () if state.block.drain() => Status::Running,
      () if starved => Status::NoPower,
      () if idle_maker && supply.grid.wanted == 0 => Status::Unloaded,
      () if stocked => Status::Running,
      () => load.status
    }
  }
}

pub const TICK_SECONDS: f32 = TICK;

pub const PARCEL_SPEED: f32 = 2.5;

fn room_for(slots: &[Option<Stack>], stack: Stack) -> bool {
  let mut trial = slots.to_vec();
  crate::chest::stash(&mut trial, stack).is_none()
}

fn takes(
  states: &HashMap<IVec3, State>,
  coffers: &crate::chest::Coffers,
  block: Block,
  at: IVec3,
  stack: Stack
) -> bool {
  match block {
    Block::EmptyChest => coffers.0.get(&at).is_none_or(|slots| room_for(slots, stack)),
    Block::Chest => coffers.0.get(&at).is_some_and(|slots| room_for(slots, stack)),
    machine => {
      fuel(machine, stack.block).zip(states.get(&at)).is_some_and(|(each, state)| {
        state.stock + each * f32::from(stack.count) <= design(machine).hopper
      })
    }
  }
}

fn route(
  voxels: &Voxels,
  states: &HashMap<IVec3, State>,
  coffers: &crate::chest::Coffers,
  at: IVec3,
  from: usize,
  stack: Stack,
  turn: usize
) -> usize {
  let sides: Vec<(usize, Block)> = (0..SIDES.len())
    .map(|offset| (offset + turn) % SIDES.len())
    .filter(|&side| side != from)
    .map(|side| (side, voxels.block(at + SIDES[side]).unwrap_or_default()))
    .collect();
  sides
    .iter()
    .find(|&&(side, near)| takes(states, coffers, near, at + SIDES[side], stack))
    .or_else(|| {
      sides.iter().find(|&&(_, near)| near == Block::ItemPipe || near.conveyor())
    })
    .map_or(from, |&(side, _)| side)
}

pub fn deliver(
  states: &mut HashMap<IVec3, State>,
  coffers: &mut crate::chest::Coffers,
  voxels: &mut Voxels,
  changes: &mut MessageWriter<ToClients<Altered>>,
  target: IVec3,
  toward: usize,
  stack: Stack
) -> bool {
  let block = voxels.ensure(target);
  match block {
    Block::ItemPipe => {
      let turn = states.get(&target).map_or(0, |state| state.stock as usize);
      let to = route(voxels, states, coffers, target, toward, stack, turn);
      states
        .get_mut(&target)
        .filter(|state| state.parcel.is_none())
        .map(|state| {
          state.stock = (state.stock + 1.0) % 64.0;
          state.parcel =
            Some(Parcel { stack, from: toward as u8, to: to as u8, progress: 0.0 })
        })
        .is_some()
    }
    Block::Chest | Block::EmptyChest => {
      let accepted = takes(states, coffers, block, target, stack);
      if accepted {
        let slots =
          coffers.0.entry(target).or_insert_with(|| vec![None; crate::chest::ROOM]);
        crate::chest::stash(slots, stack);
        if block == Block::EmptyChest {
          voxels.set(target, Block::Chest);
          changes.write(ToClients {
            targets: SendTargets::All,
            message: Altered { at: target, block: Block::Chest }
          });
        }
      }
      accepted
    }
    machine => {
      let accepted = takes(states, coffers, machine, target, stack);
      if accepted
        && let Some(each) = fuel(machine, stack.block)
        && let Some(state) = states.get_mut(&target)
      {
        state.stock += each * f32::from(stack.count)
      }
      accepted
    }
  }
}

fn onto_belt(
  voxels: &Voxels,
  commands: &mut Commands,
  from: IVec3,
  belt: IVec3,
  stack: Stack
) {
  let edge = (from.as_vec3() + belt.as_vec3()) / 2.0 + Vec3::splat(0.5);
  let point = match belt.y == from.y {
    true => edge.lerp(belt.as_vec3() + Vec3::splat(0.5), 0.3),
    false => belt.as_vec3() + Vec3::splat(0.5)
  };
  let top = crate::conveyor::surface_at(voxels, belt, point).unwrap_or(point.y);
  crate::loose::fling(commands, stack, point.with_y(top + 0.02), Vec3::ZERO, 0.5)
}

fn ferry(
  states: &mut HashMap<IVec3, State>,
  coffers: &mut crate::chest::Coffers,
  voxels: &mut Voxels,
  changes: &mut MessageWriter<ToClients<Altered>>,
  commands: &mut Commands
) {
  let pipes: Vec<IVec3> =
    ordered(states.iter().filter(|(_, state)| state.parcel.is_some()).map(|(&at, _)| at));
  for at in pipes {
    if let Some(Parcel { stack, from, to, progress }) = states[&at].parcel {
      let progress = (progress + PARCEL_SPEED * TICK).min(1.0);
      let exit = usize::from(to);
      let target = at + SIDES[exit];
      let near = voxels.block(target).unwrap_or_default();
      let arrived = progress >= 1.0;
      let handed = arrived
        && match near.conveyor() {
          true => {
            onto_belt(voxels, commands, at, target, stack);
            true
          }
          false => deliver(
            states,
            coffers,
            voxels,
            changes,
            target,
            side_of(-SIDES[exit]),
            stack
          )
        };
      let waiting = arrived && !handed && near == Block::ItemPipe;
      let turn = states[&at].stock as usize;
      let parcel = match (handed, arrived && !waiting) {
        (true, _) => None,
        (false, true) => Some(Parcel {
          stack,
          from: to,
          to: route(voxels, states, coffers, at, exit, stack, turn + 1) as u8,
          progress: 0.0
        }),
        (false, false) => Some(Parcel { stack, from, to, progress })
      };
      if let Some(state) = states.get_mut(&at) {
        state.parcel = parcel;
        state.stock = (state.stock + 1.0) % 64.0
      }
    }
  }
}

fn drain(states: &mut HashMap<IVec3, State>, attached: &HashMap<IVec3, IVec3>) {
  for at in ordered(attached.keys().copied()) {
    let container = attached[&at];
    let State { block, outlet: own, .. } = states[&at];
    if block != Block::ItemDrain {
      let wanted = match block {
        Block::PipedDrain => (PIPED_DRAIN - amount(own)).min(DRAINED * TICK),
        _ => DRAINED * TICK
      };
      let taken = states.get_mut(&container).and_then(|kept| {
        let held = match kept.outlet.is_some() {
          true => &mut kept.outlet,
          false => &mut kept.intake
        };
        held
          .filter(|&(liquid, _)| own.is_none_or(|(mine, _)| mine == liquid))
          .map(|(liquid, _)| (liquid, spill(held, wanted)))
      });
      let state = states.get_mut(&at).expect("drains have states");
      state.intake = taken.filter(|&(_, moved)| moved > 0.0);
      if let Some((liquid, moved)) = state.intake
        && block == Block::PipedDrain
      {
        pour(&mut state.outlet, liquid, moved, PIPED_DRAIN);
      }
    }
  }
}

fn empty(
  states: &mut HashMap<IVec3, State>,
  attached: &HashMap<IVec3, IVec3>,
  coffers: &mut crate::chest::Coffers,
  voxels: &mut Voxels,
  changes: &mut MessageWriter<ToClients<Altered>>,
  commands: &mut Commands
) {
  for at in ordered(attached.keys().copied()) {
    let container = attached[&at];
    if let Some(state) = states.get_mut(&at)
      && state.block == Block::ItemDrain
    {
      state.stock = (state.stock + TICK).min(ITEM_EVERY);
      state.charge = 0.0;
      let due = state.stock >= ITEM_EVERY;
      let pipes: Vec<usize> = (0..SIDES.len())
        .filter(|&side| at + SIDES[side] != container)
        .filter(|&side| voxels.block(at + SIDES[side]) == Some(Block::ItemPipe))
        .collect();
      let free = pipes.iter().copied().find(|&side| {
        states.get(&(at + SIDES[side])).is_some_and(|pipe| pipe.parcel.is_none())
      });
      let outlet = match (pipes.is_empty(), free) {
        (true, _) => Some(None),
        (false, Some(side)) => Some(Some(side)),
        (false, None) => None
      };
      if due
        && let Some(outlet) = outlet
        && let Some(slots) = coffers.0.get_mut(&container)
        && let Some(slot) = slots.iter_mut().find(|slot| slot.is_some())
        && let Some(stack) = *slot
      {
        *slot = (stack.count > 1).then_some(Stack { count: stack.count - 1, ..stack });
        let emptied = slots.iter().all(Option::is_none);
        let one = Stack { count: 1, ..stack };
        if let Some(state) = states.get_mut(&at) {
          state.stock = 0.0;
          state.charge = 1.0
        }
        let away = (at - container).as_vec3();
        let middle = at.as_vec3() + Vec3::splat(0.5);
        match (outlet, voxels.block(at - IVec3::Y).is_some_and(Block::conveyor)) {
          (Some(side), _) => {
            deliver(
              states,
              coffers,
              voxels,
              changes,
              at + SIDES[side],
              side_of(-SIDES[side]),
              one
            );
          }
          (None, true) => {
            crate::loose::fling(commands, one, middle - Vec3::Y * 0.9, Vec3::ZERO, 0.5)
          }
          (None, false) => crate::loose::fling(
            commands,
            one,
            middle + away * 0.7,
            away * 1.5 + Vec3::Y,
            0.5
          )
        }
        if emptied && voxels.ensure(container) == Block::Chest {
          voxels.set(container, Block::EmptyChest);
          changes.write(ToClients {
            targets: SendTargets::All,
            message: Altered { at: container, block: Block::EmptyChest }
          });
        }
      }
    }
  }
}

fn step(
  voxels: &mut Voxels,
  sky: Sky,
  states: &mut HashMap<IVec3, State>,
  attached: &HashMap<IVec3, IVec3>,
  (coffers, changes, commands): (
    &mut crate::chest::Coffers,
    &mut MessageWriter<ToClients<Altered>>,
    &mut Commands
  )
) -> HashMap<IVec3, (Load, Supply)> {
  let supplies = power(voxels, sky, states);
  for at in ordered(states.keys().copied()) {
    let (load, supply) = supplies.get(&at).copied().unwrap_or_default();
    if let Some(state) = states.get_mut(&at) {
      labour(voxels, at, state, load, supply);
      let lamp = match supply.satisfied >= LIT_FROM && load.want > 0.0 {
        true => Block::ElectricLampLit,
        false => Block::ElectricLamp
      };
      if state.block.held() == Block::ElectricLamp && state.block != lamp {
        state.block = lamp;
        voxels.set(at, lamp);
        changes.write(ToClients {
          targets: SendTargets::All,
          message: Altered { at, block: lamp }
        });
      }
    }
  }
  drain(states, attached);
  empty(states, attached, coffers, voxels, changes, commands);
  ferry(states, coffers, voxels, changes, commands);
  plumb(states, attached);
  supplies
}

const MOST_STEPS: u32 = 8;

fn run(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut works: ResMut<Works>,
  weather: Query<&Weather>,
  mut units: Query<&mut Unit>,
  mut changes: MessageWriter<ToClients<Altered>>,
  mut coffers: ResMut<crate::chest::Coffers>,
  mut commands: Commands,
  mut since: Local<f32>
) {
  *since = (*since + time.delta_secs()).min(TICK * MOST_STEPS as f32);
  let steps = (*since / TICK) as u32;
  if steps > 0 {
    *since -= steps as f32 * TICK;
    let sky = weather.iter().next().map_or(Sky::Clear, |weather| weather.sky);
    let works = &mut *works;
    let gone: Vec<IVec3> = ordered(works.states.keys().copied())
      .into_iter()
      .filter(|&at| voxels.ensure(at).held() != works.states[&at].block.held())
      .collect();
    for at in gone {
      let carried = works.states.get(&at).and_then(|state| {
        state.parcel.map(|parcel| parcel.stack).into_iter().chain(state.item).next()
      });
      if let Some(stack) = carried {
        crate::loose::fling(
          &mut commands,
          stack,
          at.as_vec3() + Vec3::splat(0.5),
          Vec3::Y,
          0.5
        )
      }
      works.states.remove(&at);
      if let Some(entity) = works.units.remove(&at) {
        commands.entity(entity).despawn()
      }
    }
    let attached: HashMap<IVec3, IVec3> = ordered(works.states.keys().copied())
      .into_iter()
      .filter_map(|at| {
        let block = works.states[&at].block;
        attached(block, |side| voxels.block(at + side).unwrap_or_default())
          .map(|side| (at, at + SIDES[side]))
      })
      .collect();
    let supplies = (0..steps).fold(HashMap::new(), |_, _| {
      step(
        &mut voxels,
        sky,
        &mut works.states,
        &attached,
        (&mut coffers, &mut changes, &mut commands)
      )
    });
    for (&at, state) in works.states.iter() {
      let (load, supply) = supplies.get(&at).copied().unwrap_or_default();
      let shown = unit(at, state, load, supply, attached.contains_key(&at));
      match works.units.get(&at).and_then(|&entity| units.get_mut(entity).ok()) {
        Some(mut unit) => {
          unit.set_if_neq(shown);
        }
        None => {
          let entity = commands.spawn((Replicated, shown)).id();
          works.units.insert(at, entity);
        }
      }
    }
  }
}

fn install(
  mut puts: MessageReader<FromClient<Put>>,
  mut voxels: ResMut<Voxels>,
  mut works: ResMut<Works>
) {
  for &FromClient { message: Put { at, block }, .. } in puts.read() {
    if voxels.ensure(at) == block {
      works.install(at, block)
    }
  }
}

fn tend(
  mut tends: MessageReader<FromClient<Tend>>,
  mut voxels: ResMut<Voxels>,
  mut works: ResMut<Works>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  mut changes: MessageWriter<ToClients<Altered>>
) {
  for &FromClient { client_id, message: Tend { at, slot } } in tends.read() {
    let block = voxels.ensure(at);
    if let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
      && let Some(stack) = inventory.slots.get(usize::from(slot)).copied().flatten()
      && let Some(state) = works.states.get_mut(&at)
      && state.block == block
    {
      let item = stack.block;
      let Design { intake, outlet, hopper, .. } = design(block);
      let tank = block == Block::Tank;
      match (fuel(block, item), item.carrying().map(Liquid::of)) {
        _ if block == Block::FluidDrain && item == Block::CopperPipe => {
          if inventory.take(item) {
            state.block = Block::PipedDrain;
            voxels.set(at, Block::PipedDrain);
            changes.write(ToClients {
              targets: SendTargets::All,
              message: Altered { at, block: Block::PipedDrain }
            });
          }
        }
        (Some(each), _) => {
          let fits = ((hopper - state.stock) / each).floor().max(0.0) as u16;
          let taken =
            (0..fits.min(stack.count)).take_while(|_| inventory.take(item)).count();
          state.stock += each * taken as f32
        }
        (_, Some(liquid)) => {
          let room = match (tank, intake) {
            (true, _) => TANK,
            (_, Some((taken, room))) if taken.contains(&liquid) => room,
            _ => 0.0
          };
          if room - amount(state.intake) >= BUCKET
            && state.intake.is_none_or(|(kept, _)| kept == liquid)
            && inventory.take(item)
          {
            pour(&mut state.intake, liquid, BUCKET, room);
            inventory.add(Block::Bucket);
          }
        }
        _ if item == Block::Bucket => {
          let held = match tank {
            true => &mut state.intake,
            false => &mut state.outlet
          };
          if let Some((liquid, kept)) = *held
            && kept >= BUCKET
            && let Some(filled) = liquid.bucket()
            && outlet.is_some() | tank
            && inventory.clone().add(filled)
            && inventory.take(Block::Bucket)
          {
            spill(held, BUCKET);
            inventory.add(filled);
          }
        }
        _ => ()
      }
    }
  }
}

pub fn settle(voxels: &mut Voxels, saved: Vec<(IVec3, State)>) -> Works {
  let edits: Vec<(IVec3, Block)> =
    voxels.all_edits().into_iter().filter(|(_, block)| block.machine()).collect();
  let mut states: HashMap<IVec3, State> = saved
    .into_iter()
    .filter(|&(at, state)| voxels.ensure(at).held() == state.block.held())
    .collect();
  for (at, block) in edits {
    states.entry(at).or_insert(State { block, ..default() });
  }
  Works { states, units: HashMap::new() }
}

#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Running;

pub struct Machines;

impl Plugin for Machines {
  fn build(&self, app: &mut App) {
    app
      .replicate::<Unit>()
      .add_client_message::<Tend>(Channel::Ordered)
      .init_resource::<Works>()
      .add_systems(
        Update,
        (install, tend, run)
          .chain()
          .in_set(Running)
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
      );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn world() -> Voxels {
    let mut voxels = Voxels::new(1);
    let floor = IVec3::new(4000, 200, 4000);
    for x in -4..=8 {
      for z in -4..=4 {
        voxels.ensure(floor + IVec3::new(x, 0, z));
      }
    }
    voxels
  }

  fn placed(
    voxels: &mut Voxels,
    states: &mut HashMap<IVec3, State>,
    at: IVec3,
    block: Block
  ) {
    voxels.set(at, block);
    states.insert(at, State { block, ..default() });
  }

  #[test]
  fn diesel_flows_through_pipes_into_a_generator_that_charges_a_battery() {
    let mut voxels = world();
    let mut states = HashMap::new();
    let base = IVec3::new(4000, 201, 4000);
    placed(&mut voxels, &mut states, base, Block::Tank);
    for x in 1..=4 {
      placed(&mut voxels, &mut states, base + IVec3::X * x, Block::CopperPipe)
    }
    placed(&mut voxels, &mut states, base + IVec3::X * 5, Block::DieselGenerator);
    placed(&mut voxels, &mut states, base + IVec3::X * 6, Block::CopperCable);
    placed(&mut voxels, &mut states, base + IVec3::X * 7, Block::Battery);
    states.get_mut(&base).unwrap().intake = Some((Liquid::Diesel, 8.0 * BUCKET));
    for _ in 0..200 {
      let supplies = power(&mut voxels, Sky::Clear, &mut states);
      for at in ordered(states.keys().copied()) {
        let (load, supply) = supplies.get(&at).copied().unwrap_or_default();
        labour(&mut voxels, at, states.get_mut(&at).unwrap(), load, supply)
      }
      plumb(&mut states, &HashMap::new())
    }
    let battery = states[&(base + IVec3::X * 7)].charge;
    let left = amount(states[&base].intake);
    assert!(battery > 4000.0, "battery holds {battery}");
    assert!(left < 8.0 * BUCKET, "tank still holds {left}");
    assert!(
      states
        .values()
        .all(|state| state.intake.is_none_or(|(liquid, _)| liquid == Liquid::Diesel))
    )
  }

  #[test]
  fn a_piped_drain_moves_a_tank_into_another_but_not_back() {
    let mut voxels = world();
    let mut states = HashMap::new();
    let base = IVec3::new(4000, 201, 4000);
    placed(&mut voxels, &mut states, base, Block::Tank);
    placed(&mut voxels, &mut states, base - IVec3::Y, Block::PipedDrain);
    placed(&mut voxels, &mut states, base - IVec3::Y + IVec3::X, Block::CopperPipe);
    placed(&mut voxels, &mut states, base - IVec3::Y + IVec3::X * 2, Block::Tank);
    states.get_mut(&base).unwrap().intake = Some((Liquid::Water, 4.0 * BUCKET));
    let attached: HashMap<IVec3, IVec3> = [(base - IVec3::Y, base)].into_iter().collect();
    for _ in 0..80 {
      drain(&mut states, &attached);
      plumb(&mut states, &attached)
    }
    let (from, to) = (
      amount(states[&base].intake),
      amount(states[&(base - IVec3::Y + IVec3::X * 2)].intake)
    );
    assert!(to > 500.0, "moved {to}");
    assert!(from < 4.0 * BUCKET - 500.0, "left {from}")
  }

  #[test]
  fn a_pumpjack_needs_its_seep_and_power() {
    let mut voxels = world();
    let mut states = HashMap::new();
    let base = IVec3::new(4000, 201, 4000);
    placed(&mut voxels, &mut states, base, Block::Pumpjack);
    let lonely = load(&mut voxels, Sky::Clear, base, &states[&base]);
    assert_eq!(lonely.status, Status::NoSeep);
    voxels.set(base - IVec3::Y, Block::OilSeep);
    placed(&mut voxels, &mut states, base + IVec3::X, Block::WindTurbine);
    for _ in 0..40 {
      let supplies = power(&mut voxels, Sky::Storm, &mut states);
      for at in ordered(states.keys().copied()) {
        let (load, supply) = supplies.get(&at).copied().unwrap_or_default();
        labour(&mut voxels, at, states.get_mut(&at).unwrap(), load, supply)
      }
    }
    assert!(holds(states[&base].outlet, Liquid::Crude) > 10.0)
  }
}
