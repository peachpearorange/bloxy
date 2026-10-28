use {crate::{cage::Cage,
             noise::perlin3,
             sculpt::triangles,
             skin::{Face, PALETTE, Part, Skin, TALL, WIDE}},
     bevy::{asset::{RenderAssetUsages, embedded_asset},
            image::ImageSampler,
            mesh::{Indices, PrimitiveTopology},
            pbr::{ExtendedMaterial, MaterialExtension},
            prelude::*,
            render::render_resource::{AsBindGroup, Extent3d, TextureDimension,
                                      TextureFormat},
            shader::ShaderRef}};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
  Flesh,
  Hair,
  Cloth,
  Leather
}

impl Kind {
  const STEP: u8 = 40;

  fn code(self) -> u8 { self as u8 * Kind::STEP }
}

#[derive(Clone, Copy, PartialEq)]
enum Finish {
  Surface,
  Eyeball
}

struct Bounds {
  centre: Vec3,
  half: Vec3
}

impl Part {
  pub fn pivot(self) -> Vec3 {
    match self {
      Part::Head => Vec3::new(0.0, 1.44, 0.0),
      Part::Body => Vec3::new(0.0, 1.08, 0.0),
      Part::Arm => Vec3::new(0.36, 1.44, 0.0),
      Part::LeftArm => Vec3::new(-0.36, 1.44, 0.0),
      Part::Leg => Vec3::new(0.12, 0.72, 0.0),
      Part::LeftLeg => Vec3::new(-0.12, 0.72, 0.0)
    }
  }

  fn bounds(self) -> Bounds {
    match self.limb() {
      Part::Head => {
        Bounds { centre: Vec3::new(0.0, 0.24, 0.01), half: Vec3::new(0.17, 0.21, 0.19) }
      }
      Part::Body => Bounds { centre: Vec3::ZERO, half: Vec3::new(0.23, 0.36, 0.12) },
      _ => Bounds { centre: Vec3::new(0.0, -0.36, 0.0), half: Vec3::new(0.12, 0.36, 0.12) }
    }
  }

  fn side(self) -> f32 {
    match self {
      Part::LeftArm | Part::LeftLeg => -1.0,
      _ => 1.0
    }
  }

  fn index(self) -> usize { Part::ALL.iter().position(|&part| part == self).unwrap() }

  pub fn texel(self, at: Vec3) -> (Face, UVec2) {
    let Bounds { centre, half } = self.bounds();
    let q = (at - centre) / half;
    let axis = (0..3).fold(0, |most, axis| match q[axis].abs() > q[most].abs() {
      true => axis,
      false => most
    });
    let face = match (axis, q[axis] > 0.0) {
      (0, true) => Face::Right,
      (0, false) => Face::Left,
      (1, true) => Face::Top,
      (1, false) => Face::Bottom,
      (_, true) => Face::Back,
      (_, false) => Face::Front
    };
    let (_, rect) = self.faces().into_iter().find(|&(each, _)| each == face).unwrap();
    let (_, across, down) = face.axes();
    let st = Vec2::new(q.dot(across), q.dot(down)) * 0.5 + 0.5;
    let texel = (st.clamp(Vec2::ZERO, Vec2::splat(0.999)) * rect.size().as_vec2())
      .floor()
      .as_uvec2();
    (face, rect.min + texel)
  }
}

pub struct Features {
  flesh: u8,
  eyes: Vec<(Vec2, [u8; 3])>,
  mouth: Option<Vec2>
}

fn front() -> URect { Part::Head.faces()[3].1 }

fn mode(indices: impl Iterator<Item = u8>) -> u8 {
  let counts = indices.fold([0u32; 32], |mut counts, index| {
    counts[index as usize % 32] += 1;
    counts
  });
  (0..32).max_by_key(|&index| counts[index]).unwrap_or(0) as u8
}

