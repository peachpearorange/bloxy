use {super::synth::{Hall, Osc, Rng, Svf, Wave, len, midi, perc, seamless, smooth, time},
     std::f32::consts::TAU};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cue {
  Soft,
  Hard,
  Wood,
  Crumble,
  Thud,
  Punch,
  Hurt,
  Splash,
  Swim,
  Song,
  Caw,
  Cry,
  Honk,
  Bleat,
  Cast,
  Burst,
  Pop,
  Creak,
  Chime,
  Warp,
  Thunder,
  Craft
}

impl Cue {
  pub const ALL: [Cue; 22] = [
    Cue::Soft,
    Cue::Hard,
    Cue::Wood,
    Cue::Crumble,
    Cue::Thud,
    Cue::Punch,
    Cue::Hurt,
    Cue::Splash,
    Cue::Swim,
    Cue::Song,
    Cue::Caw,
    Cue::Cry,
    Cue::Honk,
    Cue::Bleat,
    Cue::Cast,
    Cue::Burst,
    Cue::Pop,
    Cue::Creak,
    Cue::Chime,
    Cue::Warp,
    Cue::Thunder,
    Cue::Craft
  ];

  pub fn variants(self) -> u64 {
    match self {
      Cue::Soft | Cue::Hard | Cue::Wood => 4,
      Cue::Crumble | Cue::Punch | Cue::Song | Cue::Caw | Cue::Cry | Cue::Swim => 3,
      Cue::Splash | Cue::Honk | Cue::Bleat | Cue::Cast | Cue::Burst | Cue::Thunder => 2,
      _ => 1
    }
  }
}

fn shaped(secs: f32, mut sample: impl FnMut(f32) -> f32) -> Vec<f32> {
  (0..len(secs)).map(|index| sample(time(index))).collect()
}

fn add(into: &mut [f32], at: f32, source: &[f32], gain: f32) {
  let start = len(at);
  for (sample, &x) in into.iter_mut().skip(start).zip(source) {
    *sample += x * gain
  }
}

fn grain(
  rng: &mut Rng,
  secs: f32,
  (low, high): (f32, f32),
  q: f32,
  attack: f32,
  decay: f32
) -> Vec<f32> {
  let mut filter = Svf::new(rng.range(low, high), q);
  shaped(secs, |t| filter.step(rng.signed()).band * perc(t, attack, decay))
}

fn murmur(rng: &mut Rng, secs: f32, cutoff: f32, attack: f32, decay: f32) -> Vec<f32> {
  let mut filter = Svf::new(cutoff, 0.7);
  shaped(secs, |t| filter.step(rng.signed()).low * perc(t, attack, decay))
}

fn glide(secs: f32, from: f32, to: f32, attack: f32, decay: f32) -> Vec<f32> {
  let mut osc = Osc::default();
  shaped(secs, |t| {
    let along = (t / secs).min(1.0);
    osc.sine(from + (to - from) * along) * perc(t, attack, decay)
  })
}

fn softened(samples: Vec<f32>, cutoff: f32) -> Vec<f32> {
  let mut filter = Svf::new(cutoff, 0.6);
  samples.into_iter().map(|x| filter.step(x).low).collect()
}

fn step(rng: &mut Rng, cue: Cue) -> Vec<f32> {
  let mut out = vec![0.0; len(0.16)];
  match cue {
    Cue::Soft => {
      let cutoff = rng.range(650.0, 1100.0);
      add(&mut out, 0.0, &murmur(rng, 0.12, cutoff, 0.004, 0.03), 1.0);
      add(
        &mut out,
        rng.range(0.015, 0.03),
        &murmur(rng, 0.1, cutoff * 0.8, 0.003, 0.025),
        0.6
      )
    }
    Cue::Hard => {
      let pitch = rng.range(140.0, 190.0);
      add(&mut out, 0.0, &grain(rng, 0.08, (1500.0, 2300.0), 1.4, 0.001, 0.016), 1.0);
      add(&mut out, 0.0, &glide(0.1, pitch, pitch * 0.8, 0.001, 0.025), 0.35)
    }
    _ => {
      let pitch = rng.range(190.0, 250.0);
      add(&mut out, 0.0, &grain(rng, 0.12, (380.0, 520.0), 5.0, 0.001, 0.03), 1.0);
      add(&mut out, 0.0, &glide(0.12, pitch, pitch * 0.92, 0.001, 0.04), 0.4)
    }
  }
  softened(out, 3200.0)
}

