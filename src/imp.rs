use {crate::{cage::Cage,
             sculpt::{Daub, merged, sculpt}},
     bevy::prelude::*};

pub const SCALE: f32 = 0.62;
pub const HEAD: Vec3 = Vec3::new(0.0, 1.46, 0.0);
pub const SOCKET: Vec3 = Vec3::new(0.085, 1.68, -0.205);
pub const GLARE: [f32; 3] = [1.0, 0.75, 0.12];
const SHOULDER: Vec3 = Vec3::new(0.3, 1.32, 0.0);
const HIP: Vec3 = Vec3::new(0.12, 0.8, 0.0);
const WING: Vec3 = Vec3::new(0.1, 1.33, 0.17);
const HIDE: [f32; 3] = [0.62, 0.13, 0.08];
const BELLY: [f32; 3] = [0.8, 0.38, 0.18];
const SCORCHED: [f32; 3] = [0.3, 0.06, 0.05];
const HORN: [f32; 3] = [0.17, 0.12, 0.1];
const MEMBRANE: [f32; 3] = [0.38, 0.07, 0.07];
const CLAW: [f32; 3] = [0.09, 0.07, 0.06];
const FANG: [f32; 3] = [0.95, 0.9, 0.74];
const MAW: [f32; 3] = [0.12, 0.02, 0.02];

pub fn pivot(joint: u16) -> Vec3 {
  let side = |joint: u16| if joint % 2 == 0 { 1.0 } else { -1.0 };
  match joint {
    1 => HEAD,
    2 | 3 => SHOULDER * Vec3::new(side(joint), 1.0, 1.0),
    4 | 5 => HIP * Vec3::new(side(joint), 1.0, 1.0),
    6 | 7 => WING * Vec3::new(side(joint), 1.0, 1.0),
    _ => Vec3::ZERO
  }
}

pub fn bindings() -> Vec<Mat4> {
  (0..8).map(|joint| Mat4::from_translation(-pivot(joint))).collect()
}

fn body() -> Cage {
  let trunk = Cage::rounded(
    Vec3::new(0.0, 1.08, 0.02),
    Vec3::new(0.46, 0.56, 0.4),
    UVec3::new(2, 3, 2),
    0.6
  )
  .shaped(|at| {
    let low = ((1.1 - at.y) / 0.25).clamp(0.0, 1.0);
    let high = ((at.y - 1.15) / 0.2).clamp(0.0, 1.0);
    let paunch = low * (1.0 - low) * 4.0 * (-at.z).max(0.0) * 0.6;
    Vec3::new(at.x * (1.0 + high * 0.15), at.y, at.z * (1.0 - high * 0.2) - paunch)
      + Vec3::Z * high * 0.05
  });
  let top = trunk.faces_where(|_, normal| normal.y > 0.6);
  let headed = trunk.grown_along(&top, &[
    (Vec3::new(0.0, 0.1, -0.1), Vec3::new(0.5, 1.0, 0.5)),
    (Vec3::new(0.0, 0.05, -0.02), Vec3::new(2.2, 1.0, 2.1)),
    (Vec3::new(0.0, 0.14, 0.0), Vec3::new(1.12, 1.0, 1.1)),
    (Vec3::new(0.0, 0.14, 0.02), Vec3::new(0.5, 1.0, 0.55))
  ]);
  let shoulders = |side: f32, cage: &Cage| {
    cage.faces_where(|centre, normal| {
      normal.x * side > 0.6 && centre.y > 1.2 && centre.y < 1.4
    })
  };
  let stubbed = [1.0, -1.0].into_iter().fold(headed, |cage, side| {
    let faces = shoulders(side, &cage);
    cage.grown(&faces, Vec3::new(side * 0.08, -0.01, 0.0), Vec3::new(1.0, 0.75, 0.8))
  });
  let per_side = |cage: &Cage, pick: &dyn Fn(Vec3, Vec3) -> bool| {
    [1.0, -1.0].map(|side| {
      cage.faces_where(|centre, normal| centre.x * side > 0.0 && pick(centre, normal))
    })
  };
  let limbed = |cage: Cage, faces: [Vec<usize>; 2], steps: &[(Vec3, Vec3)]| {
    faces.into_iter().zip([1.0, -1.0]).fold(cage, |cage, (faces, side)| {
      cage.grown_along(
        &faces,
        &steps
          .iter()
          .map(|&(toward, scale)| (toward * Vec3::new(side, 1.0, 1.0), scale))
          .collect::<Vec<_>>()
      )
    })
  };
  let armpits = per_side(&stubbed, &|centre, normal| {
    normal.y < -0.6 && centre.x.abs() > 0.2 && centre.y > 1.1
  });
  let armed = limbed(stubbed, armpits, &[
    (Vec3::new(0.05, -0.12, 0.0), Vec3::new(0.7, 1.0, 0.8)),
    (Vec3::new(0.04, -0.2, -0.04), Vec3::new(0.75, 1.0, 0.75)),
    (Vec3::new(0.0, -0.07, -0.03), Vec3::new(1.6, 1.0, 1.6))
  ]);
  let seat = per_side(&armed, &|centre, normal| {
    normal.y < -0.6 && centre.y < 0.95 && centre.x.abs() > 0.03 && centre.x.abs() < 0.2
  });
  let legged = limbed(armed, seat, &[
    (Vec3::new(0.04, -0.24, -0.17), Vec3::new(0.75, 1.0, 0.75)),
    (Vec3::new(0.0, -0.22, 0.21), Vec3::new(0.6, 1.0, 0.6)),
    (Vec3::new(0.0, -0.2, -0.02), Vec3::new(0.85, 1.0, 0.85)),
    (Vec3::new(0.0, -0.05, -0.07), Vec3::new(1.4, 1.0, 2.2))
  ]);
  let rump = legged.faces_where(|centre, normal| {
    normal.z > 0.6 && centre.y < 1.0 && centre.y > 0.82 && centre.x.abs() < 0.2
  });
  legged.grown_along(&rump, &[
    (Vec3::new(0.0, -0.04, 0.1), Vec3::new(0.55, 0.55, 1.0)),
    (Vec3::new(0.0, -0.16, 0.26), Vec3::new(0.6, 0.6, 1.0)),
    (Vec3::new(0.0, 0.2, 0.19), Vec3::new(0.4, 0.4, 1.0))
  ])
}

