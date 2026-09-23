//! The Headway default style, built with the builders in this crate.

use crate::filter::Geometry;
use crate::layer::{Cap, Join, Line};
use crate::*;
use serde_json::json;

const MAJOR_CASING: &str = "#e9ac77";
const MINOR_CASING: &str = "#cfcdca";
const RAIL: &str = "#bbb";
const WATER: &str = "rgb(141,220,248)";
const WHITE: &str = "#fff";
const HALO: &str = "rgba(255,255,255,0.8)";
const REGULAR: &str = "Roboto Regular";
const ITALIC: &str = "Roboto Condensed Italic";
const MEDIUM: &str = "Roboto Medium";

/// The road network is drawn three times over: buried, at grade, and raised.
#[derive(Clone, Copy, Debug, PartialEq)]
enum RoadLevel {
    Tunnel,
    Surface,
    Bridge,
}

use RoadLevel::{Bridge, Surface, Tunnel};

impl RoadLevel {
    /// Almost everything about a road varies by level, and only by level.
    fn pick<T>(self, tunnel: T, surface: T, bridge: T) -> T {
        match self {
            Tunnel => tunnel,
            Surface => surface,
            Bridge => bridge,
        }
    }

    fn prefix(self) -> &'static str {
        self.pick("tunnel", "road", "bridge")
    }

    /// The `brunnel` tag is only set on the copies that aren't at grade.
    fn filter(self) -> Filter {
        self.pick(
            Filter::eq("brunnel", "tunnel"),
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("brunnel", "bridge"),
        )
    }
}

/// Motorways proper, or the ramps on and off them.
fn motorways(ramp: Filter) -> Filter {
    Filter::eq("class", "motorway").and(ramp)
}

fn path_classes() -> Filter {
    Filter::geometry(Geometry::LineString).and(Filter::is_in("class", ["path", "pedestrian"]))
}

/// One road class at one level: a line, and usually a casing painted under it.
struct Road<'a> {
    roads: &'a Roads,
    name: &'static str,
    classes: Filter,
    color: Property,
    width: Stops,
    casing: Option<(&'static str, Stops)>,
    casing_name: Option<&'static str>,
    casing_classes: Option<Filter>,
    fading_casing: bool,
    dashed_when_buried: bool,
    minzoom: Option<f64>,
    capped: bool,
}

impl Road<'_> {
    /// Paints a casing of its own width underneath.
    fn cased(mut self, color: &'static str, width: Stops) -> Self {
        self.casing = Some((color, width));
        self
    }

    /// For minor roads, whose casing is named and filtered unlike their surface.
    fn casing_named(mut self, name: &'static str, classes: Filter) -> Self {
        self.casing_name = Some(name);
        self.casing_classes = Some(classes);
        self
    }

    /// Filters the casing without renaming it.
    fn casing_where(mut self, classes: Filter) -> Self {
        self.casing_classes = Some(classes);
        self
    }

    /// Fades the casing in over half a zoom level instead of popping it on.
    fn fading_casing(mut self) -> Self {
        self.fading_casing = true;
        self
    }

    /// Dashes the buried casing, so the road above still reads as continuous.
    fn dashed_when_buried(mut self) -> Self {
        self.dashed_when_buried = true;
        self
    }

    /// The zoom the road appears at, which only the copy at grade bothers with.
    fn appears_at(mut self, zoom: f64) -> Self {
        self.minzoom = Some(zoom);
        self
    }

    fn uncapped(mut self) -> Self {
        self.capped = false;
        self
    }

    fn line(&self, name: &str, classes: Filter) -> Layer<Line> {
        let level = self.roads.level;
        let mut line = self
            .roads
            .source
            .line(format!("{}_{name}", level.prefix()))
            .filter(level.filter().and(classes))
            .join(Join::Round);
        // Round caps and a low-zoom cutoff are only wanted on the copy at grade.
        if level == Surface {
            if self.capped {
                line = line.cap(Cap::Round);
            }
            if let Some(zoom) = self.minzoom {
                line = line.minzoom(zoom);
            }
        }
        line
    }

    fn push(self, streets: &mut Streets) {
        if let Some((casing_color, width)) = self.casing.clone() {
            let name = format!("{}_casing", self.casing_name.unwrap_or(self.name));
            let classes = self.casing_classes.clone().unwrap_or(self.classes.clone());
            let mut casing = self.line(&name, classes).color(casing_color).width(width);
            if self.fading_casing {
                casing = casing.opacity(zoom!(12 => 0.0, 12.5 => 1.0));
            }
            if self.dashed_when_buried && self.roads.level == Tunnel {
                casing = casing.dasharray(vec![0.5, 0.25]);
            }
            streets.casing(casing);
        }
        let surface = self
            .line(self.name, self.classes.clone())
            .color(self.color.clone())
            .width(self.width.clone());
        streets.plain(surface);
    }
}

