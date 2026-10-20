use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Aspect {
  Earth,
  Water,
  Fire,
  Air,
  Order,
  Decay,
  Growth,
  Metal,
  Magic
}

pub const ASPECTS: usize = 9;

impl Aspect {
  pub const ALL: [Aspect; ASPECTS] = [
    Aspect::Earth,
    Aspect::Water,
    Aspect::Fire,
    Aspect::Air,
    Aspect::Order,
    Aspect::Decay,
    Aspect::Growth,
    Aspect::Metal,
    Aspect::Magic
  ];

  pub fn name(self) -> &'static str {
    match self {
      Aspect::Earth => "Earth",
      Aspect::Water => "Water",
      Aspect::Fire => "Fire",
      Aspect::Air => "Air",
      Aspect::Order => "Order",
      Aspect::Decay => "Decay",
      Aspect::Growth => "Growth",
      Aspect::Metal => "Metal",
      Aspect::Magic => "Magic"
    }
  }
}

use {crate::{authority::{Controller, player_of, within_reach},
             balloon::burns,
             bird::Bird,
             block::Block,
             conveyor::Riding,
             death::Dying,
             gear::{Gear, Perk, Virtue},
             machine::{Liquid, State, Status, TICK_SECONDS, Tend, Works, holds,
                       ordered, pour, source, spill, sunlight},
             opts::opts,
             protocol::{Avatar, Beast, Folk, Health, Inventory, Loose, Stack,
                        authority},
             recipe::{Cost, RECIPES, Recipe},
             voxels::Voxels,
             weather::{Sky, Weather}},
     bevy::{platform::collections::HashMap, prelude::*},
     bevy_replicon::prelude::*};

pub const POOL: f32 = 10_000.0;
pub const ALTAR_MOST: f32 = 40_000.0;
const ALTAR_BASE: f32 = 10_000.0;
const RUNE_ROOM: f32 = 2_000.0;
const BIND: f32 = 6.0;
const DRAWN_FROM: f32 = 8.0;
const HELD_MANA: f32 = 1000.0;
const SUNSIP: f32 = 6.0;
const DEWCUP: f32 = 8.0;
const EMBERBELL: f32 = 20.0;
const EATS_WITHIN: f32 = 1.6;
const ALTAR_RATE: f32 = 400.0;
const INFUSE_RATE: f32 = 800.0;
const SACRIFICE: u8 = 2;
const LIFEBLOOD: f32 = 250.0;
const SLAIN_BLOOD: f32 = 400.0;
const SLAIN_REACH: f32 = 10.0;
const ESSENCE_MOST: u16 = 99;
const BOIL_OFF: f32 = 50.0;
const STEEPED: f32 = 100.0;
const PLINTH_REACH: i32 = 3;

