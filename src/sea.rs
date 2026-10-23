use {crate::{authority::{Controller, player_of},
             beast::{Bone, Joint, Stance},
             block::Block,
             cage::Cage,
             climate,
             condition::{Conditions, Kind},
             death::Dying,
             folk::{KNOCK, Vigour, hurt},
             generate,
             haunt::{Blamed, fell},
             island::{CELL, SEA, UNLOAD},
             noise::{hash, unit},
             protocol::{Avatar, Beast, Breed, EYE, Health, Knock, Pose, REACH, Strike,
                        authority},
             sculpt::{Daub, merged, sculpt},
             voxels::Voxels},
     bevy::{platform::collections::HashSet, prelude::*},
     bevy_replicon::prelude::*,
     std::f32::consts::{PI, TAU}};

const SLEEP: f32 = 170.0;
const LEASH: f32 = 110.0;
const SURFACE: f32 = SEA as f32 + 0.88;
const TURNING: f32 = 2.2;
const STING_EVERY: f32 = 1.5;
const FEAR: f32 = 4.0;
const GRUDGE: f32 = 25.0;

struct Temper {
  cruise: f32,
  chase: f32,
  sense: f32,
  jaws: f32,
  bite: u8,
  bite_every: f32,
  depth: f32,
  deep: i32
}

impl Breed {
  fn temper(self) -> Temper {
    match self {
      Breed::Snapjaw => Temper {
        cruise: 2.2,
        chase: 5.2,
        sense: 16.0,
        jaws: 1.6,
        bite: 3,
        bite_every: 1.2,
        depth: 0.5,
        deep: 4
      },
      Breed::Brinewyrm => Temper {
        cruise: 1.8,
        chase: 4.4,
        sense: 26.0,
        jaws: 3.2,
        bite: 6,
        bite_every: 1.8,
        depth: 0.55,
        deep: 10
      },
      Breed::Glowray => Temper {
        cruise: 1.5,
        chase: 0.0,
        sense: 0.0,
        jaws: 0.0,
        bite: 0,
        bite_every: 0.0,
        depth: 0.3,
        deep: 3
      },
      Breed::Shellback => Temper {
        cruise: 0.9,
        chase: 0.0,
        sense: 0.0,
        jaws: 0.0,
        bite: 0,
        bite_every: 0.0,
        depth: 0.4,
        deep: 2
      },
      _ => Temper {
        cruise: 0.35,
        chase: 0.0,
        sense: 0.0,
        jaws: 1.1,
        bite: 1,
        bite_every: STING_EVERY,
        depth: 0.75,
        deep: 2
      }
    }
  }

  fn hunts(self) -> bool { matches!(self, Breed::Snapjaw | Breed::Brinewyrm) }

  pub fn dangerous(self) -> bool { self.hunts() || self == Breed::Lanternjelly }
}

#[derive(Component)]
pub struct Swimmer {
  home: IVec2,
  timer: f32,
  bite: f32,
  fear: f32,
  grudge: f32,
  luck: u32
}

impl Swimmer {
  fn dice(&mut self) -> f32 {
    self.luck = hash(self.luck, 0x5E, 0xA1, 0x77);
    unit(self.luck, 0, 0, 0)
  }
}

#[derive(Resource, Default)]
pub struct Shoals(HashSet<IVec2>);

fn ahead(yaw: f32) -> Vec3 { Quat::from_rotation_y(yaw) * Vec3::NEG_Z }

fn toward(offset: Vec2) -> f32 { (-offset.x).atan2(-offset.y) }

fn cell_of(at: Vec2) -> IVec2 { (at / CELL as f32).floor().as_ivec2() }

fn centre_of(cell: IVec2) -> Vec2 { (cell * CELL).as_vec2() + CELL as f32 / 2.0 }

