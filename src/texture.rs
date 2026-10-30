use {crate::{block::{Block, Look, Tile},
             gear::Material,
             model},
     bevy::{asset::RenderAssetUsages,
            image::{ImageAddressMode, ImageFilterMode, ImageSampler,
                    ImageSamplerDescriptor},
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}}};

pub const PIXELS: u32 = 16;
pub const COLUMNS: u32 = 8;
pub const FORGED: u32 = Tile::ALL.len() as u32;
const MATERIALS: u32 = Material::ALL.len() as u32;
const PAINTED: u32 = FORGED + Block::TOOLS.len() as u32 * MATERIALS;
pub const ROWS: u32 = PAINTED.div_ceil(COLUMNS);
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
pub type Palette = [[f32; 3]; 4];

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
  "2221022121000002",
  "1120212232002300",
  "0130211222022220",
  "0000112220222222",
  "2020000000232231",
  "0222002120022322",
  "2222032212002320",
  "2221022112102203",
  "1223023222300032",
  "2222022122023002",
  "2221003230222100",
  "0000002101222110",
  "2121300021222120",
  "3221100002222103",
  "2121203300222202",
  "2222023320022022"
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
  "2332222223322222",
  "2332222223322222",
  "2112222221122222",
  "2222222222222222",
  "2222233222222332",
  "2222233222222332",
  "2222211222222112",
  "2222222222222222",
  "2332222223322222",
  "2332222223322222",
  "2112222221122222",
  "2222222222222222",
  "2222233222222332",
  "2222233222222332",
  "2222211222222112"
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
const EARTH: Palette =
  [[0.25, 0.17, 0.11], [0.36, 0.25, 0.16], [0.44, 0.31, 0.2], [0.53, 0.39, 0.26]];
const GREEN: Palette =
  [[0.12, 0.27, 0.08], [0.2, 0.39, 0.12], [0.3, 0.5, 0.17], [0.44, 0.62, 0.24]];
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

fn shades(base: [f32; 3], spread: [f32; 4]) -> Palette {
  spread.map(|by| base.map(|channel| channel * by))
}

const STONE_SPREAD: [f32; 4] = [0.62, 0.83, 1.0, 1.17];

fn rock(tile: Tile) -> [f32; 3] {
  match tile {
    Tile::Granite | Tile::GraniteCobble => [0.6, 0.42, 0.37],
    Tile::Diorite | Tile::DioriteCobble => [0.72, 0.71, 0.67],
    Tile::Andesite | Tile::AndesiteCobble => [0.4, 0.44, 0.42],
    Tile::Limestone | Tile::LimestoneCobble => [0.74, 0.68, 0.53],
    _ => [0.25, 0.27, 0.33]
  }
}

const GRAIN: Palette =
  [[0.74, 0.74, 0.74], [0.84, 0.84, 0.84], [0.93, 0.93, 0.93], [1.0, 1.0, 1.0]];

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

fn lattice(seed: u32, cells: u32, x: u32, y: u32) -> f32 {
  let scale = cells as f32 / PIXELS as f32;
  let (u, v) = ((x as f32 + 0.5) * scale, (y as f32 + 0.5) * scale);
  let (cell_x, cell_y) = (u.floor(), v.floor());
  let (fx, fy) = (u - cell_x, v - cell_y);
  let corner = |dx: u32, dy: u32| {
    let wrap = |at: f32, by: u32| ((at as u32 + by) % cells) as i32;
    crate::noise::unit(seed, wrap(cell_x, dx), wrap(cell_y, dy), 0)
  };
  let ease = |t: f32| t * t * (3.0 - 2.0 * t);
  let (sx, sy) = (ease(fx), ease(fy));
  let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * sx;
  let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * sx;
  top + (bottom - top) * sy
}

fn ramp(palette: &Palette, along: f32) -> Texel {
  let position = along.clamp(0.0, 1.0) * 3.0;
  let low = (position.floor() as usize).min(2);
  let blend = position - low as f32;
  let [r, g, b] = std::array::from_fn(|channel| {
    palette[low][channel] + (palette[low + 1][channel] - palette[low][channel]) * blend
  });
  Texel::rgb(r, g, b)
}

fn smooth(palette: &Palette, x: u32, y: u32) -> Texel { flecked(&STONE, palette, x, y) }

fn flecked(art: &Art, palette: &Palette, x: u32, y: u32) -> Texel {
  let cloud = lattice(0x57, 4, x, y) * 0.55
    + lattice(0x58, 8, x, y) * 0.3
    + lattice(0x59, 16, x, y) * 0.15;
  let fleck = match shade_of(art, x, y) {
    Some(0) => -0.28,
    Some(1) => -0.14,
    Some(3) => 0.14,
    _ => 0.0
  };
  ramp(palette, 0.15 + cloud * 0.75 + fleck)
}

fn speck(salt: u32, x: u32, y: u32) -> f32 {
  crate::noise::unit(salt, x as i32, y as i32, 0)
}

fn wave(salt: u32, cells: u32, x: f32, row: u32) -> f32 {
  let at = x.rem_euclid(PIXELS as f32) * cells as f32 / PIXELS as f32;
  let (cell, blend) = (at.floor() as u32, at.fract());
  let knot = |cell: u32| crate::noise::unit(salt, (cell % cells) as i32, row as i32, 1);
  let eased = blend * blend * (3.0 - 2.0 * blend);
  knot(cell) + (knot(cell + 1) - knot(cell)) * eased
}

fn toroidal(from: Vec2, to: Vec2) -> Vec2 {
  let size = PIXELS as f32;
  let offset = to - from;
  offset - (offset / size).round() * size
}

const STONES_OF_COBBLE: [(f32, f32, f32); 7] = [
  (3.0, 3.0, 1.1),
  (10.5, 2.0, 1.0),
  (14.5, 8.5, 0.9),
  (7.5, 9.0, 1.15),
  (1.5, 11.5, 0.95),
  (11.0, 14.5, 1.05),
  (5.0, 15.0, 0.8)
];

struct Nearest {
  stone: usize,
  offset: Vec2,
  gap: f32,
  fill: f32
}

fn nearest_stone(x: u32, y: u32) -> Nearest {
  let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
  let [(stone, offset, near), (_, _, next)] = STONES_OF_COBBLE
    .iter()
    .enumerate()
    .map(|(index, &(sx, sy, weight))| {
      let offset = toroidal(Vec2::new(sx, sy), at);
      (index, offset, offset.length() / weight)
    })
    .fold([(0, Vec2::ZERO, f32::MAX); 2], |[first, second], candidate| match () {
      () if candidate.2 < first.2 => [candidate, first],
      () if candidate.2 < second.2 => [first, candidate],
      () => [first, second]
    });
  Nearest { stone, offset, gap: next - near, fill: 2.0 * near / (near + next) }
}

const STONE_TINTS: [[f32; 3]; 6] = [
  [1.0, 1.0, 1.0],
  [1.07, 1.0, 0.9],
  [0.93, 0.97, 1.06],
  [0.8, 0.8, 0.82],
  [1.12, 1.1, 1.06],
  [1.0, 0.96, 0.92]
];

const LICHEN: [f32; 3] = [0.5, 0.54, 0.3];

fn cobble(base: [f32; 3], x: u32, y: u32) -> Texel {
  let Nearest { stone, offset, gap, fill } = nearest_stone(x, y);
  let (_, _, weight) = STONES_OF_COBBLE[stone];
  let roll = |salt: u32| crate::noise::unit(0xC0B ^ salt, stone as i32, 0, 0);
  let edge = gap * weight;
  let grit = speck(0xC0F, x, y);
  let colour = match edge < 1.1 {
    true => {
      let under = offset.normalize_or_zero().dot(Vec2::new(0.6, 0.8)).max(0.0);
      base.map(|channel| channel * (0.34 + grit * 0.12 - under * 0.12))
    }
    false => {
      let flat = 0.55 + roll(1) * 0.4;
      let dome = offset.normalize_or_zero() * (fill * flat).min(0.97);
      let normal = dome.extend((1.0 - dome.length_squared()).sqrt());
      let lit = 0.6 + normal.dot(Vec3::new(-0.5, -0.6, 0.62).normalize()).max(0.0) * 0.5;
      let grain =
        (lattice(0xC0E + stone as u32, 8, x, y) - 0.5) * 0.14 + (grit - 0.5) * 0.1;
      let rim = if edge < 2.0 { 0.07 } else { 0.0 };
      let along = Vec2::from_angle(roll(6) * std::f32::consts::PI);
      let crack = roll(5) < 0.4
        && weight > 0.95
        && offset.perp_dot(along).abs() < 0.55
        && (-1.5..2.5).contains(&offset.dot(along))
        && edge > 1.6;
      let tint = STONE_TINTS[(roll(2) * STONE_TINTS.len() as f32) as usize];
      let shade =
        (0.88 + roll(3) * 0.2) * (lit + grain - rim - if crack { 0.35 } else { 0.0 });
      let mossy = roll(4) < 0.18
        && offset.y < 0.8
        && lattice(0xC10 + stone as u32, 8, x, y) > 0.68
        && grit > 0.25;
      let [r, g, b] =
        std::array::from_fn(|channel| base[channel] * tint[channel] * shade);
      match mossy {
        true => std::array::from_fn(|channel| {
          [r, g, b][channel] * 0.35 + LICHEN[channel] * shade * 0.65
        }),
        false => [r, g, b]
      }
    }
  };
  Texel::rgb(colour[0], colour[1], colour[2])
}

fn planks(base: [f32; 3], x: u32, y: u32) -> Texel {
  let (board, row) = (y / 4, y % 4);
  let roll = |salt: u32| crate::noise::unit(0x9A1 ^ salt, board as i32, 0, 0);
  let joint = (board * 7 + 3 + (roll(1) * 4.0) as u32) % PIXELS;
  let piece = u32::from(x >= joint);
  let pick = |salt: u32| crate::noise::unit(0x9A2 ^ salt, board as i32, piece as i32, 0);
  let tone = 0.88 + pick(1) * 0.2;
  let hue = pick(2) - 0.5;
  let shift = pick(3) * 16.0;
  let grain =
    wave(0x9A5, 4, x as f32 + shift, y) * 0.6 + wave(0x9A6, 8, x as f32, y) * 0.4;
  let streak = wave(0x9A7, 3, x as f32 + shift, y) > 0.74;
  let knotty = roll(2) < 0.3;
  let knot_at = (roll(3) * PIXELS as f32).floor();
  let reach = toroidal(Vec2::new(knot_at, 1.5), Vec2::new(x as f32, row as f32 + 0.5));
  let knot = (reach.x / 1.7).powi(2) + (reach.y / 1.0).powi(2);
  let weather = (lattice(0x9A8, 2, x, y) - 0.5) * 0.08;
  let nail = row == 1 && x == (joint + 1) % PIXELS && roll(4) < 0.6;
  let shade = match () {
    () if row == 3 => 0.5 + speck(0x9A9, x, y) * 0.08,
    () if x == joint => 0.6,
    () if knotty && knot < 0.45 => 0.62,
    () if knotty && knot < 1.5 => 0.86,
    () => {
      (0.86 + grain * 0.24) * if streak { 0.84 } else { 1.0 }
        + if row == 0 { 0.07 } else { 0.0 }
        + if x == (joint + 1) % PIXELS { 0.05 } else { 0.0 }
        + weather
    }
  };
  match nail {
    true => Texel::rgb(0.24, 0.23, 0.22),
    false => {
      let warm = [1.0 + hue * 0.08, 1.0, 1.0 - hue * 0.1];
      let [r, g, b] =
        std::array::from_fn(|channel| base[channel] * warm[channel] * tone * shade);
      Texel::rgb(r, g, b)
    }
  }
}

struct Course {
  height: u32,
  joints: &'static [u32]
}

struct Masonry {
  courses: &'static [Course],
  stone: [f32; 3],
  mortar: [f32; 3],
  wear: f32,
  salt: u32
}

const GREYWACKE_BRICKS: Masonry = Masonry {
  courses: &[Course { height: 8, joints: &[6, 15] }, Course {
    height: 8,
    joints: &[2, 10]
  }],
  stone: [0.5, 0.5, 0.52],
  mortar: [0.27, 0.27, 0.27],
  wear: 0.3,
  salt: 0x5B1
};

fn masonry(
  &Masonry { courses, stone, mortar, wear, salt }: &Masonry,
  x: u32,
  y: u32
) -> Texel {
  let (index, top) = courses
    .iter()
    .enumerate()
    .scan(0, |top, (index, course)| {
      let start = *top;
      *top += course.height;
      Some((index, start))
    })
    .find(|&(index, top)| y < top + courses[index].height)
    .unwrap_or((0, 0));
  let Course { height, joints } = &courses[index];
  let row = y - top;
  let brick = joints.iter().filter(|&&joint| joint < x).count() % joints.len();
  let id = (index * 8 + brick) as i32;
  let jointed = |at: u32| joints.contains(&(at % PIXELS));
  let gapped = row == height - 1 || jointed(x);
  let grit = speck(salt ^ 0x11, x, y);
  let (left, right) = (jointed(x + PIXELS - 1), jointed(x + 1));
  let (high, low) = (row == 0, row == height - 2);
  let roll = |salt_of: u32| crate::noise::unit(salt ^ salt_of, id, 0, 0);
  let chipped = (high || low) && (left || right) && grit < wear * 1.2;
  let worn = (high || low || left || right) && grit < wear * 0.6;
  let colour = match gapped || chipped {
    true => {
      let shadow =
        if row == height - 1 { 0.82 } else { 1.0 } * if chipped { 0.9 } else { 1.0 };
      mortar.map(|channel| channel * shadow * (0.85 + grit * 0.3))
    }
    false => {
      let tone = 0.86 + roll(1) * 0.24;
      let mottle =
        0.8 + lattice(salt + id as u32 * 7, 4, x, y) * 0.24 + (grit - 0.5) * 0.1;
      let bevel = match () {
        () if high || left => 1.07,
        () if low || right => 0.86,
        () => 1.0
      };
      let pitted = if speck(salt ^ 0x12, x, y) < 0.06 || worn { 0.8 } else { 1.0 };
      let tint = STONE_TINTS[(roll(2) * STONE_TINTS.len() as f32) as usize]
        .map(|channel| 1.0 + (channel - 1.0) * 0.5);
      std::array::from_fn(|channel| {
        stone[channel] * tint[channel] * tone * mottle * bevel * pitted
      })
    }
  };
  Texel::rgb(colour[0], colour[1], colour[2])
}

fn soil(x: u32, y: u32) -> Texel { flecked(&DIRT, &EARTH, x, y) }

