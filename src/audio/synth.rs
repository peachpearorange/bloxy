use std::f32::consts::{PI, TAU};

pub const RATE: f32 = 44100.0;

pub fn len(secs: f32) -> usize { (secs.max(0.0) * RATE) as usize }

pub fn time(index: usize) -> f32 { index as f32 / RATE }

pub fn midi(note: f32) -> f32 { 440.0 * ((note - 69.0) / 12.0).exp2() }

pub fn smooth(x: f32) -> f32 {
  let x = x.clamp(0.0, 1.0);
  x * x * (3.0 - 2.0 * x)
}

pub fn perc(t: f32, attack: f32, decay: f32) -> f32 {
  if t < 0.0 {
    0.0
  } else if t < attack {
    t / attack
  } else {
    (-(t - attack) / decay).exp()
  }
}

#[derive(Clone)]
pub struct Rng(u64);

impl Default for Rng {
  fn default() -> Self { Self::new(0x51ce) }
}

impl Rng {
  pub fn new(seed: u64) -> Self {
    Self(
      seed.wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x2545_F491_4F6C_DD1D
        | 1
    )
  }

  pub fn unit(&mut self) -> f32 {
    self.0 ^= self.0 << 13;
    self.0 ^= self.0 >> 7;
    self.0 ^= self.0 << 17;
    (self.0 >> 40) as f32 / 16_777_216.0
  }

  pub fn signed(&mut self) -> f32 { self.unit() * 2.0 - 1.0 }

  pub fn range(&mut self, low: f32, high: f32) -> f32 { low + (high - low) * self.unit() }

  pub fn below(&mut self, count: usize) -> usize {
    ((self.unit() * count as f32) as usize).min(count.max(1) - 1)
  }
}

#[derive(Default, Clone, Copy)]
pub struct Taps {
  pub low: f32,
  pub band: f32
}

#[derive(Default, Clone, Copy)]
pub struct Svf {
  g: f32,
  k: f32,
  ic1: f32,
  ic2: f32
}

impl Svf {
  pub fn new(cutoff: f32, q: f32) -> Self {
    let mut filter = Self::default();
    filter.tune(cutoff, q);
    filter
  }

  pub fn tune(&mut self, cutoff: f32, q: f32) {
    self.g = (PI * cutoff.clamp(8.0, RATE * 0.45) / RATE).tan();
    self.k = 1.0 / q.max(0.3);
  }

  pub fn step(&mut self, x: f32) -> Taps {
    let &mut Svf { g, k, ic1, ic2 } = self;
    let v1 = (ic1 + g * (x - ic2)) / (1.0 + g * (g + k));
    let v2 = ic2 + g * v1;
    self.ic1 = 2.0 * v1 - ic1;
    self.ic2 = 2.0 * v2 - ic2;
    Taps { low: v2, band: v1 * k }
  }
}

fn blep(t: f32, dt: f32) -> f32 {
  if t < dt {
    let x = t / dt;
    x + x - x * x - 1.0
  } else if t > 1.0 - dt {
    let x = (t - 1.0) / dt;
    x * x + x + x + 1.0
  } else {
    0.0
  }
}

#[derive(Default, Clone, Copy)]
pub struct Osc(pub f32);

impl Osc {
  fn advance(&mut self, freq: f32) { self.0 = (self.0 + freq / RATE).rem_euclid(1.0) }

  pub fn sine(&mut self, freq: f32) -> f32 {
    let out = (self.0 * TAU).sin();
    self.advance(freq);
    out
  }

  pub fn saw(&mut self, freq: f32) -> f32 {
    let out = 2.0 * self.0 - 1.0 - blep(self.0, (freq / RATE).clamp(1e-6, 0.5));
    self.advance(freq);
    out
  }
}

pub fn seamless(raw: Vec<f32>, span: usize) -> Vec<f32> {
  let fade = raw.len().saturating_sub(span).min(span);
  (0..span)
    .map(|index| {
      (index < fade)
        .then(|| {
          let blend = index as f32 / fade as f32;
          raw[index] * blend.sqrt() + raw[span + index] * (1.0 - blend).sqrt()
        })
        .unwrap_or(raw[index])
    })
    .collect()
}

pub struct Wave(pub Vec<Vec<f32>>);

impl Wave {
  pub fn mono(samples: Vec<f32>) -> Self { Self(vec![samples]) }

  pub fn stereo(left: Vec<f32>, right: Vec<f32>) -> Self { Self(vec![left, right]) }

  pub fn peak(&self) -> f32 {
    self.0.iter().flatten().fold(0.0, |peak, sample| peak.max(sample.abs()))
  }

  #[cfg(test)]
  pub fn rms(&self) -> f32 {
    let count = self.0.iter().map(Vec::len).sum::<usize>().max(1);
    (self.0.iter().flatten().map(|sample| sample * sample).sum::<f32>() / count as f32)
      .sqrt()
  }

