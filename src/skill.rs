use {crate::{authority::{Controller, player_of},
             block::Block,
             condition::{Conditions, Kind},
             folk::{PUNCH, Vigour},
             glyph::{self, Glyph},
             hud::SHADE,
             menu::{Act, EDGE, FAINT, INK, LIT, Menu, Pressed, Tab, words},
             player::Pilot,
             protocol::{Health, Notice, authority, plays},
             stance::Stance},
     bevy::{input::mouse::AccumulatedMouseScroll, prelude::*,
            ui::RelativeCursorPosition, window::PrimaryWindow},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize},
     std::sync::LazyLock};

const SPACING: f32 = 58.0;
const NODE: f32 = 40.0;
const STARTING_POINTS: i32 = 2;
const DRAG: f32 = 4.0;
const ZOOMS: (f32, f32) = (0.35, 1.8);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Branch {
  Origin,
  Wayfarer,
  Warden,
  Brawler,
  Delver,
  Arcanist
}

impl Branch {
  pub fn colour(self) -> Color {
    match self {
      Branch::Origin => Color::srgb(0.95, 0.88, 0.6),
      Branch::Wayfarer => Color::srgb(0.45, 0.88, 0.45),
      Branch::Warden => Color::srgb(0.95, 0.4, 0.4),
      Branch::Brawler => Color::srgb(0.98, 0.62, 0.25),
      Branch::Delver => Color::srgb(0.9, 0.78, 0.35),
      Branch::Arcanist => Color::srgb(0.68, 0.5, 1.0)
    }
  }

  fn title(self) -> &'static str {
    match self {
      Branch::Origin => "Awakening",
      Branch::Wayfarer => "Wayfarer",
      Branch::Warden => "Warden",
      Branch::Brawler => "Brawler",
      Branch::Delver => "Delver",
      Branch::Arcanist => "Arcanist"
    }
  }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Boon {
  Origin,
  Stride(f32),
  Spring(f32),
  Swim(f32),
  Vigor(u8),
  Mend(f32),
  Guard(f32),
  Might(u8),
  Delve(f32),
  Fortune(f32),
  Thrift(f32),
  Spell(Block),
  Empower(Block),
  Stance(Stance),
  Lunge,
  Momentum
}

pub fn spell_glyph(spell: Block) -> Glyph {
  match spell {
    Block::SpellFirebolt => Glyph::Flame,
    Block::SpellFrost => Glyph::Snowflake,
    Block::SpellMissile => Glyph::Missile,
    Block::SpellHeal => Glyph::Mend,
    Block::SpellHaste => Glyph::Hourglass,
    Block::SpellStoneskin => Glyph::Stone,
    Block::SpellLeap => Glyph::Feather,
    Block::SpellLight => Glyph::Sun,
    Block::SpellFireball => Glyph::Burst,
    Block::SpellIceStorm => Glyph::Storm,
    Block::SpellAnimate => Glyph::Skull,
    Block::SpellGuardian => Glyph::Ghost,
    Block::SpellInvisibility => Glyph::Cloak,
    _ => Glyph::Thunder
  }
}

pub fn mastery(spell: Block) -> &'static str {
  match spell {
    Block::SpellFirebolt => "Each rank adds 2 damage; rank 2 sets targets burning",
    Block::SpellFrost => "Each rank adds 1 damage and 2 s of chill",
    Block::SpellMissile => "Each rank adds another dart",
    Block::SpellHeal => "Each rank heals 2 more; rank 2 adds regeneration",
    Block::SpellHaste => "Each rank lasts 15 s longer",
    Block::SpellStoneskin => "Each rank lasts 15 s longer",
    Block::SpellLeap => "Each rank lasts 20 s longer",
    Block::SpellLight => "Each rank lasts a minute longer",
    Block::SpellFireball => "Each rank adds 2 damage and a wider blast",
    Block::SpellIceStorm => "Each rank adds 1 damage, a second and a wider storm",
    Block::SpellAnimate => "Each rank keeps the dead up 15 s longer; rank 2 raises two",
    Block::SpellGuardian => "Each rank keeps the guardian 15 s longer",
    Block::SpellInvisibility => "Each rank lasts 10 s longer",
    _ => "Each rank adds 1 damage and a wider blast"
  }
}

pub fn spell_text(spell: Block) -> &'static str {
  match spell {
    Block::SpellFirebolt => "Hurl a mote of fire that bursts on what it hits",
    Block::SpellFrost => "A freezing ray that hurts and slows",
    Block::SpellMissile => "Glowing darts that seek out nearby foes",
    Block::SpellHeal => "Mend your wounds at once",
    Block::SpellHaste => "Move 40% faster for a while",
    Block::SpellStoneskin => "Your skin turns to stone: take half damage",
    Block::SpellLeap => "Leap far higher and fall gently",
    Block::SpellLight => "A floating light follows you through the dark",
    Block::SpellFireball => "A ball of fire that bursts, burning everything near it",
    Block::SpellIceStorm => {
      "Call a storm of hail where you aim: it hurts and chills all within"
    }
    Block::SpellAnimate => "Raise a skeleton thrall that fights at your side for a while",
    Block::SpellGuardian => "Summon a spirit guardian whose touch chills your foes",
    Block::SpellInvisibility => "Vanish from your foes' sight until you attack",
    _ => "A thunderous wave throws back everything around you"
  }
}

