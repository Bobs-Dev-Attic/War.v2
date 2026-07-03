# Changelog

Version shown in-game (bottom of the board) is wired to the crate version in
`Cargo.toml`, so what you see deployed always matches an entry below.

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
