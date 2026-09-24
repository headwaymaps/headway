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

Sixteen networks have only one blank and so can never widen. Those get their own
step, sized to the blank: usable width is the rendered width less horizontal
padding, times a factor for how much the shape tapers (a downward triangle keeps
about 62% of its box, an ellipse about 82%), divided by the character count.
Nevada is the extreme — an 18px triangle, so a three-digit ref drops to 7. The
same formula independently reproduces the 9 that Washington was hand-tuned to,
which is the main reason to trust it.

Their `padding` is expressed in the 20px space their blanks are drawn at. Text
sits at the centre of whatever the padding leaves, so the offset from centre is
`(top - bottom) / 2` vertically and `(left - right) / 2` horizontally, scaled by
`icon-size / text-size` to convert into the ems `text-offset` wants.

Two things to get right when deriving an offset.

The scale factor is that network's own `icon-size`, not a constant. Nine blanks
aren't 20px tall and carry their own, so using a flat value silently mis-scales
their offsets.

And the derivation is a starting point, not the answer. Their padding is tuned
for a renderer that fits text to the shape via `textLayout`. On `ellipse` it
over-corrects for us — Washington and Oregon both sat visibly high, and halving
the upward offset landed them right. On `triangleDown` it does not: Nevada
needed the full lift, which makes sense, since the wide half of a downward
triangle is at the top and that is where the number belongs. So halving applies
to `ellipse` only. `rect` has needed no correction at all across US:US, CA, AZ
and AK.

Symmetric padding derives to zero offset, which assumes the blank's artwork is
centred in its box. New Mexico's Zia symbol is not, and needed a small nudge
down that nothing in the definition predicts. Where a shield looks off and the
padding is symmetric, that is the reason.

  network  derived   in use
  -------  --------  ------
  US:WA    -0.24em   -0.12em
  US:OR    -0.18em   -0.09em

Not re-breaking what was approved
---------------------------------

shield-qa.json holds the resolved parameters for every network that has been
looked at on the map. A change that moves a network listed there costs someone a
second review, so diff against it before and after any sweeping edit, and
re-snapshot only once the change has been approved.

This has already bitten twice. Sizing text to fit the single-blank shields
changed Washington's four and five character steps, and gave Alaska a step it
did not have, after both had been signed off. Neither was noticed by the change
itself — only by auditing afterwards.

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
looked at yet.

`textLayout` is americana's text-fitting constraint, not the outline of the
artwork. Florida's is `rect` but its blank is a rounded rectangle carrying the
state's silhouette, and plenty of `rect` networks aren't rectangles. Don't read
a shield's shape off that column.

Sub-networks carry their own artwork and matter more than they look: Orlando's
408, 618 and 836 are `US:FL:Toll`, a yellow blank quite unlike plain `US:FL`.
All 133 of americana's sprite-backed networks are wired, not just the 41 plain
`US:XX` ones.

Most blanks are 20px tall, so `icon-size: 1.2` puts them at ~24px rendered.
Nine are not — Americana draws some shapes larger for visual parity — and carry
their own `icon-size` so everything lands at the same height: BIA, CKC, DC, GU,
NHT, NM, NV, Navajo, PANYNJ.

