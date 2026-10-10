use {crate::{beast,
             bird::{self, Bird},
             block::{Block, Fluid},
             claim::{self, Claim, Holding},
             flow::Flows,
             generate,
             island::Island,
             minimap::hue,
             opts::opts,
             protocol::*,
             shroomling,
             voxels::{HEIGHT, LAYERS, SIZE, Voxels, chunk_of}},
     bevy::prelude::*,
     bevy_replicon::prelude::*,
     std::{io::{BufRead, BufReader, Write},
           net::{TcpListener, TcpStream},
           sync::{Mutex, mpsc}}};

pub const PORT: u16 = 7778;
const MOST_FILLED: i64 = 65_536;
const WIDEST_TEXT: i32 = 64;
const WIDEST_IMAGE: i32 = 1024;

const HELP: &str = "\
bloxy admin: one command per connection, coordinates are block x y z (y up, sea level 62).
  help                          this text
  players                       who is online, where, health
  get X Y Z                     the block at a spot
  set X Y Z BLOCK               place a block (any Block name, e.g. Stone, OakSign, Air)
  fill X1 Y1 Z1 X2 Y2 Z2 BLOCK  fill a box (at most 65536 blocks)
  column X Z                    the blocks of one column, top down, as runs
  top X Z R                     top-down text map of the surface, radius R (<= 64), with a legend
  layer X Y Z R                 text map of the horizontal slice at height Y, radius R (<= 64)
  image X Z R [FILE]            top-down PNG of the surface (1 px per block, R <= 1024) with
                                players (white), mobs (red) and claims (yellow edges);
                                default FILE /tmp/bloxy-view.png
  mobs [X Z R]                  mobs with their ids (all, or within R of X Z)
  spawn KIND X Y Z [COUNT]      sheep, lizard, swan, raven, gull, parrot or shroomling
  kill ID...                    remove mobs by id
  kill KIND|all X Z R           remove every mob of a kind (or all) within R of X Z
  claims                        claimed chunks and their owners
  regen [SEED]                  regenerate the world (optionally with a new seed), keeping
                                claimed chunks exactly as they are; mobs are restocked";

struct Request {
  line: String,
  reply: mpsc::Sender<String>
}

#[derive(Resource)]
struct Desk(Mutex<mpsc::Receiver<Request>>);

fn attend(stream: TcpStream, requests: mpsc::Sender<Request>) {
  let mut line = String::new();
  let mut reader = BufReader::new(&stream);
  let (reply, answer) = mpsc::channel();
  let answered = reader
    .read_line(&mut line)
    .ok()
    .filter(|_| requests.send(Request { line: line.trim().to_string(), reply }).is_ok())
    .and_then(|_| answer.recv_timeout(std::time::Duration::from_secs(120)).ok())
    .unwrap_or_else(|| "no answer from the server".into());
  let _ = (&stream).write_all(format!("{answered}\n").as_bytes());
}

fn open(mut commands: Commands) {
  let port = opts().admin.unwrap_or(PORT);
  match TcpListener::bind(("127.0.0.1", port)) {
    Ok(listener) => {
      let (requests, desk) = mpsc::channel();
      std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
          let requests = requests.clone();
          std::thread::spawn(move || attend(stream, requests));
        }
      });
      info!("admin commands on 127.0.0.1:{port}");
      commands.insert_resource(Desk(Mutex::new(desk)))
    }
    Err(blame) => warn!("no admin port {port}: {blame}")
  }
}

pub fn ask(command: &str) -> String {
  let port = opts().admin.unwrap_or(PORT);
  TcpStream::connect(("127.0.0.1", port))
    .and_then(|mut stream| {
      stream.write_all(format!("{command}\n").as_bytes())?;
      let mut answer = String::new();
      std::io::Read::read_to_string(&mut stream, &mut answer)?;
      Ok(answer)
    })
    .unwrap_or_else(|blame| {
      format!("cannot reach a bloxy server on 127.0.0.1:{port}: {blame}\n")
    })
}

fn block_named(name: &str) -> Result<Block, String> {
  Block::ALL
    .into_iter()
    .find(|block| format!("{block:?}").eq_ignore_ascii_case(name))
    .ok_or_else(|| format!("unknown block {name}"))
}

fn number<T: std::str::FromStr>(word: Option<&&str>) -> Result<T, String> {
  word
    .ok_or("missing number".to_string())
    .and_then(|word| word.parse().map_err(|_| format!("not a number: {word}")))
}

