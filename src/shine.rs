use {crate::{block::Block,
             ground::Ground,
             player::Pilot,
             protocol::plays,
             stream::Progress,
             voxels::{Chunk, SIZE, Voxels, chunk_of, origin_of}},
     bevy::{asset::RenderAssetUsages,
            image::{ImageAddressMode, ImageFilterMode, ImageSampler,
                    ImageSamplerDescriptor},
            platform::collections::{HashMap, HashSet},
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
            tasks::{AsyncComputeTaskPool, Task, futures::check_ready}},
     std::sync::Arc};

pub const SPAN: IVec3 = IVec3::new(128, 64, 128);
const STEP: i32 = 16;
const REACH: i32 = 14;
const BRIGHTEST: f32 = 6.0;
const UNIT: u16 = 8;
const MOVES: [(IVec3, u16); 26] = {
  let mut moves = [(IVec3::ZERO, 0); 26];
  let mut index = 0;
  let mut code = 0;
  while code < 27 {
    let way = IVec3::new(code % 3 - 1, code / 3 % 3 - 1, code / 9 - 1);
    let axes = way.x.abs() + way.y.abs() + way.z.abs();
    if axes > 0 {
      moves[index] = (way, [0, 8, 11, 14][axes as usize]);
      index += 1
    }
    code += 1
  }
  moves
};

#[derive(Clone, Copy)]
struct Radiance {
  color: Vec3,
  reach: f32
}

fn radiance(block: Block) -> Option<Radiance> {
  let glow = |r: f32, g: f32, b: f32, reach: f32| {
    Some(Radiance { color: Vec3::new(r, g, b), reach })
  };
  match block {
    _ if block.torch() => glow(3.0, 1.4, 0.4, 13.0),
    Block::Brazier => glow(3.4, 1.5, 0.35, 14.0),
    Block::Lumen => glow(0.4, 1.4, 2.2, 9.0),
    Block::ElectricLampLit => glow(2.8, 2.5, 1.8, 13.0),
    Block::AetherLamp => glow(0.6, 1.6, 2.6, 11.0),
    Block::Biolume => glow(0.4, 1.6, 1.2, 7.0),
    Block::Glowkelp => glow(0.4, 1.4, 0.8, 5.0),
    _ => None
  }
}

static NATURES: std::sync::LazyLock<Vec<(bool, Option<Radiance>)>> =
  std::sync::LazyLock::new(|| {
    let mut natures = vec![
      (false, None);
      Block::ALL.len().max(
        1 + Block::ALL.iter().map(|&block| block as usize).max().unwrap_or(0)
      )
    ];
    for block in Block::ALL {
      natures[block as usize] = (block.opaque(), radiance(block))
    }
    natures
  });

struct Opacity {
  low: IVec3,
  span: IVec3,
  opaque: Vec<bool>
}

impl Opacity {
  fn index(&self, at: IVec3) -> Option<usize> {
    let inside = at - self.low;
    (inside.cmpge(IVec3::ZERO).all() && inside.cmplt(self.span).all())
      .then(|| ((inside.z * self.span.y + inside.y) * self.span.x + inside.x) as usize)
  }

  fn opaque(&self, at: IVec3) -> bool {
    self.index(at).is_some_and(|index| self.opaque[index])
  }
}

fn survey(
  chunks: &HashMap<IVec3, Arc<Chunk>>,
  low: IVec3,
  span: IVec3
) -> (Opacity, Vec<(IVec3, Radiance)>) {
  let mut opacity =
    Opacity { low, span, opaque: vec![false; (span.x * span.y * span.z) as usize] };
  let mut sources = Vec::new();
  let high = low + span - 1;
  let natures = &*NATURES;
  for (&key, chunk) in chunks.iter() {
    let origin = origin_of(key);
    let (from, to) = (origin.max(low), (origin + SIZE - 1).min(high));
    let blocks: &[Block] = match chunk.as_ref() {
      Chunk::Uniform(block) => std::slice::from_ref(block),
      Chunk::Mixed(blocks) => &blocks[..]
    };
    let uniform = blocks.len() == 1;
    let (opaque, radiant) = natures[blocks[0] as usize];
    let empty = uniform && !opaque && radiant.is_none();
    if !empty && from.cmple(to).all() {
      for z in from.z..=to.z {
        for y in from.y..=to.y {
          let row = ((z - low.z) * span.y + y - low.y) * span.x - low.x;
          for x in from.x..=to.x {
            let block = match uniform {
              true => blocks[0],
              false => blocks[Chunk::index(IVec3::new(x, y, z) - origin)]
            };
            let (opaque, radiant) = natures[block as usize];
            opacity.opaque[(row + x) as usize] = opaque;
            if let Some(glow) = radiant {
              sources.push((IVec3::new(x, y, z), glow))
            }
          }
        }
      }
    }
  }
  (opacity, sources)
}

