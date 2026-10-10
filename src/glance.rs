use {crate::{beast,
             bird::{self, Bird},
             folk,
             hud::SHADE,
             menu::Menu,
             player::{Eye, Pilot},
             protocol::{Band, Beast, Breed, Folk, Health, plays},
             voxels::Voxels},
     bevy::{platform::collections::HashMap, prelude::*}};

const SIGHT: f32 = 24.0;
const RISE: f32 = 0.9;
const RISE_FOR: f32 = 0.9;
const RECENT: f32 = 2.0;
const BAR: f32 = 160.0;
const WOUND: Color = Color::srgb(1.0, 0.25, 0.2);
const HOSTILE: Color = Color::srgb(1.0, 0.45, 0.4);
const TAME: Color = Color::srgb(0.95, 0.95, 0.95);
const FULL: Color = Color::srgb(0.35, 0.85, 0.35);
const LOW: Color = Color::srgb(0.9, 0.25, 0.2);

struct Sighted {
  name: &'static str,
  top: Vec3,
  full: u8,
  hostile: bool
}

fn sighted(
  folk: Option<&Folk>,
  beast: Option<&Beast>,
  bird: Option<&Bird>
) -> Option<Sighted> {
  let person = folk.map(|folk| Sighted {
    name: match folk.band {
      Band::Trader => "Trader",
      Band::Pirate => "Pirate",
      Band::Viking => "Viking"
    },
    top: folk.at + Vec3::Y * 2.1,
    full: folk.band.health(),
    hostile: folk.band.hostile()
  });
  let animal = beast.map(|beast| Sighted {
    name: match beast.breed {
      Breed::Sheep(_) => "Sheep",
      Breed::Lizard => "Water Lizard"
    },
    top: beast.at + Vec3::Y * 1.4,
    full: beast.breed.health(),
    hostile: false
  });
  let fowl = bird.map(|bird| Sighted {
    name: match bird.breed {
      bird::Breed::Swan => "Swan",
      bird::Breed::Raven => "Raven",
      bird::Breed::Gull => "Gull",
      bird::Breed::Parrot => "Parrot"
    },
    top: bird.at + Vec3::Y * 0.8,
    full: bird::Breed::HEALTH,
    hostile: false
  });
  person.or(animal).or(fowl)
}

#[derive(Resource, Default)]
struct Wounds(HashMap<Entity, (u8, f32, Vec3)>);

#[derive(Component)]
struct Rising {
  at: Vec3,
  age: f32
}

#[derive(Component)]
struct Target;

#[derive(Component)]
struct TargetName;

#[derive(Component)]
struct TargetFill;

fn rising(commands: &mut Commands, amount: u8, at: Vec3) {
  commands.spawn((
    Rising { at, age: 0.0 },
    Text::new(format!("-{amount}")),
    TextFont { font_size: FontSize::Px(20.0), ..default() },
    TextColor(WOUND),
    SHADE,
    Node { position_type: PositionType::Absolute, ..default() },
    Visibility::Hidden
  ));
}

fn tally(
  time: Res<Time>,
  creatures: Query<(Entity, Ref<Health>, Option<&Folk>, Option<&Beast>, Option<&Bird>)>,
  mut wounds: ResMut<Wounds>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  creatures.iter().for_each(|(entity, health, folk, beast, bird)| {
    if let Some(Sighted { top, .. }) = sighted(folk, beast, bird) {
      let before = wounds.0.get(&entity).copied();
      let struck = before.is_some_and(|(was, ..)| health.0 < was);
      if let Some((was, ..)) = before
        && struck
      {
        rising(&mut commands, was - health.0, top)
      }
      let when = match struck {
        true => now,
        false => before.map_or(f32::NEG_INFINITY, |(_, when, _)| when)
      };
      wounds.0.insert(entity, (health.0, when, top));
    }
  })
}

fn fell(
  gone: On<Remove, Health>,
  time: Res<Time>,
  mut wounds: ResMut<Wounds>,
  mut commands: Commands
) {
  if let Some((left, when, top)) = wounds.0.remove(&gone.entity)
    && left > 0
    && time.elapsed_secs() - when < RECENT
  {
    rising(&mut commands, left, top)
  }
}