Network                                   Americana blank(s)                                                    text    textLayout        status
----------------------------------------  --------------------------------------------------------------------  ------  ----------------  --------
US:AK                                     shield_us_ak                                                          black   rect              verified
US:AZ                                     shield_us_az_2,shield_us_az_3                                         black   rect              verified
US:CA                                     shield_us_ca_2,shield_us_ca_3                                         white   rect              verified
US:I                                      shield_us_interstate_2,_3                                             white   southHalfEllipse  verified
US:NM                                     shield40_us_nm_2,shield40_us_nm_3                                     black   ellipse           verified
US:OR                                     shield_us_or_2,shield_us_or_3                                         black   ellipse           verified
US:US                                     shield_badge_2,_3                                                     black   rect              verified
US:WA                                     shield_us_wa                                                          black   ellipse           verified
CA:BC                                     shield_ca_bc_2,shield_ca_bc_3                                         blue    rect              wired
CA:NB:tertiary                            shield_ca_nb                                                          black   rect              wired
CA:NS:S                                   shield_ca_ns_s_mkb                                                    black   rect              wired
CA:NT                                     shield_ca_nt                                                          white   rect              wired
CA:ON:Hamilton:Expressway                 shield_ca_on_hamilton_blue                                            black   rect              wired
CA:ON:Toronto:Expressway                  shield_ca_on_toronto                                                  black   ellipse           wired
CA:ON:primary                             shield_ca_on_primary                                                  black   rect              wired
CA:PE                                     shield_ca_pe                                                          black   rect              wired
CA:QC:A                                   shield_ca_qc_a_2,shield_ca_qc_a_3                                     white   rect              wired
CA:QC:R                                   shield_ca_qc_r                                                        white   rect              wired
CA:SK:secondary                           shield_ca_sk_secondary                                                green   rect              wired
CA:transcanada                            shield_ca_tch_2,shield_ca_tch_3                                       green   rect              wired
CA:transcanada:namedRoute                 shield_ca_tch_2                                                       black   rect              wired
CL:national                               shield_badge_2,shield_badge_3                                         white   rect              wired
CN:expressway                             shield_cn_expressway_2,shield_cn_expressway_3,shield_cn_expressway_4  white   rect              wired
EC:secundaria                             shield_ec_secundaria_2,shield_ec_secundaria_3,shield_ec_secundaria_4  white   rect              wired
GLCT                                      shield_glct_lect                                                      black   rect              wired
GLCT:Loop                                 shield_glct_lmct                                                      black   rect              wired
ID:national                               shield_id_national                                                    black   ellipse           wired
IN:NH                                     shield_in_nh_2,shield_in_nh_3,shield_in_nh_4                          black   rect              wired
KR:expressway                             shield_kr_expressway_2,shield_kr_expressway_3                         white   rect              wired
MX:CDMX:EJE:CENTRAL                       shield_mx_cdmx_eje_central                                            black   rect              wired
MX:CDMX:EJE:NTE                           shield_mx_cdmx_eje_nte                                                black   rect              wired
MX:CDMX:EJE:OTE                           shield_mx_cdmx_eje_ote                                                black   rect              wired
MX:CDMX:EJE:PTE                           shield_mx_cdmx_eje_pte                                                black   rect              wired
MX:CDMX:EJE:SUR                           shield_mx_cdmx_eje_sur                                                black   rect              wired
MX:MX                                     shield_mx_mx_2,shield_mx_mx_3,shield_mx_mx_4                          black   ellipse           wired
NL:DR:Hunebed_Highway                     shield_nl_dr_hunebed                                                  white   rect              wired
NZ:Touring:AH                             shield_nz_ah                                                          black   rect              wired
NZ:Touring:CNZWT                          shield_nz_wine                                                        black   rect              wired
NZ:Touring:MWT                            shield_nz_wine                                                        black   rect              wired
NZ:Touring:PCH                            shield_nz_pch                                                         black   rect              wired
NZ:Touring:SLH                            shield_nz_slh                                                         black   rect              wired
NZ:Touring:SSR                            shield_nz_ssr                                                         black   rect              wired
NZ:Touring:TCDH                           shield_nz_tcdh                                                        black   rect              wired
NZ:Touring:TEH                            shield_nz_teh                                                         black   rect              wired
NZ:Touring:VLH                            shield_nz_vlh                                                         black   rect              wired
NZ:WRR                                    shield_nz_wrr                                                         black   rect              wired
PE:national                               shield_pe_2,shield_pe_3                                               black   rect              wired
PK:motorway                               shield_pk_motorway                                                    white   southHalfEllipse  wired
TW:freeway                                shield_tw_freeway                                                     black   ellipse           wired
US:AL                                     shield_us_al_2,shield_us_al_3                                         black   rect              wired
US:AL:Baldwin:Foley_Beach_Express         shield_us_al_foley                                                    black   rect              wired
US:AR                                     shield_us_ar_2,shield_us_ar_3                                         black   rect              wired
US:AS                                     shield_us_as                                                          white   rect              wired
US:AZ:Indian                              shield_us_az_indian                                                   black   ellipse           wired
US:AZ:Scenic                              shield_us_az_scenic                                                   black   rect              wired
US:BIA                                    shield_us_bia                                                         black   ellipse           wired
US:CA:San_Francisco:49_Mile_Scenic_Drive  shield_us_ca_sf_49                                                    black   rect              wired
US:CKC                                    shield_us_ckc                                                         black   rect              wired
US:CO                                     shield_us_co                                                          black   rect              wired
US:CO:E470                                shield_us_co_e470                                                     black   rect              wired
US:CO:NW                                  shield_us_co_nw                                                       blue    rect              wired
US:CO:Scenic                              shield_us_co_scenic                                                   black   rect              wired
US:CT:Parkway                             shield_us_ct_parkway_merritt                                          black   rect              wired
US:DC                                     shield_us_dc                                                          black   rect              wired
US:FL                                     shield_us_fl_2,shield_us_fl_3                                         black   rect  verified
US:FL:Toll                                shield_us_fl_toll                                                     black   rect              wired
US:FL:Turnpike                            shield_us_fl_turnpike                                                 black   rect              wired
US:GA                                     shield_us_ga_2,shield_us_ga_3                                         black   rect              wired
US:GLST                                   shield_us_glst                                                        black   rect              wired
US:GRR                                    shield_us_grr                                                         black   rect              wired
US:GU                                     shield_us_gu_2,shield_us_gu_3                                         white   ellipse           wired
US:I:Business:Loop                        shield_us_interstate_business_2,shield_us_interstate_business_3       black   rect              wired
US:ID                                     shield_us_id_2,shield_us_id_3                                         white   rect              wired
US:IL:Cook:Chicago:Skyway                 shield_us_il_skyway                                                   black   rect              wired
US:IN:JHMHT                               shield_us_in_jhmht                                                    black   rect              wired
US:IN:Toll                                shield_us_in_toll                                                     black   rect              wired
US:KS                                     shield_us_ks_2,shield_us_ks_3                                         black   ellipse           wired
US:KS:Turnpike                            shield_us_ks_turnpike                                                 black   rect              wired
US:KY:Parkway                             shield_us_ky_parkway                                                  blue    rect              wired
US:LA                                     shield_us_la_2,shield_us_la_3                                         black   rect              wired
US:LA:Causeway                            shield_us_la_causeway                                                 black   rect              wired
US:LHT                                    shield_us_lht                                                         black   rect              wired
US:MA:Turnpike                            shield_us_ma_pike                                                     black   rect              wired
US:ME:Turnpike                            shield_us_me_turnpike                                                 black   rect              wired
US:MN                                     shield_us_mn_2,shield_us_mn_3                                         white   rect              wired
US:MN:Business                            shield_us_mn_business_2,shield_us_mn_business_3                       white   rect              wired
US:MO                                     shield_us_mo_2,shield_us_mo_3                                         black   rect              wired
US:MP                                     shield_us_mp_2,shield_us_mp_3                                         black   rect              wired
US:MT:secondary                           shield_us_mt_secondary                                                black   ellipse           wired
US:ND                                     shield_us_nd_2,shield_us_nd_3                                         black   rect              wired
US:NE:Scenic                              shield_us_ne_byway_noref                                              black   rect              wired
US:NH                                     shield_us_nh_2,shield_us_nh_3                                         black   rect              wired
US:NH:Turnpike                            shield_us_nh_turnpike                                                 black   rect              wired
US:NHT                                    shield_us_nht_cali                                                    black   rect              wired
US:NJ:ACE                                 shield_us_nj_ace_noref                                                black   rect              wired
US:NJ:GSP                                 shield_us_nj_gsp_noref                                                black   rect              wired
US:NJ:NJTP                                shield_us_nj_njtp_noref                                               black   rect              wired
US:NM:Frontage                            shield_us_nm_frontage                                                 black   rect              wired
US:NPS:Blue_Ridge                         shield_us_nps_brp                                                     black   rect              wired
US:NPS:Natchez_Trace                      shield_us_nps_ntp                                                     black   rect              wired
US:NV                                     shield_us_nv                                                          black   triangleDown      wired
US:NV:Clark                               shield_us_nv_clark                                                    blue    ellipse           wired
US:NY:Parkway:LI                          shield_us_ny_parkway_li                                               black   rect              wired
US:NY:Parkway:LOSP                        shield_us_ny_parkway_losp                                             black   rect              wired
US:NY:Parkway:NYC                         shield_us_ny_parkway_nyc                                              black   rect              wired
US:NY:STE                                 shield_us_ny_ste                                                      black   rect              wired
US:NY:Scenic                              shield_us_ny_scenic_adirondack                                        black   rect              wired
US:NY:TBTA                                shield_us_ny_tbta_hughlcarey                                          black   rect              wired
US:NY:Thruway                             shield_us_ny_thruway                                                  black   rect              wired
US:Navajo                                 shield_us_navajo_2,shield_us_navajo_3,shield_us_navajo_4              black   ellipse           wired
US:OH                                     shield_us_oh_2,shield_us_oh_3                                         black   rect              wired
US:OH:ASD                                 shield_us_oh_asd                                                      green   triangleDown      wired
US:OH:HOL                                 shield_us_oh_hol                                                      white   rect              wired
US:OH:JHMHT                               shield_us_oh_jhmht                                                    black   rect              wired
US:OH:OEC                                 shield_us_oh_oec                                                      black   rect              wired
US:OH:SCI                                 shield_us_oh_sci_2,shield_us_oh_sci_3                                 black   rect              wired
US:OH:TUS:Salem                           shield_us_oh_tus_salem                                                black   rect              wired
US:OH:Turnpike                            shield_us_oh_turnpike                                                 black   rect              wired
US:OK                                     shield_us_ok_2,shield_us_ok_3                                         black   rect              wired
US:OK:Turnpike                            shield_us_ok_turnpike                                                 black   rect              wired
US:ORSB                                   shield_us_orsb                                                        black   rect              wired
US:PA                                     shield_us_pa_2,shield_us_pa_3                                         black   rect              wired
US:PA:Turnpike                            shield_us_pa_2,shield_us_pa_3                                         white   rect              wired
US:PANYNJ                                 shield_us_panynj_tunnel                                               black   rect              wired
US:PIPC                                   shield_us_pipc                                                        black   rect              wired
US:SC                                     shield_us_sc                                                          blue    rect              wired
US:SD                                     shield_us_sd_2,shield_us_sd_3                                         black   rect              wired
US:SD:Custer:CSP                          shield_us_sd_csp                                                      black   ellipse           wired
US:TN:primary                             shield_us_tn_primary                                                  black   rect              wired
US:TX:Fort_Bend:FBCTRA                    shield_us_tx_fbctra                                                   white   rect              wired
US:UT                                     shield_us_ut_2,shield_us_ut_3                                         black   rect              wired
US:VT                                     shield_us_vt_2,shield_us_vt_3                                         green   rect              wired
US:WI                                     shield_us_wi_2,shield_us_wi_3                                         black   rect              wired
US:WI:Rustic                              shield_us_wi_rustic                                                   yellow  rect              wired
co:national                               shield_co_national_2,shield_co_national_3                             black   southHalfEllipse  wired