fn turf(palette: &Palette, x: u32, y: u32) -> Texel {
  let cloud = lattice(0x6A, 4, x, y) * 0.6 + lattice(0x6B, 8, x, y) * 0.4;
  let stagger = crate::noise::hash(0x6C, x as i32, 0, 0) % 3;
  let (clump, within) = ((y + stagger) / 3, (y + stagger) % 3);
  let stroke = match crate::noise::unit(0x6D, x as i32, clump as i32, 0) {
    blade if blade < 0.3 => 0.2 - within as f32 * 0.09,
    gap if gap > 0.86 => -0.2,
    _ => 0.0
  };
  ramp(palette, 0.22 + cloud * 0.5 + stroke)
}

fn fringed(over: &Palette, x: u32, y: u32) -> Texel {
  let hang = 2
    + crate::noise::hash(0x6E, x as i32, 0, 0) % 3
    + u32::from(crate::noise::unit(0x6F, x as i32 / 2, 0, 0) < 0.4) * 2;
  match y {
    y if y < hang => turf(over, x, y).scaled(1.0 - y as f32 * 0.04),
    y if y == hang => turf(over, x, y).scaled(0.62),
    y if y == hang + 1 => soil(x, y).scaled(0.7),
    _ => soil(x, y)
  }
}

fn cutout(art: &Art, palette: &Palette, x: u32, y: u32) -> Texel {
  drawn(art, palette, x, y).unwrap_or(Texel::rgb(0.0, 0.0, 0.0).alpha(0.0))
}

fn inked(art: &Art, [first, second, third]: [&Palette; 3], x: u32, y: u32) -> Texel {
  art[y as usize]
    .as_bytes()
    .get(x as usize)
    .and_then(|&mark| match mark {
      b'0'..=b'3' => Some(first[usize::from(mark - b'0')]),
      b'a'..=b'd' => Some(second[usize::from(mark - b'a')]),
      b'A'..=b'D' => Some(third[usize::from(mark - b'A')]),
      _ => None
    })
    .map_or(Texel::rgb(0.0, 0.0, 0.0).alpha(0.0), |[r, g, b]| Texel::rgb(r, g, b))
}

fn ore(x: u32, y: u32, [dark, base, light]: [[f32; 3]; 3], glow: f32) -> Texel {
  drawn(&SPOTS, &[dark, base, light, light], x, y)
    .map(|texel| texel.glowing(glow))
    .unwrap_or_else(|| smooth(&GREY, x, y))
}

const WAYSTONE: [f32; 3] = [0.62, 0.6, 0.55];

fn waystone(x: u32, y: u32) -> Texel { smooth(&shades(WAYSTONE, STONE_SPREAD), x, y) }

const RUNES: Art = [
  "................",
  ".....33.33......",
  ".....3...3......",
  ".....3333.......",
  "................",
  ".......333......",
  "......3..3......",
  ".....33..3......",
  "................",
  ".....3.3.3......",
  ".....33333......",
  ".......3........",
  "................",
  "......333.......",
  ".....3...3......",
  "......33.3......"
];

const HAMMER: Art = [
  "................",
  "................",
  "..3333..........",
  "..3333....2.....",
  "...11....222....",
  "...11...222.....",
  "...11..222......",
  "...11.222.......",
  "...11..2........",
  "...11...........",
  "................",
  "................",
  "................",
  "................",
  "................",
  "................"
];

const BOAT: Art = [
  "................",
  "................",
  "................",
  "................",
  "................",
  "................",
  "3..............3",
  "23............32",
  "1222222222222221",
  ".12222222222221.",
  "..111111111111..",
  "...1111111111...",
  "................",
  "................",
  "................",
  "................"
];

const BUCKET: Art = [
  "................",
  "................",
  "....11111111....",
  "...1........1...",
  "..1..........1..",
  "..233333333332..",
  "..311111111113..",
  "..322222222221..",
  "...3222222221...",
  "...3222222221...",
  "...3222222211...",
  "....32222211....",
  "....32222110....",
  "....22111110....",
  "................",
  "................"
];

const IRON: Palette =
  [[0.16, 0.16, 0.18], [0.3, 0.3, 0.33], [0.6, 0.61, 0.64], [0.84, 0.85, 0.87]];

fn bucket(fill: Option<Texel>, x: u32, y: u32) -> Texel {
  match (fill, y == 6 && shade_of(&BUCKET, x, y) == Some(1)) {
    (Some(fill), true) => fill,
    _ => cutout(&BUCKET, &IRON, x, y)
  }
}

const WOOD: Palette =
  [[0.32, 0.22, 0.12], [0.46, 0.33, 0.19], [0.6, 0.45, 0.27], [0.7, 0.55, 0.34]];
const RUNGS: Palette =
  [[0.53, 0.4, 0.24], [0.57, 0.43, 0.26], [0.6, 0.46, 0.28], [0.63, 0.48, 0.3]];
const TOOLS: Palette =
  [[0.2, 0.2, 0.2], [0.42, 0.28, 0.15], [0.6, 0.6, 0.62], [0.55, 0.55, 0.58]];

const SIGN: Art = [
  "................",
  "................",
  "3333333333333332",
  "3222222222222221",
  "3210211021102121",
  "3222222222222221",
  "3201121021012021",
  "3222222222222221",
  "2111111111111110",
  ".......32.......",
  ".......32.......",
  ".......21.......",
  ".......21.......",
  ".......21.......",
  ".......10.......",
  "................"
];

const FEATHER: Art = [
  "................",
  "............33..",
  "...........3332.",
  "..........33322.",
  ".........333221.",
  "........333221..",
  ".......333221...",
  "......333221....",
  ".....333221.....",
  "....333221......",
  "....33221.......",
  "...3221.........",
  "...0............",
  "..0.............",
  ".0..............",
  "................"
];

const CHARCOAL: Art = [
  "................",
  "................",
  "......2332......",
  "....23332221....",
  "...2332222111...",
  "..233222212110..",
  "..3322221221110.",
  ".23222212211110.",
  ".32221221111100.",
  ".22212211111010.",
  "..1121111110100.",
  "..11111101100...",
  "...111010100....",
  ".....00000......",
  "................",
  "................"
];

const EMBERLESS: Palette =
  [[0.06, 0.06, 0.07], [0.16, 0.15, 0.16], [0.28, 0.27, 0.28], [0.42, 0.41, 0.43]];

const QUILL: Palette =
  [[0.3, 0.3, 0.32], [0.7, 0.72, 0.76], [0.86, 0.88, 0.9], [0.98, 0.98, 1.0]];

const ROD: Art = [
  "................",
  ".............11.",
  "............1.3.",
  "...........1..3.",
  "..........1...3.",
  ".........1....3.",
  "........1.....3.",
  ".......1......3.",
  "......1.......3.",
  ".....1........3.",
  "....0.........3.",
  "...0.........222",
  "..0...........2.",
  ".0..............",
  "................",
  "................"
];

const TACKLE: Palette =
  [[0.3, 0.2, 0.11], [0.52, 0.37, 0.2], [0.8, 0.18, 0.12], [0.85, 0.85, 0.82]];

const FISH: Art = [
  "................",
  "................",
  "................",
  "......1111......",
  "....11111111...1",
  "...1122222211.11",
  "..112022222221.1",
  ".1122222222221.1",
  "..2223333333221.",
  "...22333333322.1",
  "....222222222..1",
  "......2222.....1",
  "................",
  "................",
  "................",
  "................"
];

const PUFFER: Art = [
  "................",
  ".......1........",
  "...1..1111..1...",
  "....11111111....",
  "...1111111111...",
  "..112011111111..",
  "1.11111111111.11",
  "..222222222221.1",
  "..2333333333221.",
  "1.23333333332..1",
  "...233333332....",
  "....2222222.....",
  "...2...2...2....",
  "................",
  "................",
  "................"
];

const PICKAXE: Art = [
  "................",
  ".....3333332....",
  "...23322222221..",
  "..231.....12221.",
  "..21......c1221.",
  ".21......cb.121.",
  ".1......cb...221",
  "........cb...121",
  ".......cb.....21",
  "......cb......11",
  ".....cb.......0.",
  "....cb..........",
  "...cb...........",
  "..cb............",
  ".ba.............",
  "................"
];

const AXE: Art = [
  "................",
  "....33..........",
  "...3322.........",
  "..332221........",
  "..3322221.......",
  "..33222221cb....",
  "..332222cb21....",
  "..33222cb21.....",
  "...322cb1.......",
  "....2cb.........",
  ".....cb.........",
  "....cb..........",
  "...cb...........",
  "..cb............",
  ".ba.............",
  "................"
];

const SHOVEL: Art = [
  "................",
  "...........33...",
  "..........3332..",
  ".........332221.",
  "........3322221.",
  "........3222211.",
  ".........22211..",
  "........cb111...",
  ".......cb.......",
  "......cb........",
  ".....cb.........",
  "....cb..........",
  "...cb...........",
  "..cb............",
  ".ba.............",
  "................"
];

const SWORD: Art = [
  "................",
  ".............33.",
  "............3332",
  "...........33321",
  "..........33321.",
  ".........33321..",
  "........33321...",
  "..A....33321....",
  "..BC..33321.....",
  "...BC33321......",
  "....BC321.......",
  "....cBC1........",
  "...cb.BC........",
  "..cb...A........",
  ".BA.............",
  "................"
];

const HAT: Art = [
  "................",
  "................",
  "................",
  "......3332......",
  ".....333221.....",
  ".....3322211....",
  "....33322211....",
  "....33222211....",
  "....aaaaaaaa....",
  "....dccccccb....",
  "..3332222222210.",
  ".332222222222210",
  "..0011111111100.",
  "................",
  "................",
  "................"
];

const AMULET: Art = [
  "................",
  "....ca....ca....",
  "...c..b..c..a...",
  "...b...cb...a...",
  "....b......a....",
  ".....b....a.....",
  "......b..a......",
  ".......dc.......",
  "......dcbb......",
  ".....d3321a.....",
  ".....c3211a.....",
  ".....c2210a.....",
  "......b10a......",
  ".......aa.......",
  "................",
  "................"
];

const MAIL: Art = [
  "................",
  "................",
  "...2323..1313...",
  "..23231..31313..",
  ".23231313131313.",
  ".32312121212121.",
  ".231.121212.212.",
  "....12121212....",
  "....21212121....",
  "....12121212....",
  "....21212120....",
  "....12121201....",
  "....10101010....",
  "................",
  "................",
  "................"
];

const BELT: Art = [
  "................",
  "................",
  "................",
  "................",
  "................",
  "................",
  "cccccbAAAAccccc.",
  "dddddcDCCBdddddc",
  "ccccccC..Bcccccb",
  "bbbbbbCBBAbbbbba",
  "aaaaabAAAAaaaaa.",
  "................",
  "................",
  "................",
  "................",
  "................"
];

const BOOTS: Art = [
  "................",
  "................",
  "....dddcb.......",
  "....ccccb.......",
  "....cccbb.......",
  "....cccbb.......",
  "....cccbb.......",
  "....cccbb.......",
  "....ccccbb......",
  "....cccccbbb....",
  "....ccccccbbba..",
  "...dccccccbbbba.",
  "...ccccccccbbba.",
  "...aaaaaaaaaaaa.",
  "................",
  "................"
];

const RING: Art = [
  "................",
  "................",
  "................",
  "......D33.......",
  ".....D3221......",
  ".....CD210......",
  "....DC..CA......",
  "...DC....CA.....",
  "...C......B.....",
  "...C......B.....",
  "...CB....BA.....",
  "....BB..BA......",
  ".....BAAA.......",
  "................",
  "................",
  "................"
];

const HAFT: Palette =
  [[0.3, 0.2, 0.11], [0.43, 0.3, 0.17], [0.56, 0.41, 0.24], [0.68, 0.52, 0.32]];
const GILT: Palette =
  [[0.55, 0.36, 0.06], [0.82, 0.62, 0.14], [0.96, 0.82, 0.3], [1.0, 0.95, 0.62]];
const LEATHER: Palette =
  [[0.22, 0.12, 0.06], [0.36, 0.21, 0.11], [0.5, 0.31, 0.17], [0.64, 0.42, 0.25]];
const CRIMSON: Palette =
  [[0.35, 0.06, 0.06], [0.6, 0.12, 0.1], [0.82, 0.22, 0.18], [0.98, 0.55, 0.45]];
const SAPPHIRE: Palette =
  [[0.06, 0.12, 0.35], [0.12, 0.28, 0.65], [0.25, 0.5, 0.9], [0.65, 0.85, 1.0]];
const TALON: Palette =
  [[0.25, 0.22, 0.16], [0.55, 0.5, 0.38], [0.78, 0.74, 0.6], [0.93, 0.91, 0.8]];
const SCALES: Palette =
  [[0.12, 0.2, 0.08], [0.22, 0.34, 0.14], [0.36, 0.5, 0.22], [0.5, 0.64, 0.3]];

pub fn forged(tool: Block, material: Material, x: u32, y: u32) -> Texel {
  let art = match tool {
    Block::Pickaxe => &PICKAXE,
    Block::Axe => &AXE,
    Block::Shovel => &SHOVEL,
    _ => &SWORD
  };
  let texel = inked(art, [&material.palette(), &HAFT, &GILT], x, y);
  match material.glows() && shade_of(art, x, y).is_some() {
    true => texel.glowing(0.35),
    false => texel
  }
}

pub fn forged_slot(tool: Block, material: Material) -> u32 {
  let kind = Block::TOOLS.iter().position(|&known| known == tool).unwrap_or(0) as u32;
  FORGED + kind * MATERIALS + material.index() as u32
}

pub fn painted(slot: u32, x: u32, y: u32) -> Texel {
  match Tile::ALL.get(slot as usize) {
    Some(&tile) => paint(tile, x, y),
    None => {
      let index = slot.saturating_sub(FORGED);
      forged(
        Block::TOOLS[(index / MATERIALS) as usize % Block::TOOLS.len()],
        Material::ALL[(index % MATERIALS) as usize],
        x,
        y
      )
    }
  }
}

fn fish(tile: Tile) -> Palette {
  match tile {
    Tile::Salmon => {
      [[0.1, 0.06, 0.06], [0.55, 0.22, 0.2], [0.85, 0.42, 0.34], [0.95, 0.72, 0.6]]
    }
    Tile::TropicalFish => {
      [[0.05, 0.05, 0.08], [0.95, 0.45, 0.08], [0.98, 0.72, 0.15], [0.98, 0.95, 0.9]]
    }
    Tile::Pufferfish => {
      [[0.08, 0.06, 0.03], [0.72, 0.6, 0.18], [0.9, 0.8, 0.3], [0.97, 0.94, 0.75]]
    }
    _ => [[0.06, 0.06, 0.05], [0.42, 0.38, 0.28], [0.62, 0.58, 0.44], [0.86, 0.84, 0.74]]
  }
}

