# Changelog

Version shown in-game (bottom of the board) is wired to the crate version in
`Cargo.toml`, so what you see deployed always matches an entry below.

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
