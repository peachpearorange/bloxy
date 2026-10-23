use {crate::{block::{Block, Tile},
             noise::hash},
     bevy::{math::Affine3A, prelude::*}};

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

const TORCH_BITS: &[Bit] =
  &[skin([7, 0, 7], [9, 9, 9], Tile::LogSide), skin([7, 9, 7], [9, 12, 9], Tile::Flame)];

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

const IRON: [f32; 3] = [0.2, 0.2, 0.22];
const EMBER: [f32; 3] = [0.45, 0.1, 0.03];
const GILT: [f32; 3] = [0.85, 0.66, 0.22];
const HOLLOW: [f32; 3] = [0.12, 0.08, 0.05];

const BRAZIER_BITS: &[Bit] = &[
  bit([3, 0, 3], [5, 6, 5], IRON),
  bit([11, 0, 3], [13, 6, 5], IRON),
  bit([3, 0, 11], [5, 6, 13], IRON),
  bit([11, 0, 11], [13, 6, 13], IRON),
  bit([2, 6, 2], [14, 8, 14], IRON),
  bit([2, 8, 2], [14, 11, 3], IRON),
  bit([2, 8, 13], [14, 11, 14], IRON),
  bit([2, 8, 3], [3, 11, 13], IRON),
  bit([13, 8, 3], [14, 11, 13], IRON),
  bit([3, 8, 3], [13, 10, 13], EMBER),
  skin([5, 10, 5], [11, 14, 11], Tile::Flame),
  skin([6, 14, 7], [10, 16, 9], Tile::Flame)
];

const URN_BITS: &[Bit] = &[
  skin([5, 0, 5], [11, 2, 11], Tile::Terracotta),
  skin([3, 2, 3], [13, 9, 13], Tile::Terracotta),
  skin([4, 9, 4], [12, 10, 12], Tile::Terracotta),
  skin([5, 10, 5], [11, 12, 11], Tile::Terracotta),
  skin([4, 12, 4], [12, 13, 12], Tile::Terracotta)
];

const CHEST_BITS: &[Bit] = &[
  skin([1, 0, 1], [15, 13, 15], Tile::Planks),
  bit([0, 0, 3], [16, 14, 5], IRON),
  bit([0, 0, 11], [16, 14, 13], IRON),
  bit([7, 7, 0], [9, 10, 1], GILT)
];

const EMPTY_CHEST_BITS: &[Bit] = &[
  skin([1, 0, 1], [15, 6, 15], Tile::Planks),
  bit([2, 6, 2], [14, 7, 14], HOLLOW),
  skin([1, 6, 1], [15, 9, 2], Tile::Planks),
  skin([1, 6, 2], [2, 9, 14], Tile::Planks),
  skin([14, 6, 2], [15, 9, 14], Tile::Planks),
  skin([1, 6, 14], [15, 16, 16], Tile::Planks),
  bit([0, 0, 3], [1, 9, 5], IRON),
  bit([15, 0, 3], [16, 9, 5], IRON),
  bit([0, 0, 11], [1, 9, 13], IRON),
  bit([15, 0, 11], [16, 9, 13], IRON)
];

const TABLE_BITS: &[Bit] = &[
  skin([0, 12, 0], [16, 16, 16], Tile::TableTop),
  skin([2, 8, 2], [14, 12, 14], Tile::TableSide),
  skin([1, 0, 1], [4, 12, 4], Tile::Planks),
  skin([12, 0, 1], [15, 12, 4], Tile::Planks),
  skin([1, 0, 12], [4, 12, 15], Tile::Planks),
  skin([12, 0, 12], [15, 12, 15], Tile::Planks),
  skin([2, 3, 2], [14, 4, 14], Tile::Planks)
];

const FURNACE_BITS: &[Bit] = &[
  skin([0, 0, 0], [16, 1, 16], Tile::FurnaceTop),
  skin([1, 1, 1], [15, 13, 15], Tile::FurnaceSide),
  skin([0, 13, 0], [16, 14, 16], Tile::FurnaceTop),
  skin([5, 14, 5], [11, 16, 11], Tile::FurnaceTop)
];

const fn tinted(low: [u8; 3], high: [u8; 3], tile: Tile, color: [f32; 3]) -> Bit {
  Bit { low, high, color, tile }
}

const GOLDEN: [f32; 3] = [1.45, 1.1, 0.42];
const DIAMOND: [f32; 3] = [0.62, 1.35, 1.4];
const RED: [f32; 3] = [1.5, 0.45, 0.4];
const BIO: [f32; 3] = [0.62, 1.15, 0.5];
const AMBER: [f32; 3] = [1.35, 1.0, 0.42];
const SOOTY: [f32; 3] = [0.55, 0.55, 0.6];
const BRASS: [f32; 3] = [1.3, 1.05, 0.55];

const TANK_BITS: &[Bit] = &[
  skin([0, 0, 0], [16, 2, 16], Tile::Copper),
  skin([0, 14, 0], [16, 16, 16], Tile::Copper),
  skin([0, 2, 0], [2, 14, 2], Tile::Copper),
  skin([14, 2, 0], [16, 14, 2], Tile::Copper),
  skin([0, 2, 14], [2, 14, 16], Tile::Copper),
  skin([14, 2, 14], [16, 14, 16], Tile::Copper),
  skin([2, 2, 0], [14, 14, 1], Tile::PipeGlass),
  skin([2, 2, 15], [14, 14, 16], Tile::PipeGlass),
  skin([0, 2, 2], [1, 14, 14], Tile::PipeGlass),
  skin([15, 2, 2], [16, 14, 14], Tile::PipeGlass)
];