fn wool(x: u32, y: u32) -> Texel {
  let curl = lattice(0x3A, 8, x, y) * 0.6 + lattice(0x3B, 16, x, y) * 0.4;
  let tuft = (x + y / 3 * 2) % 4 == 0 && y % 3 == 0;
  ramp(&WHITE, 0.25 + curl * 0.6 - if tuft { 0.25 } else { 0.0 })
}

pub fn timber(sign: Block) -> Palette {
  match sign {
    Block::BirchSign => {
      [[0.42, 0.37, 0.26], [0.66, 0.6, 0.45], [0.8, 0.74, 0.57], [0.88, 0.83, 0.67]]
    }
    Block::SpruceSign => {
      [[0.14, 0.09, 0.05], [0.27, 0.18, 0.1], [0.37, 0.25, 0.14], [0.45, 0.31, 0.18]]
    }
    Block::PalmSign => {
      [[0.36, 0.23, 0.12], [0.6, 0.41, 0.22], [0.74, 0.54, 0.31], [0.82, 0.63, 0.4]]
    }
    _ => WOOD
  }
}

fn flame(x: u32, y: u32) -> Texel {
  let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
  let heat = (1.0 - (dx * dx + dy * dy).sqrt() / 9.0).clamp(0.0, 1.0);
  Texel::rgb(1.0, 0.55 + heat * 0.3, 0.15 + heat * 0.25).glowing(0.7 + heat * 0.5)
}

fn rune() -> Texel { Texel::rgb(0.85, 0.97, 1.0).glowing(0.9) }

const ANCIENT: [f32; 3] = [0.37, 0.4, 0.41];

const ANCIENT_BRICKS: Masonry = Masonry {
  courses: &[Course { height: 8, joints: &[15] }, Course { height: 8, joints: &[7] }],
  stone: ANCIENT,
  mortar: [0.12, 0.13, 0.14],
  wear: 0.45,
  salt: 0xA4C
};

fn ancient(x: u32, y: u32) -> Texel { masonry(&ANCIENT_BRICKS, x, y) }

const MOSS: Palette =
  [[0.1, 0.2, 0.08], [0.17, 0.3, 0.11], [0.24, 0.4, 0.14], [0.33, 0.5, 0.2]];

fn mossy(x: u32, y: u32) -> Texel {
  let patch = lattice(0x60, 4, x, y) * 0.7 + lattice(0x61, 8, x, y) * 0.3;
  let hanging = 1.0 - y as f32 / 16.0;
  match patch * 0.8 + hanging * 0.45 > 0.78 {
    true => ramp(&MOSS, lattice(0x62, 16, x, y)),
    false => ancient(x, y)
  }
}

const BONES: Art = [
  "....0000........",
  "...033330....00.",
  "..03302330..0330",
  "..03203320.03320",
  "..0333332003220.",
  "...032220032210.",
  "00..0000032210..",
  "0330...032210...",
  "0332220322110.00",
  ".033333332101033",
  "..0222222110.032",
  "...0000000...022",
  ".00.....00000000",
  "0330...033333330",
  "03220.0322222220",
  ".000...000000000"
];

const BALLOON: Art = [
  "......dd33......",
  "....3ddd333c....",
  "...33ddd322cc...",
  "...33ddd222cc...",
  "..333ddc222bbb..",
  "..333ccc221bbb..",
  "..332ccc211bba..",
  "...22ccb111ba...",
  "....2cbb111a....",
  ".....bbb100.....",
  "......bb00......",
  ".......a0.......",
  "......A..A......",
  ".....DDDDDD.....",
  ".....CCCBBB.....",
  ".....BBBBAA....."
];

const IVORY: Palette =
  [[0.18, 0.15, 0.12], [0.58, 0.54, 0.44], [0.78, 0.74, 0.62], [0.92, 0.9, 0.82]];

const WEB: Art = [
  "3......3.......3",
  ".3.....3......3.",
  "..3....3.....3..",
  "...33333333333..",
  "...3...3...3.3..",
  "....3..3..3.3...",
  "3333333333333333",
  ".....3.3.3......",
  "....3..3..3.....",
  "...3...3...3....",
  "..33333333333...",
  "..3....3....3...",
  ".3.....3.....3..",
  "3......3......3.",
  "3......3.......3",
  "3......3.......3"
];

const DUST: Art = [
  "................",
  "................",
  "................",
  ".......3........",
  "....3......3....",
  "........2.......",
  "..3...........3.",
  "......2.3.......",
  "...........2....",
  ".......33.......",
  ".....322223.....",
  "....32222223....",
  "...3221222123...",
  "..321111111123..",
  "..011111111110..",
  "................"
];

const CLAW: Art = [
  "................",
  "................",
  "..........233...",
  "........23332...",
  ".......2332211..",
  "......2321110...",
  ".....232110.....",
  ".....33210......",
  "....23210.......",
  "....3321........",
  "....3210........",
  "...dc21.........",
  "...cbba.........",
  "...bbaa.........",
  "....aa..........",
  "................"
];

const CANDLE: Art = [
  "................",
  ".......3........",
  "......323.......",
  "......323.......",
  ".......0........",
  "......3221......",
  "......3321......",
  "......3221......",
  "......3221......",
  "......3211......",
  "......3221......",
  ".....332211.....",
  "...dcccccbba....",
  "....abbbbaa.....",
  "................",
  "................"
];

fn lumen(x: u32, y: u32) -> Texel {
  let facet = ((x + y * 2) / 5 + (x * 3 + 16 - y) / 7) % 4;
  let vein = (x * 7 + y * 3) % 11 == 0 || (x + 16 - y) % 9 == 0;
  let base =
    [[0.05, 0.42, 0.36], [0.12, 0.62, 0.52], [0.25, 0.82, 0.68], [0.6, 1.0, 0.88]];
  match vein {
    true => Texel::rgb(0.04, 0.16, 0.14).glowing(0.3),
    false => {
      let [r, g, b] = base[facet as usize];
      Texel::rgb(r, g, b).glowing(0.55 + facet as f32 * 0.12)
    }
  }
}

fn terracotta(x: u32, y: u32) -> Texel {
  let band = y == 3 || y == 12;
  let meander = (6..10).contains(&y) && ((x / 2 + y) % 4 == 0 || y == 6 && x % 4 < 3);
  match (band, meander) {
    (true, _) => Texel::rgb(0.2, 0.1, 0.06),
    (_, true) => Texel::rgb(0.24, 0.12, 0.07),
    _ => ramp(
      &[[0.45, 0.22, 0.12], [0.58, 0.3, 0.16], [0.68, 0.38, 0.2], [0.76, 0.46, 0.26]],
      0.35 + lattice(0x7C, 4, x, y) * 0.5
    )
  }
}

fn spell(glyph: crate::glyph::Glyph, palette: Palette, x: u32, y: u32) -> Texel {
  glyph.shade(x, y).map_or(Texel::rgb(0.0, 0.0, 0.0).alpha(0.0), |shade| {
    let lit = 1.08 - (x + y) as f32 / 30.0 * 0.24;
    let [r, g, b] = palette[shade];
    Texel::rgb(r * lit, g * lit, b * lit).glowing(if shade == 3 { 0.5 } else { 0.0 })
  })
}

fn brushed(base: [f32; 3], x: u32, y: u32) -> Texel {
  let streak = lattice(0x3A1, 2, x, y * 4) * 0.6 + lattice(0x3A2, 4, x / 2, y * 8) * 0.4;
  let edge = x == 0 || y == 0 || x == PIXELS - 1 || y == PIXELS - 1;
  let rivet = (x == 2 || x == 13) && (y == 2 || y == 13);
  let glint = (x == 1 || x == 12) && (y == 1 || y == 12);
  let [r, g, b] = base;
  Texel::rgb(r, g, b).scaled(match (edge, rivet, glint) {
    (true, ..) => 0.62,
    (_, true, _) => 0.74,
    (.., true) => 1.3,
    _ => 0.86 + streak * 0.28
  })
}

const COPPER: [f32; 3] = [0.74, 0.43, 0.24];
const STEEL: [f32; 3] = [0.58, 0.6, 0.63];

fn pipe_glass(x: u32, y: u32) -> Texel {
  let shine = match x {
    6 | 2 if y % 5 != 3 => Some(0.95),
    13 => Some(0.5),
    _ if (x * 5 + y * 3) % 23 == 0 => Some(0.7),
    _ => None
  };
  shine.map_or(Texel::rgb(0.0, 0.0, 0.0).alpha(0.0), |light| {
    Texel::rgb(0.82 * light + 0.12, 0.92 * light + 0.06, light).alpha(0.9)
  })
}

fn casing(x: u32, y: u32) -> Texel {
  let slot = (10..13).contains(&y) && (3..13).contains(&x) && y != 11;
  let light = y == 4 && (x == 11 || x == 12);
  match (slot, light) {
    (true, _) => Texel::rgb(0.08, 0.08, 0.09),
    (_, true) => Texel::rgb(0.3, 0.95, 0.4).glowing(1.2),
    _ => brushed([0.42, 0.45, 0.5], x, y)
  }
}

fn grille(x: u32, y: u32) -> Texel {
  let frame = x < 2 || y < 2 || x > PIXELS - 3 || y > PIXELS - 3;
  match (frame, y % 3) {
    (true, _) => brushed([0.32, 0.33, 0.36], x, y),
    (_, 0) => Texel::rgb(0.05, 0.05, 0.06),
    (_, 1) => Texel::rgb(0.55, 0.56, 0.6),
    _ => Texel::rgb(0.3, 0.31, 0.34)
  }
}

fn solar(x: u32, y: u32) -> Texel {
  let line = x % 4 == 0 || y % 4 == 0;
  let glint = (x + y) % 9 == 0 || (x + 16 - y) % 13 == 0;
  match (line, glint) {
    (true, _) => Texel::rgb(0.66, 0.68, 0.72),
    (_, true) => Texel::rgb(0.3, 0.42, 0.78),
    _ => Texel::rgb(0.08, 0.13, 0.36).scaled(0.85 + lattice(0x501, 4, x, y) * 0.3)
  }
}

fn cable(x: u32, y: u32) -> Texel {
  let shine = y % 8 == 2 || (y % 8 == 3 && x % 3 != 0);
  let band = x % 8 == 7;
  match (band, shine) {
    (true, _) => Texel::rgb(0.74, 0.43, 0.24).scaled(0.8),
    (_, true) => Texel::rgb(0.24, 0.24, 0.26),
    _ => Texel::rgb(0.07, 0.07, 0.08)
  }
}

fn oil(x: u32, y: u32) -> Texel {
  let swirl = lattice(0x011, 4, x, y) * 0.6 + lattice(0x012, 8, x, y) * 0.4;
  let (dx, dy) = (x as f32 - 10.5, y as f32 - 5.5);
  let bubble = ((dx * dx + dy * dy).sqrt() - 2.2).abs() < 0.6;
  let sheen = |phase: f32| (swirl * 6.0 + phase).fract();
  match bubble {
    true => Texel::rgb(0.32, 0.3, 0.34),
    false => Texel::rgb(
      0.03 + sheen(0.0) * 0.1,
      0.03 + sheen(0.33) * 0.07,
      0.05 + sheen(0.66) * 0.12
    )
  }
}

fn tar(x: u32, y: u32) -> Texel {
  let stain = lattice(0x013, 4, x, y) + (16 - y.min(16)) as f32 / 40.0;
  match stain > 0.72 {
    true => oil(x, y),
    false => soil(x, y).scaled(0.45 + (0.72 - stain) * 0.6)
  }
}

fn cells(x: u32, y: u32) -> Texel {
  let window = (4..12).contains(&x) && (3..13).contains(&y);
  let bar = window && x % 2 == 1 && y > 4;
  let charged = bar && y > 6;
  match (window, bar, charged) {
    (_, _, true) => Texel::rgb(0.32, 0.95, 0.36).glowing(1.0),
    (_, true, _) => Texel::rgb(0.12, 0.3, 0.14),
    (true, ..) => Texel::rgb(0.05, 0.06, 0.06),
    _ if y < 2 => brushed(COPPER, x, y),
    _ => brushed([0.2, 0.22, 0.26], x, y)
  }
}

fn coil(x: u32, y: u32) -> Texel {
  let run = y % 4 == 1 && (2..14).contains(&x);
  let turn =
    (y % 8 == 2 || y % 8 == 3) && x == 13 || (y % 8 == 6 || y % 8 == 7) && x == 2;
  let frame = x == 0 || y == 0 || x == PIXELS - 1 || y == PIXELS - 1;
  match (frame, run || turn) {
    (true, _) => Texel::rgb(0.3, 0.31, 0.34),
    (_, true) => Texel::rgb(1.0, 0.45 + (x % 3) as f32 * 0.08, 0.1).glowing(1.4),
    _ => Texel::rgb(0.12, 0.06, 0.04)
  }
}

fn bulb(lit: bool, x: u32, y: u32) -> Texel {
  let filament =
    (x == 7 || x == 8) && (5..11).contains(&y) || y == 5 && (5..11).contains(&x);
  let rim = x == 0 || x == PIXELS - 1;
  let glass = 0.82 + lattice(0x0B1, 2, x, y) * 0.12;
  match (lit, filament, rim) {
    (true, true, _) => Texel::rgb(1.0, 0.96, 0.8).glowing(4.0),
    (true, _, _) => Texel::rgb(1.0, 0.86, 0.56).scaled(glass).glowing(2.4),
    (false, true, _) => Texel::rgb(0.45, 0.28, 0.12),
    (false, _, true) => Texel::rgb(0.66, 0.66, 0.6),
    _ => Texel::rgb(0.86, 0.85, 0.76).scaled(glass)
  }
}

fn rubber(x: u32, y: u32) -> Texel {
  let rib = y % 4 == 0;
  let wear = lattice(0x0D1, 4, x, y) * 0.25;
  match rib {
    true => Texel::rgb(0.2, 0.2, 0.21),
    false => Texel::rgb(0.09, 0.09, 0.1).scaled(0.9 + wear)
  }
}

fn hazard(x: u32, y: u32) -> Texel {
  match (x + y) / 4 % 2 == 0 {
    true => Texel::rgb(0.92, 0.72, 0.1).scaled(0.9 + lattice(0x0C1, 4, x, y) * 0.15),
    false => Texel::rgb(0.08, 0.08, 0.09)
  }
}