fn crumble(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(0.45)];
  add(&mut out, 0.0, &glide(0.2, 110.0, 60.0, 0.002, 0.06), 0.25);
  for _ in 0..12 {
    let at = rng.range(0.0, 0.25).powf(1.3);
    let piece = grain(rng, 0.06, (500.0, 2400.0), 2.0, 0.001, 0.012);
    add(&mut out, at, &piece, rng.range(0.4, 1.0) * (1.0 - at * 2.5).max(0.2))
  }
  softened(out, 3500.0)
}

fn punch(rng: &mut Rng, low: f32) -> Vec<f32> {
  let mut out = vec![0.0; len(0.3)];
  let pitch = rng.range(120.0, 150.0) * low;
  add(&mut out, 0.0, &glide(0.25, pitch, pitch * 0.4, 0.002, 0.07), 1.0);
  add(&mut out, 0.0, &murmur(rng, 0.06, 1400.0, 0.001, 0.018), 0.5);
  softened(out, 2400.0)
}

fn hurt(rng: &mut Rng) -> Vec<f32> {
  let mut out = punch(rng, 0.8);
  let mut osc = Osc::default();
  let mut formant = Svf::new(650.0, 2.0);
  let oof =
    shaped(0.2, |t| formant.step(osc.saw(170.0 - t * 200.0)).band * perc(t, 0.015, 0.06));
  add(&mut out, 0.01, &oof, 0.9);
  softened(out, 1800.0)
}

fn splash(rng: &mut Rng) -> Vec<f32> {
  let secs = 0.85;
  let mut filter = Svf::new(2000.0, 0.9);
  let mut out = shaped(secs, |t| {
    filter.tune(2400.0 - 1900.0 * (t / 0.5).min(1.0), 0.9);
    filter.step(rng.signed()).band * perc(t, 0.012, 0.18)
  });
  for index in 0..7 {
    let at = 0.08 + index as f32 * rng.range(0.04, 0.08);
    let pitch = rng.range(500.0, 1100.0);
    add(&mut out, at, &glide(0.05, pitch, pitch * 1.6, 0.002, 0.018), 0.18)
  }
  softened(out, 4000.0)
}

fn swim(rng: &mut Rng) -> Vec<f32> {
  let mut filter = Svf::new(700.0, 0.8);
  let cutoff = rng.range(550.0, 900.0);
  shaped(0.4, |t| {
    filter.tune(cutoff + 400.0 * smooth(t / 0.2), 0.8);
    filter.step(rng.signed()).low
      * (smooth(t / 0.12) * (-(t - 0.12).max(0.0) / 0.1).exp())
  })
}

fn song(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(0.6)];
  let chirps = 2 + rng.below(3);
  let base = rng.range(2200.0, 3000.0);
  for index in 0..chirps {
    let at = index as f32 * rng.range(0.08, 0.12);
    let (from, to) = (base * rng.range(0.9, 1.1), base * rng.range(1.25, 1.5));
    add(&mut out, at, &glide(0.07, from, to, 0.006, 0.025), 1.0)
  }
  softened(out, 5000.0)
}

fn caw(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(0.7)];
  for index in 0..1 + rng.below(2) {
    let mut osc = Osc::default();
    let mut formant = Svf::new(1000.0, 2.5);
    let pitch = rng.range(380.0, 460.0);
    let one = shaped(0.22, |t| {
      formant.step(osc.saw(pitch - t * 300.0)).band * perc(t, 0.02, 0.07)
    });
    add(&mut out, index as f32 * 0.28, &one, 1.0)
  }
  softened(out, 1800.0)
}

