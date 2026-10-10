use {crate::{block::Block,
             chart::{self, Chart},
             crafting,
             editor::{self, Draft},
             identity::{Credentials, Identity, Standing},
             local,
             opts::opts,
             player::Pilot,
             protocol::{HUES, Player, Tint, hue, plays},
             settings::{Knob, Settings},
             sign::{self, Inscription},
             stream::Palette,
             trade, waystone},
     bevy::{input::{ButtonState,
                    keyboard::{Key, KeyboardInput}},
            prelude::*,
            window::{CursorGrabMode, CursorOptions, PrimaryWindow}}};

pub const INK: Color = Color::srgb(0.93, 0.95, 0.93);
pub const FAINT: Color = Color::srgb(0.62, 0.66, 0.66);
pub const BUTTON: Color = Color::srgb(0.17, 0.19, 0.22);
const HOVERED: Color = Color::srgb(0.25, 0.28, 0.32);
pub const EDGE: Color = Color::srgb(0.3, 0.33, 0.36);
pub const LIT: Color = Color::srgb(0.95, 0.85, 0.45);
const ESCAPE_GRACE: f32 = 0.3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
  Inventory,
  Settings,
  Profile,
  Skin,
  Waystones,
  Sign,
  Trade,
  Map
}

impl Tab {
  const ALL: [Tab; 8] = [
    Tab::Inventory,
    Tab::Map,
    Tab::Settings,
    Tab::Profile,
    Tab::Skin,
    Tab::Waystones,
    Tab::Sign,
    Tab::Trade
  ];

  fn listed(self) -> bool { !matches!(self, Tab::Sign | Tab::Trade) }

  fn label(self) -> &'static str {
    match self {
      Tab::Inventory => "Inventory",
      Tab::Settings => "Settings",
      Tab::Profile => "Profile",
      Tab::Skin => "Skin",
      Tab::Waystones => "Waystones",
      Tab::Sign => "Sign",
      Tab::Trade => "Trade",
      Tab::Map => "Map"
    }
  }
}

#[derive(Resource)]
pub struct Menu {
  pub open: bool,
  pub tab: Tab,
  pub since: f32,
  pub chatting: bool
}

impl Menu {
  pub fn show(&mut self, tab: Tab, now: f32) {
    self.open = true;
    self.tab = tab;
    self.since = now
  }

  pub fn showing(&self, tab: Tab) -> bool { self.open && self.tab == tab }

  pub fn idle(&self) -> bool { !self.open && !self.chatting }
}

pub fn closed(menu: Res<Menu>) -> bool { menu.idle() }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
  Name,
  Password,
  Search,
  Line(u8)
}

#[derive(Resource, Default)]
pub struct Search(pub String);

impl Entry {
  fn longest(self) -> usize {
    match self {
      Entry::Name => crate::account::LONGEST_NAME,
      Entry::Password => 64,
      Entry::Search => 32,
      Entry::Line(_) => sign::LINE
    }
  }

  fn of<'a>(
    self,
    credentials: &'a mut Credentials,
    search: &'a mut Search,
    inscription: &'a mut Inscription
  ) -> &'a mut String {
    match self {
      Entry::Name => &mut credentials.name,
      Entry::Password => &mut credentials.password,
      Entry::Search => &mut search.0,
      Entry::Line(line) => &mut inscription.lines[usize::from(line)]
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
  Wear,
  Unhide,
  Travel(IVec2),
  Slot(u8),
  Inspect(Block),
  Craft(u16),
  Trade(u8),
  Hue(u8)
}

#[derive(Message, Clone, Copy)]
pub struct Pressed(pub Act);

#[derive(Resource, Default)]
pub struct Focus(pub Option<Entry>);

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
        padding: UiRect::axes(px(8), px(3)),
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
  for knob in Knob::ALL.into_iter() {
    page
      .spawn(Node { align_items: AlignItems::Center, column_gap: px(6), ..default() })
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
  }
  page.spawn(words(
    "Tab or Esc opens and closes this menu, E the inventory. Click the world to look \
     around.",
    14.0,
    FAINT
  ));
}

