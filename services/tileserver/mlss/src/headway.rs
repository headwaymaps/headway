//! The Headway default style, built with the builders in this crate.

use crate::filter::Geometry;
use crate::layer::{Cap, Join, Symbol};
use crate::style::AttachedLayer;
use crate::*;
use serde_json::json;

const WATER: &str = "rgb(141,220,248)";
pub(super) const HALO: &str = "rgba(255,255,255,0.8)";
pub(super) const REGULAR: &str = "Roboto Regular";
pub(super) const ITALIC: &str = "Roboto Condensed Italic";
pub(super) const MEDIUM: &str = "Roboto Medium";

/// The shared icon and typography for point-of-interest labels.
pub(super) fn poi_label<'a>(
    style: &'a mut Style,
    poi: &SourceLayer,
    id: &str,
) -> AttachedLayer<'a, Symbol> {
    style
        .symbol(poi, id)
        .icon_image("{class}_11")
        .text_font([ITALIC])
        .text_max_width(9.0)
        .text_size(12.0)
        .text_halo("#ffffff", 1.0)
        .text_halo_blur(0.5)
}

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

    style
        .line(&transportation, "tunnel_service_track_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::is_in("class", ["service", "track"]),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (15.0, "hsl(0, 0%, 94%)"),
                (18.0, "hsl(0, 0%, 87%)"),
                (20.0, "hsl(0, 0%, 82%)"),
            ]),
        )
        .dasharray(vec![0.5, 0.25])
        .paint(
            "line-width",
            stops([(13.0, 0.76), (16.0, 4.0), (20.0, 11.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_link_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("ramp", 1.0),
            Filter::eq("brunnel", "tunnel"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 93%)"),
                (12.0, "hsl(0, 0%, 93%)"),
                (15.0, "hsl(0, 0%, 89%)"),
                (20.0, "hsl(0, 0%, 82%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 2.12), (14.0, 4.0), (20.0, 23.5)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_street_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::is_in("class", ["street", "street_limited"]),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (13.0, "hsl(0, 0%, 93%)"),
                (16.0, "hsl(0, 0%, 87.5%)"),
                (20.0, "hsl(0, 0%, 82%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 2.12), (14.0, 4.0), (20.0, 20.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::is_in("class", ["secondary", "tertiary"]),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 91%)"),
                (11.0, "hsl(0, 0%, 91%)"),
                (12.0, "hsl(0, 0%, 91%)"),
                (13.0, "hsl(0, 0%, 89%)"),
                (20.0, "hsl(0, 0%, 82%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.76), (13.0, 3.6), (20.0, 30.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_primary_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::eq("class", "primary"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(13.0, "hsl(0, 0%, 83.5%)"), (20.0, "hsl(0, 0%, 76%)")]),
        )
        .paint("line-width", stops([(13.0, 5.9), (20.0, 36.0)]).base(1.2));
    style
        .line(&transportation, "tunnel_trunk_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::eq("class", "trunk"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(13.0, "hsl(0, 0%, 81%)"), (20.0, "hsl(0, 0%, 74.5%)")]),
        )
        .paint("line-width", stops([(13.0, 5.9), (20.0, 36.0)]).base(1.2));
    style
        .line(&transportation, "tunnel_motorway_link_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("class", "motorway"),
            Filter::eq("ramp", 1.0),
            Filter::eq("brunnel", "tunnel"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (12.0, "hsl(0, 0%, 82%)"),
                (15.0, "hsl(0, 0%, 77%)"),
                (20.0, "hsl(0, 0%, 70%)"),
            ]),
        )
        .dasharray(vec![0.5, 0.25])
        .paint(
            "line-width",
            stops([(8.0, 1.74), (13.0, 5.9), (20.0, 35.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_motorway_casing")
        .minzoom(10.0)
        .filter(Filter::all([
            Filter::eq("class", "motorway"),
            Filter::ne("ramp", 1.0),
            Filter::eq("brunnel", "tunnel"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (7.0, "hsl(0, 0%, 80%)"),
                (13.0, "hsl(0, 0%, 75%)"),
                (20.0, "hsl(0, 0%, 70%)"),
            ]),
        )
        .dasharray(vec![0.5, 0.25])
        .paint(
            "line-width",
            stops([(5.0, 1.4), (7.0, 1.75), (13.0, 5.9), (20.0, 40.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_service_track")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::is_in("class", ["service", "track"]),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 97%)"),
                (13.0, "hsl(0, 0%, 92%)"),
                (20.0, "hsl(0, 0%, 87%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(13.0, 0.0), (16.0, 2.0), (20.0, 7.5)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_link")
        .minzoom(12.0)
        .filter(Filter::all([
            Filter::eq("ramp", 1.0),
            Filter::eq("brunnel", "tunnel"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 94%)"),
                (12.0, "hsl(0, 0%, 94%)"),
                (20.0, "hsl(0, 0%, 87%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.0), (14.0, 2.5), (20.0, 20.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_minor")
        .minzoom(12.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::is_in("class", ["minor"]),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 93%)"),
                (13.0, "hsl(0, 0%, 93%)"),
                (20.0, "hsl(0, 0%, 87%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.0), (14.0, 2.5), (20.0, 18.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_path_pedestrian")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::geometry(Geometry::LineString),
            Filter::eq("brunnel", "tunnel"),
            Filter::is_in("class", ["path", "pedestrian"]),
        ]))
        .color("hsl(0, 0%, 98%)")
        .dasharray(vec![1.0, 0.75])
        .paint("line-width", stops([(13.0, 0.75), (20.0, 10.0)]).base(1.2));
    style
        .line(&transportation, "tunnel_secondary_tertiary")
        .minzoom(12.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::is_in("class", ["secondary", "tertiary"]),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 95%)"),
                (11.0, "hsl(0, 0%, 95%)"),
                (12.0, "hsl(0, 0%, 95%)"),
                (20.0, "hsl(0, 0%, 87%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.0), (13.0, 2.5), (20.0, 26.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_primary")
        .minzoom(8.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::eq("class", "primary"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 64%)")]),
        )
        .paint(
            "line-width",
            stops([(8.0, 1.0), (13.0, 4.5), (20.0, 32.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_trunk")
        .minzoom(6.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::eq("class", "trunk"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 61%)")]),
        )
        .paint(
            "line-width",
            stops([(6.0, 1.0), (10.0, 2.85), (13.0, 4.5), (20.0, 32.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_motorway_link")
        .minzoom(8.0)
        .filter(Filter::all([
            Filter::eq("class", "motorway"),
            Filter::eq("ramp", 1.0),
            Filter::eq("brunnel", "tunnel"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 84%)"), (20.0, "hsl(0, 0%, 76%)")]),
        )
        .paint(
            "line-width",
            stops([(8.0, 1.0), (13.0, 4.5), (20.0, 32.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_motorway")
        .minzoom(5.0)
        .filter(Filter::all([
            Filter::eq("class", "motorway"),
            Filter::ne("ramp", 1.0),
            Filter::eq("brunnel", "tunnel"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 84%)"), (20.0, "hsl(0, 0%, 76%)")]),
        )
        .paint(
            "line-width",
            stops([(5.0, 1.0), (7.0, 1.0), (13.0, 4.5), (20.0, 36.0)]).base(1.2),
        );
    style
        .line(&transportation, "tunnel_major_rail")
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::is_in("class", ["rail"]),
        ]))
        .cap(Cap::Round)
        .color("hsl(0, 0%, 86.6667%)")
        .paint(
            "line-width",
            stops([(14.0, 0.4), (15.0, 0.75), (20.0, 2.0)]).base(1.4),
        );
    style
        .line(&transportation, "tunnel_major_rail_hatching")
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::eq("class", "rail"),
        ]))
        .color("hsl(0, 0%, 86.6667%)")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops([(14.5, 0.0), (15.0, 3.0), (20.0, 8.0)]).base(1.4),
        );
    style
        .line(&transportation, "tunnel_transit_rail")
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::is_in("class", ["transit"]),
        ]))
        .cap(Cap::Round)
        .color("hsl(0, 0%, 86.6667%)")
        .paint(
            "line-width",
            stops([(14.0, 0.4), (15.0, 0.75), (20.0, 2.0)]).base(1.4),
        );
    style
        .line(&transportation, "tunnel_transit_rail_hatching")
        .filter(Filter::all([
            Filter::eq("brunnel", "tunnel"),
            Filter::eq("class", "transit"),
        ]))
        .color("hsl(0, 0%, 86.6667%)")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops([(14.5, 0.0), (15.0, 3.0), (20.0, 8.0)]).base(1.4),
        );
    style
        .fill(&transportation, "road_area_pattern")
        .filter(Filter::geometry(Geometry::Polygon))
        .paint("fill-pattern", json!("pedestrian_polygon"));
    style
        .line(&transportation, "road_service_track_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::is_in("class", ["service", "track"]),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (15.0, "hsl(0, 0%, 88%)"),
                (18.0, "hsl(0, 0%, 74%)"),
                (20.0, "hsl(0, 0%, 64%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(13.0, 0.76), (16.0, 4.0), (20.0, 11.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_link_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::not_in(
                "class",
                ["pedestrian", "path", "track", "service", "motorway"],
            ),
            Filter::eq("ramp", 1.0),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 86%)"),
                (12.0, "hsl(0, 0%, 86%)"),
                (15.0, "hsl(0, 0%, 78%)"),
                (20.0, "hsl(0, 0%, 64%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 2.12), (14.0, 4.0), (20.0, 23.5)]).base(1.2),
        );
    style
        .line(&transportation, "road_minor_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::geometry(Geometry::LineString),
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::is_in("class", ["minor"]),
            Filter::ne("ramp", 1.0),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (13.0, "hsl(0, 0%, 86%)"),
                (16.0, "hsl(0, 0%, 75%)"),
                (20.0, "hsl(0, 0%, 64%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 2.12), (14.0, 4.0), (20.0, 20.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::is_in("class", ["secondary", "tertiary"]),
            Filter::ne("ramp", 1.0),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 82%)"),
                (11.0, "hsl(0, 0%, 82%)"),
                (12.0, "hsl(0, 0%, 82%)"),
                (13.0, "hsl(0, 0%, 78%)"),
                (20.0, "hsl(0, 0%, 64%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.76), (13.0, 3.6), (20.0, 30.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_primary_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "primary"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(13.0, "hsl(0, 0%, 67%)"), (20.0, "hsl(0, 0%, 52%)")]),
        )
        .paint("line-width", stops([(13.0, 5.9), (20.0, 36.0)]).base(1.2));
    style
        .line(&transportation, "road_trunk_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "trunk"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(13.0, "hsl(0, 0%, 62%)"), (20.0, "hsl(0, 0%, 49%)")]),
        )
        .paint("line-width", stops([(13.0, 5.9), (20.0, 36.0)]).base(1.2));
    style
        .line(&transportation, "road_motorway_link_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "motorway"),
            Filter::eq("ramp", 1.0),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (12.0, "hsl(0, 0%, 64%)"),
                (15.0, "hsl(0, 0%, 54%)"),
                (20.0, "hsl(0, 0%, 40%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(8.0, 1.74), (13.0, 5.9), (20.0, 35.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_motorway_casing")
        .minzoom(10.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "motorway"),
            Filter::ne("ramp", 1.0),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (7.0, "hsl(0, 0%, 60%)"),
                (13.0, "hsl(0, 0%, 50%)"),
                (20.0, "hsl(0, 0%, 40%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(5.0, 1.4), (7.0, 1.75), (13.0, 5.9), (20.0, 40.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_service_track")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::is_in("class", ["service", "track"]),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 94%)"),
                (13.0, "hsl(0, 0%, 84%)"),
                (20.0, "hsl(0, 0%, 74%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(13.0, 0.0), (16.0, 2.0), (20.0, 7.5)]).base(1.2),
        );
    style
        .line(&transportation, "road_link")
        .minzoom(12.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("ramp", 1.0),
            Filter::not_in(
                "class",
                ["pedestrian", "path", "track", "service", "motorway"],
            ),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 88%)"),
                (12.0, "hsl(0, 0%, 88%)"),
                (20.0, "hsl(0, 0%, 74%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.0), (14.0, 2.5), (20.0, 20.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_minor")
        .minzoom(12.0)
        .filter(Filter::all([
            Filter::geometry(Geometry::LineString),
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::is_in("class", ["minor"]),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 86%)"),
                (13.0, "hsl(0, 0%, 86%)"),
                (20.0, "hsl(0, 0%, 74%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.0), (14.0, 2.5), (20.0, 18.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_path_pedestrian")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::geometry(Geometry::LineString),
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::is_in("class", ["path", "pedestrian"]),
        ]))
        .join(Join::Round)
        .color("hsl(0, 0%, 96%)")
        .dasharray(vec![1.0, 0.7])
        .paint("line-width", stops([(13.0, 0.75), (20.0, 10.0)]).base(1.2));
    style
        .line(&transportation, "road_secondary_tertiary")
        .minzoom(12.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::is_in("class", ["secondary", "tertiary"]),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 90%)"),
                (11.0, "hsl(0, 0%, 90%)"),
                (12.0, "hsl(0, 0%, 90%)"),
                (20.0, "hsl(0, 0%, 74%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.0), (13.0, 2.5), (20.0, 26.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_primary")
        .minzoom(8.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "primary"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 64%)")]),
        )
        .paint(
            "line-width",
            stops([(8.0, 1.0), (13.0, 4.5), (20.0, 32.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_trunk")
        .minzoom(6.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "trunk"),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 61%)")]),
        )
        .paint(
            "line-width",
            stops([(6.0, 1.0), (10.0, 2.85), (13.0, 4.5), (20.0, 32.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_motorway_link")
        .minzoom(8.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "motorway"),
            Filter::eq("ramp", 1.0),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 52%)")]),
        )
        .paint(
            "line-width",
            stops([(8.0, 1.0), (13.0, 4.5), (20.0, 32.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_motorway")
        .minzoom(5.0)
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "motorway"),
            Filter::ne("ramp", 1.0),
        ]))
        .cap(Cap::Round)
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 52%)")]),
        )
        .paint(
            "line-width",
            stops([(5.0, 1.0), (7.0, 1.0), (13.0, 4.5), (20.0, 36.0)]).base(1.2),
        );
    style
        .line(&transportation, "road_major_rail")
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "rail"),
        ]))
        .cap(Cap::Round)
        .color("#bbb")
        .paint(
            "line-width",
            stops([(14.0, 0.4), (15.0, 0.75), (20.0, 2.0)]).base(1.4),
        );
    style
        .line(&transportation, "road_major_rail_hatching")
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "rail"),
        ]))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops([(14.5, 0.0), (15.0, 3.0), (20.0, 8.0)]).base(1.4),
        );
    style
        .line(&transportation, "road_transit_rail")
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "transit"),
        ]))
        .cap(Cap::Round)
        .color("#bbb")
        .paint(
            "line-width",
            stops([(14.0, 0.4), (15.0, 0.75), (20.0, 2.0)]).base(1.4),
        );
    style
        .line(&transportation, "road_transit_rail_hatching")
        .filter(Filter::all([
            Filter::not_in("brunnel", ["bridge", "tunnel"]),
            Filter::eq("class", "transit"),
        ]))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops([(14.5, 0.0), (15.0, 3.0), (20.0, 8.0)]).base(1.4),
        );
    style
        .line(&transportation, "bridge_service_track_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::is_in("class", ["service", "track"]),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (15.0, "hsl(0, 0%, 88%)"),
                (18.0, "hsl(0, 0%, 74%)"),
                (20.0, "hsl(0, 0%, 64%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(13.0, 0.76), (16.0, 4.0), (20.0, 11.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_link_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("class", "link"),
            Filter::eq("brunnel", "bridge"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 86%)"),
                (12.0, "hsl(0, 0%, 86%)"),
                (15.0, "hsl(0, 0%, 78%)"),
                (20.0, "hsl(0, 0%, 64%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 2.12), (14.0, 4.0), (20.0, 23.5)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_street_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::is_in("class", ["street", "street_limited"]),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (13.0, "hsl(0, 0%, 86%)"),
                (16.0, "hsl(0, 0%, 75%)"),
                (20.0, "hsl(0, 0%, 64%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 2.12), (14.0, 4.0), (20.0, 20.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_path_pedestrian_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::geometry(Geometry::LineString),
            Filter::eq("brunnel", "bridge"),
            Filter::is_in("class", ["path", "pedestrian"]),
        ]))
        .paint(
            "line-color",
            stops([(14.0, "hsl(0, 0%, 88%)"), (20.0, "hsl(0, 0%, 64%)")]),
        )
        .dasharray(vec![1.0, 0.0])
        .paint("line-width", stops([(14.0, 1.125), (20.0, 18.0)]).base(1.2));
    style
        .line(&transportation, "bridge_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::is_in("class", ["secondary", "tertiary"]),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 82%)"),
                (11.0, "hsl(0, 0%, 82%)"),
                (12.0, "hsl(0, 0%, 82%)"),
                (13.0, "hsl(0, 0%, 78%)"),
                (20.0, "hsl(0, 0%, 64%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.76), (13.0, 3.6), (20.0, 30.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_primary_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::eq("class", "primary"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(13.0, "hsl(0, 0%, 67%)"), (20.0, "hsl(0, 0%, 52%)")]),
        )
        .paint("line-width", stops([(13.0, 5.9), (20.0, 36.0)]).base(1.2));
    style
        .line(&transportation, "bridge_trunk_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::eq("class", "trunk"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(13.0, "hsl(0, 0%, 62%)"), (20.0, "hsl(0, 0%, 49%)")]),
        )
        .paint("line-width", stops([(13.0, 5.9), (20.0, 36.0)]).base(1.2));
    style
        .line(&transportation, "bridge_motorway_link_casing")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("class", "motorway"),
            Filter::eq("ramp", 1.0),
            Filter::eq("brunnel", "bridge"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (12.0, "hsl(0, 0%, 64%)"),
                (15.0, "hsl(0, 0%, 54%)"),
                (20.0, "hsl(0, 0%, 40%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(8.0, 1.74), (13.0, 5.9), (20.0, 35.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_motorway_casing")
        .minzoom(10.0)
        .filter(Filter::all([
            Filter::eq("class", "motorway"),
            Filter::ne("ramp", 1.0),
            Filter::eq("brunnel", "bridge"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (7.0, "hsl(0, 0%, 60%)"),
                (13.0, "hsl(0, 0%, 50%)"),
                (20.0, "hsl(0, 0%, 40%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(5.0, 1.4), (7.0, 1.75), (13.0, 5.9), (20.0, 40.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_service_track")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::is_in("class", ["service", "track"]),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 94%)"),
                (13.0, "hsl(0, 0%, 84%)"),
                (20.0, "hsl(0, 0%, 74%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(13.0, 0.0), (16.0, 2.0), (20.0, 7.5)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_link")
        .minzoom(12.0)
        .filter(Filter::all([
            Filter::eq("class", "link"),
            Filter::eq("brunnel", "bridge"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 88%)"),
                (12.0, "hsl(0, 0%, 88%)"),
                (20.0, "hsl(0, 0%, 74%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.0), (14.0, 2.5), (20.0, 20.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_street")
        .minzoom(12.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::is_in("class", ["minor"]),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 86%)"),
                (13.0, "hsl(0, 0%, 86%)"),
                (20.0, "hsl(0, 0%, 74%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.0), (14.0, 2.5), (20.0, 18.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_path_pedestrian")
        .minzoom(13.0)
        .filter(Filter::all([
            Filter::geometry(Geometry::LineString),
            Filter::eq("brunnel", "bridge"),
            Filter::is_in("class", ["path", "pedestrian"]),
        ]))
        .color("hsl(0, 0%, 96%)")
        .dasharray(vec![1.0, 0.3])
        .paint("line-width", stops([(13.0, 0.75), (20.0, 10.0)]).base(1.2));
    style
        .line(&transportation, "bridge_secondary_tertiary")
        .minzoom(12.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::is_in("class", ["secondary", "tertiary"]),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([
                (10.0, "hsl(0, 0%, 90%)"),
                (11.0, "hsl(0, 0%, 90%)"),
                (12.0, "hsl(0, 0%, 90%)"),
                (20.0, "hsl(0, 0%, 74%)"),
            ]),
        )
        .paint(
            "line-width",
            stops([(12.0, 1.0), (13.0, 2.5), (20.0, 26.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_primary")
        .minzoom(8.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::eq("class", "primary"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 64%)")]),
        )
        .paint(
            "line-width",
            stops([(8.0, 1.0), (13.0, 4.5), (20.0, 32.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_trunk")
        .minzoom(6.0)
        .filter(Filter::all([
            Filter::eq("brunnel", "bridge"),
            Filter::eq("class", "trunk"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 61%)")]),
        )
        .paint(
            "line-width",
            stops([(6.0, 1.0), (10.0, 2.85), (13.0, 4.5), (20.0, 32.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_motorway_link")
        .minzoom(8.0)
        .filter(Filter::all([
            Filter::eq("class", "motorway"),
            Filter::eq("ramp", 1.0),
            Filter::eq("brunnel", "bridge"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 52%)")]),
        )
        .paint(
            "line-width",
            stops([(8.0, 1.0), (13.0, 4.5), (20.0, 32.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_motorway")
        .minzoom(5.0)
        .filter(Filter::all([
            Filter::eq("class", "motorway"),
            Filter::ne("ramp", 1.0),
            Filter::eq("brunnel", "bridge"),
        ]))
        .join(Join::Round)
        .paint(
            "line-color",
            stops([(10.0, "hsl(0, 0%, 68%)"), (20.0, "hsl(0, 0%, 52%)")]),
        )
        .paint(
            "line-width",
            stops([(5.0, 1.0), (7.0, 1.0), (13.0, 4.5), (20.0, 36.0)]).base(1.2),
        );
    style
        .line(&transportation, "bridge_major_rail")
        .filter(Filter::all([
            Filter::eq("class", "rail"),
            Filter::eq("brunnel", "bridge"),
        ]))
        .color("#bbb")
        .paint(
            "line-width",
            stops([(14.0, 0.4), (15.0, 0.75), (20.0, 2.0)]).base(1.4),
        );
    style
        .line(&transportation, "bridge_major_rail_hatching")
        .filter(Filter::all([
            Filter::eq("class", "rail"),
            Filter::eq("brunnel", "bridge"),
        ]))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops([(14.5, 0.0), (15.0, 3.0), (20.0, 8.0)]).base(1.4),
        );
    style
        .line(&transportation, "bridge_transit_rail")
        .filter(Filter::all([
            Filter::eq("class", "transit"),
            Filter::eq("brunnel", "bridge"),
        ]))
        .color("#bbb")
        .paint(
            "line-width",
            stops([(14.0, 0.4), (15.0, 0.75), (20.0, 2.0)]).base(1.4),
        );
    style
        .line(&transportation, "bridge_transit_rail_hatching")
        .filter(Filter::all([
            Filter::eq("class", "transit"),
            Filter::eq("brunnel", "bridge"),
        ]))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops([(14.5, 0.0), (15.0, 3.0), (20.0, 8.0)]).base(1.4),
        );

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

    style
        .symbol(&transportation_name, "road_label")
        .layout("symbol-placement", json!("line"))
        .layout("text-field", json!("{name}"))
        .layout("text-font", json!(["Roboto Regular"]))
        .layout("text-offset", json!([0, 0.15]))
        .layout("text-size", stops([(13.0, 12.0), (14.0, 13.0)]))
        .paint("text-color", json!("#765"))
        .paint("text-halo-blur", json!(0.5))
        .paint("text-halo-color", json!("#ffffff"))
        .paint("text-halo-width", json!(1));
    style
        .symbol(&transportation_name, "road_shield")
        .minzoom(7.0)
        .filter(Filter::raw(json!([
            "all",
            ["has", "network"],
            ["has", "ref"],
            [
                "<=",
                [
                    "length",
                    ["coalesce", ["get", "route_1_ref"], ["get", "ref"]]
                ],
                6
            ]
        ])))
        .layout(
            "icon-image",
            json!([
                "let",
                "ref",
                ["coalesce", ["get", "route_1_ref"], ["get", "ref"]],
                [
                    "let",
                    "shield_base",
                    [
                        "match",
                        ["get", "route_1_network"],
                        "US:I",
                        "us-interstate",
                        "US:US",
                        "us-highway",
                        "US:CA",
                        "us-state-ca",
                        "US:OR",
                        "us-state-or",
                        "US:AL",
                        "us-state-al",
                        "US:AR",
                        "us-state-ar",
                        "US:AZ",
                        "us-state-az",
                        "US:FL",
                        "us-state-fl",
                        "US:GA",
                        "us-state-ga",
                        "US:GU",
                        "us-state-gu",
                        "US:ID",
                        "us-state-id",
                        "US:KS",
                        "us-state-ks",
                        "US:LA",
                        "us-state-la",
                        "US:MN",
                        "us-state-mn",
                        "US:MO",
                        "us-state-mo",
                        "US:MP",
                        "us-state-mp",
                        "US:ND",
                        "us-state-nd",
                        "US:NH",
                        "us-state-nh",
                        "US:NM",
                        "us-state-nm",
                        "US:Navajo",
                        "us-state-navajo",
                        "US:OH",
                        "us-state-oh",
                        "US:OK",
                        "us-state-ok",
                        "US:PA",
                        "us-state-pa",
                        "US:SD",
                        "us-state-sd",
                        "US:UT",
                        "us-state-ut",
                        "US:VT",
                        "us-state-vt",
                        "US:WI",
                        "us-state-wi",
                        "CA:BC",
                        "us-ca-bc",
                        "CA:QC:A",
                        "us-ca-qc-a",
                        "CA:transcanada",
                        "us-ca-transcanada",
                        "CL:national",
                        "us-cl-national",
                        "CN:expressway",
                        "us-cn-expressway",
                        "EC:secundaria",
                        "us-ec-secundaria",
                        "IN:NH",
                        "us-in-nh",
                        "KR:expressway",
                        "us-kr-expressway",
                        "MX:MX",
                        "us-mx-mx",
                        "PE:national",
                        "us-pe-national",
                        "US:I:Business:Loop",
                        "us-i-business-loop",
                        "US:MN:Business",
                        "us-mn-business",
                        "US:OH:SCI",
                        "us-oh-sci",
                        "US:PA:Turnpike",
                        "us-pa-turnpike",
                        "co:national",
                        "us-co-national",
                        ""
                    ],
                    [
                        "case",
                        ["!=", ["var", "shield_base"], ""],
                        [
                            "concat",
                            ["var", "shield_base"],
                            [
                                "case",
                                ["<=", ["length", ["var", "ref"]], 2],
                                "_2",
                                [
                                    "all",
                                    [">=", ["length", ["var", "ref"]], 4],
                                    [
                                        "match",
                                        ["get", "route_1_network"],
                                        "US:Navajo",
                                        true,
                                        "CN:expressway",
                                        true,
                                        "EC:secundaria",
                                        true,
                                        "IN:NH",
                                        true,
                                        "MX:MX",
                                        true,
                                        false
                                    ]
                                ],
                                "_4",
                                "_3"
                            ]
                        ],
                        [
                            "match",
                            ["get", "route_1_network"],
                            "US:WA",
                            "us-state-wa",
                            "US:AK",
                            "us-state-ak",
                            "US:AS",
                            "us-state-as",
                            "US:BIA",
                            "us-state-bia",
                            "US:CKC",
                            "us-state-ckc",
                            "US:CO",
                            "us-state-co",
                            "US:DC",
                            "us-state-dc",
                            "US:GLST",
                            "us-state-glst",
                            "US:GRR",
                            "us-state-grr",
                            "US:LHT",
                            "us-state-lht",
                            "US:NHT",
                            "us-state-nht",
                            "US:NV",
                            "us-state-nv",
                            "US:ORSB",
                            "us-state-orsb",
                            "US:PANYNJ",
                            "us-state-panynj",
                            "US:PIPC",
                            "us-state-pipc",
                            "US:SC",
                            "us-state-sc",
                            "CA:NB:tertiary",
                            "us-ca-nb-tertiary",
                            "CA:NS:S",
                            "us-ca-ns-s",
                            "CA:NT",
                            "us-ca-nt",
                            "CA:ON:Hamilton:Expressway",
                            "us-ca-on-hamilton-expressway",
                            "CA:ON:Toronto:Expressway",
                            "us-ca-on-toronto-expressway",
                            "CA:ON:primary",
                            "us-ca-on-primary",
                            "CA:PE",
                            "us-ca-pe",
                            "CA:QC:R",
                            "us-ca-qc-r",
                            "CA:SK:secondary",
                            "us-ca-sk-secondary",
                            "CA:transcanada:namedRoute",
                            "us-ca-transcanada-namedroute_2",
                            "GLCT",
                            "us-glct",
                            "GLCT:Loop",
                            "us-glct-loop",
                            "ID:national",
                            "us-id-national",
                            "MX:CDMX:EJE:CENTRAL",
                            "us-mx-cdmx-eje-central",
                            "MX:CDMX:EJE:NTE",
                            "us-mx-cdmx-eje-nte",
                            "MX:CDMX:EJE:OTE",
                            "us-mx-cdmx-eje-ote",
                            "MX:CDMX:EJE:PTE",
                            "us-mx-cdmx-eje-pte",
                            "MX:CDMX:EJE:SUR",
                            "us-mx-cdmx-eje-sur",
                            "NL:DR:Hunebed_Highway",
                            "us-nl-dr-hunebed-highway",
                            "NZ:Touring:AH",
                            "us-nz-touring-ah",
                            "NZ:Touring:CNZWT",
                            "us-nz-touring-cnzwt",
                            "NZ:Touring:MWT",
                            "us-nz-touring-mwt",
                            "NZ:Touring:PCH",
                            "us-nz-touring-pch",
                            "NZ:Touring:SLH",
                            "us-nz-touring-slh",
                            "NZ:Touring:SSR",
                            "us-nz-touring-ssr",
                            "NZ:Touring:TCDH",
                            "us-nz-touring-tcdh",
                            "NZ:Touring:TEH",
                            "us-nz-touring-teh",
                            "NZ:Touring:VLH",
                            "us-nz-touring-vlh",
                            "NZ:WRR",
                            "us-nz-wrr",
                            "PK:motorway",
                            "us-pk-motorway",
                            "TW:freeway",
                            "us-tw-freeway",
                            "US:AL:Baldwin:Foley_Beach_Express",
                            "us-al-baldwin-foley-beach-express",
                            "US:AZ:Indian",
                            "us-az-indian",
                            "US:AZ:Scenic",
                            "us-az-scenic",
                            "US:CA:San_Francisco:49_Mile_Scenic_Drive",
                            "us-ca-san-francisco-49-mile-scenic-drive",
                            "US:CO:E470",
                            "us-co-e470",
                            "US:CO:NW",
                            "us-co-nw",
                            "US:CO:Scenic",
                            "us-co-scenic",
                            "US:CT:Parkway",
                            "us-ct-parkway",
                            "US:FL:Toll",
                            "us-fl-toll",
                            "US:FL:Turnpike",
                            "us-fl-turnpike",
                            "US:IL:Cook:Chicago:Skyway",
                            "us-il-cook-chicago-skyway",
                            "US:IN:JHMHT",
                            "us-in-jhmht",
                            "US:IN:Toll",
                            "us-in-toll",
                            "US:KS:Turnpike",
                            "us-ks-turnpike",
                            "US:KY:Parkway",
                            "us-ky-parkway",
                            "US:LA:Causeway",
                            "us-la-causeway",
                            "US:MA:Turnpike",
                            "us-ma-turnpike",
                            "US:ME:Turnpike",
                            "us-me-turnpike",
                            "US:MT:secondary",
                            "us-mt-secondary",
                            "US:NE:Scenic",
                            "us-ne-scenic",
                            "US:NH:Turnpike",
                            "us-nh-turnpike",
                            "US:NJ:ACE",
                            "us-nj-ace",
                            "US:NJ:GSP",
                            "us-nj-gsp",
                            "US:NJ:NJTP",
                            "us-nj-njtp",
                            "US:NM:Frontage",
                            "us-nm-frontage",
                            "US:NPS:Blue_Ridge",
                            "us-nps-blue-ridge",
                            "US:NPS:Natchez_Trace",
                            "us-nps-natchez-trace",
                            "US:NV:Clark",
                            "us-nv-clark",
                            "US:NY:Parkway:LI",
                            "us-ny-parkway-li",
                            "US:NY:Parkway:LOSP",
                            "us-ny-parkway-losp",
                            "US:NY:Parkway:NYC",
                            "us-ny-parkway-nyc",
                            "US:NY:STE",
                            "us-ny-ste",
                            "US:NY:Scenic",
                            "us-ny-scenic",
                            "US:NY:TBTA",
                            "us-ny-tbta",
                            "US:NY:Thruway",
                            "us-ny-thruway",
                            "US:OH:ASD",
                            "us-oh-asd",
                            "US:OH:HOL",
                            "us-oh-hol",
                            "US:OH:JHMHT",
                            "us-oh-jhmht",
                            "US:OH:OEC",
                            "us-oh-oec",
                            "US:OH:TUS:Salem",
                            "us-oh-tus-salem",
                            "US:OH:Turnpike",
                            "us-oh-turnpike",
                            "US:OK:Turnpike",
                            "us-ok-turnpike",
                            "US:SD:Custer:CSP",
                            "us-sd-custer-csp",
                            "US:TN:primary",
                            "us-tn-primary",
                            "US:TX:Fort_Bend:FBCTRA",
                            "us-tx-fort-bend-fbctra",
                            "US:WI:Rustic",
                            "us-wi-rustic",
                            [
                                "concat",
                                "default_",
                                ["to-string", ["length", ["var", "ref"]]]
                            ]
                        ]
                    ]
                ]
            ]),
        )
        .layout("icon-rotation-alignment", json!("viewport"))
        .layout("symbol-placement", stops([(10.0, "point"), (11.0, "line")]))
        .layout("symbol-spacing", json!(500))
        .layout(
            "text-field",
            json!([
                "let",
                "ref",
                ["coalesce", ["get", "route_1_ref"], ["get", "ref"]],
                [
                    "match",
                    ["get", "route_1_network"],
                    "CA:transcanada:namedRoute",
                    "",
                    "MX:CDMX:EJE:CENTRAL",
                    "",
                    "US:FL:Turnpike",
                    "",
                    "US:GLST",
                    "",
                    "US:GRR",
                    "",
                    "US:IL:Cook:Chicago:Skyway",
                    "",
                    "US:IN:Toll",
                    "",
                    "US:KS:Turnpike",
                    "",
                    "US:MA:Turnpike",
                    "",
                    "US:NPS:Blue_Ridge",
                    "",
                    "US:NPS:Natchez_Trace",
                    "",
                    "US:NY:Parkway:LOSP",
                    "",
                    "US:NY:STE",
                    "",
                    "US:NY:Thruway",
                    "",
                    ["var", "ref"]
                ]
            ]),
        )
        .layout("text-font", json!(["Overpass Heavy"]))
        .layout(
            "text-offset",
            json!([
                "match",
                ["get", "route_1_network"],
                ["US:I", "US:I:Business:Loop"],
                ["literal", [0.0, 0.06]],
                [
                    "US:OH",
                    "US:MT:secondary",
                    "US:NM:Frontage",
                    "US:OH:SCI",
                    "US:OR",
                    "US:SD",
                    "US:NV:Clark"
                ],
                ["literal", [0, -0.12]],
                "US:WA",
                ["literal", [-0.06, -0.01]],
                "CA:NB:tertiary",
                ["literal", [-0.06, 0.11]],
                ["US:CA", "US:SC", "US:VT", "PE:national"],
                ["literal", [0, 0.18]],
                "US:AK",
                ["literal", [0, -0.45]],
                ["US:AL", "US:MO", "US:WI", "US:SD:Custer:CSP"],
                ["literal", [0, -0.18]],
                ["US:AS", "US:CO", "US:CO:E470", "CA:SK:secondary"],
                ["literal", [0, 0.51]],
                "US:DC",
                ["literal", [0, 0.29]],
                "US:FL",
                ["literal", [-0.15, 0.12]],
                "US:ID",
                ["literal", [0.24, -0.45]],
                "US:ND",
                ["literal", [-0.18, 0]],
                "US:NH",
                ["literal", [0.12, 0]],
                "US:NV",
                ["literal", [0, -0.64]],
                "CA:NT",
                ["literal", [0, 0.41]],
                "MX:CDMX:EJE:NTE",
                ["literal", [0, 0.36]],
                "MX:CDMX:EJE:OTE",
                ["literal", [-0.36, 0]],
                "MX:CDMX:EJE:PTE",
                ["literal", [0.36, 0]],
                [
                    "MX:CDMX:EJE:SUR",
                    "US:NY:Parkway:NYC",
                    "US:TX:Fort_Bend:FBCTRA"
                ],
                ["literal", [0, -0.36]],
                [
                    "US:MN",
                    "US:OK",
                    "CA:ON:primary",
                    "CN:expressway",
                    "KR:expressway",
                    "NL:DR:Hunebed_Highway",
                    "US:MN:Business"
                ],
                ["literal", [0, 0.24]],
                "US:LA",
                ["literal", [0, 0.31]],
                "US:CO:NW",
                ["literal", [0, 0.6]],
                "US:FL:Toll",
                ["literal", [-0.18, 0.18]],
                ["US:KY:Parkway", "co:national"],
                ["literal", [0, -0.24]],
                "US:NY:Parkway:LI",
                ["literal", [0.24, -0.36]],
                "US:OH:TUS:Salem",
                ["literal", [-0.12, -0.18]],
                "US:TN:primary",
                ["literal", [0, -0.3]],
                "US:WI:Rustic",
                ["literal", [-0.15, 0.3]],
                ["US:AZ:Indian", "US:BIA"],
                ["literal", [0, -0.01]],
                ["literal", [0, 0.11]]
            ]),
        )
        .layout("text-rotation-alignment", json!("viewport"))
        .layout(
            "text-size",
            json!([
                "let",
                "ref",
                ["coalesce", ["get", "route_1_ref"], ["get", "ref"]],
                [
                    "min",
                    [
                        "match",
                        ["get", "route_1_network"],
                        ["US:I", "US:I:Business:Loop"],
                        9.5,
                        [
                            "US:US",
                            "US:WA",
                            "US:CA",
                            "US:OR",
                            "US:AK",
                            "US:AL",
                            "US:AR",
                            "US:AS",
                            "US:AZ",
                            "US:BIA",
                            "US:CKC",
                            "US:CO",
                            "US:DC",
                            "US:FL",
                            "US:GA",
                            "US:GLST",
                            "US:GRR",
                            "US:GU",
                            "US:ID",
                            "US:KS",
                            "US:LA",
                            "US:LHT",
                            "US:MN",
                            "US:MO",
                            "US:MP",
                            "US:ND",
                            "US:NH",
                            "US:NHT",
                            "US:NM",
                            "US:NV",
                            "US:Navajo",
                            "US:OH",
                            "US:OK",
                            "US:ORSB",
                            "US:PA",
                            "US:PANYNJ",
                            "US:PIPC",
                            "US:SC",
                            "US:SD",
                            "US:UT",
                            "US:VT",
                            "US:WI",
                            "CA:BC",
                            "CA:NB:tertiary",
                            "CA:NS:S",
                            "CA:NT",
                            "CA:ON:Hamilton:Expressway",
                            "CA:ON:Toronto:Expressway",
                            "CA:ON:primary",
                            "CA:PE",
                            "CA:QC:A",
                            "CA:QC:R",
                            "CA:SK:secondary",
                            "CA:transcanada",
                            "CA:transcanada:namedRoute",
                            "CL:national",
                            "CN:expressway",
                            "EC:secundaria",
                            "GLCT",
                            "GLCT:Loop",
                            "ID:national",
                            "IN:NH",
                            "KR:expressway",
                            "MX:CDMX:EJE:CENTRAL",
                            "MX:CDMX:EJE:NTE",
                            "MX:CDMX:EJE:OTE",
                            "MX:CDMX:EJE:PTE",
                            "MX:CDMX:EJE:SUR",
                            "MX:MX",
                            "NL:DR:Hunebed_Highway",
                            "NZ:Touring:AH",
                            "NZ:Touring:CNZWT",
                            "NZ:Touring:MWT",
                            "NZ:Touring:PCH",
                            "NZ:Touring:SLH",
                            "NZ:Touring:SSR",
                            "NZ:Touring:TCDH",
                            "NZ:Touring:TEH",
                            "NZ:Touring:VLH",
                            "NZ:WRR",
                            "PE:national",
                            "PK:motorway",
                            "TW:freeway",
                            "US:AL:Baldwin:Foley_Beach_Express",
                            "US:AZ:Indian",
                            "US:AZ:Scenic",
                            "US:CA:San_Francisco:49_Mile_Scenic_Drive",
                            "US:CO:E470",
                            "US:CO:NW",
                            "US:CO:Scenic",
                            "US:CT:Parkway",
                            "US:FL:Toll",
                            "US:FL:Turnpike",
                            "US:IL:Cook:Chicago:Skyway",
                            "US:IN:JHMHT",
                            "US:IN:Toll",
                            "US:KS:Turnpike",
                            "US:KY:Parkway",
                            "US:LA:Causeway",
                            "US:MA:Turnpike",
                            "US:ME:Turnpike",
                            "US:MN:Business",
                            "US:MT:secondary",
                            "US:NE:Scenic",
                            "US:NH:Turnpike",
                            "US:NJ:ACE",
                            "US:NJ:GSP",
                            "US:NJ:NJTP",
                            "US:NM:Frontage",
                            "US:NPS:Blue_Ridge",
                            "US:NPS:Natchez_Trace",
                            "US:NV:Clark",
                            "US:NY:Parkway:LI",
                            "US:NY:Parkway:LOSP",
                            "US:NY:Parkway:NYC",
                            "US:NY:STE",
                            "US:NY:Scenic",
                            "US:NY:TBTA",
                            "US:NY:Thruway",
                            "US:OH:ASD",
                            "US:OH:HOL",
                            "US:OH:JHMHT",
                            "US:OH:OEC",
                            "US:OH:SCI",
                            "US:OH:TUS:Salem",
                            "US:OH:Turnpike",
                            "US:OK:Turnpike",
                            "US:PA:Turnpike",
                            "US:SD:Custer:CSP",
                            "US:TN:primary",
                            "US:TX:Fort_Bend:FBCTRA",
                            "US:WI:Rustic",
                            "co:national"
                        ],
                        9,
                        8.5
                    ],
                    [
                        "/",
                        [
                            "match",
                            ["get", "route_1_network"],
                            ["US:GU", "US:AK"],
                            24.5,
                            ["US:BIA", "MX:CDMX:EJE:NTE", "MX:CDMX:EJE:SUR"],
                            22.5,
                            ["US:NV", "US:FL:Toll", "US:NY:Parkway:LI"],
                            21.5,
                            "US:WA",
                            22,
                            ["US:CA", "US:NM:Frontage"],
                            29,
                            ["CA:ON:primary", "CA:PE", "CA:QC:R"],
                            28,
                            "US:DC",
                            24,
                            "US:AZ:Indian",
                            24,
                            ["CA:NB:tertiary", "US:CO:NW"],
                            23,
                            ["CA:SK:secondary", "NZ:Touring:TEH", "US:NV:Clark"],
                            27,
                            "CA:ON:Toronto:Expressway",
                            15,
                            "TW:freeway",
                            18.5,
                            "US:MT:secondary",
                            21,
                            ["CA:NT", "US:OH:ASD", "US:TX:Fort_Bend:FBCTRA"],
                            25,
                            ["US:I", "US:I:Business:Loop"],
                            32,
                            30
                        ],
                        ["length", ["var", "ref"]]
                    ]
                ]
            ]),
        )
        .layout(
            "icon-size",
            json!([
                "match",
                ["get", "route_1_network"],
                [
                    "US:I",
                    "US:US",
                    "US:WA",
                    "US:CA",
                    "US:OR",
                    "US:AK",
                    "US:AL",
                    "US:AR",
                    "US:AS",
                    "US:AZ",
                    "US:BIA",
                    "US:CKC",
                    "US:CO",
                    "US:DC",
                    "US:FL",
                    "US:GA",
                    "US:GLST",
                    "US:GRR",
                    "US:GU",
                    "US:ID",
                    "US:KS",
                    "US:LA",
                    "US:LHT",
                    "US:MN",
                    "US:MO",
                    "US:MP",
                    "US:ND",
                    "US:NH",
                    "US:NHT",
                    "US:NM",
                    "US:NV",
                    "US:Navajo",
                    "US:OH",
                    "US:OK",
                    "US:ORSB",
                    "US:PA",
                    "US:PANYNJ",
                    "US:PIPC",
                    "US:SC",
                    "US:SD",
                    "US:UT",
                    "US:VT",
                    "US:WI",
                    "CA:BC",
                    "CA:NB:tertiary",
                    "CA:NS:S",
                    "CA:NT",
                    "CA:ON:Hamilton:Expressway",
                    "CA:ON:Toronto:Expressway",
                    "CA:ON:primary",
                    "CA:PE",
                    "CA:QC:A",
                    "CA:QC:R",
                    "CA:SK:secondary",
                    "CA:transcanada",
                    "CA:transcanada:namedRoute",
                    "CL:national",
                    "CN:expressway",
                    "EC:secundaria",
                    "GLCT",
                    "GLCT:Loop",
                    "ID:national",
                    "IN:NH",
                    "KR:expressway",
                    "MX:CDMX:EJE:CENTRAL",
                    "MX:CDMX:EJE:NTE",
                    "MX:CDMX:EJE:OTE",
                    "MX:CDMX:EJE:PTE",
                    "MX:CDMX:EJE:SUR",
                    "MX:MX",
                    "NL:DR:Hunebed_Highway",
                    "NZ:Touring:AH",
                    "NZ:Touring:CNZWT",
                    "NZ:Touring:MWT",
                    "NZ:Touring:PCH",
                    "NZ:Touring:SLH",
                    "NZ:Touring:SSR",
                    "NZ:Touring:TCDH",
                    "NZ:Touring:TEH",
                    "NZ:Touring:VLH",
                    "NZ:WRR",
                    "PE:national",
                    "PK:motorway",
                    "TW:freeway",
                    "US:AL:Baldwin:Foley_Beach_Express",
                    "US:AZ:Indian",
                    "US:AZ:Scenic",
                    "US:CA:San_Francisco:49_Mile_Scenic_Drive",
                    "US:CO:E470",
                    "US:CO:NW",
                    "US:CO:Scenic",
                    "US:CT:Parkway",
                    "US:FL:Toll",
                    "US:FL:Turnpike",
                    "US:I:Business:Loop",
                    "US:IL:Cook:Chicago:Skyway",
                    "US:IN:JHMHT",
                    "US:IN:Toll",
                    "US:KS:Turnpike",
                    "US:KY:Parkway",
                    "US:LA:Causeway",
                    "US:MA:Turnpike",
                    "US:ME:Turnpike",
                    "US:MN:Business",
                    "US:MT:secondary",
                    "US:NE:Scenic",
                    "US:NH:Turnpike",
                    "US:NJ:ACE",
                    "US:NJ:GSP",
                    "US:NJ:NJTP",
                    "US:NM:Frontage",
                    "US:NPS:Blue_Ridge",
                    "US:NPS:Natchez_Trace",
                    "US:NV:Clark",
                    "US:NY:Parkway:LI",
                    "US:NY:Parkway:LOSP",
                    "US:NY:Parkway:NYC",
                    "US:NY:STE",
                    "US:NY:Scenic",
                    "US:NY:TBTA",
                    "US:NY:Thruway",
                    "US:OH:ASD",
                    "US:OH:HOL",
                    "US:OH:JHMHT",
                    "US:OH:OEC",
                    "US:OH:SCI",
                    "US:OH:TUS:Salem",
                    "US:OH:Turnpike",
                    "US:OK:Turnpike",
                    "US:PA:Turnpike",
                    "US:SD:Custer:CSP",
                    "US:TN:primary",
                    "US:TX:Fort_Bend:FBCTRA",
                    "US:WI:Rustic",
                    "co:national"
                ],
                1,
                0.667
            ]),
        )
        .paint(
            "text-color",
            json!([
                "match",
                ["get", "route_1_network"],
                ["US:I", "US:I:Business:Loop"],
                "#ffffff",
                "US:CA",
                "#ffffff",
                "US:AS",
                "#ffffff",
                "US:GU",
                "#ffffff",
                "US:ID",
                "#ffffff",
                "US:MN",
                "#ffffff",
                "US:SC",
                "#003f87",
                "US:VT",
                "#006747",
                "CA:BC",
                "#003f87",
                "CA:NT",
                "#ffffff",
                "CA:QC:A",
                "#ffffff",
                "CA:QC:R",
                "#ffffff",
                "CA:SK:secondary",
                "#006747",
                "CA:transcanada",
                "#006747",
                "CL:national",
                "#ffffff",
                "CN:expressway",
                "#ffffff",
                "EC:secundaria",
                "#ffffff",
                "KR:expressway",
                "#ffffff",
                "NL:DR:Hunebed_Highway",
                "#ffffff",
                "PK:motorway",
                "#ffffff",
                "US:CO:NW",
                "#003f87",
                "US:KY:Parkway",
                "#003f87",
                "US:MN:Business",
                "#ffffff",
                "US:NV:Clark",
                "#003f87",
                "US:OH:ASD",
                "#006747",
                "US:OH:HOL",
                "#ffffff",
                "US:PA:Turnpike",
                "#ffffff",
                "US:TX:Fort_Bend:FBCTRA",
                "#ffffff",
                "US:WI:Rustic",
                "#ffcd00",
                "#000000"
            ]),
        )
        .paint(
            "text-halo-color",
            json!([
                "match",
                ["get", "route_1_network"],
                "US:DC",
                "#ffffff",
                "rgba(0,0,0,0)"
            ]),
        )
        .paint(
            "text-halo-width",
            json!(["match", ["get", "route_1_network"], "US:DC", 1, 0]),
        );

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