impl Boon {
  fn glyph(self) -> Glyph {
    match self {
      Boon::Origin => Glyph::Star,
      Boon::Stride(_) => Glyph::Boot,
      Boon::Spring(_) => Glyph::Wing,
      Boon::Swim(_) => Glyph::Wave,
      Boon::Vigor(_) => Glyph::Heart,
      Boon::Mend(_) => Glyph::Mend,
      Boon::Guard(_) => Glyph::Shield,
      Boon::Might(_) => Glyph::Fist,
      Boon::Delve(_) => Glyph::Pick,
      Boon::Fortune(_) => Glyph::Gem,
      Boon::Thrift(_) => Glyph::Rune,
      Boon::Spell(spell) | Boon::Empower(spell) => spell_glyph(spell),
      Boon::Stance(stance) => stance.glyph(),
      Boon::Lunge | Boon::Momentum => Glyph::Dash
    }
  }

  pub fn name(self) -> String {
    match self {
      Boon::Origin => "Awakening".into(),
      Boon::Stride(_) => "Fleet Foot".into(),
      Boon::Spring(_) => "Spring Step".into(),
      Boon::Swim(_) => "Waterborn".into(),
      Boon::Vigor(_) => "Hardy".into(),
      Boon::Mend(_) => "Quick Mending".into(),
      Boon::Guard(_) => "Thick Skin".into(),
      Boon::Might(_) => "Heavy Hands".into(),
      Boon::Delve(_) => "Delver's Knack".into(),
      Boon::Fortune(_) => "Prospector".into(),
      Boon::Thrift(_) => "Frugal Casting".into(),
      Boon::Spell(spell) => format!("Learn {}", spell.name()),
      Boon::Empower(spell) => format!("{} Mastery", spell.name()),
      Boon::Stance(stance) => stance.name().into(),
      Boon::Lunge => "Lunge".into(),
      Boon::Momentum => "Momentum".into()
    }
  }

