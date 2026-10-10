use {crate::{block::Block,
             hud::icon_rect,
             menu::{Act, BUTTON, EDGE, Entry, FAINT, Focus, INK, LIT, Menu, Pressed,
                    Search, Shaded, Tab, button, field, words},
             player::Pilot,
             protocol::{Bookmarks, CURSOR, Craft, HOTBAR, Inventory, Mark, Pick,
                        Picked, REACH, SLOTS, Shuffle, plays},
             recipe::{self, Recipe},
             stream::Palette,
             voxels::Voxels},
     bevy::{input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
            prelude::*,
            ui::RelativeCursorPosition,
            window::PrimaryWindow}};

const CELL: f32 = 46.0;
const GAP: f32 = 0.0;
const CAN: Color = Color::srgb(0.45, 0.85, 0.4);
const SHORT: Color = Color::srgb(0.95, 0.42, 0.36);
const SHADOW: TextShadow = crate::hud::SHADE;

#[derive(Resource, Default)]
struct Chosen(Option<Block>);

#[derive(Component)]
struct SlotIcon(usize);

#[derive(Component)]
struct SlotCount(usize);

#[derive(Component)]
struct Catalogue;

#[derive(Component)]
struct Recipes;

#[derive(Component)]
struct Marks;

#[derive(Component)]
struct Tooltip;

#[derive(Component)]
struct Scrolled;

#[derive(Component)]
struct Carried;

#[derive(Component)]
struct Track(Entity);

#[derive(Component)]
struct Thumb;

const TRACK: f32 = 10.0;
const TRACK_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.06);
const THUMB_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.35);

fn scrolled(parent: &mut ChildSpawnerCommands, content: impl Bundle) {
  parent
    .spawn(Node {
      flex_grow: 1.0,
      flex_basis: px(0),
      min_height: px(0),
      column_gap: px(4),
      width: percent(100),
      ..default()
    })
    .with_children(|frame| {
      let pane = frame
        .spawn((
          Scrolled,
          RelativeCursorPosition::default(),
          ScrollPosition::default(),
          Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_height: px(0),
            overflow: Overflow::scroll_y(),
            ..default()
          }
        ))
        .with_child(content)
        .id();
      frame
        .spawn((
          Track(pane),
          RelativeCursorPosition::default(),
          Node { width: px(TRACK), flex_shrink: 0.0, ..default() },
          BackgroundColor(TRACK_COLOR)
        ))
        .with_child((
          Thumb,
          Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            ..default()
          },
          BackgroundColor(THUMB_COLOR)
        ));
    });
}

fn reach(computed: &ComputedNode) -> f32 {
  (computed.content_size.y - computed.size.y + computed.scrollbar_size.y).max(0.0)
    * computed.inverse_scale_factor
}

#[derive(Resource, Default)]
struct Nearby(Vec<Block>);

pub fn icon(block: Block, size: f32, palette: &Palette) -> impl Bundle {
  (
    ImageNode { rect: Some(icon_rect(block)), ..ImageNode::new(palette.icons.clone()) },
    Node { width: px(size), height: px(size), ..default() }
  )
}

fn tile(act: Act, border: Color) -> impl Bundle {
  (
    Button,
    act,
    Shaded,
    Node {
      width: px(CELL),
      height: px(CELL),
      border: UiRect::all(px(2)),
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      ..default()
    },
    BorderColor::all(border),
    BackgroundColor(BUTTON)
  )
}

fn slot(parent: &mut ChildSpawnerCommands, index: usize, palette: &Palette) {
  parent.spawn(tile(Act::Slot(index as u8), EDGE)).with_children(|slot| {
    slot.spawn((SlotIcon(index), icon(Block::Stone, 40.0, palette), Visibility::Hidden));
    slot.spawn((SlotCount(index), words("", 13.0, INK), SHADOW, Node {
      position_type: PositionType::Absolute,
      right: px(3),
      bottom: px(1),
      ..default()
    }));
  });
}

