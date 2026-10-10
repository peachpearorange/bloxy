use {crate::{block::Block, generate, mesh::TORCH_REACH, model},
     bevy::{platform::collections::{HashMap, HashSet},
            prelude::*},
     std::sync::Arc};

pub const SIZE: i32 = 32;
pub const VOLUME: usize = (SIZE * SIZE * SIZE) as usize;
pub const LAYERS: i32 = 8;
pub const HEIGHT: i32 = SIZE * LAYERS;

#[derive(Clone)]
pub enum Chunk {
  Uniform(Block),
  Mixed(Box<[Block; VOLUME]>)
}

impl Chunk {
  pub fn index(local: IVec3) -> usize {
    ((local.y * SIZE + local.z) * SIZE + local.x) as usize
  }

  pub fn get(&self, local: IVec3) -> Block {
    match self {
      Chunk::Uniform(block) => *block,
      Chunk::Mixed(blocks) => blocks[Chunk::index(local)]
    }
  }

  pub fn set(&mut self, local: IVec3, block: Block) {
    match self {
      Chunk::Uniform(filled) if *filled == block => (),
      Chunk::Uniform(filled) => {
        let mut blocks = Box::new([*filled; VOLUME]);
        blocks[Chunk::index(local)] = block;
        *self = Chunk::Mixed(blocks)
      }
      Chunk::Mixed(blocks) => blocks[Chunk::index(local)] = block
    }
  }

  pub fn settled(self) -> Chunk {
    match self {
      Chunk::Mixed(blocks) if blocks.iter().all(|&block| block == blocks[0]) => {
        Chunk::Uniform(blocks[0])
      }
      chunk => chunk
    }
  }
}

pub fn chunk_of(at: IVec3) -> IVec3 { at.div_euclid(IVec3::splat(SIZE)) }

pub fn local_of(at: IVec3) -> IVec3 { at.rem_euclid(IVec3::splat(SIZE)) }

pub fn origin_of(key: IVec3) -> IVec3 { key * SIZE }

pub fn in_world(key: IVec3) -> bool { (0..LAYERS).contains(&key.y) }

pub fn beyond(at: IVec3) -> Option<Block> {
  (at.y < 0).then_some(Block::Bedrock).or((at.y >= HEIGHT).then_some(Block::Air))
}

pub type Edits = HashMap<IVec3, HashMap<IVec3, Block>>;

pub const COLUMN: usize = (SIZE * SIZE * HEIGHT) as usize;

pub type Kept = Arc<HashMap<IVec2, Arc<Vec<Block>>>>;

fn spot(local: IVec3) -> usize { ((local.y * SIZE + local.z) * SIZE + local.x) as usize }

pub fn generated(seed: u32, kept: &Kept, key: IVec3) -> Chunk {
  match kept.get(&key.xz()) {
    Some(column) => {
      let floor = key.y * SIZE;
      Chunk::Mixed(Box::new(std::array::from_fn(|index| {
        let index = index as i32;
        let local = IVec3::new(index % SIZE, index / (SIZE * SIZE), index / SIZE % SIZE);
        column[spot(local + IVec3::Y * floor)]
      })))
      .settled()
    }
    None => generate::chunk(seed, key)
  }
}

pub fn pack(column: &[Block]) -> Vec<u8> {
  miniz_oxide::deflate::compress_to_vec(
    &column.iter().map(|&block| block as u8).collect::<Vec<u8>>(),
    6
  )
}

pub fn unpack(packed: &[u8]) -> Option<Vec<Block>> {
  miniz_oxide::inflate::decompress_to_vec(packed)
    .ok()
    .filter(|bytes| bytes.len() == COLUMN)
    .map(|bytes| {
      bytes
        .into_iter()
        .map(|byte| Block::ALL.get(usize::from(byte)).copied().unwrap_or_default())
        .collect()
    })
}

