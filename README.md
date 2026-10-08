# Bloxy

![Two players in the same world](docs/multiplayer.png)

A browser-first multiplayer voxel sandbox in Bevy, heading toward the feel of FTB-modded Minecraft. Procedural terrain with caves, ore veins (coal, copper, tin, iron, gold, diamond) and forests; mining and building; other players over WebSockets. No mesh or texture assets: everything is generated.

- Play solo: `cargo run` (or open the web build).
- Run a server: `BLOXY='{serve: 7777}' cargo run`.
- Join: `BLOXY='{connect: "ws://host:7777", name: "Ann"}' cargo run`, or in the browser `?connect=wss://host&name=Ann` (browsers on https need `wss://`, e.g. behind Caddy).

Controls: click to capture the mouse, WASD, Space jump/swim, Ctrl sprint, hold left mouse to dig, right mouse to place, 1–9 or the wheel for the hotbar, Esc to release the mouse. Add `creative: true` to the opts for a hotbar of building blocks.
