use {crate::{block::{Block, Look},
             climate,
             ground::{Concealed, Elements},
             noise::{fbm2, smoothstep, unit},
             player::Pilot,
             protocol::Role,
             settings::Settings,
             stream::Progress,
             voxels::{Chunk, LAYERS, SIZE, Voxels, chunk_of}},
     bevy::{asset::{RenderAssetUsages, embedded_asset},
            camera::visibility::NoFrustumCulling,
            light::NotShadowCaster,
            mesh::{Indices, PrimitiveTopology},
            pbr::{ExtendedMaterial, MaterialExtension},
            platform::collections::{HashMap, HashSet},
            prelude::*,
            render::render_resource::{AsBindGroup, Extent3d, TextureDimension,
                                      TextureFormat},
            shader::ShaderRef,
            tasks::{AsyncComputeTaskPool, Task, futures::check_ready}},
     std::sync::Arc};

const SPAN: i32 = 256;
const AROUND: i32 = 3;
const SURVEYING: usize = if cfg!(target_arch = "wasm32") { 1 } else { 6 };
const UNKNOWN: [f32; 4] = [-1.0, f32::MAX, f32::MAX, 0.0];

const BLADES_PER_BLOCK: u32 = 512;
const JOINTS: u32 = 2;
const CLOSE: i32 = 5;
const THICKET: i32 = 48;

fn tufts() -> Mesh {
  let side = CLOSE * 2;
  let per_blade = JOINTS * 2 + 1;
  let blades: Vec<(i32, i32, u32)> = (0..side * side)
    .flat_map(|cell| {
      (0..BLADES_PER_BLOCK)
        .map(move |slot| (cell % side - CLOSE, cell / side - CLOSE, slot))
    })
    .collect();
  let corners = |along: f32, flank: f32, (x, z, slot): (i32, i32, u32)| {
    ([x as f32, along, z as f32], [slot as f32, flank])
  };
  let (positions, slots): (Vec<[f32; 3]>, Vec<[f32; 2]>) = blades
    .iter()
    .flat_map(|&blade| {
      (0..JOINTS)
        .flat_map(move |joint| {
          let along = joint as f32 / JOINTS as f32;
          [corners(along, -1.0, blade), corners(along, 1.0, blade)]
        })
        .chain([corners(1.0, 0.0, blade)])
    })
    .unzip();
  let indices: Vec<u32> = (0..blades.len() as u32)
    .flat_map(|blade| {
      let first = blade * per_blade;
      let tip = first + JOINTS * 2;
      (0..JOINTS - 1)
        .flat_map(move |joint| {
          let (low, high) = (first + joint * 2, first + joint * 2 + 2);
          [low, low + 1, high + 1, low, high + 1, high]
        })
        .chain([tip - 2, tip - 1, tip])
    })
    .collect();
  let count = positions.len();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, vec![[0.0f32; 2]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32; 4]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, slots)
    .with_inserted_indices(Indices::U32(indices))
}

const FACES: [[[f32; 3]; 4]; 5] = [
  [[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 1.0], [1.0, 1.0, 0.0]],
  [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 1.0, 0.0]],
  [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0]],
  [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
  [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]]
];

fn volumes() -> Mesh {
  let side = THICKET * 2;
  let quads: Vec<(f32, f32, usize)> = (0..side * side)
    .flat_map(|cell| {
      (0..FACES.len()).map(move |face| {
        ((cell % side - THICKET) as f32, (cell / side - THICKET) as f32, face)
      })
    })
    .collect();
  let positions: Vec<[f32; 3]> =
    quads.iter().flat_map(|&(_, _, face)| FACES[face]).collect();
  let blocks: Vec<[f32; 2]> = quads.iter().flat_map(|&(x, z, _)| [[x, z]; 4]).collect();
  let faces: Vec<[f32; 2]> =
    quads.iter().flat_map(|&(_, _, face)| [[face as f32, 0.0]; 4]).collect();
  let indices: Vec<u32> = (0..quads.len() as u32)
    .flat_map(|quad| [0, 1, 2, 0, 2, 3].map(|corner| quad * 4 + corner))
    .collect();
  let count = positions.len();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, blocks)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, faces)
    .with_inserted_indices(Indices::U32(indices))
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct Blades {
  #[uniform(100)]
  dug: Vec4,
  #[uniform(100)]
  weather: Vec4,
  #[uniform(100)]
  tread: Vec4,
  #[texture(
    101,
    sample_type = "float",
    filterable = false,
    visibility(vertex, fragment)
  )]
  tops: Handle<Image>
}