const PUMPJACK_BITS: &[Bit] = &[
  skin([1, 0, 1], [15, 2, 15], Tile::Steel),
  skin([1, 2, 4], [6, 7, 12], Tile::Casing),
  skin([7, 2, 3], [9, 13, 5], Tile::Hazard),
  skin([7, 2, 11], [9, 13, 13], Tile::Hazard),
  skin([7, 12, 5], [9, 14, 11], Tile::Steel),
  skin([12, 2, 7], [14, 6, 9], Tile::Copper),
  skin([11, 6, 6], [15, 7, 10], Tile::Steel)
];

const REFINERY_BITS: &[Bit] = &[
  skin([1, 0, 1], [11, 9, 15], Tile::Casing),
  skin([11, 0, 9], [15, 16, 13], Tile::Steel),
  skin([10, 15, 8], [16, 16, 14], Tile::Steel),
  skin([11, 0, 3], [14, 12, 6], Tile::Copper),
  skin([8, 9, 10], [11, 11, 12], Tile::Copper),
  skin([3, 9, 3], [8, 11, 8], Tile::Grille)
];

const FERMENTER_BITS: &[Bit] = &[
  skin([2, 0, 2], [14, 12, 14], Tile::Planks),
  skin([1, 2, 1], [15, 3, 15], Tile::Steel),
  skin([1, 9, 1], [15, 10, 15], Tile::Steel),
  skin([3, 12, 3], [13, 13, 13], Tile::Planks),
  skin([7, 13, 7], [9, 16, 9], Tile::Copper),
  skin([6, 15, 6], [10, 16, 10], Tile::PipeGlass)
];

const STEAM_ENGINE_BITS: &[Bit] = &[
  skin([1, 0, 1], [15, 6, 15], Tile::Bricks),
  skin([5, 1, 0], [11, 5, 1], Tile::Coil),
  skin([2, 6, 3], [14, 12, 13], Tile::Copper),
  skin([4, 6, 2], [5, 12, 14], Tile::Steel),
  skin([11, 6, 2], [12, 12, 14], Tile::Steel),
  skin([11, 12, 10], [14, 16, 13], Tile::Steel),
  skin([4, 12, 7], [6, 14, 9], Tile::Copper)
];

const BIOFUEL_BITS: &[Bit] = &[
  skin([1, 0, 2], [15, 10, 14], Tile::Casing),
  skin([3, 2, 1], [13, 8, 2], Tile::Grille),
  tinted([2, 10, 3], [9, 14, 13], Tile::Steel, BIO),
  skin([11, 10, 10], [13, 16, 12], Tile::Steel),
  skin([4, 14, 7], [7, 15, 9], Tile::Copper)
];

const DIESEL_BITS: &[Bit] = &[
  tinted([0, 0, 2], [16, 9, 14], Tile::Casing, AMBER),
  skin([1, 9, 3], [7, 14, 13], Tile::Grille),
  skin([10, 9, 9], [12, 16, 11], Tile::Steel),
  tinted([10, 15, 8], [12, 16, 12], Tile::Steel, SOOTY),
  skin([12, 9, 4], [14, 10, 6], Tile::Copper)
];

const GEOTHERMAL_BITS: &[Bit] = &[
  tinted([0, 0, 0], [16, 3, 16], Tile::Steel, SOOTY),
  skin([2, 3, 2], [14, 11, 14], Tile::Grille),
  skin([3, 5, 1], [13, 9, 2], Tile::Lava),
  skin([3, 5, 14], [13, 9, 15], Tile::Lava),
  skin([3, 11, 3], [13, 13, 13], Tile::Casing),
  skin([6, 13, 6], [10, 16, 10], Tile::Steel)
];

const SOLAR_BITS: &[Bit] = &[
  skin([7, 0, 7], [9, 6, 9], Tile::Steel),
  skin([0, 6, 0], [16, 7, 16], Tile::Steel),
  skin([1, 7, 1], [15, 8, 15], Tile::Solar)
];

const TURBINE_BITS: &[Bit] = &[
  skin([4, 0, 4], [12, 2, 12], Tile::Steel),
  skin([6, 2, 6], [10, 16, 10], Tile::Steel)
];

const BATTERY_BITS: &[Bit] = &[
  skin([2, 0, 2], [14, 13, 14], Tile::Cells),
  skin([2, 13, 2], [14, 14, 14], Tile::Steel),
  skin([4, 14, 6], [6, 16, 8], Tile::Copper),
  tinted([10, 14, 6], [12, 16, 8], Tile::Steel, RED)
];

const PUMP_BITS: &[Bit] = &[
  skin([2, 0, 2], [14, 3, 14], Tile::Steel),
  skin([3, 3, 3], [13, 10, 13], Tile::Casing),
  skin([5, 10, 4], [11, 14, 12], Tile::Copper),
  skin([7, 14, 7], [9, 15, 9], Tile::Steel)
];

const ELECTRIC_FURNACE_BITS: &[Bit] = &[
  skin([1, 0, 1], [15, 15, 15], Tile::Casing),
  skin([3, 3, 0], [13, 11, 1], Tile::Coil),
  skin([4, 15, 4], [12, 16, 12], Tile::Grille)
];

const LAMP_BITS: &[Bit] = &[
  skin([5, 0, 5], [11, 2, 11], Tile::Steel),
  skin([7, 2, 7], [9, 6, 9], Tile::Steel),
  skin([5, 6, 5], [11, 13, 11], Tile::Bulb),
  skin([6, 13, 6], [10, 14, 10], Tile::Steel)
];

const LIT_LAMP_BITS: &[Bit] = &[
  skin([5, 0, 5], [11, 2, 11], Tile::Steel),
  skin([7, 2, 7], [9, 6, 9], Tile::Steel),
  skin([5, 6, 5], [11, 13, 11], Tile::BulbLit),
  skin([6, 13, 6], [10, 14, 10], Tile::Steel)
];

