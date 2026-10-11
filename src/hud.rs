use {crate::{block::Block,
             menu::Menu,
             model,
             opts::opts,
             player::{Aim, Pilot, Selected},
             protocol::{HOTBAR, Health, Inventory, Notice, Role, plays},
             stream::{Palette, Progress},
             texture::{ICON, ICON_COLUMNS},
             voxels::Voxels},
     bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
            prelude::*,
            window::PrimaryWindow},
     bevy_replicon::prelude::*};

#[derive(Component)]
struct Slot(usize);

#[derive(Component)]
struct Icon(usize);

#[derive(Component)]
struct Count(usize);

#[derive(Component)]
struct Status;

#[derive(Component)]
struct Hotbar;

#[derive(Component)]
struct Outline;

#[derive(Component)]
struct Edge(usize);

#[derive(Component)]
struct Heart(u8);

#[derive(Component)]
struct Flash;

#[derive(Component)]
struct Announcement;

#[derive(Component)]
struct Cooling(usize);

#[derive(Component)]
struct Learning;

#[derive(Component)]
struct Level;

const HEARTS: u8 = 25;

const HEART: [&str; 9] = [
  ".00...00.",
  "0220.0110",
  "022101110",
  "021111110",
  "011111110",
  ".0111110.",
  "..01110..",
  "...010...",
  "....0...."
];
const HEART_SIZE: u32 = 9;

fn hearts() -> Image {
  let texel = |frame: u32, x: u32, y: u32| -> [u8; 4] {
    let mark = HEART[y as usize].as_bytes()[x as usize];
    let filled = match frame {
      0 => true,
      1 => x < HEART_SIZE / 2 + 1,
      _ => false
    };
    match (mark, filled) {
      (b'.', _) => [0, 0, 0, 0],
      (b'0', _) => [24, 10, 12, 255],
      (b'2', true) => [255, 150, 150, 255],
      (_, true) => [214, 34, 44, 255],
      _ => [70, 56, 58, 255]
    }
  };
  let mut image = Image::new(
    bevy::render::render_resource::Extent3d {
      width: HEART_SIZE * 3,
      height: HEART_SIZE,
      depth_or_array_layers: 1
    },
    bevy::render::render_resource::TextureDimension::D2,
    (0..HEART_SIZE * 3 * HEART_SIZE)
      .flat_map(|index| {
        let (x, y) = (index % (HEART_SIZE * 3), index / (HEART_SIZE * 3));
        texel(x / HEART_SIZE, x % HEART_SIZE, y)
      })
      .collect(),
    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
    bevy::asset::RenderAssetUsages::default()
  );
  image.sampler = bevy::image::ImageSampler::nearest();
  image
}

const EDGE: f32 = 0.012;

const SLOT: f32 = 46.0;
const ICON_SHOWN: f32 = 40.0;
const STRIP: Color = Color::srgba(0.42, 0.42, 0.44, 0.7);
const PICKED: Color = Color::srgb(0.95, 0.95, 0.95);

pub const SHADE: TextShadow = TextShadow { offset: Vec2::ONE, color: Color::BLACK };

fn scale(
  windows: Query<&Window, (With<PrimaryWindow>, Changed<Window>)>,
  mut scale: ResMut<UiScale>
) {
  for window in windows.iter() {
    let fitted = ((window.height() / 720.0) * 4.0).floor().max(4.0) / 4.0;
    if scale.0 != fitted {
      scale.0 = fitted
    }
  }
}

fn tuck(
  menu: Res<Menu>,
  mut bars: Query<&mut Visibility, Or<(With<Hotbar>, With<Status>)>>
) {
  for mut visibility in bars.iter_mut() {
    visibility.set_if_neq(if menu.open {
      Visibility::Hidden
    } else {
      Visibility::Inherited
    });
  }
}

