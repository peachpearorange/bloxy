use {crate::{block::{Block, Tile},
             noise::hash},
     bevy::prelude::*};

#[derive(Clone, Copy)]
pub struct Bit {
  pub low: [u8; 3],
  pub high: [u8; 3],
  pub color: [f32; 3],
  pub tile: Tile
}

const fn bit(low: [u8; 3], high: [u8; 3], color: [f32; 3]) -> Bit {
  Bit { low, high, color, tile: Tile::Grain }
}

const fn skin(low: [u8; 3], high: [u8; 3], tile: Tile) -> Bit {
  Bit { low, high, color: [1.0; 3], tile }
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
const OAK_GREEN: [f32; 3] = [0.2, 0.44, 0.15];
const BIRCH_GREEN: [f32; 3] = [0.42, 0.6, 0.22];
const SPRUCE_GREEN: [f32; 3] = [0.1, 0.27, 0.18];
const PALM_GREEN: [f32; 3] = [0.3, 0.6, 0.15];

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
  skin([7, 0, 7], [9, 5, 9], Tile::LogSide),
  bit([5, 5, 5], [11, 9, 11], OAK_GREEN),
  bit([6, 9, 6], [10, 10, 10], OAK_GREEN),
  bit([4, 6, 6], [5, 8, 9], OAK_GREEN),
  bit([11, 6, 7], [12, 8, 10], OAK_GREEN)
];

const BIRCH_SAPLING_BITS: &[Bit] = &[
  skin([7, 0, 7], [9, 6, 9], Tile::BirchSide),
  bit([6, 6, 6], [10, 11, 10], BIRCH_GREEN),
  bit([5, 7, 7], [6, 10, 9], BIRCH_GREEN),
  bit([10, 7, 7], [11, 10, 9], BIRCH_GREEN)
];

const SPRUCE_SAPLING_BITS: &[Bit] = &[
  skin([7, 0, 7], [9, 3, 9], Tile::SpruceSide),
  bit([4, 3, 4], [12, 5, 12], SPRUCE_GREEN),
  bit([5, 5, 5], [11, 7, 11], SPRUCE_GREEN),
  bit([6, 7, 6], [10, 9, 10], SPRUCE_GREEN),
  bit([7, 9, 7], [9, 12, 9], SPRUCE_GREEN)
];

const PALM_SAPLING_BITS: &[Bit] = &[
  skin([7, 0, 7], [9, 3, 9], Tile::PalmSide),
  skin([8, 3, 7], [10, 6, 9], Tile::PalmSide),
  skin([9, 6, 7], [11, 8, 9], Tile::PalmSide),
  bit([5, 8, 7], [15, 9, 9], PALM_GREEN),
  bit([9, 8, 3], [11, 9, 13], PALM_GREEN),
  bit([4, 7, 7], [5, 8, 9], PALM_GREEN),
  bit([9, 7, 2], [11, 8, 3], PALM_GREEN),
  bit([9, 7, 13], [11, 8, 14], PALM_GREEN)
];

const RED_MUSHROOM_BITS: &[Bit] = &[
  skin([6, 0, 6], [10, 5, 10], Tile::Stem),
  skin([3, 5, 3], [13, 8, 13], Tile::RedCap),
  skin([4, 8, 4], [12, 10, 12], Tile::RedCap)
];

const BROWN_MUSHROOM_BITS: &[Bit] = &[
  skin([6, 0, 6], [10, 4, 10], Tile::Stem),
  skin([2, 4, 2], [14, 5, 14], Tile::BrownCap),
  skin([3, 5, 3], [13, 6, 13], Tile::BrownCap)
];

const FLAME: [f32; 3] = [1.0, 0.85, 0.4];

const TORCH_BITS: &[Bit] = &[
  skin([7, 0, 7], [9, 9, 9], Tile::LogSide),
  skin([7, 9, 7], [9, 11, 9], Tile::Flame),
  bit([7, 11, 7], [9, 12, 9], FLAME)
];