fn rise(
  time: Res<Time>,
  eyes: Query<(&Camera, &GlobalTransform), With<Eye>>,
  mut numbers: Query<(Entity, &mut Rising, &mut Node, &mut TextColor, &mut Visibility)>,
  mut commands: Commands
) {
  let dt = time.delta_secs();
  numbers.iter_mut().for_each(
    |(entity, mut rising, mut node, mut color, mut visibility)| {
      rising.age += dt;
      let lifted = rising.at + Vec3::Y * RISE * rising.age / RISE_FOR;
      let spot = eyes
        .single()
        .ok()
        .and_then(|(camera, eye)| camera.world_to_viewport(eye, lifted).ok());
      match (rising.age > RISE_FOR, spot) {
        (true, _) => commands.entity(entity).despawn(),
        (false, Some(spot)) => {
          node.left = px(spot.x - 8.0);
          node.top = px(spot.y - 10.0);
          color.0.set_alpha(1.0 - (rising.age / RISE_FOR).powi(2));
          *visibility = Visibility::Inherited
        }
        (false, None) => *visibility = Visibility::Hidden
      }
    }
  )
}

fn panel(mut commands: Commands) {
  commands
    .spawn((
      Target,
      Node {
        width: percent(100),
        position_type: PositionType::Absolute,
        top: px(8),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(3),
        ..default()
      },
      Visibility::Hidden
    ))
    .with_children(|target| {
      target.spawn((
        TargetName,
        Text::new(""),
        TextFont { font_size: FontSize::Px(16.0), ..default() },
        TextColor(TAME),
        SHADE
      ));
      target
        .spawn((
          Node { width: px(BAR), height: px(6), ..default() },
          BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.7))
        ))
        .with_child((
          TargetFill,
          Node { width: percent(100), height: percent(100), ..default() },
          BackgroundColor(FULL)
        ));
    });
}

fn target(
  pilot: Res<Pilot>,
  menu: Res<Menu>,
  voxels: Option<Res<Voxels>>,
  creatures: Query<(&Health, Option<&Folk>, Option<&Beast>, Option<&Bird>)>,
  mut panels: Query<&mut Visibility, With<Target>>,
  mut names: Query<(&mut Text, &mut TextColor), With<TargetName>>,
  mut fills: Query<(&mut Node, &mut BackgroundColor), With<TargetFill>>
) {
  let (from, toward) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
  let blocked = voxels
    .and_then(|voxels| voxels.cast(from, toward, SIGHT))
    .map_or(SIGHT, |hit| (hit.at.as_vec3() + 0.5).distance(from));
  let seen = creatures
    .iter()
    .filter_map(|(health, folk, beast, bird)| {
      let near = match (folk, beast, bird) {
        (Some(folk), ..) => folk::struck(folk.at, from, toward),
        (_, Some(beast), _) => beast::struck(beast, from, toward),
        (.., Some(bird)) => bird::struck(bird, from, toward),
        _ => None
      };
      near
        .filter(|&near| near <= blocked)
        .zip(sighted(folk, beast, bird).map(|sighted| (health.0, sighted)))
    })
    .min_by(|a, b| a.0.total_cmp(&b.0))
    .map(|(_, seen)| seen)
    .filter(|_| !menu.open);
  panels.iter_mut().for_each(|mut visibility| {
    visibility.set_if_neq(match seen {
      Some(_) => Visibility::Inherited,
      None => Visibility::Hidden
    });
  });
  if let Some((health, Sighted { name, full, hostile, .. })) = seen {
    names.iter_mut().for_each(|(mut text, mut color)| {
      if text.0 != name {
        text.0 = name.to_string()
      }
      color.set_if_neq(TextColor(if hostile { HOSTILE } else { TAME }));
    });
    let share = (f32::from(health) / f32::from(full.max(1))).clamp(0.0, 1.0);
    fills.iter_mut().for_each(|(mut node, mut fill)| {
      node.width = percent(share * 100.0);
      fill.set_if_neq(BackgroundColor(LOW.mix(&FULL, share)));
    })
  }
}

pub struct Glances;

impl Plugin for Glances {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Wounds>()
      .add_observer(fell)
      .add_systems(Startup, panel.run_if(plays))
      .add_systems(
        Update,
        (tally, target.run_if(resource_exists::<Pilot>), rise).chain().run_if(plays)
      );
  }
}