fn stock(seed: u32, cell: IVec2) -> Vec<(Breed, Vec3, u32)> {
  let key = hash(seed ^ 0x5EA, cell.x, 0x41, cell.y);
  let roll = |salt: u32, n: u32| unit(key ^ salt.wrapping_mul(0x9E37), n as i32, 0, 0);
  let near_home = cell.abs().max_element();
  let depth_at =
    |at: Vec2| SEA - generate::height(seed, at.x.floor() as i32, at.y.floor() as i32);
  let spots: Vec<(Vec2, i32)> = (0..12)
    .map(|n| {
      let at = (cell * CELL).as_vec2() + Vec2::new(roll(1, n), roll(2, n)) * CELL as f32;
      (at, depth_at(at))
    })
    .filter(|&(_, depth)| depth >= 3)
    .collect();
  let heat = climate::heat(seed, centre_of(cell));
  let groups = [
    (Breed::Lanternjelly, 0.3 + heat.max(0.0) * 0.4, 3),
    (Breed::Glowray, if heat > -0.25 { 0.3 } else { 0.0 }, 2),
    (Breed::Shellback, if heat > 0.05 { 0.25 } else { 0.0 }, 1),
    (Breed::Snapjaw, if near_home <= 1 { 0.0 } else { 0.3 }, 2),
    (Breed::Brinewyrm, if near_home <= 2 { 0.0 } else { 0.14 }, 1)
  ];
  groups
    .into_iter()
    .enumerate()
    .filter(|&(index, (_, chance, _))| roll(10 + index as u32, 0) < chance)
    .flat_map(|(index, (breed, _, most))| {
      let salt = index as u32;
      let count = 1 + (roll(20 + salt, 0) * most as f32) as u32;
      let deep = breed.temper().deep;
      let centre = spots
        .iter()
        .filter(|&&(_, depth)| depth >= deep)
        .nth((roll(25 + salt, 0) * 3.0) as usize)
        .or(spots.iter().find(|&&(_, depth)| depth >= deep))
        .map(|&(at, _)| at);
      centre
        .into_iter()
        .flat_map(move |centre| {
          (0..count).map(move |n| {
            let offset =
              Vec2::new(roll(30 + salt, n) - 0.5, roll(40 + salt, n) - 0.5) * 7.0;
            (centre + offset, n)
          })
        })
        .filter(|&(at, _)| depth_at(at) >= 2)
        .map(move |(at, n)| {
          let y = SURFACE - breed.temper().depth;
          (breed, at.extend(y).xzy(), key ^ (salt * 31 + n).wrapping_mul(0x85EB))
        })
        .collect::<Vec<_>>()
    })
    .collect()
}

pub fn swimmer(breed: Breed, at: Vec3, home: IVec2, luck: u32) -> impl Bundle {
  (
    Replicated,
    Beast { breed, at, yaw: unit(luck, 7, 0, 0) * TAU, pose: Pose::Swim },
    Health(breed.health()),
    Swimmer {
      home,
      timer: unit(luck, 8, 0, 0) * 4.0,
      bite: 0.0,
      fear: 0.0,
      grudge: 0.0,
      luck
    }
  )
}

fn muster(
  voxels: Res<Voxels>,
  mut shoals: ResMut<Shoals>,
  players: Query<&Avatar>,
  mut commands: Commands
) {
  let seed = voxels.seed;
  let wanted: Vec<IVec2> = players
    .iter()
    .flat_map(|avatar| {
      let cell = cell_of(avatar.at.xz());
      (-1..=1).flat_map(move |dx| (-1..=1).map(move |dz| cell + IVec2::new(dx, dz)))
    })
    .collect();
  for cell in wanted {
    if shoals.0.insert(cell) {
      for (breed, at, luck) in stock(seed, cell) {
        commands.spawn(swimmer(breed, at, cell, luck));
      }
    }
  }
}

fn disperse(
  players: Query<&Avatar>,
  swimmers: Query<(Entity, &Swimmer)>,
  mut shoals: ResMut<Shoals>,
  mut commands: Commands
) {
  let far = UNLOAD + CELL as f32;
  let forgotten: Vec<IVec2> = shoals
    .0
    .iter()
    .copied()
    .filter(|&cell| {
      !players.iter().any(|avatar| avatar.at.xz().distance(centre_of(cell)) < far)
    })
    .collect();
  for cell in forgotten.iter() {
    shoals.0.remove(cell);
  }
  for (entity, swimmer) in swimmers.iter() {
    if forgotten.contains(&swimmer.home) {
      commands.entity(entity).despawn()
    }
  }
}

fn afloat(voxels: &Voxels, at: Vec3) -> bool {
  [at + Vec3::Y * 0.3, at - Vec3::Y * 0.6]
    .into_iter()
    .any(|spot| voxels.block(spot.floor().as_ivec3()).is_some_and(Block::fluid))
}

