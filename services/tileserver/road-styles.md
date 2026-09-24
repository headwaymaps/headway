Road hierarchy styles (basic-v3)
================================

Surface-road rules in services/tileserver/assets/styles/basic-v3.json.
Lightness ranges are `(at first stop .. at z20)` as fractions; for example,
`0.60 .. 0.40` means `hsl(0, 0%, 60%) .. hsl(0, 0%, 40%)`.
Widths use the same range notation.

Casing padding is per side: `fill width + (2 × padding) = casing width`.

Class / rule           min_zoom  fill width    fill lightness    casing padding  casing lightness
---------------------  --------  ------------  ----------------  ------------  ----------------
motorway                     5   1.0 .. 36     0.68 .. 0.52      0.2 .. 2       0.60 .. 0.40
trunk                        6  1.0 .. 32     0.68 .. 0.61      0.7 .. 2       0.62 .. 0.49
primary                      8  1.0 .. 32     0.68 .. 0.64      0.7 .. 2       0.67 .. 0.52
motorway link / ramp         8  1.0 .. 32     0.68 .. 0.52      0.37 .. 1.5    0.64 .. 0.40
secondary / tertiary        12  1.0 .. 26     0.90 .. 0.74      0.38 .. 2      0.82 .. 0.64
minor / residential         12  1.0 .. 18     0.86 .. 0.74      0.56 .. 1      0.86 .. 0.64
other link / ramp           12  1.0 .. 20     0.88 .. 0.74      0.56 .. 1.75   0.86 .. 0.64
path / pedestrian           13  0.75 .. 10     0.96 .. 0.96*     --             --
service / track             13     0 .. 7.5    0.84 .. 0.74      0.38 .. 1.75   0.88 .. 0.64

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
- *Paths use the shared neutral hue and a dash pattern.
- Solid road and tunnel lines use `line-cap: round`. Only `line-join` smooths
  vertices within a feature, so the cap is what covers the seam where two
  features abut; `butt` leaves a notch at every one of them. The cost is that a
  round cap overhangs half a width past a tunnel portal, which the lighter
  tunnel color shows as a bulb. That is accepted: the seams are everywhere and
  the portals are not.
- Bridges are the exception and stay on `butt`. Bridge casings are the only
  casings drawn above the surface fills, so a round cap there puts a dark blob
  on the road at each end of the bridge instead of hiding under the fill drawn
  after it.
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
