use {crate::{block::Block,
             ground::Concealed,
             mesh::{Corner, lone},
             noise::unit,
             player::{Aim, Pilot},
             protocol::plays,
             stream::Palette,
             voxels::{Voxels, chunk_of}},
     bevy::{asset::RenderAssetUsages,
            light::NotShadowCaster,
            mesh::{Indices, PrimitiveTopology},
            prelude::*}};

const GRID: f32 = 8.0;
const CENTRE: Vec3 = Vec3::splat(0.5);
const FINISHING: f32 = 0.4;

type Shard = ([Corner; 3], f32);

#[derive(Component)]
struct Crumbling {
  mesh: Handle<Mesh>,
  shape: Option<(IVec3, Block, Vec<Shard>)>
}

fn between(a: Corner, b: Corner, c: Corner, s: f32, t: f32) -> Corner {
  let r = 1.0 - s - t;
  let color = [0, 1, 2, 3]
    .map(|channel| a.color[channel] * r + b.color[channel] * s + c.color[channel] * t);
  Corner {
    position: a.position * r + b.position * s + c.position * t,
    normal: a.normal,
    uv: a.uv * r + b.uv * s + c.uv * t,
    color
  }
}

fn shatter(triangles: Vec<[Corner; 3]>, at: IVec3) -> Vec<Shard> {
  triangles
    .into_iter()
    .flat_map(|[a, b, c]| {
      let longest = [(a, b), (b, c), (c, a)]
        .iter()
        .map(|(from, to)| from.position.distance(to.position))
        .fold(0.0, f32::max);
      let parts = (longest * GRID).ceil().clamp(1.0, GRID) as u32;
      let step = 1.0 / parts as f32;
      let point =
        move |i: u32, j: u32| between(a, b, c, i as f32 * step, j as f32 * step);
      (0..parts).flat_map(move |j| {
        (0..parts - j).flat_map(move |i| {
          let up = [point(i, j), point(i + 1, j), point(i, j + 1)];
          let down = (i + j + 1 < parts)
            .then(|| [point(i + 1, j), point(i + 1, j + 1), point(i, j + 1)]);
          std::iter::once(up).chain(down)
        })
      })
    })
    .map(|shard| {
      let middle = (shard[0].position + shard[1].position + shard[2].position) / 3.0;
      let cell = (middle * GRID - shard[0].normal * 0.01).floor().as_ivec3();
      let side = shard[0].normal.dot(Vec3::new(1.0, 2.0, 4.0)).round() as i32;
      let crack = unit(
        at.x as u32 ^ (side as u32) << 20,
        at.y + cell.x * 31,
        at.z + cell.y * 17,
        cell.z
      );
      (shard, crack)
    })
    .collect()
}

fn contort(point: Vec3, progress: f32, now: f32) -> Vec3 {
  let wave = (point.x * 7.1 + now * 9.0).sin()
    * (point.y * 6.3 - now * 7.0).sin()
    * (point.z * 8.7 + now * 11.0).sin();
  let swell = 0.02 + progress * (0.12 + 0.08 * wave);
  let beat = (now * 13.0).sin() * 0.5 + 0.5;
  let twist = progress * 0.12 * (now * 6.0 + point.y * 3.0).sin();
  let stretch = Vec3::new(
    (1.0 + progress * 0.16 * (1.0 - beat)) * (1.0 + twist.abs()),
    1.0 + progress * 0.16 * beat,
    (1.0 + progress * 0.16 * (1.0 - beat)) * (1.0 + twist.abs())
  );
  CENTRE + Quat::from_rotation_y(twist) * ((point - CENTRE) * stretch * (1.0 + swell))
}

