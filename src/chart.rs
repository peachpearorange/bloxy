use {crate::{block::{Block, Fluid},
             claim::{Claim, MOST, Stake},
             generate,
             island::Island,
             menu::{FAINT, INK, Menu, Tab, words},
             minimap::hue,
             player::{Me, Pilot},
             protocol::{Avatar, Player, Visited, plays},
             voxels::{SIZE, Voxels}},
     bevy::{asset::RenderAssetUsages,
            image::ImageSampler,
            input::mouse::AccumulatedMouseScroll,
            platform::time::Instant,
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
            ui::RelativeCursorPosition}};

const WIDE: i32 = 960;
const TALL: i32 = 540;
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
  drawn: Option<(Vec2, usize)>,
  base: Vec<[u8; 4]>,
  heights: Vec<i32>,
  row: i32,
  since: f32,
  grip: Option<(Vec2, Vec2)>,
  dragged: bool,
  opened: bool
}

impl Chart {
  fn scale(&self) -> f32 { SCALES[self.zoom] }

  fn world(&self, pixel: Vec2) -> Vec2 {
    self.centre + (pixel - Vec2::new(WIDE as f32, TALL as f32) / 2.0) * self.scale()
  }

  fn pixel(&self, world: Vec2) -> Vec2 {
    (world - self.centre) / self.scale() + Vec2::new(WIDE as f32, TALL as f32) / 2.0
  }
}

#[derive(Component)]
struct Canvas;

#[derive(Component)]
struct Legend;

pub fn prepare(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
  let mut image = Image::new_fill(
    Extent3d { width: WIDE as u32, height: TALL as u32, depth_or_array_layers: 1 },
    TextureDimension::D2,
    &UNKNOWN,
    TextureFormat::Rgba8UnormSrgb,
    RenderAssetUsages::default()
  );
  image.sampler = ImageSampler::nearest();
  commands.insert_resource(Chart {
    image: images.add(image),
    hues: Block::ALL.map(hue).to_vec(),
    centre: Vec2::ZERO,
    zoom: 3,
    drawn: None,
    base: vec![UNKNOWN; (WIDE * TALL) as usize],
    heights: vec![0; (WIDE * TALL) as usize],
    row: 0,
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
      height: vh(62),
      aspect_ratio: Some(WIDE as f32 / TALL as f32),
      align_self: AlignSelf::Center,
      border: UiRect::all(px(2)),
      ..default()
    },
    BorderColor::all(Color::srgb(0.3, 0.33, 0.36))
  ));
  page.spawn((Legend, words("", 16.0, INK)));
  page.spawn(words(
    format!(
      "Wheel zooms, drag pans. Click a chunk to claim it or give it back (up to {MOST}). \
       Claimed land is yours to build on and survives when the world is regenerated."
    ),
    14.0,
    FAINT
  ));
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