const INGOT: Art = [
  "................",
  "................",
  "................",
  "................",
  "................",
  ".......33333....",
  ".....333222333..",
  "...3332222222221",
  "..33222222222211",
  ".322222222221110",
  ".222222222211100",
  ".111122222111000",
  "..0011111110000.",
  "....00011100....",
  ".......000......",
  "................"
];

const GEM: Art = [
  "................",
  "................",
  "................",
  "....33333333....",
  "...3332222223...",
  "..333222222221..",
  ".33322222222211.",
  ".22222222222110.",
  "..222222222110..",
  "...1222222110...",
  "....12222110....",
  ".....122110.....",
  "......1210......",
  ".......10.......",
  "................",
  "................"
];

const PEARL: Art = [
  "................",
  "................",
  "................",
  "......2222......",
  "....22333222....",
  "...2233322222...",
  "...2333222221...",
  "..222322222211..",
  "..222222222211..",
  "..222222222111..",
  "...2222221111...",
  "...1222211110...",
  "....11111100....",
  "......0000......",
  "................",
  "................"
];

const TABLET: Art = [
  "................",
  "................",
  "................",
  "..333333333333..",
  "..322222222221..",
  "..321111111121..",
  "..321111111121..",
  "..321111111121..",
  "..321111111121..",
  "..321111111121..",
  "..321111111121..",
  "..322222222221..",
  "..111111111110..",
  "...0000000000...",
  "................",
  "................"
];

const BOUND_TABLET: Art = [
  "................",
  "................",
  "................",
  "..AAAAAAAAAAAA..",
  "..A2222222222A..",
  "..A21111d11121..",
  "..A2111dd11121..",
  "..A211d1d11121..",
  "..A21111d11121..",
  "..A2111ddd1121..",
  "..A21111111121..",
  "..A2222222222A..",
  "..BAAAAAAAAAAB..",
  "...0000000000...",
  "................",
  "................"
];

const KNIFE: Art = [
  "................",
  "..............3.",
  ".............32.",
  "............321.",
  "...........321..",
  "..........321...",
  ".........321....",
  "........321.....",
  ".......321......",
  "..CC..321.......",
  "..DCC321........",
  "...DCC1.........",
  "...bDCC.........",
  "..bbaDCC........",
  ".bba..DC........",
  ".aa............."
];

const SIGIL: Art = [
  "................",
  ".....AAAAAA.....",
  "...AABBBBBBAA...",
  "..ABBaaaaaaBBA..",
  "..ABaaaa3aaaBA..",
  ".ABaaaa332aaaBA.",
  ".ABaaa33322aaBA.",
  ".ABaa3332221aBA.",
  ".ABaa3322211aBA.",
  ".ABaa3222111aBA.",
  ".ABaaa22111aaBA.",
  "..ABaaa110aaBA..",
  "..ABBaaaaaaBBA..",
  "...AABBBBBBAA...",
  ".....AAAAAA.....",
  "................"
];

const GLIMSTEEL: Palette =
  [[0.1, 0.24, 0.5], [0.22, 0.46, 0.82], [0.42, 0.7, 0.98], [0.78, 0.92, 1.0]];
const VERDANTINE: Palette =
  [[0.08, 0.3, 0.08], [0.2, 0.55, 0.16], [0.4, 0.82, 0.3], [0.78, 1.0, 0.62]];
const HEXBRASS: Palette =
  [[0.18, 0.08, 0.32], [0.34, 0.18, 0.56], [0.52, 0.34, 0.78], [0.8, 0.66, 0.98]];
const MITHRIL: Palette =
  [[0.3, 0.36, 0.5], [0.52, 0.6, 0.78], [0.74, 0.8, 0.95], [0.94, 0.97, 1.0]];
const STARHEART: Palette =
  [[0.1, 0.4, 0.62], [0.28, 0.72, 0.95], [0.55, 0.9, 1.0], [0.88, 1.0, 1.0]];
const TIDEPEARL: Palette =
  [[0.06, 0.32, 0.42], [0.16, 0.56, 0.62], [0.36, 0.82, 0.84], [0.8, 1.0, 0.98]];
const BLOOD: Palette =
  [[0.22, 0.01, 0.02], [0.45, 0.03, 0.05], [0.7, 0.08, 0.1], [1.0, 0.45, 0.4]];
const SLATE_GREY: Palette =
  [[0.14, 0.15, 0.17], [0.3, 0.32, 0.35], [0.42, 0.44, 0.47], [0.56, 0.58, 0.6]];
const WATER_DROP: Palette =
  [[0.05, 0.2, 0.5], [0.12, 0.38, 0.8], [0.3, 0.6, 0.95], [0.75, 0.9, 1.0]];
const MOONSTONE: [f32; 3] = [0.84, 0.84, 0.78];

fn moonstone(x: u32, y: u32) -> Texel { smooth(&shades(MOONSTONE, STONE_SPREAD), x, y) }

fn inlaid(x: u32, y: u32, rune: Texel, under: Texel) -> Texel {
  let (dx, dy) = (x as i32 - 8, y as i32 - 8);
  let ring = dx.abs().max(dy.abs()) == 5;
  let cross =
    (dx == 0 || dy == 0 || dx == dy || dx == -dy) && dx.abs() < 4 && dy.abs() < 4;
  match ring || cross {
    true => rune,
    false => under
  }
}

fn glinting(art: &Art, palette: &Palette, glow: f32, x: u32, y: u32) -> Texel {
  match cutout(art, palette, x, y) {
    texel if shade_of(art, x, y) == Some(3) => texel.glowing(glow),
    texel => texel.glowing(glow * 0.3)
  }
}

const BAG: Art = [
  "................",
  ".....aaaaaa.....",
  "....a......a....",
  "....a......a....",
  "..333333333333..",
  ".32222222222221.",
  ".32222222222221.",
  ".3BBBBBBBBBBBB1.",
  ".322222BB222221.",
  ".322222BB222221.",
  ".32222222222221.",
  ".32222222222221.",
  ".31111111111111.",
  ".21111111111110.",
  "..000000000000..",
  "................"
];

const RIFT: Art = [
  "................",
  "................",
  ".....333333.....",
  "...3322222233...",
  "..322aaaaaa221..",
  "..32aabbbbaa21..",
  ".32aabccccbaa21.",
  ".32abccddccba21.",
  ".32abccddccba21.",
  ".32aabccccbaa21.",
  "..32aabbbbaa21..",
  "..122aaaaaa221..",
  "...1122222211...",
  ".....111111.....",
  "................",
  "................"
];

const PLATINUM: Palette =
  [[0.42, 0.44, 0.48], [0.66, 0.69, 0.72], [0.83, 0.85, 0.87], [0.97, 0.98, 1.0]];
const CANVAS: Palette =
  [[0.42, 0.36, 0.24], [0.6, 0.53, 0.38], [0.74, 0.67, 0.5], [0.86, 0.8, 0.64]];
const OILCLOTH: Palette =
  [[0.1, 0.18, 0.12], [0.18, 0.3, 0.2], [0.27, 0.42, 0.28], [0.38, 0.54, 0.38]];
const STRAP: Palette =
  [[0.16, 0.12, 0.08], [0.26, 0.18, 0.1], [0.34, 0.24, 0.14], [0.44, 0.32, 0.2]];
const RIVET: Palette =
  [[0.3, 0.3, 0.32], [0.48, 0.48, 0.5], [0.62, 0.62, 0.65], [0.8, 0.8, 0.82]];
const VOID: Palette =
  [[0.32, 0.14, 0.55], [0.2, 0.06, 0.38], [0.08, 0.02, 0.16], [0.75, 0.55, 1.0]];

fn rift(rim: &Palette, x: u32, y: u32) -> Texel {
  match RIFT[y as usize].as_bytes()[x as usize] {
    b'd' => Texel::rgb(0.75, 0.55, 1.0).glowing(1.4),
    b'a' => inked(&RIFT, [rim, &VOID, &VOID], x, y).glowing(0.5),
    _ => inked(&RIFT, [rim, &VOID, &VOID], x, y)
  }
}

fn crusher(x: u32, y: u32) -> Texel {
  let tooth = (x % 4).min(4 - x % 4) as i32 >= (y as i32 - 7).abs() / 2;
  match (y < 3 || y > 12, tooth) {
    (true, _) => brushed(STEEL, x, y),
    (false, true) => Texel::rgb(0.62, 0.62, 0.66).scaled(0.9 + (x % 4) as f32 * 0.05),
    (false, false) => Texel::rgb(0.06, 0.06, 0.07)
  }
}

fn model_icon(block: Block, x: u32, y: u32) -> Texel {
  model::icon(block, x, y).map_or(Texel::rgb(0.0, 0.0, 0.0).alpha(0.0), |bit| {
    let Texel { color: [r, g, b, _], glow } = paint(bit.tile, x, y);
    let [tint_r, tint_g, tint_b] = bit.color;
    Texel { color: [r * tint_r, g * tint_g, b * tint_b, 1.0], glow }
  })
}

fn clear() -> Texel { Texel::rgb(0.0, 0.0, 0.0).alpha(0.0) }

fn lit(x: u32, y: u32) -> f32 { 1.12 - (x + y) as f32 / 30.0 * 0.32 }

fn tone(palette: &Palette, along: f32, x: u32, y: u32) -> Texel {
  ramp(palette, along).scaled(lit(x, y))
}

const KELP: Palette =
  [[0.12, 0.2, 0.06], [0.28, 0.38, 0.1], [0.44, 0.52, 0.14], [0.66, 0.7, 0.26]];
const GLOWKELP: Palette =
  [[0.04, 0.22, 0.24], [0.08, 0.42, 0.42], [0.2, 0.68, 0.62], [0.6, 1.0, 0.9]];
const SEAGRASS: Palette =
  [[0.08, 0.24, 0.1], [0.16, 0.42, 0.16], [0.28, 0.58, 0.22], [0.5, 0.76, 0.36]];
const FAN: Palette =
  [[0.42, 0.06, 0.12], [0.72, 0.16, 0.24], [0.92, 0.36, 0.4], [1.0, 0.66, 0.62]];
const TUBE: Palette =
  [[0.2, 0.08, 0.36], [0.42, 0.2, 0.66], [0.62, 0.38, 0.86], [0.86, 0.68, 1.0]];
const ANEMONE: Palette =
  [[0.5, 0.16, 0.08], [0.88, 0.38, 0.16], [1.0, 0.62, 0.32], [1.0, 0.88, 0.7]];
const POLYP: Palette =
  [[0.04, 0.3, 0.4], [0.1, 0.62, 0.72], [0.36, 0.9, 0.95], [0.82, 1.0, 1.0]];
const WORM: Palette =
  [[0.5, 0.48, 0.42], [0.74, 0.72, 0.66], [0.9, 0.88, 0.82], [1.0, 0.98, 0.94]];
const PLUME: Palette =
  [[0.4, 0.02, 0.04], [0.72, 0.06, 0.08], [0.95, 0.2, 0.16], [1.0, 0.5, 0.42]];
const SHELL: Palette =
  [[0.26, 0.24, 0.34], [0.46, 0.44, 0.56], [0.64, 0.62, 0.72], [0.84, 0.82, 0.9]];
const MANTLE: Palette =
  [[0.08, 0.14, 0.42], [0.14, 0.34, 0.7], [0.3, 0.6, 0.9], [0.6, 0.88, 1.0]];
const BRAIN: Palette =
  [[0.42, 0.26, 0.2], [0.66, 0.46, 0.34], [0.82, 0.64, 0.48], [0.94, 0.82, 0.66]];
const SPONGE: Palette =
  [[0.5, 0.32, 0.04], [0.8, 0.56, 0.1], [0.94, 0.74, 0.2], [1.0, 0.9, 0.44]];
const ABYSS: Palette =
  [[0.04, 0.05, 0.09], [0.08, 0.1, 0.16], [0.13, 0.15, 0.23], [0.2, 0.23, 0.32]];
const VENT: Palette =
  [[0.07, 0.05, 0.04], [0.14, 0.1, 0.08], [0.22, 0.16, 0.12], [0.32, 0.24, 0.18]];
const SILT: Palette =
  [[0.16, 0.15, 0.13], [0.24, 0.23, 0.2], [0.32, 0.3, 0.26], [0.4, 0.38, 0.33]];
const ALLOY: Palette =
  [[0.12, 0.16, 0.18], [0.22, 0.28, 0.3], [0.34, 0.42, 0.44], [0.52, 0.62, 0.64]];
const AETHER: [f32; 3] = [0.35, 0.95, 1.0];
const REEF: Palette =
  [[0.5, 0.46, 0.38], [0.68, 0.64, 0.54], [0.82, 0.78, 0.68], [0.94, 0.92, 0.84]];
const BIOLUME: Palette =
  [[0.06, 0.32, 0.28], [0.14, 0.6, 0.5], [0.34, 0.86, 0.68], [0.76, 1.0, 0.9]];
const PEARLY: Palette =
  [[0.6, 0.58, 0.64], [0.8, 0.78, 0.84], [0.92, 0.9, 0.95], [1.0, 1.0, 1.0]];
const RESONANT: Palette =
  [[0.24, 0.08, 0.42], [0.46, 0.2, 0.74], [0.68, 0.44, 0.96], [0.92, 0.8, 1.0]];
const SUBMARINE: Palette =
  [[0.44, 0.3, 0.04], [0.78, 0.56, 0.08], [0.96, 0.76, 0.16], [1.0, 0.92, 0.5]];

fn frond(palette: &Palette, glow: f32, x: u32, y: u32) -> Texel {
  let (fx, fy) = (x as f32, y as f32);
  let middle = 7.5 + (fy * 0.55).sin() * 2.2;
  let width = 1.6 + (fy * 0.9 + 1.0).sin().abs() * 1.0;
  let bladder = [(4.0, 1.0), (10.0, -1.0)]
    .into_iter()
    .any(|(row, side)| Vec2::new(fx - (middle + side * 2.6), fy - row).length() < 1.4);
  let offset = fx - middle;
  match (bladder, offset.abs() <= width) {
    (true, _) => tone(palette, 0.95, x, y).glowing(glow * 2.0),
    (_, true) => {
      tone(palette, 0.35 + (1.0 - offset.abs() / width) * 0.5, x, y).glowing(glow)
    }
    _ => clear()
  }
}

fn blades(x: u32, y: u32) -> Texel {
  let (fx, fy) = (x as f32, y as f32);
  [(3.0, 3.0, 0.6), (6.0, 1.0, -0.4), (9.0, 4.0, 0.5), (12.0, 2.0, -0.7)]
    .into_iter()
    .find(|&(root, top, lean)| {
      fy >= top && (fx - (root + (15.0 - fy) * lean * 0.25)).abs() < 0.8
    })
    .map_or(clear(), |(_, top, _)| tone(&SEAGRASS, 0.3 + (fy - top) / 20.0, x, y))
}

