use {crate::{hud::SHADE, protocol::plays},
     bevy::{app::App,
            log::{BoxedLayer,
                  tracing_subscriber::{Layer, layer::Context}},
            prelude::*},
     std::{fmt::Write, sync::Mutex},
     tracing::{Event, Level, Subscriber,
               field::{Field, Visit}}};

const SHOWN_FOR: f32 = 5.0;
const SLIDE: f32 = 0.25;
const MOST: usize = 8;
const LONGEST: usize = 140;

static HEARD: Mutex<Vec<(Level, String)>> = Mutex::new(Vec::new());

struct Message(String);

impl Visit for Message {
  fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
    if field.name() == "message" {
      let _ = write!(self.0, "{value:?}");
    }
  }

  fn record_str(&mut self, field: &Field, value: &str) {
    if field.name() == "message" {
      self.0.push_str(value)
    }
  }
}

struct Tap;

impl<S: Subscriber> Layer<S> for Tap {
  fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
    let level = *event.metadata().level();
    if level <= Level::INFO {
      let mut message = Message(String::new());
      event.record(&mut message);
      let line: String = message.0.chars().take(LONGEST).collect();
      if let Ok(mut heard) = HEARD.lock() {
        heard.push((level, line))
      }
    }
  }
}

pub fn tap(_: &mut App) -> Option<BoxedLayer> { Some(Box::new(Tap)) }

#[derive(Component)]
struct Board;

#[derive(Component)]
struct Line {
  age: f32
}

fn board(mut commands: Commands) {
  commands.spawn((Board, Node {
    position_type: PositionType::Absolute,
    left: px(10),
    top: px(28),
    max_width: percent(45),
    flex_direction: FlexDirection::Column,
    row_gap: px(1),
    ..default()
  }));
}

fn shade(level: Level) -> Color {
  match level {
    Level::ERROR => Color::srgb(1.0, 0.35, 0.3),
    Level::WARN => Color::srgb(1.0, 0.85, 0.3),
    _ => Color::srgb(0.85, 0.88, 0.92)
  }
}

fn post(
  boards: Query<Entity, With<Board>>,
  lines: Query<(Entity, &Line)>,
  mut commands: Commands
) {
  let heard: Vec<(Level, String)> =
    HEARD.lock().map(|mut heard| heard.drain(..).collect()).unwrap_or_default();
  if let Ok(board) = boards.single() {
    let surplus = (lines.iter().count() + heard.len()).saturating_sub(MOST);
    let mut oldest: Vec<(Entity, &Line)> = lines.iter().collect();
    oldest.sort_by(|a, b| b.1.age.total_cmp(&a.1.age));
    oldest
      .into_iter()
      .take(surplus)
      .for_each(|(line, _)| commands.entity(line).despawn());
    heard.into_iter().rev().take(MOST).rev().for_each(|(level, text)| {
      commands.entity(board).with_child((
        Line { age: 0.0 },
        Text::new(text),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(shade(level)),
        SHADE,
        Node { left: px(-40), ..default() }
      ));
    })
  }
}

fn fade(
  time: Res<Time<Real>>,
  mut lines: Query<(Entity, &mut Line, &mut Node, &mut TextColor)>,
  mut commands: Commands
) {
  let dt = time.delta_secs();
  lines.iter_mut().for_each(|(entity, mut line, mut node, mut color)| {
    line.age += dt;
    let arriving = (line.age / SLIDE).min(1.0);
    let leaving = ((SHOWN_FOR - line.age) / SLIDE).clamp(0.0, 1.0);
    node.left = px(-40.0 * (1.0 - arriving).powi(2));
    color.0.set_alpha(arriving.min(leaving));
    if line.age > SHOWN_FOR {
      commands.entity(entity).despawn()
    }
  })
}

pub struct Console;

impl Plugin for Console {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, board.run_if(plays))
      .add_systems(Update, (post, fade).chain().run_if(plays));
  }
}