pub fn aspects(block: Block) -> &'static [(Aspect, u16)] {
  use Aspect::*;
  match block {
    Block::Stone | Block::Granite | Block::Diorite | Block::Andesite | Block::Slate => {
      &[(Earth, 2)]
    }
    Block::Limestone => &[(Earth, 1), (Order, 1)],
    cobble if Block::ROCKS.iter().any(|&(_, rubble)| rubble == cobble) => {
      &[(Earth, 1), (Decay, 1)]
    }
    Block::Dirt | Block::Grass => &[(Earth, 1), (Growth, 1)],
    Block::Sand | Block::Gravel => &[(Earth, 1), (Decay, 1)],
    Block::Clay => &[(Earth, 1), (Water, 1)],
    Block::Snow => &[(Water, 1)],
    Block::Ice => &[(Water, 2), (Order, 1)],
    Block::Basalt => &[(Earth, 1), (Fire, 1)],
    Block::Log | Block::BirchLog | Block::SpruceLog | Block::PalmLog => &[(Growth, 3)],
    Block::Planks => &[(Growth, 1)],
    leaves if leaves.leafy() => &[(Growth, 1), (Air, 1)],
    Block::TallGrass => &[(Growth, 1)],
    Block::Poppy | Block::Dandelion | Block::Cornflower | Block::Daisy => {
      &[(Growth, 1), (Air, 1)]
    }
    glimmer if glimmer.glimmer() => &[(Growth, 1), (Magic, 2)],
    Block::OakSapling
    | Block::BirchSapling
    | Block::SpruceSapling
    | Block::PalmSapling => &[(Growth, 2), (Earth, 1)],
    Block::RedMushroom | Block::BrownMushroom => &[(Growth, 1), (Decay, 1)],
    Block::CoalOre => &[(Fire, 3), (Earth, 1)],
    Block::Charcoal => &[(Fire, 3)],
    Block::IronOre | Block::CopperOre | Block::TinOre => &[(Metal, 3), (Earth, 1)],
    Block::GoldOre => &[(Metal, 3), (Order, 2)],
    Block::DiamondOre => &[(Order, 6), (Magic, 2)],
    Block::Glass => &[(Order, 1), (Water, 1)],
    Block::Feather => &[(Air, 3)],
    Block::Wool => &[(Air, 1), (Growth, 1)],
    fish if fish.fish() => &[(Water, 3), (Growth, 1)],
    Block::Lumen => &[(Magic, 4), (Order, 2)],
    Block::ArcaneDust => &[(Magic, 3)],
    Block::BoneDust => &[(Decay, 3)],
    Block::BonePile => &[(Decay, 4), (Earth, 1)],
    Block::LizardClaw => &[(Air, 2), (Decay, 2)],
    Block::Candle => &[(Fire, 2), (Order, 1)],
    Block::Cobweb => &[(Air, 1), (Decay, 1)],
    Block::Tidepearl => &[(Magic, 6), (Water, 2)],
    _ => &[]
  }
}

pub struct Charm {
  pub virtue: Virtue,
  pub by: u8,
  pub pedestals: &'static [Block],
  pub mana: u32
}

pub const CHARMS: &[Charm] = &[
  Charm {
    virtue: Virtue::Springy,
    by: 15,
    pedestals: &[Block::Feather, Block::Feather, Block::Tidepearl],
    mana: 3000
  },
  Charm {
    virtue: Virtue::Swift,
    by: 12,
    pedestals: &[Block::Feather, Block::LizardClaw, Block::Tidepearl],
    mana: 3000
  },
  Charm {
    virtue: Virtue::Swimmer,
    by: 20,
    pedestals: &[Block::Cod, Block::Salmon, Block::Tidepearl],
    mana: 3000
  },
  Charm {
    virtue: Virtue::Fireproof,
    by: 40,
    pedestals: &[Block::Basalt, Block::Basalt, Block::Tidepearl],
    mana: 3000
  },
  Charm {
    virtue: Virtue::Delving,
    by: 25,
    pedestals: &[Block::Hexbrass, Block::Hexbrass, Block::Tidepearl],
    mana: 4000
  },
  Charm {
    virtue: Virtue::Fortune,
    by: 12,
    pedestals: &[Block::GoldOre, Block::GoldOre, Block::Starheart],
    mana: 5000
  },
  Charm {
    virtue: Virtue::Warding,
    by: 10,
    pedestals: &[Block::BoundTablet, Block::BoundTablet, Block::Starheart],
    mana: 5000
  },
  Charm {
    virtue: Virtue::Might,
    by: 20,
    pedestals: &[Block::BoneDust, Block::BoneDust, Block::Heartstone],
    mana: 5000
  },
  Charm {
    virtue: Virtue::Mending,
    by: 25,
    pedestals: &[Block::Candle, Block::Candle, Block::Heartstone],
    mana: 5000
  },
  Charm {
    virtue: Virtue::Quickcast,
    by: 15,
    pedestals: &[Block::ArcaneDust, Block::ArcaneDust, Block::Lumen, Block::Tidepearl],
    mana: 4000
  }
];

