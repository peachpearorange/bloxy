use {crate::{cage::Cage,
             noise::hash,
             protocol::Band,
             sculpt::{merged, sculpt}},
     bevy::prelude::*,
     std::f32::consts::{FRAC_PI_2, PI}};

pub const VARIANTS: u32 = 16;
const SKINS: [[f32; 3]; 3] = [[0.93, 0.74, 0.6], [0.8, 0.57, 0.41], [0.52, 0.35, 0.24]];
const HAIRS: [[f32; 3]; 5] =
  [[0.88, 0.72, 0.38], [0.62, 0.25, 0.1], [0.36, 0.22, 0.12], [0.12, 0.1, 0.09], [
    0.6, 0.58, 0.55
  ]];
const LINEN: [f32; 3] = [0.9, 0.86, 0.76];
const IRON: [f32; 3] = [0.5, 0.51, 0.55];
const STEEL: [f32; 3] = [0.78, 0.8, 0.84];
const GOLD: [f32; 3] = [0.86, 0.66, 0.2];
const WOOD: [f32; 3] = [0.45, 0.29, 0.15];
const LEATHER: [f32; 3] = [0.36, 0.22, 0.12];
const FUR: [f32; 3] = [0.55, 0.45, 0.33];
const BONE: [f32; 3] = [0.9, 0.86, 0.72];
const SOOT: [f32; 3] = [0.09, 0.08, 0.08];
const EYE: [f32; 3] = [0.08, 0.06, 0.06];
const HEAD: Vec3 = Vec3::new(0.0, 1.69, 0.0);

struct Rig {
  pirate: bool,
  skin: [f32; 3],
  hair: [f32; 3],
  shirt: [f32; 3],
  trim: [f32; 3],
  legs: [f32; 3],
  roll: u32
}

impl Rig {
  fn of(band: Band, variant: u32) -> Rig {
    let pick = |salt: i32, count: usize| hash(variant, salt, 7, 11) as usize % count;
    let pirate = band == Band::Pirate;
    let shirts: &[[f32; 3]] = match pirate {
      true => &[LINEN, [0.62, 0.12, 0.1], [0.16, 0.2, 0.38], LINEN, LINEN],
      false => {
        &[[0.25, 0.4, 0.22], [0.2, 0.3, 0.52], [0.55, 0.16, 0.12], [0.45, 0.36, 0.24], [
          0.42, 0.42, 0.4
        ]]
      }
    };
    let trims: &[[f32; 3]] = match pirate {
      true => &[[0.7, 0.1, 0.1], [0.15, 0.2, 0.42], SOOT],
      false => &[[0.75, 0.6, 0.25], [0.6, 0.15, 0.12], [0.85, 0.82, 0.7]]
    };
    let legs: &[[f32; 3]] = match pirate {
      true => &[[0.3, 0.25, 0.2], [0.2, 0.2, 0.26], [0.55, 0.5, 0.42]],
      false => &[[0.4, 0.33, 0.26], [0.3, 0.3, 0.32], [0.45, 0.4, 0.32]]
    };
    Rig {
      pirate,
      skin: SKINS[pick(1, SKINS.len())],
      hair: HAIRS[pick(2, HAIRS.len())],
      shirt: shirts[pick(3, shirts.len())],
      trim: trims[pick(4, trims.len())],
      legs: legs[pick(5, legs.len())],
      roll: hash(variant, 9, 13, 17)
    }
  }

  fn chance(&self, salt: u32, in_every: u32) -> bool {
    hash(self.roll, salt as i32, 3, 1) % in_every == 0
  }

  fn striped(&self) -> bool { self.pirate && self.shirt == LINEN && self.chance(1, 2) }
}

fn skull() -> Cage {
  Cage::rounded(HEAD, Vec3::new(0.25, 0.28, 0.27), UVec3::splat(2), 0.6)
    .shaped(|at| {
      let jaw = ((HEAD.y - at.y) / 0.14).clamp(0.0, 1.0);
      Vec3::new(at.x * (1.0 - jaw * 0.15), at.y, at.z)
    })
    .subdivided(1)
}

fn face(skull: &Cage, x: f32, y: f32) -> Vec3 {
  let z = skull
    .points
    .iter()
    .filter(|at| Vec2::new(at.x - x, at.y - y).length() < 0.04)
    .map(|at| at.z)
    .fold(0.0, f32::min);
  Vec3::new(x, y, z)
}

