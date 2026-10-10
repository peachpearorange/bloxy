use {crate::{generate,
             mesh::{self, Meshes, Padded},
             player::Pilot,
             protocol::plays,
             texture,
             voxels::{Chunk, LAYERS, SIZE, Voxels, chunk_of, in_world, origin_of}},
     bevy::{light::NotShadowCaster,
            platform::collections::HashMap,
            prelude::*,
            tasks::{AsyncComputeTaskPool, Task, futures::check_ready}}};

const GENERATING: usize = if cfg!(target_arch = "wasm32") { 2 } else { 12 };
const MESHING: usize = if cfg!(target_arch = "wasm32") { 3 } else { 12 };
const KEEP_BEYOND: i32 = 2;
const GLOW: f32 = 6.0;

#[derive(Resource)]
pub struct Palette {
  pub solid: Handle<StandardMaterial>,
  pub liquid: Handle<StandardMaterial>,
  pub atlas: Handle<Image>
}

#[derive(Resource, Default)]
struct Streaming {
  generating: HashMap<IVec3, Task<Chunk>>,
  meshing: HashMap<IVec3, Task<Meshes>>,
  shown: HashMap<IVec3, Vec<Entity>>
}

#[derive(Resource, Default)]
pub struct Progress {
  pub pending: usize
}

pub fn paint(
  mut commands: Commands,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let atlas = images.add(texture::albedo());
  let glow = images.add(texture::glow());
  let solid = materials.add(StandardMaterial {
    base_color_texture: Some(atlas.clone()),
    emissive_texture: Some(glow.clone()),
    emissive: LinearRgba::rgb(GLOW, GLOW, GLOW),
    perceptual_roughness: 0.92,
    reflectance: 0.25,
    alpha_mode: AlphaMode::Mask(0.5),
    ..default()
  });
  let liquid = materials.add(StandardMaterial {
    base_color_texture: Some(atlas.clone()),
    perceptual_roughness: 0.14,
    reflectance: 0.55,
    ..default()
  });
  commands.insert_resource(Palette { solid, liquid, atlas })
}

const RIPPLE_EVERY: f32 = 0.18;

fn ripple(
  time: Res<Time>,
  palette: Res<Palette>,
  mut images: ResMut<Assets<Image>>,
  mut shown: Local<u32>
) {
  let frame = (time.elapsed_secs() / RIPPLE_EVERY) as u32 % texture::WATER_FRAMES;
  if frame != *shown
    && let Some(mut atlas) = images.get_mut(&palette.atlas)
  {
    *shown = frame;
    texture::ripple(&mut atlas, frame)
  }
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

fn stream(
  mut commands: Commands,
  mut streaming: ResMut<Streaming>,
  mut progress: ResMut<Progress>,
  voxels: Option<ResMut<Voxels>>,
  pilot: Option<Res<Pilot>>,
  palette: Res<Palette>,
  settings: Res<crate::settings::Settings>,
  mut meshes: ResMut<Assets<Mesh>>
) {
  if let Some(mut voxels) = voxels
    && let Some(pilot) = pilot
  {
    let reach = settings.reach;
    let centre = chunk_of(pilot.at.floor().as_ivec3());
    let pool = AsyncComputeTaskPool::get();
    let seed = voxels.seed;
    let mut missing: Vec<IVec3> = wanted(centre, reach)
      .filter(|key| {
        !voxels.chunks.contains_key(key) && !streaming.generating.contains_key(key)
      })
      .collect();
    missing.sort_by_key(|&key| (horizontal(key, centre), (key.y - centre.y).abs()));
    let room = GENERATING.saturating_sub(streaming.generating.len());
    missing.iter().take(room).for_each(|&key| {
      streaming
        .generating
        .insert(key, pool.spawn(async move { generate::chunk(seed, key) }));
    });
    let generated: Vec<(IVec3, Chunk)> = streaming
      .generating
      .iter_mut()
      .filter_map(|(&key, task)| check_ready(task).map(|chunk| (key, chunk)))
      .collect();
    generated.into_iter().for_each(|(key, chunk)| {
      streaming.generating.remove(&key);
      voxels.insert(key, chunk)
    });
    let far = |key: &IVec3| horizontal(*key, centre) > (reach + KEEP_BEYOND).pow(2);
    voxels.chunks.retain(|key, _| !far(key));
    let mut dirty: Vec<IVec3> = voxels
      .dirty
      .iter()
      .copied()
      .filter(|key| in_world(*key) && horizontal(*key, centre) <= reach * reach)
      .collect();
    dirty.sort_by_key(|&key| (horizontal(key, centre), (key.y - centre.y).abs()));
    let room = MESHING.saturating_sub(streaming.meshing.len());
    let gathered: Vec<(IVec3, Padded)> = dirty
      .into_iter()
      .filter(|key| !streaming.meshing.contains_key(key))
      .filter_map(|key| {
        Padded::gather(key, |near| voxels.chunks.get(&near).cloned())
          .map(|padded| (key, padded))
      })
      .take(room)
      .collect();
    let started: Vec<IVec3> = gathered
      .into_iter()
      .map(|(key, padded)| {
        streaming
          .meshing
          .insert(key, pool.spawn(async move { mesh::build(&padded, key, seed) }));
        key
      })
      .collect();
    voxels.dirty.retain(|key| in_world(*key) && !far(key) && !started.contains(key));
    let built: Vec<(IVec3, Meshes)> = streaming
      .meshing
      .iter_mut()
      .filter_map(|(&key, task)| check_ready(task).map(|built| (key, built)))
      .collect();
    built.into_iter().for_each(|(key, built)| {
      streaming.meshing.remove(&key);
      let shown: Vec<Entity> =
        [(built.solid, palette.solid.clone()), (built.liquid, palette.liquid.clone())]
          .into_iter()
          .filter_map(|(mesh, material)| mesh.map(|mesh| (mesh, material)))
          .map(|(mesh, material)| {
            let liquid = material == palette.liquid;
            let mut entity = commands.spawn((
              Mesh3d(meshes.add(mesh)),
              MeshMaterial3d(material),
              Transform::from_translation(origin_of(key).as_vec3())
            ));
            if liquid {
              entity.insert(NotShadowCaster);
            }
            entity.id()
          })
          .collect();
      streaming
        .shown
        .insert(key, shown)
        .into_iter()
        .flatten()
        .for_each(|old| commands.entity(old).despawn())
    });
    let gone: Vec<IVec3> = streaming
      .shown
      .keys()
      .copied()
      .filter(|key| horizontal(*key, centre) > reach * reach)
      .collect();
    gone.into_iter().for_each(|key| {
      streaming
        .shown
        .remove(&key)
        .into_iter()
        .flatten()
        .for_each(|old| commands.entity(old).despawn())
    });
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

pub const CHUNK_METRES: f32 = SIZE as f32;

pub struct Stream;

impl Plugin for Stream {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Streaming>()
      .init_resource::<Progress>()
      .add_systems(Startup, paint.run_if(plays))
      .add_systems(Update, (stream, ripple).run_if(plays));
  }
}
