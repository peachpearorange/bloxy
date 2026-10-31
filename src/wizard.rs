use {crate::{generate, player::Pilot, protocol::Role, voxels::Voxels},
     bevy::{asset::{RenderAssetUsages, embedded_asset},
            mesh::{Indices, PrimitiveTopology},
            pbr::{ExtendedMaterial, MaterialExtension},
            prelude::*,
            render::render_resource::AsBindGroup,
            shader::ShaderRef},
     std::f32::consts::{FRAC_PI_2, TAU}};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Finish {
  Robe,
  Mantle,
  Lining,
  Felt,
  Band,
  Leather,
  Gold,
  Wood,
  Gem,
  Skin,
  Hair,
  Slipper,
  Glass,
  Cork
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
  Body,
  Buckle,
  Collar,
  Head,
  Staff,
  Pouch,
  Shoe,
  Hat,
  Mote,
  Hand
}

#[derive(Clone, Copy)]
struct Ring {
  y: f32,
  wide: f32,
  deep: f32,
  x: f32,
  z: f32,
  pleat: f32,
  hard: bool,
  finish: Finish
}

impl Ring {
  const fn at(self, x: f32, z: f32) -> Self { Self { x, z, ..self } }

  const fn pleated(self, pleat: f32) -> Self { Self { pleat, ..self } }
}

const fn ring(y: f32, wide: f32, deep: f32, finish: Finish) -> Ring {
  Ring { y, wide, deep, x: 0.0, z: 0.0, pleat: 0.0, hard: false, finish }
}

const fn edge(y: f32, wide: f32, deep: f32, finish: Finish) -> Ring {
  Ring { hard: true, ..ring(y, wide, deep, finish) }
}

const fn round(y: f32, radius: f32, finish: Finish) -> Ring {
  ring(y, radius, radius, finish)
}

const fn rim(y: f32, radius: f32, finish: Finish) -> Ring {
  edge(y, radius, radius, finish)
}

use Finish::*;

const ROBE: [Ring; 11] = [
  ring(0.07, 0.30, 0.29, Lining).pleated(0.06),
  edge(0.025, 0.41, 0.39, Lining).pleated(0.09),
  edge(0.025, 0.41, 0.39, Robe).pleated(0.09),
  ring(0.12, 0.39, 0.37, Robe).pleated(0.085),
  ring(0.45, 0.31, 0.28, Robe).pleated(0.06),
  ring(0.86, 0.225, 0.175, Robe).pleated(0.02),
  ring(1.02, 0.21, 0.155, Robe),
  ring(1.2, 0.225, 0.15, Robe),
  ring(1.33, 0.2, 0.13, Robe),
  ring(1.41, 0.09, 0.085, Robe),
  ring(1.45, 0.0, 0.0, Robe)
];

const BELT: [Ring; 4] = [
  edge(0.895, 0.2, 0.15, Leather),
  edge(0.895, 0.236, 0.18, Leather),
  ring(0.93, 0.24, 0.184, Leather),
  edge(0.965, 0.232, 0.176, Leather)
];

const BUCKLE: [Ring; 5] = [
  edge(-0.006, 0.042, 0.034, Gold),
  edge(0.006, 0.044, 0.036, Gold),
  ring(0.011, 0.04, 0.032, Gold),
  edge(0.013, 0.024, 0.017, Gold),
  ring(0.008, 0.0, 0.0, Gold)
];

const POUCH: [Ring; 7] = [
  edge(0.70, 0.0, 0.0, Leather),
  edge(0.70, 0.05, 0.03, Leather),
  ring(0.76, 0.064, 0.04, Leather),
  ring(0.84, 0.06, 0.037, Leather),
  edge(0.84, 0.068, 0.045, Leather),
  ring(0.9, 0.06, 0.04, Leather),
  ring(0.925, 0.0, 0.0, Leather)
];

const FLASK: [Ring; 9] = [
  edge(0.705, 0.0, 0.0, Glass),
  ring(0.71, 0.03, 0.03, Glass),
  ring(0.735, 0.042, 0.042, Glass),
  ring(0.77, 0.036, 0.036, Glass),
  ring(0.79, 0.013, 0.013, Glass),
  edge(0.825, 0.012, 0.012, Glass),
  edge(0.822, 0.015, 0.015, Cork),
  ring(0.85, 0.016, 0.016, Cork),
  ring(0.855, 0.0, 0.0, Cork)
];

