# Bloxy
A browser-first multiplayer voxel sandbox in Bevy 0.19.1, aiming at the feel of FTB-modded Minecraft: procedural terrain with ores, caves and trees, mining and building, other players around you, and (later) machines, power and automation. Everything is procedural: no mesh or texture assets.

# Roles
`protocol::Role` (from opts): `Solo` (no network, local authority), `Host` (native window + WebSocket server), `Dedicated` (headless server, `MinimalPlugins`), `Guest` (connects to a server; the browser build is usually this or `Solo`). `authority` systems run for every role but `Guest`; `plays` systems for every role but `Dedicated`. Use these, not replicon's states, to gate game logic: replicon's `ClientState::Disconnected` also covers a guest that lost its connection.

# Modules
- `block.rs` — `Block` (u8 repr, `look` → `Opaque/Cutout/Liquid/Invisible`, `solid`, `breakable`, `seconds_to_break`, `drop`, `tiles` = top/side/bottom `Tile`s). Add a block: variant + every match, and a `Tile` with its `paint`.
- `noise.rs` — integer-hash value noise and Perlin (`perlin2`, `perlin3`, `fbm2`, `unit`, `hash`). Only `+ - * floor` and integer ops, so wasm and native generate bit-identical worlds; never use `sin`/`exp`/`powf` in anything that feeds generation.
- `generate.rs` — `height(seed, x, z)` (continents, hills, ridged highlands), `column` (grass/sand/clay/gravel/snow/stone cover), `chunk(seed, key)` (bedrock, water below `SEA`, caves as interpolated 3D noise on a 4-block grid, ore `veins` seeded per neighbouring chunk so they cross borders, trees seeded per column within `TREE_REACH`), `spawn_point`. Pure: clients generate the same world from the seed, the server only sends edits.
- `voxels.rs` — `Voxels` resource: `chunks` (32³ `Chunk`, `Uniform` or `Mixed`, `Arc` so meshing tasks can share them), `edits` (overlay applied on every `insert`, the source of truth that is networked), `dirty` chunk keys to remesh, `set`, `ensure` (generate synchronously, server side), `cast` (voxel DDA). World is 8 chunks tall (y 0..256), bedrock below, air above.
- `texture.rs` — 16 px pixel-art tiles painted by `paint(tile, x, y) -> Texel { color, glow }` into an 8×4 atlas with a hand-built mip chain (tiles stay aligned down to 1 px, so no bleeding), nearest sampling; `albedo()` and `glow()` (emissive) atlases, `uv_corner`.
- `mesh.rs` — `Padded::gather` copies a chunk plus a 1-block border from its 26 neighbours, `build` emits culled faces (no greedy meshing: the atlas can't repeat) with per-vertex ambient occlusion (quad flipped against anisotropy) times a sky term (depth below the generated surface, so caves go dark), split into `solid` (alpha-masked) and `liquid` meshes; water tops are lowered.
- `stream.rs` — client chunk streaming around the `Pilot`: generation and meshing in `AsyncComputeTaskPool` tasks, nearest first, a few in flight (fewer on wasm); chunks beyond `reach + KEEP_BEYOND` dropped (edits survive); `Palette` materials; `Progress.pending` (screenshots wait for 0).
- `protocol.rs` — everything both ends register in the same order: replicated `Player { name, tint }`, `Avatar { at, yaw, pitch }`, `Inventory` (hotbar `Stack`s); client messages `Hello`, `Moved`, `Dig`, `Put`; server messages `Welcome { seed, edits }`, `Possess(Entity)` (mapped), `Altered`. `Role`, `authority`, `plays`.
- `authority.rs` — the server: founds the world, spawns a player per authorized client (`Controller(ClientId)`, plus one for the host itself), sends `Welcome` + `Possess`, applies `Moved`, validates `Dig`/`Put` (reach, inventory, not inside a player) and broadcasts `Altered`, or sends the true block back to the one client to undo its prediction.
- `net.rs` — replicon messaging backend over WebSockets: one binary frame per message, first byte the channel. Server (native only): non-blocking `TcpListener`, handshakes on threads, a `Link` per `ConnectedClient`. Client: `tungstenite` natively (`ws://` only), `web_sys::WebSocket` in the browser; held as the non-send `Uplink`. Drives `ClientState`/`ServerState`.
- `player.rs` — client side of the local player: `Pilot` (client-authoritative position, sent as `Moved` at 20 Hz), fixed-step AABB physics against voxels (unloaded chunks are solid; frozen until the chunks around are loaded), swimming, mouse look (click to grab, Esc to release), `Aim` (target, digging progress), dig (hold LMB) / place (RMB) with prediction for guests, hotbar `Selected` (1–9, wheel). `arrive` builds `Voxels` from `Welcome`.
- `figure.rs` — other players as blocky figures (tinted shirt, head turned by pitch, swinging limbs, smoothed toward `Avatar`), UI name tags projected over their heads.
- `hud.rs` — crosshair, digging progress bar, hotbar with atlas icons and counts, status line (role/connection, players, position, target, fps, chunks loading), block outline.
- `sky.rs` — physical atmosphere, sun with cascaded shadows, `lens()` (exposure, ACES, bloom, distance fog that hides the streaming edge).

# Opts
Natively the env var `BLOXY` holds JSON5 `opts::Opts`; in the browser the URL query (`?connect=wss://host:port&name=Ann`, or `?opts={…}` for anything else). Fields: `connect` (server URL: guest), `serve: port` (headless dedicated server), `host: port` (play and serve), `name`, `seed` (1), `reach` (view radius in chunks; 9 native, 6 web), `hour` (10), `creative` (starter hotbar with stacks of building blocks, fast digging), `shot: <secs>` (screenshot to `screenshots/shot-$SHOT_NAME.png` once loaded, then exit), `at: [dx, dz]` (spawn offset from the spawn point, on the ground), `yaw`, `pitch` (degrees), `press: [[secs, 'LMB'|'RMB'|'W'|'A'|'S'|'D'|'Space'|'Ctrl'|'Shift'|'1'…'9'], …]` (taps of 0.15 s), `hold: [[from, to, key], …]`.
e.g. `BLOXY='{shot: 2, yaw: 40, pitch: -15}' SHOT_NAME=valley tools/shot`. Screenshots are the way to check visuals — always look at them.

# Multiplayer
- Dedicated server: `BLOXY='{serve: 7777}' cargo run --features dev`. Browser pages served over https (GitHub Pages) can only open `wss://`, so put the server behind a TLS proxy (e.g. Caddy `reverse_proxy localhost:7777`) and join with `?connect=wss://your.host`. Native clients use `ws://host:7777`.
- `tools/duo` runs a dedicated server and two clients under xvfb and screenshots the second seeing the first: the quickest end-to-end check of netcode.
- Movement is client-authoritative; blocks, inventories and player list are server-authoritative.
- World edits are kept in memory only (not saved yet); restarting the server resets the world.

# Build
Priority is fast incremental builds over runtime speed. Debug builds only; release only when asked.
- `cargo check` to verify compilation; `cargo run --features dev` to play (`dev` = Bevy dynamic linking).
- `tools/shot <name> '<opts>'` builds and screenshots under xvfb.
- Cloud sessions have no GPU: wrap runs in `xvfb-run -a -s "-screen 0 1600x900x24"` (lavapipe, software Vulkan, slow frames).
- Bevy runs without default features: `Cargo.toml` lists only what the game uses. Add a feature there when using a new Bevy part.
- Web build: `trunk build --release` (`index.html`), deployed to GitHub Pages on master push. Needs WebGPU.
- On a panic or startup failure (including Bevy system param conflicts), rerun with `RUST_BACKTRACE=1`.
- Docs: https://docs.rs/bevy/0.19.1/bevy/ , https://docs.rs/bevy_replicon/0.44.3/bevy_replicon/

# Style
Code structure should mirror the thinking behind it: translate the user's conceptual framework fairly directly into code. Functional-ish.
- Format with `cargo +nightly fmt` (repo `rustfmt.toml` uses nightly-only options). Avoid comments; name things well.
- Order within a file: if A refers to B, A comes after B. Keep related code together.
- Semicolons are a verbosity signal: prefer expressions over statements.
- No `return`, `continue`, `let … else`, or early exits.
- Prefer let chains over nested `if let`/`if` or `.and_then()`.
- Prefer `Option` methods over matching on `Option` (not so for most other enums).
- Prefer `find`, `find_map`, `any`, `fold` over loops with break (socket read loops are the exception).
- Eagerly destructure: `let &Thing { field } = thing()`.
- Domain model with named instances, newtypes and associated consts.
- No more function-calls-function indirection than needed.
- ECS: merge components that always co-occur. Name components by behaviour, not entity.

# Collaboration
- Don't implement features that weren't asked for. Ask when unclear.
- If a requested approach can't be done, say so; never change direction without asking.
