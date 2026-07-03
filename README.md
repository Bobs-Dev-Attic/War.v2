# War.v2

A small [Bevy](https://bevyengine.org/) demo compiled to **WebAssembly** and
hosted on **Vercel** — built to test how well Bevy works as a web target.

The scene is a 2D Bevy `App`: floating "cards" drift around and bounce off the
edges, and clicking deals more cards at the cursor. It exercises the real Bevy
render loop, sprites, transforms, text, and mouse input, all running in the
browser.

## Why the previous deploy 404'd

The repository originally contained only `README.md`. Vercel built it fine but
had no `index.html` / framework output to serve, so every route returned
`404 NOT_FOUND`. This project adds an actual application.

## How it's deployed

Vercel does **not** compile Rust. Instead the WebAssembly is built ahead of time
and the static output in `dist/` is committed and served directly:

- `vercel.json` sets `outputDirectory: dist` with a no-op build command, so
  Vercel just serves the pre-built files.
- `dist/` contains `index.html`, the `*.wasm` module, and the JS glue generated
  by `wasm-bindgen`.

Pushing to `main` triggers a production deploy; pushing to any other branch
triggers a Vercel preview deploy.

## Building locally

Prerequisites: a Rust toolchain (**1.95+**, required by Bevy 0.19), the wasm
target, and `wasm-bindgen-cli` at the version pinned in `Cargo.lock`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version =0.2.126

# Build the WebAssembly and assemble ./dist:
./build.sh

# Preview it locally (any static server works):
python3 -m http.server -d dist 8080   # then open http://localhost:8080
```

After running `./build.sh`, commit the regenerated `dist/` so Vercel picks up
the new build.

## Layout

| Path           | Purpose                                             |
| -------------- | --------------------------------------------------- |
| `src/main.rs`  | The Bevy app (scene, systems, input).               |
| `index.html`   | HTML shell that loads the wasm glue onto the canvas.|
| `build.sh`     | Builds the wasm and assembles `dist/`.              |
| `Cargo.toml`   | Bevy dependency (trimmed features) + wasm profile.  |
| `vercel.json`  | Tells Vercel to serve the pre-built `dist/`.        |
| `dist/`        | Committed static output that Vercel deploys.        |