const fn glimmer(petal: [f32; 3], heart: [f32; 3]) -> [Bit; 8] {
  [
    bit([7, 0, 7], [8, 9, 8], STALK),
    bit([8, 3, 7], [10, 4, 8], LEAF),
    bit([5, 5, 7], [7, 6, 8], LEAF),
    bit([6, 9, 6], [10, 10, 10], petal),
    bit([5, 10, 7], [11, 12, 9], petal),
    bit([7, 10, 5], [9, 12, 11], petal),
    bit([6, 12, 6], [10, 13, 10], petal),
    bit([7, 13, 7], [9, 14, 9], heart)
  ]
}

const GLIMMER_PINK_BITS: [Bit; 8] = glimmer([0.98, 0.42, 0.72], [1.0, 0.82, 0.9]);
const GLIMMER_CYAN_BITS: [Bit; 8] = glimmer([0.25, 0.85, 0.92], [0.82, 1.0, 1.0]);
const GLIMMER_VIOLET_BITS: [Bit; 8] = glimmer([0.6, 0.32, 0.95], [0.9, 0.8, 1.0]);
const GLIMMER_GOLD_BITS: [Bit; 8] = glimmer([1.0, 0.78, 0.2], [1.0, 0.96, 0.7]);

const SEEDHEAD: [f32; 3] = [0.5, 0.3, 0.1];
const HYDRANGEA: [f32; 3] = [0.35, 0.55, 1.0];
const HYDRANGEA_PALE: [f32; 3] = [0.6, 0.8, 1.0];
const ORANGE: [f32; 3] = [1.0, 0.45, 0.08];

const DAYBLOOM_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 9, 9], STALK),
  bit([9, 3, 7], [12, 4, 9], LEAF),
  bit([4, 5, 7], [7, 6, 9], LEAF),
  bit([4, 9, 7], [12, 15, 8], YELLOW),
  bit([5, 8, 7], [11, 16, 8], YELLOW),
  bit([3, 10, 7], [13, 14, 8], YELLOW),
  bit([6, 10, 6], [10, 14, 7], SEEDHEAD)
];

const HYDROANGEA_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 6, 9], STALK),
  bit([4, 2, 7], [7, 3, 9], LEAF),
  bit([9, 3, 7], [12, 4, 9], LEAF),
  bit([4, 6, 4], [9, 10, 9], HYDRANGEA),
  bit([8, 7, 7], [12, 11, 12], HYDRANGEA_PALE),
  bit([6, 9, 8], [10, 13, 11], HYDRANGEA),
  bit([5, 10, 5], [8, 12, 8], HYDRANGEA_PALE)
];

const ENDOFLAME_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 6, 9], STALK),
  bit([4, 2, 7], [7, 3, 9], LEAF),
  bit([9, 3, 7], [12, 4, 9], LEAF),
  bit([5, 6, 5], [11, 9, 11], RED_PETAL),
  bit([6, 9, 6], [10, 12, 10], ORANGE),
  skin([7, 12, 7], [9, 15, 9], Tile::Flame)
];

const RED_PETAL: [f32; 3] = [0.8, 0.12, 0.06];

const POOL_BITS: &[Bit] = &[
  skin([0, 0, 0], [16, 2, 16], Tile::Moonstone),
  skin([0, 2, 0], [16, 8, 2], Tile::Moonstone),
  skin([0, 2, 14], [16, 8, 16], Tile::Moonstone),
  skin([0, 2, 2], [2, 8, 14], Tile::Moonstone),
  skin([14, 2, 2], [16, 8, 14], Tile::Moonstone)
];

const CAULDRON: [f32; 3] = [0.5, 0.5, 0.55];

const CAULDRON_BITS: &[Bit] = &[
  bit([1, 0, 1], [4, 3, 4], IRON),
  bit([12, 0, 1], [15, 3, 4], IRON),
  bit([1, 0, 12], [4, 3, 15], IRON),
  bit([12, 0, 12], [15, 3, 15], IRON),
  tinted([1, 2, 1], [15, 3, 15], Tile::Steel, CAULDRON),
  tinted([1, 3, 1], [15, 13, 2], Tile::Steel, CAULDRON),
  tinted([1, 3, 14], [15, 13, 15], Tile::Steel, CAULDRON),
  tinted([1, 3, 2], [2, 13, 14], Tile::Steel, CAULDRON),
  tinted([14, 3, 2], [15, 13, 14], Tile::Steel, CAULDRON),
  bit([0, 13, 0], [16, 14, 2], IRON),
  bit([0, 13, 14], [16, 14, 16], IRON),
  bit([0, 13, 2], [2, 14, 14], IRON),
  bit([14, 13, 2], [16, 14, 14], IRON)
];

const CRIMSON_ALTAR_BITS: &[Bit] = &[
  skin([0, 0, 0], [16, 3, 16], Tile::AncientBrick),
  skin([2, 3, 2], [14, 10, 14], Tile::CrimsonRune),
  skin([0, 10, 0], [16, 12, 16], Tile::AncientBrick),
  skin([0, 12, 0], [16, 15, 1], Tile::AncientBrick),
  skin([0, 12, 15], [16, 15, 16], Tile::AncientBrick),
  skin([0, 12, 1], [1, 15, 15], Tile::AncientBrick),
  skin([15, 12, 1], [16, 15, 15], Tile::AncientBrick)
];

