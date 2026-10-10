use {crate::{account::random,
             figure::{Clad, Kit, assemble},
             menu::{Act, BUTTON, FAINT, INK, Menu, Pressed, Tab, button, words},
             player::Pilot,
             protocol::{Paint, plays},
             skin::{PALETTE, Part, Skin, TALL, WIDE, used}},
     bevy::{camera::{RenderTarget, visibility::RenderLayers},
            core_pipeline::tonemapping::Tonemapping,
            prelude::*,
            render::render_resource::TextureFormat,
            ui::RelativeCursorPosition}};

const SCALE: f32 = 8.0;
const PREVIEW: usize = 3;
const VIEW: UVec2 = UVec2::new(260, 380);

#[derive(Resource)]
pub struct Draft {
  pub skin: Skin,
  pub colour: u8,
  pub touched: bool,
  raw: Handle<Image>,
  canvas: Handle<Image>,
  view: Handle<Image>,
  material: Handle<StandardMaterial>
}

#[derive(Component)]
struct Canvas;

#[derive(Component)]
struct Unsaved;

#[derive(Component)]
struct Turntable;

#[derive(Component)]
struct Viewer;

pub fn prepare(
  mut commands: Commands,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>
) {
  let skin = Skin::fresh(0);
  let raw = images.add(skin.image(false));
  let material = materials.add(StandardMaterial {
    base_color_texture: Some(raw.clone()),
    perceptual_roughness: 0.85,
    ..default()
  });
  commands.insert_resource(Draft {
    canvas: images.add(skin.image(true)),
    view: images.add(Image::new_target_texture(
      VIEW.x,
      VIEW.y,
      TextureFormat::Rgba16Float,
      None
    )),
    raw,
    material,
    skin,
    colour: 0,
    touched: false
  })
}

fn stage(mut commands: Commands, draft: Res<Draft>, kit: Res<Kit>) {
  let layers = RenderLayers::layer(PREVIEW);
  commands.spawn((
    Viewer,
    Camera3d::default(),
    Camera {
      order: -1,
      is_active: false,
      clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.12, 0.15)),
      ..default()
    },
    RenderTarget::Image(draft.view.clone().into()),
    Projection::Perspective(PerspectiveProjection {
      fov: 34f32.to_radians(),
      ..default()
    }),
    Tonemapping::AcesFitted,
    AmbientLight { brightness: 700.0, ..default() },
    layers.clone(),
    Transform::from_xyz(0.0, 1.2, -4.3).looking_at(Vec3::new(0.0, 0.93, 0.0), Vec3::Y)
  ));
  commands.spawn((
    DirectionalLight { illuminance: 6000.0, shadow_maps_enabled: false, ..default() },
    layers.clone(),
    Transform::from_xyz(-2.0, 3.0, -3.0).looking_at(Vec3::ZERO, Vec3::Y)
  ));
  let clad = Clad { material: draft.material.clone(), image: draft.raw.clone() };
  let mut figure = commands.spawn((
    Turntable,
    Transform::default(),
    Visibility::default(),
    layers.clone()
  ));
  assemble(&mut figure, &kit, &clad, layers);
}

pub fn page(page: &mut ChildSpawnerCommands, draft: &Draft) {
  page.spawn(Node { column_gap: px(20), ..default() }).with_children(|row| {
    row
      .spawn(Node { flex_direction: FlexDirection::Column, row_gap: px(6), ..default() })
      .with_children(|left| {
        left
          .spawn((
            Canvas,
            ImageNode::new(draft.canvas.clone()),
            RelativeCursorPosition::default(),
            Node {
              width: px(WIDE as f32 * SCALE),
              height: px(TALL as f32 * SCALE),
              ..default()
            }
          ))
          .with_children(|canvas| {
            Part::ALL.into_iter().for_each(|part| {
              let corner = part.origin().as_vec2() * SCALE;
              canvas.spawn((words(part.name(), 12.0, INK), Node {
                position_type: PositionType::Absolute,
                left: px(corner.x + 2.0),
                top: px(corner.y + 1.0),
                ..default()
              }));
            })
          });
        left.spawn(Node { flex_wrap: FlexWrap::Wrap, width: px(WIDE as f32 * SCALE), ..default() }).with_children(
          |palette| {
            PALETTE.iter().enumerate().for_each(|(index, &[r, g, b])| {
              palette.spawn((
                Button,
                Act::Swatch(index as u8),
                Node {
                  width: px(32),
                  height: px(28),
                  border: UiRect::all(px(2)),
                  ..default()
                },
                BorderColor::all(BUTTON),
                BackgroundColor(Color::srgb_u8(r, g, b))
              ));
            })
          }
        );
        left.spawn(words(
          "Left mouse paints, right mouse picks a colour. Arms and legs share one pattern.",
          13.0,
          FAINT
        ));
      });
    row
      .spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(6),
        align_items: AlignItems::Center,
        ..default()
      })
      .with_children(|right| {
        right.spawn((ImageNode::new(draft.view.clone()), Node {
          width: px(VIEW.x as f32),
          height: px(VIEW.y as f32),
          ..default()
        }));
        right.spawn(Node { column_gap: px(8), ..default() }).with_children(|buttons| {
          button(buttons, "Random", Act::Randomize);
          button(buttons, "Undo all", Act::Revert);
          button(buttons, "Wear", Act::Wear);
        });
        right.spawn((Unsaved, words("", 14.0, FAINT)));
      });
  });
}