fn cry(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(0.9)];
  for (index, gain) in [(0, 1.0), (1, 0.6)] {
    let mut osc = Osc::default();
    let start = rng.range(1150.0, 1400.0);
    let secs = 0.34 - index as f32 * 0.12;
    let one = shaped(secs, |t| {
      let along = t / secs;
      let pitch = start - 380.0 * along + (t * TAU * 9.0).sin() * 25.0;
      (osc.sine(pitch) * smooth(t / 0.04)) * (1.0 - along).max(0.0).powf(0.6)
    });
    add(&mut out, index as f32 * 0.4, &one, gain)
  }
  softened(out, 3000.0)
}

fn honk(rng: &mut Rng) -> Vec<f32> {
  let mut osc = Osc::default();
  let mut tone = Svf::new(700.0, 1.2);
  let pitch = rng.range(300.0, 360.0);
  shaped(0.3, |t| tone.step(osc.saw(pitch * (1.0 - t * 0.2))).low * perc(t, 0.03, 0.08))
}

fn bleat(rng: &mut Rng) -> Vec<f32> {
  let mut osc = Osc::default();
  let (mut low, mut high) = (Svf::new(800.0, 2.0), Svf::new(1800.0, 3.0));
  let pitch = rng.range(240.0, 300.0);
  shaped(0.55, |t| {
    let wobble = (t * TAU * 7.0).sin() * 0.04;
    let x = osc.saw(pitch * (1.0 + wobble - t * 0.15));
    (low.step(x).band + high.step(x).band * 0.5)
      * smooth(t / 0.05)
      * (1.0 - t / 0.55).max(0.0)
  })
}

fn cast(rng: &mut Rng) -> Vec<f32> {
  let secs = 0.7;
  let mut sweep = Svf::new(400.0, 2.0);
  let mut partials = [Osc::default(); 3];
  let lift = rng.range(0.9, 1.1);
  shaped(secs, |t| {
    sweep.tune((400.0 + 2000.0 * smooth(t / 0.35)) * lift, 2.0);
    let whoosh = sweep.step(rng.signed()).band
      * smooth(t / 0.2)
      * (-(t - 0.2).max(0.0) / 0.12).exp();
    let shimmer = partials
      .iter_mut()
      .zip([880.0, 1320.0, 1760.0])
      .map(|(osc, pitch)| osc.sine(pitch * lift) * (1.0 + (t * TAU * 11.0).sin()) * 0.5)
      .sum::<f32>()
      * perc(t - 0.15, 0.04, 0.18)
      * 0.12;
    whoosh + shimmer
  })
}

fn burst(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(1.0)];
  add(&mut out, 0.0, &glide(0.4, 130.0, 55.0, 0.004, 0.12), 0.8);
  add(&mut out, 0.0, &murmur(rng, 0.5, 1100.0, 0.003, 0.12), 0.6);
  let root = rng.range(600.0, 720.0);
  for (ratio, gain) in [(1.0, 0.2), (1.5, 0.14), (2.0, 0.08)] {
    add(&mut out, 0.02, &glide(0.8, root * ratio, root * ratio, 0.01, 0.25), gain)
  }
  softened(out, 3000.0)
}

fn pop() -> Vec<f32> { glide(0.1, 520.0, 900.0, 0.002, 0.035) }

fn creak(rng: &mut Rng) -> Vec<f32> {
  let mut osc = Osc::default();
  let mut body = Svf::new(300.0, 3.0);
  let mut out = shaped(0.5, |t| {
    body.tune(300.0 + 500.0 * (t / 0.5), 3.0);
    body.step(osc.saw(80.0 + (t * TAU * 5.0).sin() * 8.0)).band
      * smooth(t / 0.08)
      * (1.0 - t / 0.5).max(0.0)
  });
  add(&mut out, 0.38, &step(rng, Cue::Wood), 1.2);
  softened(out, 1600.0)
}

fn chime() -> Vec<f32> {
  let mut out = vec![0.0; len(1.4)];
  for (index, note) in [72.0, 76.0, 79.0, 84.0].into_iter().enumerate() {
    let pitch = midi(note);
    let mut tones = [Osc::default(); 2];
    let bell = shaped(0.9, |t| {
      (tones[0].sine(pitch) + tones[1].sine(pitch * 2.0) * 0.25) * perc(t, 0.005, 0.3)
    });
    add(&mut out, index as f32 * 0.11, &bell, 0.5)
  }
  out
}