fn crumbled(shards: &[Shard], progress: f32, now: f32) -> Mesh {
  let shake = Vec3::new((now * 53.0).sin(), (now * 61.0).sin(), (now * 47.0).sin())
    * 0.02
    * progress;
  let corners: Vec<(Vec3, Vec3, Vec2, [f32; 4])> = shards
    .iter()
    .flat_map(|&(shard, crack)| {
      let broken = crack < progress * 0.8;
      let (shade, pop) = match broken {
        true => (0.15 + crack * 0.6, 0.02 + progress * 0.05 * (1.0 - crack)),
        false => (1.0, 0.0)
      };
      shard.map(|Corner { position, normal, uv, color: [r, g, b, a] }| {
        (contort(position, progress, now) + shake + normal * pop, normal, uv, [
          r * shade,
          g * shade,
          b * shade,
          a
        ])
      })
    })
    .collect();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      corners.iter().map(|(position, ..)| position.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_NORMAL,
      corners.iter().map(|(_, normal, ..)| normal.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_0,
      corners.iter().map(|(.., uv, _)| uv.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_COLOR,
      corners.iter().map(|&(.., color)| color).collect::<Vec<_>>()
    )
    .with_inserted_indices(Indices::U32((0..corners.len() as u32).collect()))
}

fn prepare(
  mut commands: Commands,
  palette: Res<Palette>,
  mut meshes: ResMut<Assets<Mesh>>
) {
  let mesh = meshes.add(Cuboid::default());
  commands.spawn((
    Crumbling { mesh: mesh.clone(), shape: None },
    Mesh3d(mesh),
    MeshMaterial3d(palette.solid.clone()),
    NotShadowCaster,
    Transform::default(),
    Visibility::Hidden
  ));
}

fn crumble(
  time: Res<Time>,
  aim: Res<Aim>,
  voxels: Option<Res<Voxels>>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut shells: Query<(&mut Crumbling, &mut Transform, &mut Visibility)>,
  mut concealed: ResMut<Concealed>,
  mut finishing: Local<(f32, f32)>
) {
  let (last, grace) = *finishing;
  let grace = match aim.digging.is_none() && last > 0.0 {
    true if last + time.delta_secs() * 3.0 >= 0.9 => FINISHING,
    _ => (grace - time.delta_secs()).max(0.0)
  };
  *finishing = (aim.progress, grace);
  let voxels = voxels.as_deref();
  let digging =
    aim.digging.zip(voxels).filter(|_| aim.progress > 0.0).and_then(|(at, voxels)| {
      voxels
        .block(at)
        .filter(|block| !block.fluid() && !block.sign())
        .map(|block| (at, block, voxels))
    });
  let hidden = digging.map(|(at, ..)| at).or(concealed.0.filter(|&cell| {
    grace > 0.0 || voxels.is_some_and(|voxels| voxels.block(cell) == Some(Block::Air))
  }));
  concealed.set_if_neq(Concealed(hidden));
  for (mut shell, mut transform, mut visibility) in shells.iter_mut() {
    match digging {
      Some((at, block, voxels)) => {
        if shell.shape.as_ref().is_none_or(|(was, made, _)| *was != at || *made != block)
        {
          let torches = voxels.torches_near(chunk_of(at));
          shell.shape =
            Some((at, block, shatter(lone(block, at, voxels.seed, &torches), at)))
        }
        if let Some((.., shards)) = &shell.shape
          && let Some(mut mesh) = meshes.get_mut(&shell.mesh)
        {
          *mesh = crumbled(shards, aim.progress, time.elapsed_secs())
        }
        transform.translation = at.as_vec3();
        visibility.set_if_neq(Visibility::Inherited);
      }
      None => {
        visibility.set_if_neq(Visibility::Hidden);
      }
    }
  }
}

pub struct Crumbles;

impl Plugin for Crumbles {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, prepare.after(crate::stream::paint).run_if(plays))
      .add_systems(Update, crumble.run_if(plays).run_if(resource_exists::<Pilot>));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn logs_crumble_as_octagons() {
    let at = IVec3::new(5, 80, 9);
    let triangles = lone(Block::Log, at, 1, &[]);
    let corners = || triangles.iter().flat_map(|triangle| triangle.iter());
    assert!(corners().any(|corner| {
      let Vec3 { x, z, .. } = corner.position;
      x > 0.01 && x < 0.99 && (z < 0.01 || z > 0.99)
    }));
    assert!(
      !corners().any(|corner| corner.position.x < 0.01 && corner.position.z < 0.01)
    );
    assert!(!shatter(triangles.clone(), at).is_empty())
  }
}
