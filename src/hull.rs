use {crate::voxels::chunk_of,
     bevy::{platform::collections::{HashMap, HashSet},
            prelude::*},
     std::sync::Arc};

#[derive(Clone, Copy)]
pub struct Facet {
  pub normal: IVec3,
  pub corners: [Vec3; 4]
}

impl Facet {
  fn triangles(self) -> impl Iterator<Item = ([Vec3; 3], Vec3)> {
    let [a, b, c, d] = self.corners;
    let facing = self.normal.as_vec3();
    [[a, b, c], [a, c, d]].into_iter().filter_map(move |triangle @ [a, b, c]| {
      let normal = (b - a).cross(c - a).normalize_or_zero();
      (normal != Vec3::ZERO).then(|| {
        (triangle, match normal.dot(facing) < 0.0 {
          true => -normal,
          false => normal
        })
      })
    })
  }
}

#[derive(Default)]
pub struct Hull(pub HashMap<IVec3, Vec<Facet>>);

#[derive(Resource, Default)]
pub struct Hulls(pub HashMap<IVec3, Arc<Hull>>);

fn closest(point: Vec3, [a, b, c]: [Vec3; 3]) -> Vec3 {
  let (ab, ac, ap) = (b - a, c - a, point - a);
  let (d1, d2) = (ab.dot(ap), ac.dot(ap));
  let bp = point - b;
  let (d3, d4) = (ab.dot(bp), ac.dot(bp));
  let cp = point - c;
  let (d5, d6) = (ab.dot(cp), ac.dot(cp));
  let vc = d1 * d4 - d3 * d2;
  let vb = d5 * d2 - d1 * d6;
  let va = d3 * d6 - d5 * d4;
  if d1 <= 0.0 && d2 <= 0.0 {
    a
  } else if d3 >= 0.0 && d4 <= d3 {
    b
  } else if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
    a + ab * (d1 / (d1 - d3))
  } else if d6 >= 0.0 && d5 <= d6 {
    c
  } else if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
    a + ac * (d2 / (d2 - d6))
  } else if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
    b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)))
  } else {
    let total = 1.0 / (va + vb + vc);
    a + ab * (vb * total) + ac * (vc * total)
  }
}

fn pierced(from: Vec3, toward: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
  let (ab, ac) = (b - a, c - a);
  let across = toward.cross(ac);
  let det = ab.dot(across);
  (det.abs() > 1e-9)
    .then(|| {
      let inverse = 1.0 / det;
      let offset = from - a;
      let u = offset.dot(across) * inverse;
      let up = offset.cross(ab);
      let v = toward.dot(up) * inverse;
      (u >= -1e-5 && v >= -1e-5 && u + v <= 1.0 + 1e-5).then(|| ac.dot(up) * inverse)
    })
    .flatten()
}

pub fn entry(from: Vec3, toward: Vec3, low: Vec3, high: Vec3) -> Option<f32> {
  let inverse = toward.normalize_or(Vec3::NEG_Z).recip();
  let (a, b) = ((low - from) * inverse, (high - from) * inverse);
  let (near, far) = (a.min(b).max_element(), a.max(b).min_element());
  (near <= far && far >= 0.0).then_some(near.max(0.0))
}

impl Hulls {
  pub fn ready(&self, key: IVec3) -> bool { self.0.contains_key(&key) }

  fn facets(&self, cell: IVec3) -> impl Iterator<Item = (IVec3, Facet)> {
    self
      .0
      .get(&chunk_of(cell))
      .and_then(|hull| hull.0.get(&cell))
      .into_iter()
      .flatten()
      .map(move |&facet| (cell, facet))
  }

  fn around(&self, low: Vec3, high: Vec3) -> impl Iterator<Item = (IVec3, Facet)> {
    let (low, high) = ((low - 0.5).floor().as_ivec3(), (high + 0.5).floor().as_ivec3());
    (low.y..=high.y).flat_map(move |y| {
      (low.z..=high.z).flat_map(move |z| {
        (low.x..=high.x).flat_map(move |x| self.facets(IVec3::new(x, y, z)))
      })
    })
  }

