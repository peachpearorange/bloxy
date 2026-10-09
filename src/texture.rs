use {crate::{block::{Block, Tile},
             model},
     bevy::{asset::RenderAssetUsages,
            image::{ImageAddressMode, ImageFilterMode, ImageSampler,
                    ImageSamplerDescriptor},
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}}};

pub const PIXELS: u32 = 16;
pub const COLUMNS: u32 = 8;
pub const ROWS: u32 = 7;
const MIPS: u32 = 5;

#[derive(Clone, Copy)]
pub struct Texel {
  pub color: [f32; 4],
  pub glow: [f32; 3]
}

impl Texel {
  fn rgb(r: f32, g: f32, b: f32) -> Texel {
    Texel { color: [r, g, b, 1.0], glow: [0.0; 3] }
  }

  fn scaled(self, by: f32) -> Texel {
    let [r, g, b, a] = self.color;
    Texel { color: [r * by, g * by, b * by, a], ..self }
  }

  fn alpha(self, a: f32) -> Texel {
    Texel { color: [self.color[0], self.color[1], self.color[2], a], ..self }
  }

  fn glowing(self, by: f32) -> Texel {
    let [r, g, b, _] = self.color;
    Texel { glow: [r * by, g * by, b * by], ..self }
  }
}

type Art = [&'static str; 16];
type Palette = [[f32; 3]; 4];

const STONE: Art = [
  "2222223322222122",
  "2233222222221112",
  "2222211222222222",
  "2222222222332222",
  "1122222222222221",
  "2222233222111222",
  "2222222222221022",
  "2211222222223322",
  "2210222112222222",
  "2332222222222211",
  "2222221222222222",
  "2222222222332222",
  "2112222222222222",
  "2222223322221122",
  "2222222222222102",
  "2223322211222222"
];

const COBBLE: Art = [
  "0001222200012220",
  "0122332201233220",
  "1233322212332221",
  "1232222212222221",
  "1222222101222210",
  "0122221000111100",
  "0011110012222100",
  "0122210123332210",
  "1233221123322221",
  "1232222112222221",
  "1222222101222210",
  "0122221000111100",
  "0011100122100122",
  "2210012332101233",
  "3221012322101232",
  "2221001221000122"
];

const DIRT: Art = [
  "2222122222221222",
  "2132222212222222",
  "2222222222223222",
  "2222212222222212",
  "1222222232222222",
  "2222322222212222",
  "2222222122222222",
  "2122222222222322",
  "2222232222122222",
  "2222222222222222",
  "2232222122222212",
  "2222222222322222",
  "1222212222222222",
  "2222222222221222",
  "2222322222222222",
  "2212222212222232"
];

const BLADES: Art = [
  "2232222122232222",
  "2132212222132212",
  "2222232232222232",
  "2322132132223132",
  "2122222222212222",
  "2222322223222222",
  "2321222212222322",
  "2222232222322122",
  "2232212322122222",
  "2122222122222232",
  "2222322222232212",
  "3222122322212222",
  "2222222122222322",
  "2232232222322122",
  "2132122223122222",
  "2222222222222222"
];

const FRINGE: Art = [
  "2323232332323232",
  "2222222222222222",
  "1221212221221212",
  "1.21.1.21..2.1.1",
  "..1....1....1...",
  "................",
  "................",
  "................",
  "................",
  "................",
  "................",
  "................",
  "................",
  "................",
  "................",
  "................"
];

const DRIFT: Art = [
  "2222222222222222",
  "2232222222322222",
  "2222221122222222",
  "2222222222222232",
  "3222222222222222",
  "2222222232222122",
  "2211222222222222",
  "2222222222223222",
  "2222232222222222",
  "2222222222112222",
  "1222222222222222",
  "2222222322222223",
  "2222222222222222",
  "2223222112222222",
  "2222222222222232",
  "2222112222222222"
];

const PEBBLES: Art = [
  "1221012210122321",
  "2332123321233332",
  "2321023210123221",
  "1210012100012110",
  "0101221012210101",
  "1012332123321012",
  "2123321123321123",
  "1012210012210012",
  "2101100121100121",
  "3210123321012332",
  "3321233210123321",
  "2210123210012210",
  "1100012100121100",
  "0122101210123321",
  "1233212321233332",
  "1232101221012321"
];

const GROOVES: Art = [
  "2103212310232120",
  "2103212310232120",
  "2203211310232210",
  "2213211320132210",
  "1213221320132211",
  "1203221320122211",
  "1202231210123210",
  "2102231210123210",
  "2102331210113210",
  "2103321211103220",
  "2103321201103220",
  "2113221201203221",
  "2123211201203221",
  "1123210202213211",
  "1023210302213210",
  "1023210312212210"
];

const PAPER: Art = [
  "3332233333322333",
  "3322223333222233",
  "3333333333333333",
  "3330003333333323",
  "3332223333333333",
  "3333333332333333",
  "2333333300003333",
  "3333333322223333",
  "3333333333333333",
  "3333233333333233",
  "3000033333333333",
  "3222233333233333",
  "3333333333333300",
  "3333333233333322",
  "3323333333333333",
  "3333330003333333"
];

const PLATES: Art = [
  "1221012210122101",
  "2332123321233212",
  "2321123211232112",
  "2221122211222112",
  "1100011000110001",
  "0122101221012210",
  "1233212332123321",
  "1232112321123211",
  "1222112221122211",
  "0110001100011000",
  "2101221012210122",
  "3212332123321233",
  "3211232112321123",
  "2211222112221122",
  "1000110001100011",
  "1221012210122101"
];

const RINGS: Art = [
  "0111011101110111",
  "2223222322232223",
  "2332333233323332",
  "1222122212221222",
  "1101110111011101",
  "2322232223222322",
  "3323332333233323",
  "2212221222122212",
  "0111011101110111",
  "2223222322232223",
  "2332333233323332",
  "1222122212221222",
  "1101110111011101",
  "2322232223222322",
  "3323332333233323",
  "2212221222122212"
];

const FOLIAGE: Art = [
  "21.2232.12322.12",
  "3212.1223231.123",
  "2.1232.2.212332.",
  "123.2123323.1221",
  ".221232.1.22321.",
  "2312.123323.2123",
  "1.232212.1223.32",
  "23212.3223212.21",
  ".12322.1232.3221",
  "2321.23212.12322",
  "12.2312.3221232.",
  "3221232.12.23212",
  "2.12322321.232.1",
  "121.2.12322.1223",
  "2322312.2.123221",
  ".2123221232.2312"
];

const FRONDS: Art = [
  "3.21..3.21..3.21",
  ".21..3.21..3.21.",
  "21..3.21..3.21..",
  "1..3.21..3.21..3",
  "..3.21..3.21..3.",
  ".3.21..3.21..3.2",
  "3.21..3.21..3.21",
  ".21..3.21..3.21.",
  "21..3.21..3.21..",
  "1..3.21..3.21..3",
  "..3.21..3.21..3.",
  ".3.21..3.21..3.2",
  "3.21..3.21..3.21",
  ".21..3.21..3.21.",
  "21..3.21..3.21..",
  "1..3.21..3.21..3"
];

const SPOTS: Art = [
  "................",
  "..12......1.....",
  ".1221....121....",
  "..11......1.....",
  "..........12....",
  "......12..221...",
  ".....1221..1....",
  "......11........",
  "..1.............",
  ".121.......12...",
  "..1.......1221..",
  "...........11...",
  "....12..........",
  "...1221....1....",
  "....11....121...",
  "...........1...."
];

const DOTTED: Art = [
  "2222222222222222",
  "2233322222222222",
  "2333332222223322",
  "2333332222233332",
  "2233322222233332",
  "2222222222223322",
  "1222222222222221",
  "2222222333222222",
  "2222223333322222",
  "2222223333322222",
  "2222222333222222",
  "2222222222222222",
  "2233222222222332",
  "2333322222223333",
  "2233222222222332",
  "1222222112222222"
];

const FIBRES: Art = [
  "2232223222322232",
  "2232223221322232",
  "2231223222322232",
  "2232223222321232",
  "2232213222322232",
  "2232223222322232",
  "2132223222322231",
  "2232223212322232",
  "2232223222322232",
  "2232223222312232",
  "2232123222322232",
  "2232223222322132",
  "1232223222322232",
  "2232223221322232",
  "2232223222322232",
  "2231223222322232"
];

const MAGMA: Art = [
  "2223322111222332",
  "2233332210122333",
  "2333322100012233",
  "2233221000001223",
  "1222210000011222",
  "0122100001122223",
  "0011000112222333",
  "1100011223333332",
  "2211122333322221",
  "2222223332221110",
  "3322222221110000",
  "3333222211000001",
  "2333322210000012",
  "2223332221000122",
  "1222333322111222",
  "1122233332222222"
];

const CRACKS: Art = [
  "2222222222223222",
  "2222222222232222",
  "2221222222322222",
  "2222122223222222",
  "2222212232222222",
  "2222221322222222",
  "2222222322222212",
  "2222223222222122",
  "2222232222221222",
  "2222322222212222",
  "3223222222122222",
  "2332222221222222",
  "2222222222222223",
  "2222222222222232",
  "2212222222222322",
  "2222222222223222"
];

const SWELL: Art = [
  "2222222222222222",
  "2222333322222222",
  "2233222233222222",
  "1222222222222111",
  "2222222222222222",
  "2222222222233332",
  "3322222222332222",
  "2211122222222222",
  "2222222222222222",
  "2222223333222222",
  "2222332222332222",
  "2222222222221112",
  "2222222222222222",
  "3332222222222233",
  "2223322222223322",
  "2111222222222222"
];

const GREY: Palette =
  [[0.3, 0.3, 0.32], [0.4, 0.4, 0.42], [0.48, 0.48, 0.5], [0.56, 0.56, 0.58]];
const COBBLES: Palette =
  [[0.2, 0.2, 0.21], [0.34, 0.34, 0.36], [0.46, 0.46, 0.48], [0.58, 0.58, 0.6]];
const EARTH: Palette =
  [[0.25, 0.17, 0.11], [0.36, 0.25, 0.16], [0.44, 0.31, 0.2], [0.53, 0.39, 0.26]];
const GREEN: Palette =
  [[0.17, 0.33, 0.12], [0.23, 0.42, 0.16], [0.29, 0.5, 0.2], [0.38, 0.6, 0.25]];
const WHITE: Palette =
  [[0.7, 0.75, 0.82], [0.82, 0.86, 0.92], [0.92, 0.95, 0.99], [1.0, 1.0, 1.0]];
const SPORES: Palette =
  [[0.32, 0.26, 0.35], [0.4, 0.33, 0.43], [0.47, 0.39, 0.49], [0.58, 0.5, 0.58]];
const SANDY: Palette =
  [[0.7, 0.62, 0.42], [0.8, 0.72, 0.5], [0.86, 0.79, 0.56], [0.92, 0.87, 0.66]];
const STONY: Palette =
  [[0.26, 0.25, 0.24], [0.4, 0.39, 0.37], [0.52, 0.51, 0.49], [0.64, 0.63, 0.61]];
const CLAYEY: Palette =
  [[0.5, 0.53, 0.6], [0.56, 0.59, 0.66], [0.61, 0.64, 0.71], [0.68, 0.71, 0.78]];
const BLACK: Palette =
  [[0.08, 0.08, 0.09], [0.18, 0.18, 0.19], [0.3, 0.3, 0.31], [0.42, 0.42, 0.43]];
const OAK: Palette =
  [[0.22, 0.15, 0.09], [0.3, 0.21, 0.13], [0.38, 0.27, 0.17], [0.45, 0.33, 0.21]];
const BIRCH: Palette =
  [[0.12, 0.11, 0.1], [0.62, 0.61, 0.57], [0.8, 0.79, 0.75], [0.9, 0.89, 0.86]];
const SPRUCE: Palette =
  [[0.16, 0.1, 0.06], [0.22, 0.15, 0.09], [0.29, 0.2, 0.12], [0.35, 0.25, 0.15]];
const PALM: Palette =
  [[0.38, 0.29, 0.18], [0.48, 0.38, 0.24], [0.57, 0.46, 0.3], [0.66, 0.55, 0.37]];
const OAK_LEAVES: Palette =
  [[0.11, 0.27, 0.09], [0.16, 0.36, 0.12], [0.22, 0.45, 0.16], [0.3, 0.55, 0.21]];
const BIRCH_LEAVES: Palette =
  [[0.24, 0.38, 0.12], [0.33, 0.49, 0.17], [0.42, 0.6, 0.22], [0.52, 0.69, 0.29]];
const SPRUCE_LEAVES: Palette =
  [[0.06, 0.17, 0.12], [0.09, 0.23, 0.16], [0.12, 0.29, 0.2], [0.17, 0.36, 0.25]];
const PALM_LEAVES: Palette =
  [[0.15, 0.36, 0.08], [0.22, 0.47, 0.11], [0.3, 0.58, 0.15], [0.4, 0.68, 0.22]];
const AMANITA: Palette =
  [[0.45, 0.06, 0.05], [0.62, 0.09, 0.08], [0.76, 0.12, 0.1], [0.94, 0.91, 0.86]];
const BOLETE: Palette =
  [[0.36, 0.25, 0.16], [0.46, 0.33, 0.22], [0.55, 0.4, 0.27], [0.64, 0.49, 0.34]];
const STALK: Palette =
  [[0.6, 0.57, 0.5], [0.74, 0.71, 0.63], [0.84, 0.81, 0.73], [0.9, 0.88, 0.81]];
const EMBERS: Palette =
  [[0.28, 0.08, 0.03], [0.75, 0.22, 0.04], [0.97, 0.42, 0.07], [1.0, 0.75, 0.26]];
const FROZEN: Palette =
  [[0.5, 0.65, 0.84], [0.57, 0.72, 0.9], [0.64, 0.8, 0.96], [0.8, 0.9, 1.0]];
const FOAM: Palette =
  [[0.78, 0.78, 0.78], [0.86, 0.86, 0.86], [0.93, 0.93, 0.93], [1.0, 1.0, 1.0]];

fn drawn(art: &Art, palette: &Palette, x: u32, y: u32) -> Option<Texel> {
  art[y as usize]
    .as_bytes()
    .get(x as usize)
    .and_then(|&mark| (mark as char).to_digit(10))
    .map(|shade| {
      let [r, g, b] = palette[shade as usize];
      Texel::rgb(r, g, b)
    })
}

fn shade_of(art: &Art, x: u32, y: u32) -> Option<u32> {
  art[y as usize].as_bytes().get(x as usize).and_then(|&mark| (mark as char).to_digit(10))
}

fn solid(art: &Art, palette: &Palette, x: u32, y: u32) -> Texel {
  drawn(art, palette, x, y).unwrap_or(Texel::rgb(1.0, 0.0, 1.0))
}

fn fringed(over: &Palette, x: u32, y: u32) -> Texel {
  drawn(&FRINGE, over, x, y).unwrap_or_else(|| solid(&DIRT, &EARTH, x, y))
}

fn cutout(art: &Art, palette: &Palette, x: u32, y: u32) -> Texel {
  drawn(art, palette, x, y).unwrap_or(Texel::rgb(0.0, 0.0, 0.0).alpha(0.0))
}

fn ore(x: u32, y: u32, [dark, base, light]: [[f32; 3]; 3], glow: f32) -> Texel {
  drawn(&SPOTS, &[dark, base, light, light], x, y)
    .map(|texel| texel.glowing(glow))
    .unwrap_or_else(|| solid(&STONE, &GREY, x, y))
}

fn waystone(x: u32, y: u32) -> Texel {
  let rim = x == 0 || y == 0 || x == PIXELS - 1 || y == PIXELS - 1;
  let inner = x == 1 || y == 1 || x == PIXELS - 2 || y == PIXELS - 2;
  Texel::rgb(0.2, 0.22, 0.28).scaled(match (rim, inner) {
    (true, _) => 1.45,
    (_, true) => 0.8,
    _ => 1.0
  })
}

fn rune() -> Texel { Texel::rgb(0.3, 0.85, 1.0).glowing(0.7) }

pub fn paint(tile: Tile, x: u32, y: u32) -> Texel {
  match tile {
    Tile::Stone => solid(&STONE, &GREY, x, y),
    Tile::Cobblestone => solid(&COBBLE, &COBBLES, x, y),
    Tile::Dirt => solid(&DIRT, &EARTH, x, y),
    Tile::GrassTop => solid(&BLADES, &GREEN, x, y),
    Tile::GrassSide => fringed(&GREEN, x, y),
    Tile::Sand => solid(&DRIFT, &SANDY, x, y),
    Tile::Gravel => solid(&PEBBLES, &STONY, x, y),
    Tile::Clay => solid(&DRIFT, &CLAYEY, x, y),
    Tile::Snow => solid(&DRIFT, &WHITE, x, y),
    Tile::SnowSide => fringed(&WHITE, x, y),
    Tile::Bedrock => solid(&COBBLE, &BLACK, x, y),
    Tile::LogSide => solid(&GROOVES, &OAK, x, y),
    Tile::LogTop => {
      let ring = (x as i32 * 2 - 15).abs().max((y as i32 * 2 - 15).abs()) / 2;
      match ring {
        7 => Texel::rgb(0.36, 0.26, 0.16),
        ring if ring % 2 == 0 => Texel::rgb(0.66, 0.52, 0.33),
        _ => Texel::rgb(0.57, 0.44, 0.27)
      }
    }
    Tile::Leaves => cutout(&FOLIAGE, &OAK_LEAVES, x, y),
    Tile::Planks => {
      let board = y / 4;
      let seam = y % 4 == 3 || (x + board * 5) % 16 == 0;
      let grain = y % 4 == 1 && (x + board * 7) % 6 < 2;
      Texel::rgb(0.66, 0.5, 0.3).scaled(match (seam, grain) {
        (true, _) => 0.62,
        (_, true) => 0.88,
        _ => 1.0 - (board % 2) as f32 * 0.05
      })
    }
    Tile::Glass => {
      let frame = x == 0 || y == 0 || x == PIXELS - 1 || y == PIXELS - 1;
      let streak = (x + y == 5 || x + y == 7 || x + y == 20) && x > 1 && y > 1;
      match (frame, streak) {
        (true, _) => Texel::rgb(0.78, 0.86, 0.9),
        (false, true) => Texel::rgb(0.9, 0.95, 1.0).alpha(0.9),
        _ => Texel::rgb(0.0, 0.0, 0.0).alpha(0.0)
      }
    }
    Tile::Water => solid(&SWELL, &FOAM, x, y),
    Tile::CoalOre => {
      ore(x, y, [[0.05, 0.05, 0.06], [0.12, 0.12, 0.13], [0.22, 0.22, 0.24]], 0.0)
    }
    Tile::IronOre => {
      ore(x, y, [[0.55, 0.4, 0.32], [0.76, 0.6, 0.5], [0.88, 0.75, 0.65]], 0.0)
    }
    Tile::CopperOre => {
      ore(x, y, [[0.55, 0.28, 0.12], [0.82, 0.47, 0.22], [0.95, 0.65, 0.38]], 0.0)
    }
    Tile::TinOre => {
      ore(x, y, [[0.6, 0.62, 0.66], [0.8, 0.82, 0.85], [0.93, 0.94, 0.96]], 0.0)
    }
    Tile::GoldOre => {
      ore(x, y, [[0.7, 0.52, 0.1], [0.96, 0.8, 0.24], [1.0, 0.93, 0.55]], 0.35)
    }
    Tile::DiamondOre => {
      ore(x, y, [[0.18, 0.6, 0.65], [0.45, 0.92, 0.95], [0.8, 1.0, 1.0]], 1.0)
    }
    Tile::Lamp => {
      let rim = x == 0 || y == 0 || x == PIXELS - 1 || y == PIXELS - 1;
      let lattice = x % 5 == 0 || y % 5 == 0;
      match (rim, lattice) {
        (true, _) => Texel::rgb(0.3, 0.24, 0.16),
        (false, true) => Texel::rgb(0.95, 0.75, 0.4).glowing(1.2),
        _ => Texel::rgb(1.0, 0.86, 0.55).glowing(2.0)
      }
    }
    Tile::Bricks => {
      let course = y / 4;
      let mortar = y % 4 == 3 || (x + course % 2 * 4) % 8 == 7;
      let brick = (x + course % 2 * 4) / 8 + course * 3;
      match (mortar, y % 4 == 0) {
        (true, _) => Texel::rgb(0.62, 0.6, 0.56),
        (false, true) => Texel::rgb(0.68, 0.33, 0.25),
        _ => Texel::rgb(0.6, 0.27, 0.2).scaled(0.9 + (brick % 3) as f32 * 0.06)
      }
    }
    Tile::BirchSide => solid(&PAPER, &BIRCH, x, y),
    Tile::BirchLeaves => cutout(&FOLIAGE, &BIRCH_LEAVES, x, y),
    Tile::SpruceSide => solid(&PLATES, &SPRUCE, x, y),
    Tile::SpruceLeaves => cutout(&FOLIAGE, &SPRUCE_LEAVES, x, y),
    Tile::PalmSide => solid(&RINGS, &PALM, x, y),
    Tile::PalmLeaves => cutout(&FRONDS, &PALM_LEAVES, x, y),
    Tile::MyceliumTop => solid(&BLADES, &SPORES, x, y),
    Tile::MyceliumSide => fringed(&SPORES, x, y),
    Tile::Stem => solid(&FIBRES, &STALK, x, y),
    Tile::RedCap => solid(&DOTTED, &AMANITA, x, y),
    Tile::BrownCap => solid(&STONE, &BOLETE, x, y),
    Tile::Pores => {
      Texel::rgb(0.8, 0.74, 0.62).scaled(if x % 2 == 0 { 0.82 } else { 1.0 })
    }
    Tile::Basalt => {
      let joint = (x + y / 6 * 3) % 5 == 0 || y % 6 == 5;
      let column = (x + y / 6 * 3) / 5;
      Texel::rgb(0.2, 0.2, 0.22).scaled(match joint {
        true => 0.6,
        false => 0.95 + (column % 3) as f32 * 0.1
      })
    }
    Tile::Lava => {
      let shade = shade_of(&MAGMA, x, y).unwrap_or(2);
      solid(&MAGMA, &EMBERS, x, y).glowing([0.05, 0.15, 0.22, 0.35][shade as usize])
    }
    Tile::Ice => solid(&CRACKS, &FROZEN, x, y),
    Tile::WaystoneSide => {
      let (dx, dy) = ((x as f32 - 7.5).abs(), (y as f32 - 7.5).abs());
      let diamond = (4.0..5.5).contains(&(dx + dy));
      let spine = dx < 1.0 && (2.0..14.0).contains(&(y as f32));
      match diamond || spine {
        true => rune(),
        false => waystone(x, y)
      }
    }
    Tile::Blank => Texel::rgb(1.0, 1.0, 1.0),
    Tile::Poppy
    | Tile::Dandelion
    | Tile::Cornflower
    | Tile::Daisy
    | Tile::OakSapling
    | Tile::BirchSapling
    | Tile::SpruceSapling
    | Tile::PalmSapling
    | Tile::RedMushroom
    | Tile::BrownMushroom => {
      let block = Block::ALL_MODELS[tile as usize - Tile::Poppy as usize];
      model::icon(block, x, y)
        .map_or(Texel::rgb(0.0, 0.0, 0.0).alpha(0.0), |[r, g, b]| Texel::rgb(r, g, b))
    }
    Tile::WaystoneTop => {
      let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
      let ring = (dx * dx + dy * dy).sqrt();
      match (4.0..5.2).contains(&ring) || ring < 1.6 {
        true => rune(),
        false => waystone(x, y)
      }
    }
  }
}

fn to_srgb(linear_ish: f32) -> u8 { (linear_ish.clamp(0.0, 1.0) * 255.0).round() as u8 }

fn atlas_levels(texel: impl Fn(Tile, u32, u32) -> [f32; 4]) -> Vec<u8> {
  let (width, height) = (COLUMNS * PIXELS, ROWS * PIXELS);
  let base: Vec<[f32; 4]> = (0..width * height)
    .map(|index| {
      let (x, y) = (index % width, index / width);
      let slot = (y / PIXELS) * COLUMNS + x / PIXELS;
      Tile::ALL
        .get(slot as usize)
        .map_or([0.0; 4], |&tile| texel(tile, x % PIXELS, y % PIXELS))
    })
    .collect();
  let levels =
    std::iter::successors(Some((base, width, height)), |(level, width, height)| {
      (*width > COLUMNS).then(|| {
        let (half_width, half_height) = (width / 2, height / 2);
        let smaller = (0..half_width * half_height)
          .map(|index| {
            let (x, y) = (index % half_width * 2, index / half_width * 2);
            let quad = [(0, 0), (1, 0), (0, 1), (1, 1)]
              .map(|(dx, dy)| level[((y + dy) * width + x + dx) as usize]);
            let coverage: f32 = quad.iter().map(|texel| texel[3]).sum();
            let weighted = |channel: usize| {
              quad.iter().map(|texel| texel[channel] * texel[3].max(0.02)).sum::<f32>()
                / quad.iter().map(|texel| texel[3].max(0.02)).sum::<f32>()
            };
            [weighted(0), weighted(1), weighted(2), coverage / 4.0]
          })
          .collect();
        (smaller, half_width, half_height)
      })
    });
  levels
    .take(MIPS as usize)
    .flat_map(|(level, _, _)| level.into_iter().flat_map(|texel| texel.map(to_srgb)))
    .collect()
}

fn image(data: Vec<u8>) -> Image {
  let mut image = Image::new_uninit(
    Extent3d { width: COLUMNS * PIXELS, height: ROWS * PIXELS, depth_or_array_layers: 1 },
    TextureDimension::D2,
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::RENDER_WORLD
  );
  image.data = Some(data);
  image.texture_descriptor.mip_level_count = MIPS;
  image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
    address_mode_u: ImageAddressMode::ClampToEdge,
    address_mode_v: ImageAddressMode::ClampToEdge,
    mag_filter: ImageFilterMode::Nearest,
    min_filter: ImageFilterMode::Nearest,
    mipmap_filter: ImageFilterMode::Linear,
    ..default()
  });
  image
}

