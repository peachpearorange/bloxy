use {crate::{block::Block,
             machine::{BUCKET, Design, Liquid, Status, TANK, Unit, bore, design,
                       pooled},
             menu::{FAINT, INK, Menu, Tab, words},
             model::{self, ends},
             protocol::plays,
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            mesh::{Indices, PrimitiveTopology},
            platform::collections::HashMap,
            prelude::*}};

#[derive(Resource, Default)]
pub struct Watching(pub Option<IVec3>);

type Span = (Vec3, Vec3);

fn boxes(spans: &[Span]) -> Mesh {
  let faces = spans.iter().flat_map(|&(low, high)| {
    [
      (Vec3::X, Vec3::NEG_Z, Vec3::Y),
      (Vec3::NEG_X, Vec3::Z, Vec3::Y),
      (Vec3::Y, Vec3::X, Vec3::NEG_Z),
      (Vec3::NEG_Y, Vec3::X, Vec3::Z),
      (Vec3::Z, Vec3::X, Vec3::Y),
      (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y)
    ]
    .map(move |(normal, across, up)| {
      let centre = (low + high) / 2.0;
      let half = (high - low) / 2.0;
      let corner = |a: f32, b: f32| centre + (normal + across * a + up * b) * half;
      (
        [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)],
        normal
      )
    })
  });
  let (positions, normals): (Vec<[f32; 3]>, Vec<[f32; 3]>) = faces
    .flat_map(|(corners, normal)| {
      corners.map(|corner| (corner.to_array(), normal.to_array()))
    })
    .unzip();
  let quads = positions.len() as u32 / 4;
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(
      (0..quads)
        .flat_map(|quad| [0, 1, 2, 0, 2, 3].map(|corner| quad * 4 + corner))
        .collect()
    ))
}

const BORE_LOW: f32 = 6.4 / 16.0;
const BORE_HIGH: f32 = 9.6 / 16.0;
const LEVELS: f32 = 8.0;
const TANK_LEVELS: f32 = 48.0;

fn piped(links: u16, fill: f32) -> Vec<Span> {
  let top = BORE_LOW + (BORE_HIGH - BORE_LOW) * fill;
  let core = (Vec3::splat(BORE_LOW), Vec3::new(BORE_HIGH, top, BORE_HIGH));
  std::iter::once(core)
    .chain((0..6).filter(|index| ends(links) & (1 << index) != 0).map(|index| {
      let axis = index / 2;
      let (mut low, mut high) =
        (Vec3::splat(BORE_LOW), Vec3::new(BORE_HIGH, top, BORE_HIGH));
      if axis == 1 {
        high.y = BORE_HIGH
      }
      match index % 2 {
        0 => {
          low[axis] = BORE_HIGH;
          high[axis] = 1.0
        }
        _ => {
          low[axis] = 0.0;
          high[axis] = BORE_LOW
        }
      }
      (low, high)
    }))
    .collect()
}

fn tanked(fill: f32) -> Vec<Span> {
  vec![(Vec3::new(0.08, 0.125, 0.08), Vec3::new(0.92, 0.125 + 0.75 * fill, 0.92))]
}

#[derive(Resource)]
struct Fittings {
  liquids: HashMap<Liquid, Handle<StandardMaterial>>,
  spills: HashMap<Liquid, Handle<bevy_hanabi::EffectAsset>>,
  meshes: HashMap<(bool, u16, u8), Handle<Mesh>>,
  steel: Handle<StandardMaterial>,
  hazard: Handle<StandardMaterial>,
  blade: Handle<StandardMaterial>,
  beam: Handle<Mesh>,
  rotor: Handle<Mesh>,
  mast: Handle<Mesh>
}