fn joint(at: Vec3) -> u16 {
  let side = |right: u16, left: u16| if at.x > 0.0 { right } else { left };
  match () {
    _ if at.y > HEAD.y => 1,
    _ if at.x.abs() > 0.27 && at.y > 0.6 => side(2, 3),
    _ if at.y < HIP.y - 0.06 && at.z < 0.2 => side(4, 5),
    _ => 0
  }
}

fn hide(centre: Vec3, normal: Vec3) -> [f32; 3] {
  match () {
    _ if centre.y < 0.12 => SCORCHED,
    _ if centre.x.abs() > 0.27 && centre.y < 1.02 && centre.y > 0.6 => SCORCHED,
    _ if normal.z < -0.3
      && (centre.x / 0.16).powi(2) + ((centre.y - 1.04) / 0.22).powi(2) < 1.0 =>
    {
      BELLY
    }
    _ => HIDE
  }
}

fn horns() -> Vec<Cage> {
  [1.0, -1.0]
    .map(|side| {
      Cage::tube(
        &[
          (Vec3::new(side * 0.11, 1.84, -0.12), Vec2::splat(0.045)),
          (Vec3::new(side * 0.16, 1.95, -0.08), Vec2::splat(0.036)),
          (Vec3::new(side * 0.19, 2.04, 0.02), Vec2::splat(0.024)),
          (Vec3::new(side * 0.17, 2.08, 0.13), Vec2::splat(0.004))
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
          (Vec3::new(side * 0.25, 1.73, -0.02), Vec2::new(0.02, 0.06)),
          (Vec3::new(side * 0.36, 1.8, 0.02), Vec2::new(0.012, 0.035)),
          (Vec3::new(side * 0.45, 1.88, 0.05), Vec2::new(0.006, 0.004))
        ],
        4
      )
    })
    .to_vec()
}

fn claws(body: &Cage) -> Vec<(Cage, u16)> {
  let tip = |pick: &dyn Fn(Vec3) -> bool, along: &dyn Fn(Vec3) -> f32| {
    body
      .points
      .iter()
      .copied()
      .filter(|&at| pick(at))
      .min_by(|a, b| along(*a).total_cmp(&along(*b)))
      .unwrap_or_default()
  };
  [1.0, -1.0]
    .into_iter()
    .flat_map(|side| {
      let palm = tip(&|at| at.x * side > 0.27 && at.y > 0.5, &|at| at.y);
      let toe = tip(&|at| at.x * side > 0.0 && at.y < 0.3, &|at| at.z - at.y);
      let hand = [-0.035, 0.0, 0.035].map(|spread| {
        let root = palm + Vec3::new(0.0, 0.03, spread - 0.02);
        (
          Cage::spike(root, root + Vec3::new(0.0, -0.1, -0.06), Vec2::splat(0.016)),
          if side > 0.0 { 2 } else { 3 }
        )
      });
      let foot = [-0.03, 0.0, 0.03].map(|spread| {
        let root = toe + Vec3::new(side * spread, 0.01, 0.03);
        (
          Cage::spike(
            root,
            root + Vec3::new(side * spread, -0.02, -0.07),
            Vec2::splat(0.015)
          ),
          if side > 0.0 { 4 } else { 5 }
        )
      });
      hand.into_iter().chain(foot)
    })
    .collect()
}