fn swim(
  time: Res<Time>,
  mut voxels: ResMut<Voxels>,
  mut players: Query<(
    Entity,
    &Controller,
    &Avatar,
    &mut Health,
    &mut Vigour,
    &mut Conditions
  )>,
  mut swimmers: Query<(&mut Beast, &mut Swimmer), Without<Dying>>,
  mut knocks: MessageWriter<ToClients<Knock>>
) {
  let dt = time.delta_secs().min(0.1);
  let seed = voxels.seed;
  let spots: Vec<(Entity, Vec3, bool)> = players
    .iter()
    .filter(|(.., health, _, conditions)| {
      health.0 > 0 && !conditions.has(Kind::Invisible)
    })
    .map(|(entity, _, avatar, ..)| (entity, avatar.at, afloat(&voxels, avatar.at)))
    .collect();
  let watched = |at: Vec3| {
    players.iter().any(|(_, _, avatar, ..)| avatar.at.xz().distance(at.xz()) < SLEEP)
  };
  let mut wounds: Vec<(Entity, Vec3, u8, Breed)> = vec![];
  for (mut beast, mut swimmer) in
    swimmers.iter_mut().filter(|(beast, _)| watched(beast.at))
  {
    let Beast { breed, mut at, mut yaw, .. } = *beast;
    let temper = breed.temper();
    let size = breed.size();
    for offset in [IVec3::ZERO, IVec3::X * 4, IVec3::X * -4, IVec3::Z * 4, IVec3::Z * -4]
    {
      voxels.ensure(at.floor().as_ivec3() + offset);
    }
    swimmer.timer -= dt;
    swimmer.bite -= dt;
    swimmer.fear -= dt;
    swimmer.grudge -= dt;
    let prey = spots
      .iter()
      .filter(|&&(_, spot, wet)| {
        breed.hunts()
          && wet
          && spot.distance(at)
            < temper.sense * if swimmer.grudge > 0.0 { 1.6 } else { 1.0 }
      })
      .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)))
      .copied();
    let stung = spots
      .iter()
      .find(|&&(_, spot, _)| {
        breed == Breed::Lanternjelly
          && spot.xz().distance(at.xz()) < temper.jaws
          && (spot.y + 0.9 - at.y).abs() < 1.6
      })
      .copied();
    let biting = prey
      .filter(|&(_, spot, _)| {
        spot.xz().distance(at.xz()) < temper.jaws * size && (spot.y - at.y).abs() < 2.2
      })
      .or(stung);
    if let Some((victim, spot, _)) = biting
      && swimmer.bite <= 0.0
    {
      swimmer.bite = temper.bite_every;
      wounds.push((
        victim,
        (spot - at).with_y(0.0).normalize_or_zero(),
        temper.bite,
        breed
      ));
    }
    let home = centre_of(swimmer.home);
    let goal = match (prey, swimmer.fear > 0.0) {
      (Some((_, spot, _)), _) => toward(spot.xz() - at.xz()),
      _ if at.xz().distance(home) > LEASH => toward(home - at.xz()),
      _ => {
        if swimmer.timer <= 0.0 {
          swimmer.timer = 3.0 + swimmer.dice() * 6.0;
          yaw += (swimmer.dice() - 0.5) * PI * 1.2
        }
        yaw
      }
    };
    let turn = (goal - yaw + PI).rem_euclid(TAU) - PI;
    yaw += turn.clamp(-TURNING * dt, TURNING * dt);
    let speed = match (prey, swimmer.fear > 0.0) {
      (Some(_), _) if biting.is_some() => temper.cruise * 0.5,
      (Some(_), _) => temper.chase,
      (_, true) => temper.cruise * 2.5 + 1.0,
      _ => temper.cruise
    };
    let cruise = SURFACE - temper.depth;
    let bob = (time.elapsed_secs() * 0.9 + (swimmer.luck % 97) as f32).sin() * 0.12;
    let wanted_y = match prey {
      Some((_, spot, _)) => (spot.y + 0.3).clamp(SEA as f32 - 6.0, cruise),
      None => cruise + bob
    };
    let open = |spot: Vec3| {
      voxels
        .block(spot.with_y(spot.y.min(SEA as f32 - 0.2)).floor().as_ivec3())
        .is_some_and(Block::fluid)
    };
    let next = at + ahead(yaw) * speed * dt;
    let probe = at + ahead(yaw) * (1.0 + size);
    let shallow = SEA
      - generate::height(seed, probe.x.floor() as i32, probe.z.floor() as i32)
      < temper.deep.min(4);
    match open(next) && open(probe) && (!shallow || prey.is_some()) {
      true => {
        at.x = next.x;
        at.z = next.z
      }
      false => {
        yaw += PI * (0.5 + swimmer.dice() * 0.5);
        swimmer.timer = 2.0
      }
    }
    let rising = at.y + (wanted_y - at.y).clamp(-1.0, 1.0) * dt * 1.5;
    at.y = match open(at.with_y(rising)) {
      true => rising.min(cruise + 0.15),
      false => at.y
    };
    let pose = match prey.is_some() || swimmer.fear > 0.0 {
      true => Pose::Run,
      false => Pose::Swim
    };
    beast.set_if_neq(Beast { breed, at, yaw: yaw.rem_euclid(TAU), pose });
  }
  for (victim, away, damage, breed) in wounds {
    if let Ok((_, controller, _, mut health, mut vigour, mut conditions)) =
      players.get_mut(victim)
    {
      hurt(&mut health, &mut vigour, damage);
      let push = match breed {
        Breed::Brinewyrm => away * KNOCK * 1.4 + Vec3::Y * 7.0,
        Breed::Lanternjelly => {
          conditions.afflict(Kind::Poisoned, 5.0);
          away * KNOCK * 0.3 + Vec3::Y * 2.0
        }
        _ => away * KNOCK * 0.7 + Vec3::Y * 3.5
      };
      knocks.write(ToClients {
        targets: SendTargets::Single(controller.client),
        message: Knock(push)
      });
    }
  }
}

