# AGENTS.md

Operational context for any agent (or human) working on **bike-fitter-1000**.
This is the single source of truth for project conventions, decisions, and
domain math. Update it whenever a decision changes.

---

## 1. Project goal

A small desktop application that helps a rider determine, for a chosen bike
frame, the **stem length, stem angle, and number/height of headset spacers**
required to land their cockpit on a previously-validated "ideal fit"
position.

The motivating use case: the user has a current bike that fits well and is
building a second bike. They know exactly where their saddle and bar tops
need to live in space, and they need to spec the new bike's components to
hit those points within a few millimeters — close enough that a
professional fitter has a clear starting point.

## 2. Headline decisions

| Decision                  | Choice                                                                                 |
| ------------------------- | -------------------------------------------------------------------------------------- |
| Language / GUI            | **Rust + egui** (via `eframe`); same UI code targets native and web                    |
| Scope of v1               | Full thing: bundled bike DB + multiple fit profiles + side-by-side comparison          |
| Native target             | Built and prioritized first; primary development target through milestone 8            |
| Web target                | wasm32 + WebGL via `eframe`, added as milestone 9 once native is feature-complete      |
| Web sharing               | Self-contained URL fragment encoding the fit+frame — no backend                        |
| Web hosting               | TBD (decided when we build the web target)                                             |
| Fit input model           | **BB-relative coordinates** for both saddle and bar target points                      |
| Units                     | Millimeters and degrees only (no inch toggle)                                          |
| Bike DB                   | Hand-curated JSON in `data/bikes.json`, compiled in via `include_str!` for portability |
| Reference rendering style | Bike Insights "bike-on-bike" line-art (see `docs/reference-rendering.png`)             |
| Stem angle convention     | Angle relative to perpendicular-to-steerer (industry standard)                         |
| Default spacer SKUs       | `[3, 5, 10, 20]` mm, max stack 60 mm                                                   |
| Default stem catalog      | Lengths 70–130 mm in 10 mm steps; angles ±6° and ±17°                                  |
| Headset top cap default   | 5 mm, configurable per build                                                           |

## 3. Coordinate convention

- **Origin:** bottom bracket (BB) center.
- **+X:** forward (toward the front wheel).
- **+Y:** up.
- All public APIs use millimeters and degrees. Convert to radians at the
  boundary inside math functions.

## 4. Domain definitions

### 4.1 The "bar target point"

The point we solve for is **the geometric center of the stem's far clamp
face** — i.e. where the stem clamps the handlebar — expressed in
BB-relative coordinates. Conceptually: _if you removed the handlebar
entirely and looked at the front face of the stem, where in space is the
center of that face?_

- Traditional stem + bar: literally the bar-clamp bolts' center.
- Aero stem + separate bar: same.
- Fully integrated (bar bonded to stem): the _virtual_ equivalent point
  where the bar emerges from the stem portion. Manufacturers spec the
  stem-portion length on the part itself (e.g. "100 mm × 42 cm cockpit").

**Why this point and not the hoods:** it makes the fit profile
bar-agnostic. A rider can swap bars without redoing the fit, as long as
the stem-clamp-face lands at the same target. The bar's own reach/drop is
a property of the bar/cockpit part, kept separate, and used for rendering
and as a sanity-check on hood position (post-v1).

### 4.2 The "saddle target point"

The user-recorded saddle position in BB-relative coordinates: typically
`x` is negative (saddle behind BB) and `y` is positive (saddle above BB).
This is the saddle reference point used by professional fitters (often
called HX/HY or SX/SY).

### 4.3 Stem angle

Measured **relative to a line perpendicular to the steerer axis**.

- `0°`: stem points perpendicular to the steerer.
- `+°` (positive): stem rises above the perpendicular line.
- `−°` (negative): stem drops below the perpendicular line.

A "−6° stem" is the common default. Flipping a stem inverts its angle.

### 4.4 Cockpit kinds