const PLINTH_BITS: &[Bit] = &[
  skin([3, 0, 3], [13, 2, 13], Tile::Moonstone),
  skin([5, 2, 5], [11, 11, 11], Tile::Moonstone),
  skin([4, 4, 4], [12, 5, 12], Tile::Runework),
  skin([3, 11, 3], [13, 13, 13], Tile::Moonstone)
];

const CONFLUENCE_ALTAR_BITS: &[Bit] = &[
  skin([0, 0, 0], [16, 3, 16], Tile::Moonstone),
  skin([2, 3, 2], [14, 10, 14], Tile::Runework),
  skin([1, 10, 1], [15, 13, 15], Tile::Moonstone),
  skin([1, 13, 1], [3, 16, 3], Tile::Lumen),
  skin([13, 13, 1], [15, 16, 3], Tile::Lumen),
  skin([1, 13, 13], [3, 16, 15], Tile::Lumen),
  skin([13, 13, 13], [15, 16, 15], Tile::Lumen)
];

const APOTHECARY_BITS: &[Bit] = &[
  skin([3, 0, 3], [13, 2, 13], Tile::Moonstone),
  skin([6, 2, 6], [10, 8, 10], Tile::Moonstone),
  skin([2, 8, 2], [14, 10, 14], Tile::Moonstone),
  skin([2, 10, 2], [14, 14, 3], Tile::Moonstone),
  skin([2, 10, 13], [14, 14, 14], Tile::Moonstone),
  skin([2, 10, 3], [3, 14, 13], Tile::Moonstone),
  skin([13, 10, 3], [14, 14, 13], Tile::Moonstone),
  skin([3, 10, 3], [13, 13, 13], Tile::Water)
];

const MOONPETAL: [f32; 3] = [0.9, 0.94, 1.0];
const MOONHEART: [f32; 3] = [0.6, 0.75, 1.0];

const MOONLACE_BITS: &[Bit] = &[
  bit([7, 0, 7], [8, 11, 8], STALK),
  bit([8, 4, 7], [11, 5, 8], LEAF),
  bit([4, 6, 7], [7, 7, 8], LEAF),
  bit([8, 11, 7], [11, 12, 8], STALK),
  bit([10, 8, 6], [12, 11, 9], MOONPETAL),
  bit([10, 7, 7], [12, 8, 8], MOONHEART),
  bit([4, 11, 7], [7, 12, 8], STALK),
  bit([3, 8, 6], [5, 11, 9], MOONPETAL),
  bit([3, 7, 7], [5, 8, 8], MOONHEART),
  bit([6, 12, 6], [9, 15, 9], MOONPETAL),
  bit([7, 11, 7], [8, 12, 8], MOONHEART)
];

const CRUSHER_BITS: &[Bit] = &[
  skin([0, 0, 0], [16, 3, 16], Tile::Steel),
  skin([1, 3, 1], [15, 10, 15], Tile::Casing),
  skin([3, 4, 0], [13, 9, 1], Tile::Crusher),
  skin([3, 4, 15], [13, 9, 16], Tile::Crusher),
  skin([2, 10, 2], [14, 12, 14], Tile::Steel),
  skin([3, 12, 3], [13, 15, 4], Tile::Steel),
  skin([3, 12, 12], [13, 15, 13], Tile::Steel),
  skin([3, 12, 4], [4, 15, 12], Tile::Steel),
  skin([12, 12, 4], [13, 15, 12], Tile::Steel),
  skin([15, 4, 5], [16, 8, 11], Tile::Coil)
];

pub const ACROSS: u16 = 0b11;

pub fn links(block: Block, near: impl Fn(IVec3) -> Block) -> u16 {
  let joined =
    crate::machine::SIDES.iter().enumerate().fold(0, |links, (index, &side)| {
      match crate::machine::joins(block, near(side)) {
        true => links | 1 << index,
        false => links
      }
    });
  crate::machine::attached(block, &near)
    .map_or(joined, |side| (joined & !(1 << side)) | 1 << (ATTACHED + side))
}

pub fn ends(links: u16) -> u16 {
  let links = links & JOINS;
  match links.count_ones() {
    0 => ACROSS,
    1 => links | 1 << (links.trailing_zeros() ^ 1),
    _ => links
  }
}

pub const ATTACHED: usize = 8;
const JOINS: u16 = 0b11_1111;

const DRAIN_ABOVE_BITS: &[Bit] = &[
  skin([3, 14, 3], [13, 16, 13], Tile::Copper),
  skin([5, 9, 5], [11, 14, 11], Tile::Steel),
  skin([7, 4, 7], [9, 9, 9], Tile::Copper),
  tinted([3, 11, 7], [13, 12, 9], Tile::Steel, RED)
];

const DRAIN_BELOW_BITS: &[Bit] = &[
  skin([3, 0, 3], [13, 2, 13], Tile::Copper),
  skin([6, 2, 6], [10, 8, 10], Tile::Steel),
  skin([10, 5, 6], [14, 8, 10], Tile::Copper),
  tinted([4, 6, 7], [6, 7, 9], Tile::Steel, RED)
];

const DRAIN_SIDE_BITS: [Bit; 4] = [
  skin([3, 3, 0], [13, 13, 2], Tile::Copper),
  skin([6, 6, 2], [10, 10, 10], Tile::Steel),
  skin([6, 3, 6], [10, 6, 10], Tile::Copper),
  tinted([5, 10, 4], [11, 11, 6], Tile::Steel, RED)
];

const DRAIN_EAST_BITS: [Bit; 4] = turned(DRAIN_SIDE_BITS, 1);
const DRAIN_SOUTH_BITS: [Bit; 4] = turned(DRAIN_SIDE_BITS, 2);
const DRAIN_WEST_BITS: [Bit; 4] = turned(DRAIN_SIDE_BITS, 3);