  pub fn describe(self) -> String {
    let percent = |by: f32| (by * 100.0).round() as i32;
    match self {
      Boon::Origin => "Where every path begins".into(),
      Boon::Stride(by) => format!("Walk, run and climb {}% faster", percent(by)),
      Boon::Spring(by) => format!("Jump {}% higher", percent(by)),
      Boon::Swim(by) => format!("Swim {}% faster", percent(by)),
      Boon::Vigor(by) => format!("{} more maximum health", by),
      Boon::Mend(by) => format!("Wounds mend {}% faster", percent(by)),
      Boon::Guard(by) => format!("Take {}% less damage", percent(by)),
      Boon::Might(by) => format!("Punches deal {} more damage", by),
      Boon::Delve(by) => format!("Dig {}% faster", percent(by)),
      Boon::Fortune(by) => format!("{}% chance of a second ore", percent(by)),
      Boon::Thrift(by) => format!("{}% chance a spell costs nothing", percent(by)),
      Boon::Spell(spell) => format!(
        "{}. Craft it from {}: each cast uses one",
        spell_text(spell),
        crate::recipe::making(spell)
          .flat_map(|(_, recipe)| recipe.inputs.iter())
          .map(|&(block, count)| format!("{count} {}", block.name()))
          .collect::<Vec<_>>()
          .join(" and ")
      ),
      Boon::Empower(spell) => mastery(spell).into(),
      Boon::Stance(stance) => format!(
        "{}. Only one stance can be active: switch with G or on this page",
        stance.describe()
      ),
      Boon::Lunge => {
        "Press F to dash forward, striking up to three foes in your path".into()
      }
      Boon::Momentum => "Lunges deal 2 more damage".into()
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Talent {
  pub at: Vec2,
  pub parent: Option<u16>,
  pub boon: Boon,
  pub cost: u8,
  pub branch: Branch
}

struct Grower(Vec<Talent>);

impl Grower {
  fn add(&mut self, parent: Option<u16>, at: Vec2, boon: Boon, branch: Branch) -> u16 {
    let cost = match boon {
      Boon::Spell(_) | Boon::Stance(_) | Boon::Lunge => 2,
      Boon::Origin => 0,
      _ => 1
    };
    self.0.push(Talent { at, parent, boon, cost, branch });
    (self.0.len() - 1) as u16
  }

  fn arm(
    &mut self,
    from: u16,
    heading: f32,
    side: f32,
    boons: &[Boon],
    branch: Branch
  ) -> Vec<u16> {
    let heading = self.0[usize::from(from)].parent.map_or(heading, |parent| {
      (self.0[usize::from(from)].at - self.0[usize::from(parent)].at).to_angle()
    });
    boons
      .iter()
      .enumerate()
      .fold((Vec::new(), from), |(mut ids, parent), (step, &boon)| {
        let turn = (62.0 - step as f32 * 11.0).max(16.0).to_radians() * side;
        let at = self.0[usize::from(parent)].at + Vec2::from_angle(heading + turn);
        let id = self.add(Some(parent), at, boon, branch);
        ids.push(id);
        (ids, id)
      })
      .0
  }

  fn stem(
    &mut self,
    from: u16,
    heading: f32,
    bend: f32,
    boons: &[Boon],
    branch: Branch
  ) -> Vec<u16> {
    boons
      .iter()
      .fold((Vec::new(), from, heading), |(mut ids, parent, heading), &boon| {
        let at = self.0[usize::from(parent)].at + Vec2::from_angle(heading);
        let id = self.add(Some(parent), at, boon, branch);
        ids.push(id);
        (ids, id, heading + bend)
      })
      .0
  }
}

fn grow() -> Vec<Talent> {
  let mut grower = Grower(Vec::new());
  let origin = grower.add(None, Vec2::ZERO, Boon::Origin, Branch::Origin);
  let degrees = |angle: f32| angle.to_radians();
  let spell_arm =
    |grower: &mut Grower, from: u16, heading: f32, side: f32, spell: Block, branch| {
      grower.arm(
        from,
        heading,
        side,
        &[
          Boon::Spell(spell),
          Boon::Empower(spell),
          Boon::Thrift(0.04),
          Boon::Empower(spell),
          Boon::Empower(spell)
        ],
        branch
      )
    };
  let trunk = |grower: &mut Grower, heading: f32, boons: &[Boon], branch| {
    let start =
      grower.add(Some(origin), Vec2::from_angle(heading) * 1.6, boons[0], branch);
    std::iter::once(start)
      .chain(grower.stem(start, heading, degrees(1.6), &boons[1..], branch))
      .collect::<Vec<u16>>()
  };
  let wayfarer = degrees(-90.0);
  let ways = trunk(
    &mut grower,
    wayfarer,
    &[
      Boon::Stride(0.03),
      Boon::Stride(0.03),
      Boon::Spring(0.06),
      Boon::Stride(0.03),
      Boon::Swim(0.15),
      Boon::Stride(0.03),
      Boon::Spring(0.06),
      Boon::Stride(0.03),
      Boon::Stride(0.04),
      Boon::Swim(0.15),
      Boon::Stride(0.04),
      Boon::Spring(0.08),
      Boon::Stride(0.04),
      Boon::Stride(0.04),
      Boon::Swim(0.2),
      Boon::Stride(0.05),
      Boon::Spring(0.1),
      Boon::Stride(0.05),
      Boon::Stride(0.06)
    ],
    Branch::Wayfarer
  );
  spell_arm(&mut grower, ways[2], wayfarer, -1.0, Block::SpellLeap, Branch::Wayfarer);
  spell_arm(&mut grower, ways[8], wayfarer, 1.0, Block::SpellHaste, Branch::Wayfarer);
  grower.arm(
    ways[13],
    wayfarer,
    -1.0,
    &[Boon::Stride(0.04), Boon::Spring(0.08), Boon::Swim(0.25), Boon::Stride(0.06)],
    Branch::Wayfarer
  );
  let warden = degrees(-18.0);
  let guards = trunk(
    &mut grower,
    warden,
    &[
      Boon::Vigor(2),
      Boon::Guard(0.03),
      Boon::Mend(0.15),
      Boon::Vigor(2),
      Boon::Guard(0.03),
      Boon::Vigor(2),
      Boon::Mend(0.15),
      Boon::Guard(0.04),
      Boon::Vigor(2),
      Boon::Guard(0.04),
      Boon::Mend(0.2),
      Boon::Vigor(2),
      Boon::Guard(0.05),
      Boon::Vigor(2),
      Boon::Mend(0.25),
      Boon::Guard(0.05),
      Boon::Vigor(2),
      Boon::Vigor(2)
    ],
    Branch::Warden
  );
  spell_arm(&mut grower, guards[3], warden, -1.0, Block::SpellHeal, Branch::Warden);
  spell_arm(&mut grower, guards[9], warden, 1.0, Block::SpellStoneskin, Branch::Warden);
  let brawler = degrees(54.0);
  let fists = trunk(
    &mut grower,
    brawler,
    &[
      Boon::Might(1),
      Boon::Guard(0.02),
      Boon::Vigor(1),
      Boon::Might(1),
      Boon::Stride(0.02),
      Boon::Guard(0.03),
      Boon::Might(1),
      Boon::Vigor(2),
      Boon::Mend(0.1),
      Boon::Might(1),
      Boon::Guard(0.03),
      Boon::Vigor(2),
      Boon::Might(1),
      Boon::Guard(0.04),
      Boon::Might(2)
    ],
    Branch::Brawler
  );
  spell_arm(&mut grower, fists[4], brawler, 1.0, Block::SpellThunder, Branch::Brawler);
  grower.arm(
    fists[9],
    brawler,
    -1.0,
    &[Boon::Might(1), Boon::Vigor(2), Boon::Might(1), Boon::Guard(0.05)],
    Branch::Brawler
  );
  let delver = degrees(126.0);
  let picks = trunk(
    &mut grower,
    delver,
    &[
      Boon::Delve(0.08),
      Boon::Delve(0.08),
      Boon::Fortune(0.04),
      Boon::Delve(0.08),
      Boon::Stride(0.02),
      Boon::Delve(0.1),
      Boon::Fortune(0.05),
      Boon::Delve(0.1),
      Boon::Vigor(1),
      Boon::Delve(0.1),
      Boon::Fortune(0.06),
      Boon::Delve(0.12),
      Boon::Delve(0.12),
      Boon::Fortune(0.08),
      Boon::Delve(0.15)
    ],
    Branch::Delver
  );
  spell_arm(&mut grower, picks[3], delver, 1.0, Block::SpellLight, Branch::Delver);
  grower.arm(
    picks[9],
    delver,
    -1.0,
    &[Boon::Fortune(0.05), Boon::Delve(0.1), Boon::Fortune(0.06), Boon::Delve(0.12)],
    Branch::Delver
  );
  let arcanist = degrees(198.0);
  let runes = trunk(
    &mut grower,
    arcanist,
    &[
      Boon::Thrift(0.04),
      Boon::Thrift(0.03),
      Boon::Mend(0.1),
      Boon::Thrift(0.03),
      Boon::Vigor(1),
      Boon::Thrift(0.04),
      Boon::Thrift(0.04),
      Boon::Guard(0.03),
      Boon::Thrift(0.05),
      Boon::Thrift(0.05),
      Boon::Vigor(2),
      Boon::Thrift(0.06)
    ],
    Branch::Arcanist
  );
  spell_arm(
    &mut grower,
    runes[0],
    arcanist,
    -1.0,
    Block::SpellFirebolt,
    Branch::Arcanist
  );
  spell_arm(&mut grower, runes[4], arcanist, 1.0, Block::SpellFrost, Branch::Arcanist);
  spell_arm(&mut grower, runes[8], arcanist, -1.0, Block::SpellMissile, Branch::Arcanist);
  spell_arm(
    &mut grower,
    ways[16],
    wayfarer,
    1.0,
    Block::SpellInvisibility,
    Branch::Wayfarer
  );
  spell_arm(&mut grower, guards[14], warden, -1.0, Block::SpellGuardian, Branch::Warden);
  grower.arm(
    guards[15],
    warden,
    1.0,
    &[Boon::Stance(Stance::Defender), Boon::Guard(0.04), Boon::Vigor(2)],
    Branch::Warden
  );
  grower.arm(
    fists[1],
    brawler,
    -1.0,
    &[
      Boon::Stance(Stance::Berserker),
      Boon::Might(1),
      Boon::Stance(Stance::Duelist),
      Boon::Stride(0.03)
    ],
    Branch::Brawler
  );
  grower.arm(
    fists[12],
    brawler,
    1.0,
    &[Boon::Lunge, Boon::Momentum, Boon::Stride(0.03), Boon::Momentum, Boon::Momentum],
    Branch::Brawler
  );
  let tip = runes[runes.len() - 1];
  let onward = (grower.0[usize::from(tip)].at
    - grower.0[usize::from(runes[runes.len() - 2])].at)
    .to_angle();
  let runes: Vec<u16> = runes
    .iter()
    .copied()
    .chain(grower.stem(
      tip,
      onward + degrees(1.6),
      degrees(1.6),
      &[
        Boon::Thrift(0.04),
        Boon::Mend(0.1),
        Boon::Thrift(0.05),
        Boon::Vigor(2),
        Boon::Thrift(0.05),
        Boon::Thrift(0.06),
        Boon::Vigor(2),
        Boon::Thrift(0.08)
      ],
      Branch::Arcanist
    ))
    .collect();
  spell_arm(
    &mut grower,
    runes[10],
    arcanist,
    1.0,
    Block::SpellFireball,
    Branch::Arcanist
  );
  spell_arm(
    &mut grower,
    runes[13],
    arcanist,
    -1.0,
    Block::SpellIceStorm,
    Branch::Arcanist
  );
  grower.arm(
    runes[15],
    arcanist,
    1.0,
    &[Boon::Stance(Stance::Mystic), Boon::Thrift(0.05), Boon::Mend(0.1)],
    Branch::Arcanist
  );
  spell_arm(
    &mut grower,
    runes[17],
    arcanist,
    -1.0,
    Block::SpellAnimate,
    Branch::Arcanist
  );
  grower.0
}

pub static TREE: LazyLock<Vec<Talent>> = LazyLock::new(grow);

#[derive(Component, Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct Skills {
  pub xp: u32,
  pub level: u32,
  pub learned: Vec<u16>,
  #[serde(default)]
  pub stance: Option<Stance>
}

pub fn needed(level: u32) -> u32 { 60 + level * 35 }

#[derive(Clone, Copy, Debug)]
pub struct Stats {
  pub stride: f32,
  pub spring: f32,
  pub swim: f32,
  pub vigor: u8,
  pub mend: f32,
  pub guard: f32,
  pub might: u8,
  pub delve: f32,
  pub fortune: f32,
  pub thrift: f32
}

impl Stats {
  fn with(self, stance: Option<Stance>) -> Stats {
    match stance {
      Some(stance) => Stats {
        stride: self.stride * stance.pace(),
        thrift: (self.thrift + stance.thrift()).min(0.8),
        ..self
      },
      None => self
    }
  }
}

impl Skills {
  pub fn knows(&self, node: u16) -> bool { node == 0 || self.learned.contains(&node) }

  pub fn spent(&self) -> i32 {
    self
      .learned
      .iter()
      .filter_map(|&node| TREE.get(usize::from(node)))
      .map(|node| i32::from(node.cost))
      .sum()
  }

  pub fn points(&self) -> i32 { STARTING_POINTS + self.level as i32 - self.spent() }

  pub fn open(&self, node: u16) -> bool {
    TREE
      .get(usize::from(node))
      .is_some_and(|found| found.parent.is_none_or(|parent| self.knows(parent)))
  }

  pub fn boons(&self) -> impl Iterator<Item = Boon> + '_ {
    self
      .learned
      .iter()
      .filter_map(|&node| TREE.get(usize::from(node)))
      .map(|node| node.boon)
  }

  pub fn spells(&self) -> Vec<Block> {
    self
      .boons()
      .filter_map(|boon| match boon {
        Boon::Spell(spell) => Some(spell),
        _ => None
      })
      .collect()
  }

  pub fn stances(&self) -> Vec<Stance> {
    Stance::ALL
      .into_iter()
      .filter(|&stance| self.boons().any(|boon| boon == Boon::Stance(stance)))
      .collect()
  }

  pub fn lunges(&self) -> bool { self.boons().any(|boon| boon == Boon::Lunge) }

  pub fn momentum(&self) -> u8 {
    self.boons().filter(|&boon| boon == Boon::Momentum).count() as u8
  }

  pub fn rank(&self, spell: Block) -> u8 {
    self.boons().filter(|&boon| boon == Boon::Empower(spell)).count() as u8
  }

  pub fn stats(&self) -> Stats {
    self
      .boons()
      .fold(
        Stats {
          stride: 1.0,
          spring: 1.0,
          swim: 1.0,
          vigor: 0,
          mend: 1.0,
          guard: 0.0,
          might: 0,
          delve: 1.0,
          fortune: 0.0,
          thrift: 0.0
        },
        |stats, boon| match boon {
          Boon::Stride(by) => Stats { stride: stats.stride + by, ..stats },
          Boon::Spring(by) => Stats { spring: stats.spring + by, ..stats },
          Boon::Swim(by) => Stats { swim: stats.swim + by, ..stats },
          Boon::Vigor(by) => Stats { vigor: stats.vigor + by, ..stats },
          Boon::Mend(by) => Stats { mend: stats.mend + by, ..stats },
          Boon::Guard(by) => Stats { guard: (stats.guard + by).min(0.6), ..stats },
          Boon::Might(by) => Stats { might: stats.might + by, ..stats },
          Boon::Delve(by) => Stats { delve: stats.delve + by, ..stats },
          Boon::Fortune(by) => Stats { fortune: stats.fortune + by, ..stats },
          Boon::Thrift(by) => Stats { thrift: (stats.thrift + by).min(0.6), ..stats },
          _ => stats
        }
      )
      .with(self.stance)
  }

  pub fn most(&self) -> u8 { Health::FULL + self.stats().vigor }
}

#[derive(Message, Clone, Copy)]
pub struct Earned {
  pub player: Entity,
  pub xp: u32
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Learn(pub u16);

fn study(
  mut lessons: MessageReader<FromClient<Learn>>,
  mut players: Query<(&Controller, &mut Skills)>
) {
  for &FromClient { client_id, message: Learn(node) } in lessons.read() {
    if let Some(mut skills) = player_of(players.iter_mut(), client_id)
      && let Some(found) = TREE.get(usize::from(node))
      && !skills.knows(node)
      && skills.open(node)
      && skills.points() >= i32::from(found.cost)
    {
      skills.learned.push(node)
    }
  }
}

fn earn(
  mut earnings: MessageReader<Earned>,
  mut players: Query<(&Controller, &mut Skills)>,
  mut notices: MessageWriter<ToClients<Notice>>
) {
  for &Earned { player, xp } in earnings.read() {
    if let Ok((controller, mut skills)) = players.get_mut(player) {
      let before = skills.level;
      skills.xp += xp;
      while skills.xp >= needed(skills.level) {
        let cost = needed(skills.level);
        skills.xp -= cost;
        skills.level += 1
      }
      if skills.level > before {
        notices.write(ToClients {
          targets: SendTargets::Single(controller.client),
          message: Notice(format!(
            "Level {}! You have {} skill points: press K",
            skills.level,
            skills.points()
          ))
        });
      }
    }
  }
}

fn temper(mut players: Query<(&Skills, &Conditions, &mut Vigour), With<Controller>>) {
  for (skills, conditions, mut vigour) in players.iter_mut() {
    let stats = skills.stats();
    let stance = skills.stance;
    let stone = if conditions.has(Kind::Stoneskin) { 0.5 } else { 1.0 };
    let might =
      stance.map_or(PUNCH + stats.might, |stance| stance.punch(PUNCH + stats.might));
    let punch = if conditions.has(Kind::Weakened) { might.div_ceil(2) } else { might };
    let most = Health::FULL + stats.vigor;
    let ward = (1.0 - stats.guard) * stone * stance.map_or(1.0, Stance::ward);
    let quick = stance.map_or(crate::stance::STRIKE_EVERY, Stance::quick);
    if vigour.most != most
      || vigour.ward != ward
      || vigour.punch != punch
      || vigour.mend != stats.mend
      || vigour.quick != quick
    {
      vigour.most = most;
      vigour.ward = ward;
      vigour.punch = punch;
      vigour.mend = stats.mend;
      vigour.quick = quick
    }
  }
}

#[derive(Resource)]
pub struct Glyphs(pub Handle<Image>);

fn prepare(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
  let grey = [[0.12, 0.12, 0.12], [0.5, 0.5, 0.5], [0.8, 0.8, 0.8], [1.0, 1.0, 1.0]];
  let paints: Vec<(Glyph, glyph::Shades)> =
    Glyph::ALL.iter().map(|&glyph| (glyph, grey)).collect();
  commands.insert_resource(Glyphs(images.add(glyph::sheet(&paints))))
}

pub fn glyph_image(
  glyphs: &Glyphs,
  glyph: Glyph,
  colour: Color,
  size: f32
) -> impl Bundle {
  (
    ImageNode {
      rect: Some(glyph::cell(glyph.index())),
      color: colour,
      ..ImageNode::new(glyphs.0.clone())
    },
    Node { width: px(size), height: px(size), ..default() }
  )
}

#[derive(Component)]
struct Canvas;

#[derive(Component)]
struct Header;

#[derive(Component)]
struct Stances;

#[derive(Component)]
struct Spot(u16);

#[derive(Component)]
struct Mark(u16);

#[derive(Component)]
struct Bough(u16);

#[derive(Component)]
struct Label(Branch);

#[derive(Component)]
struct Hint;

#[derive(Resource)]
struct View {
  pan: Vec2,
  zoom: f32,
  grip: Option<(Vec2, Vec2)>,
  dragged: bool
}

impl Default for View {
  fn default() -> Self { View { pan: Vec2::ZERO, zoom: 0.8, grip: None, dragged: false } }
}

pub fn page(page: &mut ChildSpawnerCommands) {
  page
    .spawn(Node { column_gap: px(14), align_items: AlignItems::Center, ..default() })
    .with_children(|row| {
      row.spawn((Header, words("", 18.0, INK)));
    });
  page.spawn((Stances, Node {
    column_gap: px(6),
    align_items: AlignItems::Center,
    min_height: px(30),
    ..default()
  }));
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
    BackgroundColor(Color::srgba(0.02, 0.025, 0.04, 0.9))
  ));
  page.spawn(words(
    "Drag to pan and scroll to zoom. Click a glowing node next to one you know to learn it. G switches stance, F lunges once learned. \
     Earn experience by defeating foes, mining ore, opening chests and finding waystones.",
    13.0,
    FAINT
  ));
}

