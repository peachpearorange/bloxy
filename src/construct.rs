use {crate::{cage::Cage,
             sculpt::{merged, sculpt}},
     bevy::prelude::*,
     std::f32::consts::TAU};

pub const CUSTODIAN_SCALE: f32 = 1.45;
pub const GLARE: [f32; 3] = [0.35, 1.0, 1.0];
const PLATE: [f32; 3] = [0.2, 0.27, 0.3];
const TRIM: [f32; 3] = [0.42, 0.52, 0.54];
const DARK: [f32; 3] = [0.08, 0.1, 0.12];
const VEIN: [f32; 3] = [0.3, 0.85, 0.9];
const CORE: Vec3 = Vec3::new(0.0, 1.05, 0.0);
const VANE: Vec3 = Vec3::new(0.36, 1.08, 0.04);
const TENDRIL: Vec3 = Vec3::new(0.1, 0.78, 0.0);
const HEAD: Vec3 = Vec3::new(0.0, 1.64, 0.0);
const SHOULDER: Vec3 = Vec3::new(0.42, 1.42, 0.0);
const HIP: Vec3 = Vec3::new(0.17, 0.86, 0.0);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
  Sentinel,
  Custodian
}

pub fn pivot(kind: Kind, joint: u16) -> Vec3 {
  let side = if joint % 2 == 0 { 1.0 } else { -1.0 };
  let mirrored = |at: Vec3| at * Vec3::new(side, 1.0, 1.0);
  match (kind, joint) {
    (Kind::Sentinel, 1) => CORE,
    (Kind::Sentinel, 2 | 3) => mirrored(VANE),
    (Kind::Sentinel, 4 | 5) => mirrored(TENDRIL),
    (Kind::Custodian, 1) => HEAD,
    (Kind::Custodian, 2 | 3) => mirrored(SHOULDER),
    (Kind::Custodian, 4 | 5) => mirrored(HIP),
    _ => Vec3::ZERO
  }
}

pub fn bindings(kind: Kind) -> Vec<Mat4> {
  (0..6).map(|joint| Mat4::from_translation(-pivot(kind, joint))).collect()
}

pub fn glints(kind: Kind) -> Vec<(Vec3, Vec3)> {
  match kind {
    Kind::Sentinel => vec![
      (Vec3::new(0.0, 0.0, -0.33), Vec3::new(0.14, 0.14, 0.02)),
      (Vec3::new(0.0, 0.0, -0.345), Vec3::new(0.06, 0.06, 0.02)),
    ],
    Kind::Custodian => vec![
      (Vec3::new(0.0, 0.02, -0.16), Vec3::new(0.2, 0.035, 0.02)),
      (Vec3::new(0.0, -0.32, -0.27), Vec3::new(0.14, 0.14, 0.02)),
    ]
  }
}

fn orb() -> Cage {
  Cage::rounded(CORE, Vec3::splat(0.62), UVec3::splat(2), 0.85).subdivided(1)
}

fn banded(centre: Vec3, normal: Vec3) -> [f32; 3] {
  let local = centre - CORE;
  match () {
    () if local.y.abs() < 0.035 => VEIN,
    () if normal.z < -0.8 && local.y.abs() < 0.16 && local.x.abs() < 0.16 => DARK,
    () if (local.x * 12.0).sin().abs() < 0.12 => TRIM,
    () => PLATE
  }
}

fn vane(side: f32) -> Cage {
  let at = |x: f32, y: f32, z: f32| Vec3::new(side * x, y, z);
  Cage::tube(
    &[
      (at(0.3, 1.08, 0.04), Vec2::new(0.05, 0.12)),
      (at(0.48, 1.2, 0.08), Vec2::new(0.035, 0.16)),
      (at(0.62, 1.36, 0.12), Vec2::new(0.025, 0.13)),
      (at(0.7, 1.55, 0.16), Vec2::new(0.012, 0.05))
    ],
    4
  )
}

fn tendril(side: f32, sway: f32) -> Cage {
  let at = |x: f32, y: f32, z: f32| Vec3::new(side * x, y, z + sway);
  Cage::tube(
    &[
      (at(0.1, 0.8, 0.0), Vec2::splat(0.045)),
      (at(0.13, 0.6, 0.04), Vec2::splat(0.035)),
      (at(0.12, 0.4, 0.1), Vec2::splat(0.025)),
      (at(0.08, 0.22, 0.12), Vec2::splat(0.012))
    ],
    4
  )
}

