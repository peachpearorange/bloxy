use {crate::{block::{Block, Fluid},
             folk::{Vigour, harm},
             glyph::Glyph,
             hud::SHADE,
             player::Pilot,
             protocol::{Avatar, Health, Notice, authority, plays},
             skill::{Glyphs, glyph_image}},
     bevy::prelude::*,
     serde::{Deserialize, Serialize}};

const SHARE_EVERY: f32 = 0.5;
const POISON_EVERY: f32 = 2.0;
const BURN_EVERY: f32 = 1.0;
const MEND_EVERY: f32 = 1.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Hash)]
pub enum Kind {
  Hasted,
  Stoneskin,
  Leaping,
  Lit,
  Regenerating,
  Poisoned,
  Burning,
  Chilled,
  Weakened,
  Frightened,
  Invisible
}

impl Kind {
  pub fn boon(self) -> bool {
    matches!(
      self,
      Kind::Hasted
        | Kind::Stoneskin
        | Kind::Leaping
        | Kind::Lit
        | Kind::Regenerating
        | Kind::Invisible
    )
  }

  pub fn glyph(self) -> Glyph {
    match self {
      Kind::Hasted => Glyph::Hourglass,
      Kind::Stoneskin => Glyph::Stone,
      Kind::Leaping => Glyph::Feather,
      Kind::Lit => Glyph::Sun,
      Kind::Regenerating => Glyph::Mend,
      Kind::Poisoned => Glyph::Drop,
      Kind::Burning => Glyph::Flame,
      Kind::Chilled => Glyph::Snowflake,
      Kind::Weakened => Glyph::Broken,
      Kind::Frightened => Glyph::Eye,
      Kind::Invisible => Glyph::Cloak
    }
  }

  pub fn colour(self) -> [f32; 3] {
    match self {
      Kind::Hasted => [0.98, 0.82, 0.25],
      Kind::Stoneskin => [0.66, 0.64, 0.58],
      Kind::Leaping => [0.75, 0.9, 0.98],
      Kind::Lit => [1.0, 0.9, 0.55],
      Kind::Regenerating => [0.4, 0.92, 0.45],
      Kind::Poisoned => [0.55, 0.85, 0.2],
      Kind::Burning => [0.98, 0.45, 0.12],
      Kind::Chilled => [0.5, 0.78, 1.0],
      Kind::Weakened => [0.85, 0.3, 0.28],
      Kind::Frightened => [0.7, 0.45, 0.95],
      Kind::Invisible => [0.78, 0.68, 0.95]
    }
  }

  pub fn name(self) -> &'static str {
    match self {
      Kind::Hasted => "Hasted",
      Kind::Stoneskin => "Stoneskin",
      Kind::Leaping => "Jump",
      Kind::Lit => "Dancing Light",
      Kind::Regenerating => "Regenerating",
      Kind::Poisoned => "Poisoned",
      Kind::Burning => "Burning",
      Kind::Chilled => "Chilled",
      Kind::Weakened => "Weakened",
      Kind::Frightened => "Frightened",
      Kind::Invisible => "Invisible"
    }
  }

  pub fn describe(self) -> &'static str {
    match self {
      Kind::Hasted => "Moving 40% faster",
      Kind::Stoneskin => "Taking half damage",
      Kind::Leaping => "Leaping higher and falling gently",
      Kind::Lit => "A floating light follows you",
      Kind::Regenerating => "Healing over time",
      Kind::Poisoned => "Losing health over time",
      Kind::Burning => "On fire: losing health quickly",
      Kind::Chilled => "Slowed to 60% speed",
      Kind::Weakened => "Punches hit half as hard",
      Kind::Frightened => "Too shaken to sprint",
      Kind::Invisible => "Unseen by foes until you attack"
    }
  }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Condition {
  pub kind: Kind,
  pub left: f32,
  pub total: f32
}

#[derive(Component, Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Conditions(pub Vec<Condition>);

impl Conditions {
  pub fn has(&self, kind: Kind) -> bool {
    self.0.iter().any(|condition| condition.kind == kind)
  }

  pub fn afflict(&mut self, kind: Kind, secs: f32) {
    match self.0.iter_mut().find(|condition| condition.kind == kind) {
      Some(condition) => {
        condition.left = condition.left.max(secs);
        condition.total = condition.total.max(secs)
      }
      None => self.0.push(Condition { kind, left: secs, total: secs })
    }
  }

  pub fn pace(&self) -> f32 {
    match (self.has(Kind::Hasted), self.has(Kind::Chilled)) {
      (true, true) => 0.85,
      (true, false) => 1.4,
      (false, true) => 0.6,
      _ => 1.0
    }
  }
}