fn wound(
  mut strikes: MessageReader<FromClient<Strike>>,
  time: Res<Time>,
  mut players: Query<(&Controller, (Entity, &Avatar, &mut Vigour))>,
  mut swimmers: Query<(&mut Beast, &mut Health, &mut Swimmer), Without<Dying>>,
  mut commands: Commands
) {
  let now = time.elapsed_secs();
  for &FromClient { client_id, message: Strike(target) } in strikes.read() {
    if let Ok((mut beast, mut health, mut swimmer)) = swimmers.get_mut(target)
      && let Some((player, avatar, mut vigour)) = player_of(players.iter_mut(), client_id)
      && now - vigour.swung >= vigour.quick
      && (avatar.at + Vec3::Y * EYE).distance(beast.at)
        <= REACH + 1.0 + beast.breed.size()
      && health.0 > 0
    {
      vigour.swung = now;
      health.0 = health.0.saturating_sub(vigour.punch);
      commands.entity(target).insert(Blamed(player));
      let away = (beast.at - avatar.at).with_y(0.0).normalize_or_zero();
      match beast.breed.hunts() {
        true => swimmer.grudge = GRUDGE,
        false => {
          swimmer.fear = FEAR;
          beast.yaw = toward(away.xz())
        }
      }
      if health.0 == 0 {
        commands.entity(target).remove::<Swimmer>();
        fell(&mut commands, target)
      }
    }
  }
}

pub const SEA_LIFE: [Breed; 5] = [
  Breed::Glowray,
  Breed::Lanternjelly,
  Breed::Snapjaw,
  Breed::Brinewyrm,
  Breed::Shellback
];

fn bone(parent: Option<usize>, joint: Joint, pivot: [f32; 3]) -> Bone {
  Bone { parent, joint, pivot: Vec3::from(pivot), parts: vec![] }
}

const JELLY_ARMS: usize = 6;

pub fn rig(breed: Breed) -> Vec<Bone> {
  match breed {
    Breed::Glowray => vec![
      bone(None, Joint::Fixed, [0.0; 3]),
      bone(Some(0), Joint::Hips, [0.0, 0.2, 0.0]),
      bone(Some(1), Joint::Trunk, [0.0; 3]),
      bone(Some(2), Joint::Wing(0.0), [0.3, 0.0, 0.0]),
      bone(Some(2), Joint::Wing(PI), [-0.3, 0.0, 0.0]),
      bone(Some(2), Joint::Tail, [0.0, 0.0, 0.45]),
      bone(Some(5), Joint::TailTip, [0.0, 0.0, 0.45]),
    ],
    Breed::Lanternjelly => [
      bone(None, Joint::Fixed, [0.0; 3]),
      bone(Some(0), Joint::Hips, [0.0, 0.6, 0.0]),
      bone(Some(1), Joint::Trunk, [0.0; 3])
    ]
    .into_iter()
    .chain((0..JELLY_ARMS).map(|arm| {
      let angle = arm as f32 / JELLY_ARMS as f32 * TAU;
      bone(Some(2), Joint::Leg(angle), [angle.cos() * 0.2, -0.08, angle.sin() * 0.2])
    }))
    .collect(),
    Breed::Shellback => vec![
      bone(None, Joint::Fixed, [0.0; 3]),
      bone(Some(0), Joint::Hips, [0.0, 0.3, 0.0]),
      bone(Some(1), Joint::Trunk, [0.0; 3]),
      bone(Some(2), Joint::Neck, [0.0, 0.0, -0.45]),
      bone(Some(2), Joint::Arm(0.0), [0.3, -0.03, -0.25]),
      bone(Some(2), Joint::Arm(PI), [-0.3, -0.03, -0.25]),
      bone(Some(2), Joint::Leg(0.0), [0.27, -0.04, 0.35]),
      bone(Some(2), Joint::Leg(PI), [-0.27, -0.04, 0.35]),
    ],
    _ => {
      let hips = if breed == Breed::Brinewyrm { 0.25 } else { 0.3 };
      vec![
        bone(None, Joint::Fixed, [0.0; 3]),
        bone(Some(0), Joint::Hips, [0.0, hips, 0.0]),
        bone(Some(1), Joint::Trunk, [0.0; 3]),
        bone(Some(2), Joint::Neck, [0.0, 0.0, -0.45]),
        bone(Some(2), Joint::Tail, [0.0, 0.0, 0.35]),
        bone(Some(4), Joint::TailTip, [0.0, 0.0, 0.5]),
      ]
    }
  }
}