fn survey(voxels: Option<Res<Voxels>>, menu: Res<Menu>, mut chart: ResMut<Chart>) {
  if let Some(voxels) = voxels
    && menu.showing(Tab::Map)
  {
    let view = (chart.centre, chart.zoom);
    if chart.drawn != Some(view) {
      chart.drawn = Some(view);
      chart.row = 0;
      chart.base.fill(UNKNOWN)
    }
    let started = Instant::now();
    while chart.row < TALL && started.elapsed().as_secs_f32() < BUDGET {
      let row = chart.row;
      (0..WIDE).for_each(|column| {
        let world = chart
          .world(Vec2::new(column as f32 + 0.5, row as f32 + 0.5))
          .floor()
          .as_ivec2();
        let (block, top, floor) = generate::overview(voxels.seed, world.x, world.y);
        let index = (row * WIDE + column) as usize;
        let north = if row == 0 { top } else { chart.heights[index - WIDE as usize] };
        chart.heights[index] = top;
        chart.base[index] = colour(&chart, block, top, floor, north)
      });
      chart.row += 1
    }
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
    let pixel = (spot + 0.5) * Vec2::new(WIDE as f32, TALL as f32);
    let over = cursor.cursor_over;
    let wheel = scroll.delta.y;
    if over && wheel != 0.0 {
      let anchor = chart.world(pixel);
      let zoom =
        (chart.zoom as i32 - wheel.signum() as i32).clamp(0, SCALES.len() as i32 - 1);
      chart.zoom = zoom as usize;
      chart.centre = (anchor
        - (pixel - Vec2::new(WIDE as f32, TALL as f32) / 2.0) * chart.scale())
      .floor()
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
      if chart.dragged {
        chart.centre = (centre - moved * chart.scale()).floor()
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
    let mut pixels = chart.base.clone();
    let mut paint = |pixel: IVec2, colour: [f32; 3], blend: f32| {
      if pixel.cmpge(IVec2::ZERO).all() && pixel.cmplt(IVec2::new(WIDE, TALL)).all() {
        let index = (pixel.y * WIDE + pixel.x) as usize;
        let [r, g, b, a] = pixels[index];
        let mix =
          |old: u8, new: f32| (old as f32 * (1.0 - blend) + new * 255.0 * blend) as u8;
        pixels[index] = [mix(r, colour[0]), mix(g, colour[1]), mix(b, colour[2]), a]
      }
    };
    let chunk = SIZE as f32;
    claims.iter().for_each(|claim| {
      let low = chart.pixel(claim.column.as_vec2() * chunk).floor().as_ivec2();
      let high = chart.pixel((claim.column.as_vec2() + 1.0) * chunk).floor().as_ivec2();
      let colour = crate::protocol::hue(claim.hue).to_srgba().to_f32_array_no_alpha();
      (low.y.max(0)..high.y.min(TALL)).for_each(|y| {
        (low.x.max(0)..high.x.min(WIDE)).for_each(|x| {
          let edge = x == low.x || y == low.y || x == high.x - 1 || y == high.y - 1;
          paint(IVec2::new(x, y), colour, if edge { 0.9 } else { 0.35 })
        })
      })
    });
    if chart.scale() <= 1.0 {
      let first = (chart.world(Vec2::ZERO) / chunk).floor().as_ivec2();
      let last =
        (chart.world(Vec2::new(WIDE as f32, TALL as f32)) / chunk).ceil().as_ivec2();
      (first.x..=last.x).for_each(|cx| {
        let x = chart.pixel(Vec2::new(cx as f32 * chunk, 0.0)).x as i32;
        (0..TALL).step_by(2).for_each(|y| paint(IVec2::new(x, y), [0.0; 3], 0.25))
      });
      (first.y..=last.y).for_each(|cz| {
        let y = chart.pixel(Vec2::new(0.0, cz as f32 * chunk)).y as i32;
        (0..WIDE).step_by(2).for_each(|x| paint(IVec2::new(x, y), [0.0; 3], 0.25))
      });
    }
    let dot = |paint: &mut dyn FnMut(IVec2, [f32; 3], f32),
               at: Vec2,
               radius: i32,
               colour: [f32; 3]| {
      let centre = chart.pixel(at).floor().as_ivec2();
      (-radius..=radius).for_each(|dy| {
        (-radius..=radius).for_each(|dx| paint(centre + IVec2::new(dx, dy), colour, 1.0))
      })
    };
    visits.get(pilot.me).into_iter().flat_map(|visited| visited.0.iter()).for_each(
      |&cell| {
        if let Some(island) = Island::at(voxels.seed, cell) {
          dot(&mut paint, island.stone.xz().as_vec2(), 2, [0.9, 0.25, 0.9])
        }
      }
    );
    others.iter().for_each(|(avatar, player)| {
      dot(
        &mut paint,
        avatar.at.xz(),
        2,
        crate::protocol::hue(player.hue).to_srgba().to_f32_array_no_alpha()
      )
    });
    let facing = (pilot.facing() * Vec3::NEG_Z).xz().normalize_or(Vec2::NEG_Y);
    let across = facing.perp();
    let centre = chart.pixel(pilot.at.xz());
    let (tip, left, right) = (
      centre + facing * 9.0,
      centre - facing * 5.0 + across * 6.0,
      centre - facing * 5.0 - across * 6.0
    );
    let side = |a: Vec2, b: Vec2, point: Vec2| (b - a).perp_dot(point - a);
    (-10..=10).for_each(|dy| {
      (-10..=10).for_each(|dx| {
        let point = centre.floor() + Vec2::new(dx as f32, dy as f32) + 0.5;
        let signs =
          [side(tip, left, point), side(left, right, point), side(right, tip, point)];
        if signs.iter().all(|&sign| sign >= 0.0) || signs.iter().all(|&sign| sign <= 0.0)
        {
          paint(point.floor().as_ivec2(), [1.0, 0.27, 0.2], 1.0)
        }
      })
    });
    if let Some(mut image) = images.get_mut(&chart.image) {
      image.data = Some(pixels.concat())
    }
    let hovered = canvas
      .single()
      .ok()
      .filter(|cursor| cursor.cursor_over)
      .and_then(|cursor| cursor.normalized)
      .map(|spot| {
        chart.world((spot + 0.5) * Vec2::new(WIDE as f32, TALL as f32)).floor().as_ivec2()
      });
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
    legends.iter_mut().for_each(|mut text| {
      if text.0 != line {
        text.0 = line.clone()
      }
    })
  }
}

pub struct Charting;

impl Plugin for Charting {
  fn build(&self, app: &mut App) {
    app.add_systems(Startup, prepare.run_if(plays)).add_systems(
      Update,
      (steer, survey, draw).chain().run_if(resource_exists::<Chart>)
    );
  }
}
