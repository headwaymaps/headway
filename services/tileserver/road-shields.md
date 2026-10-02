Road shields
============

Route shields in services/tileserver/assets/styles/basic-v3.json are drawn by the
`road_shield` layer. Exit numbers are a separate layer, `road_exit_shield`.

Where the artwork comes from
----------------------------

Blanks come from [OpenStreetMap Americana](https://github.com/osm-americana/openstreetmap-americana),
which is CC0-1.0 — public domain, no attribution required. Their `icons/`
directory holds ~230 `shield_*.svg` blanks and `src/js/shield_defs.js` records,
per network, which blank to use and how the text sits inside it.

To refresh the upstream copy:

    curl -sfL https://codeload.github.com/osm-americana/openstreetmap-americana/tar.gz/refs/heads/main \
      | tar -xz --strip-components=1 -C <dest> '*/icons/*' '*/src/js/shield_defs.js'

Only blanks for networks we draw are checked in. Anything in assets/sprites/
lands in the sprite sheet every client downloads.

How ours differs from theirs
----------------------------

Americana composes the shield and its number into a single bitmap at runtime,
in the browser, via maplibre-gl-js's `styleimagemissing` event. We use the blank
as a plain sprite and let maplibre draw the ref over it as glyph text, because
the iOS app renders the same style through maplibre-native and a browser-side
renderer would leave it with no shields. What that costs:

- No route concurrencies: where two routes share a road, we draw one shield.
- No banners (ALT, BUS, TRK, SPUR above the shield).
- No fitting text to the shape. Their `textLayout` constraints (`ellipse`,
  `southHalfEllipse`, `triangleDown`) fit text inside the outline; we centre it
  and offset it per network.
- No runtime recolouring. Blanks Americana tints with `colorLighten` (black
  becomes that colour) and `colorDarken` (white becomes that colour) are checked
  in already tinted: PA Turnpike, CL:national and CN:expressway. Tints applied
  only to specific refs (AR, GA, CA:ON:primary and others) aren't, so those refs
  draw as the network's ordinary shield.
- No narrow-character discount. They count `1IJijl .-` as two-thirds of a
  character when choosing a blank width; we count every character as one.

Choosing the network and ref
----------------------------

Every shield property keys on `route_1_network` (`US:I`, `US:US`, `US:WA`,
`US:NV:Clark`, `MX:MX`, ...), as Americana does. The tiles' `network` is
OpenMapTiles' coarse value, `road` for anything more specific than a plain state
route, so it is used only in the filter: a feature without one gets no shield.

The number is `route_1_ref`, falling back to the way's `ref`. The way's is
often prefixed or suffixed ("CC 215", "SR 408 Toll", "EJE 3 OTE") where the
route's is just the number. Each property binds it once as `["var", "ref"]`,
and its length picks the blank width and text size.

A route network with no blank of ours, like `US:NY` or `US:I:Express`, gets the
generic `default_N` box. `icon-size`, the `text-size` cap and `text-offset` each
list the networks that have a blank, so that the generic box keeps its own
scale, text size and offset.

Sizing
------

Every blank is drawn at its own pixel size, `icon-size: 1`, as Americana does.
Most are 20px tall; Americana draws some shapes larger for visual parity (NV's
triangle and NM's Zia are 24-25px) and a few wide ones smaller (CKC, PANYNJ).
The generic `default_N` box is drawn at 0.667.

Americana ships one blank for refs up to two characters and a wider one for
three, suffixed `_2` and `_3` (and `_4` for a few networks). Past that the text
shrinks to fit:

    "text-size": ["min", cap, ["/", W, ["length", ["var", "ref"]]]]

`cap` is the largest the text gets: 9.5 for the interstate and business loops,
9 for every other network with a blank of ours, 8.5 for the generic
`default_N` box. `W` is the blank's capacity: `W / length` is the size at which
a ref of that length fills it. It's 32.5 for the interstate and 30 for
everything else, which shrinks four- and five-character refs on an ordinary
`_3` blank.

Networks with one blank narrower than that get their own `W` (WA, NV, AK,
FL:Toll among 21). It comes from usable width, the rendered width less
horizontal padding, times a factor for how much the shape tapers (a downward
triangle keeps about 62% of its box, an ellipse about 82%). The same estimate
reproduces Washington's hand-tuned size, which is the main reason to trust it.
Nevada is the extreme: a three-digit ref drops to about 7.

To resize shields, scale `icon-size` and every `cap` and `W` by the same factor.
`text-offset` is in ems, so that keeps each number where it sits.

Offsets
-------

Americana's `padding` is in the blank's own pixels. Text sits at the centre of
whatever the padding leaves, so the offset from centre is `(top - bottom) / 2`
vertically and `(left - right) / 2` horizontally, divided by the text size to
get the ems `text-offset` wants. Symmetric padding gives no offset, which
assumes the artwork is centred in its box; where a shield looks off despite
symmetric padding, that is why.

The derivation is a starting point. On `ellipse` it over-lifts for us, so those
use half; on `triangleDown` the wide half is at the top, where the number
belongs, and Nevada wants more than the full lift. `rect` needs no correction.
A few are set by eye:

  network  derived   in use
  -------  --------  ------
  US:WA    -0.24em   -0.12em
  US:OR    -0.18em   -0.09em
  US:NV    -0.36em   -0.54em
  US:CO     0.45em    0.51em
  US:I      0.05em    0.06em

MapLibre preserves fractional-pixel text offsets, so there is no renderer
cutoff below which an offset is a no-op. Keep a small offset only when it
visibly improves the shield. A network with its own blank but no offset of its
own sits at `[0, 0.11]`, a pixel below centre at text size 9; the generic box
at `[0, 0.22]`.

Colours
-------

The interstate blank departs from Americana's FHWA colours, #003f87 and #bf2033.
Against our background (rgb(241,239,231), roads in greys from 68% to 52%
lightness) the navy is about 8.9:1, the heaviest mark on the map. Ours are
#1966b3 and #c43142: about 5.1:1, with white text on it near 5.8:1. Business
loops share the interstate's text size, offset and white text.

Known gaps
----------

- A road whose first route relation has no ref takes the way's ref instead of
  a later route's. MEX 57 shows "MEX 57" because route 1 is a stray
  `network=MEX` relation and the real `MX:MX` 57 is route 2.
- Refs joined with `;` ("57;57D") print as they are.
- No shields for networks Americana draws without a blank of their own, such as
  the Allegheny County belts.

Not re-breaking what was approved
---------------------------------

shield-qa.json holds the resolved parameters for every network that has been
looked at on the map. A change that moves a network listed there costs someone a
second review, so diff against it before and after any sweeping edit, and
re-snapshot only once the change has been approved.

Finding a shield to check
-------------------------

The dev-only `/dev/shields` page draws every network at every ref length, using
the real `road_shield` layer. Run it under `bin/dev-local-assets` to see this
checkout's style and sprites.

Places where production tiles carry these networks, with real refs:

  US:NV          Las Vegas         592 (Flamingo Rd)
  US:NV:Clark    Las Vegas         215 (Bruce Woodbury Beltway)
  US:NM          Albuquerque       47
  US:FL          Orlando           426 423 436
  US:FL:Toll     Orlando, Miami    408 417 429 826 869 924
  US:PA:Turnpike Pittsburgh        43 576
  US:I:Business  Pittsburgh        376 (Airport Parkway)
  US:CO:E470     Denver            E470
  US:NJ:GSP      New Jersey        Garden State Parkway
  US:NH          Manchester        28A 28 101
  US:DC          Washington        295
  co:national    Bogotá            45A 50 55
  CL:national    Santiago          5 70 79
  PE:national    Lima              20 20A PE-1N
  KR:expressway  Seoul             29 35 37
  TW:freeway     Taipei            1 2 5 1甲
  CN:expressway  Beijing           G1 G2 G3
  IN:NH          Delhi             148A 248A 344M
  PK:motorway    Lahore            M-2 M-3

Status
------

`verified` means someone has looked at it on the map in its current form.
`wired` means the blank is installed and the style references it, but nobody has
looked yet.

`textLayout` is Americana's text-fitting constraint, not the outline of the
artwork. Florida's is `rect` but its blank is a rounded rectangle carrying the
state's silhouette. Don't read a shield's shape off that column.

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
CA:transcanada                            shield_ca_tch_2,shield_ca_tch_3                                       green   rect              verified
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
MX:CDMX:EJE:NTE                           shield_mx_cdmx_eje_nte                                                black   rect              verified
MX:CDMX:EJE:OTE                           shield_mx_cdmx_eje_ote                                                black   rect              verified
MX:CDMX:EJE:PTE                           shield_mx_cdmx_eje_pte                                                black   rect              verified
MX:CDMX:EJE:SUR                           shield_mx_cdmx_eje_sur                                                black   rect              verified
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
US:CO                                     shield_us_co                                                          black   rect              verified
US:CO:E470                                shield_us_co_e470                                                     black   rect              wired
US:CO:NW                                  shield_us_co_nw                                                       blue    rect              wired
US:CO:Scenic                              shield_us_co_scenic                                                   black   rect              wired
US:CT:Parkway                             shield_us_ct_parkway_merritt                                          black   rect              wired
US:DC                                     shield_us_dc                                                          black   rect              verified
US:FL                                     shield_us_fl_2,shield_us_fl_3                                         black   rect              verified
US:FL:Toll                                shield_us_fl_toll                                                     black   rect              wired
US:FL:Turnpike                            shield_us_fl_turnpike                                                 black   rect              wired
US:GA                                     shield_us_ga_2,shield_us_ga_3                                         black   rect              wired
US:GLST                                   shield_us_glst                                                        black   rect              wired
US:GRR                                    shield_us_grr                                                         black   rect              wired
US:GU                                     shield_us_gu_2,shield_us_gu_3                                         white   ellipse           wired
US:I:Business:Loop                        shield_us_interstate_business_2,shield_us_interstate_business_3       white   rect              wired
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
US:NH                                     shield_us_nh_2,shield_us_nh_3                                         black   rect              verified
US:NH:Turnpike                            shield_us_nh_turnpike                                                 black   rect              wired
US:NHT                                    shield_us_nht_cali                                                    black   rect              wired
US:NJ:ACE                                 shield_us_nj_ace_noref                                                black   rect              wired
US:NJ:GSP                                 shield_us_nj_gsp_noref                                                black   rect              wired
US:NJ:NJTP                                shield_us_nj_njtp_noref                                               black   rect              wired
US:NM:Frontage                            shield_us_nm_frontage                                                 black   rect              wired
US:NPS:Blue_Ridge                         shield_us_nps_brp                                                     black   rect              wired
US:NPS:Natchez_Trace                      shield_us_nps_ntp                                                     black   rect              wired
US:NV                                     shield_us_nv                                                          black   triangleDown      verified
US:NV:Clark                               shield_us_nv_clark                                                    blue    ellipse           verified
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
US:SD                                     shield_us_sd_2,shield_us_sd_3                                         black   rect              verified
US:SD:Custer:CSP                          shield_us_sd_csp                                                      black   ellipse           wired
US:TN:primary                             shield_us_tn_primary                                                  black   rect              wired
US:TX:Fort_Bend:FBCTRA                    shield_us_tx_fbctra                                                   white   rect              wired
US:UT                                     shield_us_ut_2,shield_us_ut_3                                         black   rect              wired
US:VT                                     shield_us_vt_2,shield_us_vt_3                                         green   rect              wired
US:WI                                     shield_us_wi_2,shield_us_wi_3                                         black   rect              wired
US:WI:Rustic                              shield_us_wi_rustic                                                   yellow  rect              wired
co:national                               shield_co_national_2,shield_co_national_3                             black   southHalfEllipse  wired