fn tube(path: &[(f32, f32, f32, f32)], side: f32, sides: u32) -> Cage {
  Cage::tube(
    &path
      .iter()
      .map(|&(x, y, z, thick)| (Vec3::new(side * x, y, z), Vec2::splat(thick)))
      .collect::<Vec<_>>(),
    sides
  )
}

fn box_at(middle: Vec3, size: Vec3, round: f32) -> Cage {
  Cage::rounded(middle, size, UVec3::ONE, round)
}

fn head(rig: &Rig) -> Vec<(Cage, [f32; 3])> {
  let skull = skull();
  let on = |x: f32, y: f32| face(&skull, x, y);
  let patched = rig.pirate && rig.chance(2, 3);
  let eye = |side: f32| {
    let color = if patched && side < 0.0 { SOOT } else { EYE };
    let size = if patched && side < 0.0 {
      Vec3::new(0.06, 0.05, 0.02)
    } else {
      Vec3::new(0.032, 0.03, 0.012)
    };
    (box_at(on(side * 0.055, 1.71) + Vec3::Z * 0.002, size, 0.3), color)
  };
  let brow = |side: f32| {
    (
      box_at(on(side * 0.058, 1.75) - Vec3::Z * 0.004, Vec3::new(0.06, 0.018, 0.02), 0.3),
      rig.hair
    )
  };
  let nose = {
    let root = on(0.0, 1.7);
    (
      Cage::tube(
        &[
          (root + Vec3::Z * 0.01, Vec2::new(0.02, 0.022)),
          (root + Vec3::new(0.0, -0.035, -0.035), Vec2::new(0.022, 0.026))
        ],
        4
      ),
      rig.skin
    )
  };
  let ears = [1.0, -1.0].map(|side| {
    (
      box_at(Vec3::new(side * 0.125, 1.69, 0.01), Vec3::new(0.03, 0.06, 0.045), 0.5),
      rig.skin
    )
  });
  let bearded = !rig.pirate || rig.chance(3, 2);
  let beard = match (bearded, rig.pirate) {
    (false, _) => vec![(
      box_at(on(0.0, 1.62) - Vec3::Z * 0.004, Vec3::new(0.07, 0.012, 0.012), 0.2),
      EYE
    )],
    (true, true) => vec![
      (
        Cage::rounded(
          Vec3::new(0.0, 1.58, -0.07),
          Vec3::new(0.2, 0.1, 0.14),
          UVec3::ONE,
          0.5
        ),
        rig.hair
      ),
      (
        Cage::tube(
          &[(-0.05, 1.635), (0.0, 1.645), (0.05, 1.635)]
            .map(|(x, y)| (on(x, y) - Vec3::Z * 0.01, Vec2::new(0.012, 0.016))),
          4
        ),
        rig.hair
      ),
    ],
    (true, false) => {
      let middle = Vec3::new(0.0, 1.55, -0.08);
      vec![(
        Cage::rounded(middle, Vec3::new(0.23, 0.22, 0.14), UVec3::new(2, 2, 1), 0.4)
          .shaped(move |at| {
            let low = ((middle.y + 0.04 - at.y) / 0.14).clamp(0.0, 1.0);
            Vec3::new(at.x * (1.0 - low * 0.7), at.y - low * 0.05, at.z - low * 0.03)
          }),
        rig.hair
      )]
    }
  };
  let hat = match (rig.pirate, rig.chance(4, 2)) {
    (true, true) => {
      let brim = Cage::prism(3, 0.26, 0.03).shaped(|at| {
        let turned = Quat::from_rotation_y(FRAC_PI_2).mul_vec3(at);
        turned + Vec3::Y * (1.82 + turned.xz().length() * 0.2)
      });
      vec![
        (box_at(Vec3::new(0.0, 1.87, 0.0), Vec3::new(0.25, 0.12, 0.26), 0.5), SOOT),
        (brim, SOOT),
        (
          box_at(on(0.0, 1.86) - Vec3::Z * 0.035, Vec3::new(0.045, 0.045, 0.015), 0.3),
          BONE
        ),
      ]
    }
    (true, false) => vec![
      (
        Cage::rounded(
          Vec3::new(0.0, 1.78, 0.005),
          Vec3::new(0.27, 0.17, 0.29),
          UVec3::new(2, 1, 2),
          0.6
        )
        .shaped(|at| at.with_y(at.y.max(1.755))),
        rig.trim
      ),
      (
        Cage::spike(
          Vec3::new(0.02, 1.76, 0.13),
          Vec3::new(0.06, 1.62, 0.2),
          Vec2::new(0.025, 0.012)
        ),
        rig.trim
      ),
    ],
    (false, horned) => {
      let horn = |side: f32| {
        (
          Cage::tube(
            &[
              (Vec3::new(side * 0.13, 1.8, 0.0), Vec2::splat(0.04)),
              (Vec3::new(side * 0.24, 1.86, -0.02), Vec2::splat(0.03)),
              (Vec3::new(side * 0.28, 1.98, -0.05), Vec2::splat(0.004))
            ],
            4
          ),
          BONE
        )
      };
      [
        vec![
          (
            Cage::rounded(
              Vec3::new(0.0, 1.79, 0.0),
              Vec3::new(0.28, 0.2, 0.29),
              UVec3::new(2, 1, 2),
              0.6
            )
            .shaped(|at| at.with_y(at.y.max(1.76))),
            IRON
          ),
          (
            Cage::tube(
              &[
                (Vec3::new(0.0, 1.77, -0.15), Vec2::new(0.012, 0.022)),
                (Vec3::new(0.0, 1.66, -0.16), Vec2::new(0.012, 0.016))
              ],
              4
            ),
            IRON
          ),
        ],
        match horned {
          true => vec![horn(1.0), horn(-1.0)],
          false => vec![]
        }
      ]
      .concat()
    }
  };
  let locks = match rig.pirate {
    true => vec![],
    false => vec![(
      Cage::rounded(
        Vec3::new(0.0, 1.62, 0.1),
        Vec3::new(0.24, 0.24, 0.08),
        UVec3::ONE,
        0.5
      ),
      rig.hair
    )]
  };
  [
    vec![(skull.clone(), rig.skin), eye(1.0), eye(-1.0), brow(1.0), brow(-1.0), nose],
    ears.to_vec(),
    beard,
    hat,
    locks
  ]
  .concat()
}