fn build(
  mut commands: Commands,
  palette: Res<Palette>,
  mut meshes: ResMut<Assets<Mesh>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut images: ResMut<Assets<Image>>
) {
  let heart = images.add(hearts());
  commands.spawn((
    Flash,
    Node {
      width: percent(100),
      height: percent(100),
      position_type: PositionType::Absolute,
      ..default()
    },
    BackgroundColor(Color::srgba(0.8, 0.0, 0.0, 0.0))
  ));
  commands
    .spawn((Hotbar, Node {
      width: percent(100),
      position_type: PositionType::Absolute,
      bottom: px(8.0 + SLOT + 9.0),
      justify_content: JustifyContent::Center,
      ..default()
    }))
    .with_children(|row| {
      row
        .spawn(Node {
          width: px(SLOT * HOTBAR as f32),
          column_gap: px(2),
          row_gap: px(1),
          flex_wrap: FlexWrap::WrapReverse,
          ..default()
        })
        .with_children(|hearts| {
          for index in 0..HEARTS {
            hearts.spawn((
              Heart(index),
              ImageNode {
                rect: Some(Rect::new(0.0, 0.0, HEART_SIZE as f32, HEART_SIZE as f32)),
                ..ImageNode::new(heart.clone())
              },
              Node { width: px(22), height: px(22), ..default() }
            ));
          }
        });
    });
  commands
    .spawn(Node {
      width: percent(100),
      height: percent(100),
      justify_content: JustifyContent::Center,
      align_items: AlignItems::Center,
      position_type: PositionType::Absolute,
      ..default()
    })
    .with_children(|centre| {
      centre.spawn((
        Node {
          width: px(2),
          height: px(18),
          position_type: PositionType::Absolute,
          ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8))
      ));
      centre.spawn((
        Node {
          width: px(18),
          height: px(2),
          position_type: PositionType::Absolute,
          ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8))
      ));
    });
  commands
    .spawn((Hotbar, Node {
      width: percent(100),
      position_type: PositionType::Absolute,
      bottom: px(8),
      justify_content: JustifyContent::Center,
      ..default()
    }))
    .with_children(|row| {
      row.spawn((Node::default(), BackgroundColor(STRIP))).with_children(|bar| {
        for index in 0..HOTBAR {
          bar
            .spawn((
              Slot(index),
              Node {
                width: px(SLOT),
                height: px(SLOT),
                border: UiRect::all(px(2)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
              },
              BorderColor::all(Color::NONE)
            ))
            .with_children(|slot| {
              slot.spawn((
                Icon(index),
                ImageNode::new(palette.icons.clone()),
                Node { width: px(ICON_SHOWN), height: px(ICON_SHOWN), ..default() },
                Visibility::Hidden
              ));
              slot.spawn((
                Cooling(index),
                Node {
                  position_type: PositionType::Absolute,
                  left: px(0),
                  right: px(0),
                  bottom: px(0),
                  height: percent(0),
                  ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))
              ));
              slot.spawn((
                Count(index),
                Text::new(""),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                crate::hud::SHADE,
                Node {
                  position_type: PositionType::Absolute,
                  right: px(3),
                  bottom: px(1),
                  ..default()
                }
              ));
            });
        }
      });
    });
  commands
    .spawn((Hotbar, Node {
      width: percent(100),
      position_type: PositionType::Absolute,
      bottom: px(8.0 + SLOT + 2.0),
      justify_content: JustifyContent::Center,
      ..default()
    }))
    .with_children(|row| {
      row
        .spawn((
          Node { width: px(SLOT * HOTBAR as f32), height: px(5), ..default() },
          BackgroundColor(Color::srgba(0.05, 0.05, 0.06, 0.75))
        ))
        .with_children(|bar| {
          bar.spawn((
            Learning,
            Node { width: percent(0), height: percent(100), ..default() },
            BackgroundColor(Color::srgb(0.5, 0.95, 0.35))
          ));
          bar.spawn((
            Level,
            Text::new(""),
            TextFont { font_size: FontSize::Px(14.0), ..default() },
            TextColor(Color::srgb(0.6, 1.0, 0.45)),
            crate::hud::SHADE,
            Node {
              position_type: PositionType::Absolute,
              left: px(-34),
              top: px(-7),
              ..default()
            }
          ));
        });
    });
  commands
    .spawn((
      Node {
        width: percent(100),
        position_type: PositionType::Absolute,
        bottom: px(12.0 + SLOT + 40.0),
        justify_content: JustifyContent::Center,
        ..default()
      },
      GlobalZIndex(20)
    ))
    .with_children(|row| {
      row.spawn((
        Announcement,
        Text::new(""),
        TextFont { font_size: FontSize::Px(18.0), ..default() },
        TextColor(Color::srgba(1.0, 0.95, 0.75, 0.0)),
        crate::hud::SHADE
      ));
    });
  commands.spawn((
    Status,
    Text::new(""),
    TextFont { font_size: FontSize::Px(14.0), ..default() },
    crate::hud::SHADE,
    Node { position_type: PositionType::Absolute, left: px(10), top: px(8), ..default() }
  ));
  let bar = meshes.add(Cuboid::default());
  let ink = materials.add(StandardMaterial {
    base_color: Color::srgba(0.05, 0.05, 0.05, 0.9),
    unlit: true,
    ..default()
  });
  commands.spawn((Outline, Transform::default(), Visibility::Hidden)).with_children(
    |outline| {
      for index in 0..12 {
        outline.spawn((
          Edge(index),
          Mesh3d(bar.clone()),
          MeshMaterial3d(ink.clone()),
          Transform::default()
        ));
      }
    }
  );
}