fn furnish(
  mut commands: Commands,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut effects: ResMut<Assets<bevy_hanabi::EffectAsset>>
) {
  let spills = Liquid::ALL
    .into_iter()
    .map(|liquid| {
      let glowing = liquid.glow() > 1.0;
      let color = LinearRgba::from(liquid.color()).to_vec4()
        * if glowing { Vec4::new(6.0, 6.0, 6.0, 1.0) } else { Vec4::ONE };
      (liquid, effects.add(crate::fx::spill(color, glowing)))
    })
    .collect();
  let plain = |color: Color, rough: f32, metal: f32| StandardMaterial {
    base_color: color,
    perceptual_roughness: rough,
    metallic: metal,
    ..default()
  };
  let liquids = Liquid::ALL
    .into_iter()
    .map(|liquid| {
      let color = liquid.color();
      (
        liquid,
        materials.add(StandardMaterial {
          emissive: LinearRgba::from(color) * liquid.glow(),
          reflectance: 0.6,
          ..plain(color, 0.12, 0.0)
        })
      )
    })
    .collect();
  let beam = boxes(&[
    (Vec3::new(-0.62, -0.05, -0.06), Vec3::new(0.82, 0.07, 0.06)),
    (Vec3::new(0.74, -0.32, -0.09), Vec3::new(0.92, 0.12, 0.09)),
    (Vec3::new(-0.74, -0.4, -0.12), Vec3::new(-0.5, 0.02, 0.12)),
    (Vec3::new(-0.06, -0.08, -0.14), Vec3::new(0.06, 0.04, 0.14))
  ]);
  let blades: Vec<Span> = vec![
    (Vec3::new(-0.08, -0.08, -0.12), Vec3::new(0.08, 0.08, 0.06)),
    (Vec3::new(-0.06, 0.06, -0.02), Vec3::new(0.06, 1.35, 0.02)),
  ];
  let rotor = {
    let mut mesh = boxes(&blades);
    for turn in 1..3 {
      mesh
        .merge(&boxes(&blades[1..]).transformed_by(Transform::from_rotation(
          Quat::from_rotation_z(turn as f32 * std::f32::consts::TAU / 3.0)
        )))
        .ok();
    }
    mesh
  };
  let mast = boxes(&[
    (Vec3::new(-0.09, 0.0, -0.09), Vec3::new(0.09, 1.0, 0.09)),
    (Vec3::new(-0.14, 0.95, -0.34), Vec3::new(0.14, 1.2, 0.16))
  ]);
  commands.insert_resource(Fittings {
    liquids,
    spills,
    meshes: HashMap::new(),
    steel: materials.add(plain(Color::srgb(0.6, 0.62, 0.66), 0.45, 0.6)),
    hazard: materials.add(plain(Color::srgb(0.9, 0.7, 0.12), 0.6, 0.1)),
    blade: materials.add(plain(Color::srgb(0.92, 0.92, 0.9), 0.5, 0.0)),
    beam: meshes.add(beam),
    rotor: meshes.add(rotor),
    mast: meshes.add(mast)
  })
}

#[derive(Component)]
struct Filled(Option<(Liquid, u16, u8)>, Option<Entity>);

#[derive(Component)]
struct Beam(f32);

#[derive(Component)]
struct Rotor(f32);

const BEAM_PIVOT: Vec3 = Vec3::new(0.5, 13.0 / 16.0, 0.5);
const MAST_FOOT: Vec3 = Vec3::new(0.5, 1.0, 0.5);
const HUB: Vec3 = Vec3::new(0.0, 1.07, -0.38);