fn row(
  parent: &mut ChildSpawnerCommands,
  slots: std::ops::Range<usize>,
  palette: &Palette
) {
  parent
    .spawn(Node {
      width: px(HOTBAR as f32 * (CELL + GAP)),
      flex_wrap: FlexWrap::Wrap,
      column_gap: px(GAP),
      row_gap: px(GAP),
      ..default()
    })
    .with_children(|row| {
      for index in slots {
        slot(row, index, palette)
      }
    });
}

fn column(width: Option<f32>, grow: f32) -> Node {
  Node {
    flex_direction: FlexDirection::Column,
    row_gap: px(4),
    width: width.map_or(Val::Auto, px),
    flex_grow: if width.is_some() { 0.0 } else { grow },
    flex_basis: if width.is_some() { Val::Auto } else { px(0) },
    min_width: px(0),
    min_height: px(0),
    flex_shrink: 0.0,
    ..default()
  }
}

fn grid() -> Node {
  Node { flex_wrap: FlexWrap::Wrap, column_gap: px(GAP), row_gap: px(GAP), ..default() }
}

pub fn page(page: &mut ChildSpawnerCommands, palette: &Palette) {
  page
    .spawn(Node {
      column_gap: px(16),
      width: percent(100),
      flex_grow: 1.0,
      min_height: px(0),
      ..default()
    })
    .with_children(|columns| {
      columns.spawn(column(None, 1.0)).with_children(|left| {
        left.spawn(words("Bookmarks", 15.0, FAINT));
        scrolled(left, (Marks, Node { flex_shrink: 0.0, ..grid() }));
        left.spawn(words(
          "Press A over an item to bookmark it or take it off.",
          13.0,
          FAINT
        ));
      });
      columns
        .spawn(column(Some(HOTBAR as f32 * (CELL + GAP) + 180.0), 0.0))
        .with_children(|middle| {
          scrolled(
            middle,
            (Recipes, Node {
              flex_direction: FlexDirection::Column,
              row_gap: px(4),
              flex_shrink: 0.0,
              ..default()
            })
          );
          middle.spawn(words("Backpack", 15.0, FAINT));
          row(middle, HOTBAR..SLOTS, palette);
          middle.spawn(words("Hotbar", 15.0, FAINT));
          row(middle, 0..HOTBAR, palette);
        });
      columns.spawn(column(None, 1.6)).with_children(|right| {
        right
          .spawn(Node {
            column_gap: px(10),
            align_items: AlignItems::Center,
            width: percent(100),
            ..default()
          })
          .with_children(|search| {
            search.spawn(words("Search", 15.0, FAINT));
            field(search, Entry::Search, Val::Auto);
          });
        scrolled(right, (Catalogue, Node { flex_shrink: 0.0, ..grid() }));
        right.spawn(words(
          "Click an item for its recipes. Items you can craft here and now come first, \
         outlined in green. Crafting tables and furnaces count within reach.",
          13.0,
          FAINT
        ));
      });
    });
}

fn tooltip(mut commands: Commands, palette: Res<Palette>) {
  commands
    .spawn((
      Carried,
      Node { position_type: PositionType::Absolute, ..default() },
      GlobalZIndex(40),
      Visibility::Hidden
    ))
    .with_children(|carried| {
      carried.spawn(icon(Block::Stone, 40.0, &palette));
      carried.spawn((words("", 13.0, INK), SHADOW, Node {
        position_type: PositionType::Absolute,
        right: px(1),
        bottom: px(-1),
        ..default()
      }));
    });
  commands
    .spawn((
      Tooltip,
      Node {
        position_type: PositionType::Absolute,
        padding: UiRect::axes(px(5), px(2)),
        border: UiRect::all(px(2)),
        ..default()
      },
      BorderColor::all(Color::srgb(0.25, 0.15, 0.45)),
      BackgroundColor(Color::srgba(0.06, 0.03, 0.1, 0.95)),
      GlobalZIndex(30),
      Visibility::Hidden
    ))
    .with_child(words("", 15.0, INK));
}