pub fn rites(station: Block) -> impl Iterator<Item = &'static Recipe> {
  RECIPES.iter().filter(move |recipe| recipe.station == Some(station))
}

fn rite_for(station: Block, item: Block) -> Option<&'static Recipe> {
  rites(station)
    .find(|recipe| recipe.inputs.first().is_some_and(|&(input, _)| input == item))
}

fn centre(at: IVec3) -> Vec3 { at.as_vec3() + Vec3::splat(0.5) }

fn hot(block: Block) -> bool {
  block.liquid().is_some_and(|(fluid, _)| fluid == crate::block::Fluid::Lava)
    || block.lit()
    || matches!(block, Block::Furnace | Block::GeothermalGenerator | Block::Emberbell)
}

fn runes(voxels: &mut Voxels, at: IVec3) -> u32 {
  (-2..=2)
    .flat_map(|x| (-1..=0).flat_map(move |y| (-2..=2).map(move |z| IVec3::new(x, y, z))))
    .filter(|&offset| voxels.ensure(at + offset) == Block::CrimsonRune)
    .count() as u32
}

pub fn altar_room(runes: u32) -> f32 { ALTAR_BASE + runes as f32 * RUNE_ROOM }

fn pools_near(states: &HashMap<IVec3, State>, at: IVec3, reach: f32) -> Vec<IVec3> {
  let mut pools: Vec<IVec3> = ordered(
    states
      .iter()
      .filter(|(pool, state)| {
        state.block == Block::ManaCistern && centre(**pool).distance(centre(at)) <= reach
      })
      .map(|(&pool, _)| pool)
  );
  pools.sort_by(|a, b| {
    centre(*a).distance(centre(at)).total_cmp(&centre(*b).distance(centre(at)))
  });
  pools
}

fn mana_in(states: &HashMap<IVec3, State>, pools: &[IVec3]) -> f32 {
  pools.iter().map(|pool| holds(states[pool].intake, Liquid::Mana)).sum()
}

fn draw_mana(states: &mut HashMap<IVec3, State>, pools: &[IVec3], wanted: f32) -> f32 {
  pools.iter().fold(0.0, |taken, pool| {
    let state = states.get_mut(pool).expect("pools have states");
    taken + spill(&mut state.intake, wanted - taken)
  })
}

fn bloom(voxels: &mut Voxels, sky: Sky, at: IVec3, state: &mut State) -> Status {
  let made = match state.block {
    Block::Sunsip => {
      let open = (1..=24).all(|lift| !voxels.ensure(at + IVec3::Y * lift).solid());
      SUNSIP * sunlight(opts().hour) * sky.sun() * f32::from(u8::from(open))
    }
    Block::Dewcup => match source(voxels, at) == Some(Liquid::Water) {
      true => DEWCUP,
      false => 0.0
    },
    _ => match state.stock > 0.0 {
      true => {
        state.stock = (state.stock - TICK_SECONDS).max(0.0);
        EMBERBELL
      }
      false => 0.0
    }
  };
  state.charge = (state.charge + made * TICK_SECONDS).min(HELD_MANA);
  match (made > 0.0, state.block) {
    (true, _) => Status::Running,
    (false, Block::Sunsip) => Status::Dark,
    (false, Block::Dewcup) => Status::NoSource,
    _ => Status::NoFuel
  }
}

fn bind(states: &mut HashMap<IVec3, State>, flowers: &[IVec3]) {
  for &at in flowers {
    let pool = pools_near(states, at, BIND).into_iter().find(|pool| {
      holds(states[pool].intake, Liquid::Mana) < POOL
        && states[pool].intake.is_none_or(|(liquid, _)| liquid == Liquid::Mana)
    });
    let any = !pools_near(states, at, BIND).is_empty();
    let held = states[&at].charge;
    let given = pool.map_or(0.0, |pool| {
      let state = states.get_mut(&pool).expect("pools have states");
      pour(&mut state.intake, Liquid::Mana, held, POOL)
    });
    let state = states.get_mut(&at).expect("flowers have states");
    state.charge -= given;
    state.link = pool;
    state.status = match (any, pool.is_some()) {
      (false, _) => Status::Unbound,
      (true, false) if state.status == Status::Running => Status::Full,
      _ => state.status
    }
  }
}

