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

fn basin(block: Block, runes: u32) -> Option<(f32, f32, f32, f32)> {
  match block {
    Block::Tank => Some((0.08, 0.125, 0.875, TANK)),
    Block::ManaCistern => Some((2.0 / 16.0, 2.0 / 16.0, 7.5 / 16.0, crate::arcana::POOL)),
    Block::Cauldron => Some((2.0 / 16.0, 3.0 / 16.0, 12.5 / 16.0, BUCKET)),
    Block::CrimsonAltar => {
      Some((1.0 / 16.0, 12.0 / 16.0, 14.5 / 16.0, crate::arcana::altar_room(runes)))
    }
    _ => None
  }
}

fn tanked((side, floor, top, _): (f32, f32, f32, f32), fill: f32) -> Vec<Span> {
  vec![(
    Vec3::new(side, floor, side),
    Vec3::new(1.0 - side, floor + (top - floor) * fill, 1.0 - side)
  )]
}

#[derive(Resource)]
struct Fittings {
  liquids: HashMap<Liquid, Handle<StandardMaterial>>,
  spills: HashMap<Liquid, Handle<bevy_hanabi::EffectAsset>>,
  meshes: HashMap<(Block, u16, u8), Handle<Mesh>>,
  mote: Handle<Mesh>,
  glows: HashMap<Liquid, Handle<StandardMaterial>>,
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
  let glows = Liquid::ALL
    .into_iter()
    .map(|liquid| {
      let color = liquid.color();
      (
        liquid,
        materials.add(StandardMaterial {
          base_color: color,
          emissive: LinearRgba::from(color) * 4.0,
          unlit: true,
          ..default()
        })
      )
    })
    .collect();
  commands.insert_resource(Fittings {
    liquids,
    spills,
    glows,
    mote: meshes.add(Cuboid::from_length(0.07)),
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
    for (entity, unit, mut filled) in units.iter_mut().filter(|(_, unit, _)| {
      pooled(unit.block) || basin(unit.block, unit.stock).is_some()
    }) {
      let shape = basin(unit.block, unit.stock);
      let (capacity, levels) = match (bore(unit.block), shape) {
        (Some((capacity, _)), _) => (capacity, LEVELS),
        (None, Some((.., capacity))) => (capacity, TANK_LEVELS),
        _ => (TANK, TANK_LEVELS)
      };
      let links = match shape {
        Some(_) => 0,
        None => voxels.links(unit.at, unit.block)
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
            .entry((unit.block, links, level))
            .or_insert_with(|| {
              meshes.add(boxes(&match shape {
                Some(shape) => tanked(shape, fraction),
                None => piped(links, fraction)
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

#[derive(Component)]
struct Displayed {
  item: Option<(Block, Option<crate::gear::Material>)>,
  shown: Option<Entity>
}

#[derive(Component)]
struct Hovering(f32);

fn perch(block: Block) -> f32 {
  match block {
    Block::Plinth => 13.0 / 16.0,
    Block::CrimsonAltar => 12.0 / 16.0,
    _ => 1.0
  }
}

fn display(
  palette: Option<Res<crate::stream::Palette>>,
  mut looks: ResMut<crate::loose::Looks>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut holders: Query<(Entity, &Unit, Option<&mut Displayed>)>,
  mut commands: Commands
) {
  if let Some(palette) = palette {
    for (entity, unit, displayed) in
      holders.iter_mut().filter(|(_, unit, _)| unit.block.holder())
    {
      let wanted =
        unit.item.map(|stack| (stack.block, stack.gear.and_then(|gear| gear.material)));
      if displayed.as_ref().is_none_or(|displayed| displayed.item != wanted) {
        if let Some(old) = displayed.and_then(|displayed| displayed.shown) {
          commands.entity(old).despawn()
        }
        let shown = wanted.map(|(block, material)| {
          let lift = match crate::loose::cubic(block) {
            true => 0.05,
            false => 0.2
          };
          commands
            .spawn((
              Hovering(perch(unit.block) + lift),
              Mesh3d(looks.mesh(block, material, &mut meshes)),
              MeshMaterial3d(palette.solid.clone()),
              Transform::from_translation(Vec3::new(0.5, perch(unit.block) + lift, 0.5))
                .with_scale(Vec3::splat(1.4)),
              ChildOf(entity)
            ))
            .id()
        });
        commands.entity(entity).insert(Displayed { item: wanted, shown });
      }
    }
  }
}

fn hover(time: Res<Time>, mut hovering: Query<(&Hovering, &mut Transform)>) {
  let now = time.elapsed_secs();
  for (&Hovering(height), mut transform) in hovering.iter_mut() {
    transform.translation.y = height + (now * 2.0).sin() * 0.04;
    transform.rotation = Quat::from_rotation_y(now * 0.8)
  }
}

#[derive(Component)]
struct Mote {
  from: Vec3,
  to: Vec3,
  age: f32,
  life: f32
}

fn sparkle(
  time: Res<Time>,
  fittings: Option<Res<Fittings>>,
  units: Query<&Unit>,
  mut commands: Commands,
  mut clock: Local<f32>
) {
  *clock += time.delta_secs();
  if let Some(fittings) = fittings
    && *clock >= 0.3
  {
    *clock = 0.0;
    let beat = (time.elapsed_secs() * 3.3) as u32;
    let mut emit = |from: Vec3, to: Vec3, liquid: Liquid, life: f32| {
      commands.spawn((
        Mote { from, to, age: 0.0, life },
        Mesh3d(fittings.mote.clone()),
        MeshMaterial3d(fittings.glows[&liquid].clone()),
        Transform::from_translation(from),
        bevy::light::NotShadowCaster
      ));
    };
    let top = |at: IVec3, height: f32| at.as_vec3() + Vec3::new(0.5, height, 0.5);
    let jitter = |at: IVec3| {
      let seed = (at.x * 73 + at.z * 37 + beat as i32 * 11) as f32;
      Vec3::new(seed.sin() * 0.3, 0.0, seed.cos() * 0.3)
    };
    for unit in units.iter().filter(|unit| unit.status == Status::Running) {
      match unit.block {
        flower if flower.bloomer() => {
          if let Some(pool) = unit.link
            && (beat + unit.at.x as u32) % 2 == 0
          {
            emit(top(unit.at, 0.8), top(pool, 0.4), Liquid::Mana, 1.2)
          }
        }
        Block::ManaCistern => {
          emit(top(unit.at, 0.3) + jitter(unit.at), top(unit.at, 1.4), Liquid::Mana, 0.8)
        }
        Block::Cauldron => emit(
          top(unit.at, 0.75) + jitter(unit.at) * 0.6,
          top(unit.at, 1.3) + jitter(unit.at),
          Liquid::Biofuel,
          0.7
        ),
        Block::CrimsonAltar => emit(
          top(unit.at, 0.9) + jitter(unit.at),
          top(unit.at, 1.6),
          Liquid::Lifeblood,
          0.9
        ),
        Block::ConfluenceAltar => {
          for pedestal in units.iter().filter(|near| {
            let offset = near.at - unit.at;
            near.block == Block::Plinth
              && near.item.is_some()
              && offset.x.abs() <= 3
              && offset.z.abs() <= 3
              && offset.y.abs() <= 1
          }) {
            emit(top(pedestal.at, 1.05), top(unit.at, 1.2), Liquid::Mana, 0.9)
          }
        }
        _ => ()
      }
    }
  }
}

fn drift(
  time: Res<Time>,
  mut motes: Query<(Entity, &mut Mote, &mut Transform)>,
  mut commands: Commands
) {
  for (entity, mut mote, mut transform) in motes.iter_mut() {
    mote.age += time.delta_secs();
    let along = (mote.age / mote.life).min(1.0);
    let arc = (along * std::f32::consts::PI).sin() * 0.5;
    transform.translation = mote.from.lerp(mote.to, along) + Vec3::Y * arc;
    transform.scale = Vec3::splat(1.0 - along * 0.6);
    if along >= 1.0 {
      commands.entity(entity).despawn()
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
    Block::Sunsip => {
      "Makes mana under open sky by day and sends it to a mana cistern within 6 blocks."
    }
    Block::Dewcup => {
      "Makes mana beside a water source and sends it to a mana cistern within 6 blocks."
    }
    Block::Emberbell => {
      "Burns coal, charcoal, logs or planks (right-click it, or toss them beside it) into mana for a cistern within 6 blocks."
    }
    Block::ManaCistern => {
      "Holds the mana that flowers send it. Toss iron ore, diamond ore, lumen or bone dust in to infuse them."
    }
    Block::Cauldron => {
      "Fill it with water and set it over fire or lava. Items tossed in melt into essence; a catalyst tossed in uses the essence up."
    }
    Block::CrimsonAltar => {
      "Right-click with a Ritual Knife to bleed into it; creatures slain nearby feed it too. Set an item on it to transmute it. Crimson runes around and below it hold more and work faster."
    }
    Block::Crusher => {
      "Crushes greywacke cobble into gravel, gravel into sand (sometimes sifting out platinum dust) and platinum ore into two dusts. Right-click it with what to crush, or pipe it in; what comes out goes into a pipe or chest beside it. Needs power."
    }
    Block::Plinth => "Holds one item for an Confluence Altar within 3 blocks.",
    Block::ConfluenceAltar => {
      "Set a catalyst on it and ingredients on plinths around it: mana from cisterns within 8 blocks fuses them. A tool or wearable takes a charm."
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

const MANA: Color = Color::srgb(0.35, 0.78, 1.0);
const BLOODY: Color = Color::srgb(0.75, 0.1, 0.12);
const ESSENCE: Color = Color::srgb(0.7, 0.45, 0.95);

fn lexicon(parent: &mut ChildSpawnerCommands, block: Block) {
  let lines: Vec<String> = crate::arcana::rites(block)
    .map(|recipe| {
      let inputs: Vec<&str> =
        recipe.inputs.iter().map(|&(input, _)| input.name()).collect();
      let (output, count) = recipe.output;
      let cost = match recipe.cost {
        crate::recipe::Cost::Mana(mana) => format!("{mana} mana"),
        crate::recipe::Cost::Lifeblood(blood) => format!("{blood} lifeblood"),
        crate::recipe::Cost::Essence(essence) => essence
          .iter()
          .map(|(aspect, count)| format!("{count} {}", aspect.name()))
          .collect::<Vec<_>>()
          .join(", "),
        crate::recipe::Cost::Chance(percent) => format!("{percent}% chance"),
        crate::recipe::Cost::Free => String::new()
      };
      format!("{} -> {} x{count} ({cost})", inputs.join(" + "), output.name())
    })
    .chain(crate::arcana::CHARMS.iter().filter(|_| block == Block::ConfluenceAltar).map(
      |charm| {
        let pedestals: Vec<&str> =
          charm.pedestals.iter().map(|block| block.name()).collect();
        format!(
          "Tool or wearable + {} -> {} ({} mana)",
          pedestals.join(" + "),
          charm.virtue.line(charm.by),
          charm.mana
        )
      }
    ))
    .collect();
  if !lines.is_empty() {
    parent.spawn((words("Recipes", 16.0, INK), Node {
      margin: UiRect::top(px(8)),
      ..default()
    }));
    for line in lines {
      parent.spawn(words(line, 13.0, FAINT));
    }
  }
}

fn arcane(parent: &mut ChildSpawnerCommands, unit: &Unit) {
  let held = |liquid: Liquid| {
    unit.intake.filter(|&(kept, _)| kept == liquid).map_or(0, |(_, kept)| kept)
  };
  match unit.block {
    flower if flower.bloomer() => {
      gauge(
        parent,
        format!("Holding {} mana", unit.charge),
        unit.charge as f32 / 1000.0,
        MANA
      );
      if flower == Block::Emberbell {
        gauge(
          parent,
          format!("Fuel: {} s of burning", unit.stock),
          unit.stock as f32 / 600.0,
          FUEL
        )
      }
      if let Some(pool) = unit.link {
        parent.spawn(words(
          format!("Feeding the cistern at {} {} {}", pool.x, pool.y, pool.z),
          15.0,
          FAINT
        ));
      }
    }
    Block::ManaCistern => {
      let mana = held(Liquid::Mana);
      gauge(
        parent,
        format!("Mana: {mana} of {}", crate::arcana::POOL as u32),
        mana as f32 / crate::arcana::POOL,
        MANA
      )
    }
    Block::Cauldron => {
      let water = held(Liquid::Water);
      gauge(
        parent,
        format!("Water: {} of 1 bucket", buckets(water)),
        water as f32 / BUCKET,
        Liquid::Water.color()
      );
      let essence: Vec<String> = crate::arcana::Aspect::ALL
        .into_iter()
        .filter(|&aspect| unit.essence[aspect as usize] > 0)
        .map(|aspect| format!("{} {}", unit.essence[aspect as usize], aspect.name()))
        .collect();
      parent.spawn(words(
        match essence.is_empty() {
          true => "No essence yet".to_string(),
          false => format!("Essence: {}", essence.join(", "))
        },
        15.0,
        ESSENCE
      ));
    }
    Block::CrimsonAltar => {
      let (blood, room) =
        (held(Liquid::Lifeblood), crate::arcana::altar_room(unit.stock));
      gauge(
        parent,
        format!("Life essence: {blood} of {}", room as u32),
        blood as f32 / room,
        BLOODY
      );
      parent.spawn(words(format!("Crimson runes nearby: {}", unit.stock), 15.0, FAINT));
    }
    _ => ()
  }
  if unit.block.holder() {
    parent.spawn(words(
      unit.item.map_or("Holding nothing".to_string(), |stack| {
        format!("Holding {}", crate::gear::title(&stack))
      }),
      15.0,
      INK
    ));
    if unit.charge > 0 {
      let color = match unit.block {
        Block::CrimsonAltar => BLOODY,
        _ => MANA
      };
      parent.spawn(words(format!("Poured in so far: {}", unit.charge), 15.0, color));
    }
  }
  lexicon(parent, unit.block)
}

fn readout(parent: &mut ChildSpawnerCommands, unit: &Unit) {
  let Design { intake, outlet, stores, hopper, makes, draws } = design(unit.block);
  let tank = unit.block == Block::Tank;
  parent.spawn(words(unit.block.name(), 22.0, INK));
  parent.spawn(words(unit.status.line(), 16.0, match unit.status {
    Status::Running => RUNNING,
    _ => STALLED
  }));
  if unit.block.arcane() {
    arcane(parent, unit)
  } else {
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
    if unit.block == Block::Crusher {
      parent.spawn(words(
        unit.item.map_or("Nothing to crush".to_string(), |stack| {
          format!("Crushing {} {}", stack.count, stack.block.name())
        }),
        15.0,
        INK
      ));
    }
    if let Some((flowing, _)) = unit.intake.filter(|_| unit.block.drain()) {
      parent.spawn(words(
        format!("Draining {}", flowing.name().to_lowercase()),
        15.0,
        INK
      ));
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
        (
          mount,
          fill,
          spill,
          ferry,
          display,
          hover,
          sparkle,
          drift,
          animate,
          show.run_if(resource_exists::<Menu>)
        )
          .chain()
          .run_if(plays)
      );
  }
}
