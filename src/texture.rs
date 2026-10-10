use {crate::{block::{Block, Look, Tile},
             model},
     bevy::{asset::RenderAssetUsages,
            image::{ImageAddressMode, ImageFilterMode, ImageSampler,
                    ImageSamplerDescriptor},
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}}};

pub const PIXELS: u32 = 16;
pub const COLUMNS: u32 = 8;
pub const ROWS: u32 = 12;
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

fn mortar(x: u32, y: u32) -> bool { shade_of(&COBBLE, x, y) == Some(0) }

fn wrapped(x: i32, y: i32) -> (u32, u32) {
  (x.rem_euclid(PIXELS as i32) as u32, y.rem_euclid(PIXELS as i32) as u32)
}

static PEBBLES_OF_COBBLE: std::sync::LazyLock<Vec<u32>> =
  std::sync::LazyLock::new(|| {
    let size = PIXELS * PIXELS;
    let mut labels = vec![0u32; size as usize];
    (0..size).for_each(|start| {
      if !mortar(start % PIXELS, start / PIXELS) && labels[start as usize] == 0 {
        labels[start as usize] = start + 1;
        let neighbours = |at: u32| {
          let (x, y) = ((at % PIXELS) as i32, (at / PIXELS) as i32);
          [(1, 0), (-1, 0), (0, 1), (0, -1)].map(|(dx, dy)| {
            let (nx, ny) = wrapped(x + dx, y + dy);
            ny * PIXELS + nx
          })
        };
        std::iter::successors(Some(vec![start]), |frontier: &Vec<u32>| {
          let next: Vec<u32> = frontier
            .iter()
            .flat_map(|&at| neighbours(at))
            .filter(|&next| {
              let fresh =
                !mortar(next % PIXELS, next / PIXELS) && labels[next as usize] == 0;
              if fresh {
                labels[next as usize] = start + 1
              }
              fresh
            })
            .collect();
          (!next.is_empty()).then_some(next)
        })
        .count();
      }
    });
    labels
  });