const MANTLE: [Ring; 7] = [
  ring(1.13, 0.22, 0.16, Lining).pleated(0.05),
  edge(1.08, 0.3, 0.235, Lining).pleated(0.08),
  edge(1.08, 0.3, 0.235, Mantle).pleated(0.08),
  ring(1.2, 0.29, 0.22, Mantle).pleated(0.06),
  ring(1.33, 0.255, 0.18, Mantle).pleated(0.02),
  ring(1.41, 0.14, 0.12, Mantle),
  ring(1.445, 0.075, 0.075, Mantle)
];

const COLLAR: [Ring; 5] = [
  ring(0.0, 0.085, 0.075, Mantle).at(0.0, 0.01),
  ring(0.09, 0.12, 0.09, Mantle).at(0.0, 0.045),
  edge(0.18, 0.19, 0.115, Mantle).at(0.0, 0.1),
  edge(0.18, 0.175, 0.1, Lining).at(0.0, 0.1),
  ring(0.1, 0.11, 0.075, Lining).at(0.0, 0.06)
];

const HEAD: [Ring; 7] = [
  ring(0.0, 0.055, 0.055, Skin),
  ring(0.05, 0.09, 0.098, Skin),
  ring(0.11, 0.104, 0.112, Skin),
  ring(0.17, 0.104, 0.11, Skin),
  ring(0.22, 0.088, 0.094, Skin),
  ring(0.255, 0.05, 0.055, Skin),
  ring(0.27, 0.0, 0.0, Skin)
];

const NOSE: [Ring; 5] = [
  round(-0.01, 0.017, Skin),
  round(0.025, 0.02, Skin),
  round(0.052, 0.024, Skin),
  round(0.072, 0.016, Skin),
  round(0.08, 0.0, Skin)
];

const BROW: [Ring; 4] = [
  ring(0.0, 0.016, 0.009, Hair),
  ring(0.035, 0.02, 0.012, Hair),
  ring(0.07, 0.012, 0.008, Hair),
  ring(0.1, 0.0, 0.0, Hair)
];

const MOUSTACHE: [Ring; 4] = [
  round(0.0, 0.016, Hair),
  round(0.045, 0.019, Hair),
  round(0.1, 0.011, Hair),
  round(0.135, 0.0, Hair)
];

const BEARD: [Ring; 8] = [
  ring(0.8, 0.0, 0.0, Hair).at(0.0, -0.215),
  ring(0.86, 0.03, 0.02, Hair).at(0.0, -0.21).pleated(0.15),
  ring(1.0, 0.07, 0.04, Hair).at(0.0, -0.205).pleated(0.15),
  ring(1.17, 0.105, 0.052, Hair).at(0.0, -0.195).pleated(0.12),
  ring(1.33, 0.12, 0.06, Hair).at(0.0, -0.16).pleated(0.1),
  ring(1.45, 0.112, 0.06, Hair).at(0.0, -0.115).pleated(0.06),
  ring(1.53, 0.11, 0.055, Hair).at(0.0, -0.085),
  ring(1.555, 0.0, 0.0, Hair).at(0.0, -0.08)
];

const BEAD: [Ring; 4] = [
  edge(0.955, 0.068, 0.048, Gold).at(0.0, -0.207),
  ring(0.975, 0.08, 0.056, Gold).at(0.0, -0.207),
  ring(0.995, 0.08, 0.056, Gold).at(0.0, -0.207),
  edge(1.015, 0.07, 0.05, Gold).at(0.0, -0.207)
];

const BRIM: [Ring; 8] = [
  edge(-0.01, 0.12, 0.12, Felt),
  ring(-0.024, 0.24, 0.24, Felt).pleated(0.025),
  edge(-0.03, 0.315, 0.315, Felt).pleated(0.05),
  edge(-0.016, 0.32, 0.32, Felt).pleated(0.05),
  ring(-0.008, 0.29, 0.29, Felt).pleated(0.04),
  ring(0.005, 0.2, 0.2, Felt).pleated(0.015),
  ring(0.02, 0.15, 0.15, Felt),
  ring(0.03, 0.135, 0.135, Felt)
];