fn plant(
  canvases: Query<Entity, (With<Canvas>, Without<Children>)>,
  glyphs: Res<Glyphs>,
  mut commands: Commands
) {
  for canvas in canvases.iter() {
    commands.entity(canvas).with_children(|canvas| {
      for (index, node) in TREE.iter().enumerate() {
        if node.parent.is_some() {
          canvas.spawn((
            Bough(index as u16),
            Node { position_type: PositionType::Absolute, ..default() },
            BackgroundColor(EDGE)
          ));
        }
      }
      for branch in [
        Branch::Wayfarer,
        Branch::Warden,
        Branch::Brawler,
        Branch::Delver,
        Branch::Arcanist
      ] {
        canvas.spawn((
          Label(branch),
          words(branch.title().to_uppercase(), 20.0, branch.colour().with_alpha(0.75)),
          SHADE,
          Node { position_type: PositionType::Absolute, ..default() }
        ));
      }
      for (index, node) in TREE.iter().enumerate() {
        canvas
          .spawn((
            Spot(index as u16),
            Button,
            Act::Learn(index as u16),
            Node {
              position_type: PositionType::Absolute,
              border: UiRect::all(px(3)),
              border_radius: BorderRadius::MAX,
              justify_content: JustifyContent::Center,
              align_items: AlignItems::Center,
              ..default()
            },
            BorderColor::all(EDGE),
            BackgroundColor(Color::srgb(0.06, 0.07, 0.09))
          ))
          .with_child((
            Mark(index as u16),
            glyph_image(&glyphs, node.boon.glyph(), Color::WHITE, 24.0)
          ));
      }
    });
  }
}

