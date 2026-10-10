use {crate::{authority::{Controller, player_of, within_reach},
             menu::{Menu, Tab},
             noise::unit,
             protocol::{Avatar, Inscribe, Sign, authority, plays},
             texture::{Palette, timber},
             voxels::Voxels},
     bevy::{asset::RenderAssetUsages,
            image::ImageSampler,
            mesh::{Indices, PrimitiveTopology},
            prelude::*,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat}},
     bevy_replicon::prelude::*,
     std::f32::consts::TAU};

pub const LINES: usize = 4;
pub const LINE: usize = 16;
const WIDE: u32 = 64;
const FACE: u32 = 32;
const TALL: u32 = 48;
const ROW: u32 = 7;

pub fn tidy(text: &str) -> String {
  text
    .split('\n')
    .take(LINES)
    .map(|line| line.chars().filter(|c| !c.is_control()).take(LINE).collect::<String>())
    .collect::<Vec<_>>()
    .join("\n")
}

pub fn facing(from: Vec3, at: IVec3) -> f32 {
  let offset = from.xz() - (at.as_vec3().xz() + 0.5);
  let step = TAU / 16.0;
  (offset.x.atan2(offset.y) / step).round() * step
}

#[derive(Resource, Default)]
pub struct Inscription {
  pub at: Option<IVec3>,
  pub lines: [String; LINES]
}

impl Inscription {
  pub fn begin(&mut self, at: IVec3, text: &str) {
    let mut lines = text.split('\n').map(String::from);
    self.at = Some(at);
    self.lines = std::array::from_fn(|_| lines.next().unwrap_or_default())
  }
}

fn inscribe(
  mut inscriptions: MessageReader<FromClient<Inscribe>>,
  players: Query<(&Controller, &Avatar)>,
  mut signs: Query<&mut Sign>
) {
  for FromClient { client_id, message: Inscribe { at, text } } in inscriptions.read() {
    if let Some(avatar) = player_of(players.iter(), *client_id)
      && within_reach(avatar, *at)
      && let Some(mut sign) = signs.iter_mut().find(|sign| sign.at == *at)
    {
      let text = tidy(text);
      if sign.text != text {
        sign.text = text
      }
    }
  }
}

fn topple(voxels: Res<Voxels>, signs: Query<(Entity, &Sign)>, mut commands: Commands) {
  for (entity, _) in signs
    .iter()
    .filter(|(_, sign)| voxels.block(sign.at).is_some_and(|block| !block.sign()))
  {
    commands.entity(entity).despawn()
  }
}

const GLYPHS: [(char, &str); 55] = [
  ('A', "010101111101101"),
  ('B', "110101110101110"),
  ('C', "011100100100011"),
  ('D', "110101101101110"),
  ('E', "111100110100111"),
  ('F', "111100110100100"),
  ('G', "011100101101011"),
  ('H', "101101111101101"),
  ('I', "111010010010111"),
  ('J', "001001001101010"),
  ('K', "101101110101101"),
  ('L', "100100100100111"),
  ('M', "101111111101101"),
  ('N', "110101101101101"),
  ('O', "010101101101010"),
  ('P', "110101110100100"),
  ('Q', "010101101110011"),
  ('R', "110101110101101"),
  ('S', "011100010001110"),
  ('T', "111010010010010"),
  ('U', "101101101101111"),
  ('V', "101101101101010"),
  ('W', "101101111111101"),
  ('X', "101101010101101"),
  ('Y', "101101010010010"),
  ('Z', "111001010100111"),
  ('0', "111101101101111"),
  ('1', "010110010010111"),
  ('2', "110001010100111"),
  ('3', "110001010001110"),
  ('4', "101101111001001"),
  ('5', "111100110001110"),
  ('6', "011100111101111"),
  ('7', "111001010010010"),
  ('8', "111101111101111"),
  ('9', "111101111001110"),
  ('.', "000000000000010"),
  (',', "000000000010100"),
  ('!', "010010010000010"),
  ('?', "110001010000010"),
  ('-', "000000111000000"),
  (':', "000010000010000"),
  ('\'', "010010000000000"),
  ('"', "101101000000000"),
  ('/', "001001010100100"),
  ('+', "000010111010000"),
  ('=', "000111000111000"),
  ('(', "010100100100010"),
  (')', "010001001001010"),
  ('<', "001010100010001"),
  ('>', "100010001010100"),
  ('#', "101111101111101"),
  ('_', "000000000000111"),
  ('*', "000101010101000"),
  ('&', "010101010101011")
];

const UNKNOWN: &str = "110001010000010";

pub fn inked(letter: char, x: u32, y: u32) -> bool {
  let upper = letter.to_ascii_uppercase();
  GLYPHS
    .iter()
    .find(|&&(known, _)| known == upper)
    .map(|&(_, bits)| bits)
    .or((!upper.is_whitespace()).then_some(UNKNOWN))
    .is_some_and(|bits| bits.as_bytes()[(y * 3 + x) as usize] == b'1')
}

fn grain(palette: &Palette, x: u32, y: u32) -> [f32; 3] {
  let band = y / 8;
  let streak = unit(0x516, (x / 5) as i32, band as i32, (y % 8 / 3) as i32);
  match y % 8 == 7 {
    true => palette[1],
    false => palette[2].map(|channel| channel * (0.9 + streak * 0.16))
  }
}

fn written(text: &str, x: u32, y: u32) -> bool {
  let line = y.saturating_sub(2) / ROW;
  let row = y.saturating_sub(2) % ROW;
  text.split('\n').nth(line as usize).is_some_and(|line| {
    let letters: Vec<char> = line.chars().collect();
    let width = (letters.len() as u32 * 4).saturating_sub(1);
    let left = WIDE.saturating_sub(width) / 2;
    y >= 2
      && row < 5
      && x >= left
      && letters
        .get(((x - left) / 4) as usize)
        .is_some_and(|&letter| (x - left) % 4 < 3 && inked(letter, (x - left) % 4, row))
  })
}