const CROWN: [Ring; 8] = [
  edge(0.03, 0.138, 0.138, Band),
  ring(0.052, 0.136, 0.136, Band),
  edge(0.076, 0.131, 0.131, Band),
  edge(0.076, 0.127, 0.127, Felt),
  ring(0.2, 0.1, 0.1, Felt).at(0.0, 0.01).pleated(0.05),
  ring(0.33, 0.075, 0.075, Felt).at(-0.003, 0.035).pleated(0.06),
  ring(0.45, 0.052, 0.052, Felt).at(-0.008, 0.07).pleated(0.05),
  ring(0.54, 0.037, 0.037, Felt).at(-0.016, 0.11).pleated(0.04)
];

const SLUMP: [Ring; 5] = [
  round(-0.03, 0.04, Felt),
  round(0.06, 0.03, Felt),
  round(0.13, 0.02, Felt),
  round(0.19, 0.011, Felt),
  round(0.215, 0.0, Felt)
];

const BELL: [Ring; 5] = [
  round(-0.016, 0.0, Gold),
  round(-0.012, 0.012, Gold),
  round(0.0, 0.017, Gold),
  round(0.012, 0.012, Gold),
  round(0.016, 0.0, Gold)
];

const SLEEVE: [Ring; 7] = [
  round(0.0, 0.07, Robe),
  round(0.12, 0.078, Robe),
  ring(0.24, 0.095, 0.09, Robe).pleated(0.04),
  edge(0.33, 0.118, 0.11, Robe).pleated(0.08),
  edge(0.33, 0.112, 0.104, Lining).pleated(0.08),
  ring(0.29, 0.09, 0.086, Lining).pleated(0.04),
  ring(0.2, 0.065, 0.065, Lining)
];

const WRIST: [Ring; 3] =
  [round(0.2, 0.04, Skin), round(0.3, 0.036, Skin), round(0.4, 0.033, Skin)];

const FIST: [Ring; 6] = [
  edge(0.965, 0.0, 0.0, Skin),
  edge(0.965, 0.03, 0.028, Skin),
  ring(0.975, 0.043, 0.038, Skin).at(0.006, 0.0),
  ring(1.035, 0.046, 0.04, Skin).at(0.006, 0.0),
  ring(1.05, 0.034, 0.03, Skin),
  edge(1.055, 0.0, 0.0, Skin)
];

const PALM: [Ring; 5] = [
  edge(-0.014, 0.0, 0.0, Skin),
  edge(-0.014, 0.036, 0.05, Skin),
  ring(0.0, 0.043, 0.062, Skin).at(0.0, -0.008),
  ring(0.014, 0.038, 0.058, Skin).at(0.0, -0.01),
  edge(0.016, 0.0, 0.0, Skin)
];

const THUMB: [Ring; 3] =
  [round(0.0, 0.013, Skin), round(0.035, 0.012, Skin), round(0.052, 0.0, Skin)];

const SHOE: [Ring; 9] = [
  ring(0.0, 0.0, 0.0, Slipper).at(0.0, 0.045),
  ring(0.015, 0.042, 0.04, Slipper).at(0.0, 0.045),
  ring(0.08, 0.052, 0.046, Slipper).at(0.0, 0.046),
  ring(0.17, 0.05, 0.042, Slipper).at(0.0, 0.042),
  ring(0.25, 0.04, 0.031, Slipper).at(0.0, 0.034),
  ring(0.31, 0.026, 0.02, Slipper).at(0.0, 0.034),
  ring(0.35, 0.016, 0.013, Slipper).at(0.0, 0.05),
  ring(0.37, 0.009, 0.008, Slipper).at(0.0, 0.075),
  ring(0.372, 0.0, 0.0, Slipper).at(0.0, 0.098)
];

const STAFF: [Ring; 13] = [
  edge(0.0, 0.0, 0.0, Gold),
  ring(0.006, 0.015, 0.015, Gold),
  rim(0.05, 0.02, Gold),
  rim(0.05, 0.018, Wood),
  ring(0.42, 0.022, 0.02, Wood),
  ring(0.5, 0.025, 0.023, Wood).at(0.002, 0.0),
  ring(0.58, 0.021, 0.02, Wood),
  ring(1.3, 0.02, 0.021, Wood).at(-0.003, 0.002),
  ring(1.62, 0.024, 0.023, Wood),
  edge(1.74, 0.03, 0.03, Wood),
  edge(1.74, 0.034, 0.034, Gold),
  ring(1.765, 0.038, 0.038, Gold),
  edge(1.79, 0.03, 0.03, Gold)
];

