use {crate::{cage::Cage,
             generate::Waystone,
             noise::{perlin3, unit},
             player::Pilot,
             protocol::Role,
             sculpt::triangles,
             voxels::Voxels},
     bevy::{asset::{RenderAssetUsages, embedded_asset},
            light::NotShadowCaster,
            mesh::{Indices, PrimitiveTopology},
            pbr::{ExtendedMaterial, MaterialExtension},
            platform::collections::HashMap,
            prelude::*,
            render::render_resource::AsBindGroup,
            shader::ShaderRef}};

const NEAR: f32 = 256.0;
const HOVER: f32 = 1.05;
const POST: f32 = 0.34;
const LEAN: f32 = 0.025;
const RUNE_GLOW: Vec4 = Vec4::new(0.35, 0.95, 1.0, 0.0);

#[derive(Asset, AsBindGroup, TypePath, Clone, Copy)]
pub struct Carving {
  #[uniform(100)]
  glow: Vec4
}

impl MaterialExtension for Carving {
  fn fragment_shader() -> ShaderRef { "embedded://bloxy/runestone.wgsl".into() }
}

type Runestone = ExtendedMaterial<StandardMaterial, Carving>;

#[derive(Clone, Copy, PartialEq)]
enum Finish {
  Hewn,
  Carved,
  Capped
}

fn weathered(cage: Cage, seed: u32) -> Cage {
  cage.displaced(|point, _| {
    perlin3(seed, point.x * 2.5, point.y * 2.5, point.z * 2.5) * 0.028
      + perlin3(seed + 1, point.x * 7.0, point.y * 7.0, point.z * 7.0) * 0.01
  })
}

fn smoothed(cage: Cage, bevel: f32, crisp: f32) -> Cage {
  cage.beveled(bevel).sharpened(40.0, crisp).subdivided(3)
}

fn leaning(cage: Cage, side: f32) -> Cage {
  cage.shaped(|point| point - Vec3::X * side * LEAN * point.y)
}

fn gate() -> Vec<(Cage, Finish)> {
  let post = |side: f32| {
    let ring = |at: f32, half: Vec2| {
      (Vec3::new(side * POST, at, 0.0), half * std::f32::consts::SQRT_2)
    };
    leaning(
      Cage::tube(
        &[
          ring(0.16, Vec2::new(0.13, 0.15)),
          ring(0.7, Vec2::new(0.12, 0.14)),
          ring(1.25, Vec2::new(0.11, 0.135)),
          ring(1.74, Vec2::new(0.105, 0.13))
        ],
        4
      ),
      side
    )
  };
  let footing =
    Cage::rounded(Vec3::Y * 0.08, Vec3::new(1.0, 0.18, 0.62), UVec3::new(4, 1, 2), 0.3);
  let lintel = Cage::block(Vec3::new(1.04, 0.2, 0.36), UVec3::new(4, 1, 1))
    .shaped(|point| point + Vec3::Y * (1.84 + (point.x * 2.2).powi(2) * 0.035));
  let keystone = Cage::cuboid(Vec3::new(0.16, 0.24, 0.4)).shaped(|point| {
    Vec3::new(point.x * (1.0 - point.y * 1.2), point.y, point.z) + Vec3::Y * 1.82
  });
  vec![
    (weathered(smoothed(footing, 0.04, 0.5), 3), Finish::Hewn),
    (weathered(smoothed(post(-1.0), 0.025, 1.0), 5), Finish::Carved),
    (weathered(smoothed(post(1.0), 0.025, 1.0), 6), Finish::Carved),
    (weathered(smoothed(lintel, 0.03, 1.0), 7), Finish::Hewn),
    (smoothed(keystone, 0.015, 2.0), Finish::Capped),
  ]
}

fn smooth(pieces: &[(Cage, Finish)]) -> Mesh {
  let code = |finish: Finish, normal: Vec3| match finish {
    Finish::Carved if normal.y.abs() < 0.5 => 1.0,
    Finish::Capped => 2.0,
    _ => 0.0
  };
  let (positions, normals, uvs, across, indices) = pieces.iter().fold(
    (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()),
    |(mut positions, mut normals, mut uvs, mut across, mut indices): (
      Vec<[f32; 3]>,
      Vec<[f32; 3]>,
      Vec<[f32; 2]>,
      Vec<[f32; 2]>,
      Vec<u32>
    ),
     (cage, finish)| {
      let first = positions.len() as u32;
      for (point, normal) in cage.points.iter().zip(cage.vertex_normals()) {
        positions.push(point.to_array());
        normals.push(normal.to_array());
        uvs.push([point.y, code(*finish, normal)]);
        across.push([point.x, point.z])
      }
      indices.extend(triangles(cage).into_iter().flatten().map(|corner| first + corner));
      (positions, normals, uvs, across, indices)
    }
  );
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, across)
    .with_inserted_indices(Indices::U32(indices))
}

