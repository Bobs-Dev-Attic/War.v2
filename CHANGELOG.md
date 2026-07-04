# Changelog

Version shown in-game (bottom of the board) is wired to the crate version in
`Cargo.toml`, so what you see deployed always matches an entry below.

## v0.7.0

- **The battle is now live and commandable (real-time strategy).** You command
  the British; the Central army you placed is driven by AI.
- **RTS controls**: left-click a unit or left-drag a box to select your troops
  (selected units get a green ring); right-click the ground to move there in
  formation, or right-click an enemy to attack.
- **Full combat**: every armed unit auto-fires at the nearest enemy in range
  with a visible tracer, dealing damage; units die and are removed when their
  hit points reach zero. Per-type stats (hp, range, damage, rate of fire, speed)
  differentiate infantry, snipers, machine gunners, artillery, scouts (runners
  are unarmed couriers).
- **Enemy AI** advance-attacks the nearest player unit. A live **HUD** shows the
  selected count and running casualties for both sides.
- Sim runs continuously so the battle never stalls.

## v0.6.0

- **Deploy tab — top-down mini-map placement.** Pick a side (British / Central),
  pick a unit type, and left-click your half of the mini-map to place units;
  right-click removes the nearest. You position **both** armies.
- **Points budget**: each side has 60 points; unit types cost differently
  (infantry/runner 1, scout 2, sniper 3, machine gun 4, artillery 8). A live
  budget bar and per-type counts keep you honest. The mini-map is tinted by the
  chosen location and marks both trench lines and no-man's-land.
- **Start Battle builds exactly what you placed** — the 3D battle spawns your
  units at their mini-map positions (the auto-garrison remains only as a fallback
  when you place nothing).
- Placements are handed to the wasm battle as a compact `side:type:x:z` list.

## v0.5.0

- **Pre-battle setup page**: a tabbed HTML screen loads first (deferring the
  wasm download until you commit). Choose your **location** (The Somme, Ypres,
  Gallipoli, Sinai & Palestine, Verdun) and **weather** (clear, overcast, rain,
  fog, snow), then hit **Start Battle**.
- The battlefield is **generated to match**: each location sets the terrain
  palette and crater density; each weather sets the sky, lighting, ground
  wetness and distance fog. e.g. Gallipoli + Fog = sandy ground under a haze.
- The setup config is handed to the Bevy wasm battle via the browser globals the
  page sets before boot; the chosen location/weather is shown in-game.
- A **Deploy** tab is present as a placeholder — top-down mini-map unit placement
  for both sides lands in v0.6.0.

## v0.4.0

- **Trench warfare**: each side now has a sandbagged trench line — duckboard
  channel, crenellated front parapet and a rear parados — dug in on its side of
  no-man's-land.
- **Unit-type roster**: distinct low-poly models for infantry, snipers (kneeling,
  long rifle), machine gunners (crewed tripod gun), artillery (wheeled field gun
  with shield and crew), runners (satchel, no rifle) and scouts (cap,
  binoculars).
- Each side deploys a mixed garrison: infantry holding the fire trench, MG nests
  along it, snipers on the flanks, scouts pushed forward, runners behind, and an
  artillery battery in the rear.
- Foundation for interactive setup (v0.5.0) and real-time commands (v0.6.0).

## v0.3.1

- **Soldiers, not tanks**: the units you'll command are low-poly infantry —
  legs, uniformed torso, helmeted head (Brodie / Stahlhelm) and a shouldered
  rifle — replacing the placeholder tanks.
- Tighter infantry-scale formations and separation radius; more troops per side.
- Camera zoomed in and armies deployed closer to the line so the clash is framed
  from the opening shot.

## v0.3.0

- Pivot from the 2D grid to an **isometric 3D** battlefield: orthographic iso
  camera, low-poly muddy terrain with a no-man's-land strip and scattered
  craters, directional sun + shadows.
- Two low-poly tank armies — **British** (khaki) and **Central Powers /
  Ottoman-German** (field-grey) — deploy in formation and advance on the centre
  line, separating so no two vehicles overlap (carries the collision rule into
  continuous 3D space).
- Camera panning with WASD / arrow keys.
- Foundation for the WWI RTS: rigid-body physics, unit command and playable
  factions land in later versions.

## v0.2.0

- Show the version on screen (bottom-center badge) so the deployed build is
  identifiable at a glance.

## v0.1.0

- First playable build: a grid-based tank skirmish in Bevy 0.19, compiled to
  WebAssembly and served as static files on Vercel.
- Core rule: no two vehicles may occupy the same cell. The green player tank
  moves one cell per key press (arrow keys / WASD); red/orange AI tanks wander
  on a timer. Every move is rejected if the target cell is off-board or
  occupied.
- Fixed the empty-repo `404 NOT_FOUND` on Vercel by shipping an actual app.