fn pierces(from: Vec3, toward: Vec3, low: Vec3, high: Vec3) -> bool {
  let (near, far) =
    (0..3).fold((0.0_f32, f32::INFINITY), |(near, far), axis| {
      match toward[axis] == 0.0 {
        true => match (low[axis]..=high[axis]).contains(&from[axis]) {
          true => (near, far),
          false => (1.0, 0.0)
        },
        false => {
          let (first, second) = (
            (low[axis] - from[axis]) / toward[axis],
            (high[axis] - from[axis]) / toward[axis]
          );
          (near.max(first.min(second)), far.min(first.max(second)))
        }
      }
    });
  near <= far
}

pub struct Hit {
  pub at: IVec3,
  pub normal: IVec3,
  pub block: Block
}

#[derive(Resource)]
pub struct Voxels {
  pub seed: u32,
  pub chunks: HashMap<IVec3, Arc<Chunk>>,
  pub edits: Edits,
  pub dirty: HashSet<IVec3>,
  pub kept: Kept,
  pub touched: HashSet<IVec2>
}

impl Voxels {
  pub fn new(seed: u32) -> Self {
    Self {
      seed,
      chunks: default(),
      edits: default(),
      dirty: default(),
      kept: default(),
      touched: default()
    }
  }

  pub fn block(&self, at: IVec3) -> Option<Block> {
    beyond(at)
      .or_else(|| self.chunks.get(&chunk_of(at)).map(|chunk| chunk.get(local_of(at))))
  }

  pub fn solid(&self, at: IVec3) -> bool { self.block(at).is_none_or(Block::solid) }

  pub fn insert(&mut self, key: IVec3, mut chunk: Chunk) {
    self
      .edits
      .get(&key)
      .into_iter()
      .flatten()
      .for_each(|(&local, &block)| chunk.set(local, block));
    self.chunks.insert(key, Arc::new(chunk));
    self.touch(key)
  }

  pub fn ensure(&mut self, at: IVec3) -> Block {
    let key = chunk_of(at);
    if in_world(key) && !self.chunks.contains_key(&key) {
      self.insert(key, generated(self.seed, &self.kept, key))
    }
    self.block(at).unwrap_or_default()
  }

  fn touch(&mut self, key: IVec3) {
    (-1..=1)
      .flat_map(|x| {
        (-1..=1).flat_map(move |y| (-1..=1).map(move |z| IVec3::new(x, y, z)))
      })
      .for_each(|offset| {
        self.dirty.insert(key + offset);
      })
  }

  pub fn set(&mut self, at: IVec3, block: Block) {
    let (key, local) = (chunk_of(at), local_of(at));
    if in_world(key) {
      self.touched.insert(key.xz());
      self.edits.entry(key).or_default().insert(local, block);
      if let Some(chunk) = self.chunks.get_mut(&key) {
        let lit = block == Block::Torch || chunk.get(local) == Block::Torch;
        Arc::make_mut(chunk).set(local, block);
        let near_edge = |axis: i32| axis == 0 || axis == SIZE - 1;
        match lit || local.to_array().into_iter().any(near_edge) {
          true => self.touch(key),
          false => {
            self.dirty.insert(key);
          }
        }
      }
    }
  }

  pub fn torches_near(&self, key: IVec3) -> Vec<IVec3> {
    let (low, high) = (
      origin_of(key).as_vec3() - TORCH_REACH,
      origin_of(key + IVec3::ONE).as_vec3() + TORCH_REACH
    );
    (-1..=1)
      .flat_map(|x| {
        (-1..=1).flat_map(move |y| (-1..=1).map(move |z| IVec3::new(x, y, z)))
      })
      .flat_map(|offset| {
        let near = key + offset;
        self.edits.get(&near).into_iter().flatten().filter_map(move |(&local, &block)| {
          (block == Block::Torch).then_some(origin_of(near) + local)
        })
      })
      .filter(|torch| {
        let centre = torch.as_vec3() + 0.5;
        centre.cmpge(low).all() && centre.cmple(high).all()
      })
      .collect()
  }

