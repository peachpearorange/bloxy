use strum::VariantArray;
use {crate::{authority::{Controller, player_of},
             condition::{Conditions, Kind},
             death::Dying,
             folk::Raider,
             glyph::Glyph,
             haunt::Haunt,
             menu::{Menu, closed},
             player::{Pilot, Poke},
             protocol::*,
             skill::Skills,
             spell::{Flourish, Spellbook, harm, targets}},
     bevy::prelude::*,
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

pub const STRIKE_EVERY: f32 = 0.35;
pub const LUNGE_EVERY: f32 = 3.5;
const LUNGE_SPEED: f32 = 15.0;
const LUNGE_GLIDE: f32 = 0.3;
const LUNGE_REACH: f32 = 5.5;
const LUNGE_WIDTH: f32 = 1.4;

#[derive(
  Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Hash, VariantArray,
)]
pub enum Stance {
  Berserker,
  Defender,
  Duelist,
  Mystic
}

impl Stance {
  pub const ALL: [Stance; Stance::VARIANTS.len()] =
    *Stance::VARIANTS.first_chunk().unwrap();

  pub fn name(self) -> &'static str {
    match self {
      Stance::Berserker => "Berserker Stance",
      Stance::Defender => "Defender Stance",
      Stance::Duelist => "Duelist Stance",
      Stance::Mystic => "Mystic Stance"
    }
  }

  pub fn short(self) -> &'static str {
    match self {
      Stance::Berserker => "Berserker",
      Stance::Defender => "Defender",
      Stance::Duelist => "Duelist",
      Stance::Mystic => "Mystic"
    }
  }

  pub fn describe(self) -> &'static str {
    match self {
      Stance::Berserker => "Punch 50% harder but take 25% more damage",
      Stance::Defender => "Take 35% less damage but move 15% slower",
      Stance::Duelist => "Strike almost twice as often and move 10% faster",
      Stance::Mystic => {
        "Spells recharge 25% faster and cost nothing a fifth of the time, but punches weaken"
      }
    }
  }

  pub fn glyph(self) -> Glyph {
    match self {
      Stance::Berserker => Glyph::Axe,
      Stance::Defender => Glyph::Shield,
      Stance::Duelist => Glyph::Sword,
      Stance::Mystic => Glyph::Rune
    }
  }

  pub fn colour(self) -> [f32; 3] {
    match self {
      Stance::Berserker => [0.95, 0.3, 0.25],
      Stance::Defender => [0.55, 0.75, 0.95],
      Stance::Duelist => [0.95, 0.85, 0.4],
      Stance::Mystic => [0.72, 0.5, 1.0]
    }
  }

  pub fn punch(self, base: u8) -> u8 {
    match self {
      Stance::Berserker => (base * 3).div_ceil(2),
      Stance::Mystic => base.saturating_sub(1).max(1),
      _ => base
    }
  }

  pub fn ward(self) -> f32 {
    match self {
      Stance::Berserker => 1.25,
      Stance::Defender => 0.65,
      _ => 1.0
    }
  }

  pub fn pace(self) -> f32 {
    match self {
      Stance::Defender => 0.85,
      Stance::Duelist => 1.1,
      _ => 1.0
    }
  }

  pub fn quick(self) -> f32 {
    match self {
      Stance::Duelist => 0.2,
      _ => STRIKE_EVERY
    }
  }

  pub fn thrift(self) -> f32 {
    match self {
      Stance::Mystic => 0.2,
      _ => 0.0
    }
  }

  pub fn recharge(self) -> f32 {
    match self {
      Stance::Mystic => 0.75,
      _ => 1.0
    }
  }
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Adopt(pub Option<Stance>);

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Lunge(pub Vec3);

fn adopt(
  mut adoptions: MessageReader<FromClient<Adopt>>,
  mut players: Query<(&Controller, &mut Skills)>,
  mut notices: MessageWriter<ToClients<Notice>>
) {
  for &FromClient { client_id, message: Adopt(stance) } in adoptions.read() {
    if let Some(mut skills) = player_of(players.iter_mut(), client_id)
      && stance.is_none_or(|stance| skills.stances().contains(&stance))
      && skills.stance != stance
    {
      skills.stance = stance;
      notices.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Notice(stance.map_or("No stance".into(), |stance| {
          format!("{}: {}", stance.name(), stance.describe())
        }))
      });
    }
  }
}

