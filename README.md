# War.v2

War.v2 is a browser-hosted 3D RPG strategy prototype built with the Bevy game engine and packaged for Vercel. It models a World War 1 era battlefield where players set up and command opposing forces, then watch a real-time simulation unfold.

## Gameplay prototype

- Low-poly 3D battlefield with trenches, infantry formations, clouds, smoke, and dust-like impact particles.
- Two sides, Entente and Central, spawn across no man's land and use simple battlefield AI to advance, fire, lose morale, and absorb damage.
- Real-time command layer: press `1` or `2` to choose a side, use arrow keys to push the selected group, and use `WASD` to pan the observer camera.
- Lightweight rigid-body-style motion with gravity, drag, projectile travel, impact effects, and bounded battlefield movement.

## Local development

```bash
cargo run
```

## Web build

Install the wasm target and `wasm-bindgen-cli`, then run:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --locked
bash scripts/build-web.sh
```

The static web build is emitted to `dist/` and is configured for Vercel by `vercel.json`.
