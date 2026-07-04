# Changelog

Version shown in-game (bottom of the board) is wired to the crate version in
`Cargo.toml`, so what you see deployed always matches an entry below.

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