fn spot(words: &[&str]) -> Result<IVec3, String> {
  Ok(IVec3::new(number(words.first())?, number(words.get(1))?, number(words.get(2))?))
}

fn place(world: &mut World, at: IVec3, block: Block) {
  let now = world.resource::<Time>().elapsed_secs();
  world.resource_mut::<Voxels>().set(at, block);
  world.resource_mut::<Flows>().stir(at, now + Fluid::Water.delay());
  world.write_message(ToClients {
    targets: SendTargets::All,
    message: Altered { at, block }
  });
}

fn top(voxels: &mut Voxels, x: i32, z: i32) -> (i32, Block) {
  (0..HEIGHT)
    .rev()
    .map(|y| (y, voxels.ensure(IVec3::new(x, y, z))))
    .find(|&(_, block)| block != Block::Air)
    .unwrap_or((0, Block::Bedrock))
}

fn surface(voxels: &mut Voxels, x: i32, z: i32) -> (i32, Block) {
  let column = chunk_of(IVec3::new(x, 0, z)).xz();
  let known = (0..LAYERS).any(|layer| {
    let key = IVec3::new(column.x, layer, column.y);
    voxels.chunks.contains_key(&key) || voxels.edits.contains_key(&key)
  }) || voxels.kept.contains_key(&column);
  match known {
    true => top(voxels, x, z),
    false => {
      let (block, height, _) = generate::overview(voxels.seed, x, z);
      (height, block)
    }
  }
}

fn letter(block: Block) -> char {
  match block {
    Block::Air => ' ',
    water if water.liquid().is_some_and(|(fluid, _)| fluid == Fluid::Water) => '~',
    lava if lava.fluid() => '%',
    Block::Grass => '"',
    Block::TallGrass => ',',
    Block::Sand => '.',
    Block::Snow => '_',
    Block::Ice => '=',
    Block::Stone => '#',
    Block::Cobblestone => '&',
    Block::Dirt => ':',
    Block::Torch => '!',
    leaves if leaves.leafy() => '*',
    log if log.look() == crate::block::Look::Log => 'T',
    plant if plant.plant() => '\'',
    Block::Waystone | Block::WaystoneTop => 'W',
    other => format!("{other:?}").chars().next().unwrap_or('?')
  }
}

fn chart(blocks: Vec<Vec<Block>>, label: String) -> String {
  let mut used: Vec<Block> = blocks.iter().flatten().copied().collect();
  used.sort();
  used.dedup();
  let rows: Vec<String> =
    blocks.iter().map(|row| row.iter().map(|&block| letter(block)).collect()).collect();
  let legend: Vec<String> =
    used.iter().map(|&block| format!("'{}' {block:?}", letter(block))).collect();
  format!(
    "{label}\nnorth (-z) is up, east (+x) is right\n{}\nlegend: {}",
    rows.join("\n"),
    legend.join(", ")
  )
}

#[derive(Clone, Copy)]
enum Kind {
  Beast(Breed),
  Bird(bird::Breed),
  Shroomling
}

fn kind_named(name: &str) -> Option<Kind> {
  match name.to_ascii_lowercase().as_str() {
    "sheep" => Some(Kind::Beast(Breed::Sheep(Fleece::White))),
    "lizard" => Some(Kind::Beast(Breed::Lizard)),
    "swan" => Some(Kind::Bird(bird::Breed::Swan)),
    "raven" => Some(Kind::Bird(bird::Breed::Raven)),
    "gull" => Some(Kind::Bird(bird::Breed::Gull)),
    "parrot" => Some(Kind::Bird(bird::Breed::Parrot)),
    "shroomling" => Some(Kind::Shroomling),
    _ => None
  }
}

fn mobs(world: &mut World) -> Vec<(Entity, String, Vec3)> {
  let beasts: Vec<(Entity, String, Vec3)> = world
    .query::<(Entity, &Beast)>()
    .iter(world)
    .map(|(entity, beast)| {
      let kind = match beast.breed {
        Breed::Sheep(_) => "sheep",
        Breed::Lizard => "lizard"
      };
      (entity, kind.to_string(), beast.at)
    })
    .collect();
  let birds: Vec<(Entity, String, Vec3)> = world
    .query::<(Entity, &Bird)>()
    .iter(world)
    .map(|(entity, bird)| (entity, format!("{:?}", bird.breed).to_lowercase(), bird.at))
    .collect();
  let hoppers: Vec<(Entity, String, Vec3)> = world
    .query::<(Entity, &Hopper)>()
    .iter(world)
    .map(|(entity, hopper)| (entity, "shroomling".to_string(), hopper.at))
    .collect();
  let folk: Vec<(Entity, String, Vec3)> = world
    .query::<(Entity, &Folk)>()
    .iter(world)
    .map(|(entity, folk)| (entity, format!("{:?}", folk.band).to_lowercase(), folk.at))
    .collect();
  let ships: Vec<(Entity, String, Vec3)> = world
    .query::<(Entity, &Ship)>()
    .iter(world)
    .map(|(entity, ship)| {
      (entity, format!("{:?} ship", ship.band).to_lowercase(), ship.at)
    })
    .collect();
  [beasts, birds, hoppers, folk, ships].concat()
}

