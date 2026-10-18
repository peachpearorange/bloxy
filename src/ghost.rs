use {crate::{block::Block,
             menu::Menu,
             mesh::lone,
             player::{Aim, Pilot, Placement, Selected, placement},
             protocol::{Inventory, plays},
             stream::Palette,
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            light::{NotShadowCaster, NotShadowReceiver},
            mesh::{Indices, PrimitiveTopology},
            platform::collections::HashMap,
            prelude::*}};

const SHRUNK: f32 = 0.98;
const FITS: Color = Color::srgba(0.78, 0.94, 1.0, 0.42);
const BLOCKED: Color = Color::srgba(1.0, 0.32, 0.28, 0.42);
const PULSE: f32 = 0.12;

#[derive(Component)]
struct Ghost {
  head: bool
}

#[derive(Resource)]
struct Phantoms {
  fits: Handle<StandardMaterial>,
  blocked: Handle<StandardMaterial>,
  shapes: HashMap<Block, Handle<Mesh>>
}

fn shape(block: Block, seed: u32) -> Mesh {
  let triangles = lone(block, IVec3::ZERO, seed, &[]);
  let corners = triangles.iter().flatten();
  let middle = Vec3::splat(0.5);
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      corners
        .clone()
        .map(|corner| ((corner.position - middle) * SHRUNK + middle).to_array())
        .collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_NORMAL,
      corners.clone().map(|corner| corner.normal.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_0,
      corners.map(|corner| corner.uv.to_array()).collect::<Vec<_>>()
    )
    .with_inserted_indices(Indices::U32((0..triangles.len() as u32 * 3).collect()))
}

fn summon(
  mut commands: Commands,
  palette: Res<Palette>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let atlas = materials.get(&palette.solid).and_then(|solid| solid.base_color_texture.clone());
  let mut tinted = |color: Color| {
    materials.add(StandardMaterial {
      base_color: color,
      base_color_texture: atlas.clone(),
      alpha_mode: AlphaMode::Blend,
      unlit: true,
      ..default()
    })
  };
  commands.insert_resource(Phantoms {
    fits: tinted(FITS),
    blocked: tinted(BLOCKED),
    shapes: HashMap::default()
  });
  for head in [false, true] {
    commands.spawn((
      Ghost { head },
      Mesh3d::default(),
      MeshMaterial3d::<StandardMaterial>::default(),
      Transform::default(),
      Visibility::Hidden,
      NotShadowCaster,
      NotShadowReceiver
    ));
  }
}

fn haunt(
  time: Res<Time>,
  pilot: Res<Pilot>,
  aim: Res<Aim>,
  selected: Res<Selected>,
  voxels: Option<Res<Voxels>>,
  inventories: Query<&Inventory>,
  menu: Option<Res<Menu>>,
  mut phantoms: ResMut<Phantoms>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut ghosts: Query<(
    &Ghost,
    &mut Mesh3d,
    &mut MeshMaterial3d<StandardMaterial>,
    &mut Transform,
    &mut Visibility
  )>
) {
  let item = inventories
    .get(pilot.me)
    .ok()
    .and_then(|inventory| inventory.slots[selected.0])
    .map(|stack| stack.block)
    .filter(|item| item.placeable());
  let shown = voxels.as_deref().and_then(|voxels| {
    aim
      .hit
      .as_ref()
      .filter(|hit| {
        !(hit.block.waystone()
          || hit.block.station()
          || hit.block.sign()
          || hit.block.bed()
          || matches!(hit.block, Block::Chest | Block::EmptyChest))
      })
      .zip(item)
      .filter(|_| menu.as_ref().is_some_and(|menu| menu.idle()))
      .map(|(hit, item)| (placement(voxels, &pilot, hit, item), voxels.seed))
  });
  let glow = 1.0 + PULSE * (time.elapsed_secs() * 4.0).sin();
  for handle in [phantoms.fits.clone(), phantoms.blocked.clone()] {
    if let Some(mut material) = materials.get_mut(&handle) {
      let base = if handle == phantoms.fits { FITS } else { BLOCKED };
      material.base_color = base.with_alpha(base.alpha() * glow)
    }
  }
  for (&Ghost { head }, mut mesh, mut material, mut transform, mut visibility) in
    ghosts.iter_mut()
  {
    let cell = shown.as_ref().and_then(|(Placement { at, block, head: other, .. }, _)| {
      match head {
        true => *other,
        false => Some((*at, *block))
      }
    });
    match cell.zip(shown.as_ref()) {
      Some(((at, block), (Placement { fits, .. }, seed))) => {
        let shape = phantoms
          .shapes
          .entry(block)
          .or_insert_with(|| meshes.add(shape(block, *seed)))
          .clone();
        if mesh.0 != shape {
          mesh.0 = shape
        }
        let tint = if *fits { phantoms.fits.clone() } else { phantoms.blocked.clone() };
        if material.0 != tint {
          material.0 = tint
        }
        transform.translation = at.as_vec3();
        *visibility = Visibility::Visible
      }
      None => *visibility = Visibility::Hidden
    }
  }
}

pub struct Ghosts;

impl Plugin for Ghosts {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, summon.after(crate::stream::paint).run_if(plays))
      .add_systems(Update, haunt.run_if(plays).run_if(resource_exists::<Pilot>));
  }
}