fn carried(pilot: &Option<Res<Pilot>>, inventories: &Query<&Inventory>) -> Inventory {
  pilot
    .as_ref()
    .and_then(|pilot| inventories.get(pilot.me).ok().cloned())
    .unwrap_or_default()
}

fn fill(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  mut icons: Query<(&SlotIcon, &mut ImageNode, &mut Visibility)>,
  mut counts: Query<(&SlotCount, &mut Text)>,
  mut picks: MessageWriter<Picked>,
  mut was: Local<bool>
) {
  let inventory = carried(&pilot, &inventories);
  let showing = menu.showing(Tab::Inventory);
  if *was && !showing && inventory.slots[CURSOR].is_some() {
    picks.write(Picked(Pick::Stow));
  }
  *was = showing;
  if showing {
    for (icon, mut image, mut visibility) in icons.iter_mut() {
      match inventory.slots[icon.0] {
        Some(stack) => {
          let rect = Some(icon_rect(stack.block));
          if image.rect != rect {
            image.rect = rect
          }
          visibility.set_if_neq(Visibility::Inherited);
        }
        None => {
          visibility.set_if_neq(Visibility::Hidden);
        }
      }
    }
    for (count, mut text) in counts.iter_mut() {
      let shown = inventory.slots[count.0]
        .filter(|stack| stack.count > 1)
        .map_or(String::new(), |stack| stack.count.to_string());
      if text.0 != shown {
        text.0 = shown
      }
    }
  }
}

fn survey(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  voxels: Option<Res<Voxels>>,
  mut nearby: ResMut<Nearby>
) {
  if menu.showing(Tab::Inventory)
    && let Some(pilot) = pilot
    && let Some(voxels) = voxels
  {
    let found = voxels.stations_near(pilot.eye(), REACH);
    if nearby.0 != found {
      nearby.0 = found
    }
  }
}

fn entry(
  list: &mut ChildSpawnerCommands,
  (block, craftable, picked): (Block, bool, bool),
  palette: &Palette
) {
  let border = match (picked, craftable) {
    (true, _) => LIT,
    (false, true) => CAN,
    (false, false) => EDGE
  };
  list.spawn(tile(Act::Inspect(block), border)).with_child(icon(block, 40.0, palette));
}

fn catalogue(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  chosen: Res<Chosen>,
  nearby: Res<Nearby>,
  search: Res<Search>,
  bookmarks: Query<&Bookmarks>,
  palette: Res<Palette>,
  lists: Query<Entity, With<Catalogue>>,
  marks: Query<Entity, With<Marks>>,
  mut commands: Commands,
  mut shown: Local<(Vec<(Block, bool, bool)>, Vec<(Block, bool, bool)>)>
) {
  if menu.showing(Tab::Inventory) {
    let inventory = carried(&pilot, &inventories);
    let describe = |block: Block| {
      (block, recipe::craftable(block, &inventory, &nearby.0), chosen.0 == Some(block))
    };
    let query = search.0.trim().to_lowercase();
    let mut entries: Vec<(Block, bool, bool)> = Block::ALL
      .into_iter()
      .filter(|block| block.item() && block.name().to_lowercase().contains(&query))
      .map(describe)
      .collect();
    entries.sort_by_key(|&(_, craftable, _)| !craftable);
    let marked: Vec<(Block, bool, bool)> = pilot
      .as_ref()
      .and_then(|pilot| bookmarks.get(pilot.me).ok())
      .into_iter()
      .flat_map(|bookmarks| bookmarks.0.iter().copied().map(describe))
      .collect();
    if shown.0 != entries {
      for list in lists.iter() {
        commands.entity(list).despawn_children().with_children(|list| {
          for &found in entries.iter() {
            entry(list, found, &palette)
          }
        });
      }
      shown.0 = entries
    }
    if shown.1 != marked {
      for list in marks.iter() {
        commands.entity(list).despawn_children().with_children(|list| {
          for &found in marked.iter() {
            entry(list, found, &palette)
          }
        });
      }
      shown.1 = marked
    }
  }
}