/// Builds the transportation layers for one level out of a shared vocabulary.
struct Roads {
    level: RoadLevel,
    source: SourceLayer,
}

impl Roads {
    /// Every road layer for one level of the network.
    fn at(level: RoadLevel, source: &SourceLayer) -> Streets {
        Roads {
            level,
            source: source.clone(),
        }
        .streets()
    }

    fn road(
        &self,
        name: &'static str,
        classes: Filter,
        color: impl Into<Property>,
        width: Stops,
    ) -> Road<'_> {
        Road {
            roads: self,
            name,
            classes,
            color: color.into(),
            width,
            casing: None,
            casing_name: None,
            casing_classes: None,
            fading_casing: false,
            dashed_when_buried: false,
            minzoom: None,
            capped: true,
        }
    }

    /// Trunks, secondaries and ramps share their fill color.
    fn major_fill(&self) -> &'static str {
        self.level.pick("#fff4c6", "#fea", "#fea")
    }

    /// Paths are dashed at every level, and cased at none but the raised one.
    fn path(&self) -> Layer<Line> {
        let level = self.level;
        let path = self
            .source
            .line(format!("{}_path_pedestrian", level.prefix()))
            .filter(level.filter().and(path_classes()))
            .color("hsl(0, 0%, 100%)")
            .dasharray(level.pick(vec![1.0, 0.75], vec![1.0, 0.7], vec![1.0, 0.3]))
            .width(zoom!(base 1.2; 14 => level.pick(0.5, 1.0, 0.5), 20 => 10.0));
        match level {
            Surface => path.minzoom(14.0).join(Join::Round),
            _ => path,
        }
    }

    fn rails(&self, streets: &mut Streets) {
        for (class, name) in [("rail", "major_rail"), ("transit", "transit_rail")] {
            let rail = |suffix: &str| {
                self.source
                    .line(format!("{}_{name}{suffix}", self.level.prefix()))
                    .filter(self.level.filter().and(Filter::eq("class", class)))
                    .color(RAIL)
            };
            streets.plain(rail("").width(zoom!(base 1.4; 14 => 0.4, 15 => 0.75, 20 => 2.0)));
            // Fat dashes drawn over the thin line read as sleepers.
            streets.plain(
                rail("_hatching")
                    .dasharray(vec![0.2, 8.0])
                    .width(zoom!(base 1.4; 14.5 => 0.0, 15 => 3.0, 20 => 8.0)),
            );
        }
    }

    /// Every road layer for this level: casings first, then surfaces.
    fn streets(&self) -> Streets {
        let level = self.level;
        let mut streets = Streets::new();
        streets.plain(self.path());

        // The widths roads and their casings come in, shared across road classes.
        let ramp = zoom!(base 1.2; 12.5 => 0.0, 13 => 1.5, 14 => 2.5, 20 => 11.5);
        let ramp_casing = zoom!(base 1.2; 12 => 1.0, 13 => 3.0, 14 => 4.0, 20 => 15.0);
        let major = zoom!(base 1.2; 5 => 0.0, 7 => 1.0, 20 => 18.0);
        let major_casing = zoom!(base 1.2; 5 => 0.4, 6 => 0.7, 7 => 1.75, 20 => 22.0);
        let on_ramp = Filter::eq("ramp", 1);

        let links = motorways(on_ramp.clone());
        self.road("motorway_link", links, "#fc8", ramp.clone())
            .cased(MAJOR_CASING, ramp_casing.clone())
            .dashed_when_buried()
            .appears_at(12.0)
            .push(&mut streets);

        let tracks = Filter::is_in("class", ["service", "track"]);
        let track_width = zoom!(base 1.2; 15.5 => 0.0, 16 => 2.0, 20 => 7.5);
        self.road("service_track", tracks, WHITE, track_width)
            .cased(
                MINOR_CASING,
                zoom!(base 1.2; 15 => 1.0, 16 => 4.0, 20 => 11.0),
            )
            .dashed_when_buried()
            .push(&mut streets);

        // Raised ramps come through tagged as their own class.
        let off_ramp = Filter::not_in(
            "class",
            ["pedestrian", "path", "track", "service", "motorway"],
        );
        let ramps = level.pick(
            on_ramp.clone(),
            on_ramp.and(off_ramp),
            Filter::eq("class", "link"),
        );
        self.road("link", ramps, self.major_fill(), ramp)
            .cased(MAJOR_CASING, ramp_casing)
            .appears_at(13.0)
            .push(&mut streets);

        // Minor roads arrive as `street` underground and overhead, but `minor` at grade.
        let lines = Filter::geometry(Geometry::LineString);
        let minor = Filter::is_in("class", ["minor"]);
        let street = Filter::is_in("class", ["street", "street_limited"]);
        let minor_casing = level.pick(
            street.clone(),
            lines.clone().and(minor.clone()).and(Filter::ne("ramp", 1)),
            street,
        );
        let minor_classes = level.pick(minor.clone(), lines.and(minor.clone()), minor);
        let minor_width =
            zoom!(base 1.2; 13.5 => 0.0, 14 => 2.5, 20 => level.pick(11.5, 18.0, 18.0));
        let minor_casing_width =
            zoom!(base 1.2; 12 => 0.5, 13 => 1.0, 14 => 4.0, 20 => level.pick(15.0, 20.0, 25.0));
        let minor_casing_color = level.pick(MINOR_CASING, MINOR_CASING, "hsl(36, 6%, 74%)");
        self.road(
            level.pick("minor", "minor", "street"),
            minor_classes,
            WHITE,
            minor_width,
        )
        .cased(minor_casing_color, minor_casing_width)
        .casing_named(level.pick("street", "minor", "street"), minor_casing)
        .fading_casing()
        .push(&mut streets);

        if level == Bridge {
            // A raised path is the only surface that gets a casing of its own.
            streets.casing(
                self.source
                    .line("bridge_path_pedestrian_casing")
                    .filter(level.filter().and(path_classes()))
                    .color("hsl(35, 6%, 80%)")
                    .dasharray(vec![1.0, 0.0])
                    .width(zoom!(base 1.2; 14 => 1.5, 20 => 18.0)),
            );
        }

        // At grade, a ramp off a secondary is cased as a ramp instead.
        let secondary = Filter::is_in("class", ["secondary", "tertiary"]);
        let secondary_casing = level.pick(
            secondary.clone(),
            secondary.clone().and(Filter::ne("ramp", 1)),
            secondary.clone(),
        );
        let bend = level.pick(7.0, 8.0, 7.0);
        let widest = level.pick(10.0, 13.0, 10.0);
        let secondary_width = zoom!(base 1.2; 6.5 => 0.0, bend => 0.5, 20 => widest);
        self.road(
            "secondary_tertiary",
            secondary,
            self.major_fill(),
            secondary_width,
        )
        .cased(MAJOR_CASING, zoom!(base 1.2; 8 => 1.5, 20 => 17.0))
        .casing_where(secondary_casing)
        .push(&mut streets);

        let trunk = Filter::is_in("class", ["primary", "trunk"]);
        self.road("trunk_primary", trunk, self.major_fill(), major.clone())
            .cased(MAJOR_CASING, major_casing.clone())
            .uncapped()
            .push(&mut streets);

        // Motorways cool from a hot orange as the country view zooms in.
        let motorway_fill: Property = level.pick(
            "#ffdaa6".into(),
            zoom!(base 1.0; 5 => "hsl(26, 87%, 62%)", 6 => "#fc8").into(),
            "#fc8".into(),
        );
        let motorway = motorways(Filter::ne("ramp", 1));
        self.road("motorway", motorway, motorway_fill, major)
            .cased(MAJOR_CASING, major_casing)
            .dashed_when_buried()
            .appears_at(5.0)
            .push(&mut streets);

        self.rails(&mut streets);
        streets
    }
}