fn fan(x: u32, y: u32) -> Texel {
  let offset = Vec2::new(x as f32 - 7.5, 14.5 - y as f32);
  let (reach, angle) = (offset.length(), offset.y.atan2(offset.x));
  let spoke = ((angle * 7.0).sin().abs() < 0.28) || reach.fract() < 0.22 && reach > 3.0;
  match (reach < 12.5 && offset.y > 0.0, spoke || reach < 2.0) {
    (true, true) => tone(&FAN, 0.75 - reach / 30.0, x, y),
    (true, false) if speck(0xFA4, x, y) < 0.35 => tone(&FAN, 0.3, x, y),
    _ => clear()
  }
}

fn tubes(x: u32, y: u32) -> Texel {
  [(2u32, 6u32), (6, 2), (10, 5), (13, 8)]
    .into_iter()
    .find(|&(left, top)| (left..left + 3).contains(&x) && y >= top)
    .map_or(clear(), |(left, top)| match y == top || y == top + 1 && x == left + 1 {
      true => Texel::rgb(0.12, 0.04, 0.2),
      false => tone(&TUBE, 0.35 + (x - left) as f32 * 0.2, x, y)
    })
}

fn tentacles(palette: &Palette, glow: f32, x: u32, y: u32) -> Texel {
  let offset = Vec2::new(x as f32 - 7.5, 11.0 - y as f32);
  let base = y >= 11 && (4..12).contains(&x);
  let arm = (0..7).any(|arm| {
    let angle = 0.35 + arm as f32 * 0.4;
    let toward = Vec2::from_angle(angle);
    let along = offset.dot(toward);
    (0.0..9.0).contains(&along) && (offset - toward * along).length() < 0.75
  });
  match (base, arm) {
    (true, _) => tone(palette, 0.3, x, y),
    (_, true) => tone(palette, 0.45 + offset.length() / 18.0, x, y)
      .glowing(glow * offset.length() / 9.0),
    _ => clear()
  }
}

fn polyps(x: u32, y: u32) -> Texel {
  let bulbs = [(4.0, 5.0), (9.0, 3.0), (12.0, 7.0), (6.0, 9.0)];
  let bulb = bulbs
    .into_iter()
    .find(|&(bx, by)| Vec2::new(x as f32 - bx, y as f32 - by).length() < 1.9);
  let stalk = bulbs.into_iter().any(|(bx, by)| {
    y as f32 > by && (x as f32 - (bx + (y as f32 - by) * (7.5 - bx) / 9.0)).abs() < 0.6
  });
  match (bulb, stalk) {
    (Some((bx, by)), _) => {
      let light =
        1.0 - Vec2::new(x as f32 - bx + 0.6, y as f32 - by + 0.6).length() / 2.5;
      ramp(&POLYP, 0.5 + light * 0.5).glowing(1.4)
    }
    (_, true) => tone(&POLYP, 0.15, x, y).glowing(0.3),
    _ => clear()
  }
}

fn worms(x: u32, y: u32) -> Texel {
  [(3u32, 3u32), (7, 1), (11, 4)]
    .into_iter()
    .find(|&(left, top)| (left..left + 2).contains(&x) && y >= top)
    .map_or(clear(), |(left, top)| match y < top + 3 {
      true => tone(&PLUME, 0.4 + speck(0x3E4, x, y) * 0.5, x, y),
      false => tone(&WORM, 0.3 + (x - left) as f32 * 0.4, x, y)
    })
}

fn clam(x: u32, y: u32) -> Texel {
  let offset = Vec2::new((x as f32 - 7.5) / 7.5, (y as f32 - 10.0) / 5.0);
  let inside = offset.length() < 1.0 && y >= 6;
  let lip = (y as f32 - (10.0 + ((x as f32) * 1.6).sin() * 0.7)).abs() < 0.7;
  match (inside, lip) {
    (true, true) => tone(&MANTLE, 0.55 + speck(0xC1A, x, y) * 0.4, x, y),
    (true, false) => tone(&SHELL, 0.3 + (x % 3) as f32 * 0.2 - offset.y * 0.2, x, y),
    _ => clear()
  }
}

fn meander(x: u32, y: u32) -> f32 {
  let (fx, fy) = (x as f32, y as f32);
  let warp = lattice(0xB4A1, 4, x, y) * 6.0;
  ((fx * 0.8 + warp).sin() + (fy * 0.8 - warp * 0.7).cos()).abs()
}

fn brain(x: u32, y: u32) -> Texel {
  match meander(x, y) < 0.35 {
    true => tone(&BRAIN, 0.12, x, y),
    false => tone(&BRAIN, 0.4 + lattice(0xB4A2, 8, x, y) * 0.5, x, y)
  }
}

fn pores(palette: &Palette, salt: u32, x: u32, y: u32) -> Texel {
  let hole = (-1..=1).any(|dx: i32| {
    (-1..=1).any(|dy: i32| {
      let (cx, cy) =
        (((x as i32 + dx).rem_euclid(16) / 4), ((y as i32 + dy).rem_euclid(16) / 4));
      let (px, py) = (
        cx * 4 + 1 + (speck(salt, cx as u32, cy as u32) * 2.0) as i32,
        cy * 4 + 1 + (speck(salt + 1, cx as u32, cy as u32) * 2.0) as i32
      );
      (x as i32 + dx * 0 - px).abs() + (y as i32 - py).abs() <= 1
        && speck(salt + 2, cx as u32, cy as u32) < 0.7
    })
  });
  match hole {
    true => ramp(palette, 0.05),
    false => ramp(palette, 0.45 + lattice(salt + 3, 4, x, y) * 0.5)
  }
}

fn abyssal(x: u32, y: u32) -> Texel {
  let fleck = speck(0xAB1, x, y) < 0.04;
  match fleck {
    true => Texel::rgb(0.3, 0.75, 0.85).glowing(0.6),
    false => smooth(&ABYSS, x, y)
  }
}

fn vent(x: u32, y: u32) -> Texel {
  let crack = (lattice(0x7E1, 4, x, y) - 0.5).abs() < 0.05;
  match crack {
    true => Texel::rgb(1.0, 0.45, 0.1).glowing(1.2),
    false => smooth(&VENT, x, y)
  }
}

fn nodules(x: u32, y: u32) -> Texel {
  let lump = [(4.0, 4.0, 2.3), (11.0, 6.0, 2.0), (6.0, 11.0, 2.6), (13.0, 13.0, 1.6)]
    .into_iter()
    .find(|&(nx, ny, r)| Vec2::new(x as f32 - nx, y as f32 - ny).length() < r);
  match lump {
    Some((nx, ny, r)) => {
      let shine = 1.0 - Vec2::new(x as f32 - nx + 0.7, y as f32 - ny + 0.7).length() / r;
      Texel::rgb(0.12, 0.09, 0.08).scaled(0.8 + shine.max(0.0) * 1.6)
    }
    None => solid(&DRIFT, &SILT, x, y)
  }
}

fn alloy(x: u32, y: u32) -> Texel {
  let edge = x == 0 || y == 0;
  let shadow = x == 15 || y == 15;
  let seam = y == 8 && (2..14).contains(&x);
  match (edge, shadow, seam) {
    (true, ..) => ramp(&ALLOY, 0.95),
    (_, true, _) => ramp(&ALLOY, 0.05),
    (.., true) => {
      let [r, g, b] = AETHER;
      Texel::rgb(r, g, b).glowing(0.8)
    }
    _ => ramp(&ALLOY, 0.42 + lattice(0xA11, 2, x, y * 6) * 0.2)
  }
}

fn precursor_glass(x: u32, y: u32) -> Texel {
  let frame = x == 0 || y == 0 || x == 15 || y == 15;
  let shine = (x + y == 9 || x + y == 11) && x < 9;
  match (frame, shine) {
    (true, _) => ramp(&ALLOY, 0.6),
    (_, true) => Texel::rgb(0.7, 1.0, 1.0).alpha(0.7),
    _ => Texel::rgb(0.2, 0.55, 0.6).alpha(0.32)
  }
}

fn conduit(x: u32, y: u32) -> Texel {
  let trace = (x == 4 || x == 11) && y % 8 != 7 || y == 4 && (4..12).contains(&x);
  let node = (x == 4 || x == 11) && y == 4;
  match (node, trace) {
    (true, _) => Texel::rgb(0.9, 1.0, 1.0).glowing(2.0),
    (_, true) => {
      let [r, g, b] = AETHER;
      Texel::rgb(r, g, b).glowing(1.3)
    }
    _ => ramp(&ALLOY, 0.2 + lattice(0xC0D, 4, x, y) * 0.2)
  }
}

fn nacre(x: u32, y: u32) -> Texel {
  let band = ((x as f32 + lattice(0x9AC, 4, x, y) * 5.0) * 0.6 + y as f32 * 0.3).sin();
  let [r, g, b] =
    [0.85 + band * 0.1, 0.84 + (band * 1.7).sin() * 0.08, 0.92 + band * 0.06];
  Texel::rgb(r, g, b).scaled(0.9 + lattice(0x9AD, 8, x, y) * 0.15)
}

fn biolume(x: u32, y: u32) -> Texel {
  let cell = lattice(0xB10, 4, x, y);
  ramp(&BIOLUME, 0.3 + cell * 0.7).glowing(0.4 + cell * 0.9)
}

fn lab(top: bool, x: u32, y: u32) -> Texel {
  let tank = match top {
    true => Vec2::new(x as f32 - 7.5, y as f32 - 7.5).length() < 5.0,
    false => (3..13).contains(&x) && (3..13).contains(&y)
  };
  let bubble = tank && speck(0x1AB, x, y) < 0.1;
  match (tank, bubble) {
    (true, true) => Texel::rgb(0.8, 1.0, 0.8).glowing(0.6),
    (true, false) => {
      Texel::rgb(0.2, 0.7, 0.35).scaled(0.7 + y as f32 / 30.0).glowing(0.35)
    }
    _ => brushed(STEEL, x, y)
  }
}

fn cell(x: u32, y: u32) -> Texel {
  let (fx, fy) = (x as f32 - 7.5, y as f32 - 7.5);
  let along = (fx + fy) / 2.0_f32.sqrt();
  let across = (fx - fy) / 2.0_f32.sqrt();
  match (along.abs() < 6.0, across.abs()) {
    (true, across) if across < 1.6 && along.abs() < 4.0 => {
      Texel::rgb(0.6, 1.0, 1.0).glowing(1.8)
    }
    (true, across) if across < 3.0 => ramp(&ALLOY, 0.8 - across / 6.0),
    (false, across) if across < 1.4 && along.abs() < 7.0 => ramp(&ALLOY, 0.95),
    _ => clear()
  }
}

fn scrap(x: u32, y: u32) -> Texel {
  let (fx, fy) = (x as f32, y as f32);
  let inside = fy > 3.0 + (fx * 1.3).sin() * 1.5
    && fy < 12.0 - (fx * 0.9).cos() * 1.5
    && (2..14).contains(&x)
    && fx + fy > 7.0;
  match (inside, y % 5 == 2) {
    (true, true) => Texel::rgb(0.35, 0.9, 0.95).glowing(0.6),
    (true, false) => tone(&ALLOY, 0.4 + (16 - y) as f32 / 30.0, x, y),
    _ => clear()
  }
}

fn datacore(x: u32, y: u32) -> Texel {
  let inside = (4..12).contains(&x) && (2..14).contains(&y);
  let line = y % 3 == 0 && (5..11).contains(&x);
  match (inside, line) {
    (true, true) => Texel::rgb(0.5, 1.0, 1.0).glowing(1.5),
    (true, false) => tone(&ALLOY, 0.5, x, y),
    _ => clear()
  }
}

fn gel(x: u32, y: u32) -> Texel {
  let offset = Vec2::new(x as f32 - 7.5, (y as f32 - 9.0) * 1.3);
  let wobble = (offset.y.atan2(offset.x) * 5.0).sin() * 0.6;
  match offset.length() < 6.0 + wobble {
    true => Texel::rgb(0.55, 0.85, 0.95)
      .scaled(0.7 + (1.0 - offset.length() / 6.0) * 0.5)
      .alpha(0.85)
      .glowing(0.5),
    false => clear()
  }
}

fn torpedo(x: u32, y: u32) -> Texel {
  let (fx, fy) = (x as f32 - 7.5, y as f32 - 7.5);
  let along = (fx - fy) / 2.0_f32.sqrt();
  let across = (fx + fy) / 2.0_f32.sqrt();
  let radius = match along {
    along if along > 4.5 => (7.0 - along).max(0.0) * 0.8,
    along if along < -5.5 => 0.0,
    _ => 2.0
  };
  let fin = along < -3.5 && along > -6.0 && across.abs() < 3.4 && across.abs() > 1.8;
  match (across.abs() < radius, fin) {
    (true, _) if along > 4.0 => tone(&PLUME, 0.6, x, y),
    (true, _) => ramp(&STONY, 0.5 - across / 6.0),
    (_, true) => ramp(&ALLOY, 0.4),
    _ => clear()
  }
}

fn submarine(x: u32, y: u32) -> Texel {
  let offset = Vec2::new((x as f32 - 7.5) / 7.0, (y as f32 - 9.0) / 3.6);
  let tower = (6..10).contains(&x) && (3..6).contains(&y);
  let port = [4u32, 8, 12].contains(&x) && y == 9;
  let screw = x == 0 && (7..11).contains(&y);
  match (offset.length() < 1.0 || tower, port, screw) {
    (_, _, true) => ramp(&ALLOY, 0.5),
    (true, true, _) => Texel::rgb(0.6, 0.9, 1.0),
    (true, false, _) => tone(&SUBMARINE, 0.6 - offset.y * 0.35, x, y),
    _ => clear()
  }
}

const MECH: Palette =
  [[0.42, 0.2, 0.04], [0.82, 0.46, 0.1], [0.96, 0.64, 0.18], [1.0, 0.85, 0.45]];

const COG: Palette =
  [[0.22, 0.23, 0.26], [0.42, 0.43, 0.47], [0.62, 0.63, 0.67], [0.84, 0.85, 0.88]];

fn cogwheel(x: u32, y: u32) -> Texel {
  let (fx, fy) = (x as f32 - 7.5, y as f32 - 7.5);
  let reach = Vec2::new(fx, fy).length();
  let teeth =
    ((fy.atan2(fx) / std::f32::consts::TAU * 8.0).rem_euclid(1.0) < 0.5) as u8 as f32;
  match () {
    () if reach < 1.6 => clear(),
    () if reach < 2.6 => tone(&COG, 0.85, x, y),
    () if reach < 5.2 => tone(&COG, 0.55 - (fx + fy) / 40.0, x, y),
    () if reach < 5.2 + teeth * 2.1 => tone(&COG, 0.38, x, y),
    () => clear()
  }
}