const STRIPE: f32 = 0.074;
const WAIST: f32 = 0.74;
const WRAP: f32 = 0.05;

fn along(profile: &[(f32, Vec4)], heights: &[f32]) -> Vec<(f32, Vec4)> {
  heights
    .iter()
    .map(|&y| {
      let shape = profile
        .windows(2)
        .find(|pair| (pair[0].0.min(pair[1].0)..=pair[0].0.max(pair[1].0)).contains(&y))
        .map_or(profile[0].1, |pair| {
          pair[0].1.lerp(pair[1].1, (y - pair[0].0) / (pair[1].0 - pair[0].0))
        });
      (y, shape)
    })
    .collect()
}

fn torso(rig: &Rig) -> Cage {
  let bulk = if rig.pirate { 1.0 } else { 1.1 };
  let profile = [
    (0.74, 0.11, 0.17),
    (0.9, 0.12, 0.18),
    (1.15, 0.12, 0.19),
    (1.38, 0.12, 0.22),
    (1.48, 0.08, 0.14)
  ]
  .map(|(y, deep, wide)| (y, Vec4::new(deep, wide * bulk, 0.0, 0.0)));
  let heights: Vec<f32> = (0..=10).map(|band| WAIST + band as f32 * STRIPE).collect();
  Cage::tube(
    &along(&profile, &heights)
      .into_iter()
      .map(|(y, shape)| (Vec3::new(0.0, y, 0.0), shape.xy()))
      .collect::<Vec<_>>(),
    8
  )
}

fn ringed(profile: &[(f32, Vec4)], heights: &[f32], side: f32, sides: u32) -> Cage {
  Cage::tube(
    &along(profile, heights)
      .into_iter()
      .map(|(y, shape)| (Vec3::new(side * shape.x, y, shape.y), Vec2::splat(shape.z)))
      .collect::<Vec<_>>(),
    sides
  )
}

