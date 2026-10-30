use {crate::{block::Block, generate, player::Pilot, protocol::Role, voxels::Voxels},
     bevy::{asset::{RenderAssetUsages, embedded_asset},
            mesh::{Indices, PrimitiveTopology},
            pbr::{ExtendedMaterial, MaterialExtension},
            prelude::*,
            render::render_resource::AsBindGroup,
            shader::ShaderRef},
     std::f32::consts::{FRAC_PI_2, PI, TAU}};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Finish {
  Chrome,
  Satin,
  Anodised,
  Rubber,
  Brass,
  Glass,
  Lamp
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
  Limb,
  Head,
  Chest,
  Dial
}

#[derive(Clone, Copy)]
struct Ring {
  y: f32,
  wide: f32,
  deep: f32,
  hard: bool,
  finish: Finish
}

const fn ring(y: f32, wide: f32, deep: f32, finish: Finish) -> Ring {
  Ring { y, wide, deep, hard: false, finish }
}

const fn edge(y: f32, wide: f32, deep: f32, finish: Finish) -> Ring {
  Ring { y, wide, deep, hard: true, finish }
}

const fn round(y: f32, radius: f32, finish: Finish) -> Ring {
  ring(y, radius, radius, finish)
}

const fn rim(y: f32, radius: f32, finish: Finish) -> Ring {
  edge(y, radius, radius, finish)
}

use Finish::*;

const HEAD: [Ring; 11] = [
  edge(0.000, 0.062, 0.059, Chrome),
  edge(0.000, 0.092, 0.088, Chrome),
  ring(0.034, 0.106, 0.102, Chrome),
  edge(0.068, 0.111, 0.107, Chrome),
  edge(0.070, 0.102, 0.098, Glass),
  ring(0.102, 0.105, 0.101, Glass),
  edge(0.134, 0.102, 0.098, Chrome),
  edge(0.136, 0.110, 0.106, Chrome),
  ring(0.178, 0.100, 0.096, Chrome),
  ring(0.212, 0.070, 0.067, Chrome),
  ring(0.231, 0.000, 0.000, Chrome)
];

const EAR: [Ring; 4] = [
  round(-0.004, 0.026, Brass),
  rim(0.012, 0.026, Brass),
  rim(0.012, 0.019, Anodised),
  round(0.015, 0.000, Anodised)
];

const ANTENNA: [Ring; 7] = [
  round(-0.006, 0.016, Brass),
  rim(0.008, 0.016, Brass),
  rim(0.008, 0.005, Satin),
  rim(0.086, 0.004, Lamp),
  round(0.100, 0.012, Lamp),
  round(0.109, 0.008, Lamp),
  round(0.113, 0.000, Lamp)
];

const NECK: [Ring; 3] =
  [round(0.000, 0.040, Rubber), round(0.024, 0.048, Rubber), round(0.052, 0.040, Rubber)];

const CHEST: [Ring; 7] = [
  edge(0.858, 0.060, 0.045, Chrome),
  edge(0.862, 0.128, 0.086, Chrome),
  ring(0.930, 0.165, 0.108, Chrome),
  ring(1.040, 0.186, 0.120, Chrome),
  ring(1.120, 0.178, 0.115, Chrome),
  ring(1.168, 0.140, 0.096, Chrome),
  ring(1.194, 0.060, 0.045, Chrome)
];

const DIAL: [Ring; 4] = [
  round(-0.006, 0.046, Brass),
  rim(0.012, 0.046, Brass),
  rim(0.012, 0.037, Glass),
  round(0.008, 0.000, Glass)
];

const WAIST: [Ring; 4] = [
  ring(0.775, 0.076, 0.056, Rubber),
  ring(0.800, 0.090, 0.066, Rubber),
  ring(0.828, 0.074, 0.054, Rubber),
  ring(0.862, 0.090, 0.066, Rubber)
];

const PELVIS: [Ring; 4] = [
  edge(0.668, 0.000, 0.000, Anodised),
  edge(0.668, 0.120, 0.074, Anodised),
  ring(0.722, 0.150, 0.088, Anodised),
  ring(0.785, 0.078, 0.058, Anodised)
];

const LEG: [Ring; 6] = [
  round(0.070, 0.028, Rubber),
  rim(0.094, 0.034, Anodised),
  ring(0.330, 0.052, 0.056, Anodised),
  rim(0.384, 0.046, Brass),
  rim(0.418, 0.050, Satin),
  round(0.715, 0.062, Satin)
];

const FOOT: [Ring; 4] = [
  edge(0.000, 0.000, 0.000, Rubber),
  edge(0.000, 0.046, 0.086, Rubber),
  edge(0.020, 0.050, 0.090, Chrome),
  ring(0.082, 0.026, 0.040, Chrome)
];

const SHOULDER: [Ring; 5] = [
  round(-0.056, 0.000, Chrome),
  round(-0.040, 0.040, Chrome),
  round(0.000, 0.057, Chrome),
  round(0.040, 0.040, Chrome),
  round(0.056, 0.000, Chrome)
];