  pub fn normalized(self, target: f32) -> Self {
    let gain = target / self.peak().max(1e-9);
    Self(
      self
        .0
        .into_iter()
        .map(|channel| channel.into_iter().map(|x| x * gain).collect())
        .collect()
    )
  }

  pub fn trimmed(self) -> Self {
    let floor = self.peak() * 0.0005;
    let end = self
      .0
      .iter()
      .map(|channel| channel.iter().rposition(|x| x.abs() > floor).unwrap_or(0) + 1)
      .max()
      .unwrap_or(1);
    let fade = len(0.02).min(end);
    Self(
      self
        .0
        .into_iter()
        .map(|channel| {
          channel
            .into_iter()
            .take(end)
            .enumerate()
            .map(|(index, x)| x * ((end - index) as f32 / fade as f32).min(1.0))
            .collect()
        })
        .collect()
    )
  }

  #[cfg(test)]
  pub fn wav(&self) -> Vec<u8> {
    let channels = self.0.len() as u16;
    let frames = self.0.first().map(Vec::len).unwrap_or(0);
    let data = frames as u32 * channels as u32 * 2;
    let rate = RATE as u32;
    let header = [
      b"RIFF".to_vec(),
      (36 + data).to_le_bytes().to_vec(),
      b"WAVEfmt ".to_vec(),
      16u32.to_le_bytes().to_vec(),
      1u16.to_le_bytes().to_vec(),
      channels.to_le_bytes().to_vec(),
      rate.to_le_bytes().to_vec(),
      (rate * channels as u32 * 2).to_le_bytes().to_vec(),
      (channels * 2).to_le_bytes().to_vec(),
      16u16.to_le_bytes().to_vec(),
      b"data".to_vec(),
      data.to_le_bytes().to_vec()
    ]
    .concat();
    header
      .into_iter()
      .chain((0..frames).flat_map(|frame| {
        self.0.iter().flat_map(move |channel| {
          ((channel[frame].clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes()
        })
      }))
      .collect()
  }
}

struct Comb {
  line: Vec<f32>,
  at: usize,
  store: f32
}

impl Comb {
  fn step(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
    let out = self.line[self.at];
    self.store = out * (1.0 - damp) + self.store * damp;
    self.line[self.at] = x + self.store * feedback;
    self.at = (self.at + 1) % self.line.len();
    out
  }
}

struct Allpass {
  line: Vec<f32>,
  at: usize
}

impl Allpass {
  fn step(&mut self, x: f32) -> f32 {
    let held = self.line[self.at];
    self.line[self.at] = x + held * 0.5;
    self.at = (self.at + 1) % self.line.len();
    held - x
  }
}

pub struct Hall {
  combs: Vec<[Comb; 2]>,
  passes: Vec<[Allpass; 2]>,
  feedback: f32,
  damp: f32
}

impl Hall {
  const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
  const PASSES: [usize; 4] = [556, 441, 341, 225];
  const SPREAD: usize = 23;

  pub fn new(size: f32, damp: f32, stretch: f32) -> Self {
    let scaled = |base: usize, side: usize| {
      ((base + side * Self::SPREAD) as f32 * stretch * RATE / 44100.0) as usize
    };
    Self {
      combs: Self::COMBS
        .iter()
        .map(|&base| {
          [0, 1].map(|side| Comb {
            line: vec![0.0; scaled(base, side)],
            at: 0,
            store: 0.0
          })
        })
        .collect(),
      passes: Self::PASSES
        .iter()
        .map(|&base| {
          [0, 1].map(|side| Allpass { line: vec![0.0; scaled(base, side)], at: 0 })
        })
        .collect(),
      feedback: 0.7 + 0.28 * size,
      damp
    }
  }

  pub fn step(&mut self, x: f32) -> [f32; 2] {
    let Hall { combs, passes, feedback, damp } = self;
    let input = x * 0.015;
    [0, 1].map(|side| {
      let wet = combs
        .iter_mut()
        .map(|pair| pair[side].step(input, *feedback, *damp))
        .sum::<f32>();
      passes.iter_mut().fold(wet, |signal, pair| pair[side].step(signal))
    })
  }

  pub fn apply(mut self, dry: &Wave, wet: f32, circular: bool) -> Wave {
    let frames = dry.0[0].len();
    let channels = dry.0.len();
    let input = |frame: usize| {
      dry.0.iter().map(|channel| channel[frame]).sum::<f32>() / channels as f32
    };
    if circular {
      for frame in 0..frames {
        self.step(input(frame));
      }
    }
    let (left, right) = (0..frames)
      .map(|frame| {
        let [l, r] = self.step(input(frame));
        (dry.0[0][frame] + l * wet, dry.0[channels - 1][frame] + r * wet)
      })
      .unzip();
    Wave::stereo(left, right)
  }
}