const PRONG: [Ring; 6] = [
  round(1.77, 0.015, Wood).at(0.012, 0.0),
  round(1.84, 0.013, Wood).at(0.045, 0.0),
  round(1.92, 0.012, Wood).at(0.07, 0.0),
  round(2.0, 0.011, Wood).at(0.06, 0.0),
  round(2.06, 0.008, Wood).at(0.025, 0.0),
  round(2.08, 0.0, Wood).at(0.004, 0.0)
];

const GEM: [Ring; 5] = [
  edge(-0.085, 0.0, 0.0, Gem),
  edge(-0.03, 0.036, 0.036, Gem),
  edge(0.015, 0.046, 0.046, Gem),
  edge(0.06, 0.03, 0.03, Gem),
  edge(0.1, 0.0, 0.0, Gem)
];

const MOTE: [Ring; 3] =
  [edge(-0.03, 0.0, 0.0, Gem), edge(0.0, 0.026, 0.026, Gem), edge(0.03, 0.0, 0.0, Gem)];

struct Lathe {
  rings: &'static [Ring],
  sides: usize,
  square: f32,
  faceted: bool,
  part: Part,
  place: Transform
}

struct Corner {
  at: Vec3,
  normal: Vec3,
  code: f32,
  arc: f32
}

fn bent(angle: f32, square: f32) -> Vec2 {
  let (sin, cos) = angle.sin_cos();
  let curve = |value: f32| value.signum() * value.abs().powf(2.0 / square);
  Vec2::new(curve(cos), curve(sin))
}

fn strips(rings: &[Ring]) -> Vec<(usize, usize)> {
  let breaks: Vec<usize> = (1..rings.len() - 1)
    .filter(|&index| rings[index].hard || rings[index].finish != rings[index - 1].finish)
    .collect();
  std::iter::once(0)
    .chain(breaks.iter().copied())
    .zip(breaks.iter().copied().chain(std::iter::once(rings.len() - 1)))
    .collect()
}

fn turned(lathe: &Lathe) -> Vec<Corner> {
  let &Lathe { rings, sides, square, faceted, part, place } = lathe;
  let around: Vec<Vec2> = (0..sides)
    .map(|side| bent(side as f32 / sides as f32 * TAU - FRAC_PI_2, square))
    .collect();
  let point = |ring: &Ring, side: usize| {
    let Vec2 { x, y } = around[side % sides];
    let fold = 1.0 + ring.pleat * if side % 2 == 0 { 1.0 } else { -1.0 };
    Vec3::new(x * ring.wide * fold + ring.x, ring.y, y * ring.deep * fold + ring.z)
  };
  let middle = |ring: &Ring| Vec3::new(ring.x, ring.y, ring.z);
  let arcs =
    rings.iter().skip(1).zip(rings).fold(vec![0.0], |mut arcs, (ring, before)| {
      arcs.push(
        arcs.last().copied().unwrap_or(0.0) + middle(ring).distance(middle(before))
      );
      arcs
    });
  strips(rings)
    .into_iter()
    .flat_map(|(from, to)| {
      let triangles: Vec<[(usize, usize); 3]> = (from..to)
        .flat_map(|low| {
          (0..sides).flat_map(move |side| {
            let [a, b, c, d] =
              [(low, side), (low + 1, side), (low + 1, side + 1), (low, side + 1)];
            [[a, b, c], [a, c, d]]
          })
        })
        .filter(|triangle| {
          let [a, b, c] = triangle.map(|(ring, side)| point(&rings[ring], side));
          (b - a).cross(c - a).length_squared() > 1e-12
        })
        .collect();
      let facet = |triangle: &[(usize, usize); 3]| {
        let [a, b, c] = triangle.map(|(ring, side)| point(&rings[ring], side));
        (b - a).cross(c - a)
      };
      let smooth = triangles.iter().fold(
        vec![Vec3::ZERO; (to - from + 1) * sides],
        |mut normals, triangle| {
          for &(ring, side) in triangle {
            normals[(ring - from) * sides + side % sides] += facet(triangle)
          }
          normals
        }
      );
      let code = (rings[from].finish as u32 + 16 * part as u32) as f32;
      triangles
        .into_iter()
        .flat_map(|triangle| {
          let flat = facet(&triangle);
          triangle.map(|(ring, side)| Corner {
            at: place.transform_point(point(&rings[ring], side)),
            normal: place.rotation
              * match faceted {
                true => flat,
                false => smooth[(ring - from) * sides + side % sides]
              }
              .normalize_or(Vec3::Y),
            code,
            arc: arcs[ring]
          })
        })
        .collect::<Vec<_>>()
    })
    .collect()
}

