use {crate::{block::Tile,
             noise::{hash, unit}},
     bevy::{asset::RenderAssetUsages,
            image::{ImageAddressMode, ImageFilterMode, ImageSampler,
                    ImageSamplerDescriptor},
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}}};

pub const PIXELS: u32 = 16;
pub const COLUMNS: u32 = 8;
pub const ROWS: u32 = 6;
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

fn speck(tile: u32, x: u32, y: u32, salt: u32) -> f32 {
  unit(salt.wrapping_add(tile * 101), x as i32, y as i32, 0)
}

fn grain(tile: u32, x: u32, y: u32, salt: u32) -> f32 {
  0.86 + speck(tile, x, y, salt) * 0.28
}

fn stone(x: u32, y: u32) -> Texel {
  let crack = speck(0, x / 2 + y, y / 3, 7) < 0.08;
  Texel::rgb(0.47, 0.47, 0.48).scaled(grain(0, x, y, 1) * if crack { 0.78 } else { 1.0 })
}

fn cell_edge(x: u32, y: u32, size: u32, salt: u32) -> (f32, u32) {
  let point = |cx: i32, cy: i32| {
    let wrap = |c: i32| c.rem_euclid((PIXELS / size) as i32);
    let jitter = hash(salt, wrap(cx), wrap(cy), 1);
    (
      cx as f32 * size as f32 + (jitter & 0xFF) as f32 / 255.0 * size as f32,
      cy as f32 * size as f32 + (jitter >> 8 & 0xFF) as f32 / 255.0 * size as f32
    )
  };
  let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
  let (cx, cy) = ((x / size) as i32, (y / size) as i32);
  let mut distances = (-1..=1)
    .flat_map(|dx| (-1..=1).map(move |dy| (dx, dy)))
    .map(|(dx, dy)| {
      let (sx, sy) = point(cx + dx, cy + dy);
      let wrap = |c: i32| c.rem_euclid((PIXELS / size) as i32);
      (
        ((sx - px).powi(2) + (sy - py).powi(2)).sqrt(),
        hash(salt, wrap(cx + dx), wrap(cy + dy), 2)
      )
    })
    .collect::<Vec<_>>();
  distances.sort_by(|a, b| a.0.total_cmp(&b.0));
  (distances[1].0 - distances[0].0, distances[0].1)
}

fn dirt(x: u32, y: u32) -> Texel {
  let pebble = speck(2, x, y, 3) < 0.07;
  Texel::rgb(0.42, 0.29, 0.19).scaled(grain(2, x, y, 2) * if pebble { 1.25 } else { 1.0 })
}

fn grass(x: u32, y: u32) -> Texel {
  Texel::rgb(0.27, 0.47, 0.19).scaled(0.82 + speck(3, x, y, 4) * 0.36)
}

fn snow(x: u32, y: u32) -> Texel {
  Texel::rgb(0.92, 0.95, 0.99).scaled(0.95 + speck(8, x, y, 5) * 0.06)
}

fn leaves(x: u32, y: u32, tile: u32, color: [f32; 3], holes: f32) -> Texel {
  let hole = speck(tile, x, y, 20) < holes;
  let tone = 0.65 + speck(tile, x, y, 21) * 0.5;
  Texel::rgb(color[0], color[1], color[2]).scaled(tone).alpha(if hole {
    0.0
  } else {
    1.0
  })
}

fn mycelium(x: u32, y: u32) -> Texel {
  let spore = speck(30, x, y, 27) < 0.12;
  Texel::rgb(0.44, 0.37, 0.46).scaled(match spore {
    true => 1.35,
    false => 0.85 + speck(30, x, y, 28) * 0.25
  })
}

fn waystone(x: u32, y: u32, tile: u32) -> Texel {
  Texel::rgb(0.2, 0.22, 0.28).scaled(0.9 + speck(tile, x / 2, y / 2, 29) * 0.18)
}

fn rune() -> Texel { Texel::rgb(0.3, 0.85, 1.0).glowing(0.7) }

fn churn(tile: u32, x: u32, y: u32) -> f32 {
  let blob = |scale: u32, salt: u32| speck(tile, x / scale, (y + x / 3) / scale, salt);
  (blob(4, 46) + blob(2, 47) + speck(tile, x, y, 48) * 0.5) / 2.5
}

fn ore(x: u32, y: u32, tile: u32, color: [f32; 3], glow: f32) -> Texel {
  let (edge, cell) = cell_edge(x, y, 4, 40 + tile);
  let spot = cell % 3 == 0 && edge > 0.9;
  match spot {
    true => {
      let shade = 0.8 + speck(tile, x, y, 9) * 0.4;
      Texel::rgb(color[0], color[1], color[2]).scaled(shade).glowing(glow)
    }
    false => stone(x, y)
  }
}