fn within(at: Vec3, centre: Vec2, radius: f32) -> bool {
  at.xz().distance(centre) <= radius
}

fn image(
  world: &mut World,
  centre: IVec2,
  radius: i32,
  file: &str
) -> Result<String, String> {
  let side = radius * 2 + 1;
  let hues = Block::ALL.map(hue);
  let mut voxels = world.resource_mut::<Voxels>();
  let surfaces: Vec<(i32, Block)> = (0..side * side)
    .map(|index| {
      surface(
        &mut voxels,
        centre.x - radius + index % side,
        centre.y - radius + index / side
      )
    })
    .collect();
  let mut pixels: Vec<[u8; 3]> = surfaces
    .iter()
    .enumerate()
    .map(|(index, &(height, block))| {
      let north =
        if index >= side as usize { surfaces[index - side as usize].0 } else { height };
      let light = 1.0 + ((height - north) as f32 * 0.12).clamp(-0.3, 0.3);
      hues[block as usize]
        .map(|channel| ((channel * light).clamp(0.0, 1.0) * 255.0) as u8)
    })
    .collect();
  let claimed: Vec<IVec2> =
    world.query::<&Claim>().iter(world).map(|claim| claim.column).collect();
  for index in 0..side * side {
    let at =
      IVec2::new(centre.x - radius + index % side, centre.y - radius + index / side);
    let local = at.rem_euclid(IVec2::splat(SIZE));
    let edge = local.x == 0 || local.y == 0 || local.x == SIZE - 1 || local.y == SIZE - 1;
    if edge && claimed.contains(&at.div_euclid(IVec2::splat(SIZE))) {
      pixels[index as usize] = [250, 220, 60]
    }
  }
  let mut mark = |at: Vec2, colour: [u8; 3]| {
    let pixel = at.floor().as_ivec2() - centre + radius;
    for dy in -1..=1 {
      for dx in -1..=1 {
        let spot = pixel + IVec2::new(dx, dy);
        if spot.cmpge(IVec2::ZERO).all() && spot.cmplt(IVec2::splat(side)).all() {
          pixels[(spot.y * side + spot.x) as usize] = colour
        }
      }
    }
  };
  for (_, _, at) in mobs(world).iter() {
    mark(at.xz(), [230, 40, 40])
  }
  for avatar in world.query::<&Avatar>().iter(world) {
    mark(avatar.at.xz(), [255, 255, 255])
  }
  let output = std::fs::File::create(file)
    .map_err(|blame| format!("cannot write {file}: {blame}"))?;
  let mut encoder =
    png::Encoder::new(std::io::BufWriter::new(output), side as u32, side as u32);
  encoder.set_color(png::ColorType::Rgb);
  encoder.set_depth(png::BitDepth::Eight);
  encoder
    .write_header()
    .and_then(|mut writer| writer.write_image_data(&pixels.concat()))
    .map_err(|blame| format!("cannot encode {file}: {blame}"))?;
  Ok(format!(
    "wrote {file}: {side} x {side} px, 1 px per block, north (-z) up, top-left corner at x {} z {}",
    centre.x - radius,
    centre.y - radius
  ))
}