fn eyed(col: u32, row: u32) -> bool { (3..=5).contains(&row) && (1..=6).contains(&col) }

fn mouthed(col: u32, row: u32) -> bool { (6..=7).contains(&row) && (2..=5).contains(&col) }

pub fn features(skin: &Skin) -> Features {
  let rect = front();
  let at = |col: u32, row: u32| skin.get(rect.min + UVec2::new(col, row));
  let flesh = mode((3..8).flat_map(|row| (0..8).map(move |col| (col, row))).filter_map(
    |(col, row)| (!eyed(col, row) && !mouthed(col, row)).then(|| at(col, row))
  ));
  let differs = |col: u32, row: u32| at(col, row) != flesh;
  let eyes = [0..4, 4..8]
    .into_iter()
    .filter_map(|cols| {
      let found: Vec<(u32, u32)> = cols
        .flat_map(|col| (3..=5).map(move |row| (col, row)))
        .filter(|&(col, row)| eyed(col, row) && differs(col, row))
        .collect();
      (!found.is_empty()).then(|| {
        let centre = found
          .iter()
          .map(|&(col, row)| Vec2::new(col as f32 + 0.5, row as f32 + 0.5))
          .sum::<Vec2>()
          / found.len() as f32;
        let iris = found
          .iter()
          .map(|&(col, row)| at(col, row))
          .find(|&index| index != 4)
          .unwrap_or(0);
        (centre, PALETTE[iris as usize])
      })
    })
    .collect();
  let lips: Vec<Vec2> = (2..=5)
    .flat_map(|col| (6..=7).map(move |row| (col, row)))
    .filter(|&(col, row)| differs(col, row))
    .map(|(col, row)| Vec2::new(col as f32 + 0.5, row as f32 + 0.5))
    .collect();
  let mouth = (!lips.is_empty()).then(|| lips.iter().sum::<Vec2>() / lips.len() as f32);
  Features { flesh, eyes, mouth }
}

fn kind(skin: &Skin, features: &Features, part: Part, face: Face, texel: UVec2) -> Kind {
  let index = skin.get(texel);
  let toned = (5..=8).contains(&index) || index == features.flesh;
  let rect = part.faces().into_iter().find(|&(each, _)| each == face).unwrap().1;
  let UVec2 { x: col, y: row } = texel - rect.min;
  let dark = PALETTE[index as usize].iter().map(|&c| c as u32).sum::<u32>() < 330;
  match (part.limb(), face) {
    (Part::Head, Face::Front)
      if (eyed(col, row) || mouthed(col, row)) && index != features.flesh =>
    {
      Kind::Flesh
    }
    (Part::Head, _) if toned => Kind::Flesh,
    (Part::Head, _) => Kind::Hair,
    _ if index == features.flesh => Kind::Flesh,
    (Part::Arm, _) if toned => Kind::Flesh,
    (Part::Leg, Face::Bottom) => Kind::Leather,
    (Part::Leg, Face::Top) => Kind::Cloth,
    (Part::Leg, _) if row >= 10 => Kind::Leather,
    (Part::Body, Face::Top | Face::Bottom) => Kind::Cloth,
    (Part::Body, _) if row == 10 && dark => Kind::Leather,
    _ => Kind::Cloth
  }
}