fn mount(
  arrivals: Query<(Entity, &Unit), Added<Unit>>,
  fittings: Option<Res<Fittings>>,
  mut commands: Commands
) {
  if let Some(fittings) = fittings {
    for (entity, unit) in arrivals.iter() {
      commands
        .entity(entity)
        .insert((
          Transform::from_translation(unit.at.as_vec3()),
          Visibility::default(),
          Filled(None, None)
        ))
        .with_children(|parts| match unit.block {
          Block::Pumpjack => {
            parts.spawn((
              Beam(0.0),
              Mesh3d(fittings.beam.clone()),
              MeshMaterial3d(fittings.hazard.clone()),
              Transform::from_translation(BEAM_PIVOT)
            ));
          }
          Block::WindTurbine => {
            parts
              .spawn((
                Mesh3d(fittings.mast.clone()),
                MeshMaterial3d(fittings.steel.clone()),
                Transform::from_translation(MAST_FOOT)
              ))
              .with_child((
                Rotor(0.0),
                Mesh3d(fittings.rotor.clone()),
                MeshMaterial3d(fittings.blade.clone()),
                Transform::from_translation(HUB)
              ));
          }
          _ => ()
        });
    }
  }
}

fn fill(
  voxels: Option<Res<Voxels>>,
  fittings: Option<ResMut<Fittings>>,
  mut units: Query<(Entity, &Unit, &mut Filled)>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut commands: Commands
) {
  if let Some(voxels) = voxels
    && let Some(mut fittings) = fittings
  {
    for (entity, unit, mut filled) in
      units.iter_mut().filter(|(_, unit, _)| pooled(unit.block))
    {
      let tank = unit.block == Block::Tank;
      let (capacity, levels) =
        bore(unit.block).map_or((TANK, TANK_LEVELS), |(capacity, _)| (capacity, LEVELS));
      let links = match tank {
        true => 0,
        false => voxels.links(unit.at, unit.block)
      };
      let wanted = unit.intake.filter(|&(_, kept)| kept > 0).map(|(liquid, kept)| {
        (
          liquid,
          links,
          ((kept as f32 / capacity * levels).ceil() as u8).clamp(1, levels as u8)
        )
      });
      if filled.0 != wanted {
        if let Some(old) = filled.1.take() {
          commands.entity(old).despawn()
        }
        filled.0 = wanted;
        filled.1 = wanted.map(|(liquid, links, level)| {
          let fraction = f32::from(level) / levels;
          let mesh = fittings
            .meshes
            .entry((tank, links, level))
            .or_insert_with(|| {
              meshes.add(boxes(&match tank {
                true => tanked(fraction),
                false => piped(links, fraction)
              }))
            })
            .clone();
          let material = fittings.liquids[&liquid].clone();
          commands
            .spawn((
              Mesh3d(mesh),
              MeshMaterial3d(material),
              Transform::IDENTITY,
              ChildOf(entity)
            ))
            .id()
        });
      }
    }
  }
}

#[derive(Component, Default)]
struct Spilling(Option<(Liquid, Entity)>);

fn spill(
  voxels: Option<Res<Voxels>>,
  fittings: Option<Res<Fittings>>,
  effects: Option<Res<crate::fx::Effects>>,
  mut drains: Query<(Entity, &Unit, Option<&mut Spilling>)>,
  mut commands: Commands
) {
  if let Some(voxels) = voxels
    && let Some(fittings) = fittings
    && let Some(effects) = effects
  {
    for (entity, unit, spilling) in
      drains.iter_mut().filter(|(_, unit, _)| unit.block == Block::FluidDrain)
    {
      let flowing =
        unit.intake.map(|(liquid, _)| liquid).filter(|_| unit.status == Status::Running);
      let shown =
        spilling.as_ref().and_then(|spilling| spilling.0).map(|(liquid, _)| liquid);
      if flowing != shown {
        let old =
          spilling.and_then(|mut spilling| spilling.0.take()).map(|(_, emitter)| emitter);
        if let Some(old) = old {
          commands.entity(old).despawn()
        }
        let lip = model::spout(voxels.links(unit.at, unit.block));
        let emitter = flowing.map(|liquid| {
          (
            liquid,
            commands
              .spawn((
                effects.emit(&fittings.spills[&liquid]),
                Transform::from_translation(lip),
                ChildOf(entity)
              ))
              .id()
          )
        });
        commands.entity(entity).insert(Spilling(emitter));
      }
    }
  }
}

