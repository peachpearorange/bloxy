use {crate::{ground::{Concealed, EMERGING, Emerge, Ground, born, settled},
             mesh::{self, Meshes, Padded},
             player::Pilot,
             protocol::plays,
             texture,
             voxels::{Chunk, LAYERS, Voxels, chunk_of, generated, in_world, origin_of},
             water::{self, Water}},
     bevy::{light::NotShadowCaster,
            platform::collections::HashMap,
            prelude::*,
            tasks::{AsyncComputeTaskPool, Task, futures::check_ready}}};

const GENERATING: usize = if cfg!(target_arch = "wasm32") { 2 } else { 12 };
const MESHING: usize = if cfg!(target_arch = "wasm32") { 3 } else { 12 };
const STARTS: usize = if cfg!(target_arch = "wasm32") { 1 } else { 12 };
const KEEP_BEYOND: i32 = 2;
const GLOW: f32 = 6.0;

#[derive(Resource)]
pub struct Palette {
  pub solid: Handle<StandardMaterial>,
  pub ground: Handle<Ground>,
  pub liquid: Handle<Water>,
  pub icons: Handle<Image>
}

#[derive(Resource, Default)]
struct Streaming {
  generating: HashMap<IVec3, Task<Chunk>>,
  meshing: HashMap<IVec3, Task<Meshes>>,
  shown: HashMap<IVec3, Vec<Entity>>,
  emerging: Vec<(Entity, f32, bool)>
}

#[derive(Resource, Default)]
pub struct Progress {
  pub pending: usize
}

pub fn paint(
  mut commands: Commands,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut grounds: ResMut<Assets<Ground>>,
  mut waters: ResMut<Assets<Water>>
) {
  let atlas = images.add(texture::albedo());
  let glow = images.add(texture::glow());
  let base = StandardMaterial {
    base_color_texture: Some(atlas),
    emissive_texture: Some(glow.clone()),
    emissive: LinearRgba::rgb(GLOW, GLOW, GLOW),
    perceptual_roughness: 0.92,
    reflectance: 0.25,
    alpha_mode: AlphaMode::Mask(0.5),
    ..default()
  };
  let solid = materials.add(base.clone());
  let ground = grounds
    .add(Ground { base, extension: Emerge { born: settled(), hidden: Vec4::ZERO } });
  let liquid = waters.add(water::water(settled()));
  commands.insert_resource(Palette {
    solid,
    ground,
    liquid,
    icons: images.add(texture::icons())
  })
}

fn wanted(centre: IVec3, reach: i32) -> impl Iterator<Item = IVec3> {
  let inside = move |x: i32, z: i32| {
    let (x, z) = ((x.abs() - 1).max(0), (z.abs() - 1).max(0));
    x * x + z * z <= reach * reach
  };
  (-reach - 1..=reach + 1).flat_map(move |x| {
    (-reach - 1..=reach + 1).filter(move |&z| inside(x, z)).flat_map(move |z| {
      (0..LAYERS).map(move |y| IVec3::new(centre.x + x, y, centre.z + z))
    })
  })
}

fn horizontal(key: IVec3, centre: IVec3) -> i32 {
  let offset = (key - centre).xz();
  offset.length_squared()
}

fn settle(
  time: Res<Time>,
  palette: Res<Palette>,
  mut streaming: ResMut<Streaming>,
  mut commands: Commands
) {
  let now = time.elapsed_secs_wrapped();
  streaming.emerging.retain(|&(entity, birth, liquid)| {
    let emerging = now >= birth && now - birth < EMERGING + 0.2;
    if !emerging {
      let mut chunk = commands.entity(entity);
      match liquid {
        true => chunk.try_insert(MeshMaterial3d(palette.liquid.clone())),
        false => chunk.try_insert(MeshMaterial3d(palette.ground.clone()))
      };
    }
    emerging
  })
}

