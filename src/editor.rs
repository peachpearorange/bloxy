use {crate::{account::random,
             figure::Kit,
             menu::{Act, BUTTON, FAINT, Menu, Pressed, Tab, button, words},
             player::Pilot,
             protocol::{Paint, plays},
             skin::{PALETTE, PX, Part, Skin}},
     bevy::{camera::{RenderTarget, visibility::RenderLayers},
            core_pipeline::tonemapping::Tonemapping,
            input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
            prelude::*,
            render::render_resource::TextureFormat,
            ui::RelativeCursorPosition}};

const PREVIEW: usize = 3;
const VIEW: UVec2 = UVec2::new(600, 640);
const PIVOT: Vec3 = Vec3::new(0.0, 0.95, 0.0);
const NEAREST: f32 = 1.6;
const FARTHEST: f32 = 5.0;
const PIECES: [(Part, Vec3, f32); 6] = [
  (Part::Head, Vec3::new(0.0, 24.0, 0.0), 4.0),
  (Part::Body, Vec3::new(0.0, 18.0, 0.0), 0.0),
  (Part::Arm, Vec3::new(6.0, 24.0, 0.0), -6.0),
  (Part::Arm, Vec3::new(-6.0, 24.0, 0.0), -6.0),
  (Part::Leg, Vec3::new(2.0, 12.0, 0.0), -6.0),
  (Part::Leg, Vec3::new(-2.0, 12.0, 0.0), -6.0)
];

#[derive(Resource)]
pub struct Draft {
  pub skin: Skin,
  pub colour: u8,
  pub touched: bool,
  raw: Handle<Image>,
  view: Handle<Image>,
  material: Handle<StandardMaterial>,
  ghost: Handle<StandardMaterial>
}

#[derive(Clone, Copy)]
enum Grip {
  Turning(Vec2),
  Spinning { from: Vec2, start: Vec2 },
  Painting
}

#[derive(Component)]
struct Ghostly;

#[derive(Resource)]
struct Orbit {
  yaw: f32,
  pitch: f32,
  distance: f32,
  grip: Option<Grip>
}

#[derive(Component)]
struct Piece {
  part: Part,
  centre: Vec3
}

impl Piece {
  fn half(&self) -> Vec3 { self.part.size().as_vec3() * PX / 2.0 }

  fn struck(&self, from: Vec3, toward: Vec3) -> Option<(f32, Vec3)> {
    let (low, high) = (self.centre - self.half(), self.centre + self.half());
    let (near, far) = ((low - from) / toward, (high - from) / toward);
    let (entry, exit) = (near.min(far), near.max(far));
    let (enter, leave) = (entry.max_element(), exit.min_element());
    let axis = (0..3).find(|&axis| entry[axis] == enter).unwrap_or(0);
    let mut normal = Vec3::ZERO;
    normal[axis] = -toward[axis].signum();
    (enter <= leave && enter > 0.0).then_some((enter, normal))
  }

  fn texel(&self, point: Vec3, normal: Vec3) -> Option<UVec2> {
    let size = self.part.size().as_vec3() * PX;
    let extent = |axis: Vec3| axis.abs().dot(size);
    let offset = point - self.centre;
    self.part.faces().into_iter().find(|(face, _)| face.axes().0 == normal).map(
      |(face, rect)| {
        let (_, across, down) = face.axes();
        let spot =
          Vec2::new(offset.dot(across) / extent(across), offset.dot(down) / extent(down))
            + 0.5;
        (rect.min.as_vec2() + spot * rect.size().as_vec2())
          .floor()
          .as_uvec2()
          .clamp(rect.min, rect.max - 1)
      }
    )
  }
}

#[derive(Component)]
struct Unsaved;

#[derive(Component)]
struct Easel;

#[derive(Component)]
struct Viewer;

#[derive(Component)]
struct Lamp;

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
  let ghost = materials.add(StandardMaterial {
    base_color: Color::srgba(1.0, 1.0, 1.0, 0.12),
    base_color_texture: Some(raw.clone()),
    alpha_mode: AlphaMode::Blend,
    perceptual_roughness: 0.85,
    ..default()
  });
  commands.insert_resource(Draft {
    view: images.add(Image::new_target_texture(
      VIEW.x,
      VIEW.y,
      TextureFormat::Rgba8UnormSrgb,
      None
    )),
    raw,
    material,
    ghost,
    skin,
    colour: 0,
    touched: false
  });
  commands.insert_resource(Orbit { yaw: 0.6, pitch: 0.15, distance: 4.0, grip: None })
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
    bevy::camera::Exposure { ev100: 13.0 },
    AmbientLight { brightness: 2200.0, ..default() },
    layers.clone(),
    Transform::default()
  ));
  commands.spawn((
    Lamp,
    DirectionalLight { illuminance: 6500.0, shadow_maps_enabled: false, ..default() },
    layers.clone(),
    Transform::default()
  ));
  PIECES.into_iter().for_each(|(part, at, lift)| {
    let mesh = match part {
      Part::Head => kit.head.clone(),
      Part::Body => kit.body.clone(),
      Part::Arm => kit.arm.clone(),
      Part::Leg => kit.leg.clone()
    };
    commands.spawn((
      Piece { part, centre: (at + Vec3::Y * lift) * PX },
      Mesh3d(mesh),
      MeshMaterial3d(draft.material.clone()),
      layers.clone(),
      Transform::from_translation(at * PX),
      Visibility::default()
    ));
  });
}