#[derive(Component, Default)]
pub struct Ticking {
  shared: f32,
  poison: f32,
  burn: f32,
  mend: f32
}

fn tick(
  time: Res<Time>,
  mut afflicted: Query<(
    Entity,
    &mut Conditions,
    Option<&mut Ticking>,
    Option<&mut Health>,
    Option<&mut Vigour>
  )>,
  mut commands: Commands
) {
  let dt = time.delta_secs();
  for (entity, mut conditions, ticking, health, vigour) in afflicted.iter_mut() {
    match ticking {
      None => {
        commands.entity(entity).insert(Ticking::default());
      }
      Some(mut ticking) => {
        let before = conditions.0.len();
        let [fire, frost, venom] =
          vigour.as_deref().map_or([1.0; 3], |vigour| vigour.resist);
        for condition in conditions.bypass_change_detection().0.iter_mut() {
          condition.left -= match condition.kind {
            Kind::Chilled => dt / frost.max(0.25),
            _ => dt
          }
        }
        conditions.bypass_change_detection().0.retain(|condition| condition.left > 0.0);
        ticking.shared += dt;
        if conditions.0.len() != before || ticking.shared >= SHARE_EVERY {
          ticking.shared = 0.0;
          conditions.set_changed()
        }
        let mut balm = 0u8;
        let every = |active: bool, clock: &mut f32, period: f32| {
          *clock = match active {
            true => *clock + dt,
            false => 0.0
          };
          let due = *clock >= period;
          if due {
            *clock -= period
          }
          due
        };
        let poisoned =
          every(conditions.has(Kind::Poisoned), &mut ticking.poison, POISON_EVERY);
        let burnt = every(conditions.has(Kind::Burning), &mut ticking.burn, BURN_EVERY);
        let pain = u8::from(poisoned) + u8::from(burnt);
        let felt =
          f32::from(u8::from(poisoned)) * venom + f32::from(u8::from(burnt)) * fire;
        if every(conditions.has(Kind::Regenerating), &mut ticking.mend, MEND_EVERY) {
          balm += 1
        }
        if let Some(mut health) = health
          && health.0 > 0
          && (pain > 0 || balm > 0)
        {
          match vigour {
            Some(mut vigour) => {
              if pain > 0 {
                harm(&mut health, &mut vigour, felt)
              }
              health.0 = (health.0 + balm).min(vigour.most)
            }
            None => {
              health.0 = health.0.saturating_sub(pain);
              if health.0 == 0 {
                crate::haunt::fell(&mut commands, entity)
              }
            }
          }
        }
      }
    }
  }
}

const SCORCHED_FOR: f32 = 4.0;

fn scorch(
  voxels: Res<crate::voxels::Voxels>,
  mut players: Query<(&Avatar, &mut Conditions), With<crate::authority::Controller>>
) {
  for (avatar, mut conditions) in players.iter_mut() {
    let lava = [0.1, 1.0].into_iter().any(|above| {
      voxels
        .block((avatar.at + Vec3::Y * above).floor().as_ivec3())
        .and_then(Block::liquid)
        .is_some_and(|(fluid, _)| fluid == Fluid::Lava)
    });
    let burning = conditions.0.iter().any(|condition| {
      condition.kind == Kind::Burning && condition.left > SCORCHED_FOR - 1.0
    });
    if lava && !burning {
      conditions.afflict(Kind::Burning, SCORCHED_FOR)
    }
  }
}

#[derive(Component)]
struct Strip;

#[derive(Component)]
struct Badge(usize);

#[derive(Component)]
struct BadgeGlyph(usize);

#[derive(Component)]
struct BadgeTime(usize);

const BADGES: usize = 10;

fn strip(mut commands: Commands, glyphs: Res<Glyphs>) {
  commands
    .spawn((Strip, Node {
      position_type: PositionType::Absolute,
      top: px(8),
      width: percent(100),
      justify_content: JustifyContent::Center,
      column_gap: px(6),
      ..default()
    }))
    .with_children(|row| {
      for index in 0..BADGES {
        row
          .spawn((Badge(index), Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            display: Display::None,
            ..default()
          }))
          .with_children(|badge| {
            badge
              .spawn((
                Node {
                  width: px(36),
                  height: px(36),
                  border: UiRect::all(px(2)),
                  border_radius: BorderRadius::all(px(6)),
                  justify_content: JustifyContent::Center,
                  align_items: AlignItems::Center,
                  ..default()
                },
                BorderColor::all(Color::WHITE),
                BackgroundColor(Color::srgba(0.03, 0.03, 0.05, 0.75))
              ))
              .with_child((
                BadgeGlyph(index),
                glyph_image(&glyphs, Glyph::Star, Color::WHITE, 24.0)
              ));
            badge.spawn((
              BadgeTime(index),
              Text::new(""),
              TextFont { font_size: FontSize::Px(13.0), ..default() },
              SHADE
            ));
          });
      }
    });
}

