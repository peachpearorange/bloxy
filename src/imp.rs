use {crate::{cage::Cage,
             sculpt::{merged, sculpt}},
     bevy::prelude::*};

pub const SCALE: f32 = 0.62;
pub const HEAD: Vec3 = Vec3::new(0.0, 1.44, 0.04);
pub const GLARE: [f32; 3] = [1.0, 0.8, 0.15];
const SHOULDER: Vec3 = Vec3::new(0.24, 1.31, 0.06);
const HIP: Vec3 = Vec3::new(0.09, 0.8, 0.02);
const WING: Vec3 = Vec3::new(0.08, 1.28, 0.14);
const EYE: Vec3 = Vec3::new(0.07, 1.665, 0.0);
const HIDE: [f32; 3] = [0.66, 0.12, 0.07];
const BELLY: [f32; 3] = [0.86, 0.42, 0.2];
const SCORCHED: [f32; 3] = [0.28, 0.05, 0.04];
const HORN: [f32; 3] = [0.15, 0.11, 0.09];
const MEMBRANE: [f32; 3] = [0.42, 0.06, 0.06];
const CLAW: [f32; 3] = [0.08, 0.06, 0.05];
const FANG: [f32; 3] = [0.96, 0.92, 0.78];
const MAW: [f32; 3] = [0.1, 0.02, 0.02];

pub fn pivot(joint: u16) -> Vec3 {
  let side = if joint % 2 == 0 { 1.0 } else { -1.0 };
  let mirrored = |at: Vec3| at * Vec3::new(side, 1.0, 1.0);
  match joint {
    1 => HEAD,
    2 | 3 => mirrored(SHOULDER),
    4 | 5 => mirrored(HIP),
    6 | 7 => mirrored(WING),
    _ => Vec3::ZERO
  }
}

pub fn bindings() -> Vec<Mat4> {
  (0..8).map(|joint| Mat4::from_translation(-pivot(joint))).collect()
}

fn skull() -> Cage {
  Cage::rounded(
    Vec3::new(0.0, 1.63, -0.03),
    Vec3::new(0.36, 0.32, 0.3),
    UVec3::splat(2),
    0.5
  )
  .shaped(|at| {
    let rise = ((at.y - 1.47) / 0.32).clamp(0.0, 1.0);
    let jaw = (1.0 - rise / 0.5).max(0.0);
    let width = match rise < 0.5 {
      true => 0.35 + rise * 1.3,
      false => 1.0 - (rise - 0.5) * 0.3
    };
    Vec3::new(at.x * width, at.y, at.z - jaw * 0.07 * (-at.z).max(0.0) / 0.15)
  })
  .subdivided(1)
}

fn face(skull: &Cage, x: f32, y: f32) -> Vec3 {
  let z = skull
    .points
    .iter()
    .filter(|at| Vec2::new(at.x - x, at.y - y).length() < 0.05)
    .map(|at| at.z)
    .fold(0.0, f32::min);
  Vec3::new(x, y, z)
}

pub fn glints() -> Vec<(Vec3, Vec3)> {
  let skull = skull();
  [1.0, -1.0]
    .into_iter()
    .flat_map(|side| {
      [(0.025, 0.01, 0.045), (-0.02, -0.006, 0.04)].map(|(out, up, wide)| {
        let at = face(&skull, side * (EYE.x + out), EYE.y + up) - Vec3::Z * 0.012;
        (at - HEAD, Vec3::new(wide, 0.024, 0.01))
      })
    })
    .collect()
}

fn torso() -> Cage {
  Cage::tube(
    &[
      (Vec3::new(0.0, 0.76, 0.03), Vec2::new(0.07, 0.09)),
      (Vec3::new(0.0, 0.88, 0.0), Vec2::new(0.11, 0.12)),
      (Vec3::new(0.0, 1.03, 0.0), Vec2::new(0.11, 0.13)),
      (Vec3::new(0.0, 1.18, 0.04), Vec2::new(0.1, 0.16)),
      (Vec3::new(0.0, 1.3, 0.07), Vec2::new(0.08, 0.19)),
      (Vec3::new(0.0, 1.38, 0.06), Vec2::new(0.05, 0.07)),
      (Vec3::new(0.0, 1.5, 0.0), Vec2::new(0.04, 0.05))
    ],
    8
  )
}

