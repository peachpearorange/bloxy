use {crate::{block::Block,
             chest::{Coffer, Handling, ROOM, Rummage},
             gear::{self, Material, Virtue, Wear},
             glyph::Glyph,
             hud::{forged_rect, icon_rect, stack_rect},
             legend,
             machine::Liquid,
             menu::{Act, BUTTON, EDGE, Entry, FAINT, Focus, INK, LIT, Menu, Pressed,
                    Search, Shaded, Tab, button, field, words},
             player::Pilot,
             protocol::{Bookmarks, CURSOR, Craft, HOTBAR, Inventory, Mark, Pick,
                        Picked, REACH, SLOTS, Shuffle, Stack, WEARING, plays},
             recipe::{self, Recipe},
             skill::{Glyphs, Skills, glyph_image},
             stream::Palette,
             voxels::Voxels},
     bevy::{input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
            prelude::*,
            ui::RelativeCursorPosition,
            window::PrimaryWindow}};

const CELL: f32 = 46.0;
const GAP: f32 = 0.0;
const BELT_ROOM: f32 = crate::hud::BELT_BOTTOM + crate::hud::SLOT - PAGE_MARGIN;
const PAGE_MARGIN: f32 = 10.0;
const CAN: Color = Color::srgb(0.45, 0.85, 0.4);
const SHORT: Color = Color::srgb(0.95, 0.42, 0.36);
const SHADOW: TextShadow = crate::hud::SHADE;

#[derive(Resource, Default)]
struct Chosen(Option<Block>);

#[derive(Clone, Copy, PartialEq, Debug)]
enum Place {
  Own(usize),
  Chest(usize)
}

impl Place {
  fn act(self) -> Act {
    match self {
      Place::Own(index) => Act::Slot(index as u8),
      Place::Chest(index) => Act::Stash(index as u8)
    }
  }

  fn held(self, inventory: &Inventory, coffer: &Coffer) -> Option<Stack> {
    match self {
      Place::Own(index) => inventory.slots[index],
      Place::Chest(index) => coffer.0.slots.get(index).copied().flatten()
    }
  }
}

#[derive(Component)]
struct SlotIcon(Place);

#[derive(Component)]
struct SlotCount(Place);

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

fn slot(parent: &mut ChildSpawnerCommands, place: Place, palette: &Palette) {
  parent.spawn(tile(place.act(), EDGE)).with_children(|slot| {
    slot.spawn((
      SlotIcon(place),
      icon(Block::Greywacke, 40.0, palette),
      Visibility::Hidden
    ));
    slot.spawn((SlotCount(place), words("", 13.0, INK), SHADOW, Node {
      position_type: PositionType::Absolute,
      right: px(3),
      bottom: px(1),
      ..default()
    }));
  });
}

fn row(
  parent: &mut ChildSpawnerCommands,
  slots: impl IntoIterator<Item = Place>,
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
      for place in slots {
        slot(row, place, palette)
      }
    });
}

