const PRIME_X: u32 = 0x9E37_79B1;
const PRIME_Y: u32 = 0x85EB_CA77;
const PRIME_Z: u32 = 0xC2B2_AE3D;

pub fn hash(seed: u32, x: i32, y: i32, z: i32) -> u32 {
  let mixed = seed
    ^ (x as u32).wrapping_mul(PRIME_X)
    ^ (y as u32).wrapping_mul(PRIME_Y)
    ^ (z as u32).wrapping_mul(PRIME_Z);
  let folded = (mixed ^ (mixed >> 15)).wrapping_mul(0x2C1B_3C6D);
  let folded = (folded ^ (folded >> 12)).wrapping_mul(0x297A_2D39);
  folded ^ (folded >> 15)
}

pub fn unit(seed: u32, x: i32, y: i32, z: i32) -> f32 {
  (hash(seed, x, y, z) >> 8) as f32 / (1u32 << 24) as f32
}

pub fn smoothstep(from: f32, to: f32, value: f32) -> f32 {
  let t = ((value - from) / (to - from)).clamp(0.0, 1.0);
  t * t * (3.0 - 2.0 * t)
}

fn fade(t: f32) -> f32 { t * t * t * (t * (t * 6.0 - 15.0) + 10.0) }

fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

const GRADIENTS_2D: [(f32, f32); 8] = [
  (1.0, 0.0),
  (-1.0, 0.0),
  (0.0, 1.0),
  (0.0, -1.0),
  (0.7071, 0.7071),
  (-0.7071, 0.7071),
  (0.7071, -0.7071),
  (-0.7071, -0.7071)
];

pub fn perlin2(seed: u32, x: f32, z: f32) -> f32 {
  let (cell_x, cell_z) = (x.floor(), z.floor());
  let (fx, fz) = (x - cell_x, z - cell_z);
  let (ix, iz) = (cell_x as i32, cell_z as i32);
  let corner = |dx: i32, dz: i32| {
    let (gx, gz) = GRADIENTS_2D[(hash(seed, ix + dx, 0, iz + dz) & 7) as usize];
    gx * (fx - dx as f32) + gz * (fz - dz as f32)
  };
  let (u, v) = (fade(fx), fade(fz));
  lerp(lerp(corner(0, 0), corner(1, 0), u), lerp(corner(0, 1), corner(1, 1), u), v) * 1.4
}

const GRADIENTS_3D: [(f32, f32, f32); 12] = [
  (1.0, 1.0, 0.0),
  (-1.0, 1.0, 0.0),
  (1.0, -1.0, 0.0),
  (-1.0, -1.0, 0.0),
  (1.0, 0.0, 1.0),
  (-1.0, 0.0, 1.0),
  (1.0, 0.0, -1.0),
  (-1.0, 0.0, -1.0),
  (0.0, 1.0, 1.0),
  (0.0, -1.0, 1.0),
  (0.0, 1.0, -1.0),
  (0.0, -1.0, -1.0)
];

pub fn perlin3(seed: u32, x: f32, y: f32, z: f32) -> f32 {
  let (cell_x, cell_y, cell_z) = (x.floor(), y.floor(), z.floor());
  let (fx, fy, fz) = (x - cell_x, y - cell_y, z - cell_z);
  let (ix, iy, iz) = (cell_x as i32, cell_y as i32, cell_z as i32);
  let corner = |dx: i32, dy: i32, dz: i32| {
    let (gx, gy, gz) =
      GRADIENTS_3D[(hash(seed, ix + dx, iy + dy, iz + dz) % 12) as usize];
    gx * (fx - dx as f32) + gy * (fy - dy as f32) + gz * (fz - dz as f32)
  };
  let (u, v, w) = (fade(fx), fade(fy), fade(fz));
  let layer = |dz: i32| {
    lerp(
      lerp(corner(0, 0, dz), corner(1, 0, dz), u),
      lerp(corner(0, 1, dz), corner(1, 1, dz), u),
      v
    )
  };
  lerp(layer(0), layer(1), w)
}

pub fn fbm2(seed: u32, x: f32, z: f32, octaves: u32) -> f32 {
  let (sum, _, _, total) =
    (0..octaves).fold((0.0, 1.0, 1.0, 0.0), |(sum, scale, amplitude, total), octave| {
      (
        sum + perlin2(seed.wrapping_add(octave * 7919), x * scale, z * scale) * amplitude,
        scale * 2.03,
        amplitude * 0.5,
        total + amplitude
      )
    });
  sum / total
}