fn attachment(links: u16) -> Option<usize> {
  (0..6).find(|&side| links & (1 << (ATTACHED + side)) != 0)
}

fn drained(links: u16) -> &'static [Bit] {
  match attachment(links) {
    Some(0) => &DRAIN_EAST_BITS,
    Some(1) => &DRAIN_WEST_BITS,
    Some(2) => DRAIN_ABOVE_BITS,
    Some(3) => DRAIN_BELOW_BITS,
    Some(4) => &DRAIN_SOUTH_BITS,
    _ => &DRAIN_SIDE_BITS
  }
}

fn hoppered(bits: &[Bit]) -> Vec<Bit> {
  bits
    .iter()
    .filter(|bit| bit.color != RED)
    .map(|&bit| match bit.tile {
      Tile::Copper => Bit { tile: Tile::Steel, color: SOOTY, ..bit },
      _ => bit
    })
    .collect()
}

pub fn spout(links: u16) -> Vec3 {
  let lip = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) / 16.0;
  match attachment(links) {
    Some(0) => lip(8.0, 3.0, 8.0),
    Some(1) => lip(8.0, 3.0, 8.0),
    Some(2) => lip(8.0, 4.0, 8.0),
    Some(3) => lip(14.0, 6.5, 8.0),
    _ => lip(8.0, 3.0, 8.0)
  }
}

fn arm(index: usize, low: [u8; 3], high: [u8; 3]) -> ([u8; 3], [u8; 3]) {
  let axis = index / 2;
  let flip = |value: u8| 16 - value;
  match index % 2 {
    0 => (low, high),
    _ => {
      let (mut low, mut high) = (low, high);
      let (from, to) = (flip(high[axis]), flip(low[axis]));
      low[axis] = from;
      high[axis] = to;
      (low, high)
    }
  }
}

fn along(axis: usize, from: u8, to: u8, thin: u8, thick: u8) -> ([u8; 3], [u8; 3]) {
  let mut low = [thin; 3];
  let mut high = [thick; 3];
  low[axis] = from;
  high[axis] = to;
  (low, high)
}

fn ringed(block: Block) -> Bit {
  match block {
    Block::IronPipe => skin([0; 3], [0; 3], Tile::Steel),
    Block::GoldPipe => tinted([0; 3], [0; 3], Tile::Steel, GOLDEN),
    Block::DiamondPipe => tinted([0; 3], [0; 3], Tile::Steel, DIAMOND),
    Block::ItemPipe => tinted([0; 3], [0; 3], Tile::Steel, BRASS),
    _ => skin([0; 3], [0; 3], Tile::Copper)
  }
}

fn hoop(
  axis: usize,
  (from, to): (u8, u8),
  (thin, thick): (u8, u8)
) -> [([u8; 3], [u8; 3]); 4] {
  let (first, second) = ((axis + 1) % 3, (axis + 2) % 3);
  let bar = |(a_low, a_high): (u8, u8), (b_low, b_high): (u8, u8)| {
    let (mut low, mut high) = ([0; 3], [0; 3]);
    (low[axis], high[axis]) = (from, to);
    (low[first], high[first]) = (a_low, a_high);
    (low[second], high[second]) = (b_low, b_high);
    (low, high)
  };
  let (outer, inner) = ((thin - 1, thick + 1), (thin, thick));
  [
    bar((thin - 1, thin), outer),
    bar((thick, thick + 1), outer),
    bar(inner, (thin - 1, thin)),
    bar(inner, (thick, thick + 1))
  ]
}

fn piping(block: Block, links: u16) -> Vec<Bit> {
  let ring = ringed(block);
  let made = |(low, high): ([u8; 3], [u8; 3]), like: Bit| Bit { low, high, ..like };
  let glass = skin([0; 3], [0; 3], Tile::PipeGlass);
  let (thin, thick) = match block {
    Block::ItemPipe => (5, 11),
    _ => (6, 10)
  };
  std::iter::once(made(([thin; 3], [thick; 3]), glass))
    .chain((0..6).filter(|index| ends(links) & (1 << index) != 0).flat_map(|index| {
      let axis = index / 2;
      let flipped = |(low, high)| made(arm(index, low, high), ring);
      let (low, high) = along(axis, thick, 16, thin, thick);
      std::iter::once(made(arm(index, low, high), glass))
        .chain(hoop(axis, (14, 16), (thin, thick)).map(flipped))
        .chain(hoop(axis, (thick, thick + 1), (thin, thick)).map(flipped))
        .collect::<Vec<_>>()
    }))
    .collect()
}

fn wiring(links: u16) -> Vec<Bit> {
  let cable = skin([0; 3], [0; 3], Tile::Cable);
  let made = |(low, high): ([u8; 3], [u8; 3]), like: Bit| Bit { low, high, ..like };
  let ends = ends(links);
  let straight = ends == ACROSS || ends == 0b1100 || ends == 0b11_0000;
  let hub = match straight {
    true => made(([7; 3], [9; 3]), cable),
    false => made(([6; 3], [10; 3]), skin([0; 3], [0; 3], Tile::Steel))
  };
  std::iter::once(hub)
    .chain((0..6).filter(|index| ends & (1 << index) != 0).map(|index| {
      let (low, high) = along(index / 2, 9, 16, 7, 9);
      made(arm(index, low, high), cable)
    }))
    .collect()
}

