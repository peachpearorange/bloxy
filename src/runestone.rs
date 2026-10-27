use {crate::{cage::Cage, generate::Waystone, noise::unit, player::Pilot,
             protocol::Role, sculpt::triangles, voxels::Voxels},
     bevy::{asset::{RenderAssetUsages, embedded_asset},
            light::NotShadowCaster,
            mesh::{Indices, PrimitiveTopology},
            pbr::{ExtendedMaterial, MaterialExtension},
            platform::collections::HashMap,
            prelude::*,
            render::render_resource::AsBindGroup,
            shader::ShaderRef}};

const NEAR: f32 = 256.0;
const HOVER: f32 = 2.45;
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

fn hewn(point: Vec3) -> Vec3 {
  let roll = |salt: i32| {
    unit(
      0x57,
      (point.x * 97.0) as i32,
      (point.y * 97.0) as i32 + salt,
      (point.z * 97.0) as i32
    ) - 0.5
  };
  point + Vec3::new(roll(0), roll(1) * 0.5, roll(2)) * 0.018
}

fn slab(width: f32, height: f32, floor: f32) -> Cage {
  Cage::cuboid(Vec3::new(width, height, width))
    .shaped(|point| point + Vec3::Y * (floor + height / 2.0))
}

fn pillar() -> Vec<(Cage, Finish)> {
  let square =
    |at: f32, half: f32| (Vec3::Y * at, Vec2::splat(half * std::f32::consts::SQRT_2));
  vec![
    (slab(0.94, 0.16, 0.0), Finish::Hewn),
    (slab(0.78, 0.13, 0.16), Finish::Hewn),
    (slab(0.6, 0.08, 0.29), Finish::Hewn),
    (Cage::tube(&[square(0.37, 0.2), square(1.74, 0.16)], 4), Finish::Carved),
    (slab(0.5, 0.08, 1.74), Finish::Hewn),
    (Cage::tube(&[square(1.82, 0.19), square(2.1, 0.0)], 4), Finish::Capped),
  ]
}

fn faceted(pieces: &[(Cage, Finish)]) -> Mesh {
  let corners: Vec<([Vec3; 3], Finish)> = pieces
    .iter()
    .flat_map(|(cage, finish)| {
      triangles(cage).into_iter().map(|triangle| {
        (triangle.map(|corner| hewn(cage.points[corner as usize])), *finish)
      })
    })
    .collect();
  let normal = |[a, b, c]: [Vec3; 3]| (b - a).cross(c - a).normalize_or(Vec3::Y);
  let code = |finish: Finish, normal: Vec3| match finish {
    Finish::Carved if normal.y.abs() < 0.5 => 1.0,
    Finish::Capped => 2.0,
    _ => 0.0
  };
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      corners
        .iter()
        .flat_map(|(points, _)| points.map(|point| point.to_array()))
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_NORMAL,
      corners
        .iter()
        .flat_map(|&(points, _)| [normal(points).to_array(); 3])
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_0,
      corners
        .iter()
        .flat_map(|&(points, finish)| {
          points.map(|point| [point.y, code(finish, normal(points))])
        })
        .collect::<Vec<_>>()
    )
    .with_inserted_indices(Indices::U32((0..corners.len() as u32 * 3).collect()))
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
    stone: meshes.add(faceted(&pillar())),
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