fn card(
  parent: &mut ChildSpawnerCommands,
  index: usize,
  recipe: &Recipe,
  inventory: &Inventory,
  nearby: &[Block],
  palette: &Palette
) {
  let ingredient = |row: &mut ChildSpawnerCommands, block: Block| {
    row
      .spawn((Button, Act::Inspect(block), Node::default()))
      .with_child(icon(block, 28.0, palette));
  };
  let missing: Vec<String> = recipe
    .missing(inventory)
    .iter()
    .map(|&(block, short)| format!("{short} {}", block.name()))
    .chain(
      recipe
        .station
        .filter(|_| !recipe.housed(nearby))
        .map(|station| format!("a {} within reach", station.name()))
    )
    .collect();
  parent
    .spawn(Node { align_items: AlignItems::Center, column_gap: px(6), ..default() })
    .with_children(|row| {
      for &(block, need) in recipe.inputs.iter() {
        let have = inventory.count(block);
        ingredient(row, block);
        row.spawn((
          words(format!("{have}/{need}"), 14.0, match have >= u32::from(need) {
            true => CAN,
            false => SHORT
          }),
          Node { margin: UiRect::right(px(6)), ..default() }
        ));
      }
      row.spawn(words("->", 16.0, FAINT));
      let (output, count) = recipe.output;
      ingredient(row, output);
      row.spawn((words(format!("x{count}"), 14.0, INK), Node {
        margin: UiRect::right(px(6)),
        ..default()
      }));
      if let Some(station) = recipe.station {
        row.spawn(words("at", 14.0, FAINT));
        ingredient(row, station);
      }
      if missing.is_empty() {
        button(row, "Craft", Act::Craft(index as u16))
      }
    });
  if !missing.is_empty() {
    parent.spawn(words(format!("Missing {}", missing.join(", ")), 13.0, SHORT));
  }
}

fn recipes(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  chosen: Res<Chosen>,
  nearby: Res<Nearby>,
  palette: Res<Palette>,
  containers: Query<Entity, With<Recipes>>,
  mut commands: Commands,
  mut shown: Local<Option<(Option<Block>, Inventory, Vec<Block>)>>
) {
  if menu.showing(Tab::Inventory) {
    let state = (chosen.0, carried(&pilot, &inventories), nearby.0.clone());
    if shown.as_ref() != Some(&state) {
      let (chosen, inventory, nearby) = &state;
      for container in containers.iter() {
        commands.entity(container).despawn_children().with_children(
          |panel| match *chosen {
            None => {
              panel.spawn(words("Crafting", 20.0, INK));
              panel.spawn(words(
                "Pick an item on the right to see its recipes.",
                15.0,
                FAINT
              ));
            }
            Some(block) => {
              panel.spawn(words(block.name(), 20.0, INK));
              panel.spawn(words("Made from", 15.0, FAINT));
              let making: Vec<_> = recipe::making(block).collect();
              if making.is_empty() {
                panel.spawn(words("Not craftable: gather it in the world.", 14.0, INK));
              }
              for (index, recipe) in making.into_iter() {
                card(panel, index, recipe, inventory, nearby, &palette)
              }
              let using: Vec<_> = recipe::using(block).collect();
              if !using.is_empty() {
                panel.spawn(words("Used in", 15.0, FAINT));
              }
              for (index, recipe) in using.into_iter() {
                card(panel, index, recipe, inventory, nearby, &palette)
              }
            }
          }
        );
      }
      *shown = Some(state)
    }
  }
}