pub fn icon_rect(block: Block) -> Rect {
  let index = block as u32;
  let corner =
    Vec2::new((index % ICON_COLUMNS) as f32, (index / ICON_COLUMNS) as f32) * ICON as f32;
  Rect::from_corners(corner, corner + Vec2::splat(ICON as f32))
}

fn refresh(
  time: Res<Time>,
  pilot: Option<Res<Pilot>>,
  selected: Res<Selected>,
  recharge: Res<crate::spell::Recharge>,
  inventories: Query<&Inventory>,
  skills: Query<&crate::skill::Skills>,
  mut slots: Query<(&Slot, &mut BorderColor)>,
  mut icons: Query<(&Icon, &mut ImageNode, &mut Visibility)>,
  mut counts: Query<(&Count, &mut Text, &mut TextColor)>,
  mut cooling: Query<(&Cooling, &mut Node)>
) {
  let stance = pilot
    .as_ref()
    .and_then(|pilot| skills.get(pilot.me).ok())
    .and_then(|skills| skills.stance);
  let inventory =
    pilot.and_then(|pilot| inventories.get(pilot.me).ok().cloned()).unwrap_or_default();
  for (slot, mut border) in slots.iter_mut() {
    border.set_if_neq(BorderColor::all(match slot.0 == selected.0 {
      true => PICKED,
      false => Color::NONE
    }));
  }
  for (icon, mut image, mut visibility) in icons.iter_mut() {
    match inventory.slots[icon.0] {
      Some(stack) => {
        image.rect = Some(icon_rect(stack.block));
        *visibility = Visibility::Inherited
      }
      None => *visibility = Visibility::Hidden
    }
  }
  for (count, mut text, mut colour) in counts.iter_mut() {
    let (shown, tint) = match inventory.slots[count.0] {
      Some(stack) if stack.block.spell() => {
        let fuel = inventory.count(crate::spell::fuel(stack.block));
        (
          fuel.to_string(),
          if fuel == 0 {
            Color::srgb(1.0, 0.4, 0.35)
          } else {
            Color::srgb(0.75, 0.85, 1.0)
          }
        )
      }
      Some(stack) if stack.count > 1 => (stack.count.to_string(), Color::WHITE),
      _ => (String::new(), Color::WHITE)
    };
    if text.0 != shown {
      text.0 = shown
    }
    colour.set_if_neq(TextColor(tint));
  }
  let now = time.elapsed_secs();
  for (cooling, mut node) in cooling.iter_mut() {
    let left = inventory.slots[cooling.0]
      .filter(|stack| stack.block.spell())
      .and_then(|stack| {
        recharge
          .0
          .get(&stack.block)
          .map(|&ready| (ready - now) / crate::spell::recharge(stack.block, stance))
      })
      .unwrap_or(0.0)
      .clamp(0.0, 1.0);
    let height = percent(left * 100.0);
    if node.height != height {
      node.height = height
    }
  }
}