const WINDOW: i32 = 2 * REACH + 3;

struct Step {
  offset: isize,
  cost: u16,
  guards: Vec<isize>
}

static STEPS: std::sync::LazyLock<Vec<Step>> = std::sync::LazyLock::new(|| {
  let flat = |way: IVec3| (way.x + (way.y + way.z * WINDOW) * WINDOW) as isize;
  MOVES
    .iter()
    .map(|&(way, cost)| Step {
      offset: flat(way),
      cost,
      guards: match way.abs().element_sum() {
        1 => Vec::new(),
        _ => [IVec3::X, IVec3::Y, IVec3::Z]
          .into_iter()
          .filter(|axis| way.dot(*axis) != 0)
          .map(|axis| flat(way * axis))
          .collect()
      }
    })
    .collect()
});

struct Flood {
  opaque: Vec<bool>,
  distance: Vec<u16>,
  buckets: Vec<Vec<usize>>
}

impl Flood {
  fn new() -> Self {
    let cells = (WINDOW * WINDOW * WINDOW) as usize;
    Flood {
      opaque: vec![true; cells],
      distance: vec![u16::MAX; cells],
      buckets: vec![Vec::new(); (REACH as u16 * UNIT) as usize + 1]
    }
  }

  fn spread(
    &mut self,
    opacity: &Opacity,
    from: IVec3,
    reach: f32,
    mut lit: impl FnMut(IVec3, f32)
  ) {
    let corner = from - (REACH + 1);
    for z in 1..WINDOW - 1 {
      for y in 1..WINDOW - 1 {
        for x in 1..WINDOW - 1 {
          self.opaque[((z * WINDOW + y) * WINDOW + x) as usize] =
            opacity.opaque(corner + IVec3::new(x, y, z))
        }
      }
    }
    let centre = (((REACH + 1) * WINDOW + REACH + 1) * WINDOW + REACH + 1) as usize;
    self.opaque[centre] = false;
    let most = ((reach.min(REACH as f32) * UNIT as f32) as u16).max(1);
    let mut reached = vec![centre];
    self.distance[centre] = 0;
    self.buckets[0].push(centre);
    for bucket in 0..=most {
      let mut cells = std::mem::take(&mut self.buckets[bucket as usize]);
      for &here in cells.iter() {
        if self.distance[here] == bucket {
          let local = IVec3::new(
            here as i32 % WINDOW,
            here as i32 / WINDOW % WINDOW,
            here as i32 / (WINDOW * WINDOW)
          );
          lit(corner + local, bucket as f32 / most as f32);
          for Step { offset, cost, guards } in STEPS.iter() {
            let there = (here as isize + offset) as usize;
            let further = bucket + cost;
            if further <= most
              && !self.opaque[there]
              && further < self.distance[there]
              && !guards.iter().any(|guard| self.opaque[(here as isize + guard) as usize])
            {
              if self.distance[there] == u16::MAX {
                reached.push(there)
              }
              self.distance[there] = further;
              self.buckets[further as usize].push(there)
            }
          }
        }
      }
      cells.clear();
      self.buckets[bucket as usize] = cells
    }
    for cell in reached {
      self.distance[cell] = u16::MAX
    }
  }
}

fn encode(light: f32) -> u8 {
  match light <= 0.0 {
    true => 0,
    false => encoded(light)
  }
}

fn encoded(light: f32) -> u8 {
  let linear = 1.0 - (-light / BRIGHTEST).exp();
  (linear.sqrt() * 255.0).round() as u8
}