fn arm(side: f32) -> Cage {
  let at = |x: f32, y: f32, z: f32| Vec3::new(side * x, y, z);
  Cage::tube(
    &[
      (at(0.23, 1.33, 0.06), Vec2::splat(0.05)),
      (at(0.27, 1.2, 0.07), Vec2::splat(0.032)),
      (at(0.31, 1.08, 0.08), Vec2::splat(0.038)),
      (at(0.32, 0.97, 0.03), Vec2::splat(0.026)),
      (at(0.33, 0.87, -0.02), Vec2::splat(0.024)),
      (at(0.335, 0.82, -0.04), Vec2::new(0.045, 0.03)),
      (at(0.335, 0.79, -0.06), Vec2::new(0.035, 0.02))
    ],
    4
  )
}

fn leg(side: f32) -> Cage {
  let at = |x: f32, y: f32, z: f32| Vec3::new(side * x, y, z);
  Cage::tube(
    &[
      (at(0.09, 0.84, 0.02), Vec2::splat(0.065)),
      (at(0.11, 0.67, -0.06), Vec2::splat(0.055)),
      (at(0.12, 0.52, -0.1), Vec2::splat(0.04)),
      (at(0.12, 0.36, 0.0), Vec2::splat(0.028)),
      (at(0.12, 0.2, 0.08), Vec2::splat(0.03)),
      (at(0.12, 0.07, 0.04), Vec2::splat(0.03)),
      (at(0.12, 0.04, -0.07), Vec2::new(0.025, 0.045))
    ],
    5
  )
}

fn tail() -> Vec<Cage> {
  let path = [
    (Vec3::new(0.0, 0.84, 0.08), 0.03),
    (Vec3::new(0.0, 0.74, 0.24), 0.024),
    (Vec3::new(0.0, 0.78, 0.4), 0.018),
    (Vec3::new(0.0, 0.92, 0.5), 0.012),
    (Vec3::new(0.0, 1.03, 0.52), 0.008)
  ];
  let end = path[path.len() - 1].0;
  vec![
    Cage::tube(&path.map(|(at, thick)| (at, Vec2::splat(thick))), 4),
    Cage::tube(
      &[
        (end, Vec2::new(0.008, 0.008)),
        (end + Vec3::new(0.0, 0.05, 0.0), Vec2::new(0.012, 0.06)),
        (end + Vec3::new(0.0, 0.13, -0.02), Vec2::splat(0.003))
      ],
      4
    ),
  ]
}

fn horns() -> Vec<Cage> {
  [1.0, -1.0]
    .map(|side| {
      Cage::tube(
        &[
          (Vec3::new(side * 0.08, 1.76, -0.1), Vec2::splat(0.04)),
          (Vec3::new(side * 0.13, 1.87, -0.08), Vec2::splat(0.027)),
          (Vec3::new(side * 0.15, 1.96, -0.01), Vec2::splat(0.004))
        ],
        4
      )
    })
    .to_vec()
}

fn ears() -> Vec<Cage> {
  [1.0, -1.0]
    .map(|side| {
      Cage::tube(
        &[
          (Vec3::new(side * 0.15, 1.66, 0.0), Vec2::new(0.015, 0.06)),
          (Vec3::new(side * 0.27, 1.7, 0.04), Vec2::new(0.01, 0.04)),
          (Vec3::new(side * 0.38, 1.76, 0.08), Vec2::splat(0.004))
        ],
        4
      )
    })
    .to_vec()
}

fn claws(side: f32) -> [(Cage, u16); 6] {
  let at = |x: f32, y: f32, z: f32| Vec3::new(side * x, y, z);
  let (hand, foot) = if side > 0.0 { (2, 4) } else { (3, 5) };
  let finger = |spread: f32| {
    (
      Cage::spike(
        at(0.335 + spread * 0.5, 0.79, -0.06 + spread),
        at(0.34 + spread, 0.69, -0.12 + spread),
        Vec2::splat(0.014)
      ),
      hand
    )
  };
  let toe = |spread: f32| {
    (
      Cage::spike(
        at(0.12 + spread, 0.04, -0.1),
        at(0.12 + spread * 1.5, 0.02, -0.17),
        Vec2::splat(0.014)
      ),
      foot
    )
  };
  [finger(-0.03), finger(0.0), finger(0.03), toe(-0.03), toe(0.0), toe(0.03)]
}

