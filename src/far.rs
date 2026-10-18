use {crate::{block::{Block, Fluid},
             generate,
             mesh::sea_tint,
             minimap::hue,
             player::Pilot,
             protocol::plays,
             stream::{Meshed, Palette, Progress},
             voxels::{SIZE, Voxels}},
     bevy::{asset::RenderAssetUsages,
            light::{NotShadowCaster, NotShadowReceiver},
            mesh::{Indices, PrimitiveTopology},
            platform::collections::HashMap,
            prelude::*,
            tasks::{AsyncComputeTaskPool, Task, futures::check_ready}},
     std::sync::{Arc, LazyLock}};

const TILE: i32 = 128;
const COLUMNS: i32 = TILE / SIZE;
pub const FAR: i32 = if cfg!(target_arch = "wasm32") { 900 } else { 1600 };
const FINE_WITHIN: i32 = 480;
const FINE: i32 = 4;
const COARSE: i32 = 8;
const SINK: f32 = 0.7;
const SKIRT: f32 = 10.0;
const WATER_TOP: f32 = 0.88;
const WATER_SINK: f32 = 0.1;
const BUILDING: usize = if cfg!(target_arch = "wasm32") { 1 } else { 6 };
const KEPT_SAMPLES: usize = 900;
const SHADE: f32 = 0.82;

static HUES: LazyLock<Vec<LinearRgba>> = LazyLock::new(|| {
  Block::ALL
    .iter()
    .map(|&block| {
      let [r, g, b] = hue(block);
      LinearRgba::from(Color::srgb(r, g, b))
    })
    .collect()
});

#[derive(Clone, Copy)]
struct Point {
  block: Block,
  top: i32,
  floor: i32
}

impl Point {
  fn water(self) -> bool { matches!(self.block.liquid(), Some((Fluid::Water, _))) }

  fn ground(self) -> f32 {
    match self.water() {
      true => self.floor as f32 + 1.0,
      false => self.top as f32 + 1.0
    }
  }
}

struct Samples {
  step: i32,
  points: Vec<Point>
}

impl Samples {
  fn side(step: i32) -> i32 { TILE / step + 3 }

  fn take(seed: u32, tile: IVec2, step: i32) -> Samples {
    let side = Samples::side(step);
    Samples {
      step,
      points: (0..side * side)
        .map(|index| {
          let spot = tile * TILE + (IVec2::new(index % side, index / side) - 1) * step;
          let (block, top, floor) = generate::overview(seed, spot.x, spot.y);
          Point { block, top, floor }
        })
        .collect()
    }
  }

  fn at(&self, x: i32, z: i32) -> Point {
    self.points[((z + 1) * Samples::side(self.step) + x + 1) as usize]
  }
}

#[derive(Default)]
struct Builder {
  positions: Vec<[f32; 3]>,
  normals: Vec<[f32; 3]>,
  colors: Vec<[f32; 4]>,
  indices: Vec<u32>
}

impl Builder {
  fn quad(&mut self, corners: [(Vec3, Vec3, LinearRgba); 4]) {
    let first = self.positions.len() as u32;
    for (position, normal, color) in corners {
      self.positions.push(position.to_array());
      self.normals.push(normal.to_array());
      self.colors.push(color.to_f32_array())
    }
    self.indices.extend([0, 2, 1, 0, 3, 2].map(|corner| first + corner))
  }

  fn mesh(self) -> Option<Mesh> {
    (!self.indices.is_empty()).then(|| {
      let count = self.positions.len();
      Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; count])
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    })
  }
}