pub fn surface(skin: &Skin) -> Image {
  let features = features(skin);
  let mut pixels: Vec<u8> = skin.pixels(false);
  for part in Part::ALL {
    for (face, rect) in part.faces() {
      for y in rect.min.y..rect.max.y {
        for x in rect.min.x..rect.max.x {
          let texel = UVec2::new(x, y);
          let kind = kind(skin, &features, part, face, texel);
          let index = ((y * WIDE + x) * 4) as usize;
          let shown = match kind == Kind::Flesh && skin.get(texel) != features.flesh {
            true if part == Part::Head && face == Face::Front => {
              PALETTE[features.flesh as usize]
            }
            _ => PALETTE[skin.get(texel) as usize]
          };
          pixels[index..index + 4]
            .copy_from_slice(&[shown[0], shown[1], shown[2], kind.code()]);
        }
      }
    }
  }
  let mut image = Image::new(
    Extent3d { width: WIDE, height: TALL, depth_or_array_layers: 1 },
    TextureDimension::D2,
    pixels,
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  );
  image.sampler = ImageSampler::nearest();
  image
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct Living {
  #[uniform(100)]
  parts: [Vec4; 19],
  #[texture(101)]
  skin: Handle<Image>
}

impl MaterialExtension for Living {
  fn fragment_shader() -> ShaderRef { "embedded://bloxy/person.wgsl".into() }
}

pub type Flesh = ExtendedMaterial<StandardMaterial, Living>;

fn mouth(features: &Features) -> Vec2 {
  features.mouth.map_or(Vec2::new(0.0, 0.1), |texel| {
    let at = on_face(texel);
    Vec2::new(at.x * 0.3, at.y.clamp(0.07, 0.13))
  })
}

pub fn flesh(worn: &Skin, skin: Handle<Image>) -> Flesh {
  let lips = mouth(&features(worn));
  let parts = Part::ALL.iter().fold([Vec4::ZERO; 19], |mut parts, &part| {
    let Bounds { centre, half } = part.bounds();
    let size = part.size().as_vec3();
    let origin = part.origin().as_vec2();
    let index = part.index();
    parts[index * 3] = centre.extend(size.x);
    parts[index * 3 + 1] = half.extend(size.y);
    parts[index * 3 + 2] = Vec4::new(origin.x, origin.y, size.z, part.side());
    parts[18] = Vec4::new(lips.x, lips.y, 0.0, 0.0);
    parts
  });
  ExtendedMaterial {
    base: StandardMaterial {
      perceptual_roughness: 0.6,
      reflectance: 0.4,
      anisotropy_strength: 0.01,
      ..default()
    },
    extension: Living { parts, skin }
  }
}

fn packed(tint: [u8; 3]) -> u32 {
  let [r, g, b] = tint.map(|channel| (channel >> 3) as u32);
  r + 32 * g + 1024 * b
}

fn code(part: Part, finish: Finish, tint: [u8; 3]) -> f32 {
  (part.index() as u32 + 8 * finish as u32 + 32 * packed(tint)) as f32
}

fn assembled(pieces: &[(Cage, f32, f32)]) -> Mesh {
  let (positions, colors, near, far, indices) = pieces.iter().fold(
    (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()),
    |(mut positions, mut colors, mut near, mut far, mut indices): (
      Vec<[f32; 3]>,
      Vec<[f32; 4]>,
      Vec<[f32; 2]>,
      Vec<[f32; 2]>,
      Vec<u32>
    ),
     (cage, code, lid)| {
      let first = positions.len() as u32;
      for (point, normal) in cage.points.iter().zip(cage.vertex_normals()) {
        positions.push(point.to_array());
        colors.push(normal.extend(*code).to_array());
        near.push([point.x, point.y]);
        far.push([point.z, *lid])
      }
      indices.extend(triangles(cage).into_iter().flatten().map(|corner| first + corner));
      (positions, colors, near, far, indices)
    }
  );
  let count = positions.len();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[1.0f32, 0.0, 0.0, 1.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, near)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, far)
    .with_inserted_indices(Indices::U32(indices))
}

fn surface_at(cage: &Cage, x: f32, y: f32) -> f32 {
  cage
    .points
    .iter()
    .filter(|at| at.z < 0.0)
    .min_by(|one, other| {
      Vec2::new(one.x - x, one.y - y)
        .length()
        .total_cmp(&Vec2::new(other.x - x, other.y - y).length())
    })
    .map_or(-0.19, |at| at.z)
}