pub fn page(page: &mut ChildSpawnerCommands, draft: &Draft) {
  page
    .spawn(Node { column_gap: px(20), flex_grow: 1.0, min_height: px(0), ..default() })
    .with_children(|row| {
      row
        .spawn(Node {
          flex_direction: FlexDirection::Column,
          row_gap: px(8),
          width: px(380),
          flex_shrink: 0.0,
          ..default()
        })
        .with_children(|left| {
          left.spawn(words("Colours", 15.0, FAINT));
          left.spawn(Node { flex_wrap: FlexWrap::Wrap, width: px(380), ..default() }).with_children(
            |palette| {
              PALETTE.iter().enumerate().for_each(|(index, &[r, g, b])| {
                palette.spawn((
                  Button,
                  Act::Swatch(index as u8),
                  Node { width: px(46), height: px(36), border: UiRect::all(px(3)), ..default() },
                  BorderColor::all(BUTTON),
                  BackgroundColor(Color::srgb_u8(r, g, b))
                ));
              })
            }
          );
          left.spawn(Node { column_gap: px(8), row_gap: px(6), flex_wrap: FlexWrap::Wrap, ..default() }).with_children(
            |buttons| {
              button(buttons, "Random", Act::Randomize);
              button(buttons, "Undo all", Act::Revert);
              button(buttons, "Show all parts", Act::Unhide);
              button(buttons, "Wear", Act::Wear);
            }
          );
          left.spawn((Unsaved, words("", 14.0, FAINT)));
          left.spawn(words(
            "Left mouse paints on the figure. Right-drag (or left-drag beside it) turns it. \
             Right-click a body part to fade it out and reach what it covers, again to bring \
             it back. Middle-click picks a colour, the wheel zooms. Both arms share one \
             pattern, as do both legs.",
            14.0,
            FAINT
          ));
        });
      row.spawn((
        Easel,
        ImageNode::new(draft.view.clone()),
        RelativeCursorPosition::default(),
        Interaction::default(),
        Node {
          height: percent(100),
          aspect_ratio: Some(VIEW.x as f32 / VIEW.y as f32),
          ..default()
        }
      ));
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

fn brush(
  buttons: Res<ButtonInput<MouseButton>>,
  wheel: Res<AccumulatedMouseScroll>,
  menu: Res<Menu>,
  windows: Query<&RelativeCursorPosition, With<Easel>>,
  viewers: Query<(&Camera, &GlobalTransform), With<Viewer>>,
  pieces: Query<(Entity, &Piece, Has<Ghostly>)>,
  mut orbit: ResMut<Orbit>,
  mut draft: ResMut<Draft>,
  mut commands: Commands
) {
  if menu.showing(Tab::Skin)
    && let Ok(cursor) = windows.single()
    && let Ok((camera, eye)) = viewers.single()
  {
    let spot = cursor.normalized.filter(|_| cursor.cursor_over);
    let ray = spot
      .and_then(|spot| camera.viewport_to_world(eye, (spot + 0.5) * VIEW.as_vec2()).ok());
    let nearest = |ghosts: bool| {
      ray.and_then(|ray| {
        pieces
          .iter()
          .filter(|&(.., ghostly)| ghosts || !ghostly)
          .filter_map(|(entity, piece, ghostly)| {
            piece.struck(ray.origin, *ray.direction).and_then(|(distance, normal)| {
              piece
                .texel(ray.origin + *ray.direction * distance, normal)
                .map(|texel| (distance, entity, ghostly, texel))
            })
          })
          .min_by(|a, b| a.0.total_cmp(&b.0))
      })
    };
    let hit = nearest(false);
    if let Some(spot) = spot {
      if buttons.just_pressed(MouseButton::Left) {
        orbit.grip = Some(match hit {
          Some(_) => Grip::Painting,
          None => Grip::Turning(spot)
        })
      }
      if buttons.just_pressed(MouseButton::Right) {
        orbit.grip = Some(Grip::Spinning { from: spot, start: spot })
      }
    }
    if let Some(Grip::Spinning { start, .. }) = orbit.grip
      && buttons.just_released(MouseButton::Right)
      && spot.is_some_and(|spot| (spot - start).length() < 0.01)
      && let Some((_, piece, ghostly, _)) = nearest(true)
    {
      let mut chosen = commands.entity(piece);
      match ghostly {
        true => chosen.remove::<Ghostly>().insert(MeshMaterial3d(draft.material.clone())),
        false => chosen.insert((Ghostly, MeshMaterial3d(draft.ghost.clone())))
      };
    }
    if !buttons.any_pressed([MouseButton::Left, MouseButton::Right]) {
      orbit.grip = None
    }
    let turn = |orbit: &mut Orbit, moved: Vec2| {
      orbit.yaw -= moved.x * 5.0;
      orbit.pitch = (orbit.pitch + moved.y * 3.0).clamp(-1.3, 1.3)
    };
    match (orbit.grip, spot, hit) {
      (Some(Grip::Painting), _, Some((.., texel))) => {
        let colour = draft.colour;
        if draft.skin.get(texel) != colour {
          draft.skin.set(texel, colour);
          draft.touched = true
        }
      }
      (Some(Grip::Turning(from)), Some(spot), _) => {
        turn(&mut orbit, spot - from);
        orbit.grip = Some(Grip::Turning(spot))
      }
      (Some(Grip::Spinning { from, start }), Some(spot), _) => {
        turn(&mut orbit, spot - from);
        orbit.grip = Some(Grip::Spinning { from: spot, start })
      }
      _ => ()
    }
    if let Some((.., texel)) = hit
      && buttons.just_pressed(MouseButton::Middle)
    {
      draft.colour = draft.skin.get(texel)
    }
    let lines = match wheel.unit {
      MouseScrollUnit::Line => wheel.delta.y,
      MouseScrollUnit::Pixel => wheel.delta.y / 40.0
    };
    if spot.is_some() && lines != 0.0 {
      orbit.distance = (orbit.distance * 0.9f32.powf(lines)).clamp(NEAREST, FARTHEST)
    }
  }
}

fn obey(
  mut pressed: MessageReader<Pressed>,
  mut draft: ResMut<Draft>,
  pieces: Query<Entity, With<Ghostly>>,
  mut commands: Commands,
  mut paints: MessageWriter<Paint>
) {
  pressed.read().for_each(|&Pressed(act)| match act {
    Act::Swatch(colour) => draft.colour = colour,
    Act::Randomize => {
      draft.skin = Skin::fresh(random());
      draft.touched = true
    }
    Act::Revert => draft.touched = false,
    Act::Unhide => pieces.iter().for_each(|piece| {
      commands
        .entity(piece)
        .remove::<Ghostly>()
        .insert(MeshMaterial3d(draft.material.clone()));
    }),
    Act::Wear => {
      paints.write(Paint(draft.skin.clone()));
    }
    _ => ()
  })
}

fn show(
  draft: Res<Draft>,
  menu: Res<Menu>,
  orbit: Res<Orbit>,
  mut images: ResMut<Assets<Image>>,
  mut materials: ResMut<Assets<StandardMaterial>>,
  mut viewers: Query<(&mut Camera, &mut Transform), (With<Viewer>, Without<Lamp>)>,
  mut lamps: Query<&mut Transform, (With<Lamp>, Without<Viewer>)>,
  mut swatches: Query<(&Act, &mut BorderColor)>,
  mut unsaved: Query<&mut Text, With<Unsaved>>,
  mut drawn: Local<Option<Skin>>
) {
  if drawn.as_ref() != Some(&draft.skin) {
    *drawn = Some(draft.skin.clone());
    if let Some(mut image) = images.get_mut(&draft.raw) {
      image.data = Some(draft.skin.pixels(false))
    }
    materials.get_mut(&draft.material);
    materials.get_mut(&draft.ghost);
  }
  let visible = menu.showing(Tab::Skin);
  let turned = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
  let placed = Transform::from_translation(PIVOT + turned * Vec3::NEG_Z * orbit.distance)
    .looking_at(PIVOT, Vec3::Y);
  viewers.iter_mut().for_each(|(mut camera, mut transform)| {
    if camera.is_active != visible {
      camera.is_active = visible
    }
    transform.set_if_neq(placed);
  });
  lamps.iter_mut().for_each(|mut lamp| {
    lamp.set_if_neq(
      Transform::from_translation(
        placed.translation + placed.right() * 2.5 + Vec3::Y * 3.0
      )
      .looking_at(PIVOT, Vec3::Y)
    );
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
      .add_systems(Update, (follow, brush, obey, show).chain().run_if(plays));
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rays_find_the_texel_they_touch() {
    let body = Piece { part: Part::Body, centre: Vec3::new(0.0, 18.0, 0.0) * PX };
    let (distance, normal) =
      body.struck(Vec3::new(0.0, 18.0 * PX, -3.0), Vec3::Z).unwrap();
    assert_eq!(normal, Vec3::NEG_Z);
    let texel =
      body.texel(Vec3::new(0.0, 18.0 * PX, -3.0) + Vec3::Z * distance, normal).unwrap();
    assert_eq!(texel, UVec2::new(24, 26));
    let head = Piece { part: Part::Head, centre: Vec3::new(0.0, 28.0, 0.0) * PX };
    let (distance, normal) =
      head.struck(Vec3::new(3.5 * PX, 3.0, PX), Vec3::NEG_Y).unwrap();
    assert_eq!(normal, Vec3::Y);
    assert!(
      head.texel(Vec3::new(3.5 * PX, 3.0, PX) + Vec3::NEG_Y * distance, normal).is_some()
    );
    assert!(body.struck(Vec3::new(1.0, 18.0 * PX, -3.0), Vec3::Z).is_none())
  }
}
