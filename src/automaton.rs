use {crate::{cage::Cage, sculpt::triangles},
     bevy::{asset::{RenderAssetUsages, embedded_asset},
            mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
            pbr::{ExtendedMaterial, MaterialExtension},
            prelude::*,
            render::render_resource::AsBindGroup,
            shader::ShaderRef},
     std::f32::consts::TAU};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Finish {
  Paint,
  Steel,
  Dark,
  Glow,
  Gilt,
  Blade,
  Lens,
  Pulse
}

#[derive(Clone, Copy)]
pub struct Coat {
  pub color: [f32; 3],
  pub finish: Finish
}

const fn coat(color: [f32; 3], finish: Finish) -> Coat { Coat { color, finish } }

pub struct Tribe {
  pub name: &'static str,
  pub armour: [f32; 3],
  pub trim: [f32; 3],
  pub glow: [f32; 3],
  pub coil: [f32; 3]
}

pub const TRIBES: [Tribe; 6] = [
  Tribe {
    name: "Ember",
    armour: [0.6, 0.07, 0.05],
    trim: [0.95, 0.62, 0.14],
    glow: [1.0, 0.5, 0.1],
    coil: [0.36, 0.2, 0.42]
  },
  Tribe {
    name: "Tide",
    armour: [0.1, 0.28, 0.68],
    trim: [0.5, 0.72, 0.9],
    glow: [0.75, 1.0, 0.3],
    coil: [0.3, 0.3, 0.5]
  },
  Tribe {
    name: "Thicket",
    armour: [0.42, 0.66, 0.1],
    trim: [0.14, 0.28, 0.1],
    glow: [1.0, 0.22, 0.12],
    coil: [0.34, 0.22, 0.4]
  },
  Tribe {
    name: "Frost",
    armour: [0.86, 0.88, 0.9],
    trim: [0.55, 0.62, 0.7],
    glow: [0.3, 0.72, 1.0],
    coil: [0.5, 0.52, 0.6]
  },
  Tribe {
    name: "Dune",
    armour: [0.76, 0.6, 0.38],
    trim: [0.36, 0.18, 0.46],
    glow: [0.95, 0.3, 0.9],
    coil: [0.4, 0.24, 0.46]
  },
  Tribe {
    name: "Crag",
    armour: [0.4, 0.24, 0.12],
    trim: [0.13, 0.12, 0.12],
    glow: [1.0, 0.82, 0.22],
    coil: [0.3, 0.26, 0.24]
  }
];

const STEEL: Coat = coat([0.6, 0.61, 0.64], Finish::Steel);
const GUNMETAL: Coat = coat([0.07, 0.07, 0.08], Finish::Dark);
const EDGE: Coat = coat([0.82, 0.84, 0.87], Finish::Blade);
const BRASS: Coat = coat([0.86, 0.62, 0.26], Finish::Gilt);

pub const PELVIS: u16 = 0;
pub const CHEST: u16 = 1;
pub const HEAD: u16 = 2;
pub const BONES: usize = 11;
pub const PARENT: [Option<u16>; BONES] = [
  None,
  Some(0),
  Some(1),
  Some(1),
  Some(3),
  Some(1),
  Some(5),
  Some(0),
  Some(7),
  Some(0),
  Some(9)
];

pub fn arm(side: f32) -> (u16, u16) { if side > 0.0 { (3, 4) } else { (5, 6) } }

pub fn leg(side: f32) -> (u16, u16) { if side > 0.0 { (7, 8) } else { (9, 10) } }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Model {
  Cogling,
  Scavver,
  Gladiator,
  Paragon
}

#[derive(Clone, Copy)]
pub struct Frame {
  pub hip: Vec3,
  pub waist: f32,
  pub knee: f32,
  pub shoulder: Vec3,
  pub elbow: Vec3,
  pub wrist: Vec3,
  pub neck: f32
}

impl Frame {
  fn scaled(self, by: f32) -> Frame {
    Frame {
      hip: self.hip * by,
      waist: self.waist * by,
      knee: self.knee * by,
      shoulder: self.shoulder * by,
      elbow: self.elbow * by,
      wrist: self.wrist * by,
      neck: self.neck * by
    }
  }

  pub fn pivot(&self, bone: u16) -> Vec3 {
    let side = if matches!(bone, 3 | 4 | 7 | 8) { 1.0 } else { -1.0 };
    let mirrored = |at: Vec3| at * Vec3::new(side, 1.0, 1.0);
    match bone {
      PELVIS => Vec3::Y * self.hip.y,
      CHEST => Vec3::Y * self.waist,
      HEAD => Vec3::Y * self.neck,
      3 | 5 => mirrored(self.shoulder),
      4 | 6 => mirrored(self.elbow),
      7 | 9 => mirrored(self.hip),
      _ => mirrored(self.hip.with_y(self.knee))
    }
  }
}

impl Model {
  pub fn size(self) -> f32 {
    match self {
      Model::Cogling | Model::Scavver | Model::Gladiator => 1.0,
      Model::Paragon => 0.9
    }
  }

  pub fn frame(self) -> Frame {
    match self {
      Model::Cogling => Frame {
        hip: Vec3::new(0.085, 0.5, 0.0),
        waist: 0.58,
        knee: 0.29,
        shoulder: Vec3::new(0.2, 0.84, 0.0),
        elbow: Vec3::new(0.225, 0.655, 0.0),
        wrist: Vec3::new(0.235, 0.47, -0.01),
        neck: 0.9
      },
      Model::Scavver => Frame {
        hip: Vec3::new(0.1, 0.52, 0.0),
        waist: 0.6,
        knee: 0.3,
        shoulder: Vec3::new(0.25, 0.9, 0.0),
        elbow: Vec3::new(0.28, 0.69, 0.0),
        wrist: Vec3::new(0.29, 0.5, -0.02),
        neck: 0.98
      },
      Model::Gladiator | Model::Paragon => Frame {
        hip: Vec3::new(0.15, 1.04, 0.0),
        waist: 1.12,
        knee: 0.58,
        shoulder: Vec3::new(0.33, 1.6, 0.0),
        elbow: Vec3::new(0.38, 1.27, 0.02),
        wrist: Vec3::new(0.4, 0.96, -0.02),
        neck: 1.7
      }
      .scaled(self.size())
    }
  }
}

fn painted(coat: Coat) -> [f32; 3] {
  let [r, g, b] = coat.color.map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round());
  [r * 65536.0 + g * 256.0 + b, coat.finish as u32 as f32, 0.0]
}

