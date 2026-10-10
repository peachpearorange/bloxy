use {crate::{block::{Block, Fluid},
             claim::{Claim, MOST, Stake},
             generate,
             island::Island,
             menu::{INK, Menu, Tab, words},
             minimap::hue,
             player::{Me, Pilot},
             protocol::{Avatar, Player, Visited, plays},
             voxels::{SIZE, Voxels}},
     bevy::{asset::RenderAssetUsages,
            image::ImageSampler,
            input::mouse::AccumulatedMouseScroll,
            platform::{collections::HashMap, time::Instant},
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
            ui::RelativeCursorPosition}};

const TILE: i32 = 64;
const KEPT_TILES: usize = 900;
const SCALES: [f32; 8] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
const BUDGET: f32 = 0.006;
const REDRAW_EVERY: f32 = 0.15;
const DRAG: f32 = 4.0;
const UNKNOWN: [u8; 4] = [16, 18, 22, 255];

#[derive(Resource)]
pub struct Chart {
  image: Handle<Image>,
  hues: Vec<[f32; 3]>,
  centre: Vec2,
  zoom: usize,
  size: IVec2,
  tiles: HashMap<(usize, IVec2), Vec<[u8; 4]>>,
  since: f32,
  grip: Option<(Vec2, Vec2)>,
  dragged: bool,
  opened: bool
}

impl Chart {
  fn scale(&self) -> f32 { SCALES[self.zoom] }

  fn origin(&self) -> Vec2 {
    (self.centre / self.scale() - self.size.as_vec2() / 2.0).floor()
  }

  fn world(&self, pixel: Vec2) -> Vec2 { (self.origin() + pixel) * self.scale() }

  fn pixel(&self, world: Vec2) -> Vec2 { world / self.scale() - self.origin() }

  fn wanted(&self) -> Vec<IVec2> {
    let origin = self.origin().as_ivec2();
    let (low, high) = (
      origin.div_euclid(IVec2::splat(TILE)),
      (origin + self.size).div_euclid(IVec2::splat(TILE))
    );
    let middle = (origin + self.size / 2).as_vec2() / TILE as f32;
    let mut tiles: Vec<IVec2> = (low.y..=high.y)
      .flat_map(|y| (low.x..=high.x).map(move |x| IVec2::new(x, y)))
      .collect();
    tiles.sort_by(|a, b| {
      (a.as_vec2() + 0.5)
        .distance(middle)
        .total_cmp(&(b.as_vec2() + 0.5).distance(middle))
    });
    tiles
  }
}

#[derive(Component)]
struct Canvas;

#[derive(Component)]
struct Legend;

fn canvas_image(size: IVec2) -> Image {
  let mut image = Image::new_fill(
    Extent3d {
      width: size.x.max(1) as u32,
      height: size.y.max(1) as u32,
      depth_or_array_layers: 1
    },
    TextureDimension::D2,
    &UNKNOWN,
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  );
  image.sampler = ImageSampler::nearest();
  image
}

pub fn prepare(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
  commands.insert_resource(Chart {
    image: images.add(canvas_image(IVec2::ONE)),
    hues: Block::ALL.map(hue).to_vec(),
    centre: Vec2::ZERO,
    zoom: 3,
    size: IVec2::ONE,
    tiles: HashMap::default(),
    since: REDRAW_EVERY,
    grip: None,
    dragged: false,
    opened: false
  })
}

pub fn page(page: &mut ChildSpawnerCommands, chart: &Chart) {
  page.spawn((
    Canvas,
    ImageNode::new(chart.image.clone()),
    RelativeCursorPosition::default(),
    Interaction::default(),
    Node {
      width: percent(100),
      flex_grow: 1.0,
      flex_basis: px(0),
      min_height: px(0),
      overflow: Overflow::clip(),
      ..default()
    }
  ));
  page.spawn((Legend, words("", 16.0, INK)));
}