fn arm(rig: &Rig, side: f32) -> Vec<(Cage, [f32; 3])> {
  let sleeve = ringed(
    &[
      (1.45, Vec4::new(0.27, 0.0, 0.07, 0.0)),
      (1.15, Vec4::new(0.29, 0.01, 0.058, 0.0)),
      (0.88, Vec4::new(0.3, -0.02, 0.05, 0.0))
    ],
    &[1.45, 1.25, 1.15, 1.02, 0.95, 0.88],
    side,
    6
  );
  let hand = box_at(Vec3::new(side * 0.3, 0.82, -0.03), Vec3::new(0.075, 0.1, 0.09), 0.5);
  let hooked = rig.pirate && side < 0.0 && rig.chance(5, 4);
  let end = match hooked {
    true => (
      Cage::tube(
        &[
          (Vec3::new(side * 0.3, 0.86, -0.02), Vec2::splat(0.018)),
          (Vec3::new(side * 0.3, 0.76, -0.03), Vec2::splat(0.016)),
          (Vec3::new(side * 0.3, 0.71, -0.09), Vec2::splat(0.013)),
          (Vec3::new(side * 0.3, 0.76, -0.13), Vec2::splat(0.004))
        ],
        4
      ),
      STEEL
    ),
    false => (hand, rig.skin)
  };
  let mantle = match rig.pirate {
    true => vec![],
    false => vec![(
      Cage::rounded(
        Vec3::new(side * 0.22, 1.47, 0.02),
        Vec3::new(0.22, 0.12, 0.3),
        UVec3::ONE,
        0.6
      ),
      FUR
    )]
  };
  [vec![(sleeve, rig.shirt), end], mantle].concat()
}

fn leg(side: f32) -> (Cage, Cage) {
  (
    ringed(
      &[
        (0.8, Vec4::new(0.1, 0.0, 0.085, 0.0)),
        (0.45, Vec4::new(0.11, 0.0, 0.07, 0.0)),
        (0.1, Vec4::new(0.11, 0.0, 0.06, 0.0))
      ],
      &[0.8, 0.45, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15, 0.1],
      side,
      6
    ),
    box_at(Vec3::new(side * 0.11, 0.055, -0.035), Vec3::new(0.12, 0.11, 0.23), 0.4)
  )
}

fn cutlass() -> Vec<(Cage, [f32; 3])> {
  let at = |x: f32, y: f32, z: f32| Vec3::new(0.3 + x, y, z);
  vec![
    (
      Cage::tube(
        &[
          (at(0.0, 0.82, 0.05), Vec2::splat(0.018)),
          (at(0.0, 0.82, -0.08), Vec2::splat(0.018))
        ],
        4
      ),
      WOOD
    ),
    (
      Cage::tube(
        &[(-0.07, 0.8), (0.0, 0.82), (0.07, 0.8)]
          .map(|(x, y)| (at(x, y, -0.1), Vec2::new(0.014, 0.018))),
        4
      ),
      GOLD
    ),
    (
      Cage::tube(
        &[(-0.1, 0.02, 1.0), (-0.35, 0.03, 1.1), (-0.58, 0.07, 1.25), (-0.7, 0.14, 0.2)]
          .map(|(z, y, girth)| (at(0.0, 0.82 + y, z), Vec2::new(0.008, 0.03 * girth))),
        4
      ),
      STEEL
    ),
  ]
}

fn axe() -> Vec<(Cage, [f32; 3])> {
  let at = |y: f32, z: f32| Vec3::new(0.3, y, z);
  let head = at(0.86, -0.5);
  vec![
    (
      Cage::tube(
        &[(at(0.82, 0.14), Vec2::splat(0.02)), (at(0.82, -0.56), Vec2::splat(0.02))],
        4
      ),
      WOOD
    ),
    (
      Cage::rounded(head, Vec3::new(0.035, 0.17, 0.13), UVec3::new(1, 2, 1), 0.2).shaped(
        move |p| {
          let up = (p.y - head.y) / 0.085;
          Vec3::new(p.x, head.y + up * 0.085 * (1.0 + (head.z - p.z - 0.065) * 4.0), p.z)
        }
      ),
      STEEL
    ),
  ]
}

fn shield(rig: &Rig) -> Vec<(Cage, [f32; 3])> {
  let middle = Vec3::new(-0.42, 1.05, -0.1);
  let facing = Quat::from_rotation_y(-0.7);
  let along = |x: f32| middle + facing * Vec3::X * x;
  let paint = [[0.7, 0.14, 0.1], [0.85, 0.7, 0.2], [0.18, 0.3, 0.6], [0.2, 0.42, 0.25]]
    [(rig.roll % 4) as usize];
  let stripe = |turn: f32| {
    (
      box_at(along(-0.028), Vec3::new(0.01, 0.56, 0.06), 0.0).shaped(move |p| {
        (facing * Quat::from_rotation_x(turn)).mul_vec3(p - along(-0.028)) + along(-0.028)
      }),
      LINEN
    )
  };
  vec![
    (
      Cage::tube(
        &[(along(0.02), Vec2::splat(0.3)), (along(-0.02), Vec2::splat(0.3))],
        10
      ),
      IRON
    ),
    (
      Cage::tube(
        &[(along(-0.019), Vec2::splat(0.275)), (along(-0.024), Vec2::splat(0.275))],
        10
      ),
      paint
    ),
    stripe(PI / 4.0),
    stripe(-PI / 4.0),
    (box_at(along(-0.05), Vec3::splat(0.09), 0.7), IRON),
  ]
}