impl MaterialExtension for Blades {
  fn vertex_shader() -> ShaderRef { "embedded://bloxy/sward.wgsl".into() }

  fn fragment_shader() -> ShaderRef { "embedded://bloxy/sward.wgsl".into() }
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct Thicket {
  #[uniform(100)]
  dug: Vec4,
  #[uniform(100)]
  weather: Vec4,
  #[uniform(100)]
  tread: Vec4,
  #[texture(
    101,
    sample_type = "float",
    filterable = false,
    visibility(vertex, fragment)
  )]
  tops: Handle<Image>
}

impl MaterialExtension for Thicket {
  fn vertex_shader() -> ShaderRef { "embedded://bloxy/thicket.wgsl".into() }

  fn fragment_shader() -> ShaderRef { "embedded://bloxy/thicket.wgsl".into() }
}

type Sward = ExtendedMaterial<StandardMaterial, Blades>;
type Thick = ExtendedMaterial<StandardMaterial, Thicket>;

#[derive(Resource)]
struct Meadow {
  sward: Handle<Sward>,
  thicket: Handle<Thick>,
  tops: Handle<Image>
}

#[derive(Component)]
struct Swath;

#[derive(Resource, Default)]
pub struct Lawns {
  pub stale: HashSet<IVec3>,
  surveying: HashMap<IVec2, Task<Vec<[f32; 2]>>>,
  outdated: HashSet<IVec2>,
  surveyed: HashSet<IVec2>
}

fn ground(block: Block) -> bool {
  block != Block::Air
    && !block.leafy()
    && block.look() != Look::Log
    && (block.solid() || block.liquid().is_some())
}

fn swarded(seed: u32, x: i32, z: i32) -> f32 {
  let wet = climate::wet(seed, Vec2::new(x as f32, z as f32));
  let lush =
    (0.12 + 0.28 * smoothstep(-0.3, 0.4, wet)) * smoothstep(-0.6, -0.1, wet).max(0.3);
  let sward = fbm2(seed.wrapping_add(43), x as f32 / 11.0, z as f32 / 11.0, 2) + 0.5;
  f32::from(u8::from(unit(seed ^ 0xF13, x, 4, z) < lush * sward.clamp(0.0, 1.0)))
}

fn survey(
  seed: u32,
  column: IVec2,
  layers: [Arc<Chunk>; LAYERS as usize]
) -> Vec<[f32; 2]> {
  let at = |x: i32, y: i32, z: i32| {
    (y < LAYERS * SIZE)
      .then(|| layers[(y / SIZE) as usize].get(IVec3::new(x, y % SIZE, z)))
      .unwrap_or(Block::Air)
  };
  (0..SIZE * SIZE)
    .map(|index| {
      let (x, z) = (index % SIZE, index / SIZE);
      (0..LAYERS * SIZE)
        .rev()
        .find(|&y| ground(at(x, y, z)))
        .filter(|&y| at(x, y, z) == Block::Grass && !at(x, y + 1, z).solid())
        .map_or([-1.0, 0.0], |y| {
          let at = column * SIZE + IVec2::new(x, z);
          [(y + 1) as f32, swarded(seed, at.x, at.y)]
        })
    })
    .collect()
}