fn infused(
  states: &HashMap<IVec3, State>,
  at: IVec3
) -> Option<(Vec<IVec3>, Stack, u32)> {
  let catalyst = states[&at].item?;
  let pedestals: Vec<(IVec3, Block)> = ordered(states.keys().copied())
    .into_iter()
    .filter(|pedestal| {
      let offset = *pedestal - at;
      states[pedestal].block == Block::Plinth
        && offset.x.abs() <= PLINTH_REACH
        && offset.z.abs() <= PLINTH_REACH
        && offset.y.abs() <= 1
    })
    .filter_map(|pedestal| states[&pedestal].item.map(|stack| (pedestal, stack.block)))
    .collect();
  let matched = |wanted: &[Block]| {
    let mut left = pedestals.clone();
    let used: Option<Vec<IVec3>> = wanted
      .iter()
      .map(|&block| {
        left
          .iter()
          .position(|&(_, offered)| offered == block)
          .map(|index| left.remove(index).0)
      })
      .collect();
    used.filter(|_| left.is_empty())
  };
  let recipe = rites(Block::ConfluenceAltar)
    .filter(|recipe| {
      recipe.inputs.first().is_some_and(|&(input, _)| input == catalyst.block)
    })
    .find_map(|recipe| {
      let wanted: Vec<Block> = recipe.inputs[1..]
        .iter()
        .flat_map(|&(block, count)| std::iter::repeat_n(block, usize::from(count)))
        .collect();
      let (output, count) = recipe.output;
      let mana = match recipe.cost {
        Cost::Mana(mana) => mana,
        _ => 0
      };
      matched(&wanted).map(|used| (used, Stack::of(output, count), mana))
    });
  let gear = catalyst.gear.unwrap_or_default();
  let charm = (catalyst.block.forged() || catalyst.block.worn())
    .then(|| {
      CHARMS.iter().find_map(|charm| {
        let free = gear.perks.iter().any(Option::is_none);
        let fresh = gear.perks().all(|perk| perk.virtue != charm.virtue);
        matched(charm.pedestals).filter(|_| free && fresh).map(|used| {
          let mut perks = gear.perks;
          if let Some(slot) = perks.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(Perk { virtue: charm.virtue, by: charm.by })
          }
          let gear = Gear { perks, ..gear };
          (used, Stack { gear: Some(gear), ..catalyst }, charm.mana)
        })
      })
    })
    .flatten();
  recipe.or(charm)
}

fn kindle(states: &mut HashMap<IVec3, State>, altars: &[IVec3]) {
  for &at in altars {
    let found = infused(states, at);
    let pools = pools_near(states, at, DRAWN_FROM);
    let status = match (states[&at].item, &found) {
      (None, _) => Status::Empty,
      _ if states[&at].done => Status::Finished,
      (Some(_), None) => Status::Unknown,
      (Some(_), Some((used, made, cost))) => {
        let progress = states[&at].charge;
        let wanted = (INFUSE_RATE * TICK_SECONDS).min(*cost as f32 - progress);
        let drawn = match mana_in(states, &pools) >= wanted {
          true => draw_mana(states, &pools, wanted),
          false => 0.0
        };
        let done = progress + drawn >= *cost as f32;
        for pedestal in used.iter().filter(|_| done) {
          if let Some(state) = states.get_mut(pedestal) {
            state.item = None
          }
        }
        let state = states.get_mut(&at).expect("altars have states");
        state.charge = match done {
          true => 0.0,
          false => progress + drawn
        };
        if done {
          state.item = Some(*made);
          state.done = true
        }
        match drawn > 0.0 || done {
          true => Status::Running,
          false => Status::NoMana
        }
      }
    };
    let state = states.get_mut(&at).expect("altars have states");
    if !matches!(status, Status::Running | Status::NoMana | Status::Finished) {
      state.charge = 0.0
    }
    state.status = status
  }
}

