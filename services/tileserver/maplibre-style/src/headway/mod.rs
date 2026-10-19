//! The Headway default style, built with the builders in this crate.

use crate::*;

fn water() -> Color {
    rgb!(141, 220, 248)
}

fn land() -> Color {
    rgb!(241, 239, 231)
}

fn halo() -> Color {
    rgba!(255, 255, 255, 0.8)
}
pub(super) const REGULAR: &str = "Roboto Regular";
pub(super) const ITALIC: &str = "Roboto Condensed Italic";
pub(super) const MEDIUM: &str = "Roboto Medium";

/// Road features below grade.
fn tunnel() -> Filter {
    Filter::eq("brunnel", "tunnel")
}

/// Road features carried over another feature.
fn bridge() -> Filter {
    Filter::eq("brunnel", "bridge")
}

/// Road features at grade.
fn surface() -> Filter {
    Filter::not_in("brunnel", ["bridge", "tunnel"])
}

/// Footpaths and pedestrian streets.
fn foot_paths() -> Filter {
    Filter::linestring().class_in(["path", "pedestrian"])
}

fn above_ground() -> Filter {
    Filter::ne("brunnel", "tunnel")
}

mod road;
mod road_shield;
use road::{Grade, Road, Roads};
use road_shield::{road_shields, RoadShield, RoadShieldText};

