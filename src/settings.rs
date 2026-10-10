use {crate::{local, opts::opts, player::Eye, protocol::plays},
     bevy::prelude::*,
     serde::{Deserialize, Serialize}};

const KEY: &str = "bloxy.settings";

#[derive(Resource, Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Settings {
  pub reach: i32,
  pub sensitivity: f32,
  pub fov: f32,
  pub invert: bool
}

impl Default for Settings {
  fn default() -> Self {
    Self { reach: opts().reach, sensitivity: 1.0, fov: 75.0, invert: false }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Knob {
  Reach,
  Sensitivity,
  Fov,
  Invert
}

impl Knob {
  pub const ALL: [Knob; 4] = [Knob::Reach, Knob::Sensitivity, Knob::Fov, Knob::Invert];

  pub fn label(self) -> &'static str {
    match self {
      Knob::Reach => "View distance",
      Knob::Sensitivity => "Mouse sensitivity",
      Knob::Fov => "Field of view",
      Knob::Invert => "Invert mouse"
    }
  }
}

impl Settings {
  fn recalled() -> Settings {
    opts()
      .shot
      .is_none()
      .then(|| local::recall(KEY))
      .flatten()
      .and_then(|text| serde_json::from_str(&text).ok())
      .unwrap_or_default()
  }

  pub fn turn(&mut self, knob: Knob, by: i32) {
    match knob {
      Knob::Reach => self.reach = (self.reach + by).clamp(2, 16),
      Knob::Sensitivity => {
        self.sensitivity = (self.sensitivity + by as f32 * 0.25).clamp(0.25, 3.0)
      }
      Knob::Fov => self.fov = (self.fov + by as f32 * 5.0).clamp(50.0, 110.0),
      Knob::Invert => self.invert = !self.invert
    }
  }

  pub fn reading(&self, knob: Knob) -> String {
    match knob {
      Knob::Reach => format!("{} chunks", self.reach),
      Knob::Sensitivity => format!("{:.2}x", self.sensitivity),
      Knob::Fov => format!("{:.0} degrees", self.fov),
      Knob::Invert => if self.invert { "On" } else { "Off" }.into()
    }
  }
}

fn apply(settings: Res<Settings>, mut eyes: Query<&mut Projection, With<Eye>>) {
  if settings.is_changed() {
    for mut projection in eyes.iter_mut() {
      if let Projection::Perspective(perspective) = projection.as_mut() {
        perspective.fov = settings.fov.to_radians()
      }
    }
    if opts().shot.is_none()
      && let Ok(text) = serde_json::to_string(&*settings)
    {
      local::remember(KEY, &text)
    }
  }
}

pub struct Tuning;

impl Plugin for Tuning {
  fn build(&self, app: &mut App) {
    app.insert_resource(Settings::recalled()).add_systems(Update, apply.run_if(plays));
  }
}