const WAYSTONE_BITS: &[Bit] = &[
  skin([1, 0, 1], [15, 3, 15], Tile::WaystoneStone),
  skin([2, 3, 2], [14, 5, 14], Tile::WaystoneStone),
  skin([3, 5, 3], [13, 6, 13], Tile::WaystoneStone),
  skin([4, 6, 4], [12, 16, 12], Tile::WaystoneSide)
];

const WAYSTONE_TOP_BITS: &[Bit] = &[
  skin([4, 0, 4], [12, 7, 12], Tile::WaystoneSide),
  skin([3, 7, 3], [13, 9, 13], Tile::WaystoneStone),
  skin([2, 9, 2], [14, 10, 14], Tile::WaystoneStone),
  skin([3, 10, 3], [13, 12, 13], Tile::WaystoneStone),
  skin([5, 12, 5], [11, 14, 11], Tile::WaystoneTop)
];

const BLADE: [f32; 3] = [0.34, 0.6, 0.2];
const BLADE_DARK: [f32; 3] = [0.22, 0.45, 0.14];
const BLADE_DRY: [f32; 3] = [0.5, 0.64, 0.26];

const TALL_GRASS_BITS: &[Bit] = &[
  bit([2, 0, 3], [3, 9, 4], BLADE),
  bit([3, 6, 3], [4, 11, 4], BLADE),
  bit([5, 0, 9], [6, 12, 10], BLADE_DARK),
  bit([4, 9, 9], [5, 14, 10], BLADE_DARK),
  bit([7, 0, 5], [8, 7, 6], BLADE_DRY),
  bit([8, 0, 12], [9, 10, 13], BLADE),
  bit([9, 7, 12], [10, 13, 13], BLADE),
  bit([10, 0, 6], [11, 13, 7], BLADE_DARK),
  bit([11, 10, 6], [12, 15, 7], BLADE_DARK),
  bit([12, 0, 2], [13, 8, 3], BLADE_DRY),
  bit([13, 0, 10], [14, 11, 11], BLADE),
  bit([12, 8, 10], [13, 12, 11], BLADE),
  bit([4, 0, 13], [5, 6, 14], BLADE_DRY),
  bit([6, 0, 1], [7, 10, 2], BLADE),
  bit([14, 0, 4], [15, 6, 5], BLADE_DARK)
];

const LADDER_BITS: [Bit; 6] = [
  skin([2, 0, 0], [4, 16, 2], Tile::Ladder),
  skin([12, 0, 0], [14, 16, 2], Tile::Ladder),
  skin([4, 2, 0], [12, 3, 1], Tile::Ladder),
  skin([4, 6, 0], [12, 7, 1], Tile::Ladder),
  skin([4, 10, 0], [12, 11, 1], Tile::Ladder),
  skin([4, 14, 0], [12, 15, 1], Tile::Ladder)
];

const fn turned<const N: usize>(bits: [Bit; N], quarters: u8) -> [Bit; N] {
  let mut turned = bits;
  let mut index = 0;
  while index < N {
    let Bit { low: [lx, ly, lz], high: [hx, hy, hz], color, tile } = bits[index];
    let (low, high) = match quarters {
      1 => ([16 - hz, ly, lx], [16 - lz, hy, hx]),
      2 => ([16 - hx, ly, 16 - hz], [16 - lx, hy, 16 - lz]),
      3 => ([lz, ly, 16 - hx], [hz, hy, 16 - lx]),
      _ => ([lx, ly, lz], [hx, hy, hz])
    };
    turned[index] = Bit { low, high, color, tile };
    index += 1
  }
  turned
}

const LADDER_EAST_BITS: [Bit; 6] = turned(LADDER_BITS, 1);
const LADDER_SOUTH_BITS: [Bit; 6] = turned(LADDER_BITS, 2);
const LADDER_WEST_BITS: [Bit; 6] = turned(LADDER_BITS, 3);

const BLANKET: [f32; 3] = [0.75, 0.16, 0.14];

const FOOT_BITS: [Bit; 5] = [
  skin([0, 0, 13], [2, 3, 15], Tile::Planks),
  skin([14, 0, 13], [16, 3, 15], Tile::Planks),
  skin([0, 3, 0], [16, 5, 16], Tile::Planks),
  skin([1, 5, 0], [15, 8, 15], Tile::Wool),
  Bit { low: [0, 7, 0], high: [16, 9, 16], color: BLANKET, tile: Tile::Wool }
];

