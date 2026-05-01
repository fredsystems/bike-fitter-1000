# bike-fitter-1000

A bike-fit solver and side-view visualizer.

Given a target bike fit (saddle position and bar-clamp-face position
relative to the bottom bracket) and a frame's geometry, recommends the
stem (length + angle) and headset spacer stack height needed to land the
cockpit on that fit. Renders a side-view of the resulting bike against
the published frame geometry so you can sanity-check the result by eye.

Built primarily as a tool for spec'ing a second bike against an already-
validated fit on a first bike — the on-screen "compare" mode draws both
frames against the same fit profile and shows the cockpit each one needs.

Runs as a native desktop app (Linux/macOS/Windows) and as a wasm bundle
in a browser. Same code, same UI.

## Building

This project is set up as a Nix flake with a Rust dev shell. The shell
includes the wasm toolchain (rustup wasm32 target, trunk, wasm-bindgen-cli,
binaryen) so both targets work out of the box.

### Native

```sh
nix develop
cargo run -p bike-fit-app
```

Or via the flake's package:

```sh
nix build
./result/bin/bike-fitter-1000
```

### Web

```sh
nix develop
cd crates/bike-fit-web
trunk serve   # http://127.0.0.1:8080, live-reload
trunk build --release   # static bundle in crates/bike-fit-web/dist/
```

The web build is a self-contained static bundle — no backend, no runtime.
State persists in `localStorage`. Sharing a fit is done via a URL fragment
(`#fit=<base64>`) that seeds the app on load.

## Layout

- `crates/bike-fit-core/` — pure logic: types, geometry math, fit solver.
- `crates/bike-fit-ui/` — shared egui UI (renderer, panels, app shell).
- `crates/bike-fit-app/` — native binary; thin shell over the shared UI.
- `crates/bike-fit-web/` — wasm cdylib + trunk pipeline for the web build.
- `data/bikes.json` — bundled frame database (compiled in via `include_str!`).
- `docs/geometry-math.md` — derivation of the geometry equations.
- `docs/reference-rendering.png` — visual style target.
- `AGENTS.md` — conventions, decisions, milestones, dep policy.

## Status

All milestones 1–9 complete: native + web builds, frame editor, fit editor,
cockpit picker, brute-force solver, side-by-side comparison, bundled
bike DB, persistence, URL-fragment sharing (consumer side).

37/37 unit tests pass. See `AGENTS.md` for the full milestone list and
the project's design decisions.

## License

Dual-licensed under MIT or Apache-2.0 at your option.
