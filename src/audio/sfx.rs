use strum::VariantArray;
use {super::synth::{Hall, Osc, Rng, Svf, Wave, len, midi, perc, seamless, smooth, time},
     std::f32::consts::TAU};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, VariantArray)]
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
  Craft,
  Rustle,
  Clod
}

impl Cue {
  pub const ALL: [Cue; Cue::VARIANTS.len()] = *Cue::VARIANTS.first_chunk().unwrap();

  pub fn variants(self) -> u64 {
    match self {
      Cue::Soft | Cue::Hard | Cue::Wood => 4,
      Cue::Crumble
      | Cue::Punch
      | Cue::Song
      | Cue::Caw
      | Cue::Cry
      | Cue::Swim
      | Cue::Bleat
      | Cue::Rustle
      | Cue::Clod => 3,
      Cue::Splash | Cue::Honk | Cue::Cast | Cue::Burst | Cue::Thunder | Cue::Hurt => 2,
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

fn thump(
  rng: &mut Rng,
  secs: f32,
  (low, high): (f32, f32),
  attack: f32,
  decay: f32
) -> Vec<f32> {
  let cutoff = rng.range(low, high);
  let (mut first, mut second) = (Svf::new(cutoff, 0.6), Svf::new(cutoff, 0.6));
  shaped(secs, |t| second.step(first.step(rng.signed()).low).low * perc(t, attack, decay))
}

fn eased(t: f32, attack: f32, decay: f32) -> f32 {
  smooth(t / attack) * (-(t - attack).max(0.0) / decay).exp()
}

fn hush(
  rng: &mut Rng,
  secs: f32,
  (low, high): (f32, f32),
  attack: f32,
  decay: f32
) -> Vec<f32> {
  let mut filter = Svf::new(rng.range(low, high), 0.8);
  shaped(secs, |t| filter.step(rng.signed()).band * eased(t, attack, decay))
}

fn step(rng: &mut Rng, cue: Cue) -> Vec<f32> {
  let mut out = vec![0.0; len(0.22)];
  match cue {
    Cue::Soft => {
      add(&mut out, 0.0, &thump(rng, 0.18, (380.0, 520.0), 0.018, 0.045), 1.0);
      for _ in 0..5 {
        let at = rng.range(0.005, 0.06);
        add(&mut out, at, &hush(rng, 0.08, (900.0, 1600.0), 0.008, 0.018), 0.05)
      }
    }
    Cue::Hard => {
      add(&mut out, 0.0, &thump(rng, 0.14, (260.0, 340.0), 0.012, 0.03), 1.0);
      add(&mut out, 0.0, &hush(rng, 0.08, (900.0, 1300.0), 0.005, 0.012), 0.22);
      add(
        &mut out,
        rng.range(0.01, 0.02),
        &hush(rng, 0.06, (700.0, 1100.0), 0.004, 0.01),
        0.1
      )
    }
    _ => {
      add(&mut out, 0.0, &thump(rng, 0.16, (220.0, 300.0), 0.008, 0.035), 1.0);
      add(&mut out, 0.0, &hush(rng, 0.08, (450.0, 750.0), 0.006, 0.018), 0.35)
    }
  }
  softened(out, 1800.0)
}

fn rustle(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(0.35)];
  for _ in 0..9 {
    let at = rng.range(0.0, 0.14);
    add(
      &mut out,
      at,
      &hush(rng, 0.12, (1000.0, 2400.0), 0.025, 0.035),
      rng.range(0.4, 1.0)
    )
  }
  softened(out, 2600.0)
}

fn clod(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(0.45)];
  add(&mut out, 0.0, &thump(rng, 0.3, (240.0, 320.0), 0.02, 0.07), 1.0);
  for _ in 0..8 {
    let at = rng.range(0.02, 0.22);
    let piece = hush(rng, 0.08, (500.0, 1100.0), 0.008, 0.02);
    add(&mut out, at, &piece, rng.range(0.1, 0.25) * (1.0 - at * 3.0).max(0.2))
  }
  softened(out, 1600.0)
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
  let pitch = rng.range(85.0, 110.0) * low;
  add(&mut out, 0.0, &glide(0.16, pitch, pitch * 0.55, 0.0015, 0.045), 0.9);
  add(&mut out, 0.0, &thump(rng, 0.14, (500.0 * low, 540.0 * low), 0.001, 0.035), 1.4);
  add(&mut out, 0.0, &grain(rng, 0.03, (1400.0, 2200.0), 0.9, 0.0005, 0.006), 0.6);
  softened(out.into_iter().map(|x| (x * 2.2).tanh()).collect(), 3200.0)
}

fn thud(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(0.25)];
  add(&mut out, 0.0, &thump(rng, 0.22, (200.0, 260.0), 0.006, 0.045), 1.0);
  add(&mut out, 0.0, &glide(0.15, 95.0, 60.0, 0.006, 0.04), 0.5);
  softened(out, 1500.0)
}