| Kind          | Description                                                                                                                               |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Traditional` | Separate stem (length × angle from catalog) + separate bar. Spacers freely combinable.                                                    |
| `AeroStem`    | Stem is one SKU per length with a fixed angle (often 0° or −6°), bolted to a separate bar. Spacers normal.                                |
| `Integrated`  | Stem and bar are one bonded part. Each SKU has fixed length, angle, bar reach, bar drop. Spacer system may be limited (modeled per part). |

The solver respects the cockpit kind: for `Traditional` it searches the
full stem catalog; for `AeroStem` and `Integrated` it searches only the
constrained SKU list.

## 5. Geometry math (summary)

Full derivation in `docs/geometry-math.md`. Quick reference:

```text
top_of_HT       = (reach, stack)
steerer_up      = (-cos(HTA), sin(HTA))
forward_perp    = ( sin(HTA), cos(HTA))           # ⟂ to steerer, points forward
bottom_of_HT    = top_of_HT + HTL * (cos(HTA), -sin(HTA))
top_of_ST       = STL * (-cos(STA), sin(STA))
rear_axle       = (-sqrt(chainstay² - bb_drop²), bb_drop)
front_axle      = (front_center_horizontal, bb_drop)   # if provided

clamp_origin    = top_of_HT + (top_cap + spacer_total) * steerer_up
stem_dir        = rotate(forward_perp, stem_angle_deg)
stem_clamp_face = clamp_origin + stem_length * stem_dir   # the bar target
```

## 6. Fit solver

Given a `Frame`, a `bar_target` in BB coords, and a `Cockpit` config:

1. Enumerate all reachable `spacer_total` heights from non-negative integer
   combinations of the spacer SKU catalog up to `max_stack`.
2. For each `(stem, spacer_total)` pair, compute `stem_clamp_face`.
3. Track the combination minimizing `‖stem_clamp_face − bar_target‖`.
4. Return the best stem, the spacer SKU breakdown that produces the chosen
   total, and a residual error vector so the UI can display
   "1.3 mm forward / 0.4 mm high of target".

The search space is small (~10 stems × ~50 spacer combos), so we just
brute-force it. No need for analytic inversion.

## 7. Repository layout

```text
bike-fitter-1000/
├── flake.nix                       # Nix dev shell + buildable packages (native + wasm)
├── flake.lock
├── Cargo.toml                      # Workspace, dependency pinning
├── Cargo.lock                      # Committed (binary crate)
├── crates/
│   ├── bike-fit-core/              # Pure logic, no UI, no platform deps
│   │   ├── src/lib.rs              # Point, common types, re-exports
│   │   ├── src/frame.rs            # Frame, derived points
│   │   ├── src/fit.rs              # FitProfile (saddle + bar targets)
│   │   ├── src/cockpit.rs          # Stem, Spacer, Cockpit kinds, catalogs
│   │   ├── src/geometry.rs         # Vector math + derived geometry
│   │   └── src/solver.rs           # solve_for_target, BuildResult
│   ├── bike-fit-ui/                # Shared egui UI: eframe::App impl, renderer, panels
│   │   ├── src/lib.rs              # App struct, Persistence trait
│   │   ├── src/render.rs           # 2D side-view painter (matches reference)
│   │   └── src/ui/                 # Frame editor, fit editor, solver panel
│   ├── bike-fit-app/               # Native bin `bike-fitter-1000`: thin shell + FilePersistence
│   │   └── src/main.rs
│   └── bike-fit-web/               # Wasm entrypoint (added at milestone 9): LocalStoragePersistence
│       └── src/lib.rs
├── data/
│   └── bikes.json                  # Bundled bike database (compiled in via include_str!)
├── docs/
│   ├── reference-rendering.png     # Visual style target
│   └── geometry-math.md            # Full math derivation
└── AGENTS.md                       # ← this file
```

## 8. Toolchain & build

- **Devshell:** `nix develop`. The shell pins a Rust toolchain and exports
  `LD_LIBRARY_PATH` for egui/winit/wgpu runtime libs (libxkbcommon,
  wayland, libGL, vulkan-loader on Linux).
- **Build:** `cargo build` inside the devshell, or `nix build` for the
  packaged binary.
- **Run:** `cargo run` inside the devshell.
- **Pre-commit:** the `.pre-commit-config.yaml` symlink is provided by the
  upstream `precommit` flake input. Hooks include rustfmt, clippy,
  codespell, nixfmt, markdownlint, prettier, statix, deadnix,
  shellcheck-bash, and `no-commit-to-branch` blocking direct commits to
  `main`.
- **Codespell:** expects a `.dictionary.txt` at repo root (currently
  empty, populate as needed).

## 9. Dependency policy

- **All third-party deps are pinned to an exact patch version** using
  `=x.y.z` in `[workspace.dependencies]` of the root `Cargo.toml`.
- **All workspace member crates use `.workspace = true`** for every shared
  dependency. The only non-workspace dependency lines permitted in member
  crates are intra-workspace `path = ".."` references.
- Bumping a dep is a deliberate, reviewable change to the workspace
  manifest. `cargo update` should not silently drift versions.

## 10. Branching & commits

- The pre-commit hook `no-commit-to-branch` forbids direct commits to
  `main`. Always work on a feature branch.
- Current working branch: `scaffold-initial`.
- Commit messages follow plain prose, focused on _why_ over _what_; the
  diff explains the _what_.

## 11. Milestones

Tracked in the running todo list. Status as of last update:

1. ✅ **Repo scaffold** — flake cleanup, Cargo workspace, first commit.
2. 🟡 **`bike-fit-core`** — types, geometry math, solver, unit tests.
3. ⬜ **Side-view renderer** matching the reference style. Lives in
   `bike-fit-ui` (split out from the start so the web target is cheap
   later).
4. ⬜ **Frame editor UI** with live preview.
5. ⬜ **Fit profile editor** with saddle/bar dot overlay.
6. ⬜ **Cockpit picker + solver output panel**.
7. ⬜ **Persistence trait + bundled `bikes.json`**. Native impl writes
   JSON to a config dir; bundled bikes use `include_str!` so the same DB
   ships in both targets.
8. ⬜ **Side-by-side comparison view**.
9. ⬜ **Web target**: `bike-fit-web` wasm crate, `LocalStoragePersistence`,
   URL-fragment encoding for shareable fits, `trunk` build, hosting TBD.

## 12. v1 simplifications (intentionally punted)

- Fork axle-to-crown / steerer length inside the head tube. We assume the
  steerer is collinear with the head tube and the spacer stack starts
  immediately above the top of the head tube (with the headset top cap
  contribution accounted for separately).
- Rider-weight sag, tire pressure effects on outer diameter.
- Crank length / saddle height interaction. Saddle height is taken as
  user-supplied.
- Bar reach/drop into the hoods point. Solver targets the stem-clamp face
  only. Bar shape will be rendered as overlay metadata, post-v1.
- Saddle setback nuances (post setback, saddle rail travel, saddle model
  variations) are collapsed into a single signed `rail_offset` number.

## 13. Test data

The reference frame for hand-checking math (a 2025 Canyon Aeroad CF SLX 7,
size 2XS, per the Bike Insights page used for the reference image):

```text
stack                       498 mm
reach                       372 mm
head tube angle              70°
head tube length             88 mm
seat tube angle              73.5°
seat tube length            441 mm
top tube effective          516 mm
bb drop                      70 mm
chainstay                   410 mm
chainstay horizontal        404 mm
front-center horizontal     571 mm
fork rake                    40.6 mm
wheel size                  700C/29 (BSD 622 mm)
tire width                   28 mm
crank length                165 mm
```

Use this in unit tests — for example, `sqrt(410² − 70²) ≈ 403.98 ≈ 404`
validates the rear-axle horizontal distance.
