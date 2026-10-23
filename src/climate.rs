use {crate::{island::{SEA, landmass},
             noise::{fbm2, smoothstep}},
     bevy::prelude::*};

pub const LAPSE: f32 = 0.009;
pub const FROZEN: f32 = -0.38;
pub const ARID: f32 = 0.3;
const HOME_CALM: (f32, f32) = (150.0, 700.0);
const HOME_HEAT: f32 = 0.08;
const HOME_SEA: (f32, f32) = (260.0, 540.0);

fn calmed(at: Vec2, raw: f32, settled: f32) -> f32 {
  let wild = smoothstep(HOME_CALM.0, HOME_CALM.1, at.length());
  raw * wild + settled * (1.0 - wild)
}

pub fn heat(seed: u32, at: Vec2) -> f32 {
  let broad = fbm2(seed.wrapping_add(60), at.x / 1100.0, at.y / 1100.0, 3);
  let local = fbm2(seed.wrapping_add(61), at.x / 300.0, at.y / 300.0, 2);
  let poleward = (at.y / 3500.0).clamp(-0.5, 0.5);
  calmed(at, broad * 1.4 + local * 0.25 + poleward, HOME_HEAT)
}

pub fn wet(seed: u32, at: Vec2) -> f32 {
  let broad = fbm2(seed.wrapping_add(62), at.x / 650.0, at.y / 650.0, 3);
  let local = fbm2(seed.wrapping_add(63), at.x / 180.0, at.y / 180.0, 2);
  calmed(at, broad * 1.2 + local * 0.3, 0.1)
}

pub fn chilled(heat: f32, height: f32) -> f32 {
  heat - (height - SEA as f32).max(0.0) * LAPSE
}

pub fn mainland(seed: u32, at: Vec2) -> f32 {
  let wave = |salt: u32, scale: f32, octaves: u32| {
    fbm2(seed.wrapping_add(salt), at.x / scale, at.y / scale, octaves)
  };
  let open = smoothstep(HOME_SEA.0, HOME_SEA.1, at.length());
  let shore = wave(50, 90.0, 3) * 0.12 + wave(51, 26.0, 2) * 0.04;
  let coast = (landmass(seed, at) - 0.2 + shore) * open - (1.0 - open) * 0.3;
  let inland = smoothstep(0.0, 0.22, coast);
  let hills = wave(52, 55.0, 3) * 0.5 + 0.5;
  let uplift = smoothstep(0.05, 0.4, wave(53, 380.0, 2));
  let ridge = (1.0 - wave(54, 110.0, 3).abs() * 1.6).max(0.0);
  match coast < 0.0 {
    true => SEA as f32 - 0.5 + coast * 60.0,
    false => {
      SEA as f32 - 0.5
        + (coast * 30.0).min(2.5)
        + inland * (hills * 12.0 + uplift * (ridge * ridge * 75.0 + 12.0))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn home_is_mild_and_world_varies() {
    assert!((heat(1, Vec2::ZERO) - HOME_HEAT).abs() < 1e-4);
    assert!(mainland(1, Vec2::ZERO) < SEA as f32);
    let samples: Vec<f32> = (-20..20)
      .flat_map(|x| (-20..20).map(move |z| Vec2::new(x as f32, z as f32) * 120.0))
      .map(|at| heat(1, at))
      .collect();
    assert!(samples.iter().any(|&heat| heat < FROZEN));
    assert!(samples.iter().any(|&heat| heat > ARID));
  }
}
