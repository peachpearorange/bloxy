use crate::block::Block;

#[derive(Clone, Copy)]
pub struct Bit {
  pub low: [u8; 3],
  pub high: [u8; 3],
  pub color: [f32; 3]
}

const fn bit(low: [u8; 3], high: [u8; 3], color: [f32; 3]) -> Bit {
  Bit { low, high, color }
}

const STALK: [f32; 3] = [0.24, 0.46, 0.16];
const LEAF: [f32; 3] = [0.18, 0.38, 0.12];
const POPPY: [f32; 3] = [0.85, 0.13, 0.1];
const SOOT: [f32; 3] = [0.1, 0.08, 0.08];
const YELLOW: [f32; 3] = [0.98, 0.82, 0.18];
const PALE_YELLOW: [f32; 3] = [1.0, 0.93, 0.5];
const BLUE: [f32; 3] = [0.28, 0.42, 0.9];
const DEEP_BLUE: [f32; 3] = [0.14, 0.2, 0.55];
const WHITE: [f32; 3] = [0.95, 0.95, 0.92];
const BARK: [f32; 3] = [0.38, 0.27, 0.17];
const BIRCH_BARK: [f32; 3] = [0.88, 0.87, 0.83];
const BIRCH_MARK: [f32; 3] = [0.15, 0.14, 0.13];
const SPRUCE_BARK: [f32; 3] = [0.27, 0.18, 0.1];
const PALM_BARK: [f32; 3] = [0.6, 0.48, 0.3];
const OAK_GREEN: [f32; 3] = [0.2, 0.44, 0.15];
const BIRCH_GREEN: [f32; 3] = [0.42, 0.6, 0.22];
const SPRUCE_GREEN: [f32; 3] = [0.1, 0.27, 0.18];
const PALM_GREEN: [f32; 3] = [0.3, 0.6, 0.15];
const CAP_RED: [f32; 3] = [0.8, 0.12, 0.1];
const CAP_BROWN: [f32; 3] = [0.55, 0.4, 0.27];
const FLESH: [f32; 3] = [0.9, 0.87, 0.78];

const POPPY_BITS: &[Bit] = &[
  bit([7, 0, 7], [8, 7, 8], STALK),
  bit([8, 2, 7], [10, 3, 8], LEAF),
  bit([5, 4, 7], [7, 5, 8], LEAF),
  bit([6, 7, 7], [10, 9, 9], POPPY),
  bit([7, 7, 6], [9, 9, 7], POPPY),
  bit([7, 7, 9], [9, 9, 10], POPPY),
  bit([7, 9, 7], [9, 10, 9], SOOT)
];

const DANDELION_BITS: &[Bit] = &[
  bit([7, 0, 7], [8, 6, 8], STALK),
  bit([5, 0, 7], [7, 1, 8], LEAF),
  bit([8, 0, 6], [10, 1, 7], LEAF),
  bit([6, 6, 6], [10, 9, 10], YELLOW),
  bit([7, 9, 7], [9, 10, 9], PALE_YELLOW)
];

const CORNFLOWER_BITS: &[Bit] = &[
  bit([7, 0, 7], [8, 8, 8], STALK),
  bit([8, 3, 7], [10, 4, 8], LEAF),
  bit([6, 8, 7], [10, 9, 9], BLUE),
  bit([7, 8, 6], [9, 9, 7], BLUE),
  bit([7, 8, 9], [9, 9, 10], BLUE),
  bit([7, 9, 7], [9, 10, 9], DEEP_BLUE)
];

const DAISY_BITS: &[Bit] = &[
  bit([7, 0, 7], [8, 6, 8], STALK),
  bit([5, 2, 7], [7, 3, 8], LEAF),
  bit([5, 6, 7], [11, 7, 9], WHITE),
  bit([7, 6, 5], [9, 7, 7], WHITE),
  bit([7, 6, 9], [9, 7, 11], WHITE),
  bit([7, 7, 7], [9, 8, 9], YELLOW)
];