fn sentinel() -> Mesh {
  let plain =
    |cage: &Cage, color: [f32; 3], bone: u16| sculpt(cage, |_, _| color, &[], |_| bone);
  let crest = Cage::spike(
    CORE + Vec3::Y * 0.28,
    CORE + Vec3::new(0.0, 0.5, 0.12),
    Vec2::splat(0.06)
  );
  let ring = Cage::tube(
    &(0..=12)
      .map(|step| {
        let angle = step as f32 / 12.0 * TAU;
        (
          CORE + Vec3::new(angle.cos() * 0.42, -0.02, angle.sin() * 0.42),
          Vec2::new(0.025, 0.05)
        )
      })
      .collect::<Vec<_>>(),
    4
  );
  let keel = Cage::spike(CORE - Vec3::Y * 0.25, CORE - Vec3::Y * 0.5, Vec2::splat(0.09));
  merged([
    sculpt(&orb(), banded, &[], |_| 1),
    plain(&crest, TRIM, 1),
    plain(&ring, VEIN, 0),
    plain(&keel, PLATE, 0),
    plain(&vane(1.0), TRIM, 2),
    plain(&vane(-1.0), TRIM, 3),
    plain(&tendril(1.0, 0.0), DARK, 4),
    plain(&tendril(-1.0, 0.0), DARK, 5),
    plain(&tendril(0.0, -0.06), VEIN, 0)
  ])
}

fn torso() -> Cage {
  Cage::tube(
    &[
      (Vec3::new(0.0, 0.84, 0.0), Vec2::new(0.2, 0.16)),
      (Vec3::new(0.0, 1.0, 0.0), Vec2::new(0.24, 0.17)),
      (Vec3::new(0.0, 1.2, 0.0), Vec2::new(0.34, 0.22)),
      (Vec3::new(0.0, 1.42, 0.0), Vec2::new(0.4, 0.24)),
      (Vec3::new(0.0, 1.54, 0.0), Vec2::new(0.22, 0.16))
    ],
    6
  )
}

fn helm() -> Cage {
  Cage::rounded(HEAD + Vec3::Y * 0.1, Vec3::new(0.26, 0.24, 0.28), UVec3::ONE, 0.25)
    .shaped(|at| {
      let rise = ((at.y - HEAD.y) / 0.24).clamp(0.0, 1.0);
      Vec3::new(at.x * (1.0 - rise * 0.25), at.y, at.z - (1.0 - rise) * 0.05)
    })
}

fn pauldron(side: f32) -> Cage {
  Cage::rounded(
    SHOULDER * Vec3::new(side, 1.0, 1.0) + Vec3::new(side * 0.06, 0.06, 0.0),
    Vec3::new(0.3, 0.2, 0.32),
    UVec3::ONE,
    0.5
  )
}

fn limb(from: Vec3, to: Vec3, thick: f32) -> Cage {
  Cage::tube(
    &[(0.0, 1.1), (0.45, 0.8), (0.55, 0.95), (1.0, 0.75)]
      .map(|(t, girth)| (from.lerp(to, t), Vec2::splat(thick * girth))),
    4
  )
}

fn fist(at: Vec3) -> Cage { Cage::rounded(at, Vec3::splat(0.2), UVec3::ONE, 0.2) }

fn custodian() -> Mesh {
  let plain =
    |cage: &Cage, color: [f32; 3], bone: u16| sculpt(cage, |_, _| color, &[], |_| bone);
  let chest = |centre: Vec3, normal: Vec3| match () {
    () if normal.z < -0.6 && (centre.y - 1.3).abs() < 0.1 && centre.x.abs() < 0.12 => {
      DARK
    }
    () if (centre.y - 1.0).abs() < 0.04 => VEIN,
    () if centre.y < 0.95 => DARK,
    () => PLATE
  };
  let arms = [1.0f32, -1.0].map(|side| {
    let at = |x: f32, y: f32| Vec3::new(side * x, y, 0.0);
    (limb(at(0.46, 1.4), at(0.52, 0.86), 0.12), fist(at(0.53, 0.76)))
  });
  let legs = [1.0f32, -1.0].map(|side| {
    let at = |x: f32, y: f32, z: f32| Vec3::new(side * x, y, z);
    (
      limb(at(0.17, 0.88, 0.0), at(0.19, 0.12, 0.0), 0.13),
      Cage::rounded(at(0.19, 0.06, -0.05), Vec3::new(0.2, 0.12, 0.32), UVec3::ONE, 0.2)
    )
  });
  merged(
    [
      sculpt(&torso(), chest, &[], |_| 0),
      plain(&helm(), PLATE, 1),
      plain(&pauldron(1.0), TRIM, 2),
      plain(&pauldron(-1.0), TRIM, 3)
    ]
    .into_iter()
    .chain(
      arms.iter().zip([2, 3]).flat_map(|((arm, hand), bone)| {
        [plain(arm, PLATE, bone), plain(hand, DARK, bone)]
      })
    )
    .chain(
      legs.iter().zip([4, 5]).flat_map(|((leg, foot), bone)| {
        [plain(leg, PLATE, bone), plain(foot, DARK, bone)]
      })
    )
  )
}

pub fn mesh(kind: Kind) -> Mesh {
  match kind {
    Kind::Sentinel => sentinel(),
    Kind::Custodian => custodian()
  }
}

#[cfg(test)]
mod tests {
  use {super::*, crate::sculpt::sheet};

  #[test]
  #[ignore]
  fn constructs() {
    sheet(&[mesh(Kind::Sentinel), mesh(Kind::Custodian)], "screenshots/constructs.ppm")
  }
}
