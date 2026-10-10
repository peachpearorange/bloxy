use {crate::{beast,
             bird::{self, Bird},
             folk,
             hud::SHADE,
             menu::Menu,
             plate::{Plate, plate},
             player::{Eye, Pilot},
             protocol::{Band, Beast, Breed, Folk, Health, plays},
             voxels::Voxels},
     bevy::{platform::collections::HashMap, prelude::*}};

const SIGHT: f32 = 24.0;
const RISE: f32 = 0.9;
const RISE_FOR: f32 = 0.9;
const RECENT: f32 = 2.0;
const WOUND: Color = Color::srgb(1.0, 0.25, 0.2);
const HOSTILE: Color = Color::srgb(1.0, 0.45, 0.4);
const TAME: Color = Color::srgb(0.95, 0.95, 0.95);

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
  scale: Res<UiScale>,
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
        .and_then(|(camera, eye)| camera.world_to_viewport(eye, lifted).ok())
        .map(|spot| spot / scale.0);
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

#[derive(Resource, Default)]
struct Plated(HashMap<Entity, Entity>);

fn target(
  time: Res<Time>,
  pilot: Res<Pilot>,
  menu: Res<Menu>,
  voxels: Option<Res<Voxels>>,
  wounds: Res<Wounds>,
  creatures: Query<(Entity, &Health, Option<&Folk>, Option<&Beast>, Option<&Bird>)>,
  mut plated: ResMut<Plated>,
  mut plates: Query<&mut Plate>,
  mut commands: Commands
) {
  let (from, toward) = (pilot.eye(), pilot.facing() * Vec3::NEG_Z);
  let blocked = voxels
    .and_then(|voxels| voxels.cast(from, toward, SIGHT))
    .map_or(SIGHT, |hit| (hit.at.as_vec3() + 0.5).distance(from));
  let seen = creatures
    .iter()
    .filter_map(|(entity, _, folk, beast, bird)| {
      let near = match (folk, beast, bird) {
        (Some(folk), ..) => folk::struck(folk.at, from, toward),
        (_, Some(beast), _) => beast::struck(beast, from, toward),
        (.., Some(bird)) => bird::struck(bird, from, toward),
        _ => None
      };
      near.filter(|&near| near <= blocked).map(|near| (near, entity))
    })
    .min_by(|a, b| a.0.total_cmp(&b.0))
    .map(|(_, entity)| entity)
    .filter(|_| !menu.open);
  let now = time.elapsed_secs();
  let wanted: HashMap<Entity, (u8, Sighted)> = creatures
    .iter()
    .filter(|(entity, ..)| {
      seen == Some(*entity)
        || wounds.0.get(entity).is_some_and(|&(_, when, _)| now - when < RECENT)
    })
    .filter_map(|(entity, health, folk, beast, bird)| {
      sighted(folk, beast, bird).map(|sighted| (entity, (health.0, sighted)))
    })
    .collect();
  plated.0.retain(|creature, plate| {
    let kept = wanted.contains_key(creature);
    if !kept {
      commands.entity(*plate).despawn()
    }
    kept
  });
  wanted.into_iter().for_each(
    |(creature, (health, Sighted { name, top, full, hostile }))| {
      let color = if hostile { HOSTILE } else { TAME };
      let share = (f32::from(health) / f32::from(full.max(1))).clamp(0.0, 1.0);
      let at = top + Vec3::Y * 0.15;
      match plated.0.get(&creature).and_then(|&plate| plates.get_mut(plate).ok()) {
        Some(mut plate) => {
          plate.at = at;
          if plate.health != Some(share) {
            plate.health = Some(share)
          }
        }
        None => {
          let plate = commands
            .spawn(plate(name, color, at))
            .insert(Plate { text: name.into(), color, health: Some(share), at })
            .id();
          plated.0.insert(creature, plate);
        }
      }
    }
  )
}

const BLINK: f32 = 0.35;
const FLUSH: Color = Color::srgb(1.0, 0.1, 0.08);

#[derive(Component)]
struct Blushing(Handle<StandardMaterial>);

#[derive(Resource, Default)]
struct Reddened(HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>);

fn blush(
  time: Res<Time>,
  wounds: Res<Wounds>,
  creatures: Query<Entity, Or<(With<Beast>, With<Bird>)>>,
  family: Query<&Children>,
  mut meshes: Query<(&mut MeshMaterial3d<StandardMaterial>, Option<&Blushing>)>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut reddened: ResMut<Reddened>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  creatures.iter().for_each(|creature| {
    let hurt = wounds.0.get(&creature).is_some_and(|&(_, when, _)| now - when < BLINK);
    family.iter_descendants(creature).for_each(|part| {
      if let Ok((mut material, blushing)) = meshes.get_mut(part) {
        match (hurt, blushing) {
          (true, None) => {
            let original = material.0.clone();
            let red = reddened
              .0
              .entry(original.id())
              .or_insert_with(|| {
                let mut red = materials.get(&original).cloned().unwrap_or_default();
                red.base_color = red.base_color.mix(&FLUSH, 0.7);
                materials.add(red)
              })
              .clone();
            material.0 = red;
            commands.entity(part).insert(Blushing(original));
          }
          (false, Some(Blushing(original))) => {
            material.0 = original.clone();
            commands.entity(part).remove::<Blushing>();
          }
          _ => ()
        }
      }
    })
  })
}

pub struct Glances;

impl Plugin for Glances {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Wounds>()
      .init_resource::<Plated>()
      .init_resource::<Reddened>()
      .add_observer(fell)
      .add_systems(
        Update,
        (tally, target.run_if(resource_exists::<Pilot>), rise, blush)
          .chain()
          .run_if(plays)
      );
  }
}