fn learning(
  pilot: Option<Res<Pilot>>,
  skills: Query<&crate::skill::Skills>,
  mut bars: Query<&mut Node, With<Learning>>,
  mut levels: Query<&mut Text, With<Level>>
) {
  let skills =
    pilot.and_then(|pilot| skills.get(pilot.me).ok().cloned()).unwrap_or_default();
  let filled =
    percent(skills.xp as f32 / crate::skill::needed(skills.level) as f32 * 100.0);
  for mut node in bars.iter_mut() {
    if node.width != filled {
      node.width = filled
    }
  }
  let line = match skills.points() > 0 {
    true => format!("{}+", skills.level),
    false => skills.level.to_string()
  };
  for mut text in levels.iter_mut() {
    if text.0 != line {
      text.0 = line.clone()
    }
  }
}

fn edge(index: usize, low: Vec3, high: Vec3) -> Transform {
  let (axis, corner) = (index / 4, index % 4);
  let size = high - low;
  let mut scale = Vec3::splat(EDGE);
  scale[axis] = size[axis] + EDGE;
  let mut centre = low + size / 2.0;
  centre[(axis + 1) % 3] = [low, high][corner % 2][(axis + 1) % 3];
  centre[(axis + 2) % 3] = [low, high][corner / 2][(axis + 2) % 3];
  Transform::from_translation(centre).with_scale(scale)
}

fn aim(
  aim: Res<Aim>,
  voxels: Option<Res<Voxels>>,
  mut outline: Query<&mut Visibility, With<Outline>>,
  mut edges: Query<(&Edge, &mut Transform)>
) {
  let bounds = aim.hit.as_ref().zip(voxels).map(|(hit, voxels)| {
    model::bounds(hit.block, voxels.seed, hit.at).map_or_else(
      || {
        let corner = hit.at.as_vec3();
        (corner - EDGE / 2.0, corner + Vec3::ONE + EDGE / 2.0)
      },
      |(low, high)| {
        let rim = Vec3::new(EDGE, 0.0, EDGE);
        (low - rim + Vec3::Y * (EDGE / 2.0 + 0.004), high + rim + Vec3::Y * EDGE)
      }
    )
  });
  for mut visibility in outline.iter_mut() {
    *visibility = match bounds {
      Some(_) => Visibility::Inherited,
      None => Visibility::Hidden
    }
  }
  if let Some((low, high)) = bounds {
    for (edge, mut transform) in edges.iter_mut() {
      transform.set_if_neq(self::edge(edge.0, low, high));
    }
  }
}