const HEAD_BITS: [Bit; 7] = [
  skin([0, 0, 1], [2, 3, 3], Tile::Planks),
  skin([14, 0, 1], [16, 3, 3], Tile::Planks),
  skin([0, 3, 0], [16, 5, 16], Tile::Planks),
  skin([0, 3, 0], [16, 13, 1], Tile::Planks),
  skin([1, 5, 1], [15, 8, 16], Tile::Wool),
  skin([2, 8, 2], [14, 10, 7], Tile::Wool),
  Bit { low: [0, 7, 8], high: [16, 9, 16], color: BLANKET, tile: Tile::Wool }
];

const FOOT_EAST_BITS: [Bit; 5] = turned(FOOT_BITS, 1);
const FOOT_SOUTH_BITS: [Bit; 5] = turned(FOOT_BITS, 2);
const FOOT_WEST_BITS: [Bit; 5] = turned(FOOT_BITS, 3);
const HEAD_EAST_BITS: [Bit; 7] = turned(HEAD_BITS, 1);
const HEAD_SOUTH_BITS: [Bit; 7] = turned(HEAD_BITS, 2);
const HEAD_WEST_BITS: [Bit; 7] = turned(HEAD_BITS, 3);

pub fn bits(block: Block) -> &'static [Bit] {
  match block {
    Block::Torch => TORCH_BITS,
    Block::Waystone => WAYSTONE_BITS,
    Block::WaystoneTop => WAYSTONE_TOP_BITS,
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
    Block::TallGrass => TALL_GRASS_BITS,
    Block::Ladder => &LADDER_BITS,
    Block::LadderEast => &LADDER_EAST_BITS,
    Block::LadderSouth => &LADDER_SOUTH_BITS,
    Block::LadderWest => &LADDER_WEST_BITS,
    Block::Bed => &FOOT_BITS,
    Block::BedEast => &FOOT_EAST_BITS,
    Block::BedSouth => &FOOT_SOUTH_BITS,
    Block::BedWest => &FOOT_WEST_BITS,
    Block::BedHead => &HEAD_BITS,
    Block::BedHeadEast => &HEAD_EAST_BITS,
    Block::BedHeadSouth => &HEAD_SOUTH_BITS,
    Block::BedHeadWest => &HEAD_WEST_BITS,
    _ => &[]
  }
}

pub fn shift(block: Block, seed: u32, at: IVec3) -> Vec3 {
  let jitter = |salt: u32| (hash(seed ^ salt, at.x, at.y, at.z) % 5) as f32 - 2.0;
  match block.plant() {
    true => Vec3::new(jitter(0x51), 0.0, jitter(0x52)),
    false => Vec3::ZERO
  }
}

const SIGN_BOUNDS: (Vec3, Vec3) = (Vec3::new(1.0, 0.0, 1.0), Vec3::new(15.0, 16.0, 15.0));

pub fn bounds(block: Block, seed: u32, at: IVec3) -> Option<(Vec3, Vec3)> {
  let shift = shift(block, seed, at);
  bits(block)
    .iter()
    .map(|bit| (Vec3::from(bit.low.map(f32::from)), Vec3::from(bit.high.map(f32::from))))
    .reduce(|(low, high), (bit_low, bit_high)| (low.min(bit_low), high.max(bit_high)))
    .or(block.sign().then_some(SIGN_BOUNDS))
    .map(|(low, high)| {
      let corner = at.as_vec3();
      (corner + (low + shift) / 16.0, corner + (high + shift) / 16.0)
    })
}

pub fn icon(block: Block, x: u32, y: u32) -> Option<Bit> {
  let (x, y) = (x as u8, 15 - y as u8);
  bits(block)
    .iter()
    .filter(|bit| (bit.low[0]..bit.high[0].max(bit.low[0] + 1)).contains(&x))
    .filter(|bit| (bit.low[1]..bit.high[1].max(bit.low[1] + 1)).contains(&y))
    .min_by_key(|bit| bit.low[2])
    .copied()
}