const OAK_SAPLING_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 5, 9], BARK),
  bit([5, 5, 5], [11, 9, 11], OAK_GREEN),
  bit([6, 9, 6], [10, 10, 10], OAK_GREEN),
  bit([4, 6, 6], [5, 8, 9], OAK_GREEN),
  bit([11, 6, 7], [12, 8, 10], OAK_GREEN)
];

const BIRCH_SAPLING_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 2, 9], BIRCH_BARK),
  bit([7, 2, 7], [9, 3, 9], BIRCH_MARK),
  bit([7, 3, 7], [9, 6, 9], BIRCH_BARK),
  bit([6, 6, 6], [10, 11, 10], BIRCH_GREEN),
  bit([5, 7, 7], [6, 10, 9], BIRCH_GREEN),
  bit([10, 7, 7], [11, 10, 9], BIRCH_GREEN)
];

const SPRUCE_SAPLING_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 3, 9], SPRUCE_BARK),
  bit([4, 3, 4], [12, 5, 12], SPRUCE_GREEN),
  bit([5, 5, 5], [11, 7, 11], SPRUCE_GREEN),
  bit([6, 7, 6], [10, 9, 10], SPRUCE_GREEN),
  bit([7, 9, 7], [9, 12, 9], SPRUCE_GREEN)
];

const PALM_SAPLING_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 3, 9], PALM_BARK),
  bit([8, 3, 7], [10, 6, 9], PALM_BARK),
  bit([9, 6, 7], [11, 8, 9], PALM_BARK),
  bit([5, 8, 7], [15, 9, 9], PALM_GREEN),
  bit([9, 8, 3], [11, 9, 13], PALM_GREEN),
  bit([4, 7, 7], [5, 8, 9], PALM_GREEN),
  bit([9, 7, 2], [11, 8, 3], PALM_GREEN),
  bit([9, 7, 13], [11, 8, 14], PALM_GREEN)
];

const RED_MUSHROOM_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 3, 9], FLESH),
  bit([5, 3, 5], [11, 5, 11], CAP_RED),
  bit([6, 5, 6], [10, 6, 10], CAP_RED),
  bit([6, 6, 7], [7, 7, 8], WHITE),
  bit([8, 6, 8], [9, 7, 9], WHITE),
  bit([4, 4, 9], [5, 5, 10], WHITE)
];

const BROWN_MUSHROOM_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 2, 9], FLESH),
  bit([4, 2, 4], [12, 3, 12], CAP_BROWN),
  bit([5, 3, 5], [11, 4, 11], CAP_BROWN)
];

pub fn bits(block: Block) -> &'static [Bit] {
  match block {
    Block::Poppy => POPPY_BITS,
    Block::Dandelion => DANDELION_BITS,
    Block::Cornflower => CORNFLOWER_BITS,
    Block::Daisy => DAISY_BITS,
    Block::OakSapling => OAK_SAPLING_BITS,
    Block::BirchSapling => BIRCH_SAPLING_BITS,
    Block::SpruceSapling => SPRUCE_SAPLING_BITS,
    Block::PalmSapling => PALM_SAPLING_BITS,
    Block::RedMushroom => RED_MUSHROOM_BITS,
    Block::BrownMushroom => BROWN_MUSHROOM_BITS,
    _ => &[]
  }
}

pub fn icon(block: Block, x: u32, y: u32) -> Option<[f32; 3]> {
  let (x, y) = (x as u8, 15 - y as u8);
  bits(block)
    .iter()
    .filter(|bit| (bit.low[0]..bit.high[0].max(bit.low[0] + 1)).contains(&x))
    .filter(|bit| (bit.low[1]..bit.high[1].max(bit.low[1] + 1)).contains(&y))
    .min_by_key(|bit| bit.low[2])
    .map(|bit| bit.color)
}