fn speckled(centre: Vec3, grain: f32, salt: u32) -> f32 {
  let cell = (centre * grain).round().as_ivec3();
  unit(salt, cell.x, cell.y, cell.z)
}

fn piece(cage: Cage, color: [f32; 3], joint: u16) -> Mesh {
  sculpt(&cage, |_, _| color, &[], |_| joint)
}

fn ray() -> Mesh {
  let body = Cage::rounded(
    Vec3::new(0.0, 0.2, 0.0),
    Vec3::new(0.6, 0.16, 0.8),
    UVec3::new(2, 1, 3),
    0.5
  );
  let side = |cage: &Cage, sign: f32| cage.faces_where(|_, normal| normal.x * sign > 0.6);
  let winged = [1.0f32, -1.0].into_iter().fold(body, |cage, sign| {
    let faces = side(&cage, sign);
    cage.grown_along(&faces, &[
      (Vec3::new(0.35 * sign, 0.0, 0.08), Vec3::new(1.0, 0.45, 0.75)),
      (Vec3::new(0.38 * sign, 0.03, 0.18), Vec3::new(1.0, 0.3, 0.3))
    ])
  });
  let front = winged.faces_where(|_, normal| normal.z < -0.6);
  let horned = winged.grown(&front, Vec3::new(0.0, 0.0, -0.12), Vec3::new(0.6, 0.6, 1.0));
  let joint = |at: Vec3| match () {
    _ if at.x > 0.33 => 3,
    _ if at.x < -0.33 => 4,
    _ if at.z > 0.9 => 6,
    _ if at.z > 0.5 => 5,
    _ => 2
  };
  let eye = |sign: f32| Daub { at: Vec3::new(sign * 0.2, 0.25, -0.45), color: [0.05; 3] };
  let hide = sculpt(
    &horned.subdivided(1),
    |centre, normal| match () {
      _ if normal.y < -0.2 => [0.82, 0.85, 0.88],
      _ if speckled(centre, 9.0, 0x6A7) < 0.14 => [0.45, 0.92, 0.95],
      _ => [0.12, 0.2, 0.3]
    },
    &[eye(1.0), eye(-1.0)],
    joint
  );
  let tail = sculpt(
    &Cage::tube(
      &[
        (Vec3::new(0.0, 0.2, 0.4), Vec2::splat(0.04)),
        (Vec3::new(0.0, 0.2, 0.9), Vec2::splat(0.025)),
        (Vec3::new(0.0, 0.21, 1.45), Vec2::splat(0.004))
      ],
      4
    ),
    |_, _| [0.1, 0.16, 0.24],
    &[],
    joint
  );
  merged([hide, tail])
}

fn jelly() -> Mesh {
  let bell = Cage::rounded(
    Vec3::new(0.0, 0.62, 0.0),
    Vec3::new(0.7, 0.5, 0.7),
    UVec3::splat(2),
    0.85
  )
  .shaped(|at| match at.y < 0.6 {
    true => at.with_y(0.6 - (0.6 - at.y) * 0.25),
    false => at
  })
  .subdivided(1);
  let hood = sculpt(
    &bell,
    |centre, normal| match () {
      _ if normal.y < -0.3 => [0.98, 0.72, 0.95],
      _ if speckled(centre, 10.0, 0x7E1) < 0.2 => [1.0, 0.85, 1.0],
      _ => [0.8, 0.42, 0.86]
    },
    &[],
    |_| 2
  );
  let arms = (0..JELLY_ARMS).map(|arm| {
    let angle = arm as f32 / JELLY_ARMS as f32 * TAU;
    let root = Vec3::new(angle.cos() * 0.2, 0.53, angle.sin() * 0.2);
    let path: Vec<(Vec3, Vec2)> = (0..6)
      .map(|step| {
        let down = step as f32 / 5.0;
        let sway = (down * 5.0 + angle).sin() * 0.05;
        (
          root + Vec3::new(sway, -down * 1.05, sway * 0.6),
          Vec2::splat(0.026 * (1.0 - down * 0.75))
        )
      })
      .collect();
    piece(Cage::tube(&path, 4), [0.98, 0.78, 0.96], 3 + arm as u16)
  });
  let mouth = piece(
    Cage::tube(
      &[
        (Vec3::new(0.0, 0.55, 0.0), Vec2::splat(0.07)),
        (Vec3::new(0.03, 0.3, 0.0), Vec2::splat(0.06)),
        (Vec3::new(-0.02, 0.05, 0.02), Vec2::splat(0.02))
      ],
      5
    ),
    [0.95, 0.55, 0.85],
    2
  );
  merged([hood, mouth].into_iter().chain(arms))
}

