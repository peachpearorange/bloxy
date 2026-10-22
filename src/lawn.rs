use {crate::{ground::{Concealed, Elements},
             mesh::{self, Padded},
             player::Pilot,
             protocol::Role,
             stream::Progress,
             voxels::{LAYERS, Voxels, chunk_of, in_world, origin_of}},
     bevy::{asset::embedded_asset,
            light::NotShadowCaster,
            pbr::{ExtendedMaterial, MaterialExtension},
            platform::collections::{HashMap, HashSet},
            prelude::*,
            render::render_resource::AsBindGroup,
            shader::ShaderRef,
            tasks::{AsyncComputeTaskPool, Task, futures::check_ready}}};

const REACH: i32 = 2;
const THINNING_FROM: f32 = 24.0;
const GONE_AT: f32 = 36.0;
const GROWING: usize = if cfg!(target_arch = "wasm32") { 1 } else { 6 };

#[derive(Resource, Default)]
pub struct Lawns {
  pub stale: HashSet<IVec3>,
  growing: HashMap<IVec3, Task<Option<Mesh>>>,
  shown: HashMap<IVec3, Option<Entity>>
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct Blades {
  #[uniform(100)]
  dug: Vec4,
  #[uniform(100)]
  weather: Vec4,
  #[uniform(100)]
  tread: Vec4
}

impl MaterialExtension for Blades {
  fn vertex_shader() -> ShaderRef { "embedded://bloxy/sward.wgsl".into() }

  fn fragment_shader() -> ShaderRef { "embedded://bloxy/sward.wgsl".into() }
}

type Sward = ExtendedMaterial<StandardMaterial, Blades>;

#[derive(Resource)]
struct Swards(Handle<Sward>);

fn sow(mut commands: Commands, mut swards: ResMut<Assets<Sward>>) {
  commands.insert_resource(Swards(swards.add(Sward {
    base: StandardMaterial {
      perceptual_roughness: 0.62,
      reflectance: 0.3,
      diffuse_transmission: 0.45,
      cull_mode: None,
      ..default()
    },
    extension: Blades {
      dug: Vec4::ZERO,
      weather: Vec4::new(0.0, 0.0, THINNING_FROM, GONE_AT),
      tread: Vec4::ZERO
    }
  })))
}

fn sway(
  concealed: Res<Concealed>,
  elements: Res<Elements>,
  pilot: Option<Res<Pilot>>,
  sward: Res<Swards>,
  mut swards: ResMut<Assets<Sward>>
) {
  let blades = Blades {
    dug: concealed.0.map_or(Vec4::ZERO, |cell| cell.as_vec3().extend(1.0)),
    weather: Vec4::new(elements.wind, elements.wet, THINNING_FROM, GONE_AT),
    tread: pilot.map_or(Vec4::ZERO, |pilot| pilot.at.extend(1.0))
  };
  let same = |old: &Blades| {
    (old.dug, old.weather, old.tread) == (blades.dug, blades.weather, blades.tread)
  };
  if swards.get(&sward.0).is_some_and(|old| !same(&old.extension))
    && let Some(mut material) = swards.get_mut(&sward.0)
  {
    material.extension = blades
  }
}

fn grow(
  mut commands: Commands,
  mut lawns: ResMut<Lawns>,
  mut progress: ResMut<Progress>,
  voxels: Option<Res<Voxels>>,
  pilot: Option<Res<Pilot>>,
  sward: Res<Swards>,
  mut meshes: ResMut<Assets<Mesh>>
) {
  if let Some(voxels) = voxels
    && let Some(pilot) = pilot
  {
    let centre = chunk_of(pilot.at.floor().as_ivec3());
    let near = |key: IVec3| (key - centre).xz().length_squared() <= REACH * REACH;
    let lawns = &mut *lawns;
    if voxels.is_added() {
      lawns.growing.clear();
      lawns.stale.extend(lawns.shown.keys().copied())
    }
    let mut wanted: Vec<IVec3> = (-REACH..=REACH)
      .flat_map(|x| {
        (-REACH..=REACH).flat_map(move |z| {
          (0..LAYERS).map(move |y| IVec3::new(centre.x + x, y, centre.z + z))
        })
      })
      .filter(|&key| {
        in_world(key)
          && near(key)
          && (lawns.stale.contains(&key) || !lawns.shown.contains_key(&key))
          && !lawns.growing.contains_key(&key)
      })
      .collect();
    wanted.sort_by_key(|&key| {
      ((key - centre).xz().length_squared(), (key.y - centre.y).abs())
    });
    let pool = AsyncComputeTaskPool::get();
    let seed = voxels.seed;
    let room = GROWING.saturating_sub(lawns.growing.len());
    let started: Vec<IVec3> = wanted
      .into_iter()
      .filter_map(|key| {
        Padded::gather(key, |near| voxels.chunks.get(&near).cloned())
          .map(|padded| (key, padded, voxels.torches_near(key)))
      })
      .take(room)
      .map(|(key, padded, torches)| {
        lawns.growing.insert(
          key,
          pool.spawn(async move { mesh::lawn(&padded, key, seed, &torches) })
        );
        key
      })
      .collect();
    for key in started {
      lawns.stale.remove(&key);
    }
    let grown: Vec<(IVec3, Option<Mesh>)> = lawns
      .growing
      .iter_mut()
      .filter_map(|(&key, task)| check_ready(task).map(|mesh| (key, mesh)))
      .collect();
    for (key, mesh) in grown {
      lawns.growing.remove(&key);
      let placed = Transform::from_translation(origin_of(key).as_vec3());
      let shown = mesh.filter(|_| near(key)).map(|mesh| {
        commands
          .spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(sward.0.clone()),
            NotShadowCaster,
            placed
          ))
          .id()
      });
      if let Some(old) = lawns.shown.insert(key, shown).flatten() {
        commands.entity(old).despawn()
      }
    }
    let gone: Vec<IVec3> =
      lawns.shown.keys().copied().filter(|&key| !near(key)).collect();
    for key in gone {
      if let Some(old) = lawns.shown.remove(&key).flatten() {
        commands.entity(old).despawn()
      }
    }
    lawns.stale.retain(|&key| near(key));
    progress.lawns = lawns.growing.len() + lawns.stale.len()
  }
}

pub struct Sowing;

impl Plugin for Sowing {
  fn build(&self, app: &mut App) {
    app.init_resource::<Lawns>();
    if app.world().resource::<Role>().plays() {
      embedded_asset!(app, "sward.wgsl");
      app
        .add_plugins(MaterialPlugin::<Sward>::default())
        .add_systems(Startup, sow)
        .add_systems(Update, grow)
        .add_systems(PostUpdate, sway);
    }
  }
}