struct Kindled {
  light: Vec<u8>,
  sources: Vec<IVec3>
}

fn kindle(chunks: HashMap<IVec3, Arc<Chunk>>, origin: IVec3) -> Kindled {
  let (opacity, sources) = survey(&chunks, origin - REACH, SPAN + 2 * REACH);
  let cells = (SPAN.x * SPAN.y * SPAN.z) as usize;
  let mut light = vec![Vec3::ZERO; cells];
  let mut flood = Flood::new();
  let kept: Vec<IVec3> = sources.iter().map(|(at, _)| *at).collect();
  for (at, Radiance { color, reach }) in sources {
    flood.spread(&opacity, at, reach, |cell, along| {
      let inside = cell - origin;
      if inside.cmpge(IVec3::ZERO).all() && inside.cmplt(SPAN).all() {
        let fading = (1.0 - along) * (1.0 - along) / (1.0 + 20.0 * along * along);
        light[((inside.z * SPAN.y + inside.y) * SPAN.x + inside.x) as usize] +=
          color * fading
      }
    })
  }
  let mut bytes = vec![0; cells * 4];
  for (texel, light) in bytes.chunks_exact_mut(4).zip(light.iter()) {
    if *light != Vec3::ZERO {
      texel.copy_from_slice(&[encode(light.x), encode(light.y), encode(light.z), 255])
    }
  }
  Kindled { light: bytes, sources: kept }
}

fn blank() -> Image {
  let mut image = Image::new_fill(
    Extent3d {
      width: SPAN.x as u32,
      height: SPAN.y as u32,
      depth_or_array_layers: SPAN.z as u32
    },
    TextureDimension::D3,
    &[0, 0, 0, 0],
    TextureFormat::Rgba8Unorm,
    RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD
  );
  image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
    address_mode_u: ImageAddressMode::ClampToEdge,
    address_mode_v: ImageAddressMode::ClampToEdge,
    address_mode_w: ImageAddressMode::ClampToEdge,
    mag_filter: ImageFilterMode::Linear,
    min_filter: ImageFilterMode::Linear,
    ..default()
  });
  image
}

#[derive(Resource)]
pub struct Shine {
  pub image: Handle<Image>,
  shown: Option<IVec3>,
  stale: bool,
  sources: Vec<IVec3>,
  kindling: Option<(IVec3, Task<Kindled>)>
}

pub fn setup(mut images: ResMut<Assets<Image>>, mut commands: Commands) {
  commands.insert_resource(Shine {
    image: images.add(blank()),
    shown: None,
    stale: true,
    sources: Vec::new(),
    kindling: None
  })
}

fn centred(eye: Vec3) -> IVec3 {
  let corner = eye.floor().as_ivec3() - SPAN / 2;
  IVec3::new(
    corner.x.div_euclid(STEP) * STEP,
    corner.y.div_euclid(STEP) * STEP,
    corner.z.div_euclid(STEP) * STEP
  )
}

