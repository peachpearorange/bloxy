use {crate::{account::Accounts,
             authority::Controller,
             beast::Herds,
             bird::{Bird, Flocks},
             block::Block,
             generate,
             island::FACING_STONE,
             opts::opts,
             protocol::*,
             shroomling::Colonies,
             voxels::{Kept, Voxels, chunk_of, pack}},
     bevy::{platform::collections::HashMap, prelude::*},
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize},
     std::sync::Arc};

pub const MOST: usize = 20;
const REFRESH_EVERY: f32 = 5.0;

#[derive(Component, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Claim {
  pub column: IVec2,
  pub owner: String,
  pub hue: u8
}

#[derive(Component)]
pub struct Holding {
  pub account: usize,
  pub snapshot: Option<Arc<Vec<u8>>>
}

#[derive(Message, Serialize, Deserialize, Clone, Copy)]
pub struct Stake(pub IVec2);

#[derive(Serialize, Deserialize, Clone)]
pub struct Deed {
  pub column: IVec2,
  pub account: usize,
  #[serde(default)]
  pub snapshot: Option<String>
}

pub fn column_of(at: IVec3) -> IVec2 { chunk_of(at).xz() }

pub type Claims<'w, 's> = Query<'w, 's, (&'static Claim, &'static Holding)>;

pub fn barred(claims: &Claims, account: usize, at: IVec3) -> Option<String> {
  let column = column_of(at);
  claims
    .iter()
    .find(|(claim, holding)| claim.column == column && holding.account != account)
    .map(|(claim, _)| format!("This land is claimed by {}", claim.owner))
}

const LETTERS: &[u8; 64] =
  b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(bytes: &[u8]) -> String {
  bytes
    .chunks(3)
    .flat_map(|group| {
      let word = group
        .iter()
        .enumerate()
        .fold(0u32, |word, (index, &byte)| word | u32::from(byte) << (16 - index * 8));
      (0..4).map(move |index| match index <= group.len() {
        true => LETTERS[(word >> (18 - index * 6) & 63) as usize] as char,
        false => '='
      })
    })
    .collect()
}

pub fn decode(text: &str) -> Vec<u8> {
  let values: Vec<u32> = text
    .bytes()
    .filter_map(|letter| LETTERS.iter().position(|&known| known == letter))
    .map(|value| value as u32)
    .collect();
  values
    .chunks(4)
    .flat_map(|group| {
      let word = group
        .iter()
        .enumerate()
        .fold(0u32, |word, (index, &value)| word | value << (18 - index * 6));
      (0..group.len().saturating_sub(1))
        .map(move |index| (word >> (16 - index * 8)) as u8)
    })
    .collect()
}

pub fn deeds(claims: &Query<(&Claim, &Holding)>) -> Vec<Deed> {
  claims
    .iter()
    .map(|(claim, holding)| Deed {
      column: claim.column,
      account: holding.account,
      snapshot: holding.snapshot.as_ref().map(|packed| encode(packed))
    })
    .collect()
}

pub fn settle(commands: &mut Commands, accounts: &Accounts, deeds: &[Deed]) {
  deeds.iter().filter(|deed| deed.account < accounts.0.len()).for_each(|deed| {
    commands.spawn((
      Replicated,
      Claim {
        column: deed.column,
        owner: accounts.0[deed.account].name.clone(),
        hue: accounts.0[deed.account].hue()
      },
      Holding {
        account: deed.account,
        snapshot: deed.snapshot.as_ref().map(|text| Arc::new(decode(text)))
      }
    ));
  })
}

fn stake(
  mut stakes: MessageReader<FromClient<Stake>>,
  players: Query<(&Controller, &Player)>,
  claims: Query<(Entity, &Claim, &Holding)>,
  mut notices: MessageWriter<ToClients<Notice>>,
  mut commands: Commands
) {
  let mut gone: Vec<Entity> = Vec::new();
  let mut made: Vec<(IVec2, usize)> = Vec::new();
  stakes.read().for_each(|&FromClient { client_id, message: Stake(column) }| {
    if let Some((controller, player)) =
      players.iter().find(|(controller, _)| controller.client == client_id)
    {
      let account = controller.account;
      let held = claims
        .iter()
        .find(|(entity, claim, _)| claim.column == column && !gone.contains(entity));
      let fresh = made.iter().any(|&(taken, _)| taken == column);
      let mine = claims
        .iter()
        .filter(|(entity, _, holding)| {
          holding.account == account && !gone.contains(entity)
        })
        .count()
        + made.iter().filter(|&&(_, owner)| owner == account).count();
      let word = match held {
        Some((entity, _, holding)) if holding.account == account => {
          gone.push(entity);
          commands.entity(entity).despawn();
          format!("Released chunk {} {}", column.x, column.y)
        }
        Some((_, claim, _)) => {
          format!("Chunk {} {} belongs to {}", column.x, column.y, claim.owner)
        }
        None if fresh => "Already claimed".into(),
        None if mine >= MOST => format!("You can claim at most {MOST} chunks"),
        None => {
          made.push((column, account));
          commands.spawn((
            Replicated,
            Claim { column, owner: player.name.clone(), hue: player.hue },
            Holding { account, snapshot: None }
          ));
          format!("Claimed chunk {} {} ({}/{MOST})", column.x, column.y, mine + 1)
        }
      };
      notices.write(ToClients {
        targets: SendTargets::Single(client_id),
        message: Notice(word)
      });
    }
  })
}

