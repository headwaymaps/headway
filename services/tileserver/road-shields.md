Road shields
============

Route shields in services/tileserver/assets/styles/basic-v3.json are drawn by the
`road_shield` layer. Exit numbers are a separate layer, `road_exit_shield`.

Where the artwork comes from
----------------------------

Blanks come from [OpenStreetMap Americana](https://github.com/osm-americana/openstreetmap-americana),
which is CC0-1.0 — public domain, no attribution required. Their `icons/`
directory holds ~230 `shield_*.svg` blanks and `src/js/shield_defs.js` records,
per network, which blank to use and how the text sits inside it. Both are worth
taking: the blanks are the easy part, the per-network padding is the knowledge.

To refresh the upstream copy:

    curl -sfL https://codeload.github.com/osm-americana/openstreetmap-americana/tar.gz/refs/heads/main \
      | tar -xz --strip-components=1 -C <dest> '*/icons/*' '*/src/js/shield_defs.js'

Only blanks for networks we actually draw are checked in. Anything in
assets/sprites/ lands in the sprite sheet every client downloads, so the set
stays to what the style references.

How ours differs from theirs
----------------------------

Americana composes the shield and its number into a single bitmap at runtime,
in the browser, via maplibre-gl-js's `styleimagemissing` event. That gives them
text fitted to each shape, route concurrencies and banners.

We don't do that. We use the blank as a plain sprite and let maplibre draw the
ref over it as ordinary glyph text. The reason is that maps.earth has a second
client: the iOS app renders the same style through maplibre-native, and a
browser-side renderer would leave it with no shields at all. Keeping shields in
the style document means every client gets them with no client code.

What that costs us:

- No route concurrencies. Where two routes share a carriageway, Americana draws
  both shields side by side by putting several `image` elements in one
  `text-field`. We draw one.
- No banners (ALT, BUS, TRK, SPUR above the shield).
- Their `textLayout` constraint functions — `ellipse`, `southHalfEllipse`,
  `triangleDown` — fit text to the shape itself. A style expression can't, so we
  centre the text and use their `padding` to offset it. Round and triangular
  shields are the ones where this shows.
- They discount narrow characters (`/[1IJijl .-]/`) as two-thirds of a character
  when choosing a blank width. Expressible in a style expression, but only by
  unrolling one block per character position; not currently done.

Fitting the text
----------------

Americana ships one blank for refs up to two characters and a wider one for
three, suffixed `_2` and `_3`. Past three characters they don't widen further,
they shrink the text — so `road_shield` does the same:

    "text-size": ["step", ["get", "ref_length"], 10, 4, 8.5, 5, 7]

Their `padding` is expressed in the 20px space their blanks are drawn at. Text
sits at the centre of whatever the padding leaves, so the offset from centre is
`(top - bottom) / 2` vertically and `(left - right) / 2` horizontally, scaled by
`icon-size / text-size` to convert into the ems `text-offset` wants.

That derivation is a starting point, not the answer. Their padding is tuned for
a renderer that fits text to the shape via `textLayout`, and on the shapes where
that constraint does real work the padding over-corrects for us. Washington and
Oregon are both `ellipse`, and both sat visibly high at the derived offset;
halving the upward offset landed them right. Expect the same on the other
`ellipse` and `triangleDown` networks — Kansas, Nevada, Guam, Navajo Nation.

  network  derived   in use
  -------  --------  ------
  US:WA    -0.24em   -0.12em
  US:OR    -0.18em   -0.09em

Finding a shield to check
-------------------------

Sampling one z12 production tile per metro turned up these, with real refs:

  US:NV  Las Vegas 36.17,-115.14      592
  US:NM  Albuquerque 35.08,-106.65    47
  US:FL  Orlando 28.54,-81.38         426 423 436
  US:MN  Minneapolis 44.98,-93.27     55 65 100
  US:UT  Salt Lake 40.76,-111.89      201 269 68
  US:LA  New Orleans 29.95,-90.07     428 46 39
  US:NH  Manchester 42.99,-71.45      28A 28 101
  US:PA  Philadelphia 39.95,-75.16    3 611
  US:MO  St Louis 38.63,-90.20        100 115
  US:AR  Little Rock 34.75,-92.29     10 365
  US:CO  Denver 39.74,-104.99         2
  US:IL  St Louis 38.63,-90.20        3
  US:SD  Sioux Falls 43.55,-96.73     115
  US:VT  Burlington 44.48,-73.21      15

NV and NM are the ones to check first: their blanks aren't 20px tall, so they
carry their own icon-size, and that adjustment is untested. NH 28A is the only
letter-suffixed ref in the sample, which is worth seeing against the width rule.

Networks
--------

`network` in the tiles is OpenMapTiles' coarse value — `us-interstate`,
`us-highway`, `us-state`. The specific state lives in `route_1_network` as
`US:WA`, `US:CA` and so on, which is what Americana keys its definitions by, so
`icon-image` matches on `network` first and `route_1_network` within `us-state`.

Recreational route relations put the network's scope in `network` rather than a
road network, so `lwn`, `rwn`, `ncn` and their siblings are filtered out of the
shield layer.

Status
------

`verified` means someone has looked at it on the map. `wired` means the blank
is installed and the style references it — it renders, it just hasn't been
looked at yet — these came in as a batch and are expected to
need the same kind of offset correction WA and OR did. Padding is in
Americana's 20px shield space.

Most blanks are 20px tall, so `icon-size: 1.2` puts them at ~24px rendered.
Nine are not — Americana draws some shapes larger for visual parity — and carry
their own `icon-size` so everything lands at the same height: BIA, CKC, DC, GU,
NHT, NM, NV, Navajo, PANYNJ.

Network    Americana blank(s)                                        text   layout            pad L R T B       status
---------  --------------------------------------------------------  -----  ----------------  ----------------  ----------
US:I       shield_us_interstate_2,shield_us_interstate_3             white  southHalfEllipse  4 4 6 5           verified
US:CA      shield_us_ca_2,shield_us_ca_3                             white  rect              4.0 4.0 6.0 4.0  verified
US:OR      shield_us_or_2,shield_us_or_3                             black  ellipse           1.0 1.0 1.0 4.0  verified
US:US      shield_badge_2,shield_badge_3                             black  rect              3 3 4 5  verified
US:WA      shield_us_wa                                              black  ellipse           2.0 3.0 2.0 6.0  verified
US:AK      shield_us_ak                                              black  rect              5.5 1.5 1.5 9.0    wired
US:AL      shield_us_al_2,shield_us_al_3                             black  rect              3.0 3.0 3.0 6.0    wired
US:AR      shield_us_ar_2,shield_us_ar_3                             black  rect              3.0 4.0 4.0 5.0    wired
US:AS      shield_us_as                                              white  rect              4.0 4.0 9.5 2.0    wired
US:AZ      shield_us_az_2,shield_us_az_3                             black  rect              4.0 3.0 3.0 4.0    wired
US:BIA     shield_us_bia                                             black  ellipse           1.0 1.0 5.0 7.0    wired
US:CKC     shield_us_ckc                                             black  rect              -  wired
US:CO      shield_us_co                                              black  rect              2.0 2.0 9.5 2.0    wired
US:DC      shield_us_dc                                              black  rect              2.0 2.0 10.0 4.0   wired
US:FL      shield_us_fl_2,shield_us_fl_3                             black  rect              2.0 4.5 6.0 4.0    wired
US:GA      shield_us_ga_2,shield_us_ga_3                             black  rect              3.0 4.0 5.0 4.0    wired
US:GLST    shield_us_glst                                            black  rect              -  wired
US:GRR     shield_us_grr                                             black  rect              -  wired
US:GU      shield_us_gu_2,shield_us_gu_3                             white  ellipse           1.0 1.0 4.0 4.0    wired
US:ID      shield_us_id_2,shield_us_id_3                             white  rect              5.5 1.5 1.5 9.0    wired
US:KS      shield_us_ks_2,shield_us_ks_3                             black  ellipse           2.0 2.0 2.0 2.0    wired
US:LA      shield_us_la_2,shield_us_la_3                             black  rect              2.5 2.5 7.0 3.0    wired
US:LHT     shield_us_lht                                             black  rect              -  wired
US:MN      shield_us_mn_2,shield_us_mn_3                             white  rect              4.0 4.0 7.0 3.0    wired
US:MO      shield_us_mo_2,shield_us_mo_3                             black  rect              4.0 4.0 2.0 5.0    wired
US:MP      shield_us_mp_2,shield_us_mp_3                             black  rect              4.0 4.0 2.0 2.0    wired
US:ND      shield_us_nd_2,shield_us_nd_3                             black  rect              2.0 5.0 4.0 4.0    wired
US:NH      shield_us_nh_2,shield_us_nh_3                             black  rect              4.0 2.0 4.0 5.0    wired
US:NHT     shield_us_nht_cali                                        black  rect              -  wired
US:NM      shield40_us_nm_2,shield40_us_nm_3                         black  ellipse           5.0 5.0 5.0 5.0    wired
US:NV      shield_us_nv                                              black  triangleDown      - - 2.0 6.0  wired
US:Navajo  shield_us_navajo_2,shield_us_navajo_3,shield_us_navajo_4  black  ellipse           2.0 2.0 7.0 7.0    wired
US:OH      shield_us_oh_2,shield_us_oh_3                             black  rect              3.0 3.0 4.0 6.0    wired
US:OK      shield_us_ok_2,shield_us_ok_3                             black  rect              3.0 3.0 7.0 3.0    wired
US:ORSB    shield_us_orsb                                            black  rect              -  wired
US:PA      shield_us_pa_2,shield_us_pa_3                             black  rect              3.0 3.0 5.0 5.0    wired
US:PANYNJ  shield_us_panynj_tunnel                                   black  rect              -  wired
US:PIPC    shield_us_pipc                                            black  rect              -  wired
US:SC      shield_us_sc                                              blue   rect              2.0 2.0 6.0 3.0    wired
US:SD      shield_us_sd_2,shield_us_sd_3                             black  rect              2.0 3.0 3.0 5.0    wired
US:UT      shield_us_ut_2,shield_us_ut_3                             black  rect              4.0 4.0 6.0 5.0    wired
US:VT      shield_us_vt_2,shield_us_vt_3                             green  rect              3.0 3.0 5.0 2.0    wired
US:WI      shield_us_wi_2,shield_us_wi_3                             black  rect              3.0 3.0 3.0 6.0    wired
