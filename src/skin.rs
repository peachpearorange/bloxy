use {bevy::{asset::RenderAssetUsages,
            image::ImageSampler,
            mesh::{Indices, PrimitiveTopology},
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}},
     serde::{Deserialize, Serialize}};

pub const WIDE: u32 = 64;
pub const TALL: u32 = 32;
pub const PX: f32 = 0.0575;

pub const PALETTE: [[u8; 3]; 32] = [
  [20, 18, 24],
  [62, 58, 66],
  [118, 114, 122],
  [178, 174, 180],
  [240, 238, 234],
  [255, 214, 186],
  [232, 176, 136],
  [196, 136, 98],
  [148, 96, 64],
  [96, 60, 40],
  [60, 38, 26],
  [232, 200, 96],
  [214, 150, 44],
  [178, 92, 34],
  [214, 64, 52],
  [146, 34, 44],
  [240, 120, 160],
  [168, 70, 160],
  [96, 50, 128],
  [52, 54, 120],
  [52, 92, 186],
  [82, 160, 226],
  [150, 214, 240],
  [44, 140, 132],
  [36, 92, 70],
  [70, 150, 60],
  [140, 196, 70],
  [210, 230, 120],
  [110, 80, 52],
  [70, 82, 98],
  [40, 48, 70],
  [156, 40, 28]
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
  Head,
  Body,
  Arm,
  Leg
}

impl Part {
  pub const ALL: [Part; 4] = [Part::Head, Part::Body, Part::Arm, Part::Leg];

  pub fn size(self) -> UVec3 {
    match self {
      Part::Head => UVec3::new(8, 8, 8),
      Part::Body => UVec3::new(8, 12, 4),
      Part::Arm | Part::Leg => UVec3::new(4, 12, 4)
    }
  }

  pub fn origin(self) -> UVec2 {
    match self {
      Part::Head => UVec2::new(0, 0),
      Part::Body => UVec2::new(16, 16),
      Part::Arm => UVec2::new(40, 16),
      Part::Leg => UVec2::new(0, 16)
    }
  }

  pub fn faces(self) -> [(Face, URect); 6] {
    let (UVec3 { x: w, y: h, z: d }, UVec2 { x: u, y: v }) = (self.size(), self.origin());
    let rect = |x, y, wide, tall| URect::new(x, y, x + wide, y + tall);
    [
      (Face::Top, rect(u + d, v, w, d)),
      (Face::Bottom, rect(u + d + w, v, w, d)),
      (Face::Right, rect(u, v + d, d, h)),
      (Face::Front, rect(u + d, v + d, w, h)),
      (Face::Left, rect(u + d + w, v + d, d, h)),
      (Face::Back, rect(u + 2 * d + w, v + d, w, h))
    ]
  }

  pub fn mesh(self) -> Mesh {
    let size = self.size().as_vec3() * PX;
    let (positions, uvs, normals) = self.faces().into_iter().fold(
      (Vec::new(), Vec::new(), Vec::new()),
      |(mut positions, mut uvs, mut normals), (face, rect)| {
        let (normal, across, down) = face.axes();
        let extent = |axis: Vec3| axis.abs().dot(size);
        let centre = normal * extent(normal) / 2.0;
        [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)].into_iter().for_each(
          |(s, t)| {
            positions.push(
              (centre
                + across * (s - 0.5) * extent(across)
                + down * (t - 0.5) * extent(down))
              .to_array()
            );
            let corner = rect.min.as_vec2() + Vec2::new(s, t) * rect.size().as_vec2();
            uvs.push((corner / Vec2::new(WIDE as f32, TALL as f32)).to_array());
            normals.push(normal.to_array())
          }
        );
        (positions, uvs, normals)
      }
    );
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
      .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
      .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
      .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
      .with_inserted_indices(Indices::U32(
        (0..6u32).flat_map(|f| [0, 1, 2, 0, 2, 3].map(|i| f * 4 + i)).collect()
      ))
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
  Top,
  Bottom,
  Right,
  Front,
  Left,
  Back
}

impl Face {
  pub fn axes(self) -> (Vec3, Vec3, Vec3) {
    match self {
      Face::Top => (Vec3::Y, Vec3::NEG_X, Vec3::NEG_Z),
      Face::Bottom => (Vec3::NEG_Y, Vec3::NEG_X, Vec3::Z),
      Face::Right => (Vec3::X, Vec3::NEG_Z, Vec3::NEG_Y),
      Face::Front => (Vec3::NEG_Z, Vec3::NEG_X, Vec3::NEG_Y),
      Face::Left => (Vec3::NEG_X, Vec3::Z, Vec3::NEG_Y),
      Face::Back => (Vec3::Z, Vec3::X, Vec3::NEG_Y)
    }
  }
}