fn automaton(tile: Tile, x: u32, y: u32) -> Texel {
  let (armour, trim, glow): ([f32; 3], [f32; 3], [f32; 3]) = match tile {
    Tile::Cogling => ([0.2, 0.42, 0.85], [0.42, 0.26, 0.5], [1.0, 0.55, 0.12]),
    Tile::Gladiator => ([0.68, 0.09, 0.06], [0.95, 0.66, 0.16], [1.0, 0.55, 0.12]),
    _ => ([0.86, 0.88, 0.92], [0.4, 0.5, 0.6], [0.35, 0.78, 1.0])
  };
  let paint = |color: [f32; 3], shade: f32| {
    let [r, g, b] = color.map(|channel| channel * shade);
    Texel::rgb(r, g, b).scaled(lit(x, y))
  };
  let steel = |shade: f32| ramp(&COG, shade).scaled(lit(x, y));
  let (fx, fy) = (x as f32 - 7.5, y as f32);
  match tile {
    Tile::Cogling => {
      let head = Vec2::new(fx / 3.6, (fy - 3.6) / 3.4).length() < 1.0;
      let eye = (fx.abs() - 1.5).abs() < 0.9 && (3.0..5.0).contains(&fy);
      let torso = fx.abs() < 3.0 && (7.0..11.0).contains(&fy);
      let heart = (fx - 1.0).abs() < 0.8 && (8.0..9.5).contains(&fy);
      let arm = (fx.abs() - 4.0).abs() < 0.8 && (7.0..12.0).contains(&fy);
      let leg = (fx.abs() - 1.6).abs() < 1.0 && (11.0..15.0).contains(&fy);
      let foot = (fx.abs() - 1.8).abs() < 1.6 && fy >= 15.0;
      match () {
        () if head && eye => Texel::rgb(glow[0], glow[1], glow[2]).glowing(1.0),
        () if head => paint(armour, 1.0 - (fy - 1.0) * 0.04),
        () if heart => Texel::rgb(glow[0], glow[1], glow[2]).glowing(1.0),
        () if torso => paint(armour, 0.85),
        () if arm => match fy > 10.0 {
          true => paint(trim, 1.0),
          false => steel(0.6)
        },
        () if leg => steel(0.45),
        () if foot => paint(armour, 0.75),
        () => clear()
      }
    }
    Tile::Gladiator => {
      let head = fx.abs() < 1.6 - (fy - 2.0).max(0.0) * 0.12 && (0.0..4.0).contains(&fy);
      let eye = fx.abs() < 1.2 && fy == 2.0;
      let pads = (fx.abs() - 3.6).abs() < 1.6 && (4.0..6.0).contains(&fy);
      let spike = (fx.abs() - 4.5).abs() < 0.6 && (2.0..4.0).contains(&fy);
      let chest = fx.abs() < 2.6 - (fy - 5.0) * 0.35 && (5.0..9.0).contains(&fy);
      let arm = (fx.abs() - 3.8).abs() < 0.7 && (6.0..11.0).contains(&fy);
      let leg = (fx.abs() - 1.5).abs() < 0.8 && (9.0..15.0).contains(&fy);
      let foot = (fx.abs() - 1.8).abs() < 1.3 && fy >= 15.0;
      let blade = x == 13 + (y < 6) as u32 && (1..10).contains(&y);
      match () {
        () if eye => Texel::rgb(glow[0], glow[1], glow[2]).glowing(1.0),
        () if blade => ramp(&COG, 0.95),
        () if head => paint(armour, 1.0),
        () if spike => paint(trim, 1.1),
        () if pads => paint(armour, 0.95),
        () if chest => paint(armour, 0.85 + (fx.abs() < 0.6) as u8 as f32 * 0.3),
        () if arm => steel(0.55),
        () if leg => match fy < 12.0 {
          true => steel(0.5),
          false => paint(armour, 0.8)
        },
        () if foot => paint(trim, 0.9),
        () => clear()
      }
    }
    _ => {
      let (mx, my) = (fx / 6.2, (fy - 8.0) / 7.0);
      let mask = Vec2::new(mx, my).length() < 1.0 && my < 0.85 - mx.abs() * 0.3;
      let socket = (fx.abs() - 2.4).abs() < 1.5 && (5.0..8.5).contains(&fy);
      let eye = (fx.abs() - 2.4).abs() < 0.7 && (6.0..7.5).contains(&fy);
      let brow = fy < 5.0 && fy >= 4.0 && fx.abs() < 4.5;
      let nose = fx.abs() < 0.7 && (8.0..11.0).contains(&fy);
      match () {
        () if !mask => clear(),
        () if eye => Texel::rgb(glow[0], glow[1], glow[2]).glowing(1.0),
        () if socket => steel(0.05),
        () if brow || nose => paint(trim, 1.0),
        () => paint(armour, 1.0 - (fy - 2.0) * 0.03 - mx.abs() * 0.15)
      }
    }
  }
}

fn mech(x: u32, y: u32) -> Texel {
  let leg = ((4..7).contains(&x) || (9..12).contains(&x)) && (10..15).contains(&y);
  let foot = ((3..7).contains(&x) || (9..13).contains(&x)) && y == 15;
  let torso = (3..13).contains(&x) && (2..10).contains(&y);
  let canopy = (5..11).contains(&x) && (3..7).contains(&y);
  let arm = ((1..3).contains(&x) || (13..15).contains(&x)) && (5..10).contains(&y);
  match (canopy, torso, leg || foot, arm) {
    (true, ..) => Texel::rgb(0.55 + y as f32 * 0.04, 0.88, 1.0),
    (_, true, ..) => tone(&MECH, 0.62 - y as f32 * 0.03, x, y),
    (_, _, true, _) => ramp(&ALLOY, 0.35 + f32::from(u8::from(foot)) * 0.2),
    (.., true) => ramp(&ALLOY, 0.55),
    _ => clear()
  }
}

fn emblem(tile: Tile, x: u32, y: u32) -> Option<f32> {
  let (fx, fy) = (x as f32 - 7.5, y as f32 - 7.5);
  let reach = Vec2::new(fx, fy).length();
  let shown = match tile {
    Tile::SonarModule | Tile::DeepSonar => {
      (reach - 1.5).abs() < 0.6 || (reach - 4.0).abs() < 0.6 && fy < 1.0
    }
    Tile::DrillModule | Tile::PlasmaDrill | Tile::MechDrill | Tile::DiamondDrill => {
      fx.abs() < (fy + 4.0) * 0.45 && fy > -4.0
    }
    Tile::HydraulicClamp => {
      (reach - 3.2).abs() < 0.7 && fy < 1.5 && fx.abs() > 0.8
        || fx.abs() < 0.7 && fy > 1.0
    }
    Tile::OreBox => {
      (2.6..3.8).contains(&fx.abs().max(fy.abs())) || fy.abs() < 0.6 && fx.abs() < 3.0
    }
    Tile::Jumpjets => {
      (fx.abs() - 2.0).abs() < 1.0 && fy < 1.5 && fy > -4.0
        || (fx.abs() - 2.0).abs() < 0.6 && fy > 2.0 && fy < 4.0
    }
    Tile::MechLaser => fy.abs() < 0.7 && fx > -3.0 || reach < 1.6,
    Tile::MissileRack => Vec2::new(fx.abs() - 2.0, fy.abs() - 2.0).length() < 1.3,
    Tile::Sleeper => fx.abs() < 0.9 && fy.abs() < 3.6 || fy.abs() < 0.9 && fx.abs() < 3.6,
    Tile::RepairDroid => {
      (reach - 2.4).abs() < 0.7 || (fx + fy).abs() < 0.8 && reach < 4.4
    }
    Tile::HeatShield => fy < 3.5 && fy > -4.0 && fx.abs() < (3.5 - fy) * 0.45,
    Tile::BuildArm => {
      fx.abs() < 0.8 && fy > -4.0 && fy < 4.0
        || fy > -4.0 && fy < -2.4 && fx > -0.8 && fx < 4.0
    }
    Tile::Dynamo => (fx + fy * 0.6 - fy.signum() * 1.2).abs() < 0.9 && fy.abs() < 4.2,
    Tile::Floodlight => reach < 2.0 || (reach < 4.6 && (fx * fy).abs() < 0.8),
    Tile::Thruster | Tile::Hydrojet => {
      ((fx - fy.abs()) - 1.0).abs() < 0.8 || ((fx - fy.abs()) + 2.5).abs() < 0.8
    }
    Tile::HullPlating | Tile::ResonantPlating => {
      fx.abs() < 4.0 && fy.abs() < 4.0 && (fy.rem_euclid(3.0) > 1.0 || fx.abs() > 3.0)
    }
    Tile::SampleClaw => {
      (reach - 3.0).abs() < 0.7 && fy < 2.0 || fx.abs() < 0.7 && fy > 0.0
    }
    Tile::TorpedoBay => fy.abs() < 1.4 && fx.abs() < 4.5,
    Tile::PowerCell => fx.abs() < 2.4 && fy.abs() < 4.0 && (fy as i32) % 2 == 0,
    _ => false
  };
  shown.then_some(1.0 - reach / 9.0)
}

fn module(tile: Tile, x: u32, y: u32) -> Texel {
  let precursor = matches!(
    tile,
    Tile::DeepSonar
      | Tile::PlasmaDrill
      | Tile::Hydrojet
      | Tile::ResonantPlating
      | Tile::DiamondDrill
      | Tile::MechLaser
  );
  let body = match precursor {
    true => ALLOY,
    false => SLATE_GREY
  };
  let inside = (1..15).contains(&x) && (1..15).contains(&y);
  let rim = x == 1 || y == 1 || x == 14 || y == 14;
  let rivet = (x == 3 || x == 12) && (y == 3 || y == 12);
  let hazard = matches!(
    tile,
    Tile::HydraulicClamp
      | Tile::MechDrill
      | Tile::DiamondDrill
      | Tile::OreBox
      | Tile::Jumpjets
      | Tile::MechLaser
      | Tile::MissileRack
      | Tile::Sleeper
      | Tile::RepairDroid
      | Tile::HeatShield
      | Tile::BuildArm
      | Tile::Dynamo
  );
  match (inside, emblem(tile, x, y)) {
    (false, _) => clear(),
    (true, None) if rim && hazard => match (x + y) / 2 % 2 == 0 {
      true => Texel::rgb(0.95, 0.75, 0.12),
      false => Texel::rgb(0.08, 0.08, 0.09)
    },
    (true, Some(light)) => match precursor {
      true => Texel::rgb(0.4 + light * 0.4, 0.95, 1.0).glowing(1.2),
      false => Texel::rgb(0.95, 0.72 + light * 0.2, 0.22)
    },
    (true, None) if rivet => ramp(&body, 0.9),
    (true, None) if rim => ramp(&body, 0.75).scaled(lit(x, y)),
    (true, None) => ramp(&body, 0.35 + lattice(0x30D, 2, x, y * 5) * 0.15)
  }
}

fn aether_lamp(x: u32, y: u32) -> Texel {
  let core = (5..11).contains(&x) && (4..12).contains(&y);
  match core {
    true => Texel::rgb(0.5, 1.0, 1.0).glowing(1.6),
    false => alloy(x, y)
  }
}