fn bleed(voxels: &mut Voxels, states: &mut HashMap<IVec3, State>, altars: &[IVec3]) {
  for &at in altars {
    let runes = runes(voxels, at);
    let state = states.get_mut(&at).expect("altars have states");
    let room = altar_room(runes);
    if let Some((liquid, kept)) = state.intake
      && kept > room
    {
      state.intake = Some((liquid, room))
    }
    state.stock = runes as f32;
    let recipe = state.item.and_then(|stack| rite_for(Block::CrimsonAltar, stack.block));
    state.status = match (state.item, recipe) {
      (None, _) => Status::Empty,
      _ if state.done => Status::Finished,
      (Some(_), None) => Status::Unknown,
      (Some(_), Some(recipe)) => {
        let cost = match recipe.cost {
          Cost::Lifeblood(blood) => blood as f32,
          _ => 0.0
        };
        let rate = ALTAR_RATE * (1.0 + runes as f32 * 0.2) * TICK_SECONDS;
        let taken = spill(&mut state.intake, rate.min(cost - state.charge));
        state.charge += taken;
        if state.charge >= cost - 0.01 {
          let (output, count) = recipe.output;
          state.item = Some(Stack::of(output, count));
          state.charge = 0.0;
          state.done = true
        }
        match taken > 0.0 {
          true => Status::Running,
          false => Status::NoBlood
        }
      }
    };
  }
}

fn resting_in(loose: &Loose, at: IVec3) -> bool {
  let offset = loose.at - at.as_vec3();
  (0.0..1.0).contains(&offset.x)
    && (0.0..1.0).contains(&offset.z)
    && (-0.05..0.9).contains(&offset.y)
}

fn thrown_out(commands: &mut Commands, at: IVec3, stack: Stack, salt: i32) {
  let angle = (salt as f32 * 2.399).rem_euclid(std::f32::consts::TAU);
  crate::loose::fling(
    commands,
    stack,
    centre(at) + Vec3::Y * 0.6,
    Vec3::new(angle.cos() * 2.2, 4.0, angle.sin() * 2.2),
    0.8
  )
}

type Lying<'a> = (Entity, Mut<'a, Loose>);

fn steep(
  voxels: &mut Voxels,
  states: &mut HashMap<IVec3, State>,
  at: IVec3,
  lying: &mut [Lying],
  commands: &mut Commands
) {
  let heated = hot(voxels.ensure(at - IVec3::Y));
  let water = holds(states[&at].intake, Liquid::Water);
  let state = states.get_mut(&at).expect("cauldrons have states");
  state.status = match (heated, water >= STEEPED) {
    (false, _) => Status::NoHeat,
    (_, false) => Status::NoWater,
    _ => Status::Running
  };
  if state.status == Status::Running
    && let Some((entity, loose)) =
      lying.iter_mut().find(|(_, loose)| loose.count > 0 && resting_in(loose, at))
  {
    let block = loose.block;
    let one = Stack { count: 1, block, gear: loose.gear };
    let brewed = rite_for(Block::Cauldron, block).filter(|recipe| match recipe.cost {
      Cost::Essence(needed) => {
        needed.iter().all(|&(aspect, count)| state.essence[aspect as usize] >= count)
      }
      _ => false
    });
    match (brewed, aspects(block)) {
      (Some(recipe), _) => {
        if let Cost::Essence(needed) = recipe.cost {
          for &(aspect, count) in needed {
            state.essence[aspect as usize] -= count
          }
        }
        let (output, count) = recipe.output;
        thrown_out(commands, at, Stack::of(output, count), entity.index_u32() as i32)
      }
      (None, []) => thrown_out(commands, at, one, entity.index_u32() as i32),
      (None, dissolved) => {
        for &(aspect, count) in dissolved {
          let held = &mut state.essence[aspect as usize];
          *held = (*held + count).min(ESSENCE_MOST)
        }
        spill(&mut state.intake, BOIL_OFF);
      }
    }
    loose.count -= 1;
    if loose.count == 0 {
      commands.entity(*entity).despawn()
    }
  }
}