fn sow(
  mut commands: Commands,
  mut images: ResMut<Assets<Image>>,
  mut swards: ResMut<Assets<Sward>>,
  mut thickets: ResMut<Assets<Thick>>,
  mut meshes: ResMut<Assets<Mesh>>
) {
  let tops = images.add(Image::new_fill(
    Extent3d { width: SPAN as u32, height: SPAN as u32, depth_or_array_layers: 1 },
    TextureDimension::D2,
    &UNKNOWN.iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<u8>>(),
    TextureFormat::Rgba32Float,
    RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD
  ));
  let base = StandardMaterial {
    perceptual_roughness: 0.6,
    reflectance: 0.2,
    diffuse_transmission: 0.3,
    ..default()
  };
  let sward = swards.add(Sward {
    base: StandardMaterial { cull_mode: None, ..base.clone() },
    extension: Blades {
      dug: Vec4::ZERO,
      weather: Vec4::ZERO,
      tread: Vec4::ZERO,
      tops: tops.clone()
    }
  });
  let thicket = thickets.add(Thick {
    base,
    extension: Thicket {
      dug: Vec4::ZERO,
      weather: Vec4::ZERO,
      tread: Vec4::ZERO,
      tops: tops.clone()
    }
  });
  commands.spawn((
    Swath,
    Mesh3d(meshes.add(tufts())),
    MeshMaterial3d(sward.clone()),
    NotShadowCaster,
    NoFrustumCulling,
    Transform::default()
  ));
  commands.spawn((
    Swath,
    Mesh3d(meshes.add(volumes())),
    MeshMaterial3d(thicket.clone()),
    NotShadowCaster,
    NoFrustumCulling,
    Transform::default()
  ));
  commands.insert_resource(Meadow { sward, thicket, tops })
}

fn follow(pilot: Option<Res<Pilot>>, mut swaths: Query<&mut Transform, With<Swath>>) {
  if let Some(pilot) = pilot {
    let snapped = pilot.at.floor().with_y(0.0);
    for mut transform in
      swaths.iter_mut().filter(|transform| transform.translation != snapped)
    {
      transform.translation = snapped
    }
  }
}

fn sway(
  concealed: Res<Concealed>,
  elements: Res<Elements>,
  pilot: Option<Res<Pilot>>,
  meadow: Res<Meadow>,
  mut swards: ResMut<Assets<Sward>>,
  mut thickets: ResMut<Assets<Thick>>
) {
  let (dug, weather, tread) = (
    concealed.0.map_or(Vec4::ZERO, |cell| cell.as_vec3().extend(1.0)),
    Vec4::new(elements.wind, elements.wet, 0.0, 0.0),
    pilot.map_or(Vec4::ZERO, |pilot| pilot.at.extend(1.0))
  );
  if swards.get(&meadow.sward).is_some_and(|old| {
    (old.extension.dug, old.extension.weather, old.extension.tread)
      != (dug, weather, tread)
  }) && let Some(mut sward) = swards.get_mut(&meadow.sward)
    && let Some(mut thicket) = thickets.get_mut(&meadow.thicket)
  {
    sward.extension.dug = dug;
    sward.extension.weather = weather;
    sward.extension.tread = tread;
    thicket.extension.dug = dug;
    thicket.extension.weather = weather;
    thicket.extension.tread = tread
  }
}

