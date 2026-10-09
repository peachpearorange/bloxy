use {crate::{editor::{self, Draft},
             identity::{Credentials, Identity, Standing},
             local,
             opts::opts,
             protocol::plays,
             settings::{Knob, Settings}},
     bevy::{input::{ButtonState,
                    keyboard::{Key, KeyboardInput}},
            prelude::*,
            window::{CursorGrabMode, CursorOptions, PrimaryWindow}}};

pub const INK: Color = Color::srgb(0.93, 0.95, 0.93);
pub const FAINT: Color = Color::srgb(0.62, 0.66, 0.66);
pub const BUTTON: Color = Color::srgb(0.17, 0.19, 0.22);
const HOVERED: Color = Color::srgb(0.25, 0.28, 0.32);
const PANEL: Color = Color::srgba(0.07, 0.08, 0.1, 0.94);
const EDGE: Color = Color::srgb(0.3, 0.33, 0.36);
const LIT: Color = Color::srgb(0.95, 0.85, 0.45);
const ESCAPE_GRACE: f32 = 0.3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
  Settings,
  Profile,
  Skin
}

impl Tab {
  const ALL: [Tab; 3] = [Tab::Settings, Tab::Profile, Tab::Skin];

  fn label(self) -> &'static str {
    match self {
      Tab::Settings => "Settings",
      Tab::Profile => "Profile",
      Tab::Skin => "Skin"
    }
  }
}

#[derive(Resource)]
pub struct Menu {
  pub open: bool,
  pub tab: Tab,
  pub since: f32
}

impl Menu {
  pub fn show(&mut self, tab: Tab, now: f32) {
    self.open = true;
    self.tab = tab;
    self.since = now
  }

  pub fn showing(&self, tab: Tab) -> bool { self.open && self.tab == tab }
}

pub fn closed(menu: Res<Menu>) -> bool { !menu.open }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
  Name,
  Password
}

impl Entry {
  fn longest(self) -> usize {
    match self {
      Entry::Name => crate::account::LONGEST_NAME,
      Entry::Password => 64
    }
  }

  fn of(self, credentials: &mut Credentials) -> &mut String {
    match self {
      Entry::Name => &mut credentials.name,
      Entry::Password => &mut credentials.password
    }
  }
}

#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub enum Act {
  Resume,
  Open(Tab),
  Turn(Knob, i32),
  Focus(Entry),
  Invent,
  Copy,
  Submit,
  Swatch(u8),
  Randomize,
  Revert,
  Wear
}

#[derive(Message, Clone, Copy)]
pub struct Pressed(pub Act);

#[derive(Resource, Default)]
struct Focus(Option<Entry>);

#[derive(Resource, Default)]
struct Notice(Option<(String, f32)>);

#[derive(Component)]
pub struct Shaded;

#[derive(Component)]
struct Overlay;

#[derive(Component)]
struct Page(Tab);

#[derive(Component)]
struct Reading(Knob);

#[derive(Component)]
struct Typed(Entry);

#[derive(Component)]
struct Remark;

pub fn words(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
  (
    Text::new(text),
    TextFont { font_size: FontSize::Px(size), ..default() },
    TextColor(color)
  )
}