fn backpack(parent: &mut ChildSpawnerCommands, palette: &Palette) {
  parent
    .spawn(Node { align_self: AlignSelf::Center, flex_shrink: 0.0, ..default() })
    .with_children(|centred| row(centred, (HOTBAR..SLOTS).map(Place::Own), palette));
  parent.spawn(Node { height: px(BELT_ROOM), flex_shrink: 0.0, ..default() });
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
          middle.spawn((words("Backpack", 15.0, FAINT), Node {
            align_self: AlignSelf::Center,
            width: px(HOTBAR as f32 * (CELL + GAP)),
            ..default()
          }));
          backpack(middle, palette);
        });
      columns.spawn(column(None, 1.0)).with_children(|right| {
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

#[derive(Component)]
struct HoardTitle;

fn fit_hoard(
  coffer: Res<Coffer>,
  mut slots: Query<(&Act, &mut Node)>,
  mut titles: Query<&mut Text, With<HoardTitle>>
) {
  let room = coffer.0.slots.len();
  for (act, mut node) in slots.iter_mut() {
    if let &Act::Stash(index) = act {
      let display = match usize::from(index) < room {
        true => Display::Flex,
        false => Display::None
      };
      if node.display != display {
        node.display = display
      }
    }
  }
  let title = match coffer.0.kind {
    rift if rift.rift() => "Pocket Rift",
    kind => kind.name()
  };
  for mut text in titles.iter_mut() {
    if text.0 != title {
      text.0 = title.to_string()
    }
  }
}

pub fn chest_page(page: &mut ChildSpawnerCommands, palette: &Palette) {
  page
    .spawn(Node {
      flex_direction: FlexDirection::Column,
      align_items: AlignItems::Center,
      justify_content: JustifyContent::FlexEnd,
      row_gap: px(4),
      width: percent(100),
      flex_grow: 1.0,
      ..default()
    })
    .with_children(|middle| {
      middle.spawn((HoardTitle, words("Chest", 20.0, INK)));
      row(middle, (0..ROOM).map(Place::Chest), palette);
      middle.spawn((
        words(
          "Click to pick up and put down stacks. Right-click takes half a stack, or puts \
           down one while carrying. Shift-click moves a stack between the chest and your \
           inventory.",
          13.0,
          FAINT
        ),
        Node { margin: UiRect::vertical(px(8)), ..default() }
      ));
      middle.spawn((words("Backpack", 15.0, FAINT), Node {
        width: px(HOTBAR as f32 * (CELL + GAP)),
        ..default()
      }));
      backpack(middle, palette);
    });
}

fn handling(menu: &Menu) -> bool {
  menu.showing(Tab::Inventory) || menu.showing(Tab::Chest)
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
      carried.spawn(icon(Block::Greywacke, 40.0, &palette));
      carried.spawn((words("", 13.0, INK), SHADOW, Node {
        position_type: PositionType::Absolute,
        right: px(1),
        bottom: px(-1),
        ..default()
      }));
    });
  commands.spawn((
    Tooltip,
    Node {
      position_type: PositionType::Absolute,
      flex_direction: FlexDirection::Column,
      row_gap: px(3),
      max_width: px(380),
      padding: UiRect::axes(px(6), px(4)),
      border: UiRect::all(px(2)),
      ..default()
    },
    BorderColor::all(Color::srgb(0.25, 0.15, 0.45)),
    BackgroundColor(Color::srgba(0.06, 0.03, 0.1, 0.95)),
    GlobalZIndex(30),
    Visibility::Hidden
  ));
}

fn carried(pilot: &Option<Res<Pilot>>, inventories: &Query<&Inventory>) -> Inventory {
  pilot
    .as_ref()
    .and_then(|pilot| inventories.get(pilot.me).ok().cloned())
    .unwrap_or_default()
}

fn known(pilot: &Option<Res<Pilot>>, skills: &Query<&Skills>) -> Vec<Block> {
  pilot
    .as_ref()
    .and_then(|pilot| skills.get(pilot.me).ok())
    .map(Skills::spells)
    .unwrap_or_default()
}