fn colour(chart: &Chart, block: Block, top: i32, floor: i32, north: i32) -> [u8; 4] {
  let [r, g, b] = chart.hues[block as usize];
  let shade = match block.liquid() {
    Some((Fluid::Water, _)) => {
      let deep = ((top - floor) as f32 / 14.0).clamp(0.0, 1.0) * 0.7;
      [
        r * (1.0 - deep) + 0.02 * deep,
        g * (1.0 - deep) + 0.08 * deep,
        b * (1.0 - deep) + 0.25 * deep
      ]
    }
    _ => {
      let light =
        1.0 + ((top - north) as f32 * 0.1 / chart.scale().max(1.0)).clamp(-0.3, 0.3);
      [r * light, g * light, b * light]
    }
  };
  let [r, g, b] = shade.map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u8);
  [r, g, b, 255]
}

fn sample(chart: &Chart, seed: u32, tile: IVec2) -> Vec<[u8; 4]> {
  let scale = chart.scale();
  let at = |x: i32, y: i32| {
    let world = ((tile * TILE + IVec2::new(x, y)).as_vec2() + 0.5) * scale;
    generate::overview(seed, world.x.floor() as i32, world.y.floor() as i32)
  };
  let mut north: Vec<i32> = (0..TILE).map(|x| at(x, -1).1).collect();
  (0..TILE)
    .flat_map(|y| {
      let row: Vec<[u8; 4]> = (0..TILE)
        .map(|x| {
          let (block, top, floor) = at(x, y);
          let shown = colour(chart, block, top, floor, north[x as usize]);
          north[x as usize] = top;
          shown
        })
        .collect();
      row
    })
    .collect()
}

fn survey(voxels: Option<Res<Voxels>>, menu: Res<Menu>, mut chart: ResMut<Chart>) {
  if let Some(voxels) = voxels
    && menu.showing(Tab::Map)
  {
    let zoom = chart.zoom;
    if chart.tiles.len() > KEPT_TILES {
      let near: Vec<IVec2> = chart.wanted();
      chart.tiles.retain(|(at_zoom, tile), _| *at_zoom == zoom && near.contains(tile))
    }
    let started = Instant::now();
    let missing: Vec<IVec2> = chart
      .wanted()
      .into_iter()
      .filter(|&tile| !chart.tiles.contains_key(&(zoom, tile)))
      .collect();
    let made = missing
      .into_iter()
      .take_while(|_| started.elapsed().as_secs_f32() < BUDGET)
      .map(|tile| (tile, sample(&chart, voxels.seed, tile)))
      .collect::<Vec<_>>();
    if !made.is_empty() {
      chart.since = REDRAW_EVERY
    }
    for (tile, pixels) in made.into_iter() {
      chart.tiles.insert((zoom, tile), pixels);
    }
  }
}

fn fit(
  menu: Res<Menu>,
  canvas: Query<&ComputedNode, With<Canvas>>,
  mut chart: ResMut<Chart>,
  mut images: ResMut<Assets<Image>>
) {
  if menu.showing(Tab::Map)
    && let Ok(node) = canvas.single()
    && let size = node.size().as_ivec2().max(IVec2::ONE)
    && size != chart.size
  {
    chart.size = size;
    chart.since = REDRAW_EVERY;
    images.insert(&chart.image, canvas_image(size)).ok();
  }
}

