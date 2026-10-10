#![recursion_limit = "256"]

mod account;
mod authority;
mod block;
mod crafting;
mod editor;
mod figure;
mod flow;
mod generate;
mod hud;
mod identity;
mod island;
mod local;
mod menu;
mod mesh;
mod model;
mod net;
mod noise;
mod opts;
mod player;
mod protocol;
mod recipe;
mod save;
mod settings;
mod shroomling;
mod skin;
mod sky;
mod stream;
mod texture;
mod voxels;
mod waystone;

use {bevy::{app::ScheduleRunnerPlugin,
            camera::{Viewport, visibility::RenderLayers},
            prelude::*,
            render::{Render, RenderApp, RenderSystems,
                     render_resource::PipelineCache,
                     view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk}},
            state::app::StatesPlugin},
     bevy_replicon::prelude::*,
     opts::opts,
     protocol::Role,
     std::time::Duration};

const WARM_UP_FRAMES: u32 = 4;
const WARM_UP_LIMIT: u32 = 60;

#[derive(Resource, Clone, Default)]
struct Compiling(std::sync::Arc<std::sync::atomic::AtomicUsize>);

fn count_compiling(compiling: Res<Compiling>, cache: Res<PipelineCache>) {
  compiling
    .0
    .store(cache.waiting_pipelines().count(), std::sync::atomic::Ordering::Relaxed)
}

fn watch_compiling(app: &mut App) {
  let compiling = Compiling::default();
  app.insert_resource(compiling.clone());
  app
    .sub_app_mut(RenderApp)
    .insert_resource(compiling)
    .add_systems(Render, count_compiling.in_set(RenderSystems::Cleanup));
}
const UNSEEN: usize = 7;
const TICKS: f64 = 60.0;

fn role() -> Role {
  match (opts().connect.is_some(), opts().serve.is_some(), opts().host.is_some()) {
    (true, _, _) => Role::Guest,
    (_, true, _) => Role::Dedicated,
    (_, _, true) => Role::Host,
    _ => Role::Solo
  }
}

fn snapshot(
  time: Res<Time>,
  progress: Res<stream::Progress>,
  pilot: Option<Res<player::Pilot>>,
  mut warmed: Local<Option<u32>>,
  mut taken: Local<bool>,
  mut cameras: Query<(Entity, &mut Camera), With<player::Eye>>,
  compiling: Res<Compiling>,
  mut commands: Commands
) {
  if let Some(at) = opts().shot {
    let settled = time.elapsed_secs() > at
      && progress.pending == 0
      && (pilot.is_some() || opts().menu.is_some());
    if time.elapsed_secs() as u32 != (time.elapsed_secs() - time.delta_secs()) as u32 {
      info!("shot waits: {} pending, pilot {}", progress.pending, pilot.is_some())
    }
    *warmed = warmed.map(|frames| frames + 1).or(settled.then_some(0));
    let drawing = warmed.is_some();
    cameras
      .iter_mut()
      .filter(|(_, camera)| camera.viewport.is_none() != drawing)
      .for_each(|(entity, mut camera)| {
        camera.viewport =
          (!drawing).then(|| Viewport { physical_size: UVec2::ONE, ..default() });
        match drawing {
          true => commands.entity(entity).remove::<RenderLayers>(),
          false => commands.entity(entity).insert(RenderLayers::layer(UNSEEN))
        };
      });
    let compiled = compiling.0.load(std::sync::atomic::Ordering::Relaxed) == 0;
    let ready = warmed.is_some_and(|frames| {
      (frames >= WARM_UP_FRAMES && compiled) || frames >= WARM_UP_LIMIT
    });
    if !*taken && ready {
      *taken = true;
      let mut save = save_to_disk(format!(
        "screenshots/shot-{}.png",
        std::env::var("SHOT_NAME").unwrap_or("latest".into())
      ));
      commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>| {
          save(captured);
          std::process::exit(0)
        }
      );
    }
  }
}