fn follow(mut draft: ResMut<Draft>, pilot: Option<Res<Pilot>>, skins: Query<&Skin>) {
  if let Some(mine) = pilot.and_then(|pilot| skins.get(pilot.me).ok()) {
    match (draft.touched, draft.skin == *mine) {
      (true, true) => draft.touched = false,
      (false, false) => draft.skin = mine.clone(),
      _ => ()
    }
  }
}

fn paint(
  buttons: Res<ButtonInput<MouseButton>>,
  menu: Res<Menu>,
  canvas: Query<&RelativeCursorPosition, With<Canvas>>,
  mut draft: ResMut<Draft>
) {
  if menu.showing(Tab::Skin)
    && let Ok(cursor) = canvas.single()
    && cursor.cursor_over
    && let Some(spot) = cursor.normalized
    && let size = Vec2::new(WIDE as f32, TALL as f32)
    && let texel =
      ((spot + 0.5) * size).floor().as_uvec2().min(UVec2::new(WIDE - 1, TALL - 1))
    && used(texel)
  {
    let colour = draft.colour;
    if buttons.pressed(MouseButton::Left) && draft.skin.get(texel) != colour {
      draft.skin.set(texel, colour);
      draft.touched = true
    }
    if buttons.just_pressed(MouseButton::Right) {
      draft.colour = draft.skin.get(texel)
    }
  }
}

fn obey(
  mut pressed: MessageReader<Pressed>,
  mut draft: ResMut<Draft>,
  mut paints: MessageWriter<Paint>
) {
  pressed.read().for_each(|&Pressed(act)| match act {
    Act::Swatch(colour) => draft.colour = colour,
    Act::Randomize => {
      draft.skin = Skin::fresh(random());
      draft.touched = true
    }
    Act::Revert => draft.touched = false,
    Act::Wear => {
      paints.write(Paint(draft.skin.clone()));
    }
    _ => ()
  })
}

fn show(
  draft: Res<Draft>,
  menu: Res<Menu>,
  time: Res<Time>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut viewers: Query<&mut Camera, With<Viewer>>,
  mut tables: Query<&mut Transform, With<Turntable>>,
  mut swatches: Query<(&Act, &mut BorderColor)>,
  mut unsaved: Query<&mut Text, With<Unsaved>>,
  mut drawn: Local<Option<Skin>>
) {
  if drawn.as_ref() != Some(&draft.skin) {
    *drawn = Some(draft.skin.clone());
    if let Some(mut image) = images.get_mut(&draft.raw) {
      image.data = Some(draft.skin.pixels(false))
    }
    if let Some(mut image) = images.get_mut(&draft.canvas) {
      image.data = Some(draft.skin.pixels(true))
    }
    materials.get_mut(&draft.material);
  }
  let visible = menu.showing(Tab::Skin);
  viewers.iter_mut().for_each(|mut camera| {
    if camera.is_active != visible {
      camera.is_active = visible
    }
  });
  tables.iter_mut().for_each(|mut table| {
    table.rotation = Quat::from_rotation_y(time.elapsed_secs() * 0.7)
  });
  swatches.iter_mut().for_each(|(act, mut border)| {
    if let Act::Swatch(colour) = *act {
      let lit = BorderColor::all(match colour == draft.colour {
        true => Color::WHITE,
        false => BUTTON
      });
      if *border != lit {
        *border = lit
      }
    }
  });
  let note = if draft.touched { "Unsaved: press Wear to put it on." } else { "" };
  unsaved.iter_mut().for_each(|mut text| {
    if text.0 != note {
      text.0 = note.into()
    }
  })
}

pub struct Editing;

impl Plugin for Editing {
  fn build(&self, app: &mut App) {
    app
      .add_systems(
        Startup,
        (prepare, stage.after(crate::figure::sew)).chain().run_if(plays)
      )
      .add_systems(Update, (follow, paint, obey, show).chain().run_if(plays));
  }
}