fn lunge(
  mut lunges: MessageReader<FromClient<Lunge>>,
  time: Res<Time>,
  mut players: Query<(
    &Controller,
    (Entity, &Avatar, &Skills, &mut Spellbook, &mut Conditions)
  )>,
  folk: Query<(Entity, &Folk), Without<Dying>>,
  beasts: Query<(Entity, &Beast), Without<Dying>>,
  birds: Query<(Entity, &crate::bird::Bird), Without<Dying>>,
  mut creatures: Query<
    (&mut Health, Option<&mut Haunt>, Option<&mut Raider>),
    Without<Controller>
  >,
  mut flourishes: MessageWriter<ToClients<Flourish>>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  for &FromClient { client_id, message: Lunge(toward) } in lunges.read() {
    if let Some((player, avatar, skills, mut book, mut conditions)) =
      player_of(players.iter_mut(), client_id)
      && skills.lunges()
      && book.lunge <= now
      && toward.is_finite()
    {
      book.lunge = now + LUNGE_EVERY * 0.9;
      conditions.0.retain(|condition| condition.kind != Kind::Invisible);
      let ahead = toward.with_y(0.0).normalize_or(Vec3::NEG_Z);
      let damage = 4 + skills.momentum() * 2;
      let mut struck: Vec<(Entity, f32, Vec3)> = targets(&folk, &beasts, &birds)
        .into_iter()
        .filter_map(|target| {
          let away = target.centre - avatar.at;
          let along = away.dot(ahead);
          let aside = (away - ahead * along).with_y(0.0).length();
          ((0.0..LUNGE_REACH).contains(&along)
            && aside < LUNGE_WIDTH
            && away.y.abs() < 2.6)
            .then_some((target.entity, along, target.centre))
        })
        .collect();
      struck.sort_by(|a, b| a.1.total_cmp(&b.1));
      for &(entity, _, centre) in struck.iter().take(3) {
        if let Ok((mut health, haunt, raider)) = creatures.get_mut(entity) {
          harm(&mut commands, entity, &mut health, damage, player);
          let push = ahead * 8.0 + Vec3::Y * 4.5;
          if let Some(mut haunt) = haunt {
            haunt.shove(push)
          }
          if let Some(mut raider) = raider {
            raider.knock(push)
          }
          flourishes.write(ToClients {
            targets: SendTargets::All,
            message: Flourish {
              spell: crate::block::Block::SpellThunder,
              at: centre,
              reach: 0.6
            }
          });
        }
      }
    }
  }
}

fn cycle(
  keys: Res<ButtonInput<KeyCode>>,
  pilot: Res<Pilot>,
  skills: Query<&Skills>,
  mut adoptions: MessageWriter<Adopt>
) {
  if keys.just_pressed(KeyCode::KeyG)
    && let Ok(skills) = skills.get(pilot.me)
  {
    let choices: Vec<Option<Stance>> =
      std::iter::once(None).chain(skills.stances().into_iter().map(Some)).collect();
    let next = choices
      .iter()
      .position(|&choice| choice == skills.stance)
      .and_then(|index| choices[(index + 1) % choices.len()]);
    if choices.len() > 1 {
      adoptions.write(Adopt(next));
    }
  }
}

fn dash(
  time: Res<Time>,
  keys: Res<ButtonInput<KeyCode>>,
  menu: Res<Menu>,
  mut pilot: ResMut<Pilot>,
  skills: Query<&Skills>,
  mut lunges: MessageWriter<Lunge>,
  mut pokes: MessageWriter<Poke>,
  mut ready: Local<f32>
) {
  let now = time.elapsed_secs();
  if menu.idle()
    && keys.just_pressed(KeyCode::KeyF)
    && now >= *ready
    && pilot.riding.is_none()
    && skills.get(pilot.me).is_ok_and(Skills::lunges)
  {
    *ready = now + LUNGE_EVERY;
    let ahead = (pilot.facing() * Vec3::NEG_Z).with_y(0.0).normalize_or(Vec3::NEG_Z);
    pilot.velocity = ahead * LUNGE_SPEED + Vec3::Y * pilot.velocity.y.max(2.5);
    pilot.grounded = false;
    pilot.staggered = LUNGE_GLIDE;
    lunges.write(Lunge(ahead));
    pokes.write(Poke);
  }
}

pub struct Stances;

impl Plugin for Stances {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        PreUpdate,
        (adopt, lunge).after(ServerSystems::Receive).run_if(authority)
      )
      .add_systems(
        Update,
        (cycle.run_if(closed), dash).run_if(plays).run_if(resource_exists::<Pilot>)
      );
  }
}