fn wing(side: f32) -> Vec<(Cage, [f32; 3])> {
  let at = |x: f32, y: f32, z: f32| Vec3::new(side * x, y, z);
  let (root, elbow) = (at(0.08, 1.28, 0.14), at(0.36, 1.48, 0.26));
  let tips = [at(0.72, 1.52, 0.32), at(0.68, 1.22, 0.38), at(0.48, 1.0, 0.34)];
  let low = at(0.12, 1.08, 0.16);
  let notch = |a: Vec3, b: Vec3| a.lerp(b, 0.5) + (elbow - a.lerp(b, 0.5)) * 0.25;
  let bones = [Cage::tube(&[(root, Vec2::splat(0.022)), (elbow, Vec2::splat(0.018))], 3)]
    .into_iter()
    .chain(tips.map(|tip| Cage::spike(elbow, tip, Vec2::splat(0.014))))
    .map(|cage| (cage, SCORCHED));
  let membrane = Cage {
    points: vec![
      root,
      elbow,
      tips[0],
      notch(tips[0], tips[1]),
      tips[1],
      notch(tips[1], tips[2]),
      tips[2],
      low,
    ],
    faces: vec![
      vec![0, 1, 7],
      vec![1, 6, 7],
      vec![1, 5, 6],
      vec![1, 4, 5],
      vec![1, 3, 4],
      vec![1, 2, 3],
    ],
    ..default()
  }
  .solidified(0.008);
  bones.chain([(membrane, MEMBRANE)]).collect()
}

pub fn fiendish() -> Mesh {
  let plain =
    &|cage: &Cage, color: [f32; 3], bone: u16| sculpt(cage, |_, _| color, &[], |_| bone);
  let skull = skull();
  let grin = Cage::tube(
    &[-0.085, -0.045, 0.0, 0.045, 0.085].map(|x| {
      (face(&skull, x, 1.575 + x * x * 5.0) - Vec3::Z * 0.004, Vec2::new(0.012, 0.013))
    }),
    4
  );
  let fangs = [1.0, -1.0].map(|side| {
    let root = face(&skull, side * 0.04, 1.57) - Vec3::Z * 0.01;
    Cage::spike(root, root + Vec3::new(0.0, -0.05, 0.004), Vec2::splat(0.013))
  });
  let sockets = [1.0, -1.0].map(|side| {
    let middle = face(&skull, side * EYE.x, EYE.y) - Vec3::Z * 0.004;
    Cage::rounded(middle, Vec3::new(0.1, 0.045, 0.02), UVec3::ONE, 0.3)
      .shaped(move |at| Quat::from_rotation_z(side * 0.3).mul_vec3(at - middle) + middle)
  });
  let nose = {
    let root = face(&skull, 0.0, 1.62);
    Cage::spike(
      root + Vec3::Y * 0.02,
      root + Vec3::new(0.0, -0.02, -0.06),
      Vec2::splat(0.025)
    )
  };
  let trunk = sculpt(
    &torso(),
    |centre, normal| match normal.z < -0.45 && centre.y < 1.22 && centre.y > 0.8 {
      true => BELLY,
      false => HIDE
    },
    &[],
    |at| if at.y > 1.46 { 1 } else { 0 }
  );
  let limb = |cage: &Cage, bone: u16, dark_below: f32| {
    sculpt(
      cage,
      |centre, _| if centre.y < dark_below { SCORCHED } else { HIDE },
      &[],
      |_| bone
    )
  };
  merged(
    [
      trunk,
      plain(&skull, HIDE, 1),
      plain(&grin, MAW, 1),
      plain(&nose, HIDE, 1),
      limb(&arm(1.0), 2, 0.9),
      limb(&arm(-1.0), 3, 0.9),
      limb(&leg(1.0), 4, 0.12),
      limb(&leg(-1.0), 5, 0.12)
    ]
    .into_iter()
    .chain(sockets.iter().map(|socket| plain(socket, MAW, 1)))
    .chain(fangs.iter().map(|fang| plain(fang, FANG, 1)))
    .chain(horns().iter().map(|horn| plain(horn, HORN, 1)))
    .chain(ears().iter().map(|ear| plain(ear, HIDE, 1)))
    .chain(tail().iter().map(|piece| plain(piece, SCORCHED, 0)))
    .chain(
      [1.0, -1.0]
        .into_iter()
        .flat_map(claws)
        .map(|(claw, bone)| plain(&claw, CLAW, bone))
    )
    .chain([(1.0, 6), (-1.0, 7)].into_iter().flat_map(|(side, bone)| {
      wing(side).into_iter().map(move |(cage, color)| plain(&cage, color, bone))
    }))
  )
}

#[cfg(test)]
mod tests {
  use {super::*, crate::sculpt::sheet};

  #[test]
  #[ignore]
  fn imps() {
    let mesh = fiendish();
    println!("{} triangles", mesh.count_vertices() / 3);
    sheet(&[mesh], "screenshots/imp.ppm")
  }
}
