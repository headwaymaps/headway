//! The Headway default style, built with the builders in this crate.

use crate::filter::Geometry;
use crate::*;
use serde_json::json;

const WATER: &str = "rgb(141,220,248)";
pub(super) const HALO: &str = "rgba(255,255,255,0.8)";
pub(super) const REGULAR: &str = "Roboto Regular";
pub(super) const ITALIC: &str = "Roboto Condensed Italic";
pub(super) const MEDIUM: &str = "Roboto Medium";

mod labels;
mod transport;

use labels::{add_v3_road_labels, poi_label};
use transport::add_v3_roads;

/// The Headway default style.
pub fn style() -> Style {
    style_from_builders()
}

fn style_from_builders() -> Style {
    let mut style = Style::new("Headway Default")
        .id("omt-headway")
        .sprite("/sprite/sprites")
        .glyphs("/font/{fontstack}/{range}")
        .metadata("openmaptiles:version", "3.x");

    let openmaptiles = style.open_map_tiles_source("/areamap-mlt");
    let landcover_source = style.source("landcover", Source::vector("/landcover").maxzoom(8.0));
    let terrain_source = style.source("terrain", Source::raster("/terrain").maxzoom(6.0));

    let landuse = openmaptiles.landuse();
    let landcover = openmaptiles.landcover();
    let waterway = openmaptiles.waterway();
    let transportation = openmaptiles.transportation();
    let transportation_name = openmaptiles.transportation_name();
    let poi = openmaptiles.poi();
    let place = openmaptiles.place();

    style.background("background").color("rgb(241,239,231)");
    style
        .raster(&terrain_source, "terrain")
        .maxzoom(8.0)
        .opacity(zoom!(5 => 0.8, 6 => 0.0));
    let sourced_landcover = landcover_source.layer("landcover");
    style
        .fill(&sourced_landcover, "landcover")
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
        ]));

    style.add_layers(openmaptiles.park(), |s| {
        s.fill("park")
            .color("hsl(101,62.6%,63.3%)")
            .opacity(0.7)
            .outline_color("rgba(95, 208, 100, 1)");
        s.line("park_outline")
            .dasharray(vec![1.0, 1.5])
            .color("rgb(228, 241, 215)");
    });

    // REVIEW: Do I even care about landuse? I feel like maybe i was using it as a proxy for
    // building coverage at zoom levels before buildings were visible but maybe I can use landover.urban instead
    // and get rid of all these landuse rules.
    style
        .fill(&landuse, "landuse_residential")
        .maxzoom(17.0)
        .filter(Filter::eq("class", "residential"))
        .opacity(zoom!(16 => 1.0, 17 => 0.0))
        .color(zoom!(9 => "hsla(0, 3%, 85%, 0.84)", 12 => "hsla(35, 57%, 88%, 0.49)"));

    // Ground cover reads as a wash, so it is drawn without antialiasing.
    let mut landcover_fill = |class: &str, css: &str, opacity: f64| {
        style
            .fill(&landcover, format!("landcover_{class}"))
            .filter(Filter::eq("class", class))
            .antialias(false)
            .color(css)
            .opacity(opacity);
    };
    // TODO: Replace the opacity with an adjustment to hsl.
    landcover_fill("wood", "hsl(101,62.6%,43.3%)", 0.3);
    landcover_fill("grass", "hsl(101,62.6%,63.3%)", 0.5);
    // TODO: Convert ice to hsl
    landcover_fill("ice", "rgba(224, 236, 236, 1)", 0.8);
    style.add_layers(landuse, |s| {
        s.fill("landuse_cemetery")
            .filter(Filter::eq("class", "cemetery"))
            .color("hsl(75, 37%, 81%)");
        s.fill("landuse_hospital")
            .filter(Filter::eq("class", "hospital"))
            .color("#fde");
        s.fill("landuse_school")
            .filter(Filter::eq("class", "school"))
            .color("rgb(236,238,204)");
    });

    style
        .line(&waterway, "waterway_tunnel")
        .filter(Filter::eq("brunnel", "tunnel"))
        .color(WATER)
        .dasharray(vec![3.0, 3.0])
        .gap_width(zoom!(12 => 0.0, 20 => 6.0))
        .width(zoom!(base 1.4; 8 => 1.0, 20 => 2.0));
    style
        .fill(&landcover, "landcover_sand")
        .filter(Filter::eq("class", "sand"))
        .color("rgba(247, 239, 195, 1)");
    style
        .fill(&openmaptiles.water(), "water")
        .filter(Filter::ne("brunnel", "tunnel"))
        .color(WATER);

    let above_ground = Filter::ne("brunnel", "tunnel");
    style
        .line(&waterway, "waterway_river")
        .filter(Filter::eq("class", "river").and(above_ground.clone()))
        .cap(Cap::Round)
        .color(WATER)
        .width(zoom!(base 1.2; 11 => 0.5, 20 => 6.0));
    style
        .line(&waterway, "waterway_other")
        .filter(Filter::ne("class", "river").and(above_ground))
        .cap(Cap::Round)
        .color(WATER)
        .width(zoom!(base 1.3; 13 => 0.5, 20 => 6.0));

    style.add_layers(openmaptiles.aeroway(), |s| {
        s.fill("aeroway_fill")
            .minzoom(11.0)
            .filter(Filter::geometry(Geometry::Polygon))
            .color("rgba(229, 228, 224, 1)")
            .opacity(0.7);
        s.line("aeroway_runway")
            .minzoom(11.0)
            .filter(Filter::geometry(Geometry::LineString).and(Filter::eq("class", "runway")))
            .color("#f0ede9")
            .width(zoom!(base 1.2; 11 => 3.0, 20 => 16.0));
        s.line("aeroway_taxiway")
            .minzoom(11.0)
            .filter(Filter::geometry(Geometry::LineString).and(Filter::eq("class", "taxiway")))
            .color("#f0ede9")
            .width(zoom!(base 1.2; 11 => 0.5, 20 => 6.0));
    });

    add_v3_roads(&mut style, &transportation);

    style
        .fill(&openmaptiles.building(), "building")
        .minzoom(14.0)
        .color("hsl(40, 5.5%, 89.2%)")
        .opacity(zoom!(14 => 0.0, 15 => 1.0))
        .outline_color(zoom!(14 => "hsla(40, 6%, 79%, 0.32)", 15 => "hsl(40, 6%, 79%)"));

    style.add_layers(openmaptiles.boundary(), |s| {
        s.line("boundary_3")
            .minzoom(3.0)
            .filter(Filter::is_in("admin_level", [3, 4]))
            .join(Join::Round)
            .color("#747980")
            .dasharray(vec![1.5, 1.5])
            .width(zoom!(3 => 0.6, 6 => 1.0, 10 => 2.0));
        // Disputed borders are only drawn once they can be labelled as such.
        s.line("boundary_2_z0-4")
            .rounded()
            .color("hsl(248, 1%, 41%)")
            .opacity(zoom!(0 => 0.4, 4 => 1.0))
            .width(zoom!(3 => 1.0, 5 => 1.2, 12 => 3.0))
            .maxzoom(5.0)
            .filter(Filter::eq("admin_level", 2).and(Filter::not_has("claimed_by")));
        s.line("boundary_2_z5-")
            .rounded()
            .color("hsl(248, 1%, 41%)")
            .opacity(zoom!(0 => 0.4, 4 => 1.0))
            .width(zoom!(3 => 1.0, 5 => 1.2, 12 => 3.0))
            .minzoom(5.0)
            .filter(Filter::eq("admin_level", 2));
    });

    style.add_layers(transportation.clone(), |s| {
        s.symbol("road_one_way_arrow")
            .minzoom(15.0)
            .filter(Filter::eq("oneway", 1))
            .icon_image("arrow")
            .placement("line");
        s.symbol("road_one_way_arrow_opposite")
            .minzoom(15.0)
            .filter(Filter::eq("oneway", -1))
            .icon_image("arrow")
            .placement("line")
            .icon_rotate(180.0);
        s.line("route_ferry")
            .minzoom(12.0)
            .filter(Filter::geometry(Geometry::LineString).and(Filter::is_in("class", ["ferry"])))
            .rounded()
            .color("#99d")
            .width(2.0)
            .opacity(zoom!(12 => 0.0, 13 => 1.0))
            .dasharray(vec![0.3, 2.0]);
    });

    style
        .symbol(&waterway, "water_name_line")
        .filter(Filter::geometry(Geometry::LineString))
        .text_field("{name}")
        .text_font([REGULAR])
        .text_max_width(5.0)
        .text_size(12.0)
        .text_color("#5d60be")
        .text_halo("rgba(255,255,255,0.7)", 1.0)
        .placement("line");
    let water_name = openmaptiles.water_name();
    style
        .symbol(&water_name, "water_name_lake_line")
        .filter(Filter::geometry(Geometry::LineString))
        .text_field("{name}")
        .text_font([REGULAR])
        .text_max_width(5.0)
        .text_size(12.0)
        .text_color("#5d60be")
        .text_halo("rgba(255,255,255,0.7)", 1.0)
        .placement("line");
    style
        .symbol(&water_name, "water_name_point")
        .filter(Filter::geometry(Geometry::Point))
        .text_field("{name}")
        .text_font([REGULAR])
        .text_max_width(5.0)
        .text_size(12.0)
        .text_color("#5d60be")
        .text_halo("rgba(255,255,255,0.7)", 1.0);

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
        poi_label(&mut style, &poi, id)
            .minzoom(zoom)
            .filter(Filter::geometry(Geometry::Point).and(rank))
            .text_field("{name}")
            .text_anchor("top")
            .text_offset([0.0, 0.6])
            .text_color("#666");
    }
    poi_label(&mut style, &poi, "poi_transit")
        .filter(Filter::is_in("class", ["bus", "rail", "airport"]))
        .text_field("{name_en}")
        .text_anchor("left")
        .text_offset([0.9, 0.0])
        .text_color("#4898ff");

    add_v3_road_labels(&mut style, &transportation_name);

    let neighbourhoods = [
        "hamlet",
        "island",
        "islet",
        "neighbourhood",
        "suburb",
        "quarter",
    ];
    style
        .symbol(&place, "place_other")
        .filter(Filter::is_in("class", neighbourhoods))
        .text_field("{name_en}")
        .text_halo(HALO, 1.2)
        .text_font([ITALIC])
        .text_letter_spacing(0.1)
        .text_max_width(9.0)
        .text_size(zoom!(base 1.2; 12 => 10.0, 15 => 14.0))
        .text_transform("uppercase")
        .text_color("#633");
    style
        .symbol(&place, "place_village")
        .filter(Filter::eq("class", "village"))
        .text_field("{name_en}")
        .text_halo(HALO, 1.2)
        .text_font([REGULAR])
        .text_max_width(8.0)
        .text_size(zoom!(base 1.2; 10 => 12.0, 15 => 22.0))
        .text_color("#333");
    // Towns and cities carry a dot until their own footprint is visible.
    let town_size = zoom!(base 1.2; 7 => 12.0, 11 => 16.0);
    style
        .symbol(&place, "place_town")
        .filter(Filter::eq("class", "town"))
        .text_field("{name_en}")
        .text_halo(HALO, 1.2)
        .icon_image(zoom!(0 => "dot_9", 8 => ""))
        .text_anchor("bottom")
        .text_font([REGULAR])
        .text_max_width(8.0)
        .text_size(town_size)
        .text_color("#333");
    let city_size = zoom!(base 1.2; 7 => 14.0, 11 => 24.0);
    style
        .symbol(&place, "place_city")
        .filter(Filter::eq("class", "city"))
        .text_field("{name_en}")
        .text_halo(HALO, 1.2)
        .icon_image(zoom!(0 => "dot_9", 8 => ""))
        .text_anchor("bottom")
        .text_font([MEDIUM])
        .text_max_width(8.0)
        .text_size(city_size)
        .text_color("#333")
        .minzoom(5.0)
        .icon_allow_overlap(true)
        .icon_optional(false);

    style
        .symbol(&place, "state")
        .filter(Filter::eq("class", "state"))
        .text_field("{name_en}")
        .text_halo(HALO, 1.2)
        .maxzoom(6.0)
        .text_font([ITALIC])
        .text_size(zoom!(4 => 11.0, 6 => 15.0))
        .text_transform("uppercase")
        .text_color("#633")
        .text_halo("rgba(255,255,255,0.7)", 1.0);
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
        style
            .symbol(&place, id)
            .filter(rank.and(Filter::eq("class", "country")))
            .text_field("{name_en}")
            .text_font([ITALIC])
            .text_max_width(6.25)
            .text_size(size)
            .text_color("#334")
            .text_halo_blur(1.0)
            .text_halo(HALO, 1.0);
    }
    style
        .symbol(&place, "continent")
        .filter(Filter::eq("class", "continent"))
        .text_field("{name_en}")
        .text_halo(HALO, 1.2)
        .maxzoom(1.0)
        .text_font([ITALIC])
        .text_size(13.0)
        .text_transform("uppercase")
        .text_color("#633")
        .text_halo("rgba(255,255,255,0.7)", 1.0);

    style.validate();
    style
}