fn fill(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  coffer: Res<Coffer>,
  mut icons: Query<(&SlotIcon, &mut ImageNode, &mut Visibility)>,
  mut counts: Query<(&SlotCount, &mut Text)>,
  mut picks: MessageWriter<Picked>,
  mut was: Local<bool>
) {
  let inventory = carried(&pilot, &inventories);
  let showing = handling(&menu);
  if *was && !showing && inventory.slots[CURSOR].is_some() {
    picks.write(Picked(Pick::Stow));
  }
  *was = showing;
  if showing {
    for (icon, mut image, mut visibility) in icons.iter_mut() {
      match icon.0.held(&inventory, &coffer) {
        Some(stack) => {
          let rect = Some(stack_rect(&stack));
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
      let shown = count
        .0
        .held(&inventory, &coffer)
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
  units: Query<&crate::machine::Unit>,
  mut nearby: ResMut<Nearby>
) {
  if menu.showing(Tab::Inventory)
    && let Some(pilot) = pilot
    && let Some(voxels) = voxels
  {
    let found = voxels.stations_near(pilot.eye(), REACH, |at| {
      units
        .iter()
        .any(|unit| unit.at == at && unit.charge as f32 >= crate::machine::SMELT)
    });
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
  skills: Query<&Skills>,
  palette: Res<Palette>,
  lists: Query<Entity, With<Catalogue>>,
  marks: Query<Entity, With<Marks>>,
  mut commands: Commands,
  mut shown: Local<(Vec<(Block, bool, bool)>, Vec<(Block, bool, bool)>)>
) {
  if menu.showing(Tab::Inventory) {
    let inventory = carried(&pilot, &inventories);
    let spells = known(&pilot, &skills);
    let describe = |block: Block| {
      (
        block,
        recipe::craftable(block, &inventory, &nearby.0, &spells),
        chosen.0 == Some(block)
      )
    };
    let query = search.0.trim().to_lowercase();
    let mut entries: Vec<(Block, bool, bool)> = Block::ALL
      .into_iter()
      .filter(|block| {
        block.item()
          && (!block.spell() || spells.contains(block))
          && block.name().to_lowercase().contains(&query)
      })
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
  spells: &[Block],
  palette: &Palette
) {
  let ingredient = |row: &mut ChildSpawnerCommands, block: Block| {
    row
      .spawn((Button, Act::Inspect(block), Node::default()))
      .with_child(icon(block, 28.0, palette));
  };
  let missing: Vec<String> = recipe
    .missing(inventory, nearby)
    .iter()
    .map(|&(block, short)| format!("{short} {}", block.name()))
    .chain(
      recipe
        .station
        .filter(|_| !recipe.housed(nearby) && !recipe.rite())
        .map(|station| format!("a {} within reach", station.name()))
    )
    .chain(
      (!recipe.learned(spells))
        .then(|| format!("the {} talent (K)", recipe.output.0.name()))
    )
    .collect();
  parent
    .spawn(Node { align_items: AlignItems::Center, column_gap: px(6), ..default() })
    .with_children(|row| {
      for (block, need) in recipe.needs(nearby) {
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
      row.spawn((Button, Act::Inspect(output), Node::default())).with_child((
        ImageNode {
          rect: Some(forged_rect(output, recipe.material)),
          ..ImageNode::new(palette.icons.clone())
        },
        Node { width: px(28), height: px(28), ..default() }
      ));
      row.spawn((words(format!("x{count}"), 14.0, INK), Node {
        margin: UiRect::right(px(6)),
        ..default()
      }));
      if let Some(station) = recipe.station {
        row.spawn(words("at", 14.0, FAINT));
        ingredient(row, match recipe.electric(nearby) {
          true => Block::ElectricFurnace,
          false => station
        });
      }
      if missing.is_empty() && !recipe.rite() {
        button(row, "Craft", Act::Craft(index as u16))
      }
    });
  if let Some(material) = recipe.material {
    let gear::Grit { hardness, toughness } = material.grit();
    parent.spawn(words(
      format!("{}: hardness {hardness}, toughness {toughness}", material.name()),
      13.0,
      FAINT
    ));
  }
  if let Some(how) = recipe.how() {
    parent.spawn(words(how, 13.0, FAINT));
  }
  if !missing.is_empty() && !recipe.rite() {
    parent.spawn(words(format!("Missing {}", missing.join(", ")), 13.0, SHORT));
  }
}

fn equipment(panel: &mut ChildSpawnerCommands, inventory: &Inventory, palette: &Palette) {
  panel.spawn(words("Equipment", 20.0, INK));
  panel
    .spawn(Node { column_gap: px(10), flex_wrap: FlexWrap::Wrap, ..default() })
    .with_children(|row| {
      for (index, wear) in Wear::ALL.into_iter().enumerate() {
        row
          .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(2),
            ..default()
          })
          .with_children(|cell| {
            slot(cell, Place::Own(WEARING + index), palette);
            cell.spawn(words(wear.name(), 12.0, FAINT));
          });
      }
    });
  let totals = inventory
    .worn()
    .iter()
    .flatten()
    .filter_map(|stack| stack.gear)
    .flat_map(|gear| gear.perks.into_iter().flatten())
    .fold(Vec::<(Virtue, u32)>::new(), |mut totals, perk| {
      match totals.iter_mut().find(|(virtue, _)| *virtue == perk.virtue) {
        Some((_, sum)) => *sum += u32::from(perk.by),
        None => totals.push((perk.virtue, u32::from(perk.by)))
      }
      totals
    });
  match totals.is_empty() {
    true => panel.spawn(words(
      "Wear hats, amulets, mail, belts, boots and rings found in dungeons here.",
      14.0,
      FAINT
    )),
    false => panel.spawn(words(
      totals
        .iter()
        .map(|&(virtue, by)| virtue.line(by.min(255) as u8))
        .collect::<Vec<_>>()
        .join("\n"),
      14.0,
      CAN
    ))
  };
  panel.spawn(words("Pick an item on the right to see its recipes.", 13.0, FAINT));
}

fn recipes(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  chosen: Res<Chosen>,
  nearby: Res<Nearby>,
  skills: Query<&Skills>,
  palette: Res<Palette>,
  containers: Query<Entity, With<Recipes>>,
  mut commands: Commands,
  mut shown: Local<Option<(Option<Block>, Inventory, Vec<Block>, Vec<Block>)>>
) {
  if menu.showing(Tab::Inventory) {
    let state =
      (chosen.0, carried(&pilot, &inventories), nearby.0.clone(), known(&pilot, &skills));
    if shown.as_ref() != Some(&state) {
      let (chosen, inventory, nearby, spells) = &state;
      for container in containers.iter() {
        commands.entity(container).despawn_children().with_children(
          |panel| match *chosen {
            None => equipment(panel, inventory, &palette),
            Some(block) => {
              panel
                .spawn(Node {
                  column_gap: px(12),
                  align_items: AlignItems::Center,
                  ..default()
                })
                .with_children(|title| {
                  title.spawn(words(block.name(), 20.0, INK));
                  button(title, "Equipment", Act::Equipment);
                });
              panel.spawn(words("Made from", 15.0, FAINT));
              let making: Vec<_> = recipe::making(block).collect();
              if making.is_empty() {
                panel.spawn(words(
                  match block.worn() {
                    true => "Not craftable: found in dungeon chests.",
                    false => "Not craftable: gather it in the world."
                  },
                  14.0,
                  INK
                ));
              }
              for (index, recipe) in making.into_iter() {
                card(panel, index, recipe, inventory, nearby, spells, &palette)
              }
              let using: Vec<_> = recipe::using(block).collect();
              if !using.is_empty() {
                panel.spawn(words("Used in", 15.0, FAINT));
              }
              for (index, recipe) in using.into_iter() {
                card(panel, index, recipe, inventory, nearby, spells, &palette)
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

fn note(stack: Stack) -> String {
  let block = stack.block;
  block
    .spell()
    .then(|| {
      format!(
        "{} - {}. Hold right click to charge, release to cast; each cast uses one",
        block.name(),
        crate::skill::spell_text(block)
      )
    })
    .or_else(|| {
      block.portable().then(|| {
        let room = crate::chest::room(block);
        match block.pack() {
          true => format!(
            "{} - right-click to open: {room} slots that travel with the bag",
            block.name()
          ),
          false => format!(
            "{} - right-click to open your pocket of folded space: {room} slots, the \
             same from every rift you carry",
            block.name()
          )
        }
      })
    })
    .or_else(|| {
      stack.gear.map(|_| {
        [gear::title(&stack)]
          .into_iter()
          .chain(gear::lines(&stack))
          .collect::<Vec<_>>()
          .join("\n")
      })
    })
    .or_else(|| {
      block.forged().then(|| {
        let (hardness, toughness) = gear::needs(block);
        format!(
          "{} - made from any material with hardness {hardness} and toughness \
           {toughness} or more",
          block.name()
        )
      })
    })
    .or_else(|| {
      Material::of(block).map(|material| {
        let gear::Grit { hardness, toughness } = material.grit();
        let tools: Vec<&str> = Block::TOOLS
          .into_iter()
          .filter(|&tool| material.suits(tool))
          .map(Block::name)
          .collect();
        format!(
          "{} - hardness {hardness}, toughness {toughness}: {}",
          block.name(),
          match tools.is_empty() {
            true => "too weak for tools".to_string(),
            false => format!("strong enough for: {}", tools.join(", "))
          }
        )
      })
    })
    .unwrap_or_else(|| match stack.count {
      count if count > 1 => format!("{} x{count}", block.name()),
      _ => block.name().to_string()
    })
}

const TAGLINE: Color = Color::srgb(0.95, 0.82, 0.5);
const MADE: Color = Color::srgb(1.0, 0.86, 0.25);
const DRAWN: Color = Color::srgb(1.0, 0.45, 0.3);
const STORED: Color = Color::srgb(0.45, 0.75, 1.0);
const FLAME: Color = Color::srgb(1.0, 0.55, 0.15);
const ARROW: Color = Color::srgb(0.6, 0.55, 0.7);
const MOST_SHOWN: usize = 9;

fn watts(power: f32) -> String {
  match power >= 1000.0 {
    true => format!("{:.0}k", power / 1000.0),
    false => format!("{power:.0}")
  }
}

fn chip(
  row: &mut ChildSpawnerCommands,
  chip: legend::Chip,
  palette: &Palette,
  glyphs: &Glyphs
) {
  let marked = |row: &mut ChildSpawnerCommands,
                glyph: Glyph,
                colour: Color,
                label: Option<String>| {
    row.spawn(glyph_image(glyphs, glyph, colour, 18.0));
    if let Some(label) = label {
      row.spawn((words(&label, 13.0, colour), SHADOW));
    }
  };
  match chip {
    legend::Chip::Liquid(liquid) => marked(row, Glyph::Drop, liquid.color(), None),
    legend::Chip::Made(power) => marked(row, Glyph::Thunder, MADE, Some(watts(power))),
    legend::Chip::Drawn(power) => marked(row, Glyph::Thunder, DRAWN, Some(watts(power))),
    legend::Chip::Stored(energy) => {
      marked(row, Glyph::Thunder, STORED, Some(watts(energy)))
    }
    legend::Chip::Fuel(seconds) => {
      marked(row, Glyph::Flame, FLAME, seconds.map(|seconds| format!("{seconds:.0}s")))
    }
    legend::Chip::Sign(glyph) => marked(row, glyph, TAGLINE, None),
    legend::Chip::Item(block) => {
      row.spawn(icon(block, 22.0, palette));
    }
  }
}

fn arrow(row: &mut ChildSpawnerCommands) { row.spawn((words(">", 14.0, ARROW), SHADOW)); }

fn strip(card: &mut ChildSpawnerCommands, fill: impl FnOnce(&mut ChildSpawnerCommands)) {
  card
    .spawn(Node {
      align_items: AlignItems::Center,
      column_gap: px(3),
      flex_wrap: FlexWrap::Wrap,
      ..default()
    })
    .with_children(fill);
}

fn summary(
  card: &mut ChildSpawnerCommands,
  stack: Stack,
  line: String,
  known: &[Block],
  palette: &Palette,
  glyphs: &Glyphs
) {
  let block = stack.block;
  card.spawn((words(&line, 15.0, INK), SHADOW));
  if let Some(tagline) = legend::tagline(block) {
    card.spawn((words(tagline, 13.0, TAGLINE), SHADOW));
  }
  let (inputs, outputs) = legend::flow(block);
  if !inputs.is_empty() || !outputs.is_empty() {
    strip(card, |row| {
      for &input in inputs.iter() {
        chip(row, input, palette, glyphs)
      }
      arrow(row);
      row.spawn(icon(block, 22.0, palette));
      if !outputs.is_empty() {
        arrow(row);
        for &output in outputs.iter() {
          chip(row, output, palette, glyphs)
        }
      }
    });
  }
  let feeds = legend::feeds(block);
  if !feeds.is_empty() {
    strip(card, |row| {
      row.spawn(icon(block, 22.0, palette));
      arrow(row);
      for &feed in feeds.iter() {
        chip(row, feed, palette, glyphs)
      }
    });
  }
  for making in legend::makes(block, known).into_iter().take(5) {
    strip(card, |row| {
      match making.station {
        Some(station) => {
          row.spawn(icon(station, 22.0, palette));
        }
        None => {
          row.spawn(glyph_image(glyphs, Glyph::Hand, ARROW, 18.0));
        }
      }
      match making.price {
        legend::Price::Mana => {
          chip(row, legend::Chip::Liquid(Liquid::Mana), palette, glyphs)
        }
        legend::Price::Lifeblood => {
          chip(row, legend::Chip::Liquid(Liquid::Lifeblood), palette, glyphs)
        }
        legend::Price::Essence => {
          row.spawn(glyph_image(glyphs, Glyph::Rune, Color::srgb(0.75, 0.55, 1.0), 18.0));
        }
        legend::Price::Chance => {
          row.spawn(glyph_image(glyphs, Glyph::Star, ARROW, 18.0));
        }
        legend::Price::Free => ()
      }
      arrow(row);
      for &product in making.products.iter().take(MOST_SHOWN) {
        row.spawn(icon(product, 22.0, palette));
      }
      if making.products.len() > MOST_SHOWN {
        row.spawn((
          words(&format!("+{}", making.products.len() - MOST_SHOWN), 13.0, ARROW),
          SHADOW
        ));
      }
    });
  }
}

fn hint(
  (menu, pilot, inventories, coffer): (
    Res<Menu>,
    Option<Res<Pilot>>,
    Query<&Inventory>,
    Res<Coffer>
  ),
  (hovered, keys, focus): (
    Query<(&Interaction, &Act)>,
    Res<ButtonInput<KeyCode>>,
    Res<Focus>
  ),
  (windows, scale): (Query<&Window, With<PrimaryWindow>>, Res<UiScale>),
  (palette, glyphs, skills): (Res<Palette>, Option<Res<Glyphs>>, Query<&Skills>),
  mut marks: MessageWriter<Mark>,
  mut tips: Query<(Entity, &mut Node, &mut Visibility, &ComputedNode), With<Tooltip>>,
  mut shown: Local<Option<Stack>>,
  mut commands: Commands
) {
  let inventory = carried(&pilot, &inventories);
  let pointed = hovered
    .iter()
    .filter(|(interaction, _)| **interaction != Interaction::None)
    .find_map(|(_, act)| match *act {
      Act::Inspect(block) => Some(Stack::of(block, 0)),
      Act::Slot(index) => inventory.slots[usize::from(index)],
      Act::Stash(index) => Place::Chest(usize::from(index)).held(&inventory, &coffer),
      _ => None
    })
    .filter(|_| handling(&menu));
  if let Some(Stack { block, .. }) = pointed
    && keys.just_pressed(KeyCode::KeyA)
    && focus.0.is_none()
  {
    marks.write(Mark(block));
  }
  let cursor =
    windows.single().ok().and_then(Window::cursor_position).map(|at| at / scale.0);
  let room = windows.single().map_or(Vec2::splat(f32::MAX), |window| {
    Vec2::new(window.width(), window.height()) / scale.0
  });
  for (tip, mut node, mut visibility, computed) in tips.iter_mut() {
    match pointed.zip(cursor) {
      Some((stack, at)) => {
        let size = computed.size * computed.inverse_scale_factor;
        let left = match at.x + 16.0 + size.x > room.x {
          true => (at.x - 12.0 - size.x).max(0.0),
          false => at.x + 16.0
        };
        let top = (at.y + 12.0).min((room.y - size.y).max(0.0));
        node.left = px(left);
        node.top = px(top);
        visibility.set_if_neq(Visibility::Inherited);
        if *shown != Some(stack)
          && let Some(glyphs) = glyphs.as_deref()
        {
          *shown = Some(stack);
          let line = note(stack);
          let known = known(&pilot, &skills);
          commands
            .entity(tip)
            .despawn_children()
            .with_children(|tip| summary(tip, stack, line, &known, &palette, glyphs));
        }
      }
      None => {
        *shown = None;
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
  let stack =
    self::carried(&pilot, &inventories).slots[CURSOR].filter(|_| handling(&menu));
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
            && image.rect != Some(stack_rect(&stack))
          {
            image.rect = Some(stack_rect(&stack))
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

fn equip(inventory: &Inventory, index: usize) -> Option<Shuffle> {
  match index >= WEARING {
    true => (HOTBAR..SLOTS)
      .chain(0..HOTBAR)
      .find(|&free| inventory.slots[free].is_none())
      .map(|free| Shuffle { from: index as u8, to: free as u8 }),
    false => inventory.slots[index]
      .and_then(|stack| Wear::of(stack.block))
      .and_then(|wear| Wear::ALL.iter().position(|&worn| worn == wear))
      .map(|worn| Shuffle { from: index as u8, to: (WEARING + worn) as u8 })
  }
}

fn obey(
  mut pressed: MessageReader<Pressed>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  (menu, coffer, keys): (Res<Menu>, Res<Coffer>, Res<ButtonInput<KeyCode>>),
  buttons: Res<ButtonInput<MouseButton>>,
  hovered: Query<(&Interaction, &Act)>,
  mut chosen: ResMut<Chosen>,
  mut rummages: MessageWriter<Rummage>,
  mut shuffles: MessageWriter<Shuffle>,
  mut picks: MessageWriter<Picked>,
  mut crafts: MessageWriter<Craft>,
  mut sounds: MessageWriter<crate::audio::Sound>
) {
  let inventory = carried(&pilot, &inventories);
  let holding = inventory.slots[CURSOR].is_some();
  let chest = coffer.0.at;
  let handled = handling(&menu);
  if buttons.just_pressed(MouseButton::Right)
    && handled
    && let Some(&act) = hovered
      .iter()
      .find(|(interaction, _)| **interaction != Interaction::None)
      .map(|(_, act)| act)
  {
    match act {
      Act::Slot(index) => {
        picks.write(Picked(if holding { Pick::One(index) } else { Pick::Half(index) }));
      }
      Act::Stash(index) => {
        rummages.write(Rummage {
          at: chest,
          handling: if holding { Handling::One(index) } else { Handling::Half(index) }
        });
      }
      _ => ()
    }
  }
  let shifted = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) && !holding;
  for &Pressed(act) in pressed.read().filter(|_| handled) {
    match act {
      Act::Slot(index) if shifted && menu.showing(Tab::Chest) => {
        rummages.write(Rummage { at: chest, handling: Handling::Deposit(index) });
      }
      Act::Slot(index)
        if shifted && let Some(shuffle) = equip(&inventory, usize::from(index)) =>
      {
        shuffles.write(shuffle);
      }
      Act::Slot(index) => {
        let (from, to) =
          if holding { (CURSOR as u8, index) } else { (index, CURSOR as u8) };
        shuffles.write(Shuffle { from, to });
      }
      Act::Stash(index) => {
        rummages.write(Rummage {
          at: chest,
          handling: match shifted {
            true => Handling::Withdraw(index),
            false => Handling::Swap(index)
          }
        });
      }
      Act::Inspect(block) => chosen.0 = Some(block),
      Act::Equipment => chosen.0 = None,
      Act::Craft(index) => {
        crafts.write(Craft(index));
        sounds.write(crate::audio::Sound::here(crate::audio::Cue::Craft));
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
        (obey, survey, fill, carry, catalogue, recipes, rewind, scroll, hint, fit_hoard)
          .chain()
          .run_if(plays)
      );
  }
}
