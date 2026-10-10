use {crate::{island::Island,
             menu::{Act, BUTTON, EDGE, FAINT, INK, Menu, Pressed, Shaded, Tab, words},
             player::Pilot,
             protocol::{Teleport, Travel, Visited, plays},
             voxels::Voxels},
     bevy::prelude::*,
     bevy_replicon::prelude::*};

#[derive(Component)]
struct Destinations;

#[derive(Component)]
struct Whereabouts;

pub fn page(page: &mut ChildSpawnerCommands) {
  page.spawn((Whereabouts, words("", 17.0, INK)));
  page.spawn((Destinations, Node {
    flex_wrap: FlexWrap::Wrap,
    column_gap: px(6),
    row_gap: px(6),
    ..default()
  }));
  page.spawn(words(
    "Every island has a waystone. Right-click one to remember its island and to travel \
     to an island you remember.",
    14.0,
    FAINT
  ));
}

fn heading(offset: Vec2) -> &'static str {
  let (east, south) = (offset.x, offset.y);
  let compass = |main: f32, cross: f32| cross.abs() < main.abs() * 0.414;
  match () {
    () if compass(south, east) && south < 0.0 => "N",
    () if compass(south, east) => "S",
    () if compass(east, south) && east > 0.0 => "E",
    () if compass(east, south) => "W",
    () if south < 0.0 && east > 0.0 => "NE",
    () if south < 0.0 => "NW",
    () if east > 0.0 => "SE",
    () => "SW"
  }
}

fn list(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  voxels: Option<Res<Voxels>>,
  visits: Query<&Visited>,
  destinations: Query<Entity, With<Destinations>>,
  mut whereabouts: Query<&mut Text, With<Whereabouts>>,
  mut commands: Commands,
  mut shown: Local<Vec<(IVec2, String)>>
) {
  if menu.showing(Tab::Waystones)
    && let Some(pilot) = pilot
    && let Some(voxels) = voxels
  {
    let seed = voxels.seed;
    let here = Island::beside(seed, pilot.at);
    let visited =
      visits.get(pilot.me).map(|visited| visited.0.clone()).unwrap_or_default();
    let line = match here {
      Some(island) => format!("Waystone of {}. Where to?", island.name(seed)),
      None => match visited.is_empty() {
        true => "Right-click a waystone to remember its island.".into(),
        false => "Stand by a waystone to travel.".into()
      }
    };
    whereabouts.iter_mut().for_each(|mut text| {
      if text.0 != line {
        text.0 = line.clone()
      }
    });
    let labels: Vec<(IVec2, String)> = visited
      .iter()
      .filter(|&&cell| here.is_none_or(|here| here.cell != cell))
      .filter_map(|&cell| Island::at(seed, cell))
      .map(|island| {
        let offset = island.stone.xz().as_vec2() - pilot.at.xz();
        let distance = (offset.length() / 10.0).round() as i32 * 10;
        (
          island.cell,
          format!(
            "{}\n{}, {distance} m {}",
            island.name(seed),
            island.kind.describe(),
            heading(offset)
          )
        )
      })
      .collect();
    if *shown != labels {
      destinations.iter().for_each(|container| {
        commands.entity(container).despawn_children().with_children(|list| {
          labels.iter().for_each(|(cell, label)| {
            list
              .spawn((
                Button,
                Act::Travel(*cell),
                Shaded,
                Node {
                  width: px(270),
                  padding: UiRect::axes(px(8), px(4)),
                  border: UiRect::all(px(2)),
                  ..default()
                },
                BorderColor::all(EDGE),
                BackgroundColor(BUTTON)
              ))
              .with_child(words(label.clone(), 15.0, INK));
          })
        });
      });
      *shown = labels
    }
  }
}

fn obey(
  mut pressed: MessageReader<Pressed>,
  pilot: Option<Res<Pilot>>,
  voxels: Option<Res<Voxels>>,
  mut travels: MessageWriter<Travel>
) {
  pressed.read().for_each(|&Pressed(act)| {
    if let Act::Travel(cell) = act
      && let Some(pilot) = &pilot
      && let Some(voxels) = &voxels
      && Island::beside(voxels.seed, pilot.at).is_some()
    {
      travels.write(Travel(cell));
    }
  })
}

fn arrive(
  mut teleports: MessageReader<Teleport>,
  pilot: Option<ResMut<Pilot>>,
  mut menu: ResMut<Menu>
) {
  if let Some(mut pilot) = pilot {
    teleports.read().for_each(|&Teleport(avatar)| {
      pilot.at = avatar.at;
      pilot.yaw = avatar.yaw;
      pilot.pitch = avatar.pitch;
      pilot.velocity = Vec3::ZERO;
      menu.open = false
    })
  }
}

pub struct Waystones;

impl Plugin for Waystones {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        PreUpdate,
        arrive.after(ClientSystems::Receive).after(ServerSystems::Receive).run_if(plays)
      )
      .add_systems(Update, (list, obey).run_if(plays));
  }
}
