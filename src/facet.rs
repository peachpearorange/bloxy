use {crate::noise::unit,
     bevy::{asset::RenderAssetUsages,
            mesh::{Indices, PrimitiveTopology},
            prelude::*},
     std::f32::consts::TAU};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Form {
  Block,
  Lump { sides: u32, bands: u32, knobbly: f32 },
  Taper { toward: IVec3, end: f32, sides: u32 }
}

impl Form {
  pub const LUMP: Form = Form::Lump { sides: 7, bands: 4, knobbly: 0.1 };
  pub const PEBBLE: Form = Form::Lump { sides: 5, bands: 3, knobbly: 0.05 };

  pub fn taper(toward: IVec3, end: f32) -> Form { Form::Taper { toward, end, sides: 5 } }

  pub fn key(self) -> [u32; 5] {
    match self {
      Form::Block => [0; 5],
      Form::Lump { sides, bands, knobbly } => [1, sides, bands, knobbly.to_bits(), 0],
      Form::Taper { toward, end, sides } => [
        2,
        sides,
        end.to_bits(),
        ((toward.x + 1) * 9 + (toward.y + 1) * 3 + toward.z + 1) as u32,
        0
      ]
    }
  }

  fn triangles(self, size: Vec3, salt: u32) -> Vec<[Vec3; 3]> {
    let half = size * 0.5;
    match self {
      Form::Block => {
        let corner = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * half;
        let quads = [
          [(1., -1., -1.), (1., 1., -1.), (1., 1., 1.), (1., -1., 1.)],
          [(-1., -1., -1.), (-1., -1., 1.), (-1., 1., 1.), (-1., 1., -1.)],
          [(-1., 1., -1.), (-1., 1., 1.), (1., 1., 1.), (1., 1., -1.)],
          [(-1., -1., -1.), (1., -1., -1.), (1., -1., 1.), (-1., -1., 1.)],
          [(-1., -1., 1.), (1., -1., 1.), (1., 1., 1.), (-1., 1., 1.)],
          [(-1., -1., -1.), (-1., 1., -1.), (1., 1., -1.), (1., -1., -1.)]
        ];
        quads
          .iter()
          .flat_map(|quad| {
            let [a, b, c, d] = quad.map(|(x, y, z)| corner(x, y, z));
            [[a, b, c], [a, c, d]]
          })
          .collect()
      }
      Form::Lump { sides, bands, knobbly } => {
        let ring = |band: u32| -> Vec<Vec3> {
          let polar = band as f32 / bands as f32 * TAU * 0.5;
          (0..sides)
            .map(|side| {
              let azimuth = (side as f32 + band as f32 * 0.5) / sides as f32 * TAU;
              let bump = 1.0
                + (unit(salt, side as i32, band as i32, sides as i32) * 2.0 - 1.0)
                  * knobbly;
              Vec3::new(
                polar.sin() * azimuth.cos(),
                -polar.cos(),
                polar.sin() * azimuth.sin()
              ) * half
                * bump
            })
            .collect()
        };
        let (bottom, top) = (Vec3::NEG_Y * half, Vec3::Y * half);
        let rings: Vec<Vec<Vec3>> = (1..bands).map(ring).collect();
        let next = |side: u32| ((side + 1) % sides) as usize;
        let caps = (0..sides).flat_map(|side| {
          let (low, high) = (&rings[0], &rings[rings.len() - 1]);
          [[bottom, low[side as usize], low[next(side)]], [
            top,
            high[next(side)],
            high[side as usize]
          ]]
        });
        let belts = rings.windows(2).flat_map(|pair| {
          let (low, high) = (&pair[0], &pair[1]);
          (0..sides).flat_map(move |side| {
            let (a, b) = (low[side as usize], low[next(side)]);
            let (c, d) = (high[side as usize], high[next(side)]);
            [[a, c, b], [b, c, d]]
          })
        });
        caps.chain(belts).collect()
      }
      Form::Taper { toward, end, sides } => {
        let along =
          [toward.x, toward.y, toward.z].iter().position(|&axis| axis != 0).unwrap_or(1);
        let sign = toward[along].signum() as f32;
        let across = [(along + 1) % 3, (along + 2) % 3];
        let ring = |t: f32| -> Vec<Vec3> {
          let scale = if t * sign > 0.0 { end } else { 1.0 };
          (0..sides)
            .map(|side| {
              let angle = (side as f32 + 0.5) / sides as f32 * TAU;
              let mut point = Vec3::ZERO;
              point[along] = t;
              point[across[0]] = angle.cos() * 0.5 * scale;
              point[across[1]] = angle.sin() * 0.5 * scale;
              point * size
            })
            .collect()
        };
        let (low, high) = (ring(-0.5), ring(0.5));
        let ends = |ring: &Vec<Vec3>| -> Vec<[Vec3; 3]> {
          (1..sides as usize - 1)
            .map(|index| [ring[0], ring[index], ring[index + 1]])
            .collect()
        };
        (0..sides as usize)
          .flat_map(|side| {
            let next = (side + 1) % sides as usize;
            [[low[side], high[side], low[next]], [low[next], high[side], high[next]]]
          })
          .chain(ends(&low))
          .chain(ends(&high))
          .collect()
      }
    }
  }

  pub fn mesh(self, size: Vec3) -> Mesh {
    let salt =
      size.to_array().iter().fold(self.key()[1], |salt, part| salt ^ part.to_bits());
    let outward = self.triangles(size, salt).into_iter().map(|[a, b, c]| {
      let normal = (b - a).cross(c - a);
      match normal.dot(a + b + c) < 0.0 {
        true => [a, c, b],
        false => [a, b, c]
      }
    });
    let (positions, normals): (Vec<[f32; 3]>, Vec<[f32; 3]>) = outward
      .flat_map(|[a, b, c]| {
        let normal = (b - a).cross(c - a).normalize_or_zero();
        [a, b, c].map(|corner| (corner.to_array(), normal.to_array()))
      })
      .unzip();
    let count = positions.len() as u32;
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
      .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
      .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
      .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; count as usize])
      .with_inserted_indices(Indices::U32((0..count).collect()))
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Part {
  pub at: Vec3,
  pub size: Vec3,
  pub color: [f32; 3],
  pub form: Form,
  pub turn: Quat
}

impl Part {
  pub fn new(form: Form, at: [f32; 3], size: [f32; 3], color: [f32; 3]) -> Part {
    Part { at: Vec3::from(at), size: Vec3::from(size), color, form, turn: Quat::IDENTITY }
  }

  pub fn block(at: [f32; 3], size: [f32; 3], color: [f32; 3]) -> Part {
    Part::new(Form::Block, at, size, color)
  }

  pub fn lump(at: [f32; 3], size: [f32; 3], color: [f32; 3]) -> Part {
    Part::new(Form::LUMP, at, size, color)
  }

  pub fn pebble(at: [f32; 3], size: [f32; 3], color: [f32; 3]) -> Part {
    Part::new(Form::PEBBLE, at, size, color)
  }

  pub fn taper(
    toward: IVec3,
    end: f32,
    at: [f32; 3],
    size: [f32; 3],
    color: [f32; 3]
  ) -> Part {
    Part::new(Form::taper(toward, end), at, size, color)
  }

  pub fn turned(self, pitch: f32, yaw: f32, roll: f32) -> Part {
    Part { turn: Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll), ..self }
  }
}