fn build(samples: &Samples, tile: IVec2, near: u16) -> (Option<Mesh>, Option<Mesh>) {
  let step = samples.step;
  let cells = TILE / step;
  let origin = (tile * TILE).as_vec2();
  let shown = |x: i32, z: i32| {
    (0..cells).contains(&x) && (0..cells).contains(&z) && {
      let column = IVec2::new(x, z) * step / SIZE;
      near & 1 << (column.y * COLUMNS + column.x) == 0
    }
  };
  let ground = |x: i32, z: i32| samples.at(x, z).ground() - SINK;
  let position = |x: i32, z: i32| {
    Vec3::new(origin.x + (x * step) as f32, ground(x, z), origin.y + (z * step) as f32)
  };
  let normal = |x: i32, z: i32| {
    Vec3::new(
      ground(x - 1, z) - ground(x + 1, z),
      2.0 * step as f32,
      ground(x, z - 1) - ground(x, z + 1)
    )
    .normalize()
  };
  let color = |x: i32, z: i32| {
    let point = samples.at(x, z);
    let block = match point.water() {
      true => Block::Sand,
      false => point.block
    };
    HUES[block as usize] * SHADE
  };
  let mut land = Builder::default();
  let mut water = Builder::default();
  const SQUARE: [(i32, i32); 4] = [(0, 0), (1, 0), (1, 1), (0, 1)];
  for index in 0..cells * cells {
    let (x, z) = (index % cells, index / cells);
    if shown(x, z) {
      land.quad(SQUARE.map(|(a, b)| {
        (position(x + a, z + b), normal(x + a, z + b), color(x + a, z + b))
      }));
      let wet = SQUARE
        .map(|(a, b)| samples.at(x + a, z + b))
        .into_iter()
        .filter(|point| point.water());
      if let Some(level) = wet.clone().map(|point| point.top).max() {
        let floor = wet.map(|point| point.floor).min().unwrap_or(level) as f32;
        let height = level as f32 + WATER_TOP - WATER_SINK;
        water.quad(SQUARE.map(|(a, b)| {
          let point = position(x + a, z + b).with_y(height);
          let floor = match samples.at(x + a, z + b).water() {
            true => samples.at(x + a, z + b).floor as f32,
            false => floor
          };
          (point, Vec3::Y, sea_tint(floor + 1.0))
        }))
      }
      for (edge, (dx, dz)) in [(0, -1), (1, 0), (0, 1), (-1, 0)].into_iter().enumerate() {
        if !shown(x + dx, z + dz) {
          let (from, to) = (SQUARE[edge], SQUARE[(edge + 1) % 4]);
          let outward = Vec3::new(dx as f32, 0.0, dz as f32);
          let (top_from, top_to) =
            (position(x + from.0, z + from.1), position(x + to.0, z + to.1));
          let (hue_from, hue_to) =
            (color(x + from.0, z + from.1), color(x + to.0, z + to.1));
          land.quad([
            (top_to, outward, hue_to),
            (top_from, outward, hue_from),
            (top_from - Vec3::Y * SKIRT, outward, hue_from),
            (top_to - Vec3::Y * SKIRT, outward, hue_to)
          ])
        }
      }
    }
  }
  (land.mesh(), water.mesh())
}

#[derive(Clone, Copy, PartialEq)]
struct Wanted {
  step: i32,
  near: u16
}

type Built = (Wanted, Arc<Samples>, Option<Mesh>, Option<Mesh>);

#[derive(Resource, Default)]
struct Distant {
  seed: Option<u32>,
  samples: HashMap<(IVec2, i32), Arc<Samples>>,
  shown: HashMap<IVec2, (Wanted, Vec<Entity>)>,
  building: HashMap<IVec2, Task<Built>>
}

#[derive(Resource)]
struct Vista(Handle<StandardMaterial>);

fn prepare(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
  commands.insert_resource(Vista(materials.add(StandardMaterial {
    perceptual_roughness: 0.95,
    reflectance: 0.2,
    ..default()
  })))
}

fn wanted(meshed: &Meshed, at: Vec2) -> Vec<(IVec2, Wanted, f32)> {
  let span = FAR / TILE + 1;
  let home = (at / TILE as f32).floor().as_ivec2();
  let mut tiles: Vec<(IVec2, Wanted, f32)> = (-span..=span)
    .flat_map(|z| (-span..=span).map(move |x| home + IVec2::new(x, z)))
    .filter_map(|tile| {
      let low = (tile * TILE).as_vec2();
      let distance = at.clamp(low, low + TILE as f32).distance(at);
      let near = (0..COLUMNS * COLUMNS).fold(0u16, |near, index| {
        let column = tile * COLUMNS + IVec2::new(index % COLUMNS, index / COLUMNS);
        match meshed.0.contains(&column) {
          true => near | 1 << index,
          false => near
        }
      });
      (distance < FAR as f32 && near != u16::MAX).then_some((
        tile,
        Wanted { step: if distance < FINE_WITHIN as f32 { FINE } else { COARSE }, near },
        distance
      ))
    })
    .collect();
  tiles.sort_by(|a, b| a.2.total_cmp(&b.2));
  tiles
}