/// The Headway default style.
pub fn style() -> Style {
    let mut style = Style::new("Headway Default")
        .id("omt-headway")
        .sprite("/sprite/sprites")
        .glyphs("/font/{fontstack}/{range}")
        .metadata("openmaptiles:version", "3.x");

    let openmaptiles = OpenMapTiles::new(style.source(
        "openmaptiles",
        Source::vector("/areamap-mlt").encoding("mlt"),
    ));
    let landcover_source = style.source("landcover", Source::vector("/landcover").maxzoom(8.0));
    let terrain_source = style.source("terrain", Source::raster("/terrain").maxzoom(6.0));

    style.background("background").color(land());
    style
        .raster(&terrain_source, "terrain")
        .maxzoom(6.0)
        .opacity(zoom!(5 => 0.8, 6 => 0.0));

    let building_color = rgb!(226, 231, 215);
    let sourced_landcover = landcover_source.layer("landcover");
    style
        .fill(&sourced_landcover, "landcover")
        .fade_out(7.0)
        .color(expr![
            Match,
            [Get, "class"],
            "barren",
            (rgba!(255, 231, 220, 0.4)),
            "crop",
            (rgba!(149, 222, 58, 0.4)),
            "grass",
            (rgba!(230, 255, 133, 0.4)),
            "shrub",
            (rgba!(230, 255, 133, 0.4)),
            "snow",
            (rgba!(250, 250, 250, 0.4)),
            "trees",
            (rgba!(130, 235, 60, 0.4)),
            "urban",
            (building_color.alpha(0.4)),
            "transparent"
        ]);
    style
        .fill(openmaptiles.park(), "park")
        .color(rgb!(140, 220, 103))
        .opacity(0.7)
        .outline_color(rgb!(95, 208, 100));
    style
        .line(openmaptiles.park(), "park_outline")
        .dasharray(vec![1.0, 1.5])
        .color(rgb!(228, 241, 215));

    style
        .fill(openmaptiles.landuse(), "landuse_built_up")
        .filter(Filter::class_in([
            "residential",
            "commercial",
            "industrial",
            "garages",
            "retail",
            "bus_station",
            "school",
            "university",
            "kindergarten",
            "college",
            "library",
            "hospital",
        ]))
        .color(building_color.alpha(0.7))
        .fade_in(7.0)
        .fade_out(14.0);

    let mut landcover_fill = |class: &str, color: Color| {
        style
            .fill(openmaptiles.landcover(), format!("landcover_{class}"))
            .filter(Filter::class(class))
            // Ground cover reads as a wash, so it is drawn without antialiasing.
            .antialias(false)
            .color(color);
    };
    landcover_fill("wood", rgba!(85, 180, 41, 0.3));
    landcover_fill("grass", rgba!(140, 220, 103, 0.5));
    // Sand over grass — QA at Golden Gardens.
    landcover_fill("sand", rgba!(247, 239, 195, 1.0));
    landcover_fill("ice", rgba!(224, 236, 236, 0.8));

    style
        .line(openmaptiles.waterway(), "waterway_tunnel")
        .filter(tunnel())
        .color(water())
        .dasharray(vec![3.0, 3.0])
        .gap_width(zoom!(12 => 0.0, 20 => 6.0))
        .width(zoom!(base: 1.4; 8 => 1.0, 20 => 2.0));
    style
        .fill(openmaptiles.water(), "water")
        .filter(above_ground())
        .color(water());

    style
        .line(openmaptiles.waterway(), "waterway_river")
        .filter(Filter::class("river").and(above_ground()))
        .cap_round()
        .color(water())
        .width(zoom!(base: 1.2; 11 => 0.5, 20 => 6.0));
    style
        .line(openmaptiles.waterway(), "waterway_other")
        .filter(Filter::class_not("river").and(above_ground()))
        .cap_round()
        .color(water())
        .width(zoom!(base: 1.3; 13 => 0.5, 20 => 6.0));
    style
        .fill(openmaptiles.aeroway(), "aeroway_fill")
        .minzoom(11.0)
        .filter(Filter::polygon())
        .color(rgb!(229, 228, 224))
        .opacity(0.7);
    style
        .line(openmaptiles.aeroway(), "aeroway_runway")
        .minzoom(11.0)
        .filter(Filter::linestring().class("runway"))
        .color(hex!(0xf0ede9))
        .width(zoom!(base: 1.2; 11 => 3.0, 20 => 16.0));
    style
        .line(openmaptiles.aeroway(), "aeroway_taxiway")
        .minzoom(11.0)
        .filter(Filter::linestring().class("taxiway"))
        .color(hex!(0xf0ede9))
        .width(zoom!(base: 1.2; 11 => 0.5, 20 => 6.0));

    let roads = Roads {
        source: openmaptiles.transportation(),
        roads: vec![
            Road::new("service_track")
                .color(zoom![13.0 => hsl!(0, 0%, 92%), 20.0 => hsl!(0, 0%, 86%)])
                .width(zoom!(base: 1.2; 13.0 => 0.0, 16.0 => 2.0, 20.0 => 7.5))
                .filter(Filter::class_in(["service", "track"]))
                .minzoom(13.0),
            Road::new("ramp")
                .color(zoom![12.0 => hsl!(0, 0%, 88%), 20.0 => hsl!(0, 0%, 74%)])
                .width(zoom!(base: 1.2; 12.0 => 1.0, 14.0 => 2.5, 20.0 => 20.0))
                .filter(Filter::eq("ramp", 1).and_not_in(
                    "class",
                    ["pedestrian", "path", "track", "service", "motorway"],
                ))
                .minzoom(12.0),
            Road::new("minor")
                .color(zoom![13.0 => hsl!(0, 0%, 86%), 20.0 => hsl!(0, 0%, 74%)])
                .width(zoom!(base: 1.2; 12.0 => 1.0, 14.0 => 2.5, 20.0 => 18.0))
                .filter(Filter::linestring().class("minor"))
                .minzoom(12.0),
            Road::new("secondary_tertiary")
                .color(zoom![12.0 => hsl!(0, 0%, 90%), 20.0 => hsl!(0, 0%, 74%)])
                .width(zoom!(base: 1.2; 12.0 => 1.0, 13.0 => 2.5, 20.0 => 26.0))
                .filter(Filter::class_in(["secondary", "tertiary"]))
                .minzoom(12.0),
            Road::new("primary")
                .color(zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 64%)])
                .width(zoom!(base: 1.2; 8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0))
                .minzoom(8.0),
            Road::new("trunk")
                .color(zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 61%)])
                .width(zoom!(base: 1.2; 6.0 => 1.0, 10.0 => 2.85, 13.0 => 4.5, 20.0 => 32.0))
                .minzoom(6.0),
            Road::new("motorway_ramp")
                .color(zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 52%)])
                .width(zoom!(base: 1.2; 8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0))
                .filter(Filter::class("motorway").and_eq("ramp", 1))
                .minzoom(8.0),
            Road::new("motorway")
                .color(zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 52%)])
                .width(zoom!(base: 1.2; 5.0 => 1.0, 13.0 => 4.5, 20.0 => 36.0))
                .filter(Filter::class("motorway").and_ne("ramp", 1))
                .minzoom(5.0)
                .casing_minzoom(10.0),
            Road::new("rail")
                .color(hex!(0xbbbbbb))
                .width(zoom!(base: 1.4; 14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0))
                .filter(Filter::class_in(["rail", "transit"]))
                .no_casing()
                .hatching(zoom![15.0 => hsl!(0, 0%, 85%), 17.0 => hex!(0xbbbbbb)]),
        ],
    };

    roads.add_layers(&mut style, Grade::Tunnel);
    style
        .line(openmaptiles.transportation(), "tunnel_path_pedestrian")
        .minzoom(13.0)
        .filter(tunnel().and(foot_paths()))
        .color(hsl!(0, 0%, 98%))
        .dasharray(vec![1.0, 0.75])
        .width(zoom!(base: 1.2; 13.0 => 0.75, 20.0 => 10.0));
    // Piers stand out over the water they're built on.
    style
        .fill(openmaptiles.transportation(), "pier")
        .filter(Filter::polygon().class("pier"))
        .color(land());
    // Plazas, footway areas, and platforms.
    style
        .fill(openmaptiles.transportation(), "road_area_pattern")
        .filter(Filter::polygon().class_in(["pedestrian", "path"]))
        .pattern("pedestrian_polygon");

    roads.add_layers(&mut style, Grade::Surface);
    style
        .line(openmaptiles.transportation(), "road_path_pedestrian")
        .minzoom(13.0)
        .filter(surface().and(foot_paths()))
        .join_round()
        .color(hsl!(0, 0%, 96%))
        .dasharray(vec![1.0, 0.7])
        .width(zoom!(base: 1.2; 13.0 => 0.75, 20.0 => 10.0));

    roads.add_layers(&mut style, Grade::Bridge);
    style
        .line(
            openmaptiles.transportation(),
            "bridge_path_pedestrian_casing",
        )
        .minzoom(13.0)
        .filter(bridge().and(foot_paths()))
        .color(zoom![14.0 => hsl!(0, 0%, 88%), 20.0 => hsl!(0, 0%, 64%)])
        .dasharray(vec![1.0, 0.0])
        .width(zoom!(base: 1.2; 14.0 => 1.125, 20.0 => 18.0));
    style
        .line(openmaptiles.transportation(), "bridge_path_pedestrian")
        .minzoom(13.0)
        .filter(bridge().and(foot_paths()))
        .color(hsl!(0, 0%, 96%))
        .dasharray(vec![1.0, 0.3])
        .width(zoom!(base: 1.2; 13.0 => 0.75, 20.0 => 10.0));

    style
        .fill(openmaptiles.building(), "building")
        .color(building_color)
        .fade_in(14.0)
        .outline_color(zoom!(14 => hsla!(40, 6%, 79%, 0.32), 15 => hsl!(40, 6%, 79%)));
    style
        .line(openmaptiles.boundary(), "boundary_3")
        .filter(Filter::is_in("admin_level", [3, 4]))
        .join_round()
        .color(hex!(0x747980))
        .fade_in(3.0)
        .dasharray(vec![1.5, 1.5])
        .width(zoom!(3 => 0.6, 6 => 1.0, 10 => 2.0));
    // Disputed borders are only drawn once they can be labelled as such.
    style
        .line(openmaptiles.boundary(), "boundary_2_z0-4")
        .rounded()
        .color(hsl!(248, 1%, 41%))
        .opacity(zoom!(0 => 0.4, 4 => 1.0))
        .width(zoom!(3 => 1.0, 5 => 1.2))
        .maxzoom(5.0)
        .filter(Filter::eq("admin_level", 2).and_not_has("claimed_by"));
    style
        .line(openmaptiles.boundary(), "boundary_2_z5-")
        .rounded()
        .color(hsl!(248, 1%, 41%))
        .width(zoom!(5 => 1.2, 12 => 3.0))
        .minzoom(5.0)
        .filter(Filter::eq("admin_level", 2));

    style
        .symbol(openmaptiles.transportation(), "road_one_way_arrow")
        .minzoom(15.0)
        .filter(Filter::eq("oneway", 1))
        .icon_image("arrow")
        .placement("line");
    style
        .symbol(openmaptiles.transportation(), "road_one_way_arrow_opposite")
        .minzoom(15.0)
        .filter(Filter::eq("oneway", -1))
        .icon_image("arrow")
        .placement("line")
        .icon_rotate(180.0);
    style
        .line(openmaptiles.transportation(), "route_ferry")
        .filter(Filter::linestring().class("ferry"))
        .rounded()
        .color(hex!(0x9999dd))
        .width(2.0)
        .fade_in(12.0)
        .dasharray(vec![0.3, 2.0]);

    {
        // The shared typography for waterway and water-body labels.
        let mut water_label = style.symbol_factory(|label| {
            label
                .text_field("{name}")
                .text_font(REGULAR)
                .text_max_width(5.0)
                .text_size(12.0)
                .text_color(hex!(0x5d60be))
                .text_halo(rgba!(255, 255, 255, 0.7), 1.0);
        });
        water_label
            .symbol(openmaptiles.waterway(), "water_name_line")
            .filter(Filter::linestring())
            .placement("line");
        water_label
            .symbol(openmaptiles.water_name(), "water_name_lake_line")
            .filter(Filter::linestring())
            .placement("line");
        water_label
            .symbol(openmaptiles.water_name(), "water_name_point")
            .filter(Filter::point());
    }

    {
        let mut poi_label = style.symbol_factory(|label| {
            label
                .icon_image("{class}_11")
                .text_font(ITALIC)
                .text_max_width(9.0)
                .text_size(12.0)
                .text_field("{name}")
                .text_anchor("top")
                .text_offset([0.0, 0.6])
                .text_color(hex!(0x666666))
                .text_halo(hex!(0xffffff), 1.0)
                .text_halo_blur(0.5);
        });
        for (id, zoom, rank) in [
            ("poi_z16", 16.0, Filter::gte("rank", 20)),
            ("poi_z15", 15.0, Filter::gte("rank", 7).and_lt("rank", 20)),
            ("poi_z14", 14.0, Filter::gte("rank", 1).and_lt("rank", 7)),
        ] {
            poi_label
                .symbol(openmaptiles.poi(), id)
                .minzoom(zoom)
                .filter(Filter::point().and(rank));
        }
        poi_label
            .symbol(openmaptiles.poi(), "poi_transit")
            .filter(Filter::is_in("class", ["bus", "rail", "airport"]))
            .text_field("{name_en}")
            .text_anchor("left")
            .text_offset([0.9, 0.0])
            .text_color(hex!(0x4898ff));
    }

    style
        .symbol(openmaptiles.transportation_name(), "road_label")
        .placement("line")
        .text_field("{name}")
        .text_font(REGULAR)
        .text_offset([0.0, 0.15])
        .text_size(zoom![13.0 => 12.0, 14.0 => 13.0])
        .text_color(hex!(0x776655))
        .text_halo(hex!(0xffffff), 1.0)
        .text_halo_blur(0.5);
    let shields = road_shields();
    style
        .symbol(openmaptiles.transportation_name(), "road_shield")
        .minzoom(7.0)
        .filter(Filter::expression(expr![
            All,
            [Has, "network"],
            [Has, "ref"],
            [
                LessEqual,
                [Length, [Coalesce, [Get, "route_1_ref"], [Get, "ref"]]],
                6
            ]
        ]))
        .icon_image(RoadShield::icon_image(&shields))
        .icon_rotation_alignment("viewport")
        .placement(zoom![10.0 => "point", 11.0 => "line"])
        .spacing(500.0)
        .text_field(RoadShield::with_ref(RoadShield::property(
            &shields,
            expr![Var, "ref"],
            |shield| match shield.text {
                RoadShieldText::NoText => Some(Expression::literal("")),
                RoadShieldText::Route1Ref => None,
            },
        )))
        .text_font("Overpass Heavy")
        .text_offset(RoadShield::property(
            &shields,
            expr![Literal, [0, 0.11]],
            |shield| shield.text_offset.map(RoadShield::offset),
        ))
        .text_rotation_alignment("viewport")
        .text_size(RoadShield::with_ref(Expression::op(
            Operator::Min,
            [
                RoadShield::property(&shields, Expression::literal(9), |shield| {
                    (shield.text_size != 9.0).then(|| Expression::literal(shield.text_size))
                }),
                Expression::op(
                    Operator::Divide,
                    [
                        RoadShield::property(&shields, Expression::literal(30), |shield| {
                            shield.text_width.map(Expression::literal)
                        }),
                        expr![Length, [Var, "ref"]],
                    ],
                ),
            ],
        )))
        .icon_size(Expression::op(
            Operator::Match,
            [
                expr![Get, "route_1_network"],
                Expression::array(
                    shields
                        .iter()
                        .map(|shield| Expression::literal(shield.network)),
                ),
                Expression::literal(1),
                Expression::literal(0.667),
            ],
        ))
        .text_color(RoadShield::property(
            &shields,
            Expression::literal(hex!(0x000000)),
            |shield| shield.text_color.map(Expression::literal),
        ))
        .text_halo_color(RoadShield::property(
            &shields,
            Expression::literal("rgba(0,0,0,0)"),
            |shield| shield.halo_color.map(Expression::literal),
        ))
        .text_halo_width(RoadShield::property(
            &shields,
            Expression::literal(0),
            |shield| shield.halo_width.map(Expression::literal),
        ));

    let neighbourhoods = [
        "hamlet",
        "island",
        "islet",
        "neighbourhood",
        "suburb",
        "quarter",
    ];
    let mut place_label = style.symbol_factory(|label| {
        label.text_field("{name_en}").text_halo(halo(), 1.2);
    });
    place_label
        .symbol(openmaptiles.place(), "place_other")
        .filter(Filter::is_in("class", neighbourhoods))
        .text_font(ITALIC)
        .text_letter_spacing(0.1)
        .text_max_width(9.0)
        .text_size(zoom!(base: 1.2; 12 => 10.0, 15 => 14.0))
        .text_transform("uppercase")
        .text_color(hex!(0x663333));
    place_label
        .symbol(openmaptiles.place(), "place_village")
        .filter(Filter::class("village"))
        .text_font(REGULAR)
        .text_max_width(8.0)
        .text_size(zoom!(base: 1.2; 10 => 12.0, 15 => 22.0))
        .text_color(hex!(0x333333));
    // Towns and cities carry a dot until their own footprint is visible.
    let town_size = zoom!(base: 1.2; 7 => 12.0, 11 => 16.0);
    place_label
        .symbol(openmaptiles.place(), "place_town")
        .filter(Filter::class("town"))
        .icon_image(zoom!(0 => "dot_9", 8 => ""))
        .text_anchor("bottom")
        .text_font(REGULAR)
        .text_max_width(8.0)
        .text_size(town_size)
        .text_color(hex!(0x333333));
    let city_size = zoom!(base: 1.2; 7 => 14.0, 11 => 24.0);
    place_label
        .symbol(openmaptiles.place(), "place_city")
        .filter(Filter::class("city"))
        .icon_image(zoom!(0 => "dot_9", 8 => ""))
        .text_anchor("bottom")
        .text_font(MEDIUM)
        .text_max_width(8.0)
        .text_size(city_size)
        .text_color(hex!(0x333333))
        .minzoom(5.0)
        .icon_allow_overlap(true)
        .icon_optional(false);

    place_label
        .symbol(openmaptiles.place(), "state")
        .filter(Filter::class("state"))
        .maxzoom(6.0)
        .text_font(ITALIC)
        .text_size(zoom!(4 => 11.0, 6 => 15.0))
        .text_transform("uppercase")
        .text_color(hex!(0x663333))
        .text_halo(rgba!(255, 255, 255, 0.7), 1.0);
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
        place_label
            .symbol(openmaptiles.place(), id)
            .filter(rank.class("country"))
            .text_font(ITALIC)
            .text_max_width(6.25)
            .text_size(size)
            .text_color(hex!(0x333344))
            .text_halo_blur(1.0)
            .text_halo(halo(), 1.0);
    }
    place_label
        .symbol(openmaptiles.place(), "continent")
        .filter(Filter::class("continent"))
        .maxzoom(1.0)
        .text_font(ITALIC)
        .text_size(13.0)
        .text_transform("uppercase")
        .text_color(hex!(0x663333))
        .text_halo(rgba!(255, 255, 255, 0.7), 1.0);

    style.validate();
    style
}
