use {crate::{hud::SHADE,
             menu::{INK, Menu},
             player::Pilot,
             protocol::{LONGEST_SAYING, Said, Say, hue, plays}},
     bevy::{input::{ButtonState,
                    keyboard::{Key, KeyboardInput}},
            prelude::*}};

const SHOWN_FOR: f32 = 12.0;
const KEPT: usize = 30;
const RECENT: usize = 8;
const SIZE: f32 = 16.0;
const WIDE: f32 = 560.0;
const BACKING: Color = Color::srgba(0.0, 0.0, 0.0, 0.4);

#[derive(Resource, Default)]
struct Draft(String);

#[derive(Component)]
struct Log;

#[derive(Component)]
struct Line {
  heard: f32
}

#[derive(Component)]
struct Typing;

fn build(mut commands: Commands) {
  commands.spawn((Log, Node {
    position_type: PositionType::Absolute,
    left: px(10),
    bottom: px(150),
    width: px(WIDE),
    flex_direction: FlexDirection::Column,
    align_items: AlignItems::FlexStart,
    row_gap: px(1),
    ..default()
  }));
  commands
    .spawn((
      Typing,
      Node {
        position_type: PositionType::Absolute,
        left: px(10),
        bottom: px(122),
        width: px(WIDE),
        padding: UiRect::axes(px(6), px(3)),
        ..default()
      },
      BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
      Visibility::Hidden
    ))
    .with_child((
      Text::new(""),
      TextFont { font_size: FontSize::Px(SIZE), ..default() },
      TextColor(INK),
      SHADE
    ));
}

fn span(text: impl Into<String>, color: Color) -> impl Bundle {
  (
    TextSpan::new(text),
    TextFont { font_size: FontSize::Px(SIZE), ..default() },
    TextColor(color)
  )
}

fn hear(
  time: Res<Time>,
  mut said: MessageReader<Said>,
  logs: Query<(Entity, Option<&Children>), With<Log>>,
  mut commands: Commands
) {
  for Said { name, hue: index, text } in said.read() {
    for (log, lines) in logs.iter() {
      for &line in lines.into_iter().flatten().rev().skip(KEPT - 1) {
        commands.entity(line).despawn()
      }
      commands.entity(log).with_children(|log| {
        log
          .spawn((
            Line { heard: time.elapsed_secs() },
            Node {
              padding: UiRect::axes(px(4), px(1)),
              max_width: px(WIDE),
              ..default()
            },
            BackgroundColor(BACKING)
          ))
          .with_children(|line| {
            line
              .spawn((
                Text::default(),
                TextFont { font_size: FontSize::Px(SIZE), ..default() },
                SHADE
              ))
              .with_children(|spans| {
                spans.spawn(span(format!("{name}: "), hue(*index)));
                spans.spawn(span(text.clone(), INK));
              });
          });
      });
    }
  }
}

fn talk(
  mut typed: MessageReader<KeyboardInput>,
  keys: Res<ButtonInput<KeyCode>>,
  pilot: Option<Res<Pilot>>,
  mut menu: ResMut<Menu>,
  mut draft: ResMut<Draft>,
  mut says: MessageWriter<Say>
) {
  match menu.chatting {
    false => {
      typed.clear();
      if !menu.open
        && pilot.is_some()
        && (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::KeyT))
      {
        menu.chatting = true;
        draft.0.clear()
      }
    }
    true => {
      for key in typed.read().filter(|key| key.state == ButtonState::Pressed) {
        match &key.logical_key {
          Key::Backspace => {
            draft.0.pop();
          }
          Key::Enter => {
            if !draft.0.trim().is_empty() {
              says.write(Say(draft.0.clone()));
            }
            menu.chatting = false
          }
          Key::Escape => menu.chatting = false,
          _ => {
            if let Some(chars) = &key.text
              && draft.0.chars().count() < LONGEST_SAYING
            {
              draft.0.extend(chars.chars().filter(|c| !c.is_control()))
            }
          }
        }
      }
    }
  }
}

fn show(
  time: Res<Time>,
  menu: Res<Menu>,
  draft: Res<Draft>,
  logs: Query<&Children, With<Log>>,
  mut lines: Query<(&Line, &mut Visibility), Without<Typing>>,
  mut typing: Query<(&mut Visibility, &Children), With<Typing>>,
  mut texts: Query<&mut Text>
) {
  let now = time.elapsed_secs();
  for children in logs.iter() {
    let count = children.len();
    for (index, child) in children.iter().enumerate() {
      if let Ok((line, mut visibility)) = lines.get_mut(child) {
        let shown = !menu.open
          && (menu.chatting || (count - index <= RECENT && now - line.heard < SHOWN_FOR));
        visibility.set_if_neq(if shown {
          Visibility::Inherited
        } else {
          Visibility::Hidden
        });
      }
    }
  }
  let caret = if (now * 2.0) as u32 % 2 == 0 { "|" } else { "" };
  for (mut visibility, children) in typing.iter_mut() {
    visibility.set_if_neq(match menu.chatting {
      true => Visibility::Inherited,
      false => Visibility::Hidden
    });
    for child in children.iter() {
      if let Ok(mut text) = texts.get_mut(child) {
        let line = format!("> {}{caret}", draft.0);
        if text.0 != line {
          text.0 = line
        }
      }
    }
  }
}

pub struct Chat;

impl Plugin for Chat {
  fn build(&self, app: &mut App) {
    app.init_resource::<Draft>().add_systems(Startup, build.run_if(plays)).add_systems(
      Update,
      (talk.after(crate::menu::toggle), hear, show).chain().run_if(plays)
    );
  }
}