struct Shown {
  glyph: Glyph,
  colour: [f32; 3],
  edge: Color,
  line: String,
  fading: bool
}

fn show(
  time: Res<Time>,
  pilot: Option<Res<Pilot>>,
  mine: Query<Ref<Conditions>>,
  skills: Query<&crate::skill::Skills>,
  mut badges: Query<(&Badge, &mut Node, &Children)>,
  mut borders: Query<&mut BorderColor>,
  mut glyphs: Query<(&BadgeGlyph, &mut ImageNode)>,
  mut times: Query<(&BadgeTime, &mut Text, &mut TextColor)>,
  mut notices: MessageWriter<Notice>,
  mut since: Local<f32>,
  mut known: Local<Vec<Kind>>
) {
  let stance = pilot
    .as_ref()
    .and_then(|pilot| skills.get(pilot.me).ok())
    .and_then(|skills| skills.stance);
  let conditions = pilot.and_then(|pilot| mine.get(pilot.me).ok());
  *since = match conditions.as_ref().is_some_and(|conditions| conditions.is_changed()) {
    true => 0.0,
    false => *since + time.delta_secs()
  };
  let mut listed: Vec<Condition> =
    conditions.map(|conditions| conditions.0.clone()).unwrap_or_default();
  listed.sort_by_key(|condition| (!condition.kind.boon(), condition.kind as u8));
  for condition in listed.iter().filter(|condition| !known.contains(&condition.kind)) {
    notices.write(Notice(format!(
      "{}: {}",
      condition.kind.name(),
      condition.kind.describe()
    )));
  }
  *known = listed.iter().map(|condition| condition.kind).collect();
  let stance = stance.map(|stance| Shown {
    glyph: stance.glyph(),
    colour: stance.colour(),
    edge: Color::srgb(0.95, 0.85, 0.45),
    line: "stance".into(),
    fading: false
  });
  let shown: Vec<Shown> = stance
    .into_iter()
    .chain(listed.iter().map(|condition| {
      let left = (condition.left - *since).max(0.0);
      Shown {
        glyph: condition.kind.glyph(),
        colour: condition.kind.colour(),
        edge: match condition.kind.boon() {
          true => Color::srgb(0.45, 0.9, 0.45),
          false => Color::srgb(0.95, 0.35, 0.3)
        },
        line: match left >= 60.0 {
          true => format!("{}m", (left / 60.0).ceil() as u32),
          false => format!("{}s", left.ceil() as u32)
        },
        fading: left < 5.0 && condition.kind.boon()
      }
    }))
    .collect();
  for (badge, mut node, children) in badges.iter_mut() {
    let found = shown.get(badge.0);
    let display = if found.is_some() { Display::Flex } else { Display::None };
    if node.display != display {
      node.display = display
    }
    if let Some(found) = found {
      let [r, g, b] = found.colour;
      for child in children.iter() {
        if let Ok(mut border) = borders.get_mut(child) {
          border.set_if_neq(BorderColor::all(found.edge));
        }
      }
      for (_, mut image) in glyphs.iter_mut().filter(|(glyph, _)| glyph.0 == badge.0) {
        let rect = Some(crate::glyph::cell(found.glyph.index()));
        if image.rect != rect || image.color != Color::srgb(r, g, b) {
          image.rect = rect;
          image.color = Color::srgb(r, g, b)
        }
      }
      for (_, mut text, mut colour) in
        times.iter_mut().filter(|(time, ..)| time.0 == badge.0)
      {
        if text.0 != found.line {
          text.0 = found.line.clone()
        }
        colour.set_if_neq(TextColor(match found.fading {
          true => Color::srgb(1.0, 0.75, 0.4),
          false => Color::WHITE
        }));
      }
    }
  }
}

pub struct Afflictions;

impl Plugin for Afflictions {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Update, tick.run_if(authority))
      .add_systems(
        Update,
        scorch.run_if(authority).run_if(resource_exists::<crate::voxels::Voxels>)
      )
      .add_systems(Startup, strip.run_if(plays))
      .add_systems(Update, show.run_if(plays));
  }
}