fn run(world: &mut World, line: &str) -> Result<String, String> {
  let words: Vec<&str> = line.split_whitespace().collect();
  let rest = words.get(1..).unwrap_or_default();
  match words.first().map(|word| word.to_ascii_lowercase()).as_deref() {
    None | Some("help") => Ok(HELP.into()),
    Some("players") => {
      let lines: Vec<String> = world
        .query::<(&Player, &Avatar, &Health)>()
        .iter(world)
        .map(|(player, avatar, health)| {
          let at = avatar.at.floor().as_ivec3();
          format!("{}  at {} {} {}  health {}", player.name, at.x, at.y, at.z, health.0)
        })
        .collect();
      Ok(if lines.is_empty() { "nobody online".into() } else { lines.join("\n") })
    }
    Some("get") => {
      let at = spot(rest)?;
      Ok(format!("{:?}", world.resource_mut::<Voxels>().ensure(at)))
    }
    Some("set") => {
      let (at, block) = (spot(rest)?, block_named(rest.get(3).ok_or("missing block")?)?);
      world.resource_mut::<Voxels>().ensure(at);
      place(world, at, block);
      Ok(format!("set {} {} {} to {block:?}", at.x, at.y, at.z))
    }
    Some("fill") => {
      let (from, to) = (spot(rest)?, spot(rest.get(3..).unwrap_or_default())?);
      let block = block_named(rest.get(6).ok_or("missing block")?)?;
      let (low, high) =
        (from.min(to), from.max(to).min(IVec3::new(i32::MAX, HEIGHT - 1, i32::MAX)));
      let size = (high - low + 1).as_i64vec3();
      match size.x * size.y * size.z {
        volume if volume > MOST_FILLED => {
          Err(format!("{volume} blocks is more than {MOST_FILLED}"))
        }
        volume => {
          for y in low.y..=high.y {
            for z in low.z..=high.z {
              for x in low.x..=high.x {
                let at = IVec3::new(x, y, z);
                world.resource_mut::<Voxels>().ensure(at);
                place(world, at, block)
              }
            }
          }
          Ok(format!("filled {volume} blocks with {block:?}"))
        }
      }
    }
    Some("column") => {
      let (x, z): (i32, i32) = (number(rest.first())?, number(rest.get(1))?);
      let mut voxels = world.resource_mut::<Voxels>();
      let runs = (0..HEIGHT).rev().map(|y| (y, voxels.ensure(IVec3::new(x, y, z)))).fold(
        Vec::<(i32, i32, Block)>::new(),
        |mut runs, (y, block)| {
          match runs.last_mut() {
            Some((_, low, last)) if *last == block => *low = y,
            _ => runs.push((y, y, block))
          }
          runs
        }
      );
      Ok(
        runs
          .iter()
          .map(|(high, low, block)| format!("y {low}..{high}  {block:?}"))
          .collect::<Vec<_>>()
          .join("\n")
      )
    }
    Some("top") => {
      let (x, z, radius): (i32, i32, i32) =
        (number(rest.first())?, number(rest.get(1))?, number(rest.get(2))?);
      let radius = radius.clamp(1, WIDEST_TEXT);
      let mut voxels = world.resource_mut::<Voxels>();
      let rows: Vec<Vec<(i32, Block)>> = (-radius..=radius)
        .map(|dz| (-radius..=radius).map(|dx| top(&mut voxels, x + dx, z + dz)).collect())
        .collect();
      let heights = rows.iter().flatten().map(|(height, _)| *height);
      let (lowest, highest) =
        (heights.clone().min().unwrap_or(0), heights.max().unwrap_or(0));
      Ok(chart(
        rows
          .into_iter()
          .map(|row| row.into_iter().map(|(_, block)| block).collect())
          .collect(),
        format!(
          "surface around x {x} z {z}, radius {radius} (top-left x {} z {}), heights {lowest}..{highest}",
          x - radius,
          z - radius
        )
      ))
    }
    Some("layer") => {
      let at = spot(rest)?;
      let radius = number::<i32>(rest.get(3))?.clamp(1, WIDEST_TEXT);
      let mut voxels = world.resource_mut::<Voxels>();
      let rows: Vec<Vec<Block>> = (-radius..=radius)
        .map(|dz| {
          (-radius..=radius)
            .map(|dx| voxels.ensure(IVec3::new(at.x + dx, at.y, at.z + dz)))
            .collect()
        })
        .collect();
      Ok(chart(
        rows,
        format!("slice at y {} around x {} z {}, radius {radius}", at.y, at.x, at.z)
      ))
    }
    Some("image") => {
      let (x, z, radius): (i32, i32, i32) =
        (number(rest.first())?, number(rest.get(1))?, number(rest.get(2))?);
      let file = rest.get(3).copied().unwrap_or("/tmp/bloxy-view.png");
      image(world, IVec2::new(x, z), radius.clamp(1, WIDEST_IMAGE), file)
    }
    Some("mobs") => {
      let area = match rest.len() {
        0 => None,
        _ => Some((
          Vec2::new(number(rest.first())?, number(rest.get(1))?),
          number::<f32>(rest.get(2))?
        ))
      };
      let lines: Vec<String> = mobs(world)
        .into_iter()
        .filter(|(_, _, at)| {
          area.is_none_or(|(centre, radius)| within(*at, centre, radius))
        })
        .map(|(entity, kind, at)| {
          format!("{}  {kind}  at {:.1} {:.1} {:.1}", entity.to_bits(), at.x, at.y, at.z)
        })
        .collect();
      Ok(format!("{} mobs\n{}", lines.len(), lines.join("\n")))
    }
    Some("spawn") => {
      let kind = rest.first().and_then(|name| kind_named(name)).ok_or("unknown kind")?;
      let at =
        spot(rest.get(1..).unwrap_or_default())?.as_vec3() + Vec3::new(0.5, 0.05, 0.5);
      let count = rest.get(4).map_or(Ok(1), |_| number::<u32>(rest.get(4)))?.clamp(1, 50);
      let seed = world.resource::<Voxels>().seed;
      let home = Island::near(seed, at)
        .first()
        .map_or(at.xz().floor().as_ivec2() / 128, |island| island.cell);
      for index in 0..count {
        let luck = crate::noise::hash(0xAD31, at.x as i32, index as i32, at.z as i32);
        let spot =
          at + Vec3::new((index % 5) as f32 * 0.7, 0.0, (index / 5) as f32 * 0.7);
        match kind {
          Kind::Beast(breed) => {
            world.spawn(beast::herd(breed, spot, home, luck));
          }
          Kind::Bird(breed) => {
            let bundle = bird::release(world.resource::<Voxels>(), breed, spot, luck);
            world.spawn(bundle);
          }
          Kind::Shroomling => {
            let mut commands = world.commands();
            shroomling::lodge(
              &mut commands,
              home,
              Hopper { at: spot, yaw: 0.0, aloft: false },
              luck
            );
            world.flush()
          }
        }
      }
      Ok(format!("spawned {count} {}", rest[0]))
    }
    Some("kill") => {
      let doomed: Vec<Entity> =
        match rest.first().and_then(|word| word.parse::<u64>().ok()) {
          Some(_) => {
            let wanted: Vec<u64> =
              rest.iter().filter_map(|word| word.parse().ok()).collect();
            mobs(world)
              .into_iter()
              .filter(|(entity, ..)| wanted.contains(&entity.to_bits()))
              .map(|(entity, ..)| entity)
              .collect()
          }
          None => {
            let kind = rest.first().ok_or("kill what?")?.to_ascii_lowercase();
            let centre = Vec2::new(number(rest.get(1))?, number(rest.get(2))?);
            let radius: f32 = number(rest.get(3))?;
            mobs(world)
              .into_iter()
              .filter(|(_, name, at)| {
                (kind == "all" || *name == kind) && within(*at, centre, radius)
              })
              .map(|(entity, ..)| entity)
              .collect()
          }
        };
      for &entity in doomed.iter() {
        world.despawn(entity);
      }
      Ok(format!("removed {} mobs", doomed.len()))
    }
    Some("claims") => {
      let lines: Vec<String> = world
        .query::<(&Claim, &Holding)>()
        .iter(world)
        .map(|(claim, holding)| {
          format!(
            "chunk {} {} (x {}..{}, z {}..{})  {}  account {}",
            claim.column.x,
            claim.column.y,
            claim.column.x * SIZE,
            claim.column.x * SIZE + SIZE - 1,
            claim.column.y * SIZE,
            claim.column.y * SIZE + SIZE - 1,
            claim.owner,
            holding.account
          )
        })
        .collect();
      Ok(format!("{} claims\n{}", lines.len(), lines.join("\n")))
    }
    Some("regen") => {
      let seed = rest.first().map(|_| number::<u32>(rest.first())).transpose()?;
      Ok(claim::regen(world, seed))
    }
    Some(other) => Err(format!("unknown command {other}; try help"))
  }
}

fn obey(world: &mut World) {
  let requests: Vec<Request> = world
    .get_resource::<Desk>()
    .and_then(|desk| desk.0.lock().ok().map(|desk| desk.try_iter().collect()))
    .unwrap_or_default();
  for Request { line, reply } in requests.into_iter() {
    info!("admin: {line}");
    let answer = run(world, &line).unwrap_or_else(|blame| format!("error: {blame}"));
    let _ = reply.send(answer);
  }
}

pub struct Admin;

impl Plugin for Admin {
  fn build(&self, app: &mut App) {
    if opts().serve.is_some() || opts().host.is_some() || opts().admin.is_some() {
      app.add_systems(Startup, open.run_if(authority)).add_systems(
        Update,
        obey.run_if(resource_exists::<Desk>).run_if(resource_exists::<Voxels>)
      );
    }
  }
}