const ARM: [Ring; 5] = [
  round(0.030, 0.037, Anodised),
  rim(0.200, 0.041, Brass),
  rim(0.226, 0.047, Satin),
  round(0.400, 0.050, Satin),
  rim(0.418, 0.032, Anodised)
];

const HAND: [Ring; 4] = [
  ring(0.430, 0.032, 0.022, Anodised),
  edge(0.452, 0.042, 0.026, Chrome),
  ring(0.520, 0.042, 0.027, Chrome),
  ring(0.566, 0.000, 0.000, Chrome)
];

struct Lathe {
  rings: &'static [Ring],
  sides: usize,
  square: f32,
  part: Part,
  place: Transform
}

struct Corner {
  at: Vec3,
  normal: Vec3,
  code: f32,
  arc: f32
}

fn bent(angle: f32, square: f32) -> Vec2 {
  let (sin, cos) = angle.sin_cos();
  let curve = |value: f32| value.signum() * value.abs().powf(2.0 / square);
  Vec2::new(curve(cos), curve(sin))
}

fn strips(rings: &[Ring]) -> Vec<(usize, usize)> {
  let breaks: Vec<usize> = (1..rings.len() - 1)
    .filter(|&index| rings[index].hard || rings[index].finish != rings[index - 1].finish)
    .collect();
  std::iter::once(0)
    .chain(breaks.iter().copied())
    .zip(breaks.iter().copied().chain(std::iter::once(rings.len() - 1)))
    .collect()
}

fn turned(lathe: &Lathe) -> Vec<Corner> {
  let &Lathe { rings, sides, square, part, place } = lathe;
  let around: Vec<Vec2> = (0..sides)
    .map(|side| bent(side as f32 / sides as f32 * TAU - FRAC_PI_2, square))
    .collect();
  let point = |ring: &Ring, side: usize| {
    let Vec2 { x, y } = around[side % sides];
    Vec3::new(x * ring.wide, ring.y, y * ring.deep)
  };
  let arcs =
    rings.iter().skip(1).zip(rings).fold(vec![0.0], |mut arcs, (ring, before)| {
      arcs.push(
        arcs.last().copied().unwrap_or(0.0) + point(ring, 0).distance(point(before, 0))
      );
      arcs
    });
  strips(rings)
    .into_iter()
    .flat_map(|(from, to)| {
      let quads: Vec<[(usize, usize); 4]> = (from..to)
        .flat_map(|low| {
          (0..sides).map(move |side| {
            [(low, side), (low + 1, side), (low + 1, side + 1), (low, side + 1)]
          })
        })
        .collect();
      let triangles: Vec<[(usize, usize); 3]> = quads
        .iter()
        .flat_map(|&[a, b, c, d]| [[a, b, c], [a, c, d]])
        .filter(|triangle| {
          let [a, b, c] = triangle.map(|(ring, side)| point(&rings[ring], side));
          (b - a).cross(c - a).length_squared() > 1e-12
        })
        .collect();
      let facet = |triangle: &[(usize, usize); 3]| {
        let [a, b, c] = triangle.map(|(ring, side)| point(&rings[ring], side));
        (b - a).cross(c - a)
      };
      let smooth = triangles.iter().fold(
        vec![Vec3::ZERO; (to - from + 1) * sides],
        |mut normals, triangle| {
          for &(ring, side) in triangle {
            normals[(ring - from) * sides + side % sides] += facet(triangle)
          }
          normals
        }
      );
      let code = (rings[from].finish as u32 + 8 * part as u32) as f32;
      triangles
        .into_iter()
        .flat_map(|triangle| {
          triangle.map(|(ring, side)| Corner {
            at: place.transform_point(point(&rings[ring], side)),
            normal: place.rotation
              * smooth[(ring - from) * sides + side % sides].normalize_or(Vec3::Y),
            code,
            arc: arcs[ring]
          })
        })
        .collect::<Vec<_>>()
    })
    .collect()
}