fn forge(cage: &Cage, paint: impl Fn(Vec3, Vec3) -> Coat, bone: u16, scale: f32) -> Mesh {
  let triangles = triangles(cage);
  let at = |corner: u32| cage.points[corner as usize] * scale;
  let facets: Vec<Vec3> =
    triangles.iter().map(|&[a, b, c]| (at(b) - at(a)).cross(at(c) - at(a))).collect();
  let around = triangles.iter().zip(&facets).fold(
    vec![Vec::new(); cage.points.len()],
    |mut around: Vec<Vec<Vec3>>, (triangle, &facet)| {
      for &corner in triangle {
        around[corner as usize].push(facet)
      }
      around
    }
  );
  let smooth = |corner: u32, facet: Vec3| {
    let own = facet.normalize_or_zero();
    around[corner as usize]
      .iter()
      .filter(|other| other.normalize_or_zero().dot(own) > 0.6)
      .sum::<Vec3>()
      .normalize_or(own)
  };
  let corners: Vec<(Vec3, Vec3, [f32; 3])> = triangles
    .iter()
    .zip(&facets)
    .flat_map(|(triangle, &facet)| {
      let centre = triangle.iter().map(|&corner| at(corner)).sum::<Vec3>() / 3.0;
      let code = painted(paint(centre / scale, facet.normalize_or_zero()));
      triangle.map(|corner| (at(corner), smooth(corner, facet), code))
    })
    .collect();
  let count = corners.len();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      corners.iter().map(|(at, ..)| at.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, vec![
      [1.0f32, 0.0, 0.0, 1.0];
      count
    ])
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_COLOR,
      corners
        .iter()
        .map(|(_, normal, code)| normal.extend(code[1]).to_array())
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_0,
      corners.iter().map(|(at, ..)| [at.x, at.y]).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_1,
      corners.iter().map(|(at, _, code)| [at.z, code[0]]).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_JOINT_INDEX,
      VertexAttributeValues::Uint16x4(vec![[bone, 0, 0, 0]; count])
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![
      [1.0f32, 0.0, 0.0, 0.0];
      count
    ])
    .with_inserted_indices(Indices::U32((0..count as u32).collect()))
}

struct Body {
  scale: f32,
  pieces: Vec<Mesh>
}

impl Body {
  fn put(&mut self, cage: Cage, coat: Coat, bone: u16) {
    self.pieces.push(forge(&cage, |_, _| coat, bone, self.scale))
  }

  fn paint(&mut self, cage: Cage, paint: impl Fn(Vec3, Vec3) -> Coat, bone: u16) {
    self.pieces.push(forge(&cage, paint, bone, self.scale))
  }

  fn mesh(self) -> Mesh {
    self
      .pieces
      .into_iter()
      .reduce(|mut whole, piece| {
        whole.merge(&piece).expect("forged pieces share their attributes");
        whole
      })
      .expect("a body has pieces")
  }
}

fn turned(cage: Cage, at: Vec3, toward: Vec3) -> Cage {
  let turn = Quat::from_rotation_arc(Vec3::NEG_Z, toward.normalize_or(Vec3::NEG_Z));
  cage.shaped(|point| at + turn * point)
}

fn ball(at: Vec3, radius: Vec3) -> Cage {
  Cage::rounded(at, radius * 2.0, UVec3::splat(2), 1.0)
}

fn orb(at: Vec3, radius: Vec3) -> Cage {
  Cage::rounded(at, radius * 2.0, UVec3::splat(3), 1.0)
}

fn slab(at: Vec3, size: Vec3, round: f32) -> Cage {
  Cage::rounded(at, size, UVec3::splat(2), round)
}

fn rod(from: Vec3, to: Vec3, radius: f32, sides: u32) -> Cage {
  Cage::tube(&[(from, Vec2::splat(radius)), (to, Vec2::splat(radius))], sides.min(5))
}

fn ring(at: Vec3, toward: Vec3, radius: f32, thick: f32) -> Cage {
  let path: Vec<(Vec3, Vec2)> = (0..=8)
    .map(|step| {
      let angle = step as f32 / 8.0 * TAU;
      (Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0), Vec2::splat(thick))
    })
    .collect();
  turned(Cage::tube(&path, 3), at, toward)
}

fn piston(from: Vec3, to: Vec3, radius: f32) -> [(Cage, Coat); 2] {
  let split = from.lerp(to, 0.52);
  [
    (
      Cage::tube(
        &[
          (from, Vec2::splat(radius * 1.12)),
          (split - (to - from) * 0.02, Vec2::splat(radius)),
          (split, Vec2::splat(radius * 1.18))
        ],
        5
      ),
      GUNMETAL
    ),
    (rod(split, to, radius * 0.55, 5), STEEL)
  ]
}

fn gear(at: Vec3, toward: Vec3, radius: f32, thick: f32, teeth: u32) -> Cage {
  let sides = teeth * 2;
  let cog = Cage::prism(sides, radius, thick).shaped(|point| {
    let step = (point.z.atan2(point.x) / TAU * sides as f32).round() as i32;
    let reach = if step.rem_euclid(2) == 0 { 1.0 } else { 0.8 };
    Vec3::new(point.x * reach, point.z * reach, point.y)
  });
  turned(cog, at, toward)
}

fn coil(from: Vec3, to: Vec3, radius: f32, thick: f32, turns: f32) -> Cage {
  let axis = to - from;
  let across = axis.any_orthonormal_vector();
  let up = axis.normalize().cross(across);
  let steps = (turns * 4.0) as usize;
  Cage::tube(
    &(0..=steps)
      .map(|step| {
        let along = step as f32 / steps as f32;
        let angle = along * turns * TAU;
        (
          from + axis * along + (across * angle.cos() + up * angle.sin()) * radius,
          Vec2::splat(thick)
        )
      })
      .collect::<Vec<_>>(),
    3
  )
}

fn shell(path: &[(Vec3, Vec2)], level: u32) -> Cage {
  Cage::tube(path, 6 + 2 * u32::from(level >= 2))
}

fn spike(from: Vec3, to: Vec3, thick: Vec2) -> Cage { Cage::spike(from, to, thick) }

