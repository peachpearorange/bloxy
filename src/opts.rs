use {serde::Deserialize, std::sync::LazyLock};

const VAR: &str = "BLOXY";

#[derive(Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Opts {
  pub connect: Option<String>,
  pub serve: Option<u16>,
  pub host: Option<u16>,
  pub name: Option<String>,
  pub seed: u32,
  pub reach: i32,
  pub hour: f32,
  pub shot: Option<f32>,
  pub at: Option<[f32; 2]>,
  pub yaw: Option<f32>,
  pub pitch: Option<f32>,
  pub creative: bool,
  pub press: Vec<(f32, String)>,
  pub hold: Vec<(f32, f32, String)>
}

impl Default for Opts {
  fn default() -> Self {
    Self {
      connect: None,
      serve: None,
      host: None,
      name: None,
      seed: 1,
      reach: if cfg!(target_arch = "wasm32") { 6 } else { 9 },
      hour: 10.0,
      shot: None,
      at: None,
      yaw: None,
      pitch: None,
      creative: false,
      press: Vec::new(),
      hold: Vec::new()
    }
  }
}

#[cfg(not(target_arch = "wasm32"))]
fn given() -> Option<String> { std::env::var(VAR).ok() }

#[cfg(target_arch = "wasm32")]
fn given() -> Option<String> {
  web_sys::window()
    .and_then(|window| window.location().search().ok())
    .and_then(|search| web_sys::UrlSearchParams::new_with_str(&search).ok())
    .map(|params| {
      let quoted = |key: &str| {
        params.get(key).map(|value| format!("{key}: {}", serde_json_quote(&value)))
      };
      let numeric = |key: &str| params.get(key).map(|value| format!("{key}: {value}"));
      let fields = [quoted("connect"), quoted("name"), numeric("seed"), numeric("reach")]
        .into_iter()
        .flatten()
        .chain(params.get("opts"))
        .collect::<Vec<_>>();
      format!("{{{}}}", fields.join(", "))
    })
}

#[cfg(target_arch = "wasm32")]
fn serde_json_quote(text: &str) -> String {
  format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn opts() -> &'static Opts {
  static OPTS: LazyLock<Opts> = LazyLock::new(|| {
    given().filter(|text| !text.trim().is_empty()).map_or_else(Opts::default, |text| {
      json5::from_str(&text).unwrap_or_else(|blame| panic!("{VAR}={text}\n  {blame}"))
    })
  });
  &OPTS
}
