use {crate::{block::Block,
             hud::icon_rect,
             menu::{Act, BUTTON, EDGE, FAINT, INK, LIT, Menu, Pressed, Shaded, Tab,
                    button, words},
             player::Pilot,
             protocol::{Craft, HOTBAR, Inventory, SLOTS, Shuffle, plays},
             recipe::{self, Recipe},
             stream::Palette},
     bevy::prelude::*};

const CELL: f32 = 44.0;
const GAP: f32 = 4.0;
const CAN: Color = Color::srgb(0.45, 0.85, 0.4);
const SHORT: Color = Color::srgb(0.95, 0.42, 0.36);
const SHADOW: TextShadow =
  TextShadow { offset: Vec2::splat(1.5), color: Color::srgba(0.0, 0.0, 0.0, 0.7) };

#[derive(Resource, Default)]
struct Held(Option<usize>);

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
struct Hint;

fn icon(block: Block, size: f32, palette: &Palette) -> impl Bundle {
  (
    ImageNode { rect: Some(icon_rect(block)), ..ImageNode::new(palette.atlas.clone()) },
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
    slot.spawn((SlotIcon(index), icon(Block::Stone, 30.0, palette), Visibility::Hidden));
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
    .with_children(|row| slots.for_each(|index| slot(row, index, palette)));
}

pub fn page(page: &mut ChildSpawnerCommands, palette: &Palette) {
  page.spawn(Node { column_gap: px(24), ..default() }).with_children(|columns| {
    columns
      .spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(8),
        width: px(HOTBAR as f32 * (CELL + GAP)),
        flex_shrink: 0.0,
        ..default()
      })
      .with_children(|left| {
        left.spawn((Recipes, Node {
          flex_direction: FlexDirection::Column,
          row_gap: px(8),
          min_height: px(210),
          ..default()
        }));
        left.spawn(words("Backpack", 15.0, FAINT));
        row(left, HOTBAR..SLOTS, palette);
        left.spawn(words("Hotbar", 15.0, FAINT));
        row(left, 0..HOTBAR, palette);
      });
    columns
      .spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(8),
        flex_grow: 1.0,
        ..default()
      })
      .with_children(|right| {
        right.spawn((Hint, words("Items", 15.0, FAINT)));
        right.spawn((Catalogue, Node {
          flex_wrap: FlexWrap::Wrap,
          column_gap: px(GAP),
          row_gap: px(GAP),
          ..default()
        }));
        right.spawn(words(
          "Click an item to see what it is made from and what it makes. Items you can \
           craft now come first, outlined in green. Click two slots to move or swap \
           stacks. E closes.",
          14.0,
          FAINT
        ));
      });
  });
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
  mut held: ResMut<Held>,
  mut slots: Query<(&Act, &mut BorderColor)>,
  mut icons: Query<(&SlotIcon, &mut ImageNode, &mut Visibility)>,
  mut counts: Query<(&SlotCount, &mut Text)>
) {
  if !menu.showing(Tab::Inventory) && held.0.is_some() {
    held.0 = None
  }
  if menu.showing(Tab::Inventory) {
    let inventory = carried(&pilot, &inventories);
    slots.iter_mut().for_each(|(act, mut border)| {
      if let &Act::Slot(index) = act {
        border.set_if_neq(BorderColor::all(match held.0 == Some(index.into()) {
          true => LIT,
          false => EDGE
        }));
      }
    });
    icons.iter_mut().for_each(|(icon, mut image, mut visibility)| {
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
    });
    counts.iter_mut().for_each(|(count, mut text)| {
      let shown = inventory.slots[count.0]
        .filter(|stack| stack.count > 1)
        .map_or(String::new(), |stack| stack.count.to_string());
      if text.0 != shown {
        text.0 = shown
      }
    })
  }
}

fn catalogue(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  chosen: Res<Chosen>,
  palette: Res<Palette>,
  containers: Query<Entity, With<Catalogue>>,
  mut commands: Commands,
  mut shown: Local<Vec<(Block, bool, bool)>>
) {
  if menu.showing(Tab::Inventory) {
    let inventory = carried(&pilot, &inventories);
    let mut entries: Vec<(Block, bool, bool)> = Block::ALL
      .into_iter()
      .filter(|block| block.item())
      .map(|block| (block, recipe::craftable(block, &inventory), chosen.0 == Some(block)))
      .collect();
    entries.sort_by_key(|&(_, craftable, _)| !craftable);
    if *shown != entries {
      containers.iter().for_each(|container| {
        commands.entity(container).despawn_children().with_children(|list| {
          entries.iter().for_each(|&(block, craftable, picked)| {
            let border = match (picked, craftable) {
              (true, _) => LIT,
              (false, true) => CAN,
              (false, false) => EDGE
            };
            list
              .spawn(tile(Act::Inspect(block), border))
              .with_child(icon(block, 30.0, &palette));
          })
        });
      });
      *shown = entries
    }
  }
}

