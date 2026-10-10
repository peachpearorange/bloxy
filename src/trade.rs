use {crate::{authority::{Controller, player_of},
             block::Block,
             crafting::icon,
             folk::struck,
             menu::{Act, BUTTON, EDGE, FAINT, INK, Menu, Pressed, Shaded, Tab, words},
             noise::hash,
             player::{Pilot, captured},
             protocol::*,
             stream::Palette,
             voxels::Voxels},
     bevy::{prelude::*,
            window::{CursorOptions, PrimaryWindow}},
     bevy_replicon::prelude::*};

const HAIL: f32 = REACH + 3.0;
const CAN: Color = Color::srgb(0.45, 0.85, 0.4);
const SHORT: Color = Color::srgb(0.95, 0.42, 0.36);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Offer {
  pub give: (Block, u16),
  pub get: (Block, u16)
}

const fn offer(give: (Block, u16), get: (Block, u16)) -> Offer { Offer { give, get } }

const OFFERS: [Offer; 14] = [
  offer((Block::Log, 8), (Block::IronOre, 2)),
  offer((Block::Cobblestone, 16), (Block::GoldOre, 1)),
  offer((Block::IronOre, 4), (Block::DiamondOre, 1)),
  offer((Block::GoldOre, 2), (Block::PalmSapling, 3)),
  offer((Block::GoldOre, 2), (Block::SpruceSapling, 3)),
  offer((Block::GoldOre, 2), (Block::BirchSapling, 3)),
  offer((Block::GoldOre, 1), (Block::Ice, 8)),
  offer((Block::GoldOre, 1), (Block::Clay, 12)),
  offer((Block::CopperOre, 3), (Block::Lamp, 4)),
  offer((Block::GoldOre, 2), (Block::Boat, 1)),
  offer((Block::CoalOre, 4), (Block::Bucket, 1)),
  offer((Block::Planks, 12), (Block::CoalOre, 4)),
  offer((Block::Sand, 16), (Block::Glass, 8)),
  offer((Block::Dirt, 32), (Block::Grass, 8))
];

pub fn offers(luck: u32) -> Vec<Offer> {
  let mut ranked: Vec<(u32, Offer)> = OFFERS
    .iter()
    .enumerate()
    .map(|(index, &offer)| (hash(luck, index as i32, 0x7, 0x3), offer))
    .collect();
  ranked.sort_by_key(|&(rank, _)| rank);
  ranked.into_iter().take(4).map(|(_, offer)| offer).collect()
}

fn barter(inventory: &Inventory, offer: Offer) -> Option<Inventory> {
  let mut after = inventory.clone();
  let ((give, price), (get, amount)) = (offer.give, offer.get);
  ((0..price).all(|_| after.take(give)) && (0..amount).all(|_| after.add(get)))
    .then_some(after)
}

fn trade(
  mut trades: MessageReader<FromClient<Trade>>,
  mut players: Query<(&Controller, (&Avatar, &mut Inventory))>,
  folk: Query<&Folk>
) {
  for &FromClient { client_id, message: Trade { trader, offer } } in trades.read() {
    if let Some((avatar, mut inventory)) = player_of(players.iter_mut(), client_id)
      && let Ok(folk) = folk.get(trader)
      && folk.band == Band::Trader
      && (avatar.at + Vec3::Y * EYE).distance(folk.at + Vec3::Y) <= HAIL + 1.0
      && let Some(&offer) = offers(folk.luck).get(usize::from(offer))
      && let Some(after) = barter(&inventory, offer)
    {
      *inventory = after
    }
  }
}

#[derive(Resource, Default)]
struct Dealing(Option<Entity>);

#[derive(Component)]
struct Bargains;

#[derive(Component)]
struct Greeting;

pub fn page(page: &mut ChildSpawnerCommands) {
  page.spawn((Greeting, words("", 17.0, INK)));
  page.spawn((Bargains, Node {
    flex_direction: FlexDirection::Column,
    row_gap: px(6),
    ..default()
  }));
  page.spawn(words(
    "Traders sail between the islands. Sail or swim close and right-click one to trade.",
    14.0,
    FAINT
  ));
}

fn hail(
  time: Res<Time>,
  buttons: Res<ButtonInput<MouseButton>>,
  cursor: Query<&CursorOptions, With<PrimaryWindow>>,
  mut menu: ResMut<Menu>,
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  folk: Query<(Entity, &Folk)>,
  mut dealing: ResMut<Dealing>
) {
  if let Some(voxels) = voxels
    && captured(&cursor, &menu)
    && buttons.just_pressed(MouseButton::Right)
  {
    let (from, toward) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
    let blocked = voxels
      .cast(from, toward, HAIL)
      .map_or(HAIL, |hit| (hit.at.as_vec3() + 0.5).distance(from));
    if let Some((trader, _)) = folk
      .iter()
      .filter(|(_, folk)| folk.band == Band::Trader)
      .filter_map(|(entity, folk)| {
        struck(folk.at, from, toward).map(|near| (entity, near))
      })
      .filter(|&(_, near)| near <= blocked)
      .min_by(|a, b| a.1.total_cmp(&b.1))
    {
      dealing.0 = Some(trader);
      menu.show(Tab::Trade, time.elapsed_secs())
    }
  }
}

