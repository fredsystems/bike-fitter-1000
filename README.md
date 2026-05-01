# bike-fitter-1000

A bike-fit solver and side-view visualizer.

Given a target bike fit (saddle position and bar/stem-end position relative
to the bottom bracket) and a frame's geometry, recommends the spacer stack
height and stem (length + angle) needed to land the cockpit on your fit.

Status: early scaffold.

## Building

This project is set up as a Nix flake with a Rust dev shell:

```sh
nix develop
cargo run
```

Or build the package directly:

```sh
nix build
./result/bin/bike-fitter-1000
```

## Layout

- `crates/bike-fit-core/` — pure logic: types, geometry math, fit solver.
- `crates/bike-fit-app/` — egui GUI app.
- `data/` — bundled bike database (JSON).
- `docs/` — reference images and math derivations.