static CRACKS_OF_COBBLE: std::sync::LazyLock<Vec<bool>> =
  std::sync::LazyLock::new(|| {
    let pebble = |x: i32, y: i32| {
      let (x, y) = wrapped(x, y);
      PEBBLES_OF_COBBLE[(y * PIXELS + x) as usize]
    };
    let sizes = (0..PIXELS * PIXELS).fold(
      std::collections::BTreeMap::<u32, Vec<(i32, i32)>>::new(),
      |mut sizes, at| {
        let (x, y) = ((at % PIXELS) as i32, (at / PIXELS) as i32);
        if pebble(x, y) != 0 {
          sizes.entry(pebble(x, y)).or_default().push((x, y))
        }
        sizes
      }
    );
    const TURNS: [(i32, i32); 8] =
      [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
    sizes
      .iter()
      .filter(|(label, cells)| {
        cells.len() >= 14 && crate::noise::unit(0xC4A, **label as i32, 0, 0) < 0.6
      })
      .map(|(&label, cells)| {
        let edge: Vec<(i32, i32, usize)> = cells
          .iter()
          .filter_map(|&(x, y)| {
            [4, 6, 0, 2]
              .into_iter()
              .find(|&turn| {
                let (dx, dy) = TURNS[turn];
                pebble(x + dx, y + dy) == 0
              })
              .map(|outward| (x, y, (outward + 4) % 8))
          })
          .collect();
        let roll = |salt: i32| crate::noise::hash(0xC4B, label as i32, salt, 0);
        let (x, y, inward) = edge[roll(0) as usize % edge.len().max(1)];
        let length = 3 + roll(1) % 4;
        (0..length)
          .scan((x, y, inward), move |(x, y, heading), step| {
            let here = (*x, *y);
            let veer = [0, 1, 7, 0][(roll(step as i32 + 2) % 4) as usize];
            *heading = (*heading + veer) % 8;
            let (dx, dy) = TURNS[*heading];
            *x += dx;
            *y += dy;
            Some(here)
          })
          .take_while(move |&(x, y)| pebble(x, y) == label)
          .collect::<Vec<_>>()
      })
      .filter(|crack| crack.len() >= 3)
      .flatten()
      .fold(vec![false; (PIXELS * PIXELS) as usize], |mut cracks, (x, y)| {
        let (x, y) = wrapped(x, y);
        cracks[(y * PIXELS + x) as usize] = true;
        cracks
      })
  });

fn cracked(x: i32, y: i32) -> bool {
  let (x, y) = wrapped(x, y);
  CRACKS_OF_COBBLE[(y * PIXELS + x) as usize]
}

const PEBBLE_TINTS: [[f32; 3]; 4] =
  [[1.0, 1.0, 1.0], [1.02, 1.0, 0.98], [0.98, 0.99, 1.01], [0.95, 0.95, 0.95]];

static DOMES_OF_COBBLE: std::sync::LazyLock<Vec<Vec2>> = std::sync::LazyLock::new(|| {
  let pebble = |at: u32| PEBBLES_OF_COBBLE[at as usize];
  let unwrapped = |from: u32, at: u32| {
    let step = |a: u32, b: u32| (b as i32 - a as i32 + 8).rem_euclid(16) - 8;
    Vec2::new(
      (from % PIXELS) as f32 + step(from % PIXELS, at % PIXELS) as f32,
      (from / PIXELS) as f32 + step(from / PIXELS, at / PIXELS) as f32
    )
  };
  (0..PIXELS * PIXELS)
    .map(|at| {
      let label = pebble(at);
      let members: Vec<u32> = (0..PIXELS * PIXELS)
        .filter(|&other| label != 0 && pebble(other) == label)
        .collect();
      let centre = members.iter().map(|&other| unwrapped(at, other)).sum::<Vec2>()
        / members.len().max(1) as f32;
      let radius = (members.len() as f32 / std::f32::consts::PI).sqrt().max(1.0) + 0.5;
      (unwrapped(at, at) - centre) / radius
    })
    .collect()
});

fn cobble(base: [f32; 3], x: u32, y: u32) -> Texel {
  let pebble = PEBBLES_OF_COBBLE[(y * PIXELS + x) as usize];
  let near = |dx: i32, dy: i32| {
    let (nx, ny) = wrapped(x as i32 + dx, y as i32 + dy);
    mortar(nx, ny)
  };
  let tint = PEBBLE_TINTS[(crate::noise::hash(0xC0B, pebble as i32, 0, 0) % 4) as usize];
  let bright = 0.92 + crate::noise::unit(0xC0C, pebble as i32, 0, 0) * 0.16;
  let dome = DOMES_OF_COBBLE[(y * PIXELS + x) as usize];
  let normal =
    dome.extend((1.0 - dome.length_squared()).max(0.0).sqrt()).normalize_or_zero();
  let sunward = Vec3::new(-0.5, -0.6, 0.62).normalize();
  let edged = [(1, 0), (0, 1)].into_iter().any(|(dx, dy)| near(dx, dy));
  let light = match pebble {
    0 => 0.5 + lattice(0xC0D, 8, x, y) * 0.1,
    _ => {
      0.66 + normal.dot(sunward).max(0.0) * 0.45 + (lattice(0xC0E, 8, x, y) - 0.5) * 0.1
        - if edged { 0.08 } else { 0.0 }
    }
  };
  let shade = shade_of(&COBBLE, x, y).map_or(0.0, |shade| (shade as f32 - 2.0) * 0.03);
  let crack = match (cracked(x as i32, y as i32), cracked(x as i32, y as i32 - 1)) {
    (true, _) => -0.42,
    (false, true) => 0.1,
    _ => 0.0
  };
  let [r, g, b] = std::array::from_fn(|channel| {
    base[channel]
      * match pebble {
        0 => light,
        _ => tint[channel] * bright * (light + shade + crack)
      }
  });
  Texel::rgb(r, g, b)
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
  "....00000000....",
  "...0........0...",
  "..0..........0..",
  "..000000000000..",
  "..011111111110..",
  "..000000000000..",
  "..032222222210..",
  "...0322222210...",
  "...0322222210...",
  "...0322222210...",
  "....03222210....",
  "....03222210....",
  "....00000000....",
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
  "0000000000000000",
  "0333333333333330",
  "0322222222222220",
  "0201010110101120",
  "0222222222222220",
  "0210110101011020",
  "0222222222222220",
  "0211111111111110",
  "0000000110000000",
  ".......120......",
  ".......120......",
  ".......120......",
  ".......120......",
  ".......000......"
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
  "......1112......",
  "....11002321....",
  "...1000012221...",
  "..100100001321..",
  "..1000000012211.",
  ".10010000001221.",
  ".10000100000121.",
  ".100000000100121",
  "..1000010000011.",
  "..10010000001...",
  "...110000011....",
  ".....111111.....",
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
  Texel::rgb(1.0, 0.55 + heat * 0.3, 0.15 + heat * 0.25).glowing(0.08 + heat * 0.04)
}

fn rune() -> Texel { Texel::rgb(0.85, 0.97, 1.0).glowing(0.9) }

fn model_icon(block: Block, x: u32, y: u32) -> Texel {
  model::icon(block, x, y).map_or(Texel::rgb(0.0, 0.0, 0.0).alpha(0.0), |bit| {
    let Texel { color: [r, g, b, _], glow } = paint(bit.tile, x, y);
    let [tint_r, tint_g, tint_b] = bit.color;
    Texel { color: [r * tint_r, g * tint_g, b * tint_b, 1.0], glow }
  })
}

pub fn paint(tile: Tile, x: u32, y: u32) -> Texel {
  match tile {
    Tile::Stone => smooth(&GREY, x, y),
    Tile::Cobblestone => cobble([0.48, 0.48, 0.5], x, y),
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

pub fn albedo() -> Image { image(atlas_levels(|tile, x, y| paint(tile, x, y).color)) }

pub fn glow() -> Image {
  image(atlas_levels(|tile, x, y| {
    let [r, g, b] = paint(tile, x, y).glow;
    [r, g, b, 1.0]
  }))
}

pub const ICON: u32 = 32;
pub const ICON_COLUMNS: u32 = 10;

fn iso(block: Block, x: u32, y: u32) -> [f32; 4] {
  let [top, side, _] = block.tiles();
  let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
  let half = ICON as f32 / 2.0;
  let faces = [
    (
      top,
      Vec2::new(half, 1.0),
      Vec2::new(half - 1.0, 7.5),
      Vec2::new(1.0 - half, 7.5),
      1.0
    ),
    (side, Vec2::new(1.0, 8.5), Vec2::new(half - 1.0, 7.5), Vec2::new(0.0, 15.0), 0.8),
    (
      side,
      Vec2::new(half, 16.0),
      Vec2::new(half - 1.0, -7.5),
      Vec2::new(0.0, 15.0),
      0.62
    )
  ];
  faces
    .into_iter()
    .find_map(|(tile, origin, across, down, shade)| {
      let local = Mat2::from_cols(across, down).inverse() * (at - origin);
      ((0.0..1.0).contains(&local.x) && (0.0..1.0).contains(&local.y))
        .then(|| {
          let [r, g, b, a] = paint(
            tile,
            (local.x * PIXELS as f32) as u32,
            (local.y * PIXELS as f32) as u32
          )
          .color;
          [r * shade, g * shade, b * shade, a]
        })
        .filter(|texel| texel[3] > 0.0)
    })
    .unwrap_or([0.0; 4])
}

fn icon_texel(block: Block, x: u32, y: u32) -> [f32; 4] {
  match block.look() {
    Look::Opaque | Look::Cutout | Look::Log if block.item() => iso(block, x, y),
    _ if block.ladder() => model_icon(block, x / 2, y / 2).color,
    _ => paint(block.tiles()[1], x / 2, y / 2).color
  }
}

pub fn icons() -> Image {
  let rows = Block::ALL.len() as u32 / ICON_COLUMNS + 1;
  let (wide, tall) = (ICON_COLUMNS * ICON, rows * ICON);
  let mut image = Image::new(
    Extent3d { width: wide, height: tall, depth_or_array_layers: 1 },
    TextureDimension::D2,
    (0..wide * tall)
      .flat_map(|index| {
        let (x, y) = (index % wide, index / wide);
        let cell = (y / ICON) * ICON_COLUMNS + x / ICON;
        Block::ALL
          .get(cell as usize)
          .map_or([0.0; 4], |&block| icon_texel(block, x % ICON, y % ICON))
          .map(to_srgb)
      })
      .collect(),
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  );
  image.sampler = ImageSampler::nearest();
  image
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
