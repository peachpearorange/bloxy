use {crate::{block::{Block, Look},
             generate,
             noise::unit,
             player::{Aim, Pilot},
             protocol::plays,
             stream::Palette,
             texture::uv_corner,
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            light::NotShadowCaster,
            mesh::{Indices, PrimitiveTopology},
            prelude::*}};

const GRID: u32 = 8;
const SKY_FALLOFF: f32 = 7.0;
const CAVE_DARK: f32 = 0.05;

#[derive(Component)]
struct Crumbling(Handle<Mesh>);

struct Side {
  normal: Vec3,
  across: Vec3,
  up: Vec3,
  slot: usize
}

const SIDES: [Side; 6] = [
  Side { normal: Vec3::Y, across: Vec3::X, up: Vec3::NEG_Z, slot: 0 },
  Side { normal: Vec3::NEG_Y, across: Vec3::X, up: Vec3::Z, slot: 2 },
  Side { normal: Vec3::X, across: Vec3::NEG_Z, up: Vec3::Y, slot: 1 },
  Side { normal: Vec3::NEG_X, across: Vec3::Z, up: Vec3::Y, slot: 1 },
  Side { normal: Vec3::Z, across: Vec3::X, up: Vec3::Y, slot: 1 },
  Side { normal: Vec3::NEG_Z, across: Vec3::NEG_X, up: Vec3::Y, slot: 1 }
];

fn bulge(point: Vec3, progress: f32, now: f32) -> Vec3 {
  let wave = (point.x * 7.1 + now * 9.0).sin()
    * (point.y * 6.3 - now * 7.0).sin()
    * (point.z * 8.7 + now * 11.0).sin();
  let swell = 0.012 + progress * (0.05 + 0.04 * wave);
  let centre = Vec3::splat(0.5);
  centre + (point - centre) * (1.0 + swell * 2.0)
}

fn crumbled(block: Block, at: IVec3, light: f32, progress: f32, now: f32) -> Mesh {
  let tiles = block.tiles();
  let shake = Vec3::new((now * 53.0).sin(), (now * 61.0).sin(), (now * 47.0).sin())
    * 0.008
    * progress;
  let quads: Vec<([Vec3; 4], [Vec2; 4], Vec3, f32)> = SIDES
    .iter()
    .enumerate()
    .flat_map(|(side_index, side)| {
      (0..GRID * GRID).map(move |cell| {
        let (i, j) = ((cell % GRID) as f32, (cell / GRID) as f32);
        let corner = |a: f32, b: f32| {
          let (u, v) = ((i + a) / GRID as f32, (j + b) / GRID as f32);
          let point = Vec3::splat(0.5)
            + side.normal * 0.5
            + side.across * (u - 0.5)
            + side.up * (v - 0.5);
          (point, uv_corner(tiles[side.slot], Vec2::new(u, 1.0 - v)))
        };
        let corners =
          [corner(0.0, 0.0), corner(1.0, 0.0), corner(1.0, 1.0), corner(0.0, 1.0)];
        let crack =
          unit(at.x as u32 ^ (side_index as u32) << 20, at.y, at.z, cell as i32);
        let shade = match crack < progress * 0.55 {
          true => 0.3 + crack,
          false => 1.0
        };
        (
          corners.map(|(point, _)| bulge(point, progress, now) + shake),
          corners.map(|(_, uv)| uv),
          side.normal,
          shade * light
        )
      })
    })
    .collect();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      quads
        .iter()
        .flat_map(|(points, ..)| points.map(|point| point.to_array()))
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_NORMAL,
      quads
        .iter()
        .flat_map(|&(_, _, normal, _)| [normal.to_array(); 4])
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_0,
      quads
        .iter()
        .flat_map(|(_, uvs, ..)| uvs.map(|uv| uv.to_array()))
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_COLOR,
      quads
        .iter()
        .flat_map(|&(.., shade)| [[shade, shade, shade, 1.0]; 4])
        .collect::<Vec<_>>()
    )
    .with_inserted_indices(Indices::U32(
      (0..quads.len() as u32)
        .flat_map(|quad| [0, 1, 2, 0, 2, 3].map(|corner| quad * 4 + corner))
        .collect()
    ))
}

fn prepare(
  mut commands: Commands,
  palette: Res<Palette>,
  mut meshes: ResMut<Assets<Mesh>>
) {
  let mesh = meshes.add(crumbled(Block::Stone, IVec3::ZERO, 1.0, 0.0, 0.0));
  commands.spawn((
    Crumbling(mesh.clone()),
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
  mut shells: Query<(&Crumbling, &mut Transform, &mut Visibility)>
) {
  let digging =
    aim.digging.zip(voxels).filter(|_| aim.progress > 0.0).and_then(|(at, voxels)| {
      voxels
        .block(at)
        .filter(|block| matches!(block.look(), Look::Opaque | Look::Cutout | Look::Log))
        .map(|block| (at, block, voxels.seed))
    });
  shells.iter_mut().for_each(|(shell, mut transform, mut visibility)| match digging {
    Some((at, block, seed)) => {
      let depth = generate::height(seed, at.x, at.z) - at.y;
      let light = (1.0 - depth as f32 / SKY_FALLOFF).clamp(CAVE_DARK, 1.0);
      if let Some(mut mesh) = meshes.get_mut(&shell.0) {
        *mesh = crumbled(block, at, light, aim.progress, time.elapsed_secs())
      }
      transform.translation = at.as_vec3();
      visibility.set_if_neq(Visibility::Inherited);
    }
    None => {
      visibility.set_if_neq(Visibility::Hidden);
    }
  })
}

pub struct Crumbles;

impl Plugin for Crumbles {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, prepare.after(crate::stream::paint).run_if(plays))
      .add_systems(Update, crumble.run_if(plays).run_if(resource_exists::<Pilot>));
  }
}