fn notable(boon: Boon) -> bool {
  matches!(boon, Boon::Spell(_) | Boon::Origin | Boon::Stance(_) | Boon::Lunge)
}

fn branch_end(branch: Branch) -> Vec2 {
  TREE
    .iter()
    .filter(|node| node.branch == branch)
    .map(|node| node.at)
    .max_by(|a, b| a.length().total_cmp(&b.length()))
    .unwrap_or_default()
}

fn steer(
  menu: Res<Menu>,
  scroll: Res<AccumulatedMouseScroll>,
  buttons: Res<ButtonInput<MouseButton>>,
  canvas: Query<(&RelativeCursorPosition, &ComputedNode), With<Canvas>>,
  mut view: ResMut<View>
) {
  if menu.showing(Tab::Skills)
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
      view.grip = Some((pixel, view.pan));
      view.dragged = false
    }
    if let Some((from, pan)) = view.grip
      && buttons.pressed(MouseButton::Left)
    {
      let moved = pixel - from;
      view.dragged |= moved.length() > DRAG;
      if view.dragged {
        view.pan = pan + moved
      }
    }
    if !buttons.pressed(MouseButton::Left) {
      view.grip = None
    }
  }
}

fn arrange(
  menu: Res<Menu>,
  view: Res<View>,
  canvas: Query<&ComputedNode, With<Canvas>>,
  mut spots: Query<(&Spot, &mut Node), (Without<Bough>, Without<Label>)>,
  mut boughs: Query<(&Bough, &mut Node, &mut UiTransform), Without<Spot>>,
  mut labels: Query<(&Label, &mut Node), (Without<Spot>, Without<Bough>)>,
  mut marks: Query<(&Mark, &mut Node), (Without<Spot>, Without<Bough>, Without<Label>)>
) {
  if menu.showing(Tab::Skills)
    && let Ok(computed) = canvas.single()
  {
    let size = computed.size * computed.inverse_scale_factor;
    let place = |at: Vec2| size / 2.0 + view.pan + at * SPACING * view.zoom;
    let wide = NODE * view.zoom.max(0.55);
    for (spot, mut node) in spots.iter_mut() {
      let centre = place(TREE[usize::from(spot.0)].at);
      let notable = notable(TREE[usize::from(spot.0)].boon);
      let wide = if notable { wide * 1.3 } else { wide };
      node.left = px(centre.x - wide / 2.0);
      node.top = px(centre.y - wide / 2.0);
      node.width = px(wide);
      node.height = px(wide);
    }
    for (mark, mut node) in marks.iter_mut() {
      let notable = notable(TREE[usize::from(mark.0)].boon);
      let glyph = (wide - 10.0).max(8.0) * if notable { 1.3 } else { 1.0 };
      node.width = px(glyph);
      node.height = px(glyph);
    }
    for (bough, mut node, mut transform) in boughs.iter_mut() {
      let child = &TREE[usize::from(bough.0)];
      let parent = &TREE[usize::from(child.parent.unwrap_or_default())];
      let (from, to) = (place(parent.at), place(child.at));
      let length = from.distance(to);
      let middle = (from + to) / 2.0;
      node.left = px(middle.x - length / 2.0);
      node.top = px(middle.y - 2.0);
      node.width = px(length);
      node.height = px(4.0);
      transform.rotation = Rot2::radians((to - from).to_angle());
    }
    for (label, mut node) in labels.iter_mut() {
      let end = branch_end(label.0);
      let at = place(end + end.normalize_or_zero() * 1.3);
      node.left = px(at.x - 50.0);
      node.top = px(at.y - 12.0);
    }
  }
}