fn steer(
  time: Res<Time>,
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  scroll: Res<AccumulatedMouseScroll>,
  buttons: Res<ButtonInput<MouseButton>>,
  canvas: Query<&RelativeCursorPosition, With<Canvas>>,
  mut stakes: MessageWriter<Stake>,
  mut chart: ResMut<Chart>
) {
  let showing = menu.showing(Tab::Map);
  if showing
    && !chart.opened
    && let Some(pilot) = &pilot
  {
    chart.centre = pilot.at.xz().floor()
  }
  chart.opened = showing && pilot.is_some();
  chart.since += time.delta_secs();
  if showing
    && let Ok(cursor) = canvas.single()
    && let Some(spot) = cursor.normalized
  {
    let pixel = (spot + 0.5) * chart.size.as_vec2();
    let over = cursor.cursor_over;
    let wheel = scroll.delta.y;
    if over && wheel != 0.0 {
      let anchor = chart.world(pixel);
      let zoom =
        (chart.zoom as i32 - wheel.signum() as i32).clamp(0, SCALES.len() as i32 - 1);
      chart.zoom = zoom as usize;
      chart.centre =
        (anchor - (pixel - chart.size.as_vec2() / 2.0) * chart.scale()).floor();
      chart.since = REDRAW_EVERY
    }
    if over && buttons.just_pressed(MouseButton::Left) {
      chart.grip = Some((pixel, chart.centre));
      chart.dragged = false
    }
    if let Some((from, centre)) = chart.grip
      && buttons.pressed(MouseButton::Left)
    {
      let moved = pixel - from;
      chart.dragged |= moved.length() > DRAG;
      if chart.dragged && chart.centre != (centre - moved * chart.scale()).floor() {
        chart.centre = (centre - moved * chart.scale()).floor();
        chart.since = REDRAW_EVERY
      }
    }
    if buttons.just_released(MouseButton::Left)
      && chart.grip.take().is_some()
      && !chart.dragged
      && over
    {
      let column = (chart.world(pixel) / SIZE as f32).floor().as_ivec2();
      stakes.write(Stake(column));
      chart.since = REDRAW_EVERY
    }
  }
}