  pub fn stations_near(&self, at: Vec3, reach: f32) -> Vec<Block> {
    let centre = at.floor().as_ivec3();
    let span = reach.ceil() as i32;
    let mut found: Vec<Block> = (-span..=span)
      .flat_map(|x| {
        (-span..=span).flat_map(move |y| (-span..=span).map(move |z| IVec3::new(x, y, z)))
      })
      .filter(|offset| offset.as_vec3().length() <= reach)
      .filter_map(|offset| self.block(centre + offset).filter(|block| block.station()))
      .collect();
    found.sort();
    found.dedup();
    found
  }

  pub fn snapshot(&mut self, column: IVec2) -> Vec<Block> {
    (0..LAYERS).for_each(|layer| {
      self.ensure(origin_of(column.extend(layer).xzy()));
    });
    (0..COLUMN as i32)
      .map(|index| {
        let local = IVec3::new(index % SIZE, index / (SIZE * SIZE), index / SIZE % SIZE);
        let key = IVec3::new(column.x, local.y / SIZE, column.y);
        self.chunks.get(&key).map_or(Block::Air, |chunk| {
          chunk.get(IVec3::new(local.x, local.y % SIZE, local.z))
        })
      })
      .collect()
  }

  pub fn all_kept(&self) -> Vec<(IVec2, Vec<u8>)> {
    self.kept.iter().map(|(&column, blocks)| (column, pack(blocks))).collect()
  }

  pub fn all_edits(&self) -> Vec<(IVec3, Block)> {
    self
      .edits
      .iter()
      .flat_map(|(&key, blocks)| {
        blocks.iter().map(move |(&local, &block)| (origin_of(key) + local, block))
      })
      .collect()
  }

  pub fn cast(&self, from: Vec3, toward: Vec3, reach: f32) -> Option<Hit> {
    self.cast_for(from, toward, reach, Block::targetable)
  }

  pub fn cast_for(
    &self,
    from: Vec3,
    toward: Vec3,
    reach: f32,
    wanted: impl Fn(Block) -> bool
  ) -> Option<Hit> {
    let toward = toward.normalize_or(Vec3::NEG_Z);
    let step = toward.signum().as_ivec3();
    let next_edge = |position: f32, direction: f32| match direction > 0.0 {
      true => position.floor() + 1.0 - position,
      false => position - position.floor()
    };
    let crossing = Vec3::from_array(std::array::from_fn(|axis| {
      (toward[axis] != 0.0)
        .then(|| next_edge(from[axis], toward[axis]) / toward[axis].abs())
        .unwrap_or(f32::INFINITY)
    }));
    let span = Vec3::from_array(std::array::from_fn(|axis| {
      (toward[axis] != 0.0).then(|| 1.0 / toward[axis].abs()).unwrap_or(f32::INFINITY)
    }));
    let start = from.floor().as_ivec3();
    std::iter::successors(
      Some((start, crossing, IVec3::ZERO, 0.0)),
      |&(at, crossing, _, _)| {
        let axis = crossing.min_position();
        let mut next = at;
        next[axis] += step[axis];
        let mut normal = IVec3::ZERO;
        normal[axis] = -step[axis];
        let mut further = crossing;
        further[axis] += span[axis];
        Some((next, further, normal, crossing[axis]))
      }
    )
    .take_while(|&(at, _, _, travelled)| travelled <= reach && self.block(at).is_some())
    .find_map(|(at, _, normal, _)| {
      self
        .block(at)
        .filter(|&block| wanted(block))
        .filter(|&block| {
          model::bounds(block, self.seed, at)
            .is_none_or(|(low, high)| pierces(from, toward, low, high))
        })
        .map(|block| Hit { at, normal, block })
    })
  }
}
