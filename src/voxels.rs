use {crate::{block::Block, generate},
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
  pub dirty: HashSet<IVec3>
}

impl Voxels {
  pub fn new(seed: u32) -> Self {
    Self { seed, chunks: default(), edits: default(), dirty: default() }
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
      self.insert(key, generate::chunk(self.seed, key))
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
      self.edits.entry(key).or_default().insert(local, block);
      if let Some(chunk) = self.chunks.get_mut(&key) {
        Arc::make_mut(chunk).set(local, block);
        let near_edge = |axis: i32| axis == 0 || axis == SIZE - 1;
        match local.to_array().into_iter().any(near_edge) {
          true => self.touch(key),
          false => {
            self.dirty.insert(key);
          }
        }
      }
    }
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
      self.block(at).filter(|block| block.targetable()).map(|block| Hit {
        at,
        normal,
        block
      })
    })
  }
}