pub const GEM_AT: Vec3 = Vec3::new(0.42, 1.93, -0.26);
pub const MOTE_AT: Vec3 = Vec3::new(-0.28, 1.17, -0.35);

fn lathes() -> Vec<Lathe> {
  let upright = |at: Vec3| Transform::from_translation(at);
  let along = |at: Vec3, toward: Vec3| {
    Transform::from_translation(at)
      .with_rotation(Quat::from_rotation_arc(Vec3::Y, toward.normalize()))
  };
  let lathe = |rings: &'static [Ring], sides, square, part, place| Lathe {
    rings,
    sides,
    square,
    faceted: false,
    part,
    place
  };
  let staff = Vec3::new(GEM_AT.x, 0.0, GEM_AT.z);
  let hat =
    Transform::from_xyz(0.0, 1.625, 0.0).with_rotation(Quat::from_rotation_x(0.1));
  let crest = hat.transform_point(Vec3::new(-0.016, 0.54, 0.11));
  let droop = hat.rotation * Vec3::new(-0.25, 0.25, 1.0);
  let tip = crest + droop.normalize() * 0.2;
  let shoulder = |side: f32| Vec3::new(side * 0.215, 1.32, 0.0);
  let fist = staff + Vec3::new(-0.01, 1.07, 0.03);
  let palm = MOTE_AT - Vec3::Y * 0.11;
  let sided = |side: f32| {
    [
      lathe(
        &SHOE,
        8,
        2.4,
        Part::Shoe,
        along(Vec3::new(side * 0.105, 0.0, -0.17), Vec3::NEG_Z)
      ),
      lathe(
        &BROW,
        4,
        1.6,
        Part::Head,
        along(Vec3::new(side * 0.03, 1.588, -0.1), Vec3::new(side, 0.22, -0.3))
      ),
      lathe(
        &MOUSTACHE,
        6,
        2.0,
        Part::Head,
        along(Vec3::new(side * 0.012, 1.522, -0.14), Vec3::new(side, -0.7, -0.25))
      )
    ]
  };
  let prong = |turn: f32| {
    lathe(
      &PRONG,
      5,
      2.0,
      Part::Staff,
      Transform::from_translation(staff).with_rotation(Quat::from_rotation_y(turn))
    )
  };
  [
    lathe(&ROBE, 16, 2.0, Part::Body, upright(Vec3::ZERO)),
    lathe(&BELT, 16, 2.4, Part::Body, upright(Vec3::ZERO)),
    lathe(&BUCKLE, 4, 4.0, Part::Buckle, along(Vec3::new(0.0, 0.93, -0.18), Vec3::NEG_Z)),
    lathe(
      &POUCH,
      6,
      2.6,
      Part::Pouch,
      Transform::from_xyz(-0.17, 0.0, -0.13).with_rotation(Quat::from_rotation_y(-0.75))
    ),
    lathe(&FLASK, 8, 2.0, Part::Body, upright(Vec3::new(0.2, 0.0, 0.11))),
    lathe(&MANTLE, 14, 2.2, Part::Body, upright(Vec3::ZERO)),
    lathe(&COLLAR, 12, 2.0, Part::Collar, upright(Vec3::Y * 1.4)),
    lathe(&HEAD, 10, 2.0, Part::Head, upright(Vec3::Y * 1.44)),
    lathe(
      &NOSE,
      6,
      2.0,
      Part::Head,
      along(Vec3::new(0.0, 1.565, -0.1), Vec3::new(0.0, -0.45, -1.0))
    ),
    lathe(&BEARD, 12, 2.0, Part::Head, upright(Vec3::ZERO)),
    lathe(&BEAD, 10, 2.0, Part::Body, upright(Vec3::ZERO)),
    lathe(&BRIM, 16, 2.0, Part::Hat, hat),
    lathe(&CROWN, 10, 2.0, Part::Hat, hat),
    lathe(&SLUMP, 8, 2.0, Part::Hat, along(crest, droop)),
    lathe(&BELL, 6, 2.0, Part::Body, upright(tip + Vec3::new(0.0, -0.022, 0.0))),
    lathe(
      &SLEEVE,
      12,
      2.0,
      Part::Body,
      along(shoulder(1.0), fist + Vec3::Y * 0.04 - shoulder(1.0))
    ),
    lathe(&SLEEVE, 12, 2.0, Part::Body, along(shoulder(-1.0), palm - shoulder(-1.0))),
    lathe(&WRIST, 6, 2.0, Part::Hand, along(shoulder(-1.0), palm - shoulder(-1.0))),
    lathe(&FIST, 8, 3.0, Part::Hand, upright(staff)),
    lathe(&PALM, 8, 2.6, Part::Hand, upright(palm)),
    lathe(
      &THUMB,
      5,
      2.0,
      Part::Hand,
      along(palm + Vec3::new(0.035, 0.008, 0.0), Vec3::new(0.6, 0.5, -0.6))
    ),
    lathe(&STAFF, 8, 2.0, Part::Staff, upright(staff)),
    prong(0.0),
    prong(TAU / 3.0),
    prong(2.0 * TAU / 3.0),
    Lathe { faceted: true, ..lathe(&GEM, 6, 2.0, Part::Staff, upright(GEM_AT)) },
    Lathe {
      faceted: true,
      ..lathe(
        &MOTE,
        4,
        2.0,
        Part::Mote,
        upright(MOTE_AT).with_rotation(Quat::from_rotation_y(0.4))
      )
    }
  ]
  .into_iter()
  .chain(sided(1.0))
  .chain(sided(-1.0))
  .collect()
}