fn on_face(texel: Vec2) -> Vec2 {
  let Bounds { centre, half } = Part::Head.bounds();
  let st = texel / 8.0;
  Vec2::new(centre.x + half.x * (1.0 - 2.0 * st.x), centre.y + half.y * (1.0 - 2.0 * st.y))
}

fn sphere(middle: Vec3, radius: f32) -> Cage {
  Cage::rounded(middle, Vec3::splat(radius * 2.0), UVec3::splat(2), 1.0)
    .subdivided(2)
    .shaped(|at| middle + (at - middle).normalize() * radius)
}

fn head(skin: &Skin, features: &Features) -> Mesh {
  let eyes: Vec<(Vec2, [u8; 3])> = match features.eyes.len() {
    0 => vec![(Vec2::new(2.0, 4.5), [60, 40, 30]), (Vec2::new(6.0, 4.5), [60, 40, 30])],
    _ => features.eyes.clone()
  };
  let eyes: Vec<(Vec2, [u8; 3])> = eyes
    .into_iter()
    .map(|(texel, iris)| {
      let at = on_face(texel);
      (Vec2::new(at.x.signum() * at.x.abs().clamp(0.05, 0.072), at.y.clamp(0.18, 0.27)), iris)
    })
    .collect();
  let mouth = mouth(features);
  let eye_y = eyes.iter().map(|(at, _)| at.y).sum::<f32>() / eyes.len() as f32;
  let skull = Cage::rounded(
    Vec3::new(0.0, 0.24, 0.015),
    Vec3::new(0.32, 0.43, 0.36),
    UVec3::new(4, 4, 4),
    0.7
  )
  .shaped(|at| {
    let low = ((0.22 - at.y) / 0.16).clamp(0.0, 1.0);
    let forward = (-at.z / 0.18).clamp(0.0, 1.0);
    Vec3::new(
      at.x * (1.0 - low * 0.22 * (0.5 + forward)),
      at.y + low * forward * 0.012,
      at.z * (1.0 - low * 0.08) + (1.0 - forward) * low * 0.03
    )
  })
  .subdivided(2);
  let reference = skull.clone();
  let front = |x: f32, y: f32| Vec3::new(x, y, surface_at(&reference, x, y));
  let shaped = eyes
    .iter()
    .fold(skull, |cage, &(at, _)| {
      let socket = front(at.x, at.y);
      cage
        .pulled(socket, 0.03, Vec3::Z * 0.01)
        .pulled(front(at.x, at.y + 0.02), 0.022, Vec3::new(0.0, -0.002, -0.006))
        .pulled(front(at.x, at.y - 0.02), 0.02, Vec3::new(0.0, 0.002, -0.005))
        .pulled(front(at.x * 1.05, at.y + 0.04), 0.045, Vec3::new(0.0, 0.003, -0.008))
        .pulled(front(at.x * 1.25, at.y - 0.055), 0.05, Vec3::new(at.x.signum() * 0.004, 0.0, -0.007))
    })
    .pulled(front(0.0, eye_y - 0.03), 0.05, Vec3::new(0.0, -0.004, -0.03))
    .pulled(front(0.0, eye_y - 0.065), 0.028, Vec3::new(0.0, -0.004, -0.016))
    .pulled(front(0.0, eye_y + 0.01), 0.03, Vec3::new(0.0, 0.0, -0.006))
    .pulled(front(0.0, mouth.y - 0.003), 0.03, Vec3::new(0.0, 0.0, -0.006))
    .pulled(front(0.0, mouth.y - 0.055), 0.05, Vec3::new(0.0, -0.006, -0.012));
  let shaped = (-3..=3).fold(shaped, |cage, step| {
    let x = mouth.x + step as f32 * 0.009;
    let depth = 0.0035 * (1.0 - (step as f32 / 4.0).powi(2));
    cage.pulled(front(x, mouth.y), 0.011, Vec3::new(0.0, 0.0, depth))
  });
  let face = shaped.subdivided(1);
  let hairness = |at: Vec3| {
    let (count, hair) = (0..27)
      .map(|probe| Vec3::new((probe % 3) as f32, (probe / 3 % 3) as f32, (probe / 9) as f32) - 1.0)
      .fold((0.0, 0.0), |(count, hair), offset| {
        let (face, texel) = Part::Head.texel(at + offset * 0.014);
        let hairy = kind(skin, features, Part::Head, face, texel) == Kind::Hair;
        (count + 1.0, hair + if hairy { 1.0 } else { 0.0 })
      });
    hair / count
  };
  let lift: Vec<f32> = face.points.iter().map(|&at| hairness(at)).collect();
  let face = Cage {
    points: face
      .points
      .iter()
      .zip(face.vertex_normals())
      .zip(&lift)
      .map(|((&at, normal), &hair)| {
        let bulk = hair * hair * (3.0 - 2.0 * hair);
        let tufts = perlin3(0x4a17, at.x * 28.0, at.y * 18.0, at.z * 28.0) * 0.006;
        at + normal * bulk * (0.02 + tufts)
      })
      .collect(),
    ..face
  };
  let ears = [1.0f32, -1.0].map(|side| {
    Cage::rounded(
      Vec3::new(side * 0.158, eye_y - 0.005, 0.035),
      Vec3::new(0.03, 0.085, 0.055),
      UVec3::new(1, 2, 2),
      0.6
    )
    .shaped(move |at| {
      let up = (at.y - eye_y) / 0.04;
      at + Vec3::new(side * 0.012 * (at.z - 0.01).max(0.0) / 0.03, 0.0, up * 0.006)
    })
    .subdivided(2)
  });
  let neck = Cage::tube(
    &[
      (Vec3::new(0.0, -0.08, 0.02), Vec2::new(0.1, 0.095)),
      (Vec3::new(0.0, 0.03, 0.03), Vec2::new(0.088, 0.08)),
      (Vec3::new(0.0, 0.14, 0.035), Vec2::new(0.09, 0.09))
    ],
    8
  )
  .subdivided(2);
  let surface = code(Part::Head, Finish::Surface, [0; 3]);
  let lid = packed(PALETTE[features.flesh as usize]) as f32;
  let eyeballs: Vec<(Cage, f32, f32)> = eyes
    .iter()
    .map(|&(at, iris)| {
      let socket = surface_at(&face, at.x, at.y);
      (
        sphere(Vec3::new(at.x, at.y, socket + 0.0135), 0.021),
        code(Part::Head, Finish::Eyeball, iris),
        lid
      )
    })
    .collect();
  assembled(
    &[(face, surface, 0.0), (neck, surface, 0.0)]
      .into_iter()
      .chain(ears.map(|ear| (ear, surface, 0.0)))
      .chain(eyeballs)
      .collect::<Vec<_>>()
  )
}