fn soak(
  states: &mut HashMap<IVec3, State>,
  at: IVec3,
  lying: &mut [Lying],
  commands: &mut Commands
) {
  let mana = holds(states[&at].intake, Liquid::Mana);
  let found = lying.iter_mut().find_map(|(entity, loose)| {
    rite_for(Block::ManaCistern, loose.block)
      .filter(|_| loose.count > 0 && resting_in(loose, at))
      .map(|recipe| (entity, loose, recipe))
  });
  let state = states.get_mut(&at).expect("pools have states");
  state.status = Status::Idle;
  if let Some((entity, loose, recipe)) = found
    && let Cost::Mana(cost) = recipe.cost
  {
    let affordable = ((mana / cost as f32) as u16).min(loose.count);
    state.status = match affordable > 0 {
      true => Status::Running,
      false => Status::NoMana
    };
    if affordable > 0 {
      spill(&mut state.intake, f32::from(affordable) * cost as f32);
      let (output, count) = recipe.output;
      crate::loose::fling(
        commands,
        Stack::of(output, count * affordable),
        centre(at) + Vec3::Y * 0.3,
        Vec3::Y * 3.5,
        0.6
      );
      loose.count -= affordable;
      if loose.count == 0 {
        commands.entity(*entity).despawn()
      }
    }
  }
}

fn feed(
  states: &mut HashMap<IVec3, State>,
  at: IVec3,
  lying: &mut [Lying],
  commands: &mut Commands
) {
  let state = states.get_mut(&at).expect("flowers have states");
  if state.stock < 5.0
    && let Some((entity, loose)) = lying.iter_mut().find(|(_, loose)| {
      loose.count > 0
        && burns(loose.block).is_some()
        && loose.at.distance(centre(at)) < EATS_WITHIN
    })
  {
    state.stock += burns(loose.block).unwrap_or_default();
    loose.count -= 1;
    if loose.count == 0 {
      commands.entity(*entity).despawn()
    }
  }
}

fn run(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut works: ResMut<Works>,
  weather: Query<&Weather>,
  mut loose: Query<(Entity, &mut Loose), Without<Riding>>,
  mut commands: Commands,
  mut since: Local<f32>
) {
  *since = (*since + time.delta_secs()).min(TICK_SECONDS * 4.0);
  if *since >= TICK_SECONDS {
    *since -= TICK_SECONDS;
    let sky = weather.iter().next().map_or(Sky::Clear, |weather| weather.sky);
    let states = &mut works.states;
    let of = |kind: fn(Block) -> bool| -> Vec<IVec3> {
      ordered(states.iter().filter(|(_, state)| kind(state.block)).map(|(&at, _)| at))
    };
    let (flowers, pools, cauldrons) = (
      of(Block::bloomer),
      of(|block| block == Block::ManaCistern),
      of(|block| block == Block::Cauldron)
    );
    let (blood, infusion) = (
      of(|block| block == Block::CrimsonAltar),
      of(|block| block == Block::ConfluenceAltar)
    );
    let mut lying: Vec<Lying> = loose.iter_mut().collect();
    for &at in flowers.iter() {
      if states[&at].block == Block::Emberbell {
        feed(states, at, &mut lying, &mut commands)
      }
      let state = states.get_mut(&at).expect("flowers have states");
      state.status = bloom(&mut voxels, sky, at, state)
    }
    bind(states, &flowers);
    for &at in pools.iter() {
      soak(states, at, &mut lying, &mut commands)
    }
    for &at in cauldrons.iter() {
      steep(&mut voxels, states, at, &mut lying, &mut commands)
    }
    bleed(&mut voxels, states, &blood);
    kindle(states, &infusion);
  }
}

