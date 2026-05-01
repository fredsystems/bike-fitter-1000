# Geometry math reference

This document derives the formulas used in `bike-fit-core`. It exists so the
code can stay terse and the math can stay reviewable.

## Conventions

- **Origin:** bottom bracket (BB) center.
- **+X:** forward (toward the front wheel).
- **+Y:** up.
- All distances in millimeters, all angles in degrees in public APIs;
  internally we convert to radians at the boundary.

```text
        +Y
         ^
         |
   <-----+----->  +X
  (back) |  (front)
         |
```

## Frame angles

Standard bike-industry convention: **angles are measured from the ground**,
and tubes lean *backward* relative to a straight-up vertical.

- **Head tube angle (HTA):** `70°` means the head tube/steerer leans 20°
  back from vertical. The top of the head tube is *behind* (less forward
  than) the bottom of the head tube.
- **Seat tube angle (STA):** `73.5°` means the seat tube leans 16.5° back
  from vertical. The top of the seat tube is *behind* the BB.

## Derived frame points

### Top of head tube

By definition of stack and reach:

```text
top_of_HT = (reach, stack)
```

### Bottom of head tube

The head tube goes *down and forward* from the top, along the head tube
axis. The axis has angle HTA from the ground, and the "downward" direction
along the axis is `(cos(HTA), -sin(HTA))`:

```text
bottom_of_HT = top_of_HT + HTL * (cos(HTA), -sin(HTA))
             = (reach + HTL * cos(HTA),  stack - HTL * sin(HTA))
```

### Steerer "up" unit vector

The steerer is collinear with the head tube and continues *up* from the
top of the head tube. The "up along steerer" direction is the negative of
the "down along axis" direction above:

```text
steerer_up = (-cos(HTA), sin(HTA))
```

Sanity check at HTA = 73° (steep):
`(-cos 73°, sin 73°) ≈ (-0.292, 0.956)` — almost straight up, slightly
back. ✓

### Top of seat tube

Seat tube goes *up and back* from BB along an axis at angle STA from the
ground. The "up along seat tube" direction is `(-cos(STA), sin(STA))`:

```text
top_of_ST = STL * (-cos(STA), sin(STA))
          = (-STL * cos(STA),  STL * sin(STA))
```

### Rear axle

The chainstay length is the straight-line distance from BB to rear axle.
The vertical component is the BB drop (BB sits below the axle line by
`bb_drop` mm, so the axle is at `+bb_drop` in our +Y-is-up frame):

```text
horizontal = sqrt(chainstay² - bb_drop²)
rear_axle = (-horizontal, bb_drop)
```

Sanity check with chainstay 410, bb_drop 70:
`sqrt(410² - 70²) = sqrt(168100 - 4900) = sqrt(163200) ≈ 403.98` mm,
which matches the published "Chainstay Length Horizontal = 404 mm". ✓

### Front axle

If the geo chart provides `front_center_horizontal`, use it directly:

```text
front_axle = (front_center_horizontal, bb_drop)
```

Otherwise we'd need fork axle-to-crown to compute it from HTA + fork rake,
which most published charts don't include. We make
`front_center_horizontal` an optional field; when absent we compute an
approximation from the steerer line projected to the wheel-axle height,
plus the fork rake offset perpendicular to the steerer. (See
`Frame::front_axle` in code.)

## Stem geometry

### Spacer clamp position

Spacers stack along the steerer above the top of the head tube. The
headset top cap (typically ~5 mm) also contributes:

```text
clamp_origin = top_of_HT + (top_cap + spacer_stack) * steerer_up
```

This is the point at which the stem clamps the steerer.

### Stem direction

Stems are spec'd by a *length* and an *angle*. The angle is conventionally
measured **relative to a line perpendicular to the steerer**. A stem at
angle `0°` would stick straight forward perpendicular to the steerer;
positive angles rise above that line, negative angles drop below it.

A "−6° stem" (the most common default) drops 6° below perpendicular.
Flipping the stem inverts the sign.

The "perpendicular to steerer, pointing forward" unit vector is
`steerer_up` rotated 90° clockwise:

```text
forward_perp = (sin(HTA), cos(HTA))
```

Sanity check at HTA = 73°: `(sin 73°, cos 73°) ≈ (0.956, 0.292)` — mostly
forward, slightly up. ✓ (At a steep HTA the stem points slightly upward
even at "0° angle".)

The stem direction is `forward_perp` rotated by `stem_angle_deg`:

```text
stem_dir = rotate(forward_perp, stem_angle_deg)
```

### Stem clamp face (the bar target point)

```text
stem_clamp_face = clamp_origin + stem_length * stem_dir
```

This is the point we solve to land on the user's bar target.

## Saddle solve

Given a target saddle position `target = (sx, sy)` in BB coords and the
seat tube axis going up from BB at angle STA:

```text
seat_axis_up = (-cos(STA), sin(STA))
```

Project `target` onto the seat axis to get how far along the seat tube
axis from BB the saddle sits, then take the perpendicular component to
get the rail offset (signed):

```text
along  = target · seat_axis_up
offset = target · perpendicular(seat_axis_up)   # signed: + means forward of axis
```

The seat post extends above the seat tube top by:

```text
post_extension_above_ST = along - seat_tube_length
```

Real-world a seat post has a setback of its own, and saddles have rail
travel — we collapse all of those into a single signed `rail_offset`
number for v1.

## The fit solver

Inputs:

- `Frame`
- `bar_target = (Tx, Ty)` in BB coords
- `Cockpit`: stem catalog (lengths × angles), spacer catalog (SKU heights),
  max stack height, headset top cap height

For each `stem` in catalog, for each reachable spacer total in
`[0, max_stack]` from non-negative integer combinations of SKUs:

```text
candidate = stem_clamp_face(frame, top_cap, spacer_total, stem)
err = ||candidate - bar_target||
```

Track the minimum-error combination. Return it along with the residual
error vector so the UI can show "you'll be 1.3 mm short / 0.4 mm low".
