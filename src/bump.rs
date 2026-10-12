use {crate::{boat::HULL,
             death::Dying,
             player::{Bulk, Pilot, slide},
             protocol::{Avatar, Beast, Folk, Health, Hopper, Vessel, authority, plays},
             shroomling::BULK,
             voxels::Voxels},
     bevy::prelude::*};

const DECK_SNAP: f32 = 0.35;

#[derive(Clone, Copy)]
pub struct Body {
  pub at: Vec3,
  pub yaw: f32,
  pub half: Vec2,
  pub tall: f32,
  pub deck: bool
}

impl Body {
  fn upright(at: Vec3, Bulk { half, tall }: Bulk) -> Body {
    Body { at, yaw: 0.0, half: Vec2::splat(half), tall, deck: false }
  }

  fn vessel(vessel: &Vessel) -> Body {
    Body { at: vessel.at, yaw: vessel.yaw, half: HULL.xz(), tall: HULL.y, deck: true }
  }

  fn folk(folk: &Folk) -> Body {
    let scale = folk.band.scale();
    Body::upright(folk.at, Bulk { half: 0.3 * scale, tall: 1.9 * scale })
  }

  fn local(&self, at: Vec3) -> Vec2 {
    (Quat::from_rotation_y(-self.yaw) * (at - self.at)).xz()
  }

  pub fn shove(&self, at: Vec3, bulk: Bulk) -> Vec2 {
    let offset = self.local(at);
    let reach = self.half + Vec2::splat(bulk.half);
    let depth = reach - offset.abs();
    let overlapping = depth.x > 0.0
      && depth.y > 0.0
      && at.y < self.at.y + self.tall
      && at.y + bulk.tall > self.at.y;
    let push = match depth.x < depth.y {
      true => Vec2::new(depth.x * offset.x.signum(), 0.0),
      false => Vec2::new(0.0, depth.y * offset.y.signum())
    };
    match overlapping {
      true => (Quat::from_rotation_y(self.yaw) * push.extend(0.0).xzy()).xz(),
      false => Vec2::ZERO
    }
  }

  pub fn deck_under(&self, at: Vec3, bulk: Bulk) -> Option<f32> {
    let top = self.at.y + self.tall;
    let inside =
      self.local(at).abs().cmplt(self.half + Vec2::splat(bulk.half * 0.5)).all();
    (self.deck && inside && (top - DECK_SNAP..=top + 0.05).contains(&at.y)).then_some(top)
  }
}

pub fn pushed(voxels: &Voxels, bulk: Bulk, at: Vec3, push: Vec2) -> Vec3 {
  let (at, _) = slide(voxels, bulk, at, 0, push.x);
  slide(voxels, bulk, at, 2, push.y).0
}

#[derive(Resource, Default)]
pub struct Bodies(pub Vec<Body>);

fn gather(
  pilot: Option<Res<Pilot>>,
  vessels: Query<&Vessel>,
  folk: Query<(&Folk, Option<&Health>)>,
  beasts: Query<(&Beast, Option<&Health>)>,
  hoppers: Query<&Hopper>,
  players: Query<(Entity, &Avatar)>,
  mut bodies: ResMut<Bodies>
) {
  let me = pilot.map(|pilot| pilot.me);
  let alive = |health: Option<&Health>| health.is_none_or(|health| health.0 > 0);
  bodies.0 = vessels
    .iter()
    .map(Body::vessel)
    .chain(
      folk
        .iter()
        .filter(|(folk, health)| !folk.aboard && alive(*health))
        .map(|(folk, _)| Body::folk(folk))
    )
    .chain(
      beasts
        .iter()
        .filter(|(_, health)| alive(*health))
        .map(|(beast, _)| Body::upright(beast.at, beast.breed.bulk()))
    )
    .chain(hoppers.iter().map(|hopper| Body::upright(hopper.at, BULK)))
    .chain(
      players
        .iter()
        .filter(|&(entity, _)| Some(entity) != me)
        .map(|(_, avatar)| Body::upright(avatar.at, Bulk::PERSON))
    )
    .collect()
}