fn crystal() -> Mesh {
  let (wide, tall) = (0.1, 0.17);
  let ring = [Vec3::X, Vec3::Z, Vec3::NEG_X, Vec3::NEG_Z].map(|side| side * wide);
  let faces: Vec<[Vec3; 3]> = (0..4)
    .flat_map(|side| {
      let (from, to) = (ring[side], ring[(side + 1) % 4]);
      [[Vec3::Y * tall, to, from], [Vec3::NEG_Y * tall, from, to]]
    })
    .collect();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      faces
        .iter()
        .flat_map(|face| face.map(|point| point.to_array()))
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_NORMAL,
      faces
        .iter()
        .flat_map(|&[a, b, c]| [(b - a).cross(c - a).normalize().to_array(); 3])
        .collect::<Vec<_>>()
    )
    .with_inserted_indices(Indices::U32((0..faces.len() as u32 * 3).collect()))
}

#[derive(Resource)]
struct Masonry {
  stone: Handle<Mesh>,
  material: Handle<Runestone>,
  crystal: Handle<Mesh>,
  shard: Handle<StandardMaterial>
}

#[derive(Resource, Default)]
struct Raised(HashMap<IVec2, Entity>);

#[derive(Component)]
struct Hover(f32);

fn quarry(
  mut commands: Commands,
  mut meshes: ResMut<Assets<Mesh>>,
  mut runestones: ResMut<Assets<Runestone>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  commands.insert_resource(Masonry {
    stone: meshes.add(smooth(&gate())),
    material: runestones.add(ExtendedMaterial {
      base: StandardMaterial { perceptual_roughness: 0.85, ..default() },
      extension: Carving { glow: RUNE_GLOW }
    }),
    crystal: meshes.add(crystal()),
    shard: materials.add(StandardMaterial {
      base_color: Color::srgb(0.2, 0.6, 0.75),
      emissive: LinearRgba::from_vec3(RUNE_GLOW.truncate()) * 1.2,
      perceptual_roughness: 0.2,
      ..default()
    })
  })
}

fn raise(
  pilot: Res<Pilot>,
  voxels: Option<Res<Voxels>>,
  masonry: Res<Masonry>,
  mut raised: ResMut<Raised>,
  mut commands: Commands
) {
  if let Some(voxels) = voxels {
    let around = pilot.at.xz().floor().as_ivec2();
    let reach = IVec2::splat(NEAR as i32);
    let near: Vec<Waystone> =
      Waystone::within(voxels.seed, around - reach, around + reach)
        .into_iter()
        .filter(|stone| stone.at.xz().as_vec2().distance(pilot.at.xz()) < NEAR)
        .collect();
    raised.0.retain(|cell, entity| {
      let kept = near.iter().any(|stone| stone.cell == *cell);
      if !kept {
        commands.entity(*entity).despawn()
      }
      kept
    });
    for stone in near.iter() {
      raised.0.entry(stone.cell).or_insert_with(|| {
        let phase = unit(0x5703, stone.cell.x, 0, stone.cell.y) * std::f32::consts::TAU;
        commands
          .spawn((
            Mesh3d(masonry.stone.clone()),
            MeshMaterial3d(masonry.material.clone()),
            Transform::from_translation(stone.at.as_vec3() + Vec3::new(0.5, 0.0, 0.5)),
            Visibility::default()
          ))
          .with_child((
            Hover(phase),
            Mesh3d(masonry.crystal.clone()),
            MeshMaterial3d(masonry.shard.clone()),
            NotShadowCaster,
            Transform::from_xyz(0.0, HOVER, 0.0)
          ))
          .id()
      });
    }
  }
}

fn hover(time: Res<Time>, mut crystals: Query<(&Hover, &mut Transform)>) {
  let now = time.elapsed_secs();
  for (&Hover(phase), mut transform) in crystals.iter_mut() {
    *transform = Transform::from_xyz(0.0, HOVER + (now * 1.3 + phase).sin() * 0.06, 0.0)
      .with_rotation(Quat::from_rotation_y(now * 0.8 + phase))
  }
}

pub struct Runestones;

impl Plugin for Runestones {
  fn build(&self, app: &mut App) {
    if app.world().resource::<Role>().plays() {
      embedded_asset!(app, "runestone.wgsl");
      app
        .add_plugins(MaterialPlugin::<Runestone>::default())
        .init_resource::<Raised>()
        .add_systems(Startup, quarry)
        .add_systems(Update, (raise.run_if(resource_exists::<Pilot>), hover).chain());
    }
  }
}