fn survey(
  mut commands: Commands,
  mut distant: ResMut<Distant>,
  mut progress: ResMut<Progress>,
  voxels: Option<Res<Voxels>>,
  pilot: Option<Res<Pilot>>,
  meshed: Res<Meshed>,
  vista: Res<Vista>,
  palette: Res<Palette>,
  mut meshes: ResMut<Assets<Mesh>>
) {
  if let Some(voxels) = voxels
    && let Some(pilot) = pilot
  {
    let seed = voxels.seed;
    if distant.seed != Some(seed) {
      for (_, (_, entities)) in distant.shown.drain() {
        for entity in entities {
          commands.entity(entity).despawn()
        }
      }
      distant.samples.clear();
      distant.building.clear();
      distant.seed = Some(seed)
    }
    let tiles = wanted(&meshed, pilot.at.xz());
    let finished: Vec<(IVec2, Built)> = distant
      .building
      .iter_mut()
      .filter_map(|(&tile, task)| check_ready(task).map(|built| (tile, built)))
      .collect();
    for (tile, (want, samples, land, water)) in finished {
      distant.building.remove(&tile);
      distant.samples.insert((tile, want.step), samples);
      let entities: Vec<Entity> = [
        land.map(|mesh| {
          commands
            .spawn((
              Mesh3d(meshes.add(mesh)),
              MeshMaterial3d(vista.0.clone()),
              NotShadowCaster,
              Transform::default()
            ))
            .id()
        }),
        water.map(|mesh| {
          commands
            .spawn((
              Mesh3d(meshes.add(mesh)),
              MeshMaterial3d(palette.liquid.clone()),
              NotShadowCaster,
              NotShadowReceiver,
              Transform::default()
            ))
            .id()
        })
      ]
      .into_iter()
      .flatten()
      .collect();
      for old in
        distant.shown.insert(tile, (want, entities)).into_iter().flat_map(|(_, old)| old)
      {
        commands.entity(old).despawn()
      }
    }
    let gone: Vec<IVec2> = distant
      .shown
      .keys()
      .copied()
      .filter(|tile| !tiles.iter().any(|(wanted, _, _)| wanted == tile))
      .collect();
    for tile in gone {
      for entity in
        distant.shown.remove(&tile).into_iter().flat_map(|(_, entities)| entities)
      {
        commands.entity(entity).despawn()
      }
    }
    let stale: Vec<(IVec2, Wanted)> = tiles
      .iter()
      .filter(|(tile, want, _)| {
        distant.shown.get(tile).is_none_or(|(shown, _)| shown != want)
      })
      .map(|&(tile, want, _)| (tile, want))
      .collect();
    progress.distant = stale.len();
    let pool = AsyncComputeTaskPool::get();
    let room = BUILDING.saturating_sub(distant.building.len());
    let starting: Vec<(IVec2, Wanted)> = stale
      .into_iter()
      .filter(|(tile, _)| !distant.building.contains_key(tile))
      .take(room)
      .collect();
    for (tile, want) in starting {
      let cached = distant.samples.get(&(tile, want.step)).cloned();
      distant.building.insert(
        tile,
        pool.spawn(async move {
          let samples =
            cached.unwrap_or_else(|| Arc::new(Samples::take(seed, tile, want.step)));
          let (land, water) = build(&samples, tile, want.near);
          (want, samples, land, water)
        })
      );
    }
    if distant.samples.len() > KEPT_SAMPLES {
      let home = pilot.at.xz();
      distant.samples.retain(|(tile, _), _| {
        ((*tile * TILE).as_vec2() + TILE as f32 / 2.0).distance(home)
          < (FAR + TILE * 2) as f32
      })
    }
  }
}

pub struct Distance;

impl Plugin for Distance {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Distant>()
      .add_systems(Startup, prepare.run_if(plays))
      .add_systems(Update, survey.run_if(plays));
  }
}