fn torso() -> Mesh {
  let cage = Cage::rounded(Vec3::ZERO, Vec3::new(0.42, 0.7, 0.23), UVec3::new(4, 6, 2), 0.4)
    .shaped(|at| {
      let t = (at.y + 0.35) / 0.7;
      let bell = |middle: f32, width: f32| (-((t - middle) / width).powi(2)).exp();
      let wide = 1.02 - bell(0.45, 0.18) * 0.12 + bell(0.84, 0.12) * 0.1;
      let deep = 1.0 + bell(0.74, 0.14) * 0.1 - bell(0.45, 0.2) * 0.06 + bell(0.05, 0.12) * 0.04;
      let shoulder = (t - 0.8).max(0.0) * (at.x.abs() - 0.06).max(0.0) * 1.4;
      let collar = (t - 0.9).max(0.0) * (1.0 - (at.x.abs() / 0.12).min(1.0)) * 0.6;
      Vec3::new(
        at.x * wide,
        at.y - shoulder + collar,
        at.z * deep - bell(0.72, 0.12) * 0.014 * (at.z < 0.0) as u8 as f32
          + bell(0.8, 0.1) * 0.01 * (at.z > 0.0) as u8 as f32
      )
    })
    .subdivided(3);
  assembled(&[(cage, code(Part::Body, Finish::Surface, [0; 3]), 0.0)])
}