pub fn mesh() -> Mesh {
  let corners: Vec<Corner> = lathes().iter().flat_map(turned).collect();
  let count = corners.len();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      corners.iter().map(|corner| corner.at.to_array()).collect::<Vec<_>>()
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
        .map(|corner| corner.normal.extend(corner.code).to_array())
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_0,
      corners.iter().map(|corner| [corner.at.x, corner.at.y]).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_1,
      corners.iter().map(|corner| [corner.at.z, corner.arc]).collect::<Vec<_>>()
    )
    .with_inserted_indices(Indices::U32((0..count as u32).collect()))
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct Weave {}

impl MaterialExtension for Weave {
  fn fragment_shader() -> ShaderRef { "embedded://bloxy/wizard.wgsl".into() }
}

pub type Woven = ExtendedMaterial<StandardMaterial, Weave>;

fn woven() -> Woven {
  ExtendedMaterial {
    base: StandardMaterial {
      perceptual_roughness: 0.6,
      anisotropy_strength: 0.01,
      double_sided: true,
      cull_mode: None,
      ..default()
    },
    extension: Weave {}
  }
}

fn stand(
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  mut done: Local<bool>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<Woven>>,
  mut commands: Commands
) {
  if !*done
    && pilot.at != Vec3::ZERO
    && let Some(voxels) = voxels
    && let spot = pilot.at + Vec3::new(-1.8, 0.0, 1.6)
    && let column = spot.floor().as_ivec3()
    && let high = generate::height(voxels.seed, column.x, column.z) + 4
    && let Some(ground) = (0..12)
      .map(|depth| column.with_y(high - depth))
      .find(|&at| voxels.block(at).is_some_and(|block| block.solid() && !block.leafy()))
  {
    *done = true;
    let at = spot.with_y(ground.y as f32 + 1.0);
    let toward = pilot.at - at;
    let yaw = (-toward.x).atan2(-toward.z);
    info!("The wizard stands at {at:?}, facing yaw {yaw}");
    commands.spawn((
      Mesh3d(meshes.add(mesh())),
      MeshMaterial3d(materials.add(woven())),
      Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw))
    ));
  }
}

pub struct Wizards;

impl Plugin for Wizards {
  fn build(&self, app: &mut App) {
    if app.world().resource::<Role>().plays() {
      embedded_asset!(app, "wizard.wgsl");
      app.add_plugins(MaterialPlugin::<Woven>::default()).add_systems(
        Update,
        stand.run_if(|| crate::opts::opts().wizard).run_if(resource_exists::<Pilot>)
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wizard_stays_within_a_triangle_budget() {
    let triangles = lathes().iter().map(|lathe| turned(lathe).len() / 3).sum::<usize>();
    println!("wizard: {triangles} triangles");
    assert!((1500..3000).contains(&triangles))
  }
}