pub fn button(parent: &mut ChildSpawnerCommands, label: &str, act: Act) {
  parent
    .spawn((
      Button,
      act,
      Shaded,
      Node {
        padding: UiRect::axes(px(14), px(7)),
        border: UiRect::all(px(2)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
      },
      BorderColor::all(EDGE),
      BackgroundColor(BUTTON)
    ))
    .with_child(words(label, 16.0, INK));
}

fn settings_page(page: &mut ChildSpawnerCommands) {
  Knob::ALL.into_iter().for_each(|knob| {
    page
      .spawn(Node { align_items: AlignItems::Center, column_gap: px(10), ..default() })
      .with_children(|row| {
        row.spawn((words(knob.label(), 17.0, INK), Node { width: px(220), ..default() }));
        match knob {
          Knob::Invert => button(row, "Toggle", Act::Turn(knob, 1)),
          _ => button(row, "-", Act::Turn(knob, -1))
        }
        row.spawn((Reading(knob), words("", 17.0, LIT), Node {
          width: px(110),
          justify_content: JustifyContent::Center,
          ..default()
        }));
        if knob != Knob::Invert {
          button(row, "+", Act::Turn(knob, 1))
        }
      });
  });
  page.spawn(words(
    "Tab or Esc opens and closes this menu. Click the world to look around.",
    14.0,
    FAINT
  ));
}

fn field(parent: &mut ChildSpawnerCommands, entry: Entry) {
  parent
    .spawn((
      Button,
      Act::Focus(entry),
      Node {
        width: px(360),
        padding: UiRect::axes(px(10), px(7)),
        border: UiRect::all(px(2)),
        ..default()
      },
      BorderColor::all(EDGE),
      BackgroundColor(Color::srgb(0.04, 0.05, 0.06))
    ))
    .with_child((Typed(entry), words("", 17.0, INK)));
}

fn profile_page(page: &mut ChildSpawnerCommands) {
  page.spawn(words("Name", 15.0, FAINT));
  page.spawn(Node { column_gap: px(10), ..default() }).with_children(|row| {
    field(row, Entry::Name);
  });
  page.spawn(words("Password", 15.0, FAINT));
  page.spawn(Node { column_gap: px(10), ..default() }).with_children(|row| {
    field(row, Entry::Password);
    button(row, "New password", Act::Invent);
    if local::CAN_COPY {
      button(row, "Copy", Act::Copy)
    }
  });
  page.spawn(Node { column_gap: px(10), ..default() }).with_children(|row| {
    button(row, "Save", Act::Submit);
  });
  page.spawn((Remark, words("", 16.0, LIT)));
  page.spawn(words(
    "This browser remembers your name and password. Copy the password somewhere safe: \
     with it you can sign in as yourself on another device or after clearing your browser. \
     Typing the name and password of an existing player and saving signs in as them; a new \
     name renames you.",
    14.0,
    FAINT
  ));
}

fn build(mut commands: Commands, draft: Res<Draft>) {
  commands
    .spawn((
      Overlay,
      Node {
        width: percent(100),
        height: percent(100),
        position_type: PositionType::Absolute,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        display: Display::None,
        ..default()
      },
      BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
      GlobalZIndex(10)
    ))
    .with_children(|overlay| {
      overlay
        .spawn((
          Node {
            width: px(900),
            max_width: percent(96),
            min_height: px(560),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(20)),
            row_gap: px(14),
            border: UiRect::all(px(2)),
            ..default()
          },
          BorderColor::all(EDGE),
          BackgroundColor(PANEL)
        ))
        .with_children(|panel| {
          panel
            .spawn(Node {
              align_items: AlignItems::Center,
              column_gap: px(10),
              ..default()
            })
            .with_children(|header| {
              header.spawn((words("BLOXY", 26.0, INK), Node {
                margin: UiRect::right(px(18)),
                ..default()
              }));
              Tab::ALL
                .into_iter()
                .for_each(|tab| button(header, tab.label(), Act::Open(tab)));
              header.spawn(Node { flex_grow: 1.0, ..default() });
              button(header, "Resume", Act::Resume);
            });
          Tab::ALL.into_iter().for_each(|tab| {
            panel
              .spawn((Page(tab), Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                display: Display::None,
                ..default()
              }))
              .with_children(|page| match tab {
                Tab::Settings => settings_page(page),
                Tab::Profile => profile_page(page),
                Tab::Skin => editor::page(page, &draft)
              });
          })
        });
    });
}

fn toggle(
  keys: Res<ButtonInput<KeyCode>>,
  time: Res<Time>,
  mut menu: ResMut<Menu>,
  mut focus: ResMut<Focus>,
  mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
  mut locked: Local<bool>
) {
  let now = time.elapsed_secs();
  let lost = local::pointer_locked().is_some_and(|held| {
    let lost = *locked && !held;
    *locked = held;
    lost
  });
  let escape = keys.just_pressed(KeyCode::Escape) && now - menu.since > ESCAPE_GRACE;
  let was = menu.open;
  match (menu.open, keys.just_pressed(KeyCode::Tab) || escape, lost) {
    (false, true, _) | (false, _, true) => {
      let tab = menu.tab;
      menu.show(tab, now)
    }
    (true, true, _) => menu.open = false,
    _ => ()
  }
  if was != menu.open
    && let Ok(mut cursor) = cursor.single_mut()
  {
    focus.0 = None;
    let playing = !menu.open && opts().shot.is_none();
    cursor.grab_mode =
      if playing { CursorGrabMode::Locked } else { CursorGrabMode::None };
    cursor.visible = !playing
  }
}

fn click(
  mut buttons: Query<
    (&Interaction, &Act, Option<&Shaded>, &mut BackgroundColor),
    Changed<Interaction>
  >,
  mut pressed: MessageWriter<Pressed>
) {
  buttons.iter_mut().for_each(|(interaction, &act, shaded, mut background)| {
    if shaded.is_some() {
      background.0 = match interaction {
        Interaction::Hovered | Interaction::Pressed => HOVERED,
        Interaction::None => BUTTON
      }
    }
    if *interaction == Interaction::Pressed {
      pressed.write(Pressed(act));
    }
  })
}

fn obey(
  mut pressed: MessageReader<Pressed>,
  time: Res<Time>,
  mut menu: ResMut<Menu>,
  mut settings: ResMut<Settings>,
  mut identity: ResMut<Identity>,
  mut focus: ResMut<Focus>,
  mut notice: ResMut<Notice>,
  mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>
) {
  let now = time.elapsed_secs();
  pressed.read().for_each(|&Pressed(act)| match act {
    Act::Resume => {
      menu.open = false;
      focus.0 = None;
      if let Ok(mut cursor) = cursor.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false
      }
    }
    Act::Open(tab) => {
      menu.tab = tab;
      focus.0 = None
    }
    Act::Turn(knob, by) => settings.turn(knob, by),
    Act::Focus(entry) => focus.0 = Some(entry),
    Act::Invent => {
      identity.fields.password = Credentials::invented_password();
      notice.0 = Some(("New password made: press Save to keep it.".into(), now))
    }
    Act::Copy => {
      local::copy(&identity.fields.password);
      notice.0 = Some(("Password copied to the clipboard.".into(), now))
    }
    Act::Submit => {
      focus.0 = None;
      notice.0 = None;
      identity.submit()
    }
    _ => ()
  })
}