  pub fn push(&self, centre: Vec3, radius: f32) -> Vec3 {
    let reach = Vec3::splat(radius);
    self
      .around(centre - reach, centre + reach)
      .flat_map(|(_, facet)| facet.triangles())
      .fold(centre, |centre, (triangle, normal)| {
        let near = closest(centre, triangle);
        let gap = centre - near;
        let distance = gap.length();
        let behind = (centre - triangle[0]).dot(normal);
        match gap.dot(normal) >= 0.0 {
          true if distance < radius && distance > 1e-5 => {
            centre + gap / distance * (radius - distance)
          }
          true if distance <= 1e-5 => centre + normal * radius,
          false if behind > -radius && (distance - behind.abs()).abs() < 1e-3 => {
            centre + normal * (radius - behind)
          }
          _ => centre
        }
      })
      - centre
  }

  pub fn floor(&self, x: f32, z: f32, top: f32, bottom: f32) -> Option<f32> {
    let from = Vec3::new(x, top, z);
    self
      .around(Vec3::new(x, bottom, z), from)
      .flat_map(|(_, facet)| facet.triangles())
      .filter(|(_, normal)| normal.y > 0.02)
      .filter_map(|(triangle, _)| pierced(from, Vec3::NEG_Y, triangle))
      .filter(|&drop| drop >= -1e-4 && drop <= top - bottom)
      .map(|drop| top - drop)
      .reduce(f32::max)
  }

  pub fn cast(
    &self,
    from: Vec3,
    toward: Vec3,
    reach: f32
  ) -> Option<(f32, IVec3, IVec3)> {
    let toward = toward.normalize_or(Vec3::NEG_Z);
    let cells: HashSet<IVec3> = (0..=(reach * 2.0).ceil() as i32)
      .map(|step| (from + toward * step as f32 * 0.5).floor().as_ivec3())
      .flat_map(|cell| {
        (-1..=1).flat_map(move |y| {
          (-1..=1).flat_map(move |z| (-1..=1).map(move |x| cell + IVec3::new(x, y, z)))
        })
      })
      .collect();
    cells
      .into_iter()
      .flat_map(|cell| self.facets(cell))
      .flat_map(|(cell, facet)| {
        facet.triangles().filter_map(move |(triangle, normal)| {
          (normal.dot(toward) < 0.0)
            .then(|| pierced(from, toward, triangle))
            .flatten()
            .filter(|&distance| (0.0..=reach).contains(&distance))
            .map(|distance| (distance, cell, facet.normal))
        })
      })
      .min_by(|a, b| a.0.total_cmp(&b.0))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn floor_at(height: f32) -> Hulls {
    let cell = IVec3::new(0, height.round() as i32 - 1, 0);
    let facet = Facet {
      normal: IVec3::Y,
      corners: [
        Vec3::new(0.0, height, 1.0),
        Vec3::new(1.0, height, 1.0),
        Vec3::new(1.0, height, 0.0),
        Vec3::new(0.0, height, 0.0)
      ]
    };
    let mut hull = Hull::default();
    hull.0.insert(cell, vec![facet]);
    let mut hulls = Hulls::default();
    hulls.0.insert(chunk_of(cell), Arc::new(hull));
    hulls
  }

  #[test]
  fn spheres_rest_on_floors() {
    let hulls = floor_at(10.6);
    let push = hulls.push(Vec3::new(0.5, 10.8, 0.5), 0.3);
    assert!((push - Vec3::Y * 0.1).length() < 1e-4, "{push}");
    let under = hulls.push(Vec3::new(0.5, 10.5, 0.5), 0.3);
    assert!((under - Vec3::Y * 0.4).length() < 1e-4, "{under}");
  }

  #[test]
  fn rays_find_floors() {
    let hulls = floor_at(10.6);
    assert_eq!(hulls.floor(0.3, 0.7, 11.5, 10.0), Some(10.6));
    assert_eq!(hulls.floor(0.3, 0.7, 11.5, 10.7), None);
    let (distance, cell, normal) =
      hulls.cast(Vec3::new(0.5, 13.0, 0.5), Vec3::NEG_Y, 5.0).unwrap();
    assert!((distance - 2.4).abs() < 1e-4);
    assert_eq!((cell, normal), (IVec3::new(0, 10, 0), IVec3::Y))
  }
}