fn wing(side: f32) -> Vec<(Cage, [f32; 3])> {
  let at = |x: f32, y: f32, z: f32| Vec3::new(side * x, y, z);
  let (root, elbow) = (at(0.12, 1.33, 0.17), at(0.42, 1.55, 0.3));
  let tips = [at(0.82, 1.6, 0.36), at(0.78, 1.25, 0.42), at(0.55, 1.0, 0.38)];
  let low = at(0.16, 1.1, 0.2);
  let bones = [Cage::tube(&[(root, Vec2::splat(0.025)), (elbow, Vec2::splat(0.02))], 3)]
    .into_iter()
    .chain(tips.map(|tip| Cage::spike(elbow, tip, Vec2::splat(0.016))))
    .map(|cage| (cage, SCORCHED));
  let rim: Vec<Vec3> = [root, elbow, tips[0]]
    .into_iter()
    .chain([
      tips[0].lerp(tips[1], 0.5) + (elbow - tips[0].lerp(tips[1], 0.5)) * 0.25,
      tips[1],
      tips[1].lerp(tips[2], 0.5) + (elbow - tips[1].lerp(tips[2], 0.5)) * 0.25,
      tips[2],
      low
    ])
    .map(|point| point + Vec3::Z * 0.004)
    .collect();
  let membrane = Cage {
    points: rim,
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
  let socket = |side: f32| Daub { at: SOCKET * Vec3::new(side, 1.0, 1.0), color: MAW };
  let grin = [-0.08, -0.04, 0.0, 0.04, 0.08]
    .map(|x| Daub { at: Vec3::new(x, 1.6 + x.abs() * 0.3, -0.23), color: MAW });
  let body = body();
  let skin = sculpt(
    &body.clone().subdivided(1),
    hide,
    &[socket(1.0), socket(-1.0)].into_iter().chain(grin).collect::<Vec<_>>(),
    joint
  );
  let plain =
    &|cage: &Cage, color: [f32; 3], bone: u16| sculpt(cage, |_, _| color, &[], |_| bone);
  let nose = Cage::spike(
    Vec3::new(0.0, 1.655, -0.215),
    Vec3::new(0.0, 1.62, -0.3),
    Vec2::new(0.035, 0.03)
  );
  let brow = Cage::tube(
    &[(-0.15, 1.735, -0.165), (0.0, 1.715, -0.21), (0.15, 1.735, -0.165)]
      .map(|(x, y, z)| (Vec3::new(x, y, z), Vec2::new(0.025, 0.02))),
    4
  );
  let fangs = [1.0, -1.0].map(|side| {
    Cage::spike(
      Vec3::new(side * 0.05, 1.6, -0.222),
      Vec3::new(side * 0.05, 1.55, -0.22),
      Vec2::splat(0.013)
    )
  });
  let end =
    body.points.iter().copied().max_by(|a, b| a.z.total_cmp(&b.z)).unwrap_or_default();
  let spade = Cage::tube(
    &[
      (end - Vec3::new(0.0, 0.01, 0.02), Vec2::new(0.012, 0.012)),
      (end + Vec3::new(0.0, 0.04, 0.05), Vec2::new(0.012, 0.065)),
      (end + Vec3::new(0.0, 0.1, 0.12), Vec2::new(0.003, 0.003))
    ],
    4
  );
  merged(
    [skin, plain(&nose, HIDE, 1), plain(&brow, SCORCHED, 1), plain(&spade, SCORCHED, 0)]
      .into_iter()
      .chain(horns().iter().map(|horn| plain(horn, HORN, 1)))
      .chain(ears().iter().map(|ear| plain(ear, HIDE, 1)))
      .chain(fangs.iter().map(|fang| plain(fang, FANG, 1)))
      .chain(claws(&body).iter().map(|(claw, bone)| plain(claw, CLAW, *bone)))
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
  fn imps() { sheet(&[fiendish()], "screenshots/imp.ppm") }
}