fn card(
  parent: &mut ChildSpawnerCommands,
  index: usize,
  recipe: &Recipe,
  inventory: &Inventory,
  palette: &Palette
) {
  let ingredient = |row: &mut ChildSpawnerCommands, block: Block| {
    row
      .spawn((Button, Act::Inspect(block), Node::default()))
      .with_child(icon(block, 28.0, palette));
  };
  let missing = recipe.missing(inventory);
  parent
    .spawn(Node { align_items: AlignItems::Center, column_gap: px(6), ..default() })
    .with_children(|row| {
      recipe.inputs.iter().for_each(|&(block, need)| {
        let have = inventory.count(block);
        ingredient(row, block);
        row.spawn((
          words(format!("{have}/{need}"), 14.0, match have >= u32::from(need) {
            true => CAN,
            false => SHORT
          }),
          Node { margin: UiRect::right(px(6)), ..default() }
        ));
      });
      row.spawn(words("->", 16.0, FAINT));
      let (output, count) = recipe.output;
      ingredient(row, output);
      row.spawn((words(format!("x{count}"), 14.0, INK), Node {
        margin: UiRect::right(px(6)),
        ..default()
      }));
      if missing.is_empty() {
        button(row, "Craft", Act::Craft(index as u16))
      }
    });
  if !missing.is_empty() {
    let list: Vec<String> =
      missing.iter().map(|&(block, short)| format!("{short} {}", block.name())).collect();
    parent.spawn(words(format!("Missing {}", list.join(", ")), 13.0, SHORT));
  }
}

fn recipes(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  chosen: Res<Chosen>,
  palette: Res<Palette>,
  containers: Query<Entity, With<Recipes>>,
  mut commands: Commands,
  mut shown: Local<Option<(Option<Block>, Inventory)>>
) {
  if menu.showing(Tab::Inventory) {
    let state = (chosen.0, carried(&pilot, &inventories));
    if shown.as_ref() != Some(&state) {
      let (chosen, inventory) = &state;
      containers.iter().for_each(|container| {
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
              making.into_iter().for_each(|(index, recipe)| {
                card(panel, index, recipe, inventory, &palette)
              });
              let using: Vec<_> = recipe::using(block).collect();
              if !using.is_empty() {
                panel.spawn(words("Used in", 15.0, FAINT));
              }
              using.into_iter().for_each(|(index, recipe)| {
                card(panel, index, recipe, inventory, &palette)
              });
            }
          }
        );
      });
      *shown = Some(state)
    }
  }
}

fn hint(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  hovered: Query<(&Interaction, &Act)>,
  mut texts: Query<&mut Text, With<Hint>>
) {
  if menu.showing(Tab::Inventory) {
    let inventory = carried(&pilot, &inventories);
    let line = hovered
      .iter()
      .filter(|(interaction, _)| **interaction != Interaction::None)
      .find_map(|(_, act)| match *act {
        Act::Inspect(block) => Some(block),
        Act::Slot(index) => inventory.slots[usize::from(index)].map(|stack| stack.block),
        _ => None
      })
      .map_or("Items".into(), |block| block.name().to_string());
    texts.iter_mut().for_each(|mut text| {
      if text.0 != line {
        text.0 = line.clone()
      }
    })
  }
}

fn obey(
  mut pressed: MessageReader<Pressed>,
  pilot: Option<Res<Pilot>>,
  inventories: Query<&Inventory>,
  mut held: ResMut<Held>,
  mut chosen: ResMut<Chosen>,
  mut shuffles: MessageWriter<Shuffle>,
  mut crafts: MessageWriter<Craft>
) {
  let inventory = carried(&pilot, &inventories);
  pressed.read().for_each(|&Pressed(act)| match act {
    Act::Slot(index) => {
      let index = usize::from(index);
      held.0 = match held.0 {
        None => inventory.slots[index].map(|_| index),
        Some(from) => {
          if from != index {
            shuffles.write(Shuffle { from: from as u8, to: index as u8 });
          }
          None
        }
      }
    }
    Act::Inspect(block) => chosen.0 = Some(block),
    Act::Craft(index) => {
      crafts.write(Craft(index));
    }
    _ => ()
  })
}

pub struct Crafting;

impl Plugin for Crafting {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Held>()
      .init_resource::<Chosen>()
      .add_systems(Update, (obey, fill, catalogue, recipes, hint).chain().run_if(plays));
  }
}