fn rename(
  players: Query<(&Controller, &Player), Changed<Player>>,
  mut claims: Query<(&mut Claim, &Holding)>
) {
  players.iter().for_each(|(controller, player)| {
    claims
      .iter_mut()
      .filter(|(claim, holding)| {
        holding.account == controller.account
          && (claim.owner != player.name || claim.hue != player.hue)
      })
      .for_each(|(mut claim, _)| {
        claim.owner = player.name.clone();
        claim.hue = player.hue
      })
  })
}

fn refresh(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut claims: Query<(&Claim, &mut Holding)>,
  mut since: Local<f32>
) {
  *since += time.delta_secs();
  if *since >= REFRESH_EVERY {
    *since = 0.0;
    let touched = std::mem::take(&mut voxels.touched);
    claims
      .iter_mut()
      .filter(|(claim, holding)| {
        holding.snapshot.is_none() || touched.contains(&claim.column)
      })
      .for_each(|(claim, mut holding)| {
        holding.snapshot = Some(Arc::new(pack(&voxels.snapshot(claim.column))))
      });
  }
}

pub fn kept_from(deeds: &[Deed]) -> Kept {
  Arc::new(
    deeds
      .iter()
      .filter_map(|deed| {
        deed
          .snapshot
          .as_ref()
          .and_then(|text| crate::voxels::unpack(&decode(text)))
          .map(|blocks| (deed.column, Arc::new(blocks)))
      })
      .collect::<HashMap<_, _>>()
  )
}

#[cfg_attr(target_arch = "wasm32", expect(dead_code))]
pub fn regen(world: &mut World, seed: Option<u32>) -> String {
  let columns: Vec<IVec2> =
    world.query::<&Claim>().iter(world).map(|claim| claim.column).collect();
  let mut voxels = world.resource_mut::<Voxels>();
  let kept: HashMap<IVec2, Arc<Vec<Block>>> =
    columns.iter().map(|&column| (column, Arc::new(voxels.snapshot(column)))).collect();
  let old = voxels.seed;
  let seed = seed.unwrap_or(old);
  let mut fresh = Voxels::new(seed);
  fresh.kept = Arc::new(kept);
  fresh.ensure(generate::spawn_point(seed).floor().as_ivec3());
  let welcome = Welcome { seed, edits: Vec::new(), kept: fresh.all_kept() };
  world.insert_resource(fresh);
  let doomed: Vec<Entity> = world
    .query_filtered::<Entity, Or<(With<Beast>, With<Bird>, With<Hopper>)>>()
    .iter(world)
    .collect();
  doomed.iter().for_each(|&entity| {
    world.despawn(entity);
  });
  world.insert_resource(Herds::default());
  world.insert_resource(Flocks::default());
  world.insert_resource(Colonies::default());
  world.write_message(ToClients { targets: SendTargets::All, message: welcome });
  if seed != old {
    let arrival = Avatar {
      at: generate::spawn_point(seed),
      yaw: FACING_STONE,
      pitch: 0.0,
      held: None
    };
    let players: Vec<(Entity, ClientId)> = world
      .query::<(Entity, &Controller)>()
      .iter(world)
      .map(|(entity, controller)| (entity, controller.client))
      .collect();
    players.into_iter().for_each(|(entity, client)| {
      world.entity_mut(entity).insert(arrival);
      world.write_message(ToClients {
        targets: SendTargets::Single(client),
        message: Teleport(arrival)
      });
    })
  }
  format!(
    "regenerated with seed {seed}: {} claimed columns kept, {} mobs cleared",
    columns.len(),
    doomed.len()
  )
}

pub struct Claiming;

impl Plugin for Claiming {
  fn build(&self, app: &mut App) {
    app
      .replicate::<Claim>()
      .add_client_message::<Stake>(Channel::Ordered)
      .add_systems(
        PreUpdate,
        (stake, rename).chain().after(ServerSystems::Receive).run_if(authority)
      )
      .add_systems(
        Update,
        refresh
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
          .run_if(|| opts().saved_at().is_some())
      );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn base64_round_trips() {
    (0..20).for_each(|length| {
      let bytes: Vec<u8> = (0..length).map(|index| (index * 37 + 11) as u8).collect();
      assert_eq!(decode(&encode(&bytes)), bytes)
    })
  }
}
