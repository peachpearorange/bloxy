mod output;
mod sfx;
mod synth;

pub use sfx::Cue;
use {crate::{bird::{Bird, Breed as Flier},
             block::Block,
             loose::SIZE,
             player::{Aim, Eye, Gait, Pilot, STEP_LENGTH},
             protocol::{Avatar, Beast, Breed, Folk, Gathered, Health, Loose, Teleport,
                        plays},
             skill::Skills,
             spell::Flourish,
             voxels::Voxels},
     bevy::{platform::collections::HashMap, prelude::*},
     output::{Mix, Output, Take, Voice},
     synth::{Rng, Wave}};

const MASTER: f32 = 0.8;
const DIG_EVERY: f32 = 0.24;
const CALLS_WITHIN: f32 = 40.0;
const SKY_SEARCH: i32 = 40;

#[derive(Message, Clone, Copy)]
pub struct Sound {
  pub cue: Cue,
  pub at: Option<Vec3>,
  pub volume: f32
}

impl Sound {
  pub fn at(cue: Cue, at: Vec3) -> Sound { Sound { cue, at: Some(at), volume: 1.0 } }

  pub fn here(cue: Cue) -> Sound { Sound { cue, at: None, volume: 1.0 } }

  pub fn louder(self, by: f32) -> Sound { Sound { volume: self.volume * by, ..self } }
}

pub fn breaking(block: Block) -> Cue {
  match footing(block) {
    _ if block.plant() || block.leafy() => Cue::Rustle,
    Cue::Soft => Cue::Clod,
    _ => Cue::Crumble
  }
}

pub fn footing(block: Block) -> Cue {
  match crate::gear::dug_with(block) {
    Some(Block::Shovel) => Cue::Soft,
    Some(Block::Axe) => Cue::Wood,
    _ if block.leafy() || block == Block::Wool || block.plant() => Cue::Soft,
    _ => Cue::Hard
  }
}