fn engrave(sign: &Sign) -> Vec<u8> {
  let palette = timber(sign.wood);
  (0..WIDE * TALL)
    .flat_map(|index| {
      let (x, y) = (index % WIDE, index / WIDE);
      let [r, g, b] = match y < FACE && written(&sign.text, x, y) {
        true => palette[0].map(|channel| channel * 0.45),
        false => grain(&palette, x, y)
      };
      [r, g, b, 1.0].map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u8)
    })
    .collect()
}

fn slab(low: Vec3, high: Vec3, front: Option<Rect>, rest: Rect) -> Mesh {
  let (centre, size) = ((low + high) / 2.0, high - low);
  let faces = [
    (Vec3::Z, Vec3::X, Vec3::NEG_Y, front.unwrap_or(rest)),
    (Vec3::NEG_Z, Vec3::NEG_X, Vec3::NEG_Y, rest),
    (Vec3::X, Vec3::NEG_Z, Vec3::NEG_Y, rest),
    (Vec3::NEG_X, Vec3::Z, Vec3::NEG_Y, rest),
    (Vec3::Y, Vec3::X, Vec3::Z, rest),
    (Vec3::NEG_Y, Vec3::X, Vec3::NEG_Z, rest)
  ];
  let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
  let quads: Vec<([f32; 3], [f32; 3], [f32; 2])> = faces
    .iter()
    .flat_map(|&(normal, across, down, rect)| {
      let extent = |axis: Vec3| axis.abs().dot(size);
      corners.map(|(s, t): (f32, f32)| {
        let at = centre
          + normal * extent(normal) / 2.0
          + across * (s - 0.5) * extent(across)
          + down * (t - 0.5) * extent(down);
        let uv = rect.min + Vec2::new(s, t) * rect.size();
        (at.to_array(), normal.to_array(), uv.to_array())
      })
    })
    .collect();
  Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_POSITION,
      quads.iter().map(|quad| quad.0).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_NORMAL,
      quads.iter().map(|quad| quad.1).collect::<Vec<_>>()
    )
    .with_inserted_attribute(
      Mesh::ATTRIBUTE_UV_0,
      quads.iter().map(|quad| quad.2).collect::<Vec<_>>()
    )
    .with_inserted_indices(Indices::U32(
      (0..6u32)
        .flat_map(|face| [0, 2, 1, 0, 3, 2].map(|corner| face * 4 + corner))
        .collect()
    ))
}

#[derive(Resource)]
struct Carpentry {
  post: Handle<Mesh>,
  board: Handle<Mesh>
}

fn plane(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
  let face = FACE as f32 / TALL as f32;
  let (front, rest) = (Rect::new(0.0, 0.0, 1.0, face), Rect::new(0.0, face, 1.0, 1.0));
  let sixteenth = 1.0 / 16.0;
  commands.insert_resource(Carpentry {
    post: meshes.add(slab(
      Vec3::new(-1.0, 0.0, -1.0) * sixteenth,
      Vec3::new(1.0, 9.0, 1.0) * sixteenth,
      None,
      Rect::new(0.0, face, 0.1, 1.0)
    )),
    board: meshes.add(slab(
      Vec3::new(-8.0, 9.0, -0.75) * sixteenth,
      Vec3::new(8.0, 17.0, 0.75) * sixteenth,
      Some(front),
      rest
    ))
  })
}

#[derive(Component)]
struct Lettered(Handle<Image>);

fn letter(
  carpentry: Res<Carpentry>,
  signs: Query<(Entity, &Sign, Option<&Lettered>), Changed<Sign>>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut commands: Commands
) {
  for (entity, sign, lettered) in signs.iter() {
    let placed =
      Transform::from_translation(sign.at.as_vec3() + Vec3::new(0.5, 0.0, 0.5))
        .with_rotation(Quat::from_rotation_y(sign.yaw));
    if let Some(lettered) = lettered
      && let Some(mut image) = images.get_mut(&lettered.0)
    {
      image.data = Some(engrave(sign))
    } else {
      let mut image = Image::new(
        Extent3d { width: WIDE, height: TALL, depth_or_array_layers: 1 },
        TextureDimension::D2,
        engrave(sign),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default()
      );
      image.sampler = ImageSampler::nearest();
      let image = images.add(image);
      let material = MeshMaterial3d(materials.add(StandardMaterial {
        base_color_texture: Some(image.clone()),
        perceptual_roughness: 0.9,
        ..default()
      }));
      commands
        .entity(entity)
        .insert((Lettered(image), placed, Visibility::default()))
        .with_children(|sign| {
          sign.spawn((Mesh3d(carpentry.post.clone()), material.clone()));
          sign.spawn((Mesh3d(carpentry.board.clone()), material));
        });
    }
  }
}

fn seal(
  menu: Res<Menu>,
  mut inscription: ResMut<Inscription>,
  mut inscriptions: MessageWriter<Inscribe>
) {
  if let Some(at) = inscription.at
    && !menu.showing(Tab::Sign)
  {
    inscriptions.write(Inscribe { at, text: tidy(&inscription.lines.join("\n")) });
    inscription.at = None
  }
}

pub struct Signs;

impl Plugin for Signs {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Inscription>()
      .add_systems(PreUpdate, inscribe.after(ServerSystems::Receive).run_if(authority))
      .add_systems(Update, topple.run_if(authority).run_if(resource_exists::<Voxels>))
      .add_systems(Startup, plane.run_if(plays))
      .add_systems(Update, (letter, seal).run_if(plays));
  }
}