fn tend(
  mut tends: MessageReader<FromClient<Tend>>,
  mut voxels: ResMut<Voxels>,
  mut works: ResMut<Works>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory, &mut Health))>
) {
  for &FromClient { client_id, message: Tend { at, slot } } in tends.read() {
    let block = voxels.ensure(at);
    if block.holder()
      && let Some((avatar, mut inventory, mut health)) =
        player_of(players.iter_mut(), client_id)
      && within_reach(avatar, at)
      && let Some(state) = works.states.get_mut(&at)
      && state.block == block
    {
      let held = inventory.slots.get(usize::from(slot)).copied().flatten();
      match (state.item, held) {
        (_, Some(stack))
          if block == Block::CrimsonAltar && stack.block == Block::RitualKnife =>
        {
          if health.0 > SACRIFICE {
            health.0 -= SACRIFICE;
            let runes = state.stock;
            pour(
              &mut state.intake,
              Liquid::Lifeblood,
              LIFEBLOOD * (1.0 + runes * 0.1),
              altar_room(runes as u32)
            );
          }
        }
        (Some(item), _) => {
          if inventory.gain(item) == 0 {
            state.item = None;
            state.charge = 0.0;
            state.done = false
          }
        }
        (None, Some(stack)) => {
          let one = Stack { count: 1, ..stack };
          inventory.slots[usize::from(slot)] =
            (stack.count > 1).then_some(Stack { count: stack.count - 1, ..stack });
          state.item = Some(one);
          state.charge = 0.0;
          state.done = false
        }
        (None, None) => ()
      }
    }
  }
}

fn sacrificed(
  slain: On<Add, Dying>,
  victims: Query<(Option<&Folk>, Option<&Beast>, Option<&Bird>)>,
  mut works: ResMut<Works>
) {
  if let Ok((folk, beast, bird)) = victims.get(slain.entity) {
    let at = folk
      .map(|folk| folk.at)
      .or(beast.map(|beast| beast.at))
      .or(bird.map(|bird| bird.at));
    if let Some(at) = at {
      for (_, state) in works.states.iter_mut().filter(|(altar, state)| {
        state.block == Block::CrimsonAltar && centre(**altar).distance(at) <= SLAIN_REACH
      }) {
        let room = altar_room(state.stock as u32);
        pour(&mut state.intake, Liquid::Lifeblood, SLAIN_BLOOD, room);
      }
    }
  }
}

pub struct Arcana;