fn press(
  time: Res<Time>,
  mut keys: ResMut<ButtonInput<KeyCode>>,
  mut buttons: ResMut<ButtonInput<MouseButton>>
) {
  let (now, before) = (time.elapsed_secs(), time.elapsed_secs() - time.delta_secs());
  let taps = opts().press.iter().map(|&(at, ref name)| (at, at + 0.15, name));
  let holds = opts().hold.iter().map(|&(from, to, ref name)| (from, to, name));
  taps.chain(holds).for_each(|(from, to, name)| {
    let (start, stop) = (before < from && from <= now, before < to && to <= now);
    let key = match name.as_str() {
      "LMB" => Err(MouseButton::Left),
      "RMB" => Err(MouseButton::Right),
      "Space" => Ok(KeyCode::Space),
      "Shift" => Ok(KeyCode::ShiftLeft),
      "Ctrl" => Ok(KeyCode::ControlLeft),
      "W" => Ok(KeyCode::KeyW),
      "A" => Ok(KeyCode::KeyA),
      "S" => Ok(KeyCode::KeyS),
      "D" => Ok(KeyCode::KeyD),
      "Tab" => Ok(KeyCode::Tab),
      "Esc" => Ok(KeyCode::Escape),
      "E" => Ok(KeyCode::KeyE),
      digit => Ok(
        [
          KeyCode::Digit1,
          KeyCode::Digit2,
          KeyCode::Digit3,
          KeyCode::Digit4,
          KeyCode::Digit5,
          KeyCode::Digit6,
          KeyCode::Digit7,
          KeyCode::Digit8,
          KeyCode::Digit9
        ][digit.parse::<usize>().unwrap_or(1).clamp(1, 9) - 1]
      )
    };
    match (key, start, stop) {
      (Ok(key), true, _) => keys.press(key),
      (Ok(key), _, true) => keys.release(key),
      (Err(button), true, _) => buttons.press(button),
      (Err(button), _, true) => buttons.release(button),
      _ => ()
    }
  })
}

fn serve(mut commands: Commands) {
  #[cfg(not(target_arch = "wasm32"))]
  if let Some(port) = opts().serve.or(opts().host) {
    match net::server::Listener::open(port) {
      Ok(listener) => commands.insert_resource(listener),
      Err(blame) => error!("could not listen on port {port}: {blame}")
    }
  }
  let _ = &mut commands;
}

fn forget_dirt(mut voxels: ResMut<voxels::Voxels>) {
  if !voxels.dirty.is_empty() {
    voxels.dirty.clear()
  }
}

fn main() {
  let role = role();
  let mut app = App::new();
  match role {
    Role::Dedicated => app.add_plugins((
      MinimalPlugins
        .set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / TICKS))),
      StatesPlugin,
      bevy::log::LogPlugin::default()
    )),
    _ => app.add_plugins(DefaultPlugins.set(WindowPlugin {
      primary_window: Some(Window {
        title: "Bloxy".into(),
        fit_canvas_to_parent: true,
        ..default()
      }),
      ..default()
    }))
  };
  app
    .insert_resource(role)
    .add_plugins((
      RepliconPlugins,
      protocol::Protocol,
      net::ClientNet,
      authority::Authority,
      save::Saving,
      flow::Flowing,
      shroomling::Shroomlings
    ))
    .add_systems(Startup, serve);
  #[cfg(not(target_arch = "wasm32"))]
  app.add_plugins(net::server::ServerNet);
  match role {
    Role::Dedicated => {
      app.add_systems(Update, forget_dirt.run_if(resource_exists::<voxels::Voxels>))
    }
    _ => app
      .add_plugins((
        stream::Stream,
        player::Piloting,
        figure::Figures,
        hud::Hud,
        sky::Sky,
        settings::Tuning,
        identity::Identifying,
        menu::Menus,
        editor::Editing,
        waystone::Waystones,
        crafting::Crafting
      ))
      .add_systems(PreUpdate, press.after(bevy::input::InputSystems))
      .add_systems(Last, snapshot)
  };
  if role.plays() {
    watch_compiling(&mut app)
  }
  app.run();
}