fn status(
  role: Res<Role>,
  state: Res<State<ClientState>>,
  pilot: Option<Res<Pilot>>,
  progress: Res<Progress>,
  players: Query<(), With<crate::protocol::Player>>,
  aim: Res<Aim>,
  diagnostics: Res<DiagnosticsStore>,
  mut texts: Query<&mut Text, With<Status>>
) {
  let fps = diagnostics
    .get(&FrameTimeDiagnosticsPlugin::FPS)
    .and_then(|fps| fps.smoothed())
    .unwrap_or_default();
  let link = match (*role, state.get()) {
    (Role::Guest, ClientState::Connecting) => {
      format!("connecting to {}...", opts().connect.clone().unwrap_or_default())
    }
    (Role::Guest, ClientState::Disconnected) => "disconnected".into(),
    (Role::Guest, ClientState::Connected) => "online".into(),
    (Role::Host, _) => "hosting".into(),
    _ => "solo".into()
  };
  let place = pilot.map_or(String::new(), |pilot| {
    let at = pilot.at.floor().as_ivec3();
    format!("{} {} {}", at.x, at.y, at.z)
  });
  let looking =
    aim.hit.as_ref().map_or(String::new(), |hit| format!("  |  {}", hit.block.name()));
  let loading = (progress.pending > 0)
    .then(|| format!("  |  loading {}", progress.pending))
    .unwrap_or_default();
  let line = format!(
    "{link}  |  {} players  |  {place}{looking}  |  {fps:.0} fps{loading}",
    players.iter().count()
  );
  for mut text in texts.iter_mut() {
    if text.0 != line {
      text.0 = line.clone()
    }
  }
}

fn vitals(
  time: Res<Time>,
  pilot: Option<Res<Pilot>>,
  healths: Query<&Health>,
  skills: Query<&crate::skill::Skills>,
  mut hearts: Query<(&Heart, &mut ImageNode, &mut Node)>,
  mut flash: Query<&mut BackgroundColor, With<Flash>>,
  mut was: Local<Option<u8>>,
  mut redness: Local<f32>
) {
  let most = pilot
    .as_ref()
    .and_then(|pilot| skills.get(pilot.me).ok())
    .map_or(Health::FULL, crate::skill::Skills::most);
  let health = pilot.and_then(|pilot| healths.get(pilot.me).ok()).map(|health| health.0);
  if let (Some(now), Some(before)) = (health, *was)
    && now < before
  {
    *redness = 0.45
  }
  *was = health;
  *redness = (*redness - time.delta_secs()).max(0.0);
  let points = health.unwrap_or(Health::FULL);
  for (heart, mut image, mut node) in hearts.iter_mut() {
    let display = if heart.0 < most.div_ceil(2) { Display::Flex } else { Display::None };
    if node.display != display {
      node.display = display
    }
    let frame = match points.saturating_sub(heart.0 * 2) {
      0 => 2,
      1 => 1,
      _ => 0
    };
    let left = (frame * HEART_SIZE) as f32;
    let rect = Some(Rect::new(left, 0.0, left + HEART_SIZE as f32, HEART_SIZE as f32));
    if image.rect != rect {
      image.rect = rect
    }
  }
  for mut background in flash.iter_mut() {
    background.0 = Color::srgba(0.8, 0.0, 0.0, *redness)
  }
}

const ANNOUNCED_FOR: f32 = 3.5;

fn announce(
  time: Res<Time>,
  mut notices: MessageReader<Notice>,
  mut texts: Query<(&mut Text, &mut TextColor, &mut TextShadow), With<Announcement>>,
  mut since: Local<f32>
) {
  let latest = notices.read().last().map(|notice| notice.0.clone());
  *since = match latest {
    Some(_) => 0.0,
    None => *since + time.delta_secs()
  };
  let fade = (ANNOUNCED_FOR - *since).clamp(0.0, 1.0);
  for (mut text, mut color, mut shadow) in texts.iter_mut() {
    if let Some(line) = &latest {
      text.0 = line.clone()
    }
    color.0.set_alpha(fade);
    shadow.color.set_alpha(fade)
  }
}

pub struct Hud;

impl Plugin for Hud {
  fn build(&self, app: &mut App) {
    app
      .add_plugins(FrameTimeDiagnosticsPlugin::default())
      .add_systems(Startup, build.after(crate::stream::paint).run_if(plays))
      .add_systems(
        Update,
        (refresh, aim, status, vitals, announce, learning).run_if(plays)
      )
      .add_systems(PostUpdate, tuck.run_if(plays))
      .add_systems(PreUpdate, scale.run_if(plays));
  }
}