fn snapjaw() -> Mesh {
  let body = Cage::tube(
    &[
      (-1.0, 0.04, 0.04, 0.27),
      (-0.85, 0.12, 0.11, 0.28),
      (-0.6, 0.17, 0.16, 0.3),
      (-0.2, 0.2, 0.2, 0.3),
      (0.2, 0.17, 0.17, 0.3),
      (0.55, 0.11, 0.11, 0.3),
      (0.85, 0.06, 0.07, 0.3),
      (1.05, 0.03, 0.04, 0.3)
    ]
    .map(|(z, wide, tall, y)| (Vec3::new(0.0, y, z), Vec2::new(wide, tall))),
    6
  )
  .subdivided(1);
  let joint = |at: Vec3| match () {
    _ if at.z < -0.45 => 3,
    _ if at.z > 0.75 => 5,
    _ if at.z > 0.3 => 4,
    _ => 2
  };
  let eye =
    |sign: f32| Daub { at: Vec3::new(sign * 0.1, 0.35, -0.8), color: [1.0, 0.82, 0.1] };
  let hide = sculpt(
    &body,
    |centre, normal| match () {
      _ if normal.y < -0.25 => [0.78, 0.78, 0.72],
      _ if (-0.62..-0.5).contains(&centre.z) && normal.x.abs() > 0.6 => [0.62, 0.12, 0.1],
      _ if (centre.z * 5.0).rem_euclid(2.0) < 0.5 => [0.13, 0.18, 0.2],
      _ => [0.2, 0.26, 0.29]
    },
    &[eye(1.0), eye(-1.0)],
    joint
  );
  let fin = |from: Vec3, to: Vec3, thick: Vec2| {
    sculpt(&Cage::spike(from, to, thick), |_, _| [0.16, 0.21, 0.24], &[], |_| joint(from))
  };
  let teeth = [1.0f32, -1.0].into_iter().flat_map(|sign| {
    (0..5).map(move |tooth| {
      let along = tooth as f32 / 4.0;
      let root = Vec3::new(sign * (0.04 + along * 0.07), 0.25, -0.95 + along * 0.22);
      piece(
        Cage::spike(root, root - Vec3::Y * 0.07, Vec2::splat(0.012)),
        [0.95, 0.93, 0.85],
        3
      )
    })
  });
  merged(
    [
      hide,
      fin(
        Vec3::new(0.0, 0.48, -0.15),
        Vec3::new(0.0, 0.98, 0.08),
        Vec2::new(0.025, 0.14)
      ),
      fin(Vec3::new(0.0, 0.3, 1.0), Vec3::new(0.0, 0.65, 1.25), Vec2::new(0.02, 0.09)),
      fin(Vec3::new(0.0, 0.3, 1.0), Vec3::new(0.0, 0.03, 1.2), Vec2::new(0.02, 0.08)),
      fin(
        Vec3::new(0.17, 0.24, -0.45),
        Vec3::new(0.48, 0.13, -0.28),
        Vec2::new(0.08, 0.015)
      ),
      fin(
        Vec3::new(-0.17, 0.24, -0.45),
        Vec3::new(-0.48, 0.13, -0.28),
        Vec2::new(0.08, 0.015)
      )
    ]
    .into_iter()
    .chain(teeth)
  )
}

const WYRM_SPINE: [(f32, f32, f32); 11] = [
  (-1.0, 1.05, 0.13),
  (-0.85, 0.95, 0.15),
  (-0.68, 0.7, 0.16),
  (-0.55, 0.4, 0.17),
  (-0.35, 0.25, 0.17),
  (-0.05, 0.2, 0.16),
  (0.25, 0.3, 0.15),
  (0.55, 0.22, 0.13),
  (0.85, 0.12, 0.1),
  (1.15, 0.15, 0.07),
  (1.45, 0.2, 0.035)
];