fn stream(
  mut commands: Commands,
  mut streaming: ResMut<Streaming>,
  mut progress: ResMut<Progress>,
  voxels: Option<ResMut<Voxels>>,
  pilot: Option<Res<Pilot>>,
  palette: Res<Palette>,
  settings: Res<crate::settings::Settings>,
  mut meshes: ResMut<Assets<Mesh>>,
  (time, concealed, mut grounds, mut waters): (
    Res<Time>,
    Res<Concealed>,
    ResMut<Assets<Ground>>,
    ResMut<Assets<Water>>
  )
) {
  if let Some(mut voxels) = voxels
    && let Some(pilot) = pilot
  {
    let reach = settings.reach;
    let centre = chunk_of(pilot.at.floor().as_ivec3());
    let pool = AsyncComputeTaskPool::get();
    let seed = voxels.seed;
    if voxels.is_added() {
      streaming.generating.clear();
      streaming.meshing.clear()
    }
    let kept = voxels.kept.clone();
    let mut missing: Vec<IVec3> = wanted(centre, reach)
      .filter(|key| {
        !voxels.chunks.contains_key(key) && !streaming.generating.contains_key(key)
      })
      .collect();
    missing.sort_by_key(|&key| (horizontal(key, centre), (key.y - centre.y).abs()));
    let room = GENERATING.saturating_sub(streaming.generating.len()).min(STARTS);
    for &key in missing.iter().take(room) {
      let kept = kept.clone();
      streaming
        .generating
        .insert(key, pool.spawn(async move { generated(seed, &kept, key) }));
    }
    let generated: Vec<(IVec3, Chunk)> = streaming
      .generating
      .iter_mut()
      .filter_map(|(&key, task)| check_ready(task).map(|chunk| (key, chunk)))
      .collect();
    for (key, chunk) in generated.into_iter() {
      streaming.generating.remove(&key);
      voxels.insert(key, chunk)
    }
    let far = |key: &IVec3| horizontal(*key, centre) > (reach + KEEP_BEYOND).pow(2);
    voxels.chunks.retain(|key, _| !far(key));
    let mut dirty: Vec<IVec3> = voxels
      .dirty
      .iter()
      .copied()
      .filter(|key| in_world(*key) && horizontal(*key, centre) <= reach * reach)
      .collect();
    dirty.sort_by_key(|&key| (horizontal(key, centre), (key.y - centre.y).abs()));
    let room = MESHING.saturating_sub(streaming.meshing.len()).min(STARTS);
    let gathered: Vec<(IVec3, Padded, Vec<IVec3>)> = dirty
      .into_iter()
      .filter(|key| !streaming.meshing.contains_key(key))
      .filter_map(|key| {
        Padded::gather(key, |near| voxels.chunks.get(&near).cloned())
          .map(|padded| (key, padded, voxels.torches_near(key)))
      })
      .take(room)
      .collect();
    let started: Vec<IVec3> = gathered
      .into_iter()
      .map(|(key, padded, torches)| {
        streaming.meshing.insert(
          key,
          pool.spawn(async move { mesh::build(&padded, key, seed, &torches) })
        );
        key
      })
      .collect();
    voxels.dirty.retain(|key| in_world(*key) && !far(key) && !started.contains(key));
    let built: Vec<(IVec3, Meshes)> = streaming
      .meshing
      .iter_mut()
      .filter_map(|(&key, task)| check_ready(task).map(|built| (key, built)))
      .collect();
    for (key, built) in built.into_iter() {
      streaming.meshing.remove(&key);
      let placed = Transform::from_translation(origin_of(key).as_vec3());
      let fresh = !streaming.shown.contains_key(&key);
      let birth = born(&time);
      let ground = match fresh {
        true => grounds
          .get(&palette.ground)
          .map(|shared| Ground {
            base: shared.base.clone(),
            extension: Emerge {
              born: Vec4::new(birth, 0.0, 0.0, 0.0),
              hidden: concealed.hidden()
            }
          })
          .map_or(palette.ground.clone(), |ground| grounds.add(ground)),
        false => palette.ground.clone()
      };
      let liquid_material = match fresh {
        true => waters.add(water::water(Vec4::new(birth, 0.0, 0.0, 0.0))),
        false => palette.liquid.clone()
      };
      let solid = built.solid.map(|mesh| {
        commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(ground), placed)).id()
      });
      let liquid = built.liquid.map(|mesh| {
        commands
          .spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(liquid_material),
            NotShadowCaster,
            placed
          ))
          .id()
      });
      if fresh {
        streaming.emerging.extend(
          solid
            .map(|entity| (entity, birth, false))
            .into_iter()
            .chain(liquid.map(|entity| (entity, birth, true)))
        )
      }
      let shown: Vec<Entity> = solid.into_iter().chain(liquid).collect();
      for old in streaming.shown.insert(key, shown).into_iter().flatten() {
        commands.entity(old).despawn()
      }
    }
    let gone: Vec<IVec3> = streaming
      .shown
      .keys()
      .copied()
      .filter(|key| horizontal(*key, centre) > reach * reach)
      .collect();
    for key in gone.into_iter() {
      voxels.dirty.insert(key);
      for old in streaming.shown.remove(&key).into_iter().flatten() {
        commands.entity(old).despawn()
      }
    }
    let unmeshed = voxels
      .dirty
      .iter()
      .filter(|key| horizontal(**key, centre) <= reach * reach)
      .count();
    progress.pending =
      missing.len() + streaming.generating.len() + streaming.meshing.len() + unmeshed;
  }
}

pub fn ready_around(voxels: &Voxels, at: Vec3) -> bool {
  let centre = chunk_of(at.floor().as_ivec3());
  (-1..=1).all(|y| {
    let key = centre + IVec3::Y * y;
    !in_world(key) || voxels.chunks.contains_key(&key)
  })
}

pub struct Stream;

impl Plugin for Stream {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Streaming>()
      .init_resource::<Progress>()
      .add_systems(Startup, paint.run_if(plays))
      .add_systems(Update, (stream, settle).chain().run_if(plays));
  }
}