pub fn paint(tile: Tile, x: u32, y: u32) -> Texel {
  let index = tile.index();
  match tile {
    Tile::Stone => stone(x, y),
    Tile::Cobblestone => {
      let (edge, cell) = cell_edge(x, y, 5, 11);
      let tone = 0.38 + (cell % 7) as f32 * 0.03;
      match edge < 0.9 {
        true => Texel::rgb(0.22, 0.22, 0.23),
        false => Texel::rgb(tone, tone, tone * 1.02).scaled(grain(index, x, y, 6))
      }
    }
    Tile::Dirt => dirt(x, y),
    Tile::GrassTop => grass(x, y),
    Tile::GrassSide => {
      let fringe = 3 + (speck(index, x, 0, 12) * 2.5) as u32;
      match y < fringe {
        true => grass(x, y),
        false => dirt(x, y)
      }
    }
    Tile::Sand => {
      Texel::rgb(0.86, 0.79, 0.56).scaled(0.9 + speck(index, x, y, 13) * 0.16)
    }
    Tile::Gravel => {
      let (edge, cell) = cell_edge(x, y, 3, 14);
      let tone = 0.35 + (cell % 5) as f32 * 0.07;
      Texel::rgb(tone, tone * 0.97, tone * 0.95).scaled(if edge < 0.6 {
        0.7
      } else {
        1.0
      })
    }
    Tile::Clay => {
      Texel::rgb(0.6, 0.63, 0.7).scaled(0.94 + speck(index, x, y / 2, 15) * 0.1)
    }
    Tile::Snow => snow(x, y),
    Tile::SnowSide => {
      let fringe = 3 + (speck(index, x, 0, 16) * 3.0) as u32;
      match y < fringe {
        true => snow(x, y),
        false => dirt(x, y)
      }
    }
    Tile::Bedrock => {
      let tone = 0.12 + speck(index, x / 2, y / 2, 17) * 0.25;
      Texel::rgb(tone, tone, tone)
    }
    Tile::LogSide => {
      let ridge = speck(index, x, y / 4, 18) * 0.25 + if x % 4 == 0 { 0.7 } else { 1.0 };
      Texel::rgb(0.36, 0.26, 0.16).scaled(ridge)
    }
    Tile::LogTop => {
      let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
      let ring = (dx * dx + dy * dy).sqrt();
      match ring > 6.8 {
        true => Texel::rgb(0.36, 0.26, 0.16).scaled(grain(index, x, y, 19)),
        false => Texel::rgb(0.66, 0.52, 0.33).scaled(if (ring as u32) % 2 == 0 {
          0.85
        } else {
          1.0
        })
      }
    }
    Tile::Leaves => leaves(x, y, index, [0.2, 0.45, 0.16], 0.16),
    Tile::Planks => {
      let seam = y % 4 == 3 || (x + (y / 4) * 5) % 16 == 0;
      let tone =
        0.9 + speck(index, x / 3, y / 4, 22) * 0.15 + speck(index, x, y, 23) * 0.06;
      Texel::rgb(0.66, 0.5, 0.3).scaled(if seam { 0.62 } else { tone })
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
    Tile::Water => {
      let ripple = speck(index, (x + y / 2) / 3, y / 2, 24) * 0.15;
      Texel::rgb(0.16, 0.34, 0.62).scaled(0.9 + ripple).alpha(0.72)
    }
    Tile::CoalOre => ore(x, y, index, [0.08, 0.08, 0.09], 0.0),
    Tile::IronOre => ore(x, y, index, [0.78, 0.6, 0.5], 0.0),
    Tile::CopperOre => ore(x, y, index, [0.85, 0.48, 0.23], 0.0),
    Tile::TinOre => ore(x, y, index, [0.82, 0.84, 0.86], 0.0),
    Tile::GoldOre => ore(x, y, index, [0.98, 0.82, 0.25], 0.35),
    Tile::DiamondOre => ore(x, y, index, [0.45, 0.93, 0.95], 1.0),
    Tile::Lamp => {
      let rim = x == 0 || y == 0 || x == PIXELS - 1 || y == PIXELS - 1;
      let lattice = x % 5 == 0 || y % 5 == 0;
      match (rim, lattice) {
        (true, _) => Texel::rgb(0.3, 0.24, 0.16),
        (false, true) => Texel::rgb(0.95, 0.75, 0.4).glowing(1.2),
        _ => Texel::rgb(1.0, 0.86, 0.55)
          .scaled(0.92 + speck(index, x, y, 25) * 0.08)
          .glowing(2.0)
      }
    }
    Tile::Bricks => {
      let course = y / 4;
      let mortar = y % 4 == 3 || (x + course % 2 * 4) % 8 == 7;
      match mortar {
        true => Texel::rgb(0.62, 0.6, 0.56),
        false => Texel::rgb(0.6, 0.27, 0.2).scaled(grain(index, x, y, 26))
      }
    }
    Tile::BirchSide => {
      let mark = speck(index, x / 3, y, 31) < 0.14 && speck(index, x, y, 32) < 0.8;
      match mark {
        true => Texel::rgb(0.12, 0.11, 0.1),
        false => Texel::rgb(0.86, 0.85, 0.8).scaled(0.92 + speck(index, x, y, 33) * 0.1)
      }
    }
    Tile::BirchLeaves => leaves(x, y, index, [0.42, 0.6, 0.22], 0.18),
    Tile::SpruceSide => {
      let ridge = speck(index, x, y / 5, 34) * 0.3 + if x % 3 == 0 { 0.65 } else { 0.9 };
      Texel::rgb(0.27, 0.18, 0.11).scaled(ridge)
    }
    Tile::SpruceLeaves => leaves(x, y, index, [0.1, 0.27, 0.18], 0.1),
    Tile::PalmSide => {
      let ring = (y + x / 8) % 4 == 0;
      Texel::rgb(0.58, 0.47, 0.3).scaled(match ring {
        true => 0.7,
        false => 0.92 + speck(index, x, y, 35) * 0.14
      })
    }
    Tile::PalmLeaves => {
      let frond = leaves(x, y, index, [0.3, 0.58, 0.14], 0.06);
      let slit = (x + y) % 4 == 0 && speck(index, x, y, 36) < 0.75;
      frond.alpha(if slit { 0.0 } else { frond.color[3] })
    }
    Tile::MyceliumTop => mycelium(x, y),
    Tile::MyceliumSide => {
      let fringe = 2 + (speck(index, x, 0, 37) * 3.0) as u32;
      match y < fringe {
        true => mycelium(x, y),
        false => dirt(x, y)
      }
    }
    Tile::Stem => Texel::rgb(0.86, 0.83, 0.74).scaled(
      0.9 + speck(index, x, y / 4, 38) * 0.12 - if x % 5 == 2 { 0.06 } else { 0.0 }
    ),
    Tile::RedCap => {
      let (edge, cell) = cell_edge(x, y, 8, 39);
      match edge > 2.2 && cell % 3 != 0 {
        true => Texel::rgb(0.93, 0.9, 0.86),
        false => Texel::rgb(0.74, 0.12, 0.1).scaled(grain(index, x, y, 40))
      }
    }
    Tile::BrownCap => Texel::rgb(0.55, 0.4, 0.27).scaled(
      grain(index, x, y, 41)
        * if speck(index, x / 2, y / 2, 42) < 0.15 { 0.85 } else { 1.0 }
    ),
    Tile::Pores => {
      let gill = x % 2 == 0;
      Texel::rgb(0.8, 0.74, 0.62).scaled(if gill { 0.82 } else { 1.0 })
    }
    Tile::Basalt => {
      let joint = (x + y / 6 * 3) % 5 == 0 || y % 6 == 5;
      Texel::rgb(0.2, 0.2, 0.22).scaled(match joint {
        true => 0.6,
        false => 0.85 + speck(index, x, y, 43) * 0.3
      })
    }
    Tile::Lava => {
      let swirl = churn(index, x, y);
      match swirl {
        crust if crust < 0.18 => Texel::rgb(0.28, 0.08, 0.03).glowing(0.05),
        bright if bright > 0.7 => Texel::rgb(1.0, 0.72, 0.25).glowing(0.35),
        _ => Texel::rgb(0.95, 0.35, 0.05).glowing(0.22)
      }
    }
    Tile::Ice => {
      let crack = (x * 3 + y * 5) % 17 == 0 || speck(index, x, y / 3, 44) < 0.05;
      Texel::rgb(0.64, 0.8, 0.96).scaled(match crack {
        true => 1.15,
        false => 0.92 + speck(index, x / 2, y, 45) * 0.1
      })
    }
    Tile::WaystoneSide => {
      let (dx, dy) = ((x as f32 - 7.5).abs(), (y as f32 - 7.5).abs());
      let diamond = (4.0..5.5).contains(&(dx + dy));
      let spine = dx < 1.0 && (2.0..14.0).contains(&(y as f32));
      let frame = x == 0 || x == PIXELS - 1;
      match (diamond || spine, frame) {
        (true, _) => rune(),
        (_, true) => waystone(x, y, index).scaled(1.5),
        _ => waystone(x, y, index)
      }
    }
    Tile::WaystoneTop => {
      let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
      let ring = (dx * dx + dy * dy).sqrt();
      match (4.0..5.2).contains(&ring) || ring < 1.6 {
        true => rune(),
        false => waystone(x, y, index).scaled(1.2)
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
