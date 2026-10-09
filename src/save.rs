use {crate::{account::{Account, Accounts, Kept},
             authority::Controller,
             block::Block,
             opts::opts,
             protocol::*,
             skin::Skin,
             voxels::Voxels},
     bevy::prelude::*,
     serde::{Deserialize, Serialize},
     std::sync::atomic::{AtomicBool, Ordering}};

const EVERY: f32 = 5.0;

#[derive(Serialize, Deserialize)]
pub struct World {
  pub seed: u32,
  pub edits: Vec<(IVec3, Block)>,
  #[serde(default)]
  pub accounts: Vec<Account>
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
  accounts: Res<Accounts>,
  players: Query<(&Controller, Kept)>,
  altered: Query<
    (),
    Or<(Changed<Avatar>, Changed<Inventory>, Changed<Skin>, Changed<Visited>)>
  >,
  mut exits: MessageReader<AppExit>,
  mut commands: Commands,
  mut pending: Local<bool>,
  mut since: Local<f32>
) {
  let terminated = TERMINATED.load(Ordering::Relaxed);
  let closing = exits.read().count() > 0 || terminated;
  *pending |= voxels.is_changed() || accounts.is_changed() || !altered.is_empty();
  *since += time.delta_secs();
  if let Some(path) = opts().saved_at()
    && *pending
    && (*since >= EVERY || closing)
  {
    let world = World {
      seed: voxels.seed,
      edits: voxels.all_edits(),
      accounts: players.iter().fold(
        accounts.0.clone(),
        |mut accounts, (controller, kept)| {
          accounts[controller.account].keep(kept);
          accounts
        }
      )
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