/// The Headway default style.
pub fn style() -> Style {
    Style::from_json(include_str!("../../assets/styles/basic-v3.json"))
        .expect("the checked-in Headway v3 style parses")
}

#[allow(dead_code)]
fn style_from_builders() -> Style {
    let mut style = Style::new("Headway Default")
        .id("omt-headway")
        .sprite("/sprite/sprites")
        .glyphs("/font/{fontstack}/{range}")
        .metadata("openmaptiles:version", "3.x");

    let openmaptiles = style.source(
        "openmaptiles",
        Source::vector("/areamap-mlt").encoding("mlt"),
    );
    let landcover_source = style.source("landcover", Source::vector("/landcover").maxzoom(8.0));
    let terrain_source = style.source("terrain", Source::raster("/terrain").maxzoom(6.0));

    let park = openmaptiles.layer("park");
    let landuse = openmaptiles.layer("landuse");
    let landcover = openmaptiles.layer("landcover");
    let waterway = openmaptiles.layer("waterway");
    let aeroway = openmaptiles.layer("aeroway");
    let transportation = openmaptiles.layer("transportation");
    let transportation_name = openmaptiles.layer("transportation_name");
    let boundary = openmaptiles.layer("boundary");
    let poi = openmaptiles.layer("poi");
    let place = openmaptiles.layer("place");

    background("background")
        .color("rgb(241,239,231)")
        .push_to(&mut style);
    terrain_source
        .raster("terrain")
        .maxzoom(8.0)
        .opacity(zoom!(5 => 0.8, 6 => 0.0))
        .push_to(&mut style);
    landcover_source
        .layer("landcover")
        .fill("landcover")
        .maxzoom(8.0)
        .opacity(zoom!(7 => 0.4, 8 => 0.0))
        .color(json!([
            "match",
            ["get", "class"],
            "barren",
            "rgb(255, 231, 220)",
            "crop",
            "rgb(149, 222, 58)",
            "grass",
            "rgb(230, 255, 133)",
            "shrub",
            "rgb(230, 255, 133)",
            "snow",
            "rgb(250, 250, 250)",
            "trees",
            "rgb(130, 235, 60)",
            "urban",
            "rgb(226, 231, 215)",
            "transparent"
        ]))
        .push_to(&mut style);
    park.fill("park")
        .color("hsl(101,62.6%,63.3%)")
        .opacity(0.7)
        .outline_color("rgba(95, 208, 100, 1)")
        .push_to(&mut style);
    park.line("park_outline")
        .dasharray(vec![1.0, 1.5])
        .color("rgb(228, 241, 215)")
        .push_to(&mut style);
    landuse
        .fill("landuse_residential")
        .maxzoom(8.0)
        .filter(Filter::eq("class", "residential"))
        .color(zoom!(base 1.0; 9 => "hsla(0, 3%, 85%, 0.84)", 12 => "hsla(35, 57%, 88%, 0.49)"))
        .push_to(&mut style);

    // Ground cover reads as a wash, so it is drawn without antialiasing.
    let ground = |id: &str, class: &str, css: &str, opacity: f64| {
        landcover
            .fill(id)
            .filter(Filter::eq("class", class))
            .antialias(false)
            .color(css)
            .opacity(opacity)
    };
    ground("landcover_wood", "wood", "hsl(101,62.6%,43.3%)", 0.3).push_to(&mut style);
    ground("landcover_grass", "grass", "hsl(101,62.6%,63.3%)", 0.5).push_to(&mut style);
    ground("landcover_ice", "ice", "rgba(224, 236, 236, 1)", 0.8).push_to(&mut style);

    let landuse_fill = |id: &str, class: &str, css: &str| {
        landuse
            .fill(id)
            .filter(Filter::eq("class", class))
            .color(css)
    };
    landuse_fill("landuse_cemetery", "cemetery", "hsl(75, 37%, 81%)").push_to(&mut style);
    landuse_fill("landuse_hospital", "hospital", "#fde").push_to(&mut style);
    landuse_fill("landuse_school", "school", "rgb(236,238,204)").push_to(&mut style);

    waterway
        .line("waterway_tunnel")
        .filter(Filter::eq("brunnel", "tunnel"))
        .color(WATER)
        .dasharray(vec![3.0, 3.0])
        .gap_width(zoom!(12 => 0.0, 20 => 6.0))
        .opacity(1.0)
        .width(zoom!(base 1.4; 8 => 1.0, 20 => 2.0))
        .push_to(&mut style);
    landcover
        .fill("landcover_sand")
        .filter(Filter::eq("class", "sand"))
        .color("rgba(247, 239, 195, 1)")
        .push_to(&mut style);
    openmaptiles
        .layer("water")
        .fill("water")
        .filter(Filter::ne("brunnel", "tunnel"))
        .color(WATER)
        .push_to(&mut style);

    let above_ground = Filter::ne("brunnel", "tunnel");
    waterway
        .line("waterway_river")
        .filter(Filter::eq("class", "river").and(above_ground.clone()))
        .cap(Cap::Round)
        .color(WATER)
        .width(zoom!(base 1.2; 11 => 0.5, 20 => 6.0))
        .push_to(&mut style);
    waterway
        .line("waterway_other")
        .filter(Filter::ne("class", "river").and(above_ground))
        .cap(Cap::Round)
        .color(WATER)
        .width(zoom!(base 1.3; 13 => 0.5, 20 => 6.0))
        .push_to(&mut style);

    aeroway
        .fill("aeroway_fill")
        .minzoom(11.0)
        .filter(Filter::geometry(Geometry::Polygon))
        .color("rgba(229, 228, 224, 1)")
        .opacity(0.7)
        .push_to(&mut style);
    let paved = |id: &str, class: &str, width: Stops| {
        aeroway
            .line(id)
            .minzoom(11.0)
            .filter(Filter::geometry(Geometry::LineString).and(Filter::eq("class", class)))
            .color("#f0ede9")
            .width(width)
    };
    let runway = zoom!(base 1.2; 11 => 3.0, 20 => 16.0);
    paved("aeroway_runway", "runway", runway).push_to(&mut style);
    let taxiway = zoom!(base 1.2; 11 => 0.5, 20 => 6.0);
    paved("aeroway_taxiway", "taxiway", taxiway).push_to(&mut style);

    let roads = |level| Roads::at(level, &transportation);
    style.extend(roads(Tunnel));
    transportation
        .fill("road_area_pattern")
        .filter(Filter::geometry(Geometry::Polygon))
        .pattern("pedestrian_polygon")
        .push_to(&mut style);
    style.extend(roads(Surface));
    style.extend(roads(Bridge));

    openmaptiles
        .layer("building")
        .fill("building")
        .minzoom(13.0)
        .color("hsl(40, 5.5%, 89.2%)")
        .outline_color(zoom!(base 1.0; 13 => "hsla(40, 6%, 79%, 0.32)", 14 => "hsl(40, 6%, 79%)"))
        .push_to(&mut style);

    boundary
        .line("boundary_3")
        .minzoom(3.0)
        .filter(Filter::is_in("admin_level", [3, 4]))
        .join(Join::Round)
        .color("#747980")
        .dasharray(vec![1.5, 1.5])
        .width(zoom!(base 1.0; 3 => 0.6, 6 => 1.0, 10 => 2.0))
        .push_to(&mut style);
    let country_boundary = |id: &str| {
        boundary
            .line(id)
            .rounded()
            .color("hsl(248, 1%, 41%)")
            .opacity(zoom!(base 1.0; 0 => 0.4, 4 => 1.0))
            .width(zoom!(base 1.0; 3 => 1.0, 5 => 1.2, 12 => 3.0))
    };
    // Disputed borders are only drawn once they can be labelled as such.
    country_boundary("boundary_2_z0-4")
        .maxzoom(5.0)
        .filter(Filter::eq("admin_level", 2).and(Filter::not_has("claimed_by")))
        .push_to(&mut style);
    country_boundary("boundary_2_z5-")
        .minzoom(5.0)
        .filter(Filter::eq("admin_level", 2))
        .push_to(&mut style);

    let one_way = |id: &str, direction: i32| {
        transportation
            .symbol(id)
            .minzoom(15.0)
            .filter(Filter::eq("oneway", direction))
            .icon_image("arrow")
            .placement("line")
    };
    one_way("road_one_way_arrow", 1).push_to(&mut style);
    one_way("road_one_way_arrow_opposite", -1)
        .icon_rotate(180.0)
        .push_to(&mut style);

    transportation
        .line("route_ferry")
        .minzoom(12.0)
        .filter(Filter::geometry(Geometry::LineString).and(Filter::is_in("class", ["ferry"])))
        .rounded()
        .color("#99d")
        .width(2.0)
        .dasharray(vec![0.3, 2.0])
        .push_to(&mut style);

    let water_label = |id: &str, source: &SourceLayer, geometry: Geometry| {
        source
            .symbol(id)
            .filter(Filter::geometry(geometry))
            .text_field("{name}")
            .text_font([REGULAR])
            .text_max_width(5.0)
            .text_size(12.0)
            .text_color("#5d60be")
            .text_halo("rgba(255,255,255,0.7)", 1.0)
    };
    water_label("water_name_line", &waterway, Geometry::LineString)
        .placement("line")
        .push_to(&mut style);
    let water_name = openmaptiles.layer("water_name");
    water_label("water_name_point", &water_name, Geometry::Point).push_to(&mut style);

    let poi_label = |id: &str| {
        poi.symbol(id)
            .icon_image("{class}_11")
            .text_font([ITALIC])
            .text_max_width(9.0)
            .text_size(12.0)
            .text_halo("#ffffff", 1.0)
            .text_halo_blur(0.5)
    };
    let ranked_poi = |id: &str, zoom: f64, rank: Filter| {
        poi_label(id)
            .minzoom(zoom)
            .filter(Filter::geometry(Geometry::Point).and(rank))
            .text_field("{name}")
            .text_anchor("top")
            .text_offset([0.0, 0.6])
            .text_color("#666")
    };
    for (id, zoom, rank) in [
        ("poi_z16", 16.0, Filter::gte("rank", 20)),
        (
            "poi_z15",
            15.0,
            Filter::gte("rank", 7).and(Filter::lt("rank", 20)),
        ),
        (
            "poi_z14",
            14.0,
            Filter::gte("rank", 1).and(Filter::lt("rank", 7)),
        ),
    ] {
        ranked_poi(id, zoom, rank).push_to(&mut style);
    }
    poi_label("poi_transit")
        .filter(Filter::is_in("class", ["bus", "rail", "airport"]))
        .text_field("{name_en}")
        .text_anchor("left")
        .text_offset([0.9, 0.0])
        .text_color("#4898ff")
        .push_to(&mut style);

    transportation_name
        .symbol("road_label")
        .placement("line")
        .text_field("{name}")
        .text_font([REGULAR])
        .text_anchor("center")
        .text_offset([0.0, 0.15])
        .text_size(zoom!(base 1.0; 13 => 12.0, 14 => 13.0))
        .text_color("#765")
        .text_halo("#ffffff", 1.0)
        .text_halo_blur(0.5)
        .push_to(&mut style);
    transportation_name
        .symbol("road_shield")
        .minzoom(7.0)
        .filter(Filter::lte("ref_length", 6))
        .icon_image("default_{ref_length}")
        .icon_size(0.8)
        .icon_rotation_alignment("viewport")
        // Shields ride the line once there is room for more than one.
        .placement(zoom!(base 1.0; 10 => "point", 11 => "line"))
        .spacing(500.0)
        .text_field("{ref}")
        .text_font([REGULAR])
        .text_offset([0.0, 0.1])
        .text_rotation_alignment("viewport")
        .text_size(10.0)
        .push_to(&mut style);

    let place_label = |id: &str, filter: Filter| {
        place
            .symbol(id)
            .filter(filter)
            .text_field("{name_en}")
            .text_halo(HALO, 1.2)
    };
    let neighbourhoods = [
        "hamlet",
        "island",
        "islet",
        "neighbourhood",
        "suburb",
        "quarter",
    ];
    place_label("place_other", Filter::is_in("class", neighbourhoods))
        .text_font([ITALIC])
        .text_letter_spacing(0.1)
        .text_max_width(9.0)
        .text_size(zoom!(base 1.2; 12 => 10.0, 15 => 14.0))
        .text_transform("uppercase")
        .text_color("#633")
        .push_to(&mut style);
    place_label("place_village", Filter::eq("class", "village"))
        .text_font([REGULAR])
        .text_max_width(8.0)
        .text_size(zoom!(base 1.2; 10 => 12.0, 15 => 22.0))
        .text_color("#333")
        .push_to(&mut style);
    // Towns and cities carry a dot until their own footprint is visible.
    let settlement = |id: &str, class: &str, font: &str, size: Stops| {
        place_label(id, Filter::eq("class", class))
            .icon_image(zoom!(base 1.0; 0 => "dot_9", 8 => ""))
            .text_anchor("bottom")
            .text_font([font])
            .text_max_width(8.0)
            .text_offset([0.0, 0.0])
            .text_size(size)
            .text_color("#333")
    };
    let town_size = zoom!(base 1.2; 7 => 12.0, 11 => 16.0);
    settlement("place_town", "town", REGULAR, town_size).push_to(&mut style);
    let city_size = zoom!(base 1.2; 7 => 14.0, 11 => 24.0);
    settlement("place_city", "city", MEDIUM, city_size)
        .minzoom(5.0)
        .icon_allow_overlap(true)
        .icon_optional(false)
        .push_to(&mut style);

    place_label("state", Filter::eq("class", "state"))
        .maxzoom(6.0)
        .text_font([ITALIC])
        .text_size(zoom!(4 => 11.0, 6 => 15.0))
        .text_transform("uppercase")
        .text_color("#633")
        .text_halo("rgba(255,255,255,0.7)", 1.0)
        .push_to(&mut style);
    let country = |id: &str, rank: Filter, size: Stops| {
        place_label(id, rank.and(Filter::eq("class", "country")))
            .text_font([ITALIC])
            .text_max_width(6.25)
            .text_size(size)
            .text_transform("none")
            .text_color("#334")
            .text_halo_blur(1.0)
            .text_halo(HALO, 1.0)
    };
    for (id, rank, size) in [
        (
            "country_3",
            Filter::gte("rank", 3),
            zoom!(3 => 11.0, 7 => 17.0),
        ),
        (
            "country_2",
            Filter::eq("rank", 2),
            zoom!(2 => 11.0, 5 => 17.0),
        ),
        (
            "country_1",
            Filter::eq("rank", 1),
            zoom!(1 => 11.0, 4 => 17.0),
        ),
    ] {
        country(id, rank, size).push_to(&mut style);
    }
    place_label("continent", Filter::eq("class", "continent"))
        .maxzoom(1.0)
        .text_font([ITALIC])
        .text_size(13.0)
        .text_transform("uppercase")
        .text_justify("center")
        .text_color("#633")
        .text_halo("rgba(255,255,255,0.7)", 1.0)
        .push_to(&mut style);

    style.validate();
    style
}