fn loudness(cue: Cue) -> (f32, f32) {
  match cue {
    Cue::Soft => (0.08, 6.0),
    Cue::Hard | Cue::Wood => (0.09, 6.0),
    Cue::Rustle => (0.08, 6.0),
    Cue::Clod => (0.2, 8.0),
    Cue::Crumble => (0.3, 8.0),
    Cue::Thud => (0.28, 8.0),
    Cue::Punch => (0.42, 10.0),
    Cue::Hurt => (0.4, 6.0),
    Cue::Splash => (0.35, 12.0),
    Cue::Swim => (0.12, 6.0),
    Cue::Song | Cue::Caw | Cue::Cry | Cue::Honk => (0.09, 16.0),
    Cue::Bleat => (0.14, 12.0),
    Cue::Cast => (0.28, 10.0),
    Cue::Burst => (0.3, 16.0),
    Cue::Pop => (0.16, 6.0),
    Cue::Creak => (0.3, 8.0),
    Cue::Chime => (0.25, 8.0),
    Cue::Warp => (0.28, 8.0),
    Cue::Thunder => (0.45, 160.0),
    Cue::Craft => (0.25, 6.0)
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Clip {
  Cue(Cue),
  Rain
}

impl Clip {
  fn render(self) -> Vec<Wave> {
    match self {
      Clip::Cue(cue) => (0..cue.variants())
        .map(|variant| sfx::render(cue, variant * 7919 + cue as u64))
        .collect(),
      Clip::Rain => vec![sfx::rain()]
    }
  }
}

#[derive(Resource)]
struct Unrendered(Vec<Clip>);

#[derive(Resource, Default)]
struct Library(HashMap<Clip, Vec<(Take, f32)>>);

struct Sounding {
  voice: Voice,
  at: Option<Vec3>,
  volume: f32,
  reach: f32,
  ends: f32
}

#[derive(Resource, Default)]
struct Voices(Vec<Sounding>);

#[derive(Resource, Default)]
struct Rain(Option<(Voice, f32)>);

fn open(world: &mut World) {
  world.insert_resource(Unrendered(
    Cue::ALL.into_iter().map(Clip::Cue).chain([Clip::Rain]).rev().collect()
  ));
  match Output::open() {
    Some(output) => world.insert_non_send(output),
    None => debug!("no audio output")
  }
}

fn gather(
  mut unrendered: ResMut<Unrendered>,
  mut library: ResMut<Library>,
  output: Option<NonSendMut<Output>>
) {
  if let Some(mut output) = output
    && let Some(clip) = unrendered.0.pop()
  {
    let takes = clip
      .render()
      .into_iter()
      .map(|wave| {
        let secs = wave.0[0].len() as f32 / synth::RATE;
        (output.load(wave), secs)
      })
      .collect();
    library.0.insert(clip, takes);
  }
}

fn wake(buttons: Res<ButtonInput<MouseButton>>, output: Option<NonSendMut<Output>>) {
  if buttons.get_just_pressed().next().is_some()
    && let Some(mut output) = output
  {
    output.resume()
  }
}

fn heard(from: &GlobalTransform, at: Vec3, volume: f32, reach: f32) -> Mix {
  let gap = at - from.translation();
  let distance = gap.length();
  Mix {
    gain: volume * MASTER * (reach / distance.max(reach)).powi(2),
    pan: gap.normalize_or_zero().dot(from.right().into())
  }
}

fn play(
  mut sounds: MessageReader<Sound>,
  library: Res<Library>,
  time: Res<Time<Real>>,
  eyes: Query<&GlobalTransform, With<Eye>>,
  output: Option<NonSendMut<Output>>,
  mut voices: ResMut<Voices>,
  mut rng: Local<Rng>
) {
  if let Some(mut output) = output
    && let Some(eye) = eyes.iter().next()
  {
    for &Sound { cue, at, volume: louder } in sounds.read() {
      if let Some(takes) = library.0.get(&Clip::Cue(cue))
        && !takes.is_empty()
        && let (volume, reach) = loudness(cue)
        && let volume = volume * louder
        && let (take, secs) = takes[rng.below(takes.len())]
        && let speed = rng.range(0.94, 1.06)
        && let mix =
          at.map_or(Mix::flat(volume * MASTER), |at| heard(eye, at, volume, reach))
        && mix.gain > 0.002
      {
        voices.0.push(Sounding {
          voice: output.start(take, mix, speed, false),
          at,
          volume,
          reach,
          ends: time.elapsed_secs() + secs / speed + 0.1
        })
      }
    }
  }
}

fn follow(
  time: Res<Time<Real>>,
  eyes: Query<&GlobalTransform, With<Eye>>,
  output: Option<NonSendMut<Output>>,
  mut voices: ResMut<Voices>
) {
  if let Some(mut output) = output
    && let Some(eye) = eyes.iter().next()
  {
    let now = time.elapsed_secs();
    voices.0.retain(|&Sounding { voice, at, volume, reach, ends }| {
      let alive = now < ends;
      match (alive, at) {
        (false, _) => output.stop(voice),
        (true, Some(at)) => output.set(voice, heard(eye, at, volume, reach)),
        _ => ()
      }
      alive
    })
  }
}

fn stride(
  pilot: Option<Res<Pilot>>,
  gait: Res<Gait>,
  voxels: Option<Res<Voxels>>,
  mut was: Local<(f32, bool)>,
  mut sounds: MessageWriter<Sound>
) {
  if let Some(pilot) = pilot
    && let Some(voxels) = voxels
  {
    let (last, swam) = *was;
    let feet = pilot.at - Vec3::Y * 0.05;
    let stepped = (gait.stride / STEP_LENGTH).floor() != (last / STEP_LENGTH).floor();
    if stepped && pilot.riding.is_none() {
      match (pilot.swimming, pilot.grounded) {
        (true, _) => {
          sounds.write(Sound::at(Cue::Swim, feet));
        }
        (false, true) => {
          if let Some(block) =
            voxels.block(feet.floor().as_ivec3()).filter(|block| block.solid())
          {
            sounds.write(Sound::at(footing(block), feet));
          }
        }
        _ => ()
      }
    }
    if pilot.swimming && !swam && pilot.velocity.y < -3.0 {
      sounds.write(
        Sound::at(Cue::Splash, feet).louder((-pilot.velocity.y / 10.0).clamp(0.5, 1.2))
      );
    }
    *was = (gait.stride, pilot.swimming)
  }
}

fn dig(
  time: Res<Time>,
  aim: Res<Aim>,
  mut since: Local<f32>,
  mut sounds: MessageWriter<Sound>
) {
  *since += time.delta_secs();
  match aim.hit.as_ref().filter(|hit| aim.digging == Some(hit.at)) {
    Some(hit) if *since >= DIG_EVERY => {
      *since = 0.0;
      sounds.write(Sound::at(footing(hit.block), hit.at.as_vec3() + 0.5).louder(1.8));
    }
    Some(_) => (),
    None => *since = DIG_EVERY
  }
}

fn wounds(
  pilot: Option<Res<Pilot>>,
  creatures: Query<(
    Entity,
    &Health,
    Option<&Folk>,
    Option<&Beast>,
    Option<&Bird>,
    Option<&Avatar>
  )>,
  mut known: Local<HashMap<Entity, u8>>,
  mut sounds: MessageWriter<Sound>
) {
  let me = pilot.as_ref().map(|pilot| pilot.me);
  let seen: HashMap<Entity, u8> = creatures
    .iter()
    .map(|(entity, health, folk, beast, bird, avatar)| {
      let at = folk
        .map(|folk| folk.at)
        .or(beast.map(|beast| beast.at))
        .or(bird.map(|bird| bird.at))
        .or(avatar.map(|avatar| avatar.at));
      if known.get(&entity).is_some_and(|&was| health.0 < was) {
        match (Some(entity) == me, at) {
          (true, _) => {
            sounds.write(Sound::here(Cue::Hurt));
          }
          (false, Some(at)) => {
            sounds.write(Sound::at(Cue::Punch, at + Vec3::Y * 0.8));
          }
          _ => ()
        }
      }
      (entity, health.0)
    })
    .collect();
  *known = seen
}

fn calls(
  time: Res<Time>,
  pilot: Option<Res<Pilot>>,
  birds: Query<&Bird>,
  beasts: Query<&Beast>,
  mut rng: Local<Rng>,
  mut sounds: MessageWriter<Sound>
) {
  if let Some(pilot) = pilot {
    let dt = time.delta_secs();
    let near = |at: Vec3| at.distance(pilot.at) < CALLS_WITHIN;
    let callers = birds
      .iter()
      .filter(|bird| near(bird.at))
      .map(|bird| {
        let cue = match bird.breed {
          Flier::Swan => Cue::Honk,
          Flier::Raven => Cue::Caw,
          Flier::Gull => Cue::Cry,
          Flier::Parrot => Cue::Song
        };
        (cue, bird.at, 0.06)
      })
      .chain(
        beasts
          .iter()
          .filter(|beast| matches!(beast.breed, Breed::Sheep(_)) && near(beast.at))
          .map(|beast| (Cue::Bleat, beast.at + Vec3::Y * 0.8, 0.025))
      )
      .collect::<Vec<_>>();
    for (cue, at, rate) in callers {
      if rng.unit() < rate * dt {
        sounds.write(Sound::at(cue, at));
      }
    }
  }
}

fn bursts(mut flourishes: MessageReader<Flourish>, mut sounds: MessageWriter<Sound>) {
  for flourish in flourishes.read() {
    sounds.write(
      Sound::at(Cue::Burst, flourish.at).louder(0.7 + flourish.reach.min(6.0) * 0.08)
    );
  }
}

fn pickups(
  pilot: Option<Res<Pilot>>,
  gathered: Query<&Loose, Added<Gathered>>,
  mut sounds: MessageWriter<Sound>
) {
  if let Some(pilot) = pilot {
    for loose in gathered.iter().filter(|loose| loose.at.distance(pilot.at) < 4.0) {
      sounds.write(Sound::at(Cue::Pop, loose.at + Vec3::Y * SIZE));
    }
  }
}

fn levels(
  pilot: Option<Res<Pilot>>,
  skills: Query<&Skills>,
  mut was: Local<Option<u32>>,
  mut sounds: MessageWriter<Sound>
) {
  let level =
    pilot.and_then(|pilot| skills.get(pilot.me).ok()).map(|skills| skills.level);
  if let (Some(before), Some(now)) = (*was, level)
    && now > before
  {
    sounds.write(Sound::here(Cue::Chime));
  }
  *was = level
}

fn travels(mut teleports: MessageReader<Teleport>, mut sounds: MessageWriter<Sound>) {
  if teleports.read().count() > 0 {
    sounds.write(Sound::here(Cue::Warp));
  }
}

fn open_sky(voxels: &Voxels, eye: Vec3) -> f32 {
  let at = eye.floor().as_ivec3();
  let columns =
    [(0, 0), (4, 0), (-4, 0), (0, 4), (0, -4), (3, 3), (-3, 3), (3, -3), (-3, -3)];
  let open = columns
    .iter()
    .filter(|&&(x, z)| {
      (1..SKY_SEARCH).all(|up| {
        voxels
          .block(at + IVec3::new(x, up, z))
          .is_none_or(|block| !block.solid() || block.leafy())
      })
    })
    .count();
  open as f32 / columns.len() as f32
}

fn rain(
  time: Res<Time>,
  library: Res<Library>,
  climate: Option<Res<crate::weather::Climate>>,
  (voxels, eyes): (Option<Res<Voxels>>, Query<&GlobalTransform, With<Eye>>),
  output: Option<NonSendMut<Output>>,
  mut rain: ResMut<Rain>,
  mut sky: Local<(f32, f32)>
) {
  if let Some(mut output) = output
    && let Some(&(take, _)) = library.0.get(&Clip::Rain).and_then(|takes| takes.first())
  {
    let (open, since) = *sky;
    *sky = match (since > 0.3, voxels, eyes.iter().next()) {
      (true, Some(voxels), Some(eye)) => (open_sky(&voxels, eye.translation()), 0.0),
      _ => (open, since + time.delta_secs())
    };
    let open = 0.15 + 0.85 * sky.0;
    let wanted = climate.map_or(0.0, |climate| climate.wet()) * 0.13 * open * MASTER;
    let (voice, level) =
      rain.0.unwrap_or_else(|| (output.start(take, Mix::SILENT, 1.0, true), 0.0));
    let eased = level + (wanted - level) * (1.0 - (-time.delta_secs() * 0.5).exp());
    if (eased - level).abs() > 1e-5 {
      output.set(voice, Mix::flat(eased))
    }
    rain.0 = Some((voice, eased))
  }
}

pub struct Sounds;

impl Plugin for Sounds {
  fn build(&self, app: &mut App) {
    app
      .add_message::<Sound>()
      .init_resource::<Library>()
      .init_resource::<Voices>()
      .init_resource::<Rain>()
      .add_systems(Startup, open.run_if(plays))
      .add_systems(
        Update,
        (
          gather.run_if(resource_exists::<Unrendered>),
          wake,
          (stride, dig, wounds, calls, bursts, pickups, levels, travels),
          play,
          follow,
          rain
        )
          .chain()
          .run_if(plays)
      );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  #[ignore]
  fn sounds_to_files() {
    let folder = std::path::Path::new("screenshots/audio");
    std::fs::create_dir_all(folder).unwrap();
    for clip in Cue::ALL.into_iter().map(Clip::Cue).chain([Clip::Rain]) {
      for (variant, wave) in clip.render().iter().enumerate() {
        let name = format!("{clip:?}").replace(['(', ')'], "").replace("Cue", "");
        std::fs::write(folder.join(format!("{name}-{variant}.wav")), wave.wav()).unwrap();
        let finite = wave.0.iter().flatten().all(|x| x.is_finite());
        println!(
          "{name:>10}-{variant} {:5.2}s peak {:.3} rms {:.3}",
          wave.0[0].len() as f32 / synth::RATE,
          wave.peak(),
          wave.rms()
        );
        assert!(finite && wave.peak() <= 0.51)
      }
    }
  }
}