impl Plugin for Arcana {
  fn build(&self, app: &mut App) {
    app.add_observer(sacrificed).add_systems(
      Update,
      (tend, run)
        .chain()
        .after(crate::machine::Running)
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
    for x in -4..=8 {
      for z in -4..=4 {
        voxels.ensure(IVec3::new(4000 + x, 200, 4000 + z));
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

  const BASE: IVec3 = IVec3::new(4000, 201, 4000);

  #[test]
  fn a_hydroangea_beside_water_fills_a_pool() {
    let mut voxels = world();
    let mut states = HashMap::new();
    voxels.set(BASE + IVec3::NEG_X, Block::Water);
    placed(&mut voxels, &mut states, BASE, Block::Dewcup);
    placed(&mut voxels, &mut states, BASE + IVec3::X * 3, Block::ManaCistern);
    for _ in 0..100 {
      let state = states.get_mut(&BASE).unwrap();
      state.status = bloom(&mut voxels, Sky::Clear, BASE, state);
      bind(&mut states, &[BASE])
    }
    let mana = holds(states[&(BASE + IVec3::X * 3)].intake, Liquid::Mana);
    assert!(mana > 150.0, "pool holds {mana}");
    assert_eq!(states[&BASE].link, Some(BASE + IVec3::X * 3))
  }

  #[test]
  fn an_infusion_altar_fuses_terrasteel_from_its_pedestals() {
    let mut voxels = world();
    let mut states = HashMap::new();
    placed(&mut voxels, &mut states, BASE, Block::ConfluenceAltar);
    placed(&mut voxels, &mut states, BASE + IVec3::new(5, 0, 0), Block::ManaCistern);
    states.get_mut(&(BASE + IVec3::new(5, 0, 0))).unwrap().intake =
      Some((Liquid::Mana, POOL));
    for (offset, item) in
      [(IVec3::new(2, 0, 0), Block::Starheart), (IVec3::new(-2, 0, 0), Block::Tidepearl)]
    {
      placed(&mut voxels, &mut states, BASE + offset, Block::Plinth);
      states.get_mut(&(BASE + offset)).unwrap().item = Some(Stack::of(item, 1))
    }
    states.get_mut(&BASE).unwrap().item = Some(Stack::of(Block::Glimsteel, 1));
    for _ in 0..60 {
      kindle(&mut states, &[BASE])
    }
    assert_eq!(states[&BASE].item.map(|stack| stack.block), Some(Block::Verdantine));
    assert!(states[&(BASE + IVec3::new(2, 0, 0))].item.is_none());
    let left = holds(states[&(BASE + IVec3::new(5, 0, 0))].intake, Liquid::Mana);
    assert!((left - (POOL - 6000.0)).abs() < 1.0, "pool left with {left}")
  }

  #[test]
  fn a_charm_adds_a_perk_to_boots() {
    let mut voxels = world();
    let mut states = HashMap::new();
    placed(&mut voxels, &mut states, BASE, Block::ConfluenceAltar);
    placed(&mut voxels, &mut states, BASE + IVec3::Z * 4, Block::ManaCistern);
    states.get_mut(&(BASE + IVec3::Z * 4)).unwrap().intake = Some((Liquid::Mana, POOL));
    for (offset, item) in [
      (IVec3::new(1, 0, 2), Block::Feather),
      (IVec3::new(-1, 0, 2), Block::Feather),
      (IVec3::new(0, 0, -2), Block::Tidepearl)
    ] {
      placed(&mut voxels, &mut states, BASE + offset, Block::Plinth);
      states.get_mut(&(BASE + offset)).unwrap().item = Some(Stack::of(item, 1))
    }
    states.get_mut(&BASE).unwrap().item = Some(Stack::of(Block::Boots, 1));
    for _ in 0..40 {
      kindle(&mut states, &[BASE])
    }
    let gear = states[&BASE].item.and_then(|stack| stack.gear).unwrap_or_default();
    assert!(gear.perks().any(|perk| perk.virtue == Virtue::Springy))
  }

  #[test]
  fn a_blood_altar_turns_stone_into_a_slate() {
    let mut voxels = world();
    let mut states = HashMap::new();
    placed(&mut voxels, &mut states, BASE, Block::CrimsonAltar);
    voxels.set(BASE + IVec3::new(2, -1, 0), Block::CrimsonRune);
    let state = states.get_mut(&BASE).unwrap();
    state.intake = Some((Liquid::Lifeblood, 3000.0));
    state.item = Some(Stack::of(Block::Stone, 1));
    for _ in 0..40 {
      bleed(&mut voxels, &mut states, &[BASE])
    }
    assert_eq!(states[&BASE].item.map(|stack| stack.block), Some(Block::BareTablet));
    assert_eq!(states[&BASE].status, Status::Finished);
    assert_eq!(states[&BASE].stock, 1.0);
    assert!((holds(states[&BASE].intake, Liquid::Lifeblood) - 2000.0).abs() < 1.0)
  }
}