pub fn field(parent: &mut ChildSpawnerCommands, entry: Entry, width: Val) {
  parent
    .spawn((
      Button,
      Act::Focus(entry),
      Node {
        width,
        flex_grow: if width == Val::Auto { 1.0 } else { 0.0 },
        flex_shrink: 0.0,
        min_height: px(30),
        padding: UiRect::axes(px(6), px(3)),
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
  page.spawn(Node { column_gap: px(6), ..default() }).with_children(|row| {
    field(row, Entry::Name, px(360));
  });
  page.spawn(words("Password", 15.0, FAINT));
  page.spawn(Node { column_gap: px(6), ..default() }).with_children(|row| {
    field(row, Entry::Password, px(360));
    button(row, "New password", Act::Invent);
    if local::CAN_COPY {
      button(row, "Copy", Act::Copy)
    }
  });
  page.spawn(Node { column_gap: px(6), ..default() }).with_children(|row| {
    button(row, "Save", Act::Submit);
  });
  page.spawn(words("Colour (your name, chat and claims show in it)", 15.0, FAINT));
  page.spawn(Node { column_gap: px(4), ..default() }).with_children(|row| {
    for index in 0..HUES.len() as u8 {
      row.spawn((
        Button,
        Act::Hue(index),
        Node { width: px(30), height: px(30), border: UiRect::all(px(3)), ..default() },
        BorderColor::all(EDGE),
        BackgroundColor(hue(index))
      ));
    }
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

fn sign_page(page: &mut ChildSpawnerCommands) {
  page.spawn(words("Write on the sign", 17.0, INK));
  for line in 0..sign::LINES as u8 {
    field(page, Entry::Line(line), px(320))
  }
  page.spawn(Node { column_gap: px(6), ..default() }).with_children(|row| {
    button(row, "Done", Act::Resume);
  });
  page.spawn(words(
    "Enter goes to the next line. Closing the menu keeps what is written.",
    14.0,
    FAINT
  ));
}

fn build(
  mut commands: Commands,
  draft: Res<Draft>,
  palette: Res<Palette>,
  chart: Res<Chart>
) {
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
      BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
      GlobalZIndex(10)
    ))
    .with_children(|overlay| {
      overlay
        .spawn((Node {
          width: percent(100),
          height: percent(100),
          overflow: Overflow::clip(),
          flex_direction: FlexDirection::Column,
          padding: UiRect::all(px(10)),
          row_gap: px(8),
          ..default()
        },))
        .with_children(|panel| {
          panel
            .spawn(Node {
              align_items: AlignItems::Center,
              column_gap: px(6),
              ..default()
            })
            .with_children(|header| {
              header.spawn((words("BLOXY", 26.0, INK), Node {
                margin: UiRect::right(px(10)),
                ..default()
              }));
              for tab in Tab::ALL.into_iter().filter(|tab| tab.listed()) {
                button(header, tab.label(), Act::Open(tab))
              }
              header.spawn(Node { flex_grow: 1.0, ..default() });
              button(header, "Resume", Act::Resume);
            });
          for tab in Tab::ALL.into_iter() {
            panel
              .spawn((Page(tab), Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                flex_grow: 1.0,
                min_height: px(0),
                display: Display::None,
                ..default()
              }))
              .with_children(|page| match tab {
                Tab::Inventory => crafting::page(page, &palette),
                Tab::Settings => settings_page(page),
                Tab::Profile => profile_page(page),
                Tab::Skin => editor::page(page, &draft),
                Tab::Waystones => waystone::page(page),
                Tab::Sign => sign_page(page),
                Tab::Trade => trade::page(page),
                Tab::Map => chart::page(page, &chart)
              });
          }
        });
    });
}

pub fn toggle(
  keys: Res<ButtonInput<KeyCode>>,
  time: Res<Time>,
  mut menu: ResMut<Menu>,
  mut focus: ResMut<Focus>,
  mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
  mut locked: Local<bool>,
  mut was: Local<bool>
) {
  let now = time.elapsed_secs();
  let chatting = menu.chatting;
  let lost = local::pointer_locked().is_some_and(|held| {
    let lost = *locked && !held;
    *locked = held;
    lost
  });
  let escape = keys.just_pressed(KeyCode::Escape) && now - menu.since > ESCAPE_GRACE;
  let shortcut = [(KeyCode::KeyE, Tab::Inventory), (KeyCode::KeyM, Tab::Map)]
    .into_iter()
    .find(|&(key, _)| keys.just_pressed(key) && focus.0.is_none())
    .map(|(_, tab)| tab);
  match (
    menu.open,
    (keys.just_pressed(KeyCode::Tab) || escape) && !chatting,
    lost && !chatting,
    shortcut.filter(|_| !chatting)
  ) {
    (false, _, _, Some(tab)) => menu.show(tab, now),
    (true, _, _, Some(tab)) if menu.tab == tab => menu.open = false,
    (true, _, _, Some(tab)) => menu.tab = tab,
    (false, true, _, _) | (false, _, true, _) => {
      let tab = menu.tab;
      menu.show(tab, now)
    }
    (true, true, _, _) => menu.open = false,
    _ => ()
  }
  if *was != menu.open
    && let Ok(mut cursor) = cursor.single_mut()
  {
    *was = menu.open;
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
  for (interaction, &act, shaded, mut background) in buttons.iter_mut() {
    if shaded.is_some() {
      background.0 = match interaction {
        Interaction::Hovered | Interaction::Pressed => HOVERED,
        Interaction::None => BUTTON
      }
    }
    if *interaction == Interaction::Pressed {
      pressed.write(Pressed(act));
    }
  }
}

fn resume(
  menu: &mut Menu,
  focus: &mut Focus,
  cursor: &mut Query<&mut CursorOptions, With<PrimaryWindow>>
) {
  menu.open = false;
  focus.0 = None;
  if let Ok(mut cursor) = cursor.single_mut() {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false
  }
}

fn hues(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  players: Query<&Player>,
  mut swatches: Query<(&Act, &mut BorderColor)>
) {
  if menu.showing(Tab::Profile)
    && let Some(player) = pilot.and_then(|pilot| players.get(pilot.me).ok())
  {
    for (act, mut border) in swatches.iter_mut() {
      if let &Act::Hue(index) = act {
        border.set_if_neq(BorderColor::all(if index == player.hue { LIT } else { EDGE }));
      }
    }
  }
}

fn obey(
  mut pressed: MessageReader<Pressed>,
  mut tints: MessageWriter<Tint>,
  time: Res<Time>,
  mut menu: ResMut<Menu>,
  mut settings: ResMut<Settings>,
  mut identity: ResMut<Identity>,
  mut focus: ResMut<Focus>,
  mut notice: ResMut<Notice>,
  mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>
) {
  let now = time.elapsed_secs();
  for &Pressed(act) in pressed.read() {
    match act {
      Act::Resume => resume(&mut menu, &mut focus, &mut cursor),
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
      Act::Hue(index) => {
        tints.write(Tint(index));
      }
      _ => ()
    }
  }
}

fn type_text(
  mut typed: MessageReader<KeyboardInput>,
  keys: Res<ButtonInput<KeyCode>>,
  mut menu: ResMut<Menu>,
  mut focus: ResMut<Focus>,
  mut identity: ResMut<Identity>,
  mut search: ResMut<Search>,
  mut inscription: ResMut<Inscription>,
  mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>
) {
  let pasted = local::pasted();
  if menu.showing(Tab::Sign) && focus.0.is_none() {
    focus.0 = Some(Entry::Line(0))
  }
  if let Some(entry) = focus.0
    && menu.open
  {
    let control = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let mut text = entry.of(&mut identity.fields, &mut search, &mut inscription).clone();
    let mut submit = false;
    for key in typed.read().filter(|key| key.state == ButtonState::Pressed) {
      match &key.logical_key {
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
    }
    for paste in pasted.iter() {
      text.extend(paste.trim().chars().filter(|c| !c.is_control()))
    }
    let text: String = text.chars().take(entry.longest()).collect();
    if *entry.of(&mut identity.fields, &mut search, &mut inscription) != text {
      *entry.of(&mut identity.fields, &mut search, &mut inscription) = text
    }
    if submit && entry == Entry::Search {
      focus.0 = None
    } else if submit && let Entry::Line(line) = entry {
      match usize::from(line) + 1 < sign::LINES {
        true => focus.0 = Some(Entry::Line(line + 1)),
        false => resume(&mut menu, &mut focus, &mut cursor)
      }
    } else if submit {
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
  search: Res<Search>,
  inscription: Res<Inscription>,
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
  for mut node in overlay.iter_mut() {
    let display = shown(menu.open);
    if node.display != display {
      node.display = display
    }
  }
  for (page, mut node) in pages.iter_mut() {
    let display = shown(page.0 == menu.tab);
    if node.display != display {
      node.display = display
    }
  }
  for (act, mut border) in tabs.iter_mut() {
    let lit = match *act {
      Act::Open(tab) => tab == menu.tab,
      Act::Focus(entry) => focus.0 == Some(entry),
      _ => false
    };
    let colour = BorderColor::all(if lit { LIT } else { EDGE });
    if matches!(act, Act::Open(_) | Act::Focus(_)) && *border != colour {
      *border = colour
    }
  }
  let set = |text: &mut Text, value: String| {
    if text.0 != value {
      text.0 = value
    }
  };
  for (reading, mut text) in readings.iter_mut() {
    set(&mut text, settings.reading(reading.0))
  }
  let caret = (time.elapsed_secs() * 2.0) as u32 % 2 == 0;
  for (typed, mut text) in typed.iter_mut() {
    let value = match typed.0 {
      Entry::Name => identity.fields.name.clone(),
      Entry::Password => identity.fields.password.clone(),
      Entry::Search => search.0.clone(),
      Entry::Line(line) => inscription.lines[usize::from(line)].clone()
    };
    let cursor = if focus.0 == Some(typed.0) && caret { "|" } else { "" };
    set(&mut text, format!("{value}{cursor}"))
  }
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
  for mut text in standing.iter_mut() {
    set(&mut text, line.clone())
  }
}

pub struct Menus;

impl Plugin for Menus {
  fn build(&self, app: &mut App) {
    let tab = match opts().menu.as_deref() {
      Some("inventory") => Tab::Inventory,
      Some("profile") => Tab::Profile,
      Some("skin") => Tab::Skin,
      Some("waystones") => Tab::Waystones,
      Some("sign") => Tab::Sign,
      Some("map") => Tab::Map,
      _ => Tab::Settings
    };
    app
      .insert_resource(Menu {
        open: opts().menu.is_some(),
        tab,
        since: 0.0,
        chatting: false
      })
      .init_resource::<Focus>()
      .init_resource::<Search>()
      .init_resource::<Notice>()
      .add_message::<Pressed>()
      .add_systems(
        Startup,
        build
          .after(editor::prepare)
          .after(crate::stream::paint)
          .after(chart::prepare)
          .run_if(plays)
      )
      .add_systems(
        Update,
        (toggle, click, obey, type_text, show, hues).chain().run_if(plays)
      );
  }
}