pub fn paint(tile: Tile, x: u32, y: u32) -> Texel {
  match tile {
    Tile::Greywacke => smooth(&GREY, x, y),
    Tile::GreywackeCobble => cobble([0.48, 0.48, 0.5], x, y),
    Tile::Granite | Tile::Diorite | Tile::Andesite | Tile::Limestone | Tile::Slate => {
      smooth(&shades(rock(tile), STONE_SPREAD), x, y)
    }
    Tile::GraniteCobble
    | Tile::DioriteCobble
    | Tile::AndesiteCobble
    | Tile::LimestoneCobble
    | Tile::SlateCobble => cobble(rock(tile), x, y),
    Tile::Dirt => soil(x, y),
    Tile::GrassTop => turf(&GREEN, x, y),
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
    Tile::Planks => planks([0.64, 0.48, 0.28], x, y),
    Tile::GreywackeBricks => masonry(&GREYWACKE_BRICKS, x, y),
    Tile::Kelp => frond(&KELP, 0.0, x, y),
    Tile::Glowkelp => frond(&GLOWKELP, 0.5, x, y),
    Tile::Seagrass => blades(x, y),
    Tile::BrainCoral => brain(x, y),
    Tile::FanCoral => fan(x, y),
    Tile::TubeCoral => tubes(x, y),
    Tile::Anemone => tentacles(&ANEMONE, 0.0, x, y),
    Tile::GlowPolyp => polyps(x, y),
    Tile::VentWorms => worms(x, y),
    Tile::Sponge => pores(&SPONGE, 0x5B0, x, y),
    Tile::Clam => clam(x, y),
    Tile::Pearl => cutout(&PEARL, &PEARLY, x, y).glowing(0.15),
    Tile::Abyssite => abyssal(x, y),
    Tile::VentStone => vent(x, y),
    Tile::NoduleOre => nodules(x, y),
    Tile::PrecursorAlloy => alloy(x, y),
    Tile::PrecursorGlass => precursor_glass(x, y),
    Tile::AetherConduit => conduit(x, y),
    Tile::AetherLamp => aether_lamp(x, y),
    Tile::AetherCell => cell(x, y),
    Tile::AlloyScrap => scrap(x, y),
    Tile::ResonantCrystal => glinting(&GEM, &RESONANT, 0.9, x, y),
    Tile::Datacore => datacore(x, y),
    Tile::JellyGel => gel(x, y),
    Tile::BioLabTop => lab(true, x, y),
    Tile::BioLabSide => lab(false, x, y),
    Tile::Reefstone => pores(&REEF, 0x2EE, x, y),
    Tile::Nacre => nacre(x, y),
    Tile::Biolume => biolume(x, y),
    Tile::Submarine => submarine(x, y),
    Tile::Torpedo => torpedo(x, y),
    Tile::SonarModule
    | Tile::DeepSonar
    | Tile::DrillModule
    | Tile::PlasmaDrill
    | Tile::Floodlight
    | Tile::Thruster
    | Tile::Hydrojet
    | Tile::HullPlating
    | Tile::ResonantPlating
    | Tile::SampleClaw
    | Tile::TorpedoBay
    | Tile::PowerCell
    | Tile::HydraulicClamp
    | Tile::MechDrill
    | Tile::DiamondDrill
    | Tile::OreBox
    | Tile::Jumpjets
    | Tile::MechLaser
    | Tile::MissileRack
    | Tile::Sleeper
    | Tile::RepairDroid
    | Tile::HeatShield
    | Tile::BuildArm
    | Tile::Dynamo => module(tile, x, y),
    Tile::Mech => mech(x, y),
    Tile::Cogwheel => cogwheel(x, y),
    Tile::Cogling | Tile::Gladiator | Tile::Paragon => automaton(tile, x, y),
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
    Tile::MyceliumTop => turf(&SPORES, x, y),
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
    Tile::WaystoneSide => match shade_of(&RUNES, x, y) {
      Some(_) => rune(),
      None => waystone(x, y)
    },
    Tile::Grain => solid(&STONE, &GRAIN, x, y),
    Tile::WaystoneStone => waystone(x, y),
    Tile::Flame => flame(x, y),
    Tile::TableTop => {
      let edge = x == 0 || y == 0 || x == PIXELS - 1 || y == PIXELS - 1;
      let grid = (x == 5 || x == 10 || y == 5 || y == 10) && !edge;
      match edge || grid {
        true => Texel::rgb(0.3, 0.2, 0.11),
        false => paint(Tile::Planks, x, y).scaled(1.08)
      }
    }
    Tile::TableSide => drawn(&HAMMER, &TOOLS, x, y).unwrap_or_else(|| match y < 2 {
      true => Texel::rgb(0.3, 0.2, 0.11),
      false => paint(Tile::Planks, x, y)
    }),
    Tile::FurnaceTop => smooth(&GREY, x, y),
    Tile::FurnaceSide => {
      let mouth = (3..13).contains(&x) && (7..14).contains(&y);
      let rim = (2..14).contains(&x) && (6..15).contains(&y);
      match (mouth, rim) {
        (true, _) if y >= 11 => {
          Texel::rgb(0.95, 0.4 + (x % 3) as f32 * 0.1, 0.08).glowing(0.8)
        }
        (true, _) => Texel::rgb(0.05, 0.04, 0.04),
        (false, true) => Texel::rgb(0.22, 0.22, 0.23),
        _ => cobble([0.5, 0.5, 0.52], x, y)
      }
    }
    Tile::Boat => cutout(&BOAT, &WOOD, x, y),
    Tile::Bucket => bucket(None, x, y),
    Tile::WaterBucket => bucket(Some(Texel::rgb(0.2, 0.45, 0.85)), x, y),
    Tile::LavaBucket => bucket(Some(Texel::rgb(1.0, 0.5, 0.1).glowing(0.4)), x, y),
    Tile::Torch => model_icon(Block::Torch, x, y),
    Tile::TallGrass => model_icon(Block::TallGrass, x, y),
    Tile::Bed => match x < PIXELS / 2 {
      true => model_icon(Block::BedEast, x * 2, y),
      false => model_icon(Block::BedHeadEast, x * 2 - PIXELS, y)
    },
    Tile::Ladder => solid(&GROOVES, &RUNGS, y, x),
    Tile::Wool => wool(x, y),
    Tile::Feather => cutout(&FEATHER, &QUILL, x, y),
    Tile::Charcoal => cutout(&CHARCOAL, &EMBERLESS, x, y),
    Tile::FishingRod => cutout(&ROD, &TACKLE, x, y),
    Tile::Cod | Tile::Salmon => cutout(&FISH, &fish(tile), x, y),
    Tile::TropicalFish => match shade_of(&FISH, x, y) {
      Some(shade) if shade > 1 && (x == 6 || x == 10) => Texel::rgb(0.98, 0.95, 0.9),
      _ => cutout(&FISH, &fish(tile), x, y)
    },
    Tile::Pufferfish => cutout(&PUFFER, &fish(tile), x, y),
    Tile::OakSign => cutout(&SIGN, &timber(Block::OakSign), x, y),
    Tile::BirchSign => cutout(&SIGN, &timber(Block::BirchSign), x, y),
    Tile::SpruceSign => cutout(&SIGN, &timber(Block::SpruceSign), x, y),
    Tile::PalmSign => cutout(&SIGN, &timber(Block::PalmSign), x, y),
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
      model_icon(Block::ALL_MODELS[tile as usize - Tile::Poppy as usize], x, y)
    }
    Tile::AncientBrick => ancient(x, y),
    Tile::MossyBrick => mossy(x, y),
    Tile::CrackedBrick => match shade_of(&CRACKS, x, y) {
      Some(0) => Texel::rgb(0.08, 0.08, 0.09),
      _ => ancient(x, y)
    },
    Tile::CarvedStone => match (shade_of(&RUNES, x, y), (2..14).contains(&x)) {
      (Some(_), _) => Texel::rgb(0.35, 0.95, 0.8).glowing(0.55),
      (None, true) if y > 0 && y < 15 => {
        smooth(&shades(ANCIENT, STONE_SPREAD), x, y).scaled(0.8)
      }
      _ => ancient(x, y)
    },
    Tile::Lumen => lumen(x, y),
    Tile::BonePile => drawn(&BONES, &IVORY, x, y)
      .unwrap_or_else(|| ramp(&BLACK, 0.2 + lattice(0x7B, 8, x, y) * 0.3)),
    Tile::Cobweb => drawn(&WEB, &WHITE, x, y)
      .map_or(Texel::rgb(0.0, 0.0, 0.0).alpha(0.0), |web| web.alpha(0.85)),
    Tile::Terracotta => terracotta(x, y),
    Tile::ArcaneDust => {
      match cutout(&DUST, &shades([0.62, 0.38, 0.95], STONE_SPREAD), x, y) {
        texel if shade_of(&DUST, x, y) == Some(3) => texel.glowing(0.8),
        texel => texel
      }
    }
    Tile::BoneDust => cutout(&DUST, &IVORY, x, y),
    Tile::LizardClaw => inked(&CLAW, [&TALON, &SCALES, &SCALES], x, y),
    Tile::Candle => match (y < 4, inked(&CANDLE, [&STALK, &GILT, &GILT], x, y)) {
      (true, texel) if texel.color[3] > 0.0 => flame(x, y + 5),
      (_, texel) => texel
    },
    Tile::SpellFirebolt => spell(
      crate::glyph::Glyph::Flame,
      [[0.45, 0.08, 0.05], [0.85, 0.25, 0.06], [1.0, 0.55, 0.1], [1.0, 0.88, 0.45]],
      x,
      y
    ),
    Tile::SpellFrost => spell(
      crate::glyph::Glyph::Snowflake,
      [[0.1, 0.2, 0.45], [0.25, 0.5, 0.85], [0.55, 0.82, 1.0], [0.9, 0.98, 1.0]],
      x,
      y
    ),
    Tile::SpellMissile => spell(
      crate::glyph::Glyph::Missile,
      [[0.28, 0.08, 0.45], [0.55, 0.2, 0.85], [0.82, 0.45, 1.0], [1.0, 0.8, 1.0]],
      x,
      y
    ),
    Tile::SpellHeal => spell(
      crate::glyph::Glyph::Mend,
      [[0.06, 0.3, 0.16], [0.2, 0.62, 0.3], [0.5, 0.88, 0.45], [0.88, 1.0, 0.75]],
      x,
      y
    ),
    Tile::SpellHaste => spell(
      crate::glyph::Glyph::Hourglass,
      [[0.45, 0.25, 0.05], [0.8, 0.55, 0.1], [0.98, 0.82, 0.3], [1.0, 0.97, 0.7]],
      x,
      y
    ),
    Tile::SpellStoneskin => spell(
      crate::glyph::Glyph::Stone,
      [[0.25, 0.22, 0.18], [0.5, 0.45, 0.36], [0.7, 0.65, 0.52], [0.9, 0.86, 0.72]],
      x,
      y
    ),
    Tile::SpellLeap => spell(
      crate::glyph::Glyph::Feather,
      [[0.15, 0.35, 0.4], [0.35, 0.65, 0.7], [0.65, 0.9, 0.88], [0.92, 1.0, 0.98]],
      x,
      y
    ),
    Tile::SpellLight => spell(
      crate::glyph::Glyph::Sun,
      [[0.5, 0.32, 0.06], [0.9, 0.66, 0.15], [1.0, 0.88, 0.45], [1.0, 1.0, 0.85]],
      x,
      y
    ),
    Tile::SpellThunder => spell(
      crate::glyph::Glyph::Thunder,
      [[0.25, 0.12, 0.45], [0.55, 0.35, 0.9], [0.95, 0.85, 0.35], [1.0, 1.0, 0.8]],
      x,
      y
    ),
    Tile::SpellFireball => spell(
      crate::glyph::Glyph::Burst,
      [[0.4, 0.05, 0.12], [0.8, 0.2, 0.1], [1.0, 0.48, 0.12], [1.0, 0.82, 0.35]],
      x,
      y
    ),
    Tile::SpellIceStorm => spell(
      crate::glyph::Glyph::Storm,
      [[0.16, 0.18, 0.4], [0.38, 0.45, 0.75], [0.65, 0.78, 0.95], [0.92, 0.96, 1.0]],
      x,
      y
    ),
    Tile::SpellAnimate => spell(
      crate::glyph::Glyph::Skull,
      [[0.18, 0.2, 0.1], [0.42, 0.5, 0.25], [0.75, 0.8, 0.55], [0.75, 1.0, 0.5]],
      x,
      y
    ),
    Tile::SpellGuardian => spell(
      crate::glyph::Glyph::Ghost,
      [[0.12, 0.2, 0.4], [0.3, 0.48, 0.78], [0.6, 0.8, 0.98], [0.9, 0.98, 1.0]],
      x,
      y
    ),
    Tile::SpellInvisibility => spell(
      crate::glyph::Glyph::Cloak,
      [[0.2, 0.12, 0.3], [0.42, 0.32, 0.58], [0.66, 0.58, 0.82], [0.88, 0.84, 0.98]],
      x,
      y
    ),
    Tile::Pickaxe => forged(Block::Pickaxe, Material::Iron, x, y),
    Tile::Axe => forged(Block::Axe, Material::Iron, x, y),
    Tile::Shovel => forged(Block::Shovel, Material::Iron, x, y),
    Tile::Sword => forged(Block::Sword, Material::Iron, x, y),
    Tile::Hat => inked(&HAT, [&LEATHER, &CRIMSON, &GILT], x, y),
    Tile::Amulet => match inked(&AMULET, [&SAPPHIRE, &GILT, &GILT], x, y) {
      gem if shade_of(&AMULET, x, y) == Some(3) => gem.glowing(0.4),
      texel => texel
    },
    Tile::Mail => inked(&MAIL, [&IRON, &HAFT, &HAFT], x, y),
    Tile::Belt => inked(&BELT, [&IRON, &LEATHER, &GILT], x, y),
    Tile::Boots => inked(&BOOTS, [&IRON, &LEATHER, &GILT], x, y),
    Tile::Balloon => inked(&BALLOON, [&CRIMSON, &IVORY, &WOOD], x, y),
    Tile::Copper => brushed(COPPER, x, y),
    Tile::Steel => brushed(STEEL, x, y),
    Tile::PipeGlass => pipe_glass(x, y),
    Tile::Casing => casing(x, y),
    Tile::Grille => grille(x, y),
    Tile::Solar => solar(x, y),
    Tile::Cable => cable(x, y),
    Tile::OilSeep => oil(x, y),
    Tile::Tar => tar(x, y),
    Tile::Cells => cells(x, y),
    Tile::Coil => coil(x, y),
    Tile::Bulb => bulb(false, x, y),
    Tile::BulbLit => bulb(true, x, y),
    Tile::Hazard => hazard(x, y),
    Tile::Rubber => rubber(x, y),
    Tile::PlatinumOre => {
      ore(x, y, [[0.6, 0.62, 0.66], [0.84, 0.86, 0.9], [1.0, 1.0, 1.0]], 0.15)
    }
    Tile::Platinum => glinting(&INGOT, &PLATINUM, 0.1, x, y),
    Tile::PlatinumDust => cutout(&DUST, &PLATINUM, x, y),
    Tile::Moonlace => model_icon(Block::Moonlace, x, y),
    Tile::Crusher => crusher(x, y),
    Tile::Satchel => inked(&BAG, [&CANVAS, &STRAP, &RIVET], x, y),
    Tile::Rucksack => inked(&BAG, [&LEATHER, &STRAP, &RIVET], x, y),
    Tile::ExpeditionPack => inked(&BAG, [&OILCLOTH, &LEATHER, &PLATINUM], x, y),
    Tile::LesserRift => rift(&GLIMSTEEL, x, y),
    Tile::GreaterRift => rift(&PLATINUM, x, y),
    Tile::GlimmerPink => model_icon(Block::GlimmerPink, x, y),
    Tile::GlimmerCyan => model_icon(Block::GlimmerCyan, x, y),
    Tile::GlimmerViolet => model_icon(Block::GlimmerViolet, x, y),
    Tile::GlimmerGold => model_icon(Block::GlimmerGold, x, y),
    Tile::Sunsip => model_icon(Block::Sunsip, x, y),
    Tile::Dewcup => model_icon(Block::Dewcup, x, y),
    Tile::Emberbell => model_icon(Block::Emberbell, x, y),
    Tile::Moonstone => moonstone(x, y),
    Tile::Runework => {
      inlaid(x, y, Texel::rgb(0.45, 0.85, 1.0).glowing(1.2), moonstone(x, y))
    }
    Tile::CrimsonRune => inlaid(
      x,
      y,
      Texel::rgb(0.85, 0.08, 0.1).glowing(0.9),
      smooth(&SLATE_GREY, x, y).scaled(0.7)
    ),
    Tile::Glimsteel => glinting(&INGOT, &GLIMSTEEL, 0.2, x, y),
    Tile::Verdantine => glinting(&INGOT, &VERDANTINE, 0.45, x, y),
    Tile::Hexbrass => cutout(&INGOT, &HEXBRASS, x, y),
    Tile::MithrilIngot => glinting(&INGOT, &MITHRIL, 0.25, x, y),
    Tile::Starheart => glinting(&GEM, &STARHEART, 0.6, x, y),
    Tile::Tidepearl => glinting(&PEARL, &TIDEPEARL, 0.5, x, y),
    Tile::Heartstone => glinting(&PEARL, &BLOOD, 0.5, x, y),
    Tile::BareTablet => cutout(&TABLET, &SLATE_GREY, x, y),
    Tile::BoundTablet => match inked(&BOUND_TABLET, [&SLATE_GREY, &BLOOD, &GILT], x, y) {
      rune if BOUND_TABLET[y as usize].as_bytes()[x as usize] == b'd' => {
        rune.glowing(0.6)
      }
      texel => texel
    },
    Tile::RitualKnife => inked(&KNIFE, [&GREY, &HAFT, &GILT], x, y),
    Tile::WellspringSigil => match inked(&SIGIL, [&WATER_DROP, &SLATE_GREY, &GILT], x, y)
    {
      drop if shade_of(&SIGIL, x, y).is_some() => drop.glowing(0.25),
      texel => texel
    },
    Tile::Ring => match inked(&RING, [&CRIMSON, &GILT, &GILT], x, y) {
      gem if shade_of(&RING, x, y) == Some(3) => gem.glowing(0.4),
      texel => texel
    },
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

fn halve(level: &[[f32; 4]], width: u32, height: u32) -> Vec<[f32; 4]> {
  let (half_width, half_height) = (width / 2, height / 2);
  (0..half_width * half_height)
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
    .collect()
}

fn atlas_levels(texel: impl Fn(u32, u32, u32) -> [f32; 4]) -> Vec<u8> {
  let (width, height) = (COLUMNS * PIXELS, ROWS * PIXELS);
  let base: Vec<[f32; 4]> = (0..width * height)
    .map(|index| {
      let (x, y) = (index % width, index / width);
      let slot = (y / PIXELS) * COLUMNS + x / PIXELS;
      match slot < PAINTED {
        true => texel(slot, x % PIXELS, y % PIXELS),
        false => [0.0; 4]
      }
    })
    .collect();
  let levels =
    std::iter::successors(Some((base, width, height)), |(level, width, height)| {
      (*width > COLUMNS).then(|| (halve(level, *width, *height), width / 2, height / 2))
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

pub fn albedo() -> Image { image(atlas_levels(|slot, x, y| painted(slot, x, y).color)) }

pub fn glow() -> Image {
  image(atlas_levels(|slot, x, y| {
    let [r, g, b] = painted(slot, x, y).glow;
    [r, g, b, 1.0]
  }))
}

pub const ICON: u32 = 96;
const ICON_DRAWN: f32 = 32.0;
pub const ICON_COLUMNS: u32 = 10;

struct Brick {
  low: Vec3,
  high: Vec3,
  faces: [Tile; 6],
  tint: [f32; 3]
}

const ICON_AXES: Mat3 = Mat3::from_cols(
  Vec3::new(15.0, 7.5, 0.0),
  Vec3::new(0.0, -15.0, 0.0),
  Vec3::new(-15.0, 7.5, 0.0)
);
const ICON_SPAN: f32 = 30.0;
const ICON_ZOOM: f32 = 2.4;
const ICON_SAMPLES: u32 = 2;
const FACE_SHADES: [f32; 6] = [0.7, 0.62, 0.5, 1.0, 0.6, 0.8];

fn bricks(block: Block) -> Vec<Brick> {
  let bits = model::shape(block, model::ACROSS);
  match bits.is_empty() {
    true => {
      let [top, side, bottom] = block.tiles();
      vec![Brick {
        low: Vec3::ZERO,
        high: Vec3::ONE,
        faces: [side, side, bottom, top, side, side],
        tint: [1.0; 3]
      }]
    }
    false => bits
      .iter()
      .map(|bit| Brick {
        low: Vec3::from(bit.low.map(f32::from)) / 16.0,
        high: Vec3::from(bit.high.map(f32::from)) / 16.0,
        faces: [bit.tile; 6],
        tint: bit.color
      })
      .collect()
  }
}

fn on_face(axis: usize, point: Vec3) -> (u32, u32) {
  let (u, v) = match axis {
    0 => (1.0 - point.z, 1.0 - point.y),
    1 => (point.x, point.z),
    _ => (point.x, 1.0 - point.y)
  };
  let texel =
    |t: f32| ((t * PIXELS as f32).floor() as i32).clamp(0, PIXELS as i32 - 1) as u32;
  (texel(u), texel(v))
}

fn iso(block: Block) -> impl Fn(u32, u32) -> [f32; 4] {
  let bricks = bricks(block);
  let painted: Vec<(Tile, Vec<[f32; 4]>)> =
    bricks.iter().flat_map(|brick| brick.faces).fold(Vec::new(), |mut painted, tile| {
      if !painted.iter().any(|(known, _)| *known == tile) {
        painted.push((
          tile,
          (0..PIXELS * PIXELS)
            .map(|index| paint(tile, index % PIXELS, index / PIXELS).color)
            .collect()
        ))
      }
      painted
    });
  let texel = move |tile: Tile, (u, v): (u32, u32)| {
    painted
      .iter()
      .find(|(known, _)| *known == tile)
      .map_or([0.0; 4], |(_, texels)| texels[(v * PIXELS + u) as usize])
  };
  let (low, high) =
    bricks.iter().fold((Vec3::splat(1.0), Vec3::ZERO), |(low, high), brick| {
      (low.min(brick.low), high.max(brick.high))
    });
  let corners: Vec<Vec2> = (0..8)
    .map(|corner| {
      let pick =
        |bit: u32, axis: usize| if corner & bit == 0 { low[axis] } else { high[axis] };
      (ICON_AXES * Vec3::new(pick(1, 0), pick(2, 1), pick(4, 2))).truncate()
    })
    .collect();
  let (least, most) =
    corners.iter().fold((Vec2::MAX, Vec2::MIN), |(least, most), &corner| {
      (least.min(corner), most.max(corner))
    });
  let zoom = (ICON_SPAN / (most - least).max_element()).min(ICON_ZOOM);
  let shift = Vec2::splat(ICON_DRAWN / 2.0) - (least + most) / 2.0 * zoom;
  let across =
    Mat2::from_cols(ICON_AXES.x_axis.truncate(), ICON_AXES.z_axis.truncate()).inverse();
  let sample = move |spot: Vec2| -> [f32; 4] {
    let flat = across * ((spot - shift) / zoom);
    let start = Vec3::new(flat.x, 0.0, flat.y);
    let mut hits: Vec<(f32, usize, &Brick)> = bricks
      .iter()
      .flat_map(|brick| {
        let near = brick.high - start;
        let far = brick.low - start;
        let enter = near.min_element();
        let leave = far.max_element();
        let entered = (0..3).find(|&axis| near[axis] == enter).unwrap_or(0);
        let left = (0..3).find(|&axis| far[axis] == leave).unwrap_or(0);
        (leave < enter)
          .then_some([(enter, entered * 2 + 1, brick), (leave, left * 2, brick)])
          .into_iter()
          .flatten()
      })
      .collect();
    hits.sort_by(|a, b| b.0.total_cmp(&a.0));
    hits
      .iter()
      .find_map(|&(t, face, brick)| {
        let point = start + Vec3::splat(t);
        let [r, g, b, a] = texel(brick.faces[face], on_face(face / 2, point));
        let shade = FACE_SHADES[face];
        let [tr, tg, tb] = brick.tint;
        (a > 0.0).then_some([r * tr * shade, g * tg * shade, b * tb * shade, a])
      })
      .unwrap_or([0.0; 4])
  };
  move |x, y| {
    let total = (0..ICON_SAMPLES * ICON_SAMPLES).fold([0.0; 4], |sum, index| {
      let offset = Vec2::new(
        (index % ICON_SAMPLES) as f32 + 0.5,
        (index / ICON_SAMPLES) as f32 + 0.5
      ) / ICON_SAMPLES as f32;
      let [r, g, b, a] =
        sample((Vec2::new(x as f32, y as f32) + offset) * ICON_DRAWN / ICON as f32);
      [sum[0] + r * a, sum[1] + g * a, sum[2] + b * a, sum[3] + a]
    });
    let coverage = total[3] / (ICON_SAMPLES * ICON_SAMPLES) as f32;
    match total[3] > 0.0 {
      true => [total[0] / total[3], total[1] / total[3], total[2] / total[3], coverage],
      false => [0.0; 4]
    }
  }
}

fn icon_painter(block: Block) -> Box<dyn Fn(u32, u32) -> [f32; 4]> {
  match block.look() {
    _ if block.ladder() || block.sign() => Box::new(move |x, y| match block.ladder() {
      true => model_icon(block, x * PIXELS / ICON, y * PIXELS / ICON).color,
      false => paint(block.tiles()[1], x * PIXELS / ICON, y * PIXELS / ICON).color
    }),
    _ if block == Block::Cobweb => Box::new(move |x, y| {
      paint(Tile::Cobweb, x * PIXELS / ICON, y * PIXELS / ICON).color
    }),
    Look::Opaque | Look::Cutout | Look::Log if block.item() => Box::new(iso(block)),
    _ if block.modelled() || block.furnished() => Box::new(iso(block)),
    _ => Box::new(move |x, y| {
      paint(block.tiles()[1], x * PIXELS / ICON, y * PIXELS / ICON).color
    })
  }
}

pub const ICON_FORGED: u32 =
  (Block::ALL.len() as u32).div_ceil(ICON_COLUMNS) * ICON_COLUMNS;

pub fn icon_cell(block: Block, material: Option<Material>) -> u32 {
  match material.filter(|_| block.forged()) {
    Some(material) => ICON_FORGED + forged_slot(block, material) - FORGED,
    None => block as u32
  }
}

pub fn icons() -> Image {
  let rows = (ICON_FORGED + PAINTED - FORGED).div_ceil(ICON_COLUMNS);
  let (wide, tall) = (ICON_COLUMNS * ICON, rows * ICON);
  let painters: Vec<Box<dyn Fn(u32, u32) -> [f32; 4]>> = (0..rows * ICON_COLUMNS)
    .map(|cell| -> Box<dyn Fn(u32, u32) -> [f32; 4]> {
      match (Block::ALL.get(cell as usize), cell >= ICON_FORGED) {
        (Some(&block), _) => icon_painter(block),
        (None, true) if cell < ICON_FORGED + PAINTED - FORGED => Box::new(move |x, y| {
          painted(cell - ICON_FORGED + FORGED, x * PIXELS / ICON, y * PIXELS / ICON).color
        }),
        _ => Box::new(|_, _| [0.0; 4])
      }
    })
    .collect();
  let mut image = Image::new(
    Extent3d { width: wide, height: tall, depth_or_array_layers: 1 },
    TextureDimension::D2,
    (0..wide * tall)
      .flat_map(|index| {
        let (x, y) = (index % wide, index / wide);
        let cell = (y / ICON) * ICON_COLUMNS + x / ICON;
        painters
          .get(cell as usize)
          .map_or([0.0; 4], |painter| painter(x % ICON, y % ICON))
          .map(to_srgb)
      })
      .collect(),
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  );
  image.sampler = ImageSampler::linear();
  image
}

pub fn uv_corner(tile: Tile, corner: Vec2) -> Vec2 { uv_slot(tile.index(), corner) }

pub fn tile_origin(tile: Tile) -> Vec2 {
  let index = tile.index();
  Vec2::new((index % COLUMNS) as f32, (index / COLUMNS) as f32)
    / Vec2::new(COLUMNS as f32, ROWS as f32)
}

pub fn uv_slot(index: u32, corner: Vec2) -> Vec2 {
  let cell = Vec2::new((index % COLUMNS) as f32, (index / COLUMNS) as f32);
  (cell + corner * 0.996 + 0.002) / Vec2::new(COLUMNS as f32, ROWS as f32)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tiles_paint_without_gaps() {
    for tile in Tile::ALL.into_iter() {
      for index in 0..PIXELS * PIXELS {
        let texel = paint(tile, index % PIXELS, index / PIXELS);
        assert!(texel.color != [1.0, 0.0, 1.0, 1.0], "{tile:?} has a hole at {index}")
      }
    }
  }

  #[test]
  #[ignore]
  fn atlas() {
    let (width, height, zoom) = (COLUMNS * PIXELS, ROWS * PIXELS, 4);
    let pixels: Vec<u8> = (0..width * zoom * height * zoom)
      .flat_map(|index| {
        let (x, y) = (index % (width * zoom) / zoom, index / (width * zoom) / zoom);
        let slot = (y / PIXELS) * COLUMNS + x / PIXELS;
        let [r, g, b, a] = match slot < PAINTED {
          true => painted(slot, x % PIXELS, y % PIXELS).color,
          false => [0.0; 4]
        };
        let backdrop = if (x / 4 + y / 4) % 2 == 0 { 0.35 } else { 0.47 };
        [r, g, b].map(|channel| to_srgb(channel * a + backdrop * (1.0 - a)))
      })
      .collect();
    let mut file = format!("P6 {} {} 255\n", width * zoom, height * zoom).into_bytes();
    file.extend(pixels);
    std::fs::write("screenshots/atlas.ppm", file).unwrap()
  }

  #[test]
  #[ignore]
  fn swatches() {
    let tiles = [
      Tile::Planks,
      Tile::GreywackeCobble,
      Tile::GraniteCobble,
      Tile::GreywackeBricks,
      Tile::AncientBrick,
      Tile::MossyBrick,
      Tile::Greywacke
    ];
    let (zoom, repeat) = (6, 3);
    let side = PIXELS * repeat * zoom;
    let width = side * tiles.len() as u32;
    let pixels: Vec<u8> = (0..width * side)
      .flat_map(|index| {
        let (x, y) = (index % width, index / width);
        let tile = tiles[(x / side) as usize];
        let (u, v) = ((x % side) / zoom % PIXELS, y / zoom % PIXELS);
        let [r, g, b, _] = paint(tile, u, v).color;
        [r, g, b].map(to_srgb)
      })
      .collect();
    let mut file = format!("P6 {width} {side} 255\n").into_bytes();
    file.extend(pixels);
    std::fs::write("screenshots/swatches.ppm", file).unwrap()
  }

  #[test]
  #[ignore]
  fn icon_sheet() {
    let image = icons();
    let (width, height, zoom) = (image.width(), image.height(), 3);
    let data = image.data.unwrap();
    let pixels: Vec<u8> = (0..width * zoom * height * zoom)
      .flat_map(|index| {
        let (x, y) = (index % (width * zoom) / zoom, index / (width * zoom) / zoom);
        let at = ((y * width + x) * 4) as usize;
        let alpha = data[at + 3] as f32 / 255.0;
        let backdrop = if (x / 4 + y / 4) % 2 == 0 { 90.0 } else { 120.0 };
        [0, 1, 2].map(|channel| {
          (data[at + channel] as f32 * alpha + backdrop * (1.0 - alpha)) as u8
        })
      })
      .collect();
    let mut file = format!("P6 {} {} 255\n", width * zoom, height * zoom).into_bytes();
    file.extend(pixels);
    std::fs::write("screenshots/icons.ppm", file).unwrap()
  }
}