fn type_text(
  mut typed: MessageReader<KeyboardInput>,
  keys: Res<ButtonInput<KeyCode>>,
  menu: Res<Menu>,
  mut focus: ResMut<Focus>,
  mut identity: ResMut<Identity>
) {
  let pasted = local::pasted();
  if let Some(entry) = focus.0
    && menu.open
  {
    let control = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let mut text = entry.of(&mut identity.fields).clone();
    let mut submit = false;
    typed.read().filter(|key| key.state == ButtonState::Pressed).for_each(
      |key| match &key.logical_key {
        Key::Backspace => {
          text.pop();
        }
        Key::Enter => submit = true,
        _ => {
          if let Some(chars) = &key.text
            && !control
          {
            text.extend(chars.chars().filter(|c| !c.is_control()))
          }
        }
      }
    );
    pasted
      .iter()
      .for_each(|paste| text.extend(paste.trim().chars().filter(|c| !c.is_control())));
    let text: String = text.chars().take(entry.longest()).collect();
    if *entry.of(&mut identity.fields) != text {
      *entry.of(&mut identity.fields) = text
    }
    if submit {
      focus.0 = None;
      identity.submit()
    }
  } else {
    typed.clear()
  }
}

fn show(
  menu: Res<Menu>,
  settings: Res<Settings>,
  identity: Res<Identity>,
  focus: Res<Focus>,
  notice: Res<Notice>,
  time: Res<Time>,
  mut overlay: Query<&mut Node, (With<Overlay>, Without<Page>)>,
  mut pages: Query<(&Page, &mut Node), Without<Overlay>>,
  mut tabs: Query<(&Act, &mut BorderColor)>,
  mut readings: Query<(&Reading, &mut Text), (Without<Typed>, Without<Remark>)>,
  mut typed: Query<(&Typed, &mut Text), (Without<Reading>, Without<Remark>)>,
  mut standing: Query<&mut Text, (With<Remark>, Without<Reading>, Without<Typed>)>
) {
  let shown = |visible: bool| if visible { Display::Flex } else { Display::None };
  overlay.iter_mut().for_each(|mut node| {
    let display = shown(menu.open);
    if node.display != display {
      node.display = display
    }
  });
  pages.iter_mut().for_each(|(page, mut node)| {
    let display = shown(page.0 == menu.tab);
    if node.display != display {
      node.display = display
    }
  });
  tabs.iter_mut().for_each(|(act, mut border)| {
    let lit = match *act {
      Act::Open(tab) => tab == menu.tab,
      Act::Focus(entry) => focus.0 == Some(entry),
      _ => false
    };
    let colour = BorderColor::all(if lit { LIT } else { EDGE });
    if !matches!(act, Act::Swatch(_)) && *border != colour {
      *border = colour
    }
  });
  let set = |text: &mut Text, value: String| {
    if text.0 != value {
      text.0 = value
    }
  };
  readings
    .iter_mut()
    .for_each(|(reading, mut text)| set(&mut text, settings.reading(reading.0)));
  let caret = (time.elapsed_secs() * 2.0) as u32 % 2 == 0;
  typed.iter_mut().for_each(|(typed, mut text)| {
    let value = match typed.0 {
      Entry::Name => identity.fields.name.clone(),
      Entry::Password => identity.fields.password.clone()
    };
    let cursor = if focus.0 == Some(typed.0) && caret { "|" } else { "" };
    set(&mut text, format!("{value}{cursor}"))
  });
  let changed = identity.fields != identity.sent;
  let line = notice
    .0
    .as_ref()
    .filter(|(_, at)| time.elapsed_secs() - at < 4.0)
    .map(|(text, _)| text.clone())
    .unwrap_or_else(|| match (&identity.standing, changed) {
      (_, true) => "Unsaved changes: press Save.".into(),
      (Standing::Asking, _) => "Signing in...".into(),
      (Standing::Accepted, _) => format!("Signed in as {}.", identity.sent.name),
      (Standing::Refused(reason), _) => reason.clone()
    });
  standing.iter_mut().for_each(|mut text| set(&mut text, line.clone()))
}

pub struct Menus;

impl Plugin for Menus {
  fn build(&self, app: &mut App) {
    let tab = match opts().menu.as_deref() {
      Some("profile") => Tab::Profile,
      Some("skin") => Tab::Skin,
      _ => Tab::Settings
    };
    app
      .insert_resource(Menu { open: opts().menu.is_some(), tab, since: 0.0 })
      .init_resource::<Focus>()
      .init_resource::<Notice>()
      .add_message::<Pressed>()
      .add_systems(Startup, build.after(editor::prepare).run_if(plays))
      .add_systems(Update, (toggle, click, obey, type_text, show).chain().run_if(plays));
  }
}
