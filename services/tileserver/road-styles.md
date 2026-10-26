Headway road hierarchy
======================

Surface-road rules in the generated Headway style.
Lightness ranges are `(at first stop .. at z20)` as fractions; for example,
`0.60 .. 0.40` means `hsl(0, 0%, 60%) .. hsl(0, 0%, 40%)`.

Zoomed out, roads are drawn at cartographic pixel widths. From z16 they are
drawn true to scale, at the width mapped in OSM (`width`), else the class
default below. Scale is exact at the 45th parallel; a style can't see latitude,
so elsewhere roads are off by up to a fifth between the 33rd and 55th
parallels.

Casing padding is per side: `fill width + (2 × padding) = casing width`. Once
true to scale the casing is the curb: 1px per side.

Class / rule           min_zoom  default width  fill lightness    casing padding  casing lightness
---------------------  --------  -------------  ----------------  ------------  ----------------
motorway                     5  11 m           0.68 .. 0.52      0.2 .. 1       0.60 .. 0.40
trunk                        6  13 m           0.68 .. 0.61      0.7 .. 1       0.62 .. 0.49
primary                      8  13 m           0.68 .. 0.64      0.7 .. 1       0.67 .. 0.52
motorway link / ramp         8   6 m           0.68 .. 0.52      0.37 .. 1      0.64 .. 0.40
secondary / tertiary        12  12 / 10 m      0.90 .. 0.74      0.38 .. 1      0.82 .. 0.64
minor / residential         12   8 m           0.86 .. 0.74      0.56 .. 1      0.86 .. 0.64
other link / ramp           12   6 m           0.88 .. 0.74      0.56 .. 1      0.86 .. 0.64
service / track             13   4 m *         0.84 .. 0.74      0.38 .. 1      0.88 .. 0.64
path / pedestrian           13  0.75 .. 10 px  0.96 .. 0.96**    --             --

\* driveway 3.5 m, alley 4.5 m, parking aisle 6 m.

Headway's tile additions
------------------------

Headway's tiles add to the OpenMapTiles transportation layer, from z14:

- `width`, as mapped.

Trunk and primary at low zoom
-----------------------------

Below z6 the road geometry comes from Natural Earth, which classifies
everything as motorway. Tiles switch to OSM-derived classes at z6 — that is the
first zoom at which a road is really known to be a trunk or a primary, and so at z6
these trunk|primary matches motorway.

Notes:
- Every class darkens monotonically as zoom increases. Where a class appears
  below its first color stop the value is clamped.
- Secondary and tertiary share one layer and therefore both begin at z12.
- All casing layers are gated to z13; except motorway at z10. Trunk and primary
  casings start their stops at that gate, so their padding column reads from
  z13; the rest read from the row's min zoom.
- Bridge variants use the same width and color rules as these surface layers.
- Tunnel variants share the widths but lighten every color, moving each one
  halfway to white: `L + (1 - L) / 2`. That is relative to the headroom a color
  has left, so it never clips and never reorders the hierarchy. Rail tunnels are
  lightened the same way. Motorway tunnel casings additionally use a dash
  pattern.
- Paths carry no surface casing; only the bridge variant has one.
- **Paths use the shared neutral hue and a dash pattern.
- Solid road and tunnel lines use `line-cap: round`. Only `line-join` smooths
  vertices within a feature, so the cap is what covers the seam where two
  features abut; `butt` leaves a notch at every one of them. The cost is that a
  round cap overhangs half a width past a tunnel portal, which the lighter
  tunnel color shows as a bulb. That is accepted: the seams are everywhere and
  the portals are not.
- Bridge casings are the exception and stay on `butt`. They are the only
  casings drawn above the surface fills, so a round cap there puts a dark blob
  on the road at each end of the bridge instead of hiding under the fill drawn
  after it. Bridge fills are round like the rest, so neighboring bridge
  features of different widths or directions don't leave notches.
- Dashed lines keep the `butt` default: paths, rail hatching and the dashed
  tunnel casings. Dash lengths are multiples of the line width and a round cap
  adds half a width at each end, which would close gaps of 0.25 to 0.75 widths
  and collapse those patterns into solid lines.

Layer order: within each of the tunnel, road and bridge groups the casings are
drawn as a block beneath that group's fills, and the groups stack
tunnel < road < bridge, so a road crossing over a tunnel renders above it.

Hoisting every casing into one block ahead of all fills would put bridge casings
under surface fills, which erases a bridge's outline exactly where it crosses
another road.