enum Mover<'a> {
  Folk(Mut<'a, Folk>),
  Beast(Mut<'a, Beast>),
  Hopper(Mut<'a, Hopper>)
}

fn jostle(
  voxels: Option<Res<Voxels>>,
  players: Query<&Avatar>,
  vessels: Query<&Vessel>,
  mut folk: Query<(Entity, &mut Folk), Without<Dying>>,
  mut beasts: Query<(Entity, &mut Beast), Without<Dying>>,
  mut hoppers: Query<(Entity, &mut Hopper)>
) {
  if let Some(voxels) = voxels {
    let mut movers: Vec<(Entity, Bulk, Mover)> = folk
      .iter_mut()
      .filter(|(_, folk)| !folk.aboard)
      .map(|(entity, folk)| {
        let scale = folk.band.scale();
        (entity, Bulk { half: 0.3 * scale, tall: 1.9 * scale }, Mover::Folk(folk))
      })
      .chain(
        beasts
          .iter_mut()
          .map(|(entity, beast)| (entity, beast.breed.bulk(), Mover::Beast(beast)))
      )
      .chain(
        hoppers.iter_mut().map(|(entity, hopper)| (entity, BULK, Mover::Hopper(hopper)))
      )
      .collect();
    let at = |mover: &Mover| match mover {
      Mover::Folk(folk) => folk.at,
      Mover::Beast(beast) => beast.at,
      Mover::Hopper(hopper) => hopper.at
    };
    let fixed: Vec<Body> = players
      .iter()
      .map(|avatar| Body::upright(avatar.at, Bulk::PERSON))
      .chain(vessels.iter().map(Body::vessel))
      .collect();
    let nudge = |entity: Entity| {
      let bits = entity.to_bits();
      Vec3::new((bits % 17) as f32 - 8.0, 0.0, (bits / 17 % 17) as f32 - 8.0) * 1e-3
    };
    let moving: Vec<(Entity, Body)> = movers
      .iter()
      .map(|(entity, bulk, mover)| {
        (*entity, Body::upright(at(mover) + nudge(*entity), *bulk))
      })
      .collect();
    for (entity, bulk, mover) in movers.iter_mut() {
      let from = at(mover);
      let push = fixed.iter().map(|body| body.shove(from, *bulk)).sum::<Vec2>()
        + moving
          .iter()
          .filter(|(other, _)| other != entity)
          .map(|(_, body)| body.shove(from + nudge(*entity), *bulk) * 0.5)
          .sum::<Vec2>();
      if push != Vec2::ZERO {
        let moved = pushed(&voxels, *bulk, from, push.clamp_length_max(0.5));
        match mover {
          Mover::Folk(folk) => folk.at = moved,
          Mover::Beast(beast) => beast.at = moved,
          Mover::Hopper(hopper) => hopper.at = moved
        }
      }
    }
  }
}

pub struct Bumping;

impl Plugin for Bumping {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Bodies>()
      .add_systems(Update, (jostle.run_if(authority), gather.run_if(plays)));
  }
}

#[cfg(test)]
mod tests {
  use {super::*, std::f32::consts::FRAC_PI_2};

  #[test]
  fn bodies_push_out_and_decks_hold() {
    let sheep = Body::upright(Vec3::ZERO, Bulk { half: 0.35, tall: 1.0 });
    let push = sheep.shove(Vec3::new(0.5, 0.0, 0.1), Bulk::PERSON);
    assert!((push - Vec2::new(0.15, 0.0)).length() < 1e-4, "{push}");
    assert_eq!(sheep.shove(Vec3::new(0.5, 1.2, 0.1), Bulk::PERSON), Vec2::ZERO);
    let boat = Body::vessel(&Vessel { at: Vec3::ZERO, yaw: FRAC_PI_2, rider: None });
    let push = boat.shove(Vec3::new(0.9, 0.0, 0.0), Bulk::PERSON);
    assert!(push.x > 0.0 && push.y.abs() < 1e-4, "{push}");
    assert_eq!(boat.deck_under(Vec3::new(0.9, 0.3, 0.0), Bulk::PERSON), Some(HULL.y));
    assert_eq!(boat.deck_under(Vec3::new(0.0, 0.3, 0.9), Bulk::PERSON), None)
  }
}