fn hand(wrist: Vec3, side: f32, size: f32) -> Vec<(Cage, Coat)> {
  let palm = wrist - Vec3::Y * size * 0.55;
  let inward = Vec3::X * -side;
  let fingers = [-0.22f32, 0.22].map(|offset| {
    let base = palm + Vec3::new(0.0, -size * 0.38, offset * size);
    let middle = base + Vec3::new(0.0, -size * 0.32, -size * 0.04) + inward * size * 0.1;
    let tip = middle + inward * size * 0.3 + Vec3::Y * -size * 0.1;
    (
      Cage::tube(
        &[
          (base, Vec2::splat(size * 0.11)),
          (middle, Vec2::splat(size * 0.1)),
          (tip, Vec2::splat(size * 0.07))
        ],
        3
      ),
      STEEL
    )
  });
  let thumb = (
    Cage::tube(
      &[
        (palm + Vec3::new(0.0, -size * 0.1, -size * 0.36), Vec2::splat(size * 0.12)),
        (
          palm + Vec3::new(0.0, -size * 0.32, -size * 0.6) + inward * size * 0.15,
          Vec2::splat(size * 0.08)
        )
      ],
      3
    ),
    STEEL
  );
  [(slab(palm, Vec3::new(0.5, 0.8, 0.85) * size, 0.5), GUNMETAL), thumb]
    .into_iter()
    .chain(fingers)
    .collect()
}

fn on_egg(centre: Vec3, radius: Vec3, x: f32, y: f32) -> (Vec3, Vec3) {
  let flat = (x / radius.x).powi(2) + (y / radius.y).powi(2);
  let depth = radius.z * (1.0 - flat).max(0.0).sqrt();
  let at = centre + Vec3::new(x, y, -depth);
  (at, ((at - centre) / (radius * radius)).normalize())
}

fn eye(body: &mut Body, at: Vec3, normal: Vec3, size: f32, glow: Coat) {
  body.put(
    turned(orb(Vec3::ZERO, Vec3::new(size, size, size * 0.5)), at, normal),
    glow,
    HEAD
  );
  body.put(
    ring(at - normal * size * 0.15, normal, size * 1.12, size * 0.22),
    GUNMETAL,
    HEAD
  )
}

fn limbs(body: &mut Body, frame: &Frame, side: f32, build: &dyn Fn(&mut Body, Limb)) {
  let mirror = |at: Vec3| at * Vec3::new(side, 1.0, 1.0);
  let (upper, lower) = arm(side);
  let (thigh, shin) = leg(side);
  build(body, Limb {
    side,
    shoulder: mirror(frame.shoulder),
    elbow: mirror(frame.elbow),
    wrist: mirror(frame.wrist),
    hip: mirror(frame.hip),
    knee: mirror(frame.hip.with_y(frame.knee)),
    bones: [upper, lower, thigh, shin]
  })
}

struct Limb {
  side: f32,
  shoulder: Vec3,
  elbow: Vec3,
  wrist: Vec3,
  hip: Vec3,
  knee: Vec3,
  bones: [u16; 4]
}

fn cogling(tribe: &Tribe) -> Mesh {
  let armour = coat(tribe.armour, Finish::Paint);
  let trim = coat(tribe.trim, Finish::Paint);
  let glow = coat(tribe.glow, Finish::Glow);
  let mut body = Body { scale: 1.0, pieces: Vec::new() };
  body.put(
    slab(Vec3::new(0.0, 0.51, 0.0), Vec3::new(0.24, 0.12, 0.15), 0.6),
    GUNMETAL,
    PELVIS
  );
  body.put(ball(Vec3::new(0.0, 0.585, 0.0), Vec3::splat(0.05)), STEEL, CHEST);
  body.paint(
    Cage::rounded(
      Vec3::new(0.0, 0.73, 0.0),
      Vec3::new(0.34, 0.27, 0.21),
      UVec3::splat(2),
      0.5
    )
    .shaped(|at| {
      let rise = ((at.y - 0.6) / 0.27).clamp(0.0, 1.0);
      Vec3::new(at.x * (0.82 + rise * 0.3), at.y, at.z - (at.y - 0.73) * 0.06)
    }),
    |at, _| if at.y < 0.625 { trim } else { armour },
    CHEST
  );
  body.put(
    slab(Vec3::new(0.0, 0.74, 0.1), Vec3::new(0.12, 0.22, 0.06), 0.5),
    GUNMETAL,
    CHEST
  );
  body.put(
    orb(Vec3::new(0.075, 0.76, -0.112), Vec3::new(0.026, 0.026, 0.012)),
    coat(tribe.glow, Finish::Pulse),
    CHEST
  );
  body.put(ring(Vec3::new(0.075, 0.76, -0.108), Vec3::NEG_Z, 0.031, 0.006), BRASS, CHEST);
  body.put(
    rod(Vec3::new(0.0, 0.83, 0.0), Vec3::new(0.0, 0.96, 0.0), 0.026, 8),
    STEEL,
    HEAD
  );
  body.put(ball(Vec3::new(0.0, 1.0, 0.03), Vec3::new(0.09, 0.1, 0.09)), GUNMETAL, HEAD);
  let (centre, radius) = (Vec3::new(0.0, 1.04, -0.01), Vec3::new(0.12, 0.155, 0.115));
  body.put(
    Cage::rounded(centre, radius * 2.0, UVec3::splat(2), 1.0)
      .shaped(|at| {
        let above = ((at.y - centre.y) / radius.y).max(0.0);
        let below = ((centre.y - at.y) / radius.y).max(0.0);
        let narrow = 1.0 - above * 0.22 - below * 0.08;
        Vec3::new(
          at.x * narrow,
          at.y + above * above * 0.03,
          centre.z + (at.z - centre.z) * narrow
        )
      })
      .subdivided(1),
    armour,
    HEAD
  );
  let shrunk = radius * 0.97;
  for side in [1.0f32, -1.0] {
    let (at, normal) = on_egg(centre, shrunk, side * 0.052, 0.008);
    eye(&mut body, at, normal, 0.034, glow)
  }
  let brow: Vec<(Vec3, Vec2)> = (0..=6)
    .map(|step| {
      let x = (step as f32 / 6.0 - 0.5) * 0.19;
      let lift = 0.058 - (x / 0.1).powi(2) * 0.012;
      (on_egg(centre, radius, x, lift).0, Vec2::splat(0.011))
    })
    .collect();
  body.put(Cage::tube(&brow, 6), trim, HEAD);
  let (mouth, facing) = on_egg(centre, radius, 0.0, -0.075);
  body.put(
    turned(slab(Vec3::ZERO, Vec3::new(0.055, 0.022, 0.02), 0.6), mouth, facing),
    GUNMETAL,
    HEAD
  );
  for slit in [-1.0f32, 0.0, 1.0] {
    let (at, facing) = on_egg(centre, radius, slit * 0.014, -0.075);
    body.put(
      turned(
        slab(Vec3::ZERO, Vec3::new(0.005, 0.016, 0.01), 0.3),
        at - facing * 0.004,
        facing
      ),
      STEEL,
      HEAD
    )
  }
  let frame = Model::Cogling.frame();
  for side in [1.0f32, -1.0] {
    limbs(&mut body, &frame, side, &|body, limb| {
      let Limb {
        side,
        shoulder,
        elbow,
        wrist,
        hip,
        knee,
        bones: [upper, lower, thigh, shin]
      } = limb;
      body.put(ball(shoulder, Vec3::splat(0.052)), STEEL, upper);
      body.put(
        slab(
          shoulder + Vec3::new(side * 0.02, 0.035, 0.0),
          Vec3::new(0.12, 0.08, 0.13),
          0.7
        ),
        armour,
        upper
      );
      body.put(rod(shoulder, elbow, 0.03, 8), GUNMETAL, upper);
      body.put(ball(elbow, Vec3::splat(0.035)), STEEL, lower);
      body.put(
        shell(
          &[
            (elbow - Vec3::Y * 0.02, Vec2::new(0.048, 0.048)),
            (elbow.lerp(wrist, 0.5), Vec2::new(0.058, 0.054)),
            (wrist + Vec3::Y * 0.035, Vec2::new(0.046, 0.044))
          ],
          1
        ),
        armour,
        lower
      );
      body.put(
        coil(wrist + Vec3::Y * 0.03, wrist - Vec3::Y * 0.06, 0.024, 0.008, 4.0),
        coat(tribe.coil, Finish::Gilt),
        lower
      );
      for (cage, finish) in hand(wrist - Vec3::Y * 0.06, side, 0.09) {
        body.put(cage, finish, lower)
      }
      body.put(ball(hip, Vec3::splat(0.045)), STEEL, thigh);
      body.put(rod(hip, knee, 0.035, 8), GUNMETAL, thigh);
      body.put(
        slab(knee + Vec3::new(0.0, 0.0, -0.025), Vec3::splat(0.1), 0.8),
        armour,
        shin
      );
      body.put(
        shell(
          &[
            (knee - Vec3::Y * 0.03, Vec2::new(0.05, 0.05)),
            (knee - Vec3::Y * 0.12, Vec2::new(0.06, 0.055)),
            (Vec3::new(knee.x, 0.1, 0.0), Vec2::new(0.045, 0.045))
          ],
          1
        ),
        armour,
        shin
      );
      body.put(
        slab(Vec3::new(knee.x, 0.048, -0.04), Vec3::new(0.12, 0.09, 0.22), 0.5)
          .shaped(|at| at - Vec3::Y * ((-0.04 - at.z).max(0.0) * 0.18)),
        armour,
        shin
      );
      body.put(
        slab(Vec3::new(knee.x, 0.008, -0.04), Vec3::new(0.115, 0.016, 0.2), 0.3),
        GUNMETAL,
        shin
      )
    })
  }
  body.mesh()
}