fn standing(skills: &Skills, node: u16) -> (bool, bool) {
  (
    skills.knows(node),
    skills.open(node) && skills.points() >= i32::from(TREE[usize::from(node)].cost)
  )
}

fn paint(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  skills: Query<&Skills>,
  time: Res<Time>,
  mut spots: Query<(&Spot, &Interaction, &mut BorderColor, &mut BackgroundColor)>,
  mut marks: Query<(&Mark, &mut ImageNode)>,
  mut boughs: Query<(&Bough, &mut BackgroundColor), Without<Spot>>,
  mut headers: Query<&mut Text, With<Header>>
) {
  if menu.showing(Tab::Skills) {
    let skills =
      pilot.and_then(|pilot| skills.get(pilot.me).ok().cloned()).unwrap_or_default();
    let pulse = 0.6 + (time.elapsed_secs() * 4.0).sin() * 0.4;
    for (spot, interaction, mut border, mut background) in spots.iter_mut() {
      let node = &TREE[usize::from(spot.0)];
      let colour = node.branch.colour();
      let (known, ready) = standing(&skills, spot.0);
      let hovered = *interaction != Interaction::None;
      let edge = match (known, ready) {
        (true, _) => colour,
        (false, true) => Color::WHITE.mix(&colour, 0.5).with_alpha(pulse),
        _ => EDGE
      };
      border.set_if_neq(BorderColor::all(if hovered { LIT } else { edge }));
      background.set_if_neq(BackgroundColor(match known {
        true => colour.darker(0.55),
        false => Color::srgb(0.06, 0.07, 0.09)
      }));
    }
    for (mark, mut image) in marks.iter_mut() {
      let node = &TREE[usize::from(mark.0)];
      let (known, ready) = standing(&skills, mark.0);
      let tint = match (known, ready) {
        (true, _) => Color::WHITE,
        (false, true) => node.branch.colour(),
        _ => node.branch.colour().darker(0.45).with_alpha(0.7)
      };
      if image.color != tint {
        image.color = tint
      }
    }
    for (bough, mut background) in boughs.iter_mut() {
      let node = &TREE[usize::from(bough.0)];
      let lit = skills.knows(bough.0);
      background.set_if_neq(BackgroundColor(match lit {
        true => node.branch.colour().with_alpha(0.85),
        false => Color::srgba(0.3, 0.33, 0.36, 0.6)
      }));
    }
    let line = format!(
      "Level {}   |   {} skill point{} to spend   |   {} / {} experience to the next level",
      skills.level,
      skills.points(),
      if skills.points() == 1 { "" } else { "s" },
      skills.xp,
      needed(skills.level)
    );
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
        max_width: px(320),
        row_gap: px(2),
        ..default()
      },
      BorderColor::all(Color::srgb(0.3, 0.25, 0.5)),
      BackgroundColor(Color::srgba(0.05, 0.04, 0.09, 0.96)),
      GlobalZIndex(40),
      Visibility::Hidden
    ))
    .with_children(|tip| {
      tip.spawn(words("", 16.0, INK));
      tip.spawn(words("", 14.0, FAINT));
      tip.spawn(words("", 13.0, LIT));
    });
}

