use {crate::{account::{Account, Accounts, Kept},
             authority::Controller,
             block::Block,
             claim::{Claim, Deed, Holding, deeds, encode},
             generate::GENERATION,
             opts::opts,
             protocol::*,
             shroomling::{Dormant, Wander},
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
  pub accounts: Vec<Account>,
  #[serde(default)]
  pub shroomlings: Vec<(IVec2, Hopper)>,
  #[serde(default)]
  pub boats: Vec<(Vec3, f32)>,
  #[serde(default)]
  pub signs: Vec<Sign>,
  #[serde(default)]
  pub claims: Vec<Deed>,
  #[serde(default)]
  pub kept: Vec<(IVec2, String)>,
  #[serde(default)]
  pub generation: Option<u32>
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
  shroomlings: Query<(&Hopper, &Wander)>,
  dormant: Res<Dormant>,
  boats: Query<&Vessel>,
  signs: Query<&Sign>,
  claims: Query<(&Claim, &Holding)>,
  altered: Query<
    (),
    Or<(
      Changed<Avatar>,
      Changed<Inventory>,
      Changed<Skin>,
      Changed<Visited>,
      Changed<Bookmarks>,
      Changed<Hopper>,
      Changed<Vessel>,
      Changed<Sign>,
      Changed<Claim>,
      Changed<Holding>
    )>
  >,
  mut released: RemovedComponents<Claim>,
  mut exits: MessageReader<AppExit>,
  mut commands: Commands,
  mut pending: Local<bool>,
  mut since: Local<f32>
) {
  let terminated = TERMINATED.load(Ordering::Relaxed);
  let closing = exits.read().count() > 0 || terminated;
  *pending |= voxels.is_changed()
    || dormant.is_changed()
    || accounts.is_changed()
    || !altered.is_empty()
    || released.read().count() > 0;
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
      ),
      shroomlings: shroomlings
        .iter()
        .map(|(&hopper, wander)| (wander.home, hopper))
        .chain(dormant.0.iter().flat_map(|(&home, sleepers)| {
          sleepers.iter().map(move |&(hopper, _)| (home, hopper))
        }))
        .collect(),
      boats: boats.iter().map(|vessel| (vessel.at, vessel.yaw)).collect(),
      signs: signs.iter().cloned().collect(),
      claims: deeds(&claims),
      kept: voxels
        .all_kept()
        .into_iter()
        .map(|(column, packed)| (column, encode(&packed)))
        .collect(),
      generation: Some(GENERATION)
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