#[derive(Component)]
struct Ferried {
  parcel: Option<(Block, u8, u8)>,
  shown: f32,
  item: Option<Entity>
}

const PARCEL_SCALE: f32 = 0.85;

fn path(from: u8, to: u8, progress: f32) -> Vec3 {
  let middle = Vec3::splat(0.5);
  let face = |side: u8| middle + crate::machine::SIDES[usize::from(side)].as_vec3() * 0.5;
  match progress < 0.5 {
    true => face(from).lerp(middle, progress * 2.0),
    false => middle.lerp(face(to), progress * 2.0 - 1.0)
  }
}

fn ferry(
  time: Res<Time>,
  palette: Option<Res<crate::stream::Palette>>,
  mut looks: ResMut<crate::loose::Looks>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut pipes: Query<(Entity, &Unit, Option<&mut Ferried>)>,
  mut transforms: Query<&mut Transform, Without<Unit>>,
  mut commands: Commands
) {
  if let Some(palette) = palette {
    for (entity, unit, ferried) in
      pipes.iter_mut().filter(|(_, unit, _)| unit.block == Block::ItemPipe)
    {
      let wanted = unit.parcel.map(|parcel| (parcel.stack.block, parcel.from, parcel.to));
      match ferried {
        Some(mut ferried) if ferried.parcel == wanted => {
          if let Some(parcel) = unit.parcel
            && let Some(item) = ferried.item
          {
            ferried.shown = (ferried.shown
              + crate::machine::PARCEL_SPEED * time.delta_secs())
            .min(1.0)
            .max(parcel.progress);
            if let Ok(mut transform) = transforms.get_mut(item) {
              let lift = match crate::loose::cubic(parcel.stack.block) {
                true => Vec3::Y * crate::loose::SIZE * PARCEL_SCALE / 2.0,
                false => Vec3::ZERO
              };
              transform.translation = path(parcel.from, parcel.to, ferried.shown) - lift
            }
          }
        }
        ferried => {
          if let Some(old) = ferried.as_ref().and_then(|ferried| ferried.item) {
            commands.entity(old).despawn()
          }
          let item = unit.parcel.map(|parcel| {
            let material = parcel.stack.gear.and_then(|gear| gear.material);
            commands
              .spawn((
                Mesh3d(looks.mesh(parcel.stack.block, material, &mut meshes)),
                MeshMaterial3d(palette.solid.clone()),
                Transform::from_translation(path(
                  parcel.from,
                  parcel.to,
                  parcel.progress
                ))
                .with_scale(Vec3::splat(PARCEL_SCALE)),
                ChildOf(entity)
              ))
              .id()
          });
          commands.entity(entity).insert(Ferried {
            parcel: wanted,
            shown: unit.parcel.map_or(0.0, |parcel| parcel.progress),
            item
          });
        }
      }
    }
  }
}

const BEAM_SWING: f32 = 0.32;
const STROKES: f32 = 1.4;
const SPIN: f32 = 4.0;

fn animate(
  time: Res<Time>,
  units: Query<&Unit>,
  mut beams: Query<(&ChildOf, &mut Beam, &mut Transform), Without<Rotor>>,
  mut rotors: Query<(&ChildOf, &mut Rotor, &mut Transform), Without<Beam>>,
  parents: Query<&ChildOf, Without<Rotor>>
) {
  let delta = time.delta_secs();
  for (parent, mut beam, mut transform) in beams.iter_mut() {
    if let Ok(unit) = units.get(parent.parent())
      && unit.status == Status::Running
    {
      beam.0 += delta * STROKES
    }
    transform.rotation = Quat::from_rotation_z(beam.0.sin() * BEAM_SWING)
  }
  for (parent, mut rotor, mut transform) in rotors.iter_mut() {
    let power = parents
      .get(parent.parent())
      .ok()
      .and_then(|mast| units.get(mast.parent()).ok())
      .map_or(0.0, |unit| unit.power.max(0) as f32 / design(unit.block).makes.max(1.0));
    rotor.0 += delta * SPIN * (0.15 + power);
    transform.rotation = Quat::from_rotation_z(rotor.0)
  }
}

