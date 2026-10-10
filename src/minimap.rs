use {crate::{block::{Block, Fluid},
             island::Island,
             menu::INK,
             player::{Me, Pilot},
             protocol::{Avatar, Visited, plays},
             texture::{self, PIXELS},
             voxels::{Chunk, LAYERS, SIZE, Voxels}},
     bevy::{asset::RenderAssetUsages,
            image::ImageSampler,
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}}};

const SPAN: i32 = 128;
const ROWS_PER_FRAME: i32 = 16;
const REDRAW_EVERY: f32 = 0.2;
const SHOWN: f32 = 192.0;
const UNKNOWN: [u8; 4] = [16, 18, 22, 255];
const RING: [u8; 4] = [12, 12, 14, 235];

#[derive(Resource)]
struct Minimap {
  image: Handle<Image>,
  hues: Vec<[f32; 3]>,
  centre: IVec2,
  colours: Vec<[u8; 4]>,
  heights: Vec<i32>,
  row: i32,
  since: f32
}

#[derive(Component)]
struct Whereabouts;

pub fn hue(block: Block) -> [f32; 3] {
  match block.liquid() {
    Some((Fluid::Water, _)) => [0.12, 0.42, 0.62],
    _ => painted(block)
  }
}

fn painted(block: Block) -> [f32; 3] {
  let tile = block.tiles()[0];
  let (sum, count) = (0..PIXELS * PIXELS)
    .map(|index| texture::paint(tile, index % PIXELS, index / PIXELS).color)
    .filter(|[_, _, _, alpha]| *alpha > 0.5)
    .fold(([0.0; 3], 0.0), |(sum, count), [r, g, b, _]| {
      ([sum[0] + r, sum[1] + g, sum[2] + b], count + 1.0)
    });
  sum.map(|channel| channel / f32::max(count, 1.0))
}

fn build(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
  let mut image = Image::new_fill(
    Extent3d { width: SPAN as u32, height: SPAN as u32, depth_or_array_layers: 1 },
    TextureDimension::D2,
    &UNKNOWN,
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  );
  image.sampler = ImageSampler::nearest();
  let image = images.add(image);
  commands.insert_resource(Minimap {
    image: image.clone(),
    hues: Block::ALL.map(hue).to_vec(),
    centre: IVec2::ZERO,
    colours: vec![UNKNOWN; (SPAN * SPAN) as usize],
    heights: vec![0; (SPAN * SPAN) as usize],
    row: 0,
    since: 0.0
  });
  commands
    .spawn(Node {
      position_type: PositionType::Absolute,
      right: px(10),
      top: px(10),
      flex_direction: FlexDirection::Column,
      align_items: AlignItems::Center,
      row_gap: px(4),
      ..default()
    })
    .with_children(|corner| {
      corner.spawn((ImageNode::new(image), Node {
        width: px(SHOWN),
        height: px(SHOWN),
        ..default()
      }));
      corner.spawn((
        Whereabouts,
        Text::new(""),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(INK),
        TextLayout { justify: Justify::Center, ..default() },
        crate::hud::SHADE
      ));
    });
}

fn surface(voxels: &Voxels, x: i32, z: i32) -> Option<(i32, Block, i32)> {
  let column = |layer: i32| {
    voxels.chunks.get(&IVec3::new(x.div_euclid(SIZE), layer, z.div_euclid(SIZE)))
  };
  let local = IVec2::new(x.rem_euclid(SIZE), z.rem_euclid(SIZE));
  let seen =
    |block: Block| block != Block::Air && !block.modelled() && block != Block::Boat;
  (0..LAYERS)
    .rev()
    .map(|layer| (layer, column(layer)))
    .take_while(|(_, chunk)| chunk.is_some())
    .find_map(|(layer, chunk)| match chunk.map(|chunk| &**chunk) {
      Some(Chunk::Uniform(block)) if !seen(*block) => None,
      Some(chunk) => (0..SIZE).rev().find_map(|y| {
        let block = chunk.get(IVec3::new(local.x, y, local.y));
        seen(block).then_some((layer * SIZE + y, block))
      }),
      None => None
    })
    .map(|(height, block)| {
      let depth = match block.fluid() {
        true => (1..12)
          .take_while(|down| {
            voxels.block(IVec3::new(x, height - down, z)).is_some_and(Block::fluid)
          })
          .count() as i32,
        false => 0
      };
      (height, block, depth)
    })
}

fn moved<T: Copy>(buffer: &[T], shift: IVec2, blank: T) -> Vec<T> {
  (0..SPAN * SPAN)
    .map(|index| {
      let from = IVec2::new(index % SPAN, index / SPAN) + shift;
      match from.cmpge(IVec2::ZERO).all() && from.cmplt(IVec2::splat(SPAN)).all() {
        true => buffer[(from.y * SPAN + from.x) as usize],
        false => blank
      }
    })
    .collect()
}

