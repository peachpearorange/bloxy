use {crate::{protocol::{Avatar, Bookmarks, Inventory, Visited},
             skin::Skin},
     bevy::prelude::*,
     serde::{Deserialize, Serialize},
     sha2::{Digest, Sha256},
     std::hash::{BuildHasher, RandomState}};

const ROUNDS: u32 = 20_000;
pub const LONGEST_NAME: usize = 24;

#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
  pub name: String,
  pub salt: String,
  pub hash: String,
  pub avatar: Option<Avatar>,
  pub inventory: Inventory,
  pub skin: Skin,
  #[serde(default)]
  pub visited: Visited,
  #[serde(default)]
  pub bookmarks: Bookmarks
}

pub fn random() -> u64 { RandomState::new().hash_one(std::time::SystemTime::now()) }

fn digest(salt: &str, password: &str) -> String {
  let seed = Sha256::digest(format!("{salt}\u{0}{password}"));
  let stretched = (0..ROUNDS).fold(seed, |state, _| {
    Sha256::new().chain_update(state).chain_update(salt).chain_update(password).finalize()
  });
  stretched.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub type Kept<'a> = (&'a Avatar, &'a Inventory, &'a Skin, &'a Visited, &'a Bookmarks);

impl Account {
  pub fn open(name: String, password: &str, inventory: Inventory) -> Account {
    let salt = format!("{:016x}{:016x}", random(), random());
    Account {
      hash: digest(&salt, password),
      salt,
      name,
      avatar: None,
      inventory,
      skin: Skin::fresh(random()),
      visited: default(),
      bookmarks: default()
    }
  }

  pub fn keep(&mut self, (&avatar, inventory, skin, visited, bookmarks): Kept) {
    self.avatar = Some(avatar);
    self.inventory = inventory.clone();
    self.skin = skin.clone();
    self.visited = visited.clone();
    self.bookmarks = bookmarks.clone()
  }

  pub fn admits(&self, password: &str) -> bool {
    digest(&self.salt, password) == self.hash
  }

  pub fn lock(&mut self, password: &str) {
    self.salt = format!("{:016x}{:016x}", random(), random());
    self.hash = digest(&self.salt, password)
  }
}

pub fn tidy(name: &str) -> Option<String> {
  let kept: String = name.chars().filter(|c| !c.is_control()).collect();
  let trimmed = kept.trim();
  (!trimmed.is_empty() && trimmed.chars().count() <= LONGEST_NAME)
    .then(|| trimmed.to_string())
}

#[derive(Resource, Default)]
pub struct Accounts(pub Vec<Account>);

impl Accounts {
  pub fn named(&self, name: &str) -> Option<usize> {
    self.0.iter().position(|account| account.name.to_lowercase() == name.to_lowercase())
  }
}