pub fn arm(part: Part) -> Mesh {
  let side = part.side();
  let ring = |x: f32, y: f32, z: f32, wide: f32, deep: f32| {
    (Vec3::new(side * x, y, z), Vec2::new(wide, deep))
  };
  let limb = Cage::tube(
    &[
      ring(-0.1, 0.0, 0.0, 0.06, 0.07),
      ring(-0.075, -0.05, 0.0, 0.092, 0.092),
      ring(-0.06, -0.2, 0.005, 0.08, 0.084),
      ring(-0.05, -0.33, 0.0, 0.066, 0.07),
      ring(-0.045, -0.43, -0.005, 0.07, 0.072),
      ring(-0.04, -0.55, 0.0, 0.05, 0.055),
      ring(-0.04, -0.6, 0.0, 0.046, 0.058)
    ],
    8
  )
  .subdivided(2);
  let palm = Cage::rounded(
    Vec3::new(side * -0.04, -0.652, 0.0),
    Vec3::new(0.05, 0.11, 0.1),
    UVec3::new(1, 2, 2),
    0.6
  )
  .subdivided(2);
  let finger = |z: f32, long: f32| {
    let path = [(-0.04, -0.695), (-0.055, -0.695 - long * 0.7), (-0.08, -0.695 - long)];
    Cage::tube(
      &path.map(|(x, y)| (Vec3::new(side * x, y, z), Vec2::splat(0.0125))),
      6
    )
    .subdivided(2)
  };
  let thumb = Cage::tube(
    &[
      (Vec3::new(side * -0.05, -0.615, -0.035), Vec2::splat(0.019)),
      (Vec3::new(side * -0.07, -0.645, -0.06), Vec2::splat(0.015)),
      (Vec3::new(side * -0.085, -0.675, -0.062), Vec2::splat(0.013))
    ],
    6
  )
  .subdivided(2);
  let surface = code(part, Finish::Surface, [0; 3]);
  assembled(
    &[(limb, surface, 0.0), (palm, surface, 0.0), (thumb, surface, 0.0)]
      .into_iter()
      .chain(
        [(-0.036, 0.05), (-0.012, 0.058), (0.012, 0.055), (0.035, 0.045)]
          .map(|(z, long)| (finger(z, long), surface, 0.0))
      )
      .collect::<Vec<_>>()
  )
}

fn leg(part: Part) -> Mesh {
  let side = part.side();
  let ring = |x: f32, y: f32, z: f32, wide: f32, deep: f32| {
    (Vec3::new(side * x, y, z), Vec2::new(wide, deep))
  };
  let limb = Cage::tube(
    &[
      ring(-0.01, 0.04, 0.0, 0.11, 0.11),
      ring(-0.005, -0.1, 0.0, 0.115, 0.115),
      ring(0.0, -0.28, 0.0, 0.09, 0.092),
      ring(0.0, -0.37, -0.005, 0.08, 0.082),
      ring(0.0, -0.47, 0.012, 0.084, 0.088),
      ring(0.0, -0.6, 0.0, 0.064, 0.066),
      ring(0.0, -0.66, 0.0, 0.068, 0.072)
    ],
    8
  )
  .subdivided(2);
  let boot = Cage::rounded(
    Vec3::new(0.0, -0.68, -0.035),
    Vec3::new(0.135, 0.085, 0.26),
    UVec3::new(2, 1, 3),
    0.5
  )
  .shaped(|at| {
    let toe = ((-0.035 - at.z) / 0.125).clamp(0.0, 1.0);
    let sole = (at.y < -0.7) as u8 as f32;
    Vec3::new(
      at.x * (1.0 - toe * 0.15),
      at.y.max(-0.72) - toe * (at.y + 0.68).max(0.0) * 0.45,
      at.z
    ) + Vec3::Y * sole * 0.0
  })
  .sharpened(60.0, 1.0)
  .subdivided(2)
  .shaped(|at| at.with_y(at.y.max(-0.72)));
  let surface = code(part, Finish::Surface, [0; 3]);
  assembled(&[(limb, surface, 0.0), (boot, surface, 0.0)])
}

