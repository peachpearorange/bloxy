use {crate::{block::Block, opts::opts, protocol::*, voxels::Voxels},
     bevy::prelude::*,
     serde::{Deserialize, Serialize},
     std::{collections::HashMap,
           sync::atomic::{AtomicBool, Ordering}}};

const EVERY: f32 = 5.0;

pub type Belongings = (Avatar, Inventory);

#[derive(Resource, Default)]
pub struct Roster(pub HashMap<String, Belongings>);

#[derive(Serialize, Deserialize)]
pub struct World {
  pub seed: u32,
  pub edits: Vec<(IVec3, Block)>,
  pub players: HashMap<String, Belongings>
}

pub fn load() -> Option<World> {
  opts().saved_at().and_then(|path| {
    std::fs::read_to_string(path).ok().map(|text| {
      info!("loading the world from {path}");
      json5::from_str(&text)
        .unwrap_or_else(|blame| panic!("{path} is unreadable: {blame}"))
    })
  })
}

static TERMINATED: AtomicBool = AtomicBool::new(false);

fn write(path: &str, world: &World) -> std::io::Result<()> {
  let draft = format!("{path}.new");
  let text = serde_json::to_string(world)?;
  std::fs::write(&draft, text)?;
  std::fs::rename(&draft, path)
}

fn store(
  time: Res<Time>,
  voxels: Res<Voxels>,
  roster: Res<Roster>,
  players: Query<(&Player, &Avatar, &Inventory)>,
  altered: Query<(), Or<(Changed<Avatar>, Changed<Inventory>)>>,
  mut exits: MessageReader<AppExit>,
  mut commands: Commands,
  mut pending: Local<bool>,
  mut since: Local<f32>
) {
  let terminated = TERMINATED.load(Ordering::Relaxed);
  let closing = exits.read().count() > 0 || terminated;
  *pending |= voxels.is_changed() || roster.is_changed() || !altered.is_empty();
  *since += time.delta_secs();
  if let Some(path) = opts().saved_at()
    && *pending
    && (*since >= EVERY || closing)
  {
    let world = World {
      seed: voxels.seed,
      edits: voxels.all_edits(),
      players: roster
        .0
        .clone()
        .into_iter()
        .chain(players.iter().map(|(player, &avatar, inventory)| {
          (player.name.clone(), (avatar, inventory.clone()))
        }))
        .collect()
    };
    match write(path, &world) {
      Ok(()) => *pending = false,
      Err(blame) => error!("could not save the world to {path}: {blame}")
    }
    *since = 0.0
  }
  if terminated {
    TERMINATED.store(false, Ordering::Relaxed);
    commands.write_message(AppExit::Success);
  }
}

pub struct Saving;

impl Plugin for Saving {
  fn build(&self, app: &mut App) {
    if opts().saved_at().is_some() {
      #[cfg(not(target_arch = "wasm32"))]
      ctrlc::set_handler(|| TERMINATED.store(true, Ordering::Relaxed))
        .unwrap_or_else(|blame| warn!("could not catch termination: {blame}"));
      app.add_systems(Last, store.run_if(authority).run_if(resource_exists::<Voxels>));
    }
  }
}