fn hurt(rng: &mut Rng) -> Vec<f32> {
  let mut out = punch(rng, 0.85);
  let mut osc = Osc::default();
  let mut formant = Svf::new(650.0, 2.0);
  let oof =
    shaped(0.2, |t| formant.step(osc.saw(170.0 - t * 200.0)).band * perc(t, 0.015, 0.06));
  add(&mut out, 0.02, &oof, 0.5);
  softened(out, 2400.0)
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

fn warble(rng: &mut Rng, secs: f32, from: f32, to: f32, flutter: f32) -> Vec<f32> {
  let (mut carrier, mut wobble) = (Osc::default(), Osc::default());
  let rate = rng.range(70.0, 110.0);
  shaped(secs, |t| {
    let along = (t / secs).min(1.0);
    let bend = 1.0 + wobble.sine(rate) * flutter;
    let pitch = (from + (to - from) * along * along.sqrt()) * bend;
    let tone = carrier.sine(pitch);
    (tone + (tone * tone - 0.5) * 0.25) * smooth(t / 0.006) * (1.0 - along).powf(0.7)
  })
}

fn song(rng: &mut Rng) -> Vec<f32> {
  let mut out = vec![0.0; len(1.0)];
  let base = rng.range(2600.0, 3400.0);
  let notes = 3 + rng.below(3);
  let trill = rng.below(notes);
  let mut at = 0.0;
  for index in 0..notes {
    match index == trill {
      true => {
        let pitch = base * rng.range(0.8, 1.2);
        for _ in 0..5 + rng.below(4) {
          add(&mut out, at, &warble(rng, 0.022, pitch * 1.25, pitch * 0.85, 0.02), 0.7);
          at += 0.034
        }
      }
      false => {
        let secs = rng.range(0.04, 0.08);
        let (from, to) = (base * rng.range(1.1, 1.5), base * rng.range(0.6, 0.85));
        add(&mut out, at, &warble(rng, secs, from, to, 0.04), 1.0);
        at += secs + rng.range(0.03, 0.07)
      }
    }
  }
  softened(out, 7000.0)
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
  let mut out = vec![0.0; len(1.2)];
  let calls = 2 + rng.below(3);
  let start = rng.range(620.0, 760.0);
  for index in 0..calls {
    let secs = match index {
      0 => 0.32,
      _ => rng.range(0.16, 0.22)
    };
    let mut osc = Osc::default();
    let (mut nasal, mut bright) = (Svf::new(1900.0, 3.0), Svf::new(3400.0, 4.0));
    let lift = 1.0 - index as f32 * 0.04;
    let one = shaped(secs, |t| {
      let along = t / secs;
      let contour = 1.0 + 0.35 * smooth(along / 0.3) - 0.55 * smooth((along - 0.3) / 0.7);
      let rough = 1.0 + rng.signed() * 0.25;
      let x = osc.saw(start * lift * contour * rough.sqrt());
      (nasal.step(x).band + bright.step(x).band * 0.5)
        * smooth(t / 0.025)
        * (1.0 - along).max(0.0).powf(0.5)
    });
    add(&mut out, 0.04 + index as f32 * 0.3, &one, if index == 0 { 1.0 } else { 0.75 })
  }
  softened(out, 4500.0)
}

fn honk(rng: &mut Rng) -> Vec<f32> {
  let mut osc = Osc::default();
  let mut tone = Svf::new(700.0, 1.2);
  let pitch = rng.range(300.0, 360.0);
  shaped(0.3, |t| tone.step(osc.saw(pitch * (1.0 - t * 0.2))).low * perc(t, 0.03, 0.08))
}

fn bleat(rng: &mut Rng) -> Vec<f32> {
  let secs = rng.range(0.6, 0.85);
  let pitch = rng.range(170.0, 240.0);
  let quaver = rng.range(17.0, 23.0);
  let mut glottis = Osc::default();
  let mut tremolo = Osc::default();
  let mut source = Svf::new(1800.0, 0.6);
  let mut lips = Svf::new(300.0, 0.7);
  let mut formants = [
    (Svf::new(650.0, 7.0), 1.0),
    (Svf::new(1150.0, 8.0), 0.6),
    (Svf::new(2600.0, 9.0), 0.25)
  ];
  shaped(secs, |t| {
    let along = t / secs;
    let shake = tremolo.sine(quaver);
    let bend = 1.0 + 0.04 * shake + 0.12 * smooth(along / 0.15)
      - 0.25 * smooth((along - 0.6) / 0.4);
    let voiced = source.step(glottis.saw(pitch * bend)).low;
    let breath = rng.signed() * 0.15;
    let vowel = smooth(along / 0.25);
    let opened = lips.step(voiced + breath);
    lips.tune(300.0 + 3200.0 * smooth(t / 0.06), 0.7);
    let x = opened.low;
    let sung = formants
      .iter_mut()
      .zip([(560.0, 760.0), (1700.0, 1250.0), (2500.0, 2700.0)])
      .map(|((filter, gain), (from, to))| {
        filter.tune(from + (to - from) * vowel, 8.0);
        filter.step(x).band * *gain
      })
      .sum::<f32>();
    sung
      * (1.0 - 0.6 * (0.5 + 0.5 * shake))
      * smooth(t / 0.05)
      * (1.0 - along).max(0.0).powf(0.8)
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
    Cue::Thud => thud(&mut rng),
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
    Cue::Craft => craft(&mut rng),
    Cue::Rustle => rustle(&mut rng),
    Cue::Clod => clod(&mut rng)
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
  let span = len(6.0);
  let total = span + len(0.5);
  let (mut body, mut air) = (Svf::new(700.0, 0.6), Svf::new(2400.0, 0.5));
  let mut swell = Svf::new(0.6, 0.6);
  let mut raw: Vec<f32> = (0..total)
    .map(|_| {
      let gust = 0.75 + swell.step(rng.signed() * 30.0).low.clamp(-0.25, 0.25);
      let x = rng.signed();
      (body.step(x).low + air.step(x).band * 0.08) * gust
    })
    .collect();
  for _ in 0..2600 {
    let at = rng.range(0.0, 6.45);
    let tick = hush(&mut rng, 0.012, (700.0, 3200.0), 0.001, 0.003);
    add(&mut raw, at, &tick, rng.range(0.05, 0.4).powi(2) * 2.0)
  }
  for _ in 0..90 {
    let at = rng.range(0.0, 6.45);
    let pitch = rng.range(1400.0, 2600.0);
    let plip = glide(0.03, pitch, pitch * 1.5, 0.002, 0.007);
    add(&mut raw, at, &plip, rng.range(0.05, 0.15))
  }
  Wave::mono(seamless(softened(raw, 3500.0), span)).normalized(0.4)
}