pub fn shape(block: Block, links: u16) -> std::borrow::Cow<'static, [Bit]> {
  match block {
    Block::FluidDrain => std::borrow::Cow::Borrowed(drained(links)),
    Block::ItemDrain => std::borrow::Cow::Owned(hoppered(drained(links))),
    Block::PipedDrain => std::borrow::Cow::Owned(
      drained(links)
        .iter()
        .copied()
        .chain(
          piping(Block::CopperPipe, links & JOINS)
            .into_iter()
            .filter(|_| links & JOINS != 0)
        )
        .collect()
    ),
    pipe if pipe.pipe() || pipe == Block::ItemPipe => {
      std::borrow::Cow::Owned(piping(block, links))
    }
    Block::CopperCable => std::borrow::Cow::Owned(wiring(links)),
    _ => std::borrow::Cow::Borrowed(bits(block))
  }
}

const CHEVRON: [f32; 3] = [0.95, 0.75, 0.2];

const FLAT_BELT_BITS: [Bit; 9] = [
  skin([0, 0, 0], [16, 1, 16], Tile::Steel),
  skin([1, 1, 0], [15, 3, 16], Tile::Rubber),
  skin([0, 1, 0], [1, 4, 16], Tile::Steel),
  skin([15, 1, 0], [16, 4, 16], Tile::Steel),
  bit([7, 3, 5], [9, 4, 7], CHEVRON),
  bit([5, 3, 7], [7, 4, 9], CHEVRON),
  bit([9, 3, 7], [11, 4, 9], CHEVRON),
  bit([3, 3, 9], [5, 4, 11], CHEVRON),
  bit([11, 3, 9], [13, 4, 11], CHEVRON)
];

const fn ramp() -> [Bit; 16] {
  let mut bits = [skin([0; 3], [0; 3], Tile::Rubber); 16];
  let mut step = 0;
  while step < 8 {
    let (near, far) = (16 - 2 * (step + 1), 16 - 2 * step);
    bits[step as usize] = skin([1, 0, near], [15, 2 * step + 3, far], Tile::Rubber);
    step += 1
  }
  let mut rail = 0;
  while rail < 4 {
    let (near, far, top) = (16 - 4 * (rail + 1), 16 - 4 * rail, 4 * rail + 5);
    bits[8 + rail as usize] = skin([0, 0, near], [1, top, far], Tile::Steel);
    bits[12 + rail as usize] = skin([15, 0, near], [16, top, far], Tile::Steel);
    rail += 1
  }
  bits
}

const RAMP_BITS: [Bit; 16] = ramp();
const FLAT_EAST_BITS: [Bit; 9] = turned(FLAT_BELT_BITS, 1);
const FLAT_SOUTH_BITS: [Bit; 9] = turned(FLAT_BELT_BITS, 2);
const FLAT_WEST_BITS: [Bit; 9] = turned(FLAT_BELT_BITS, 3);
const RAMP_EAST_BITS: [Bit; 16] = turned(RAMP_BITS, 1);
const RAMP_SOUTH_BITS: [Bit; 16] = turned(RAMP_BITS, 2);
const RAMP_WEST_BITS: [Bit; 16] = turned(RAMP_BITS, 3);

const KELP_GREEN: [f32; 3] = [0.36, 0.46, 0.12];
const KELP_DARK: [f32; 3] = [0.24, 0.32, 0.08];
const SEAGRASS_GREEN: [f32; 3] = [0.22, 0.5, 0.2];
const FAN_RED: [f32; 3] = [0.86, 0.26, 0.32];
const FAN_PINK: [f32; 3] = [0.98, 0.52, 0.5];
const TUBE_VIOLET: [f32; 3] = [0.56, 0.32, 0.8];
const TUBE_DARK: [f32; 3] = [0.16, 0.06, 0.24];
const ANEMONE_FOOT: [f32; 3] = [0.7, 0.3, 0.14];
const ANEMONE_ARM: [f32; 3] = [1.0, 0.58, 0.3];
const ANEMONE_TIP: [f32; 3] = [1.0, 0.86, 0.6];
const WORM_WHITE: [f32; 3] = [0.9, 0.88, 0.82];
const WORM_RED: [f32; 3] = [0.9, 0.12, 0.1];
const SHELL_GREY: [f32; 3] = [0.6, 0.58, 0.68];
const MANTLE_BLUE: [f32; 3] = [0.25, 0.5, 0.9];
const STEM_CYAN: [f32; 3] = [0.2, 0.5, 0.55];

const KELP_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 16, 8], KELP_DARK),
  bit([4, 1, 7], [7, 5, 8], KELP_GREEN),
  bit([9, 4, 7], [12, 8, 8], KELP_GREEN),
  bit([3, 8, 7], [7, 12, 8], KELP_GREEN),
  bit([9, 11, 7], [13, 15, 8], KELP_GREEN),
  bit([7, 2, 5], [8, 6, 7], KELP_GREEN),
  bit([8, 9, 8], [9, 13, 11], KELP_GREEN)
];

const GLOWKELP_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 16, 8], STEM_CYAN),
  skin([4, 1, 7], [7, 5, 8], Tile::Biolume),
  skin([9, 4, 7], [12, 8, 8], Tile::Biolume),
  skin([3, 8, 7], [7, 12, 8], Tile::Biolume),
  skin([9, 11, 7], [13, 15, 8], Tile::Biolume),
  skin([6, 14, 6], [10, 16, 10], Tile::Biolume)
];

const SEAGRASS_BITS: &[Bit] = &[
  bit([3, 0, 4], [4, 10, 5], SEAGRASS_GREEN),
  bit([6, 0, 9], [7, 13, 10], SEAGRASS_GREEN),
  bit([9, 0, 5], [10, 8, 6], SEAGRASS_GREEN),
  bit([12, 0, 10], [13, 11, 11], SEAGRASS_GREEN),
  bit([4, 0, 12], [5, 7, 13], SEAGRASS_GREEN),
  bit([11, 0, 2], [12, 9, 3], SEAGRASS_GREEN)
];