fn grow(
  mut lawns: ResMut<Lawns>,
  mut progress: ResMut<Progress>,
  voxels: Option<Res<Voxels>>,
  pilot: Option<Res<Pilot>>,
  meadow: Res<Meadow>,
  mut images: ResMut<Assets<Image>>
) {
  if let Some(voxels) = voxels
    && let Some(pilot) = pilot
  {
    let lawns = &mut *lawns;
    let centre = chunk_of(pilot.at.floor().as_ivec3()).xz();
    let near = |column: IVec2| (column - centre).abs().max_element() <= AROUND;
    if voxels.is_added() {
      lawns.surveying.clear();
      lawns.outdated.clear();
      lawns.surveyed.clear()
    }
    for key in lawns.stale.drain() {
      lawns.surveyed.remove(&key.xz());
      if lawns.surveying.contains_key(&key.xz()) {
        lawns.outdated.insert(key.xz());
      }
    }
    lawns.surveyed.retain(|&column| near(column));
    let mut wanted: Vec<IVec2> = (-AROUND..=AROUND)
      .flat_map(|x| (-AROUND..=AROUND).map(move |z| centre + IVec2::new(x, z)))
      .filter(|column| {
        !lawns.surveyed.contains(column) && !lawns.surveying.contains_key(column)
      })
      .collect();
    wanted.sort_by_key(|column| (*column - centre).length_squared());
    let pool = AsyncComputeTaskPool::get();
    let room = SURVEYING.saturating_sub(lawns.surveying.len());
    let seed = voxels.seed;
    let ready: Vec<(IVec2, [Arc<Chunk>; LAYERS as usize])> = wanted
      .iter()
      .filter_map(|&column| {
        let layers: Option<Vec<Arc<Chunk>>> = (0..LAYERS)
          .map(|y| voxels.chunks.get(&column.extend(y).xzy()).cloned())
          .collect();
        layers.and_then(|layers| layers.try_into().ok()).map(|layers| (column, layers))
      })
      .collect();
    let waiting = ready.len();
    for (column, layers) in ready.into_iter().take(room) {
      lawns
        .surveying
        .insert(column, pool.spawn(async move { survey(seed, column, layers) }));
    }
    let surveyed: Vec<(IVec2, Vec<[f32; 2]>)> = lawns
      .surveying
      .iter_mut()
      .filter_map(|(&column, task)| check_ready(task).map(|heights| (column, heights)))
      .collect();
    if !surveyed.is_empty()
      && let Some(mut image) = images.get_mut(&meadow.tops)
      && let Some(data) = image.data.as_mut()
    {
      for (column, heights) in surveyed {
        lawns.surveying.remove(&column);
        let current = !lawns.outdated.remove(&column);
        if near(column) {
          if current {
            lawns.surveyed.insert(column);
          }
          for (index, [height, tall]) in heights.into_iter().enumerate() {
            let at = column * SIZE + IVec2::new(index as i32 % SIZE, index as i32 / SIZE);
            let texel = at.rem_euclid(IVec2::splat(SPAN));
            let offset = ((texel.y * SPAN + texel.x) * 16) as usize;
            let cell = [height, at.x as f32, at.y as f32, tall];
            data[offset..offset + 16].copy_from_slice(
              &cell.iter().flat_map(|value| value.to_le_bytes()).collect::<Vec<u8>>()
            );
          }
        }
      }
    }
    progress.lawns = lawns.surveying.len() + waiting
  }
}

fn bladed(settings: Res<Settings>) -> bool { settings.blades }

fn mow(
  settings: Res<Settings>,
  mut swaths: Query<&mut Visibility, With<Swath>>,
  mut grounds: ResMut<Assets<crate::ground::Ground>>,
  mut progress: ResMut<Progress>
) {
  if settings.is_changed() {
    for mut visibility in swaths.iter_mut() {
      visibility.set_if_neq(match settings.blades {
        true => Visibility::Inherited,
        false => Visibility::Hidden
      });
    }
    for (_, ground) in grounds.iter_mut() {
      ground.extension.turf.z = f32::from(u8::from(!settings.blades))
    }
    if !settings.blades {
      progress.lawns = 0
    }
  }
}

pub struct Sowing;

impl Plugin for Sowing {
  fn build(&self, app: &mut App) {
    app.init_resource::<Lawns>();
    if app.world().resource::<Role>().plays() {
      embedded_asset!(app, "sward.wgsl");
      embedded_asset!(app, "thicket.wgsl");
      app
        .add_plugins((
          MaterialPlugin::<Sward>::default(),
          MaterialPlugin::<Thick>::default()
        ))
        .add_systems(Startup, sow)
        .add_systems(Update, (mow, grow.run_if(bladed)))
        .add_systems(PostUpdate, (follow, sway));
    }
  }
}