fn wyrm() -> Mesh {
  let joint = |at: Vec3| match () {
    _ if at.z < -0.5 => 3,
    _ if at.z > 1.05 => 5,
    _ if at.z > 0.45 => 4,
    _ => 2
  };
  let body =
    Cage::tube(&WYRM_SPINE.map(|(z, y, r)| (Vec3::new(0.0, y, z), Vec2::splat(r))), 6)
      .joined(Cage::rounded(
        Vec3::new(0.0, 1.06, -1.12),
        Vec3::new(0.26, 0.2, 0.38),
        UVec3::splat(2),
        0.5
      ))
      .subdivided(1);
  let eye =
    |sign: f32| Daub { at: Vec3::new(sign * 0.12, 1.12, -1.15), color: [1.0, 0.9, 0.2] };
  let hide = sculpt(
    &body,
    |centre, normal| match () {
      _ if normal.y < -0.4 => [0.62, 0.62, 0.42],
      _ if speckled(centre, 8.0, 0x3B1) < 0.25 => [0.06, 0.2, 0.22],
      _ => [0.1, 0.32, 0.31]
    },
    &[eye(1.0), eye(-1.0)],
    joint
  );
  let crest = WYRM_SPINE.windows(2).skip(1).map(|pair| {
    let [(z, y, r), (next_z, next_y, _)] = [pair[0], pair[1]];
    let root = Vec3::new(0.0, y + r * 0.8, z);
    let back = Vec3::new(0.0, next_y - y, next_z - z).normalize_or_zero();
    sculpt(
      &Cage::spike(root, root + Vec3::Y * r * 1.3 + back * r, Vec2::new(0.015, r * 0.6)),
      |_, _| [0.82, 0.22, 0.16],
      &[],
      |_| joint(root)
    )
  });
  let horns = [1.0f32, -1.0].map(|sign| {
    piece(
      Cage::tube(
        &[
          (Vec3::new(sign * 0.08, 1.14, -1.02), Vec2::splat(0.035)),
          (Vec3::new(sign * 0.13, 1.25, -0.88), Vec2::splat(0.022)),
          (Vec3::new(sign * 0.15, 1.3, -0.72), Vec2::splat(0.003))
        ],
        4
      ),
      [0.86, 0.82, 0.68],
      3
    )
  });
  let fangs = [1.0f32, -1.0].map(|sign| {
    let root = Vec3::new(sign * 0.07, 0.98, -1.26);
    piece(
      Cage::spike(root, root - Vec3::Y * 0.12, Vec2::splat(0.02)),
      [0.95, 0.93, 0.85],
      3
    )
  });
  merged([hide].into_iter().chain(crest).chain(horns).chain(fangs))
}

fn shellback() -> Mesh {
  let shell = Cage::rounded(
    Vec3::new(0.0, 0.32, 0.0),
    Vec3::new(0.8, 0.32, 1.0),
    UVec3::new(3, 2, 3),
    0.55
  )
  .shaped(|at| match at.y < 0.3 {
    true => at.with_y(0.3 - (0.3 - at.y) * 0.4),
    false => at
  })
  .subdivided(1);
  let joint = |at: Vec3| match () {
    _ if at.z < -0.45 => 3,
    _ if at.x > 0.32 && at.z < 0.0 => 4,
    _ if at.x < -0.32 && at.z < 0.0 => 5,
    _ if at.x > 0.25 && at.z > 0.3 => 6,
    _ if at.x < -0.25 && at.z > 0.3 => 7,
    _ => 2
  };
  let carapace = sculpt(
    &shell,
    |centre, normal| match () {
      _ if normal.y < -0.3 => [0.74, 0.68, 0.48],
      _ if normal.y < 0.25 => [0.22, 0.2, 0.11],
      _ => match ((centre.x * 5.0).round() + (centre.z * 5.0).round()) as i32 % 2 == 0 {
        true => [0.36, 0.3, 0.15],
        false => [0.28, 0.33, 0.16]
      }
    },
    &[],
    |_| 2
  );
  let skin = |centre: Vec3, _: Vec3| match speckled(centre, 14.0, 0x7A2) < 0.2 {
    true => [0.3, 0.36, 0.2],
    false => [0.44, 0.52, 0.32]
  };
  let head = sculpt(
    &Cage::tube(
      &[
        (Vec3::new(0.0, 0.3, -0.38), Vec2::splat(0.09)),
        (Vec3::new(0.0, 0.31, -0.55), Vec2::splat(0.08)),
        (Vec3::new(0.0, 0.33, -0.66), Vec2::new(0.09, 0.08)),
        (Vec3::new(0.0, 0.31, -0.78), Vec2::new(0.06, 0.05))
      ],
      6
    ),
    skin,
    &[Daub { at: Vec3::new(0.07, 0.36, -0.7), color: [0.04; 3] }, Daub {
      at: Vec3::new(-0.07, 0.36, -0.7),
      color: [0.04; 3]
    }],
    |_| 3
  );
  let flipper =
    |path: &[(Vec3, Vec2)]| sculpt(&Cage::tube(path, 4), skin, &[], |_| joint(path[2].0));
  let flippers = [1.0f32, -1.0].into_iter().flat_map(|sign| {
    [
      flipper(&[
        (Vec3::new(sign * 0.28, 0.27, -0.25), Vec2::new(0.1, 0.03)),
        (Vec3::new(sign * 0.55, 0.25, -0.33), Vec2::new(0.11, 0.025)),
        (Vec3::new(sign * 0.82, 0.22, -0.26), Vec2::new(0.04, 0.012))
      ]),
      flipper(&[
        (Vec3::new(sign * 0.25, 0.26, 0.35), Vec2::new(0.07, 0.025)),
        (Vec3::new(sign * 0.4, 0.25, 0.47), Vec2::new(0.07, 0.02)),
        (Vec3::new(sign * 0.5, 0.24, 0.55), Vec2::new(0.02, 0.01))
      ])
    ]
  });
  merged([carapace, head].into_iter().chain(flippers))
}