fn survey(
  time: Res<Time>,
  pilot: Option<Res<Pilot>>,
  voxels: Option<Res<Voxels>>,
  mut map: ResMut<Minimap>
) {
  if let Some(pilot) = pilot
    && let Some(voxels) = voxels
  {
    let centre = pilot.at.xz().floor().as_ivec2();
    if centre != map.centre {
      let shift = centre - map.centre;
      map.colours = moved(&map.colours, shift, UNKNOWN);
      map.heights = moved(&map.heights, shift, 0);
      map.centre = centre
    }
    let corner = centre - SPAN / 2;
    let rows = map.row..map.row + ROWS_PER_FRAME;
    rows.clone().for_each(|row| {
      (0..SPAN).for_each(|column| {
        let index = (row * SPAN + column) as usize;
        let (x, z) = (corner.x + column, corner.y + row);
        match surface(&voxels, x, z) {
          Some((height, block, depth)) => {
            let north = match row {
              0 => height,
              _ => map.heights[index - SPAN as usize]
            };
            let light = 1.0 + ((height - north) as f32 * 0.12).clamp(-0.3, 0.3);
            let [r, g, b] = map.hues[block as usize];
            let deep = (depth as f32 / 10.0).min(1.0) * 0.7;
            let colour =
              match block.liquid().is_some_and(|(fluid, _)| fluid == Fluid::Water) {
                true => [
                  r * (1.0 - deep) + 0.02 * deep,
                  g * (1.0 - deep) + 0.08 * deep,
                  b * (1.0 - deep) + 0.25 * deep
                ],
                false => [r * light, g * light, b * light]
              };
            map.heights[index] = height;
            map.colours[index] = [
              (colour[0].clamp(0.0, 1.0) * 255.0) as u8,
              (colour[1].clamp(0.0, 1.0) * 255.0) as u8,
              (colour[2].clamp(0.0, 1.0) * 255.0) as u8,
              255
            ];
          }
          None => map.colours[index] = UNKNOWN
        }
      })
    });
    map.row = rows.end % SPAN;
    map.since += time.delta_secs();
  }
}

fn dot(pixels: &mut [[u8; 4]], at: IVec2, radius: i32, colour: [u8; 4]) {
  (-radius..=radius).for_each(|dz| {
    (-radius..=radius).for_each(|dx| {
      let spot = at + IVec2::new(dx, dz);
      if spot.cmpge(IVec2::ZERO).all() && spot.cmplt(IVec2::splat(SPAN)).all() {
        pixels[(spot.y * SPAN + spot.x) as usize] = colour
      }
    })
  })
}

fn draw(
  pilot: Option<Res<Pilot>>,
  voxels: Option<Res<Voxels>>,
  others: Query<&Avatar, Without<Me>>,
  visits: Query<&Visited>,
  mut map: ResMut<Minimap>,
  mut images: ResMut<Assets<Image>>,
  mut texts: Query<&mut Text, With<Whereabouts>>
) {
  if let Some(pilot) = pilot
    && let Some(voxels) = voxels
    && map.since >= REDRAW_EVERY
  {
    map.since = 0.0;
    let corner = map.centre - SPAN / 2;
    let on_map = |at: Vec3| at.xz().floor().as_ivec2() - corner;
    let mut pixels = map.colours.clone();
    visits.get(pilot.me).into_iter().flat_map(|visited| visited.0.iter()).for_each(
      |&cell| {
        if let Some(island) = Island::at(voxels.seed, cell) {
          dot(&mut pixels, island.stone.xz() - corner, 1, [230, 60, 230, 255])
        }
      }
    );
    others
      .iter()
      .for_each(|avatar| dot(&mut pixels, on_map(avatar.at), 1, [250, 250, 250, 255]));
    let facing = (pilot.facing() * Vec3::NEG_Z).xz().normalize_or(Vec2::NEG_Y);
    let across = facing.perp();
    let centre = Vec2::splat(SPAN as f32 / 2.0);
    let (tip, left, right) = (
      centre + facing * 5.0,
      centre - facing * 3.0 + across * 3.5,
      centre - facing * 3.0 - across * 3.5
    );
    let side = |a: Vec2, b: Vec2, point: Vec2| (b - a).perp_dot(point - a);
    (-6..=6).for_each(|dz| {
      (-6..=6).for_each(|dx| {
        let point = centre + Vec2::new(dx as f32, dz as f32) + 0.5;
        let signs =
          [side(tip, left, point), side(left, right, point), side(right, tip, point)];
        if signs.iter().all(|&sign| sign >= 0.0) || signs.iter().all(|&sign| sign <= 0.0)
        {
          let spot = point.floor().as_ivec2();
          pixels[(spot.y * SPAN + spot.x) as usize] = [255, 70, 50, 255]
        }
      })
    });
    let middle = Vec2::splat(SPAN as f32 / 2.0);
    pixels.iter_mut().enumerate().for_each(|(index, pixel)| {
      let at =
        Vec2::new((index as i32 % SPAN) as f32, (index as i32 / SPAN) as f32) + 0.5;
      let away = at.distance(middle);
      *pixel = match away {
        away if away > middle.x => [0, 0, 0, 0],
        away if away > middle.x - 2.0 => RING,
        _ => *pixel
      }
    });
    if let Some(mut image) = images.get_mut(&map.image) {
      image.data = Some(pixels.concat())
    }
    let at = pilot.at.floor().as_ivec3();
    let place = Island::near(voxels.seed, pilot.at)
      .into_iter()
      .find(|island| {
        island.rise(voxels.seed, at.x, at.z).is_some_and(|rise| rise.inward > 0.0)
      })
      .map_or("Open sea".to_string(), |island| island.name(voxels.seed));
    let line = format!("{} {} {}\n{place}", at.x, at.y, at.z);
    texts.iter_mut().for_each(|mut text| {
      if text.0 != line {
        text.0 = line.clone()
      }
    })
  }
}

pub struct Minimaps;

impl Plugin for Minimaps {
  fn build(&self, app: &mut App) {
    app
      .add_systems(Startup, build.run_if(plays))
      .add_systems(Update, (survey, draw).chain().run_if(resource_exists::<Minimap>));
  }
}