pub struct Likeness {
  pub head: Mesh,
  pub body: Mesh,
  pub arm: Mesh,
  pub left_arm: Mesh,
  pub leg: Mesh,
  pub left_leg: Mesh
}

pub fn likeness(skin: &Skin) -> Likeness {
  let features = features(skin);
  Likeness {
    head: head(skin, &features),
    body: torso(),
    arm: arm(Part::Arm),
    left_arm: arm(Part::LeftArm),
    leg: leg(Part::Leg),
    left_leg: leg(Part::LeftLeg)
  }
}

pub fn register(app: &mut App) {
  embedded_asset!(app, "person.wgsl");
  app.add_plugins(MaterialPlugin::<Flesh>::default());
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_fresh_face_has_two_eyes_and_a_mouth() {
    let skin = Skin::fresh(7);
    let found = features(&skin);
    assert_eq!(found.eyes.len(), 2);
    assert!(found.mouth.is_some());
    assert_eq!(Part::Head.texel(Vec3::new(0.0, 0.5, 0.0)).0, Face::Top);
    assert_eq!(Part::Arm.texel(Vec3::new(0.0, -0.7, 0.0)).0, Face::Bottom);
  }

  fn preview(mesh: &Mesh, offset: Vec3) -> Mesh {
    use bevy::mesh::VertexAttributeValues;
    let (Some(VertexAttributeValues::Float32x3(positions)), Some(VertexAttributeValues::Float32x4(colors)), Some(Indices::U32(indices))) = (
      mesh.attribute(Mesh::ATTRIBUTE_POSITION),
      mesh.attribute(Mesh::ATTRIBUTE_COLOR),
      mesh.indices()
    ) else {
      panic!("sculpted")
    };
    let corners: Vec<usize> = indices.iter().map(|&index| index as usize).collect();
    let tone = |code: f32| match (code as u32 >> 3) & 3 {
      1 => [0.9, 0.9, 0.95, 1.0],
      _ => [0.75, 0.55, 0.45, 1.0]
    };
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
      .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        corners.iter().map(|&at| (Vec3::from(positions[at]) + offset).to_array()).collect::<Vec<_>>()
      )
      .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        corners.iter().map(|&at| [colors[at][0], colors[at][1], colors[at][2]]).collect::<Vec<_>>()
      )
      .with_inserted_attribute(
        Mesh::ATTRIBUTE_COLOR,
        corners.iter().map(|&at| tone(colors[at][3])).collect::<Vec<_>>()
      )
  }

  #[test]
  #[ignore]
  fn people() {
    let made = likeness(&Skin::fresh(3));
    for mesh in [&made.head, &made.body, &made.arm, &made.leg] {
      println!("{} triangles", mesh.indices().map_or(0, |indices| indices.len() / 3));
    }
    let whole = crate::sculpt::merged(
      [
        (&made.head, Part::Head),
        (&made.body, Part::Body),
        (&made.arm, Part::Arm),
        (&made.left_arm, Part::LeftArm),
        (&made.leg, Part::Leg),
        (&made.left_leg, Part::LeftLeg)
      ]
      .map(|(mesh, part)| preview(mesh, part.pivot()))
    );
    crate::sculpt::sheet(&[preview(&made.head, Vec3::ZERO), whole], "screenshots/person.ppm")
  }
}