fn help(block: Block) -> &'static str {
  match block {
    Block::Tank => "Holds 16 buckets of one liquid. Joins the pipes around it.",
    Block::Battery => "Stores power for the cables and machines it touches.",
    Block::ElectricPump => {
      "Pumps water or lava from a source block beside it into pipes. Needs power."
    }
    Block::Pumpjack => {
      "Stand it on an oil seep: it pumps crude oil into pipes slowly. Needs power."
    }
    Block::Refinery => "Turns crude oil piped in into diesel. Needs power.",
    Block::Fermenter => {
      "Right-click it with leaves, grass, flowers, saplings, mushrooms or fish: they ferment into biofuel."
    }
    Block::SteamEngine => {
      "Burns coal, charcoal, logs or planks (right-click to feed it) and boils water piped in."
    }
    Block::BiofuelGenerator => "Burns biofuel piped in.",
    Block::DieselGenerator => "Burns diesel piped in. The strongest generator.",
    Block::GeothermalGenerator => {
      "Runs on a lava source beside it, or on lava piped in (slowly used up)."
    }
    Block::SolarPanel => {
      "Makes a little power under open sky by day, less in bad weather."
    }
    Block::WindTurbine => "Makes more power higher up and in stronger wind.",
    Block::ElectricLamp | Block::ElectricLampLit => "Lights up while powered.",
    Block::FluidDrain => {
      "Place it against a tank or machine: it slowly spills the liquid out. Right-click it with a copper pipe to drain into pipes instead."
    }
    Block::PipedDrain => "Drains the tank or machine behind it into the pipes around it.",
    Block::ItemDrain => {
      "Place it against a chest: it drops the chest's items out one by one."
    }
    _ => ""
  }
}

#[derive(Component)]
struct Readout;

pub fn page(page: &mut ChildSpawnerCommands) {
  page.spawn((Readout, Node {
    flex_direction: FlexDirection::Column,
    row_gap: px(6),
    padding: UiRect::all(px(12)),
    ..default()
  }));
}

const RUNNING: Color = Color::srgb(0.45, 0.9, 0.45);
const STALLED: Color = Color::srgb(0.95, 0.72, 0.35);
const TROUGH: Color = Color::srgb(0.1, 0.11, 0.13);
const POWER: Color = Color::srgb(0.95, 0.85, 0.3);
const FUEL: Color = Color::srgb(0.85, 0.45, 0.2);

fn gauge(parent: &mut ChildSpawnerCommands, label: String, fraction: f32, color: Color) {
  parent
    .spawn(Node { align_items: AlignItems::Center, column_gap: px(10), ..default() })
    .with_children(|row| {
      row
        .spawn((
          Node {
            width: px(260),
            height: px(14),
            border: UiRect::all(px(2)),
            ..default()
          },
          BorderColor::all(Color::srgb(0.3, 0.33, 0.36)),
          BackgroundColor(TROUGH)
        ))
        .with_child((
          Node {
            width: percent(fraction.clamp(0.0, 1.0) * 100.0),
            height: percent(100),
            ..default()
          },
          BackgroundColor(color)
        ));
      row.spawn(words(label, 15.0, INK));
    });
}

fn buckets(amount: u32) -> String { format!("{:.1}", amount as f32 / BUCKET) }