pub fn used(texel: UVec2) -> bool {
  Part::ALL
    .iter()
    .flat_map(|part| part.faces())
    .any(|(_, rect)| rect.contains(texel) && texel.x < rect.max.x && texel.y < rect.max.y)
}

#[derive(Component, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Skin(pub Vec<u8>);

impl Skin {
  pub fn valid(&self) -> bool {
    self.0.len() == (WIDE * TALL) as usize
      && self.0.iter().all(|&index| (index as usize) < PALETTE.len())
  }

  pub fn get(&self, texel: UVec2) -> u8 { self.0[(texel.y * WIDE + texel.x) as usize] }

  pub fn set(&mut self, texel: UVec2, index: u8) {
    self.0[(texel.y * WIDE + texel.x) as usize] = index
  }

  pub fn fresh(seed: u64) -> Skin {
    let pick = |salt: u64, choices: &[u8]| {
      let mixed = (seed ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .wrapping_mul(0xbf58_476d_1ce4_e5b9)
        .rotate_left(29)
        .wrapping_mul(0x94d0_49bb_1331_11eb);
      choices[(mixed >> 33) as usize % choices.len()]
    };
    let skin = pick(1, &[5, 6, 7, 8, 9]);
    let hair = pick(2, &[0, 9, 10, 11, 13, 1, 28]);
    let eyes = pick(3, &[20, 9, 24, 0]);
    let shirt = pick(4, &[14, 15, 17, 19, 20, 21, 23, 24, 25, 12, 2, 29]);
    let trousers = pick(5, &[30, 19, 1, 28, 9, 29]);
    let shoes = pick(6, &[0, 10, 1, 9]);
    let mut painted = Skin(vec![0; (WIDE * TALL) as usize]);
    Part::ALL.iter().for_each(|&part| {
      part.faces().into_iter().for_each(|(face, rect)| {
        (rect.min.y..rect.max.y).for_each(|y| {
          (rect.min.x..rect.max.x).for_each(|x| {
            let (col, row) = (x - rect.min.x, y - rect.min.y);
            let colour = match (part, face) {
              (Part::Head, Face::Top) => hair,
              (Part::Head, Face::Bottom) => skin,
              (Part::Head, Face::Back) => match row < 6 {
                true => hair,
                false => skin
              },
              (Part::Head, Face::Front) => match (row, col) {
                (0..=1, _) | (2, 0 | 7) => hair,
                (4, 1 | 6) => 4,
                (4, 2 | 5) => eyes,
                (6, 3..=4) => (skin + 2).min(10),
                _ => skin
              },
              (Part::Head, _) => match row < 3 || (row < 6 && col > 4) {
                true => hair,
                false => skin
              },
              (Part::Body, Face::Top | Face::Bottom) => shirt,
              (Part::Body, _) => match row {
                10 => 0,
                11 => trousers,
                _ => shirt
              },
              (Part::Arm, Face::Top) => shirt,
              (Part::Arm, Face::Bottom) => skin,
              (Part::Arm, _) => match row < 4 {
                true => shirt,
                false => skin
              },
              (Part::Leg, Face::Top) => trousers,
              (Part::Leg, Face::Bottom) => shoes,
              (Part::Leg, _) => match row < 10 {
                true => trousers,
                false => shoes
              }
            };
            painted.set(UVec2::new(x, y), colour)
          })
        })
      })
    });
    painted
  }

  pub fn pixels(&self, backdrop: bool) -> Vec<u8> {
    (0..TALL)
      .flat_map(|y| (0..WIDE).map(move |x| UVec2::new(x, y)))
      .flat_map(|texel| match backdrop && !used(texel) {
        true => {
          let shade = if (texel.x + texel.y) % 2 == 0 { 34 } else { 40 };
          [shade, shade, shade + 4, 255]
        }
        false => {
          let [r, g, b] = PALETTE[self.get(texel) as usize];
          [r, g, b, 255]
        }
      })
      .collect()
  }

  pub fn image(&self, backdrop: bool) -> Image {
    let mut image = Image::new(
      Extent3d { width: WIDE, height: TALL, depth_or_array_layers: 1 },
      TextureDimension::D2,
      self.pixels(backdrop),
      TextureFormat::Rgba8UnormSrgb,
      RenderAssetUsages::default()
    );
    image.sampler = ImageSampler::nearest();
    image
  }
}