fn relight(
  pilot: Res<Pilot>,
  mut voxels: ResMut<Voxels>,
  mut shine: ResMut<Shine>,
  mut images: ResMut<Assets<Image>>,
  mut grounds: ResMut<Assets<Ground>>,
  mut progress: ResMut<Progress>
) {
  let wanted = centred(pilot.eye());
  let touched: HashSet<IVec3> = voxels.relit.get_or_insert_default().drain().collect();
  let shining = |key: &IVec3| {
    let (low, high) = (origin_of(*key) - REACH, origin_of(*key) + SIZE + REACH);
    shine.sources.iter().any(|at| at.cmpge(low).all() && at.cmplt(high).all())
      || voxels.chunks.get(key).is_some_and(|chunk| match chunk.as_ref() {
        Chunk::Uniform(block) => NATURES[*block as usize].1.is_some(),
        Chunk::Mixed(blocks) => {
          blocks.iter().any(|block| NATURES[*block as usize].1.is_some())
        }
      })
  };
  let telling: Vec<IVec3> = touched.into_iter().filter(shining).collect();
  let reaches = |origin: IVec3| {
    let (low, high) = (chunk_of(origin - REACH), chunk_of(origin + SPAN + REACH));
    telling.iter().any(|key| key.cmpge(low).all() && key.cmple(high).all())
  };
  let aiming = shine.kindling.as_ref().map(|(origin, _)| *origin).or(shine.shown);
  let altered = [shine.shown, shine.kindling.as_ref().map(|(origin, _)| *origin)]
    .into_iter()
    .flatten()
    .any(reaches);
  shine.stale = shine.stale || aiming != Some(wanted) || altered;
  let finished = shine
    .kindling
    .as_mut()
    .and_then(|(origin, task)| check_ready(task).map(|light| (*origin, light)));
  if let Some((origin, Kindled { light, sources })) = finished {
    shine.kindling = None;
    shine.shown = Some(origin);
    shine.sources = sources;
    if let Some(mut image) = images.get_mut(&shine.image) {
      image.data = Some(light)
    }
    let shone = origin.as_vec3().extend(1.0);
    for (_, ground) in grounds.iter_mut() {
      ground.extension.shone = shone;
      ground.extension.shine = Some(shine.image.clone())
    }
  }
  if shine.stale && shine.kindling.is_none() {
    let (low, high) = (chunk_of(wanted - REACH), chunk_of(wanted + SPAN + REACH));
    let chunks: HashMap<IVec3, Arc<Chunk>> = voxels
      .chunks
      .iter()
      .filter(|(key, _)| key.cmpge(low).all() && key.cmple(high).all())
      .map(|(&key, chunk)| (key, chunk.clone()))
      .collect();
    shine.stale = false;
    shine.kindling = Some((
      wanted,
      AsyncComputeTaskPool::get().spawn(async move { kindle(chunks, wanted) })
    ))
  }
  progress.shine = usize::from(shine.kindling.is_some() || shine.stale)
}

pub struct Shining;

impl Plugin for Shining {
  fn build(&self, app: &mut App) {
    app.add_systems(Startup, setup.run_if(plays)).add_systems(
      Update,
      relight
        .run_if(plays)
        .run_if(resource_exists::<Voxels>)
        .run_if(resource_exists::<Pilot>)
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn spread(opacity: &Opacity, from: IVec3, reach: f32, lit: impl FnMut(IVec3, f32)) {
    Flood::new().spread(opacity, from, reach, lit)
  }

  #[test]
  fn walls_hold_light_back() {
    let span = IVec3::splat(40);
    let low = IVec3::splat(-20);
    let mut opacity =
      Opacity { low, span, opaque: vec![false; (span.x * span.y * span.z) as usize] };
    for y in -20..20 {
      for z in -20..20 {
        let index = opacity.index(IVec3::new(3, y, z)).unwrap();
        opacity.opaque[index] = true
      }
    }
    let mut seen: HashMap<IVec3, f32> = HashMap::default();
    spread(&opacity, IVec3::ZERO, 10.0, |at, along| {
      seen.insert(at, along);
    });
    assert!(seen.contains_key(&IVec3::new(2, 0, 0)));
    assert!(!seen.contains_key(&IVec3::new(3, 0, 0)));
    assert!(!seen.contains_key(&IVec3::new(4, 0, 0)));
    let diagonal = seen[&IVec3::new(-4, 4, 0)];
    assert!((diagonal - 32f32.sqrt() / 10.0).abs() < 0.06, "{diagonal}");
  }

  #[test]
  #[ignore]
  fn kindling_the_home_dungeon_takes() {
    let origin = IVec3::new(0, 0, 0);
    let kept = crate::voxels::Kept::default();
    let chunks: HashMap<IVec3, Arc<Chunk>> = (-1..6)
      .flat_map(|x| (0..4).flat_map(move |y| (-1..6).map(move |z| IVec3::new(x, y, z))))
      .map(|key| (key, Arc::new(crate::voxels::generated(1, &kept, key))))
      .collect();
    let started = std::time::Instant::now();
    let light = kindle(chunks, origin).light;
    let lit = light.chunks(4).filter(|texel| texel[0] > 0 || texel[2] > 0).count();
    println!("kindled in {:?}, {lit} lit texels", started.elapsed());
  }
}
