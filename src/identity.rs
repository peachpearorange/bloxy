use {crate::{local,
             menu::{Menu, Tab},
             opts::opts,
             player::Pilot,
             protocol::{Hello, Verdict, plays},
             voxels::Voxels},
     bevy::prelude::*,
     bevy_replicon::prelude::*,
     serde::{Deserialize, Serialize}};

const KEY: &str = "bloxy.identity";
const LETTERS: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Credentials {
  pub name: String,
  pub password: String
}

impl Credentials {
  pub fn invented_password() -> String {
    local::random_bytes(12)
      .chunks(4)
      .map(|group| {
        group.iter().map(|&byte| LETTERS[byte as usize % LETTERS.len()] as char).collect()
      })
      .collect::<Vec<String>>()
      .join("-")
  }

  fn invented() -> Credentials {
    let number =
      local::random_bytes(2).iter().fold(0u32, |n, &byte| n * 256 + byte as u32);
    Credentials {
      name: format!("Wanderer{:04}", number % 10_000),
      password: Credentials::invented_password()
    }
  }

  fn recalled() -> Credentials {
    let stored = local::recall(KEY).and_then(|text| serde_json::from_str(&text).ok());
    let native = Credentials {
      name: opts().name.clone().unwrap_or_else(|| "Wanderer".into()),
      password: opts().password.clone().unwrap_or_default()
    };
    match (cfg!(target_arch = "wasm32"), stored) {
      (false, _) => native,
      (true, stored) => {
        let credentials: Credentials = stored.unwrap_or_else(Credentials::invented);
        Credentials {
          name: opts().name.clone().unwrap_or(credentials.name),
          password: opts().password.clone().unwrap_or(credentials.password)
        }
      }
    }
  }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Standing {
  Asking,
  Accepted,
  Refused(String)
}

#[derive(Resource)]
pub struct Identity {
  pub sent: Credentials,
  pub fields: Credentials,
  pub owed: bool,
  pub standing: Standing
}

impl Identity {
  pub fn submit(&mut self) {
    self.sent = self.fields.clone();
    self.owed = true;
    self.standing = Standing::Asking
  }
}

fn introduce(
  mut identity: ResMut<Identity>,
  voxels: Option<Res<Voxels>>,
  state: Res<State<ClientState>>,
  mut hellos: MessageWriter<Hello>
) {
  let reachable = voxels.is_some() && *state.get() != ClientState::Connecting;
  if identity.owed && reachable {
    identity.owed = false;
    let Credentials { name, password } = identity.sent.clone();
    hellos.write(Hello { name, password });
  }
}

fn hear(
  mut verdicts: MessageReader<Verdict>,
  mut identity: ResMut<Identity>,
  mut menu: ResMut<Menu>,
  pilot: Option<Res<Pilot>>,
  time: Res<Time>
) {
  verdicts.read().for_each(|verdict| match verdict {
    Verdict::Accepted { name } => {
      identity.sent.name = name.clone();
      identity.fields.name = name.clone();
      identity.standing = Standing::Accepted;
      if let Ok(text) = serde_json::to_string(&identity.sent) {
        local::remember(KEY, &text)
      }
    }
    Verdict::Refused { reason } => {
      identity.standing = Standing::Refused(reason.clone());
      if pilot.is_none() {
        menu.show(Tab::Profile, time.elapsed_secs())
      }
    }
  })
}

pub struct Identifying;

impl Plugin for Identifying {
  fn build(&self, app: &mut App) {
    let credentials = Credentials::recalled();
    local::listen_for_paste();
    app
      .insert_resource(Identity {
        sent: credentials.clone(),
        fields: credentials,
        owed: true,
        standing: Standing::Asking
      })
      .add_systems(
        PreUpdate,
        hear.after(ClientSystems::Receive).after(ServerSystems::Receive).run_if(plays)
      )
      .add_systems(Update, introduce.run_if(plays));
  }
}