pub fn seafarer(band: Band, variant: u32) -> Mesh {
  let rig = Rig::of(band, variant);
  let part = |joint: u16| {
    move |(cage, color): (Cage, [f32; 3])| sculpt(&cage, |_, _| color, &[], |_| joint)
  };
  let striped = rig.striped();
  let band = |centre: Vec3| ((centre.y - WAIST) / STRIPE) as i32;
  let body = sculpt(
    &torso(&rig),
    |centre, _| match () {
      _ if band(centre) == 0 => {
        if rig.pirate {
          rig.trim
        } else {
          LEATHER
        }
      }
      _ if striped && band(centre) % 2 == 0 && band(centre) < 9 => rig.trim,
      _ if !rig.pirate && band(centre) == 1 => rig.trim,
      _ => rig.shirt
    },
    &[],
    |_| 0
  );
  let skirt = match rig.pirate {
    true => vec![],
    false => vec![sculpt(
      &Cage::tube(
        &[(0.82, 0.12, 0.19), (0.62, 0.15, 0.23)]
          .map(|(y, deep, wide)| (Vec3::new(0.0, y, 0.0), Vec2::new(deep, wide))),
        8
      ),
      |centre, _| if centre.y < 0.66 { rig.trim } else { rig.shirt },
      &[],
      |_| 0
    )]
  };
  let buckle =
    part(0)((box_at(Vec3::new(0.0, 0.8, -0.13), Vec3::new(0.06, 0.05, 0.02), 0.2), GOLD));
  let neck =
    part(0)((tube(&[(0.0, 1.44, 0.0, 0.06), (0.0, 1.58, 0.0, 0.055)], 1.0, 6), rig.skin));
  let legs = [(1.0, 4), (-1.0, 5)].into_iter().flat_map(|(side, joint)| {
    let (shin, foot) = leg(side);
    let boot = if rig.pirate { SOOT } else { FUR };
    [
      sculpt(
        &shin,
        |centre, _| match () {
          _ if rig.pirate && centre.y < 0.4 => {
            if centre.y > 0.35 {
              LEATHER
            } else {
              boot
            }
          }
          _ if !rig.pirate && centre.y < 0.4 => {
            if ((centre.y / WRAP) as i32) % 2 == 0 {
              LINEN
            } else {
              rig.legs
            }
          }
          _ => rig.legs
        },
        &[],
        |_| joint
      ),
      part(joint)((foot, boot))
    ]
  });
  let arms = [(1.0, 2), (-1.0, 3)].into_iter().flat_map(|(side, joint)| {
    arm(&rig, side).into_iter().map(move |(cage, color)| {
      let cuffed = |centre: Vec3| match rig.pirate {
        true => centre.y < 0.95,
        false => centre.y < 1.25
      };
      sculpt(
        &cage,
        |centre, _| match color == rig.shirt && cuffed(centre) {
          true if rig.pirate => LINEN,
          true if centre.y < 1.02 => LEATHER,
          true => rig.skin,
          false => color
        },
        &[],
        |_| joint
      )
    })
  });
  let held = match rig.pirate {
    true => cutlass(),
    false => axe()
  };
  let carried = match rig.pirate {
    true => vec![],
    false => shield(&rig)
  };
  merged(
    [body, buckle, neck]
      .into_iter()
      .chain(skirt)
      .chain(head(&rig).into_iter().map(part(1)))
      .chain(legs)
      .chain(arms)
      .chain(held.into_iter().map(part(2)))
      .chain(carried.into_iter().map(part(3)))
  )
}

#[cfg(test)]
mod tests {
  use {super::*, crate::sculpt::sheet};

  #[test]
  #[ignore]
  fn crews() {
    let meshes =
      [(Band::Pirate, 0), (Band::Pirate, 1), (Band::Viking, 2), (Band::Viking, 3)]
        .map(|(band, variant)| seafarer(band, variant));
    for mesh in meshes.iter() {
      println!("{} triangles", mesh.count_vertices() / 3)
    }
    sheet(&meshes, "screenshots/crew.ppm")
  }
}