pub fn albedo() -> Image { image(atlas_levels(|tile, x, y| paint(tile, x, y).color)) }

pub fn glow() -> Image {
  image(atlas_levels(|tile, x, y| {
    let [r, g, b] = paint(tile, x, y).glow;
    [r, g, b, 1.0]
  }))
}

pub fn uv_corner(tile: Tile, corner: Vec2) -> Vec2 {
  let index = tile.index();
  let cell = Vec2::new((index % COLUMNS) as f32, (index / COLUMNS) as f32);
  (cell + corner * 0.996 + 0.002) / Vec2::new(COLUMNS as f32, ROWS as f32)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tiles_paint_without_gaps() {
    Tile::ALL.into_iter().for_each(|tile| {
      (0..PIXELS * PIXELS).for_each(|index| {
        let texel = paint(tile, index % PIXELS, index / PIXELS);
        assert!(texel.color != [1.0, 0.0, 1.0, 1.0], "{tile:?} has a hole at {index}")
      })
    })
  }

  #[test]
  #[ignore]
  fn atlas() {
    let (width, height, zoom) = (COLUMNS * PIXELS, ROWS * PIXELS, 4);
    let pixels: Vec<u8> = (0..width * zoom * height * zoom)
      .flat_map(|index| {
        let (x, y) = (index % (width * zoom) / zoom, index / (width * zoom) / zoom);
        Tile::ALL.get(((y / PIXELS) * COLUMNS + x / PIXELS) as usize).map_or(
          [0; 3],
          |&tile| {
            let [r, g, b, _] = paint(tile, x % PIXELS, y % PIXELS).color;
            [r, g, b].map(to_srgb)
          }
        )
      })
      .collect();
    let mut file = format!("P6 {} {} 255\n", width * zoom, height * zoom).into_bytes();
    file.extend(pixels);
    std::fs::write("screenshots/atlas.ppm", file).unwrap()
  }
}