fn explain(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  skills: Query<&Skills>,
  spots: Query<(&Spot, &Interaction)>,
  windows: Query<&Window, With<PrimaryWindow>>,
  scale: Res<UiScale>,
  mut tips: Query<(&mut Node, &mut Visibility, &Children), With<Hint>>,
  mut texts: Query<(&mut Text, &mut TextColor)>
) {
  let skills =
    pilot.and_then(|pilot| skills.get(pilot.me).ok().cloned()).unwrap_or_default();
  let pointed = spots
    .iter()
    .find(|(_, interaction)| **interaction != Interaction::None)
    .map(|(spot, _)| spot.0)
    .filter(|_| menu.showing(Tab::Skills));
  let cursor =
    windows.single().ok().and_then(Window::cursor_position).map(|at| at / scale.0);
  for (mut node, mut visibility, children) in tips.iter_mut() {
    match pointed.zip(cursor) {
      Some((index, at)) => {
        let found = TREE[usize::from(index)];
        let (known, ready) = standing(&skills, index);
        let state = match (known, ready, skills.open(index)) {
          (true, ..) => "Learned".to_string(),
          (false, true, _) => format!(
            "Click to learn: {} point{}",
            found.cost,
            if found.cost == 1 { "" } else { "s" }
          ),
          (false, false, true) => format!(
            "Needs {} skill point{}",
            found.cost,
            if found.cost == 1 { "" } else { "s" }
          ),
          _ => "Learn a neighbouring node first".to_string()
        };
        let lines = [found.boon.name(), found.boon.describe(), state];
        node.left = px(at.x + 18.0);
        node.top = px(at.y + 14.0);
        visibility.set_if_neq(Visibility::Inherited);
        for (child, line) in children.iter().zip(lines) {
          if let Ok((mut text, mut colour)) = texts.get_mut(child) {
            if text.0 != line {
              text.0 = line
            }
            if child == children[0] {
              colour.0 = found.branch.colour()
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

fn obey(
  mut pressed: MessageReader<Pressed>,
  view: Res<View>,
  pilot: Option<Res<Pilot>>,
  skills: Query<&Skills>,
  mut lessons: MessageWriter<Learn>,
  mut adoptions: MessageWriter<crate::stance::Adopt>
) {
  let skills =
    pilot.and_then(|pilot| skills.get(pilot.me).ok().cloned()).unwrap_or_default();
  for &Pressed(act) in pressed.read() {
    match act {
      Act::Learn(node) if !view.dragged && standing(&skills, node) == (false, true) => {
        lessons.write(Learn(node));
      }
      Act::Stance(stance) if stance != skills.stance => {
        adoptions.write(crate::stance::Adopt(stance));
      }
      _ => ()
    }
  }
}

fn postures(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  skills: Query<&Skills>,
  rows: Query<Entity, With<Stances>>,
  mut buttons: Query<(&Act, &mut BorderColor)>,
  mut commands: Commands,
  mut shown: Local<Option<(Vec<Stance>, Option<Stance>)>>
) {
  if menu.showing(Tab::Skills) {
    let skills =
      pilot.and_then(|pilot| skills.get(pilot.me).ok().cloned()).unwrap_or_default();
    let state = (skills.stances(), skills.stance);
    if shown.as_ref() != Some(&state) {
      let (known, active) = &state;
      for row in rows.iter() {
        commands.entity(row).despawn_children().with_children(|row| {
          row.spawn(words("Stance (G):", 16.0, FAINT));
          match known.is_empty() {
            true => {
              row.spawn(words(
                "learn one in the Brawler, Warden or Arcanist branches; only one can be \
                 active at a time",
                14.0,
                FAINT
              ));
            }
            false => {
              crate::menu::button(row, "None", Act::Stance(None));
              for &stance in known.iter() {
                crate::menu::button(row, stance.short(), Act::Stance(Some(stance)))
              }
              row.spawn(words(active.map_or("", Stance::describe), 14.0, FAINT));
            }
          }
        });
      }
      *shown = Some(state.clone())
    }
    for (act, mut border) in buttons.iter_mut() {
      if let &Act::Stance(stance) = act {
        border.set_if_neq(BorderColor::all(match stance == skills.stance {
          true => LIT,
          false => EDGE
        }));
      }
    }
  }
}

pub struct Skilling;

impl Plugin for Skilling {
  fn build(&self, app: &mut App) {
    app
      .add_message::<Earned>()
      .init_resource::<View>()
      .add_systems(PreUpdate, study.after(ServerSystems::Receive).run_if(authority))
      .add_systems(Update, (earn, temper).chain().run_if(authority))
      .add_systems(PreStartup, prepare.run_if(plays))
      .add_systems(Startup, tooltip.run_if(plays))
      .add_systems(
        Update,
        (plant, steer, arrange, paint, explain, obey, postures).chain().run_if(plays)
      );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_tree_branches_far_without_crowding() {
    let tree = &*TREE;
    assert!(tree.len() > 100, "{} nodes", tree.len());
    for (index, node) in tree.iter().enumerate() {
      for other in tree.iter().skip(index + 1) {
        assert!(node.at.distance(other.at) > 0.75, "{:?} and {:?} overlap", node, other)
      }
    }
    let reach = tree.iter().map(|node| node.at.length()).fold(0.0, f32::max);
    assert!(reach > 15.0, "{reach}")
  }

  fn fingerprint(count: usize) -> u64 {
    TREE.iter().take(count).fold(0xCBF29CE484222325, |sum, talent| {
      format!("{:?}{:?}{:?}", talent.boon, talent.parent, talent.branch)
        .bytes()
        .fold(sum, |sum, byte| (sum ^ u64::from(byte)).wrapping_mul(0x100000001B3))
    })
  }

  #[test]
  fn learned_talents_keep_their_ids() {
    assert_eq!(
      fingerprint(137),
      16156405072829733998,
      "talents saved by players were renumbered: append new ones"
    )
  }

  #[test]
  #[ignore]
  fn tree_map() {
    let (size, scale) = (900i32, 20.0);
    let mut image = vec![[16u8, 18, 24]; (size * size) as usize];
    for node in TREE.iter() {
      let colour = node.branch.colour().to_srgba();
      let centre = (node.at * scale).as_ivec2() + size / 2;
      for dy in -4..=4 {
        for dx in -4..=4 {
          let at = centre + IVec2::new(dx, dy);
          if (0..size).contains(&at.x)
            && (0..size).contains(&at.y)
            && dx * dx + dy * dy <= 16
          {
            image[(at.y * size + at.x) as usize] = [colour.red, colour.green, colour.blue]
              .map(|channel| (channel * 255.0) as u8)
          }
        }
      }
    }
    let mut file = format!("P6 {size} {size} 255\n").into_bytes();
    file.extend(image.into_iter().flatten());
    std::fs::write("screenshots/tree.ppm", file).unwrap()
  }
}
