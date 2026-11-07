# Measure

Status: R1 spec (outline for thickness). Features: `measure.relations`, `measure.thickness`.

Relations between fitted elements, in mm and degrees:

- plane-plane: angle; when near-parallel (under 1°), distance between them measured at the first
  plane's centre.
- point/sphere centre to plane: signed distance.
- point to point, centre to centre: distance and its x/y/z components.
- cylinder axis to plane: angle; axis to axis: angle and closest distance (parallel case handled).
- cylinder to cylinder, parallel: centre-line distance (hole spacing).
Each result says which elements it used and, for near-degenerate cases, which convention applied.

Thickness (R3, outline): distance through the material along the inward normal to the opposite wall.