const FAN_BITS: &[Bit] = &[
  bit([7, 0, 7], [9, 3, 9], FAN_RED),
  bit([5, 3, 7], [11, 6, 8], FAN_RED),
  bit([3, 6, 7], [13, 9, 8], FAN_PINK),
  bit([2, 9, 7], [14, 12, 8], FAN_RED),
  bit([3, 12, 7], [13, 14, 8], FAN_PINK),
  bit([5, 14, 7], [11, 15, 8], FAN_RED)
];

const TUBE_BITS: &[Bit] = &[
  bit([2, 0, 3], [5, 9, 6], TUBE_VIOLET),
  bit([3, 9, 4], [4, 10, 5], TUBE_DARK),
  bit([7, 0, 7], [10, 13, 10], TUBE_VIOLET),
  bit([8, 13, 8], [9, 14, 9], TUBE_DARK),
  bit([11, 0, 2], [14, 7, 5], TUBE_VIOLET),
  bit([12, 7, 3], [13, 8, 4], TUBE_DARK),
  bit([4, 0, 10], [7, 5, 13], TUBE_VIOLET)
];

const ANEMONE_BITS: &[Bit] = &[
  bit([5, 0, 5], [11, 4, 11], ANEMONE_FOOT),
  bit([4, 4, 7], [5, 9, 8], ANEMONE_ARM),
  bit([11, 4, 8], [12, 10, 9], ANEMONE_ARM),
  bit([7, 4, 4], [8, 8, 5], ANEMONE_ARM),
  bit([8, 4, 11], [9, 9, 12], ANEMONE_ARM),
  bit([6, 4, 6], [7, 11, 7], ANEMONE_ARM),
  bit([9, 4, 9], [10, 10, 10], ANEMONE_ARM),
  bit([4, 9, 7], [5, 10, 8], ANEMONE_TIP),
  bit([11, 10, 8], [12, 11, 9], ANEMONE_TIP),
  bit([6, 11, 6], [7, 12, 7], ANEMONE_TIP),
  bit([9, 10, 9], [10, 11, 10], ANEMONE_TIP)
];

const POLYP_BITS: &[Bit] = &[
  bit([4, 0, 4], [5, 6, 5], STEM_CYAN),
  skin([3, 6, 3], [6, 9, 6], Tile::GlowPolyp),
  bit([9, 0, 8], [10, 9, 9], STEM_CYAN),
  skin([8, 9, 7], [11, 12, 10], Tile::GlowPolyp),
  bit([11, 0, 3], [12, 4, 4], STEM_CYAN),
  skin([10, 4, 2], [13, 6, 5], Tile::GlowPolyp),
  bit([5, 0, 11], [6, 3, 12], STEM_CYAN),
  skin([4, 3, 10], [7, 5, 13], Tile::GlowPolyp)
];

const WORM_BITS: &[Bit] = &[
  bit([3, 0, 4], [5, 10, 6], WORM_WHITE),
  bit([3, 10, 4], [5, 13, 6], WORM_RED),
  bit([8, 0, 8], [10, 13, 10], WORM_WHITE),
  bit([8, 13, 8], [10, 16, 10], WORM_RED),
  bit([11, 0, 3], [13, 8, 5], WORM_WHITE),
  bit([11, 8, 3], [13, 11, 5], WORM_RED),
  bit([5, 0, 11], [7, 6, 13], WORM_WHITE),
  bit([5, 6, 11], [7, 8, 13], WORM_RED)
];

const CLAM_BITS: &[Bit] = &[
  bit([2, 0, 3], [14, 3, 13], SHELL_GREY),
  bit([3, 3, 4], [13, 4, 12], MANTLE_BLUE),
  bit([2, 4, 5], [14, 6, 13], SHELL_GREY),
  bit([4, 6, 7], [12, 7, 13], SHELL_GREY)
];

const AETHER_LAMP_BITS: &[Bit] = &[
  skin([2, 0, 2], [14, 3, 14], Tile::PrecursorAlloy),
  skin([4, 3, 4], [12, 5, 12], Tile::PrecursorAlloy),
  skin([5, 5, 5], [11, 13, 11], Tile::AetherLamp),
  skin([4, 13, 4], [12, 15, 12], Tile::PrecursorAlloy),
  skin([7, 15, 7], [9, 16, 9], Tile::AetherConduit)
];

const BIO_LAB_BITS: &[Bit] = &[
  skin([0, 0, 0], [16, 4, 16], Tile::Steel),
  skin([2, 4, 2], [14, 13, 14], Tile::BioLabSide),
  skin([1, 13, 1], [15, 16, 15], Tile::BioLabTop),
  skin([3, 4, 0], [6, 9, 1], Tile::Steel)
];

