# Segmentation

Status: *Outline*. Feature: `segment.surfaces`.

Split a mesh into regions that are each one plane, cylinder or sphere, grown from the flattest places
outward within an angle and RMS limit, so a user or agent can say "the top face" or "that hole"
without marking vertices by hand. Output: regions largest first, each with its kind, fit, area,
bounding box and neighbours. Round surfaces that come out as strips are joined into one region.