fn lathes() -> Vec<Lathe> {
  let upright = |at: Vec3| Transform::from_translation(at);
  let along = |at: Vec3, toward: Vec3| {
    Transform::from_translation(at)
      .with_rotation(Quat::from_rotation_arc(Vec3::Y, toward.normalize()))
  };
  let head = Vec3::Y * 1.244;
  let sided = |side: f32| {
    let shoulder = Vec3::new(side * 0.212, 1.118, 0.0);
    let hanging = Vec3::new(side * 0.16, -1.0, -0.1);
    [
      Lathe {
        rings: &LEG,
        sides: 8,
        square: 2.0,
        part: Part::Limb,
        place: upright(Vec3::X * side * 0.098)
      },
      Lathe {
        rings: &FOOT,
        sides: 6,
        square: 3.2,
        part: Part::Limb,
        place: upright(Vec3::new(side * 0.1, 0.0, -0.032))
      },
      Lathe {
        rings: &SHOULDER,
        sides: 6,
        square: 2.0,
        part: Part::Limb,
        place: along(shoulder, Vec3::new(side, 0.0, 0.0))
      },
      Lathe {
        rings: &ARM,
        sides: 8,
        square: 2.0,
        part: Part::Limb,
        place: along(shoulder, hanging)
      },
      Lathe {
        rings: &HAND,
        sides: 6,
        square: 2.4,
        part: Part::Limb,
        place: along(shoulder, hanging)
          * Transform::from_rotation(Quat::from_rotation_y(-side * FRAC_PI_2))
      },
      Lathe {
        rings: &EAR,
        sides: 6,
        square: 2.0,
        part: Part::Head,
        place: along(head + Vec3::new(side * 0.098, 0.102, 0.0), Vec3::X * side)
      }
    ]
  };
  [
    Lathe {
      rings: &HEAD,
      sides: 16,
      square: 2.4,
      part: Part::Head,
      place: upright(head)
    },
    Lathe {
      rings: &ANTENNA,
      sides: 6,
      square: 2.0,
      part: Part::Head,
      place: upright(head + Vec3::Y * 0.222)
    },
    Lathe {
      rings: &NECK,
      sides: 6,
      square: 2.0,
      part: Part::Limb,
      place: upright(Vec3::Y * 1.192)
    },
    Lathe {
      rings: &CHEST,
      sides: 12,
      square: 3.0,
      part: Part::Chest,
      place: upright(Vec3::ZERO)
    },
    Lathe {
      rings: &DIAL,
      sides: 12,
      square: 2.0,
      part: Part::Dial,
      place: along(Vec3::new(0.0, 1.0, -0.108), Vec3::NEG_Z)
        * Transform::from_rotation(Quat::from_rotation_y(PI / 12.0))
    },
    Lathe {
      rings: &WAIST,
      sides: 8,
      square: 2.6,
      part: Part::Limb,
      place: upright(Vec3::ZERO)
    },
    Lathe {
      rings: &PELVIS,
      sides: 8,
      square: 3.0,
      part: Part::Limb,
      place: upright(Vec3::ZERO)
    }
  ]
  .into_iter()
  .chain(sided(1.0))
  .chain(sided(-1.0))
  .collect()
}

pub fn mesh() -> Mesh {
  let corners: Vec<Corner> = lathes().iter().flat_map(turned).collect();
  let count = corners.len();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      corners.iter().map(|corner| corner.at.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, vec![
      [1.0f32, 0.0, 0.0, 1.0];
      count
    ])
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_COLOR,
      corners
        .iter()
        .map(|corner| corner.normal.extend(corner.code).to_array())
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_0,
      corners.iter().map(|corner| [corner.at.x, corner.at.y]).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_1,
      corners.iter().map(|corner| [corner.at.z, corner.arc]).collect::<Vec<_>>()
    )
    .with_inserted_indices(Indices::U32((0..count as u32).collect()))
}

#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct Polish {}

impl MaterialExtension for Polish {
  fn fragment_shader() -> ShaderRef { "embedded://bloxy/droid.wgsl".into() }
}

pub type Polished = ExtendedMaterial<StandardMaterial, Polish>;

fn polished() -> Polished {
  ExtendedMaterial {
    base: StandardMaterial {
      perceptual_roughness: 0.1,
      metallic: 1.0,
      anisotropy_strength: 0.01,
      ..default()
    },
    extension: Polish {}
  }
}

fn stand(
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  mut done: Local<bool>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<Polished>>,
  mut commands: Commands
) {
  if !*done
    && pilot.at != Vec3::ZERO
    && let Some(voxels) = voxels
    && let spot = pilot.at + Vec3::new(-1.8, 0.0, -1.6)
    && let column = spot.floor().as_ivec3()
    && let high = generate::height(voxels.seed, column.x, column.z) + 4
    && let Some(ground) = (0..12)
      .map(|depth| column.with_y(high - depth))
      .find(|&at| voxels.block(at).is_some_and(Block::solid))
  {
    *done = true;
    let at = spot.with_y(ground.y as f32 + 1.0);
    let toward = pilot.at - at;
    let yaw = (-toward.x).atan2(-toward.z);
    info!("The droid stands at {at:?}, facing yaw {yaw}");
    commands.spawn((
      Mesh3d(meshes.add(mesh())),
      MeshMaterial3d(materials.add(polished())),
      Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw))
    ));
  }
}

pub struct Droids;

impl Plugin for Droids {
  fn build(&self, app: &mut App) {
    if app.world().resource::<Role>().plays() {
      embedded_asset!(app, "droid.wgsl");
      app.add_plugins(MaterialPlugin::<Polished>::default()).add_systems(
        Update,
        stand.run_if(|| crate::opts::opts().droid).run_if(resource_exists::<Pilot>)
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn droid_is_about_a_thousand_triangles() {
    let triangles = lathes().iter().map(|lathe| turned(lathe).len() / 3).sum::<usize>();
    println!("droid: {triangles} triangles");
    assert!((1000..1250).contains(&triangles))
  }
}