fn scavver(tribe: &Tribe) -> Mesh {
  let armour = coat(tribe.armour, Finish::Paint);
  let trim = coat(tribe.trim, Finish::Paint);
  let glow = coat(tribe.glow, Finish::Glow);
  let mut body = Body { scale: 1.0, pieces: Vec::new() };
  body.put(
    slab(Vec3::new(0.0, 0.53, 0.0), Vec3::new(0.28, 0.13, 0.17), 0.6),
    GUNMETAL,
    PELVIS
  );
  body.put(
    shell(
      &[
        (Vec3::new(0.0, 0.6, 0.0), Vec2::new(0.07, 0.09)),
        (Vec3::new(0.0, 0.78, 0.0), Vec2::new(0.1, 0.15)),
        (Vec3::new(0.0, 0.95, 0.0), Vec2::new(0.1, 0.17))
      ],
      1
    ),
    GUNMETAL,
    CHEST
  );
  for side in [1.0f32, -1.0] {
    body.put(
      slab(Vec3::new(side * 0.115, 0.88, -0.055), Vec3::new(0.15, 0.15, 0.1), 0.55)
        .shaped(|at| at + Vec3::Z * (at.x * side - 0.115).abs() * 0.2),
      armour,
      CHEST
    )
  }
  body.put(
    gear(Vec3::new(0.0, 0.76, -0.105), Vec3::NEG_Z, 0.085, 0.03, 10),
    STEEL,
    CHEST
  );
  body.put(
    rod(Vec3::new(0.0, 0.76, -0.09), Vec3::new(0.0, 0.76, -0.135), 0.028, 8),
    BRASS,
    CHEST
  );
  body.put(
    slab(Vec3::new(0.0, 0.8, 0.1), Vec3::new(0.26, 0.28, 0.08), 0.5),
    armour,
    CHEST
  );
  body.put(
    rod(Vec3::new(0.0, 0.92, 0.0), Vec3::new(0.0, 1.04, 0.0), 0.03, 8),
    STEEL,
    HEAD
  );
  let helm = Vec3::new(0.0, 1.13, -0.005);
  body.paint(
    Cage::rounded(helm, Vec3::new(0.25, 0.25, 0.28), UVec3::splat(2), 0.62)
      .shaped(|at| {
        let low = ((helm.y - at.y) / 0.125).max(0.0);
        Vec3::new(at.x * (1.0 - low * 0.18), at.y, at.z - low * 0.025)
      })
      .subdivided(1),
    |at, _| if at.y < 1.03 { trim } else { armour },
    HEAD
  );
  body.put(
    slab(helm + Vec3::new(0.0, 0.005, -0.125), Vec3::new(0.19, 0.055, 0.05), 0.7),
    coat(tribe.glow, Finish::Lens),
    HEAD
  );
  for side in [1.0f32, -1.0] {
    body.put(
      ball(helm + Vec3::new(side * 0.048, 0.006, -0.148), Vec3::new(0.02, 0.016, 0.008)),
      glow,
      HEAD
    )
  }
  body.put(
    Cage::tube(
      &[
        (helm + Vec3::new(0.0, 0.09, -0.11), Vec2::new(0.012, 0.03)),
        (helm + Vec3::new(0.0, 0.16, -0.01), Vec2::new(0.013, 0.05)),
        (helm + Vec3::new(0.0, 0.15, 0.12), Vec2::new(0.01, 0.04)),
        (helm + Vec3::new(0.0, 0.08, 0.2), Vec2::new(0.004, 0.01))
      ],
      6
    ),
    trim,
    HEAD
  );
  let frame = Model::Scavver.frame();
  for side in [1.0f32, -1.0] {
    limbs(&mut body, &frame, side, &|body, limb| {
      let Limb {
        side,
        shoulder,
        elbow,
        wrist,
        hip,
        knee,
        bones: [upper, lower, thigh, shin]
      } = limb;
      body.put(ball(shoulder, Vec3::splat(0.06)), STEEL, upper);
      body.put(
        slab(
          shoulder + Vec3::new(side * 0.03, 0.045, 0.0),
          Vec3::new(0.15, 0.09, 0.16),
          0.65
        ),
        trim,
        upper
      );
      for (cage, finish) in piston(shoulder, elbow, 0.036) {
        body.put(cage, finish, upper)
      }
      body.put(ball(elbow, Vec3::splat(0.04)), STEEL, lower);
      body.put(
        shell(
          &[
            (elbow - Vec3::Y * 0.02, Vec2::new(0.056, 0.056)),
            (elbow.lerp(wrist, 0.55), Vec2::new(0.07, 0.066)),
            (wrist + Vec3::Y * 0.02, Vec2::new(0.058, 0.055))
          ],
          1
        ),
        armour,
        lower
      );
      for (cage, finish) in hand(wrist - Vec3::Y * 0.01, side, 0.1) {
        body.put(cage, finish, lower)
      }
      if side > 0.0 {
        let grip = wrist - Vec3::Y * 0.11 + Vec3::X * side * -0.012;
        body.put(rod(grip.with_y(0.04), grip.with_y(1.28), 0.016, 6), GUNMETAL, lower);
        body.put(ring(grip.with_y(1.24), Vec3::Y, 0.04, 0.012), BRASS, lower);
        body.put(gear(grip.with_y(1.3), Vec3::NEG_Z, 0.055, 0.02, 7), BRASS, lower);
        body.put(ball(grip.with_y(1.3), Vec3::splat(0.022)), glow, lower)
      }
      body.put(ball(hip, Vec3::splat(0.05)), STEEL, thigh);
      for (cage, finish) in piston(hip, knee, 0.04) {
        body.put(cage, finish, thigh)
      }
      body.put(
        slab(knee + Vec3::new(0.0, 0.0, -0.03), Vec3::splat(0.11), 0.75),
        trim,
        shin
      );
      body.put(
        shell(
          &[
            (knee - Vec3::Y * 0.04, Vec2::new(0.06, 0.06)),
            (knee - Vec3::Y * 0.13, Vec2::new(0.075, 0.068)),
            (Vec3::new(knee.x, 0.1, 0.0), Vec2::new(0.055, 0.052))
          ],
          1
        ),
        armour,
        shin
      );
      body.put(
        slab(Vec3::new(knee.x, 0.05, -0.045), Vec3::new(0.15, 0.1, 0.25), 0.5)
          .shaped(|at| at - Vec3::Y * ((-0.04 - at.z).max(0.0) * 0.2)),
        trim,
        shin
      );
      body.put(
        slab(Vec3::new(knee.x, 0.009, -0.045), Vec3::new(0.14, 0.018, 0.23), 0.3),
        GUNMETAL,
        shin
      )
    })
  }
  body.mesh()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Weapon {
  Sword,
  Trident,
  Axe
}

fn weapon(
  body: &mut Body,
  kind: Weapon,
  grip: Vec3,
  tribe: &Tribe,
  flames: bool,
  bone: u16
) {
  let trim = coat(tribe.trim, Finish::Paint);
  match kind {
    Weapon::Sword => {
      let toward = Vec3::new(0.0, 0.3, -1.0).normalize();
      let flat = toward.cross(Vec3::X).normalize();
      body.put(rod(grip - toward * 0.1, grip + toward * 0.1, 0.019, 6), GUNMETAL, bone);
      body.put(ball(grip - toward * 0.11, Vec3::splat(0.026)), BRASS, bone);
      body.put(
        Cage::tube(
          &[
            (grip + toward * 0.1 - flat * 0.085, Vec2::splat(0.016)),
            (grip + toward * 0.11, Vec2::new(0.022, 0.03)),
            (grip + toward * 0.1 + flat * 0.085, Vec2::splat(0.016))
          ],
          6
        ),
        BRASS,
        bone
      );
      body.put(
        Cage::tube(
          &[
            (grip + toward * 0.11, Vec2::new(0.012, 0.042)),
            (grip + toward * 0.45, Vec2::new(0.011, 0.054)),
            (grip + toward * 0.82, Vec2::new(0.008, 0.04)),
            (grip + toward * 0.98, Vec2::new(0.002, 0.004))
          ],
          6
        ),
        EDGE,
        bone
      );
      body.put(
        rod(grip + toward * 0.12, grip + toward * 0.8, 0.0135, 4),
        if flames { BRASS } else { trim },
        bone
      );
      if flames {
        for step in 0..5 {
          let base = grip + toward * (0.25 + step as f32 * 0.13) - flat * 0.045;
          body.put(
            spike(base, base + toward * 0.1 - flat * 0.07, Vec2::new(0.008, 0.03)),
            BRASS,
            bone
          )
        }
      }
    }
    Weapon::Trident => {
      body.put(rod(grip - Vec3::Y * 0.75, grip + Vec3::Y * 0.85, 0.017, 6), STEEL, bone);
      body.put(
        rod(grip - Vec3::Y * 0.12, grip + Vec3::Y * 0.12, 0.022, 8),
        GUNMETAL,
        bone
      );
      let head = grip + Vec3::Y * 0.85;
      body.put(
        Cage::tube(
          &[
            (head + Vec3::new(0.0, 0.07, -0.12), Vec2::splat(0.012)),
            (head + Vec3::new(0.0, 0.0, -0.1), Vec2::splat(0.016)),
            (head, Vec2::splat(0.022)),
            (head + Vec3::new(0.0, 0.0, 0.1), Vec2::splat(0.016)),
            (head + Vec3::new(0.0, 0.07, 0.12), Vec2::splat(0.012))
          ],
          6
        ),
        trim,
        bone
      );
      for (z, tall) in [(-0.12f32, 0.24f32), (0.0, 0.34), (0.12, 0.24)] {
        let base = head + Vec3::new(0.0, if z == 0.0 { 0.0 } else { 0.07 }, z);
        body.put(
          spike(base, base + Vec3::new(0.0, tall, z * 0.3), Vec2::new(0.022, 0.01)),
          EDGE,
          bone
        )
      }
    }
    Weapon::Axe => {
      body.put(rod(grip - Vec3::Y * 0.28, grip + Vec3::Y * 0.6, 0.02, 6), GUNMETAL, bone);
      let head = grip + Vec3::Y * 0.48;
      body.put(
        Cage::rounded(
          head + Vec3::new(0.0, 0.0, -0.13),
          Vec3::new(0.022, 0.26, 0.2),
          UVec3::new(1, 3, 2),
          0.25
        )
        .shaped(|at| {
          let rise = (at.y - head.y) / 0.13;
          Vec3::new(
            at.x * (1.0 - (at.z - head.z + 0.03).min(0.0) * -4.0).max(0.15),
            at.y,
            at.z + rise * rise * 0.05
          )
        }),
        EDGE,
        bone
      );
      body.put(
        spike(
          head + Vec3::Z * 0.02,
          head + Vec3::new(0.0, -0.04, 0.16),
          Vec2::new(0.025, 0.012)
        ),
        trim,
        bone
      );
      body.put(ball(head, Vec3::new(0.03, 0.06, 0.04)), trim, bone)
    }
  }
}

fn champion(tribe: &Tribe, which: usize, hero: bool) -> Mesh {
  let armour = coat(tribe.armour, Finish::Paint);
  let trim = coat(tribe.trim, Finish::Paint);
  let glow = coat(tribe.glow, Finish::Glow);
  let size = if hero { Model::Paragon.size() } else { 1.0 };
  let mut body = Body { scale: size, pieces: Vec::new() };
  body.put(
    slab(Vec3::new(0.0, 1.03, 0.0), Vec3::new(0.3, 0.14, 0.18), 0.5),
    GUNMETAL,
    PELVIS
  );
  body.put(gear(Vec3::new(0.0, 1.07, 0.1), Vec3::Z, 0.075, 0.03, 8), STEEL, PELVIS);
  body.put(
    slab(Vec3::new(0.0, 0.99, -0.09), Vec3::new(0.12, 0.13, 0.045), 0.6),
    if hero { coat(tribe.trim, Finish::Gilt) } else { armour },
    PELVIS
  );
  for (cage, finish) in piston(Vec3::new(0.0, 1.1, 0.05), Vec3::new(0.0, 1.5, 0.05), 0.04)
  {
    body.put(cage, finish, CHEST)
  }
  body.put(
    shell(
      &[
        (Vec3::new(0.0, 1.15, 0.0), Vec2::new(0.07, 0.1)),
        (Vec3::new(0.0, 1.38, 0.0), Vec2::new(0.1, 0.2)),
        (Vec3::new(0.0, 1.62, 0.0), Vec2::new(0.1, 0.24))
      ],
      1
    ),
    GUNMETAL,
    CHEST
  );
  let bulk = if hero { 1.12 } else { 1.0 };
  body.paint(
    shell(
      &[
        (Vec3::new(0.0, 1.24, -0.03), Vec2::new(0.09, 0.12) * bulk),
        (Vec3::new(0.0, 1.42, -0.04), Vec2::new(0.13, 0.24) * bulk),
        (Vec3::new(0.0, 1.6, -0.03), Vec2::new(0.15, 0.3) * bulk),
        (Vec3::new(0.0, 1.68, 0.0), Vec2::new(0.11, 0.18))
      ],
      2
    ),
    move |at, normal| match () {
      () if !hero && at.x.abs() < 0.022 && normal.z < -0.5 => trim,
      () if hero && at.y < 1.27 => coat(tribe.trim, Finish::Gilt),
      () => armour
    },
    CHEST
  );
  if hero {
    body.put(
      orb(Vec3::new(0.0, 1.43, -0.168), Vec3::new(0.036, 0.036, 0.014)),
      coat(tribe.glow, Finish::Pulse),
      CHEST
    );
    body.put(
      ring(Vec3::new(0.0, 1.43, -0.163), Vec3::NEG_Z, 0.045, 0.009),
      coat(tribe.trim, Finish::Gilt),
      CHEST
    );
    for angle in [0.0f32, 0.25, 0.5, 0.75] {
      let out = Vec3::new((angle * TAU).cos(), (angle * TAU).sin(), 0.0);
      let base = Vec3::new(0.0, 1.43, -0.165) + out * 0.052;
      body.put(
        spike(base, base + out * 0.04 - Vec3::Z * 0.004, Vec2::new(0.012, 0.006)),
        coat(tribe.trim, Finish::Gilt),
        CHEST
      )
    }
  }
  body.put(
    slab(Vec3::new(0.0, 1.66, 0.01), Vec3::new(0.22, 0.08, 0.16), 0.6),
    GUNMETAL,
    CHEST
  );
  body.put(
    rod(Vec3::new(0.0, 1.63, 0.0), Vec3::new(0.0, 1.79, 0.0), 0.034, 8),
    STEEL,
    HEAD
  );
  body.put(
    ball(Vec3::new(0.0, 1.85, 0.025), Vec3::new(0.085, 0.1, 0.09)),
    GUNMETAL,
    HEAD
  );
  let helm = Vec3::new(0.0, 1.88, -0.01);
  match hero {
    false => {
      body.put(
        Cage::rounded(helm, Vec3::new(0.19, 0.26, 0.23), UVec3::splat(2), 0.6)
          .shaped(|at| {
            let up = ((at.y - helm.y + 0.13) / 0.26).clamp(0.0, 1.0);
            let forward = (helm.z - at.z).max(0.0);
            Vec3::new(
              at.x * (0.62 + up * 0.38),
              at.y - forward * 0.12,
              at.z - (1.0 - (at.y - helm.y).abs() * 8.0).max(0.0) * 0.012
            )
          })
          .subdivided(1),
        armour,
        HEAD
      );
      body.put(
        slab(helm + Vec3::new(0.0, 0.012, -0.104), Vec3::new(0.15, 0.05, 0.04), 0.7),
        coat(tribe.glow, Finish::Lens),
        HEAD
      );
      for side in [1.0f32, -1.0] {
        let at = helm + Vec3::new(side * 0.042, 0.016, -0.126);
        body.put(
          slab(Vec3::ZERO, Vec3::new(0.06, 0.016, 0.014), 0.6)
            .shaped(|point| at + Quat::from_rotation_z(side * 0.32) * point),
          glow,
          HEAD
        );
        body.put(
          spike(
            helm + Vec3::new(side * 0.08, -0.07, -0.03),
            helm + Vec3::new(side * 0.16, -0.13, 0.1),
            Vec2::new(0.026, 0.01)
          ),
          trim,
          HEAD
        )
      }
      body.put(
        Cage::tube(
          &[
            (helm + Vec3::new(0.0, 0.1, -0.08), Vec2::new(0.012, 0.03)),
            (helm + Vec3::new(0.0, 0.18, 0.01), Vec2::new(0.013, 0.055)),
            (helm + Vec3::new(0.0, 0.21, 0.12), Vec2::new(0.01, 0.045)),
            (helm + Vec3::new(0.0, 0.17, 0.23), Vec2::new(0.004, 0.01))
          ],
          6
        ),
        trim,
        HEAD
      )
    }
    true => {
      let radius = Vec3::new(0.105, 0.12, 0.11);
      body.put(
        Cage::rounded(helm, radius * 2.0, UVec3::splat(2), 0.78)
          .shaped(|at| {
            let brow = ((at.y - helm.y - 0.01) / 0.05).clamp(0.0, 1.0)
              * (1.0 - ((at.y - helm.y - 0.06) / 0.06).clamp(0.0, 1.0));
            let low = ((helm.y - at.y) / radius.y).max(0.0);
            Vec3::new(
              at.x * (1.0 - low * 0.3),
              at.y,
              at.z - brow * (helm.z - at.z).max(0.0) * 0.35
            )
          })
          .subdivided(1),
        armour,
        HEAD
      );
      for side in [1.0f32, -1.0] {
        let (at, normal) = on_egg(helm, radius * 0.98, side * 0.045, -0.005);
        body.put(
          turned(slab(Vec3::ZERO, Vec3::new(0.06, 0.036, 0.02), 0.8), at, normal),
          GUNMETAL,
          HEAD
        );
        body.put(
          turned(
            orb(Vec3::ZERO, Vec3::new(0.02, 0.015, 0.008)),
            at - normal * 0.004,
            normal
          ),
          glow,
          HEAD
        );
        body.put(
          spike(
            helm + Vec3::new(side * 0.06, -0.08, -0.07),
            helm + Vec3::new(side * 0.075, -0.17, -0.1),
            Vec2::new(0.024, 0.014)
          ),
          trim,
          HEAD
        )
      }
      let brow: Vec<(Vec3, Vec2)> = (0..=6)
        .map(|step| {
          let x = (step as f32 / 6.0 - 0.5) * 0.17;
          (
            on_egg(helm, radius * 1.03, x, 0.045 - (x / 0.1).powi(2) * 0.012).0,
            Vec2::splat(0.012)
          )
        })
        .collect();
      body.put(Cage::tube(&brow, 5), trim, HEAD);
      body.put(
        spike(
          on_egg(helm, radius, 0.0, 0.03).0,
          on_egg(helm, radius, 0.0, -0.05).0 - Vec3::Z * 0.02,
          Vec2::new(0.014, 0.02)
        ),
        trim,
        HEAD
      )
    }
  }
  let frame = Model::Gladiator.frame();
  let kind = [
    Weapon::Sword,
    Weapon::Trident,
    Weapon::Axe,
    Weapon::Sword,
    Weapon::Trident,
    Weapon::Axe
  ][which % 6];
  for side in [1.0f32, -1.0] {
    limbs(&mut body, &frame, side, &|body, limb| {
      let Limb {
        side,
        shoulder,
        elbow,
        wrist,
        hip,
        knee,
        bones: [upper, lower, thigh, shin]
      } = limb;
      let pad = shoulder + Vec3::new(side * 0.04, 0.08, 0.0);
      body.put(ball(shoulder, Vec3::splat(0.06)), STEEL, upper);
      body.put(
        Cage::rounded(pad, Vec3::new(0.21, 0.11, 0.25) * bulk, UVec3::splat(2), 0.6)
          .shaped(|at| at - Vec3::Y * ((at.x - pad.x) * side).max(0.0).powi(2) * 3.0),
        armour,
        upper
      );
      match hero {
        false => {
          for (from, to, thick) in [
            (Vec3::new(0.02, 0.03, -0.06), Vec3::new(0.12, 0.26, -0.02), 0.05),
            (Vec3::new(0.04, 0.03, 0.03), Vec3::new(0.17, 0.22, 0.13), 0.045),
            (Vec3::new(0.07, 0.01, 0.09), Vec3::new(0.16, 0.1, 0.24), 0.04)
          ] {
            let mirror = Vec3::new(side, 1.0, 1.0);
            body.put(
              spike(
                pad + from * mirror,
                pad + to * mirror,
                Vec2::new(thick, thick * 0.35)
              ),
              trim,
              upper
            )
          }
        }
        true => body.put(
          ring(
            pad + Vec3::new(side * 0.02, -0.02, 0.0),
            Vec3::new(side * 0.4, 1.0, 0.0),
            0.11,
            0.012
          ),
          coat(tribe.trim, Finish::Gilt),
          upper
        )
      }
      for (cage, finish) in piston(shoulder, elbow, 0.045) {
        body.put(cage, finish, upper)
      }
      if hero {
        body.put(
          slab(
            shoulder.lerp(elbow, 0.55) + Vec3::X * side * 0.03,
            Vec3::new(0.08, 0.17, 0.11),
            0.6
          ),
          armour,
          upper
        )
      }
      body.put(ball(elbow, Vec3::splat(0.05)), STEEL, lower);
      body.put(
        shell(
          &[
            (elbow - Vec3::Y * 0.03, Vec2::new(0.068, 0.064)),
            (elbow.lerp(wrist, 0.45), Vec2::new(0.085, 0.076)),
            (wrist + Vec3::Y * 0.04, Vec2::new(0.06, 0.056))
          ],
          1
        ),
        armour,
        lower
      );
      body.put(
        spike(
          elbow + Vec3::new(side * 0.05, -0.08, 0.02),
          elbow + Vec3::new(side * 0.1, 0.06, 0.07),
          Vec2::new(0.03, 0.01)
        ),
        trim,
        lower
      );
      for (cage, finish) in hand(wrist, side, 0.12) {
        body.put(cage, finish, lower)
      }
      if side > 0.0 {
        weapon(
          body,
          kind,
          wrist - Vec3::Y * 0.1 + Vec3::X * side * -0.01,
          tribe,
          which % 6 == 0,
          lower
        )
      }
      body.put(ball(hip, Vec3::splat(0.06)), STEEL, thigh);
      for (cage, finish) in piston(hip, knee, 0.05) {
        body.put(cage, finish, thigh)
      }
      body.put(
        slab(
          hip.lerp(knee, 0.4) + Vec3::new(side * 0.035, 0.0, -0.045),
          Vec3::new(0.1, 0.24, 0.08),
          0.6
        ),
        armour,
        thigh
      );
      body.put(ball(knee, Vec3::splat(0.055)), STEEL, shin);
      body.put(
        slab(knee + Vec3::new(0.0, 0.02, -0.07), Vec3::new(0.1, 0.13, 0.06), 0.6)
          .shaped(|at| at - Vec3::Z * (at.y - knee.y).max(0.0) * 0.3),
        trim,
        shin
      );
      body.put(
        shell(
          &[
            (knee - Vec3::Y * 0.05 - Vec3::Z * 0.02, Vec2::new(0.09, 0.08)),
            (knee - Vec3::Y * 0.17 - Vec3::Z * 0.025, Vec2::new(0.12, 0.1)),
            (Vec3::new(knee.x, 0.2, -0.01), Vec2::new(0.08, 0.075)),
            (Vec3::new(knee.x, 0.14, 0.0), Vec2::new(0.055, 0.055))
          ],
          1
        ),
        armour,
        shin
      );
      body.put(ball(Vec3::new(knee.x, 0.11, 0.0), Vec3::splat(0.045)), GUNMETAL, shin);
      match hero {
        false => {
          body.put(
            slab(Vec3::new(knee.x, 0.06, -0.02), Vec3::new(0.12, 0.08, 0.16), 0.5),
            GUNMETAL,
            shin
          );
          for (dx, length) in [(-0.045f32, 0.2f32), (0.0, 0.24), (0.045, 0.2)] {
            let base = Vec3::new(knee.x + dx, 0.05, -0.08);
            body.put(
              spike(
                base,
                base + Vec3::new(dx * 0.8, -0.045, -length),
                Vec2::new(0.035, 0.03)
              ),
              BRASS,
              shin
            )
          }
          body.put(
            spike(
              Vec3::new(knee.x, 0.05, 0.05),
              Vec3::new(knee.x, 0.0, 0.16),
              Vec2::new(0.03, 0.025)
            ),
            BRASS,
            shin
          )
        }
        true => {
          body.put(
            slab(Vec3::new(knee.x, 0.06, -0.05), Vec3::new(0.15, 0.11, 0.3), 0.5)
              .shaped(|at| at - Vec3::Y * ((-0.05 - at.z).max(0.0) * 0.2)),
            armour,
            shin
          );
          body.put(
            slab(Vec3::new(knee.x, 0.01, -0.05), Vec3::new(0.14, 0.02, 0.28), 0.3),
            GUNMETAL,
            shin
          )
        }
      }
    })
  }
  body.mesh()
}

pub fn mesh(model: Model, tribe: usize) -> Mesh {
  let kin = &TRIBES[tribe % TRIBES.len()];
  match model {
    Model::Cogling => cogling(kin),
    Model::Scavver => scavver(kin),
    Model::Gladiator => champion(kin, tribe, false),
    Model::Paragon => champion(kin, tribe, true)
  }
}

pub fn bindings(model: Model) -> Vec<Mat4> {
  let frame = model.frame();
  (0..BONES as u16).map(|bone| Mat4::from_translation(-frame.pivot(bone))).collect()
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct Plating {}

impl MaterialExtension for Plating {
  fn fragment_shader() -> ShaderRef { "embedded://bloxy/automaton.wgsl".into() }
}

pub type Plated = ExtendedMaterial<StandardMaterial, Plating>;

pub fn plated() -> Plated {
  ExtendedMaterial {
    base: StandardMaterial {
      perceptual_roughness: 0.4,
      reflectance: 0.5,
      anisotropy_strength: 0.01,
      ..default()
    },
    extension: Plating {}
  }
}

pub fn register(app: &mut App) {
  embedded_asset!(app, "automaton.wgsl");
  app.add_plugins(MaterialPlugin::<Plated>::default());
}

#[cfg(test)]
mod tests {
  use {super::*, crate::sculpt::sheet};

  fn preview(mesh: Mesh) -> Mesh {
    let normals: Vec<[f32; 3]> = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
      Some(VertexAttributeValues::Float32x4(codes)) => {
        codes.iter().map(|&[x, y, z, _]| [x, y, z]).collect()
      }
      _ => Vec::new()
    };
    let colors: Vec<[f32; 4]> = match mesh.attribute(Mesh::ATTRIBUTE_UV_1) {
      Some(VertexAttributeValues::Float32x2(codes)) => codes
        .iter()
        .map(|&[_, packed]| {
          let packed = packed as u32;
          let srgb = [packed >> 16 & 255, packed >> 8 & 255, packed & 255]
            .map(|channel| channel as f32 / 255.0);
          Color::srgb(srgb[0], srgb[1], srgb[2]).to_linear().to_f32_array()
        })
        .collect(),
      _ => Vec::new()
    };
    mesh
      .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
      .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
  }

  #[test]
  fn automatons_stay_within_a_triangle_budget() {
    for (model, tribe) in [
      (Model::Cogling, 0),
      (Model::Scavver, 4),
      (Model::Gladiator, 0),
      (Model::Paragon, 1)
    ] {
      let triangles = mesh(model, tribe).count_vertices() / 3;
      println!("{model:?}: {triangles} triangles");
      assert!(triangles < 3_000)
    }
  }

  #[test]
  #[ignore]
  fn automatons() {
    sheet(
      &[
        mesh(Model::Cogling, 0),
        mesh(Model::Scavver, 4),
        mesh(Model::Gladiator, 0),
        mesh(Model::Paragon, 1)
      ]
      .map(preview),
      "screenshots/automatons.ppm"
    )
  }
}