pub fn hide(breed: Breed) -> Mesh {
  match breed {
    Breed::Glowray => ray(),
    Breed::Lanternjelly => jelly(),
    Breed::Snapjaw => snapjaw(),
    Breed::Brinewyrm => wyrm(),
    _ => shellback()
  }
}

pub fn glassy(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
  materials.add(StandardMaterial {
    base_color: Color::srgba(1.0, 1.0, 1.0, 0.72),
    emissive: LinearRgba::rgb(0.5, 0.18, 0.55),
    alpha_mode: AlphaMode::Blend,
    perceptual_roughness: 0.3,
    ..default()
  })
}

pub fn stance(breed: Breed, pose: Pose, now: f32) -> Stance {
  let hurried = pose == Pose::Run;
  let pace = if hurried { 2.2 } else { 1.0 };
  let wiggle = (now * 5.0 * pace).sin();
  let still = Stance {
    hips: 0.0,
    trunk: Quat::IDENTITY,
    neck: Quat::IDENTITY,
    tail: Quat::IDENTITY,
    tip: Quat::IDENTITY,
    leg: 0.0,
    arm: (0.0, 0.0, 0.0),
    fin: 0.0
  };
  match breed {
    Breed::Glowray => Stance {
      hips: 0.2 + (now * 1.3).sin() * 0.04,
      fin: (now * 2.2 * pace).sin() * 0.5,
      tail: Quat::from_rotation_y((now * 1.5).sin() * 0.3),
      tip: Quat::from_rotation_y((now * 1.5 - 1.0).sin() * 0.4),
      ..still
    },
    Breed::Lanternjelly => {
      let pulse = (now * 1.8).sin();
      Stance { hips: 0.6 + pulse * 0.06, leg: pulse * 0.28, ..still }
    }
    Breed::Shellback => {
      let stroke = (now * 1.6 * pace).sin();
      Stance {
        hips: 0.3 + stroke * 0.02,
        neck: Quat::from_rotation_x((now * 0.7).sin() * 0.12),
        arm: (0.0, stroke * 0.3, stroke * 0.5),
        leg: stroke * 0.4,
        ..still
      }
    }
    Breed::Brinewyrm => {
      let slow = (now * 2.4 * pace).sin();
      let lunge = if hurried { -(now * 3.0).sin().max(0.0) * 0.35 } else { 0.0 };
      Stance {
        hips: 0.25 + (now * 0.8).sin() * 0.05,
        trunk: Quat::from_rotation_y(slow * 0.08),
        neck: Quat::from_rotation_x((now * 0.9).sin() * 0.1 + lunge)
          * Quat::from_rotation_y(-slow * 0.12),
        tail: Quat::from_rotation_y(-slow * 0.35),
        tip: Quat::from_rotation_y(-(now * 2.4 * pace - 1.2).sin() * 0.5),
        ..still
      }
    }
    _ => {
      let snap = if hurried { (now * 8.0).sin().max(0.0) * 0.18 } else { 0.0 };
      Stance {
        hips: 0.3,
        trunk: Quat::from_rotation_y(wiggle * 0.1),
        neck: Quat::from_rotation_y(-wiggle * 0.12) * Quat::from_rotation_x(snap),
        tail: Quat::from_rotation_y(-wiggle * 0.45),
        tip: Quat::from_rotation_y(-(now * 5.0 * pace - 1.2).sin() * 0.6),
        ..still
      }
    }
  }
}

pub struct SeaLife;

impl Plugin for SeaLife {
  fn build(&self, app: &mut App) {
    app
      .init_resource::<Shoals>()
      .add_systems(
        Update,
        (muster, swim, disperse)
          .chain()
          .run_if(authority)
          .run_if(resource_exists::<Voxels>)
      )
      .add_systems(PreUpdate, wound.after(ServerSystems::Receive).run_if(authority));
  }
}

#[cfg(test)]
mod tests {
  use {super::*, crate::sculpt::sheet};

  #[test]
  #[ignore]
  fn sea_hides() { sheet(&SEA_LIFE.map(hide), "screenshots/sea.ppm") }

  #[test]
  fn sea_is_stocked() {
    let found: Vec<Breed> = (-6..6)
      .flat_map(|x| (-6..6).map(move |z| IVec2::new(x, z)))
      .flat_map(|cell| stock(1, cell))
      .map(|(breed, ..)| breed)
      .collect();
    for breed in SEA_LIFE {
      assert!(found.contains(&breed), "no {breed:?}")
    }
  }
}