fn rewind(
  chosen: Res<Chosen>,
  containers: Query<&ChildOf, With<Recipes>>,
  mut panes: Query<&mut ScrollPosition>
) {
  if chosen.is_changed() {
    for parent in containers.iter() {
      if let Ok(mut position) = panes.get_mut(parent.parent()) {
        position.0 = Vec2::ZERO
      }
    }
  }
}

fn scroll(
  wheel: Res<AccumulatedMouseScroll>,
  buttons: Res<ButtonInput<MouseButton>>,
  mut panes: Query<
    (&RelativeCursorPosition, &ComputedNode, &mut ScrollPosition),
    With<Scrolled>
  >,
  mut tracks: Query<(
    Entity,
    &Track,
    &RelativeCursorPosition,
    &Children,
    &mut Visibility
  )>,
  mut thumbs: Query<&mut Node, With<Thumb>>,
  mut dragging: Local<Option<(Entity, f32)>>
) {
  let lines = match wheel.unit {
    MouseScrollUnit::Line => wheel.delta.y * 40.0,
    MouseScrollUnit::Pixel => wheel.delta.y
  };
  for (cursor, computed, mut position) in panes.iter_mut() {
    let most = reach(computed);
    let wanted = match cursor.cursor_over() {
      true => position.0.y - lines,
      false => position.0.y
    }
    .clamp(0.0, most);
    if position.0.y != wanted {
      position.0.y = wanted
    }
  }
  if !buttons.pressed(MouseButton::Left) {
    *dragging = None
  }
  for (track, &Track(pane), cursor, children, mut visibility) in tracks.iter_mut() {
    if let Ok((_, computed, mut position)) = panes.get_mut(pane) {
      let content = (computed.content_size.y * computed.inverse_scale_factor).max(1.0);
      let shown = (computed.size.y * computed.inverse_scale_factor / content).min(1.0);
      let top = position.0.y / content;
      let pointer = cursor.normalized.map(|at| at.y + 0.5);
      if buttons.just_pressed(MouseButton::Left)
        && cursor.cursor_over()
        && shown < 1.0
        && let Some(pointer) = pointer
      {
        let grab = match (top..top + shown).contains(&pointer) {
          true => pointer - top,
          false => shown / 2.0
        };
        *dragging = Some((track, grab))
      }
      if let Some((held, grab)) = *dragging
        && held == track
        && let Some(pointer) = pointer
      {
        position.0.y = ((pointer - grab) * content).clamp(0.0, reach(computed))
      }
      for child in children.iter() {
        if let Ok(mut node) = thumbs.get_mut(child) {
          let top = percent(position.0.y / content * 100.0);
          let tall = percent(shown * 100.0);
          if node.top != top || node.height != tall {
            node.top = top;
            node.height = tall
          }
        }
      }
      visibility.set_if_neq(match shown < 1.0 {
        true => Visibility::Inherited,
        false => Visibility::Hidden
      });
    }
  }
}