pub fn bits(block: Block) -> &'static [Bit] {
  match block {
    torch if torch.torch() => TORCH_BITS,
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
    Block::Brazier => BRAZIER_BITS,
    Block::Urn => URN_BITS,
    Block::Chest => CHEST_BITS,
    Block::EmptyChest => EMPTY_CHEST_BITS,
    Block::CraftingTable => TABLE_BITS,
    Block::Furnace => FURNACE_BITS,
    Block::Tank => TANK_BITS,
    Block::Pumpjack => PUMPJACK_BITS,
    Block::Refinery => REFINERY_BITS,
    Block::Fermenter => FERMENTER_BITS,
    Block::SteamEngine => STEAM_ENGINE_BITS,
    Block::BiofuelGenerator => BIOFUEL_BITS,
    Block::DieselGenerator => DIESEL_BITS,
    Block::GeothermalGenerator => GEOTHERMAL_BITS,
    Block::SolarPanel => SOLAR_BITS,
    Block::WindTurbine => TURBINE_BITS,
    Block::Battery => BATTERY_BITS,
    Block::ElectricPump => PUMP_BITS,
    Block::ElectricFurnace => ELECTRIC_FURNACE_BITS,
    Block::ElectricLamp => LAMP_BITS,
    Block::ElectricLampLit => LIT_LAMP_BITS,
    Block::Conveyor => &FLAT_BELT_BITS,
    Block::ConveyorEast => &FLAT_EAST_BITS,
    Block::ConveyorSouth => &FLAT_SOUTH_BITS,
    Block::ConveyorWest => &FLAT_WEST_BITS,
    Block::ConveyorUp | Block::ConveyorDownSouth => &RAMP_BITS,
    Block::ConveyorUpEast | Block::ConveyorDownWest => &RAMP_EAST_BITS,
    Block::ConveyorUpSouth | Block::ConveyorDown => &RAMP_SOUTH_BITS,
    Block::ConveyorUpWest | Block::ConveyorDownEast => &RAMP_WEST_BITS,
    Block::GlimmerPink => &GLIMMER_PINK_BITS,
    Block::GlimmerCyan => &GLIMMER_CYAN_BITS,
    Block::GlimmerViolet => &GLIMMER_VIOLET_BITS,
    Block::GlimmerGold => &GLIMMER_GOLD_BITS,
    Block::Sunsip => DAYBLOOM_BITS,
    Block::Dewcup => HYDROANGEA_BITS,
    Block::Emberbell => ENDOFLAME_BITS,
    Block::ManaCistern => POOL_BITS,
    Block::Cauldron => CAULDRON_BITS,
    Block::CrimsonAltar => CRIMSON_ALTAR_BITS,
    Block::Plinth => PLINTH_BITS,
    Block::ConfluenceAltar => CONFLUENCE_ALTAR_BITS,
    Block::BloomFont => APOTHECARY_BITS,
    Block::Moonlace => MOONLACE_BITS,
    Block::Crusher => CRUSHER_BITS,
    Block::Kelp => KELP_BITS,
    Block::Glowkelp => GLOWKELP_BITS,
    Block::Seagrass => SEAGRASS_BITS,
    Block::FanCoral => FAN_BITS,
    Block::TubeCoral => TUBE_BITS,
    Block::Anemone => ANEMONE_BITS,
    Block::GlowPolyp => POLYP_BITS,
    Block::VentWorms => WORM_BITS,
    Block::Clam => CLAM_BITS,
    Block::AetherLamp => AETHER_LAMP_BITS,
    Block::BioLab => BIO_LAB_BITS,
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

const WALL_TORCH_LEAN: f32 = 0.4;
const WALL_TORCH_FOOT: Vec3 = Vec3::new(8.0, 3.0, 8.0);
const WALL_TORCH_SETBACK: f32 = 7.5;
const MIDDLE: Vec3 = Vec3::new(8.0, 0.0, 8.0);

pub fn pose(block: Block) -> Affine3A {
  block.torch().then(|| block.wall()).flatten().map_or(Affine3A::IDENTITY, |wall| {
    let lean = Quat::from_axis_angle(wall.cross(IVec3::Y).as_vec3(), WALL_TORCH_LEAN);
    let foot = WALL_TORCH_FOOT + wall.as_vec3() * WALL_TORCH_SETBACK;
    Affine3A::from_rotation_translation(lean, foot - lean * MIDDLE)
  })
}

const TORCH_FLAME: Vec3 = Vec3::new(8.0, 10.5, 8.0);
const BRAZIER_FLAME: Vec3 = Vec3::new(0.5, 0.8, 0.5);
const BULB: Vec3 = Vec3::new(0.5, 0.6, 0.5);

pub fn flame(block: Block) -> Vec3 {
  match (block.torch(), block) {
    (true, _) => pose(block).transform_point3(TORCH_FLAME) / 16.0,
    (_, Block::ElectricLampLit) => BULB,
    _ => BRAZIER_FLAME
  }
}

const SIGN_BOUNDS: (Vec3, Vec3) = (Vec3::new(4.5, 0.0, 4.5), Vec3::new(11.5, 17.0, 11.5));

pub fn bounds(block: Block, seed: u32, at: IVec3, links: u16) -> Option<(Vec3, Vec3)> {
  let shift = shift(block, seed, at);
  shape(block, links)
    .iter()
    .map(|bit| (Vec3::from(bit.low.map(f32::from)), Vec3::from(bit.high.map(f32::from))))
    .reduce(|(low, high), (bit_low, bit_high)| (low.min(bit_low), high.max(bit_high)))
    .map(|(low, high)| {
      let pose = pose(block);
      (0..8)
        .map(|corner| {
          pose.transform_point3(Vec3::select(
            BVec3::new(corner & 1 != 0, corner & 2 != 0, corner & 4 != 0),
            high,
            low
          ))
        })
        .fold((Vec3::MAX, Vec3::MIN), |(low, high), point| {
          (low.min(point), high.max(point))
        })
    })
    .or(block.sign().then_some(SIGN_BOUNDS))
    .map(|(low, high)| {
      let corner = at.as_vec3();
      (corner + (low + shift) / 16.0, corner + (high + shift) / 16.0)
    })
}

pub fn icon(block: Block, x: u32, y: u32) -> Option<Bit> {
  let (x, y) = (x as u8, 15 - y as u8);
  shape(block, ACROSS)
    .iter()
    .filter(|bit| (bit.low[0]..bit.high[0].max(bit.low[0] + 1)).contains(&x))
    .filter(|bit| (bit.low[1]..bit.high[1].max(bit.low[1] + 1)).contains(&y))
    .min_by_key(|bit| bit.low[2])
    .copied()
}