fn draw(
  menu: Res<Menu>,
  pilot: Option<Res<Pilot>>,
  voxels: Option<Res<Voxels>>,
  canvas: Query<&RelativeCursorPosition, With<Canvas>>,
  claims: Query<&Claim>,
  names: Query<&Player, With<Me>>,
  others: Query<(&Avatar, &Player), Without<Me>>,
  visits: Query<&Visited>,
  mut chart: ResMut<Chart>,
  mut images: ResMut<Assets<Image>>,
  mut legends: Query<&mut Text, With<Legend>>
) {
  if menu.showing(Tab::Map)
    && let Some(pilot) = pilot
    && let Some(voxels) = voxels
    && chart.since >= REDRAW_EVERY
  {
    chart.since = 0.0;
    let me = names.single().map(|player| player.name.clone()).unwrap_or_default();
    let (wide, tall) = (chart.size.x, chart.size.y);
    let origin = chart.origin().as_ivec2();
    let mut pixels = vec![UNKNOWN; (wide * tall) as usize];
    for tile in chart.wanted().into_iter() {
      if let Some(cached) = chart.tiles.get(&(chart.zoom, tile)) {
        let corner = tile * TILE - origin;
        for y in corner.y.max(0)..(corner.y + TILE).min(tall) {
          let (from, to) = (corner.x.max(0), (corner.x + TILE).min(wide));
          let source = ((y - corner.y) * TILE + from - corner.x) as usize;
          let target = (y * wide + from) as usize;
          let span = (to - from).max(0) as usize;
          pixels[target..target + span].copy_from_slice(&cached[source..source + span])
        }
      }
    }
    let mut paint = |pixel: IVec2, colour: [f32; 3], blend: f32| {
      if pixel.cmpge(IVec2::ZERO).all() && pixel.cmplt(IVec2::new(wide, tall)).all() {
        let index = (pixel.y * wide + pixel.x) as usize;
        let [r, g, b, a] = pixels[index];
        let mix =
          |old: u8, new: f32| (old as f32 * (1.0 - blend) + new * 255.0 * blend) as u8;
        pixels[index] = [mix(r, colour[0]), mix(g, colour[1]), mix(b, colour[2]), a]
      }
    };
    let chunk = SIZE as f32;
    for claim in claims.iter() {
      let low = chart.pixel(claim.column.as_vec2() * chunk).floor().as_ivec2();
      let high = chart.pixel((claim.column.as_vec2() + 1.0) * chunk).floor().as_ivec2();
      let colour = crate::protocol::hue(claim.hue).to_srgba().to_f32_array_no_alpha();
      for y in low.y.max(0)..high.y.min(tall) {
        for x in low.x.max(0)..high.x.min(wide) {
          let edge = x == low.x || y == low.y || x == high.x - 1 || y == high.y - 1;
          paint(IVec2::new(x, y), colour, if edge { 0.9 } else { 0.35 })
        }
      }
    }
    if chart.scale() <= 1.0 {
      let first = (chart.world(Vec2::ZERO) / chunk).floor().as_ivec2();
      let last = (chart.world(chart.size.as_vec2()) / chunk).ceil().as_ivec2();
      for cx in first.x..=last.x {
        let x = chart.pixel(Vec2::new(cx as f32 * chunk, 0.0)).x as i32;
        for y in (0..tall).step_by(2) {
          paint(IVec2::new(x, y), [0.0; 3], 0.25)
        }
      }
      for cz in first.y..=last.y {
        let y = chart.pixel(Vec2::new(0.0, cz as f32 * chunk)).y as i32;
        for x in (0..wide).step_by(2) {
          paint(IVec2::new(x, y), [0.0; 3], 0.25)
        }
      }
    }
    let dot = |paint: &mut dyn FnMut(IVec2, [f32; 3], f32),
               at: Vec2,
               radius: i32,
               colour: [f32; 3]| {
      let centre = chart.pixel(at).floor().as_ivec2();
      for dy in -radius..=radius {
        for dx in -radius..=radius {
          paint(centre + IVec2::new(dx, dy), colour, 1.0)
        }
      }
    };
    for &cell in visits.get(pilot.me).into_iter().flat_map(|visited| visited.0.iter()) {
      if let Some(island) = Island::at(voxels.seed, cell) {
        dot(&mut paint, island.stone.xz().as_vec2(), 2, [0.9, 0.25, 0.9])
      }
    }
    for (avatar, player) in others.iter() {
      dot(
        &mut paint,
        avatar.at.xz(),
        2,
        crate::protocol::hue(player.hue).to_srgba().to_f32_array_no_alpha()
      )
    }
    let facing = (pilot.facing() * Vec3::NEG_Z).xz().normalize_or(Vec2::NEG_Y);
    let across = facing.perp();
    let centre = chart.pixel(pilot.at.xz());
    let (tip, left, right) = (
      centre + facing * 9.0,
      centre - facing * 5.0 + across * 6.0,
      centre - facing * 5.0 - across * 6.0
    );
    let side = |a: Vec2, b: Vec2, point: Vec2| (b - a).perp_dot(point - a);
    for dy in -10..=10 {
      for dx in -10..=10 {
        let point = centre.floor() + Vec2::new(dx as f32, dy as f32) + 0.5;
        let signs =
          [side(tip, left, point), side(left, right, point), side(right, tip, point)];
        if signs.iter().all(|&sign| sign >= 0.0) || signs.iter().all(|&sign| sign <= 0.0)
        {
          paint(point.floor().as_ivec2(), [1.0, 0.27, 0.2], 1.0)
        }
      }
    }
    if let Some(mut image) = images.get_mut(&chart.image) {
      image.data = Some(pixels.concat())
    }
    let hovered = canvas
      .single()
      .ok()
      .filter(|cursor| cursor.cursor_over)
      .and_then(|cursor| cursor.normalized)
      .map(|spot| chart.world((spot + 0.5) * chart.size.as_vec2()).floor().as_ivec2());
    let mine = claims.iter().filter(|claim| claim.owner == me).count();
    let line = hovered.map_or(format!("Your claims: {mine}/{MOST}"), |at| {
      let column = (at.as_vec2() / SIZE as f32).floor().as_ivec2();
      let owner = claims
        .iter()
        .find(|claim| claim.column == column)
        .map_or("unclaimed".to_string(), |claim| format!("claimed by {}", claim.owner));
      format!(
        "x {} z {}   |   chunk {} {}: {owner}   |   your claims: {mine}/{MOST}   |   {} blocks per pixel",
        at.x,
        at.y,
        column.x,
        column.y,
        chart.scale()
      )
    });
    for mut text in legends.iter_mut() {
      if text.0 != line {
        text.0 = line.clone()
      }
    }
  }
}

pub struct Charting;

impl Plugin for Charting {
  fn build(&self, app: &mut App) {
    app.add_systems(Startup, prepare.run_if(plays)).add_systems(
      Update,
      (fit, steer, survey, draw).chain().run_if(resource_exists::<Chart>)
    );
  }
}