fn row(
  parent: &mut ChildSpawnerCommands,
  index: usize,
  offer: Offer,
  have: u32,
  palette: &Palette
) {
  let ((give, price), (get, amount)) = (offer.give, offer.get);
  let affordable = have >= u32::from(price);
  parent
    .spawn(Node { align_items: AlignItems::Center, column_gap: px(6), ..default() })
    .with_children(|row| {
      row.spawn(icon(give, 34.0, palette));
      row.spawn((
        words(
          format!("{price} {} (have {have})", give.name()),
          16.0,
          if affordable { CAN } else { SHORT }
        ),
        Node { width: px(300), ..default() }
      ));
      row.spawn(words("for", 16.0, FAINT));
      row.spawn(icon(get, 34.0, palette));
      row.spawn((words(format!("{amount} {}", get.name()), 16.0, INK), Node {
        width: px(220),
        ..default()
      }));
      row
        .spawn((
          Button,
          Act::Trade(index as u8),
          Shaded,
          Node {
            padding: UiRect::axes(px(8), px(3)),
            border: UiRect::all(px(2)),
            ..default()
          },
          BorderColor::all(if affordable { CAN } else { EDGE }),
          BackgroundColor(BUTTON)
        ))
        .with_child(words("Trade", 16.0, INK));
    });
}

fn haggle(
  menu: Res<Menu>,
  dealing: Res<Dealing>,
  palette: Res<Palette>,
  pilot: Option<Res<Pilot>>,
  folk: Query<&Folk>,
  inventories: Query<&Inventory>,
  bargains: Query<Entity, With<Bargains>>,
  mut greeting: Query<&mut Text, With<Greeting>>,
  mut commands: Commands,
  mut shown: Local<Option<(Option<Entity>, Vec<u32>)>>
) {
  if menu.showing(Tab::Trade)
    && let Some(pilot) = pilot
  {
    let trader = dealing
      .0
      .and_then(|entity| folk.get(entity).ok().map(|folk| (entity, folk)))
      .filter(|(_, folk)| folk.at.distance(pilot.at) < HAIL * 3.0);
    let inventory = inventories.get(pilot.me).cloned().unwrap_or_default();
    let deals = trader.map(|(_, folk)| offers(folk.luck)).unwrap_or_default();
    let haves: Vec<u32> =
      deals.iter().map(|offer| inventory.count(offer.give.0)).collect();
    let key = (trader.map(|(entity, _)| entity), haves.clone());
    let line = match trader {
      Some(_) => "A trader. Take a look at my wares!",
      None => "The trader has sailed on."
    };
    for mut text in greeting.iter_mut() {
      if text.0 != line {
        text.0 = line.into()
      }
    }
    if shown.as_ref() != Some(&key) {
      *shown = Some(key);
      for list in bargains.iter() {
        commands.entity(list).despawn_related::<Children>().with_children(|list| {
          for (index, (&offer, &have)) in deals.iter().zip(&haves).enumerate() {
            row(list, index, offer, have, &palette)
          }
        });
      }
    }
  } else {
    *shown = None
  }
}

fn obey(
  mut pressed: MessageReader<Pressed>,
  dealing: Res<Dealing>,
  mut trades: MessageWriter<Trade>
) {
  for &Pressed(act) in pressed.read() {
    if let Act::Trade(offer) = act
      && let Some(trader) = dealing.0
    {
      trades.write(Trade { trader, offer });
    }
  }
}

pub struct Trading;

impl Plugin for Trading {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Dealing>()
      .add_systems(PreUpdate, trade.after(ServerSystems::Receive).run_if(authority))
      .add_systems(
        Update,
        (hail.run_if(resource_exists::<Pilot>), haggle, obey).chain().run_if(plays)
      );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn traders_offer_four_distinct_deals_that_work() {
    let deals = offers(12345);
    assert_eq!(deals.len(), 4);
    assert!(deals.iter().enumerate().all(|(index, deal)| !deals[..index].contains(deal)));
    let deal = deals[0];
    let mut inventory = Inventory::default();
    assert!(barter(&inventory, deal).is_none());
    for _ in 0..deal.give.1 {
      inventory.add(deal.give.0);
    }
    let after = barter(&inventory, deal).expect("affordable");
    assert_eq!(after.count(deal.give.0), 0);
    assert_eq!(after.count(deal.get.0), u32::from(deal.get.1));
  }
}
