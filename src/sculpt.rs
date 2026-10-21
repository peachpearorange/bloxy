use {crate::{cage::Cage, noise::unit},
     bevy::{asset::RenderAssetUsages,
            mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
            platform::collections::HashMap,
            prelude::*}};

const MOTTLE: f32 = 0.06;

pub struct Daub {
  pub at: Vec3,
  pub color: [f32; 3]
}

pub fn triangles(cage: &Cage) -> Vec<[u32; 3]> {
  let at = |corner: u32| cage.points[corner as usize];
  cage
    .faces
    .iter()
    .flat_map(|face| match face.len() {
      4 if at(face[0]).distance(at(face[2])) <= at(face[1]).distance(at(face[3])) => {
        vec![[face[0], face[1], face[2]], [face[0], face[2], face[3]]]
      }
      4 => vec![[face[0], face[1], face[3]], [face[1], face[2], face[3]]],
      sides => {
        (1..sides - 1).map(|corner| [face[0], face[corner], face[corner + 1]]).collect()
      }
    })
    .collect()
}

pub fn sculpt(
  cage: &Cage,
  paint: impl Fn(Vec3, Vec3) -> [f32; 3],
  daubs: &[Daub],
  joint: impl Fn(Vec3) -> u16
) -> Mesh {
  let triangles = triangles(cage);
  let normals = cage.vertex_normals();
  let corners = |triangle: &[u32; 3]| triangle.map(|corner| cage.points[corner as usize]);
  let centre = |triangle: &[u32; 3]| corners(triangle).iter().sum::<Vec3>() / 3.0;
  let daubed: HashMap<usize, [f32; 3]> = daubs
    .iter()
    .filter_map(|&Daub { at, color }| {
      (0..triangles.len())
        .min_by(|&one, &other| {
          centre(&triangles[one])
            .distance_squared(at)
            .total_cmp(&centre(&triangles[other]).distance_squared(at))
        })
        .map(|nearest| (nearest, color))
    })
    .collect();
  let colors = triangles.iter().enumerate().map(|(index, triangle)| {
    let [a, b, c] = corners(triangle);
    let color = daubed.get(&index).copied().unwrap_or_else(|| {
      let shade = 1.0 + (unit(0x5c0f, index as i32, 0, 0) * 2.0 - 1.0) * MOTTLE;
      paint(centre(triangle), (b - a).cross(c - a).normalize_or_zero())
        .map(|channel| channel * shade)
    });
    Color::srgb(color[0], color[1], color[2]).to_linear().to_f32_array()
  });
  let (positions, shading, painted, joints): (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 4]>,
    Vec<[u16; 4]>
  ) = triangles
    .iter()
    .zip(colors)
    .flat_map(|(triangle, color)| {
      triangle.map(|corner| {
        let at = cage.points[corner as usize];
        (at.to_array(), normals[corner as usize].to_array(), color, [joint(at), 0, 0, 0])
      })
    })
    .fold(
      (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
      |(mut positions, mut shading, mut painted, mut joints),
       (at, normal, color, bone)| {
        positions.push(at);
        shading.push(normal);
        painted.push(color);
        joints.push(bone);
        (positions, shading, painted, joints)
      }
    );
  let count = positions.len();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, shading)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32, 0.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, painted)
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_JOINT_INDEX,
      VertexAttributeValues::Uint16x4(joints)
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![
      [1.0f32, 0.0, 0.0, 0.0];
      count
    ])
    .with_inserted_indices(Indices::U32((0..count as u32).collect()))
}

#[cfg(test)]
pub fn portrait(mesh: &Mesh, yaw: f32, size: usize, wire: bool) -> Vec<[u8; 3]> {
  let vectors = |attribute| match mesh.attribute(attribute) {
    Some(VertexAttributeValues::Float32x3(values)) => {
      values.iter().map(|&value| Vec3::from(value).extend(1.0)).collect()
    }
    Some(VertexAttributeValues::Float32x4(values)) => {
      values.iter().map(|&value| Vec4::from(value)).collect()
    }
    _ => Vec::new()
  };
  let (positions, normals, colors) = (
    vectors(Mesh::ATTRIBUTE_POSITION),
    vectors(Mesh::ATTRIBUTE_NORMAL),
    vectors(Mesh::ATTRIBUTE_COLOR)
  );
  let turn = Quat::from_rotation_x(0.38) * Quat::from_rotation_y(yaw.to_radians());
  let seen: Vec<Vec3> = positions.iter().map(|at| turn * at.truncate()).collect();
  let (low, high) = seen.iter().fold((Vec2::MAX, Vec2::MIN), |(low, high), at| {
    (low.min(at.truncate()), high.max(at.truncate()))
  });
  let scale = size as f32 * 0.85 / (high - low).max_element();
  let screen = |at: Vec3| {
    let flat = (at.truncate() - (low + high) / 2.0) * scale;
    Vec3::new(size as f32 / 2.0 + flat.x, size as f32 / 2.0 - flat.y, at.z)
  };
  let light = Vec3::new(-0.35, 0.8, -0.45).normalize();
  let shade = |index: usize| {
    let lit = 0.5 + 0.65 * normals[index].truncate().dot(light).max(0.0);
    colors[index].truncate().map(|channel| channel.powf(1.0 / 2.2) * lit)
  };
  let mut pixels = vec![[90u8, 106, 120]; size * size];
  let mut depth = vec![f32::MIN; size * size];
  for corner in (0..seen.len()).step_by(3) {
    let [a, b, c] = [0, 1, 2].map(|step| screen(seen[corner + step]));
    let area = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
    let (low, high) = (a.min(b).min(c).max(Vec3::ZERO), a.max(b).max(c));
    for y in low.y as usize..(high.y.ceil() as usize).min(size) {
      for x in low.x as usize..(high.x.ceil() as usize).min(size) {
        let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        let edge = |from: Vec3, to: Vec3| {
          ((from.x - at.x) * (to.y - at.y) - (to.x - at.x) * (from.y - at.y)) / area
        };
        let weights = Vec3::new(edge(b, c), edge(c, a), 0.0);
        let weights = weights.with_z(1.0 - weights.x - weights.y);
        let near = weights.x * a.z + weights.y * b.z + weights.z * c.z;
        if area < 0.0 && weights.min_element() >= 0.0 && near > depth[y * size + x] {
          depth[y * size + x] = near;
          let color =
            (0..3).map(|step| shade(corner + step) * weights[step]).sum::<Vec3>();
          pixels[y * size + x] = match wire && weights.min_element() < 0.03 {
            true => [16, 16, 16],
            false => {
              color.to_array().map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u8)
            }
          }
        }
      }
    }
  }
  pixels
}