fn readout(parent: &mut ChildSpawnerCommands, unit: &Unit) {
  let Design { intake, outlet, stores, hopper, makes, draws } = design(unit.block);
  let tank = unit.block == Block::Tank;
  parent.spawn(words(unit.block.name(), 22.0, INK));
  parent.spawn(words(unit.status.line(), 16.0, match unit.status {
    Status::Running => RUNNING,
    _ => STALLED
  }));
  let liquid = |parent: &mut ChildSpawnerCommands,
                title: &str,
                held: Option<(Liquid, u32)>,
                room: f32| {
    let (name, kept, color) = held.map_or(("empty", 0, FAINT), |(liquid, kept)| {
      (liquid.name(), kept, liquid.color())
    });
    gauge(
      parent,
      format!("{title}: {name}, {} of {} buckets", buckets(kept), buckets(room as u32)),
      kept as f32 / room,
      color
    )
  };
  if tank {
    liquid(parent, "Holds", unit.intake, TANK)
  }
  if let Some((flowing, _)) = unit.intake.filter(|_| unit.block.drain()) {
    parent.spawn(words(format!("Draining {}", flowing.name().to_lowercase()), 15.0, INK));
  }
  if let Some((_, room)) = intake {
    liquid(parent, "In", unit.intake, room)
  }
  if let Some(room) = outlet {
    liquid(parent, "Out", unit.outlet, room)
  }
  if hopper > 0.0 {
    let label = match unit.block {
      Block::SteamEngine => format!("Fuel: {} s of burning", unit.stock),
      _ => format!(
        "Fermenting: {} of {} buckets",
        buckets(unit.stock),
        buckets(hopper as u32)
      )
    };
    gauge(parent, label, unit.stock as f32 / hopper, FUEL)
  }
  if stores > 0.0 {
    let label = match unit.block {
      Block::ElectricFurnace => format!(
        "Charge: {} smelts ready (crafts furnace recipes without coal)",
        unit.charge as f32 as u32 / crate::machine::SMELT as u32
      ),
      _ => format!("Stored: {} of {} kJ", unit.charge / 1000, stores as u32 / 1000)
    };
    gauge(parent, label, unit.charge as f32 / stores, POWER)
  }
  if makes > 0.0 || draws > 0.0 {
    let line = match unit.power >= 0 {
      true => format!("Making {} W (up to {} W)", unit.power, makes as u32),
      false => format!("Using {} W", -unit.power)
    };
    parent.spawn(words(line, 15.0, INK));
  }
  if makes > 0.0 || draws > 0.0 || stores > 0.0 {
    let grid = unit.grid;
    parent.spawn(words(
      format!(
        "Grid: {} W made, {} W wanted, batteries {} of {} kJ",
        grid.made,
        grid.wanted,
        grid.stored / 1000,
        grid.room / 1000
      ),
      15.0,
      FAINT
    ));
  }
  parent.spawn((words(help(unit.block), 14.0, FAINT), Node {
    margin: UiRect::top(px(8)),
    max_width: px(560),
    ..default()
  }));
}

fn show(
  menu: Res<Menu>,
  watching: Res<Watching>,
  units: Query<&Unit>,
  readouts: Query<Entity, With<Readout>>,
  mut commands: Commands,
  mut shown: Local<Option<Unit>>
) {
  let unit = watching
    .0
    .filter(|_| menu.showing(Tab::Machine))
    .and_then(|at| units.iter().find(|unit| unit.at == at).copied());
  if *shown != unit {
    *shown = unit;
    for list in readouts.iter() {
      commands.entity(list).despawn_children().with_children(|list| match unit {
        Some(unit) => readout(list, &unit),
        None => {
          list.spawn(words("The machine is gone.", 16.0, FAINT));
        }
      });
    }
  }
}

pub struct Apparatus;

impl Plugin for Apparatus {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Watching>()
      .add_systems(Startup, furnish.run_if(plays))
      .add_systems(
        Update,
        (mount, fill, spill, ferry, animate, show.run_if(resource_exists::<Menu>))
          .chain()
          .run_if(plays)
      );
  }
}