fn warp(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(1.1)];
  for ratio in [1.0, 1.25, 1.5] {
    add(&mut out, 0.0, &glide(1.0, 300.0 * ratio, 620.0 * ratio, 0.25, 0.3), 0.25)
  }
  let mut sweep = Svf::new(500.0, 1.5);
  let whoosh = shaped(0.9, |t| {
    sweep.tune(500.0 + 1800.0 * smooth(t / 0.9), 1.5);
    sweep.step(rng.signed()).band * smooth(t / 0.4) * (1.0 - t / 0.9).max(0.0)
  });
  add(&mut out, 0.0, &whoosh, 0.6);
  out
}

fn thunder(rng: &mut Rng) -> Vec<f32> {
  let secs = 3.5;
  let (mut deep, mut deeper) = (Svf::new(220.0, 0.7), Svf::new(140.0, 0.7));
  let mut drift = Svf::new(3.0, 0.7);
  let mut out = shaped(secs, |t| {
    let roll = 0.6 + drift.step(rng.signed() * 40.0).low.clamp(-0.4, 0.6);
    deeper.step(deep.step(rng.signed()).low).low * roll * perc(t, 0.06, 1.1)
  });
  add(&mut out, 0.0, &murmur(rng, 0.25, 2200.0, 0.002, 0.07), 0.08);
  out
}

fn craft(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(0.3)];
  add(&mut out, 0.0, &step(rng, Cue::Wood), 1.0);
  add(&mut out, 0.09, &step(rng, Cue::Wood), 0.8);
  out
}

pub fn render(cue: Cue, seed: u64) -> Wave {
  let mut rng = Rng::new(seed);
  let samples = match cue {
    Cue::Soft | Cue::Hard | Cue::Wood => step(&mut rng, cue),
    Cue::Crumble => crumble(&mut rng),
    Cue::Thud => punch(&mut rng, 0.7),
    Cue::Punch => punch(&mut rng, 1.0),
    Cue::Hurt => hurt(&mut rng),
    Cue::Splash => splash(&mut rng),
    Cue::Swim => swim(&mut rng),
    Cue::Song => song(&mut rng),
    Cue::Caw => caw(&mut rng),
    Cue::Cry => cry(&mut rng),
    Cue::Honk => honk(&mut rng),
    Cue::Bleat => bleat(&mut rng),
    Cue::Cast => cast(&mut rng),
    Cue::Burst => burst(&mut rng),
    Cue::Pop => pop(),
    Cue::Creak => creak(&mut rng),
    Cue::Chime => chime(),
    Cue::Warp => warp(&mut rng),
    Cue::Thunder => thunder(&mut rng),
    Cue::Craft => craft(&mut rng)
  };
  let wave = Wave::mono(samples).normalized(0.5).trimmed();
  match cue {
    Cue::Chime | Cue::Warp | Cue::Burst | Cue::Cast => {
      Hall::new(0.5, 0.4, 1.0).apply(&wave, 0.25, false).normalized(0.5)
    }
    _ => wave
  }
}

pub fn rain() -> Wave {
  let mut rng = Rng::new(0x7A1);
  let span = len(4.0);
  let (mut hiss, mut body) = (Svf::new(3000.0, 0.6), Svf::new(500.0, 0.6));
  let mut raw: Vec<f32> = (0..span + len(0.5))
    .map(|_| {
      let x = rng.signed();
      hiss.step(x).low - body.step(x).low
    })
    .collect();
  for _ in 0..160 {
    let at = rng.range(0.0, 4.4);
    let pitch = rng.range(2000.0, 4500.0);
    let drop = glide(0.03, pitch, pitch * 0.7, 0.001, 0.008);
    add(&mut raw, at, &drop, rng.range(0.1, 0.35))
  }
  Wave::mono(seamless(raw, span)).normalized(0.4)
}