fn hint(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  hovered: Query<(&Interaction, &Act)>,
  keys: Res<ButtonInput<KeyCode>>,
  focus: Res<Focus>,
  windows: Query<&Window, With<PrimaryWindow>>,
  scale: Res<UiScale>,
  mut marks: MessageWriter<Mark>,
  mut tips: Query<(&mut Node, &mut Visibility, &Children), With<Tooltip>>,
  mut texts: Query<&mut Text>
) {
  let inventory = carried(&pilot, &inventories);
  let pointed = hovered
    .iter()
    .filter(|(interaction, _)| **interaction != Interaction::None)
    .find_map(|(_, act)| match *act {
      Act::Inspect(block) => Some((block, None)),
      Act::Slot(index) => {
        inventory.slots[usize::from(index)].map(|stack| (stack.block, Some(stack.count)))
      }
      _ => None
    })
    .filter(|_| menu.showing(Tab::Inventory));
  if let Some((block, _)) = pointed
    && keys.just_pressed(KeyCode::KeyA)
    && focus.0.is_none()
  {
    marks.write(Mark(block));
  }
  let cursor =
    windows.single().ok().and_then(Window::cursor_position).map(|at| at / scale.0);
  for (mut node, mut visibility, children) in tips.iter_mut() {
    match pointed.zip(cursor) {
      Some(((block, count), at)) => {
        node.left = px(at.x + 16.0);
        node.top = px(at.y + 12.0);
        visibility.set_if_neq(Visibility::Inherited);
        let line = match count {
          Some(count) if count > 1 => format!("{} x{count}", block.name()),
          _ => block.name().to_string()
        };
        for child in children.iter() {
          if let Ok(mut text) = texts.get_mut(child)
            && text.0 != line
          {
            text.0 = line.clone()
          }
        }
      }
      None => {
        visibility.set_if_neq(Visibility::Hidden);
      }
    }
  }
}

fn carry(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  windows: Query<&Window, With<PrimaryWindow>>,
  scale: Res<UiScale>,
  mut carried: Query<(&mut Node, &mut Visibility, &Children), With<Carried>>,
  mut images: Query<&mut ImageNode>,
  mut texts: Query<&mut Text>
) {
  let stack = self::carried(&pilot, &inventories).slots[CURSOR]
    .filter(|_| menu.showing(Tab::Inventory));
  let cursor =
    windows.single().ok().and_then(Window::cursor_position).map(|at| at / scale.0);
  for (mut node, mut visibility, children) in carried.iter_mut() {
    match stack.zip(cursor) {
      Some((stack, at)) => {
        node.left = px(at.x - 20.0);
        node.top = px(at.y - 20.0);
        visibility.set_if_neq(Visibility::Inherited);
        for child in children.iter() {
          if let Ok(mut image) = images.get_mut(child)
            && image.rect != Some(icon_rect(stack.block))
          {
            image.rect = Some(icon_rect(stack.block))
          }
          if let Ok(mut text) = texts.get_mut(child) {
            let shown =
              (stack.count > 1).then(|| stack.count.to_string()).unwrap_or_default();
            if text.0 != shown {
              text.0 = shown
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
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  buttons: Res<ButtonInput<MouseButton>>,
  hovered: Query<(&Interaction, &Act)>,
  mut chosen: ResMut<Chosen>,
  mut shuffles: MessageWriter<Shuffle>,
  mut picks: MessageWriter<Picked>,
  mut crafts: MessageWriter<Craft>
) {
  let inventory = carried(&pilot, &inventories);
  let holding = inventory.slots[CURSOR].is_some();
  if buttons.just_pressed(MouseButton::Right)
    && let Some(index) = hovered
      .iter()
      .filter(|(interaction, _)| **interaction != Interaction::None)
      .find_map(|(_, act)| match *act {
        Act::Slot(index) => Some(index),
        _ => None
      })
  {
    picks.write(Picked(if holding { Pick::One(index) } else { Pick::Half(index) }));
  }
  for &Pressed(act) in pressed.read() {
    match act {
      Act::Slot(index) => {
        let (from, to) =
          if holding { (CURSOR as u8, index) } else { (index, CURSOR as u8) };
        shuffles.write(Shuffle { from, to });
      }
      Act::Inspect(block) => chosen.0 = Some(block),
      Act::Craft(index) => {
        crafts.write(Craft(index));
      }
      _ => ()
    }
  }
}

pub struct Crafting;

impl Plugin for Crafting {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Chosen>()
      .init_resource::<Nearby>()
      .add_systems(Startup, tooltip.after(crate::stream::paint).run_if(plays))
      .add_systems(
        Update,
        (obey, survey, fill, carry, catalogue, recipes, rewind, scroll, hint)
          .chain()
          .run_if(plays)
      );
  }
}
