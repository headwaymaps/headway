//! The Headway default style, built with the builders in this crate.

use crate::filter::Geometry;
use crate::layer::Symbol;
use crate::style::AttachedLayer;
use crate::*;

const WATER: &str = "rgb(141,220,248)";
pub(super) const HALO: &str = "rgba(255,255,255,0.8)";
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

/// The shared typography for waterway and water-body labels.
fn water_label<'a>(
    style: &'a mut Style,
    water: &SourceLayer,
    id: &str,
) -> AttachedLayer<'a, Symbol> {
    style
        .symbol(water, id)
        .text_field("{name}")
        .text_font([REGULAR])
        .text_max_width(5.0)
        .text_size(12.0)
        .text_color("#5d60be")
        .text_halo("rgba(255,255,255,0.7)", 1.0)
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
        .color(Expression::op(
            Operator::Match,
            [
                Expression::op(Operator::Get, [Expression::literal("class")]),
                Expression::literal("barren"),
                Expression::literal("rgb(255, 231, 220)"),
                Expression::literal("crop"),
                Expression::literal("rgb(149, 222, 58)"),
                Expression::literal("grass"),
                Expression::literal("rgb(230, 255, 133)"),
                Expression::literal("shrub"),
                Expression::literal("rgb(230, 255, 133)"),
                Expression::literal("snow"),
                Expression::literal("rgb(250, 250, 250)"),
                Expression::literal("trees"),
                Expression::literal("rgb(130, 235, 60)"),
                Expression::literal("urban"),
                Expression::literal("rgb(226, 231, 215)"),
                Expression::literal("transparent"),
            ],
        ));

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
        .filter(Filter::class("residential"))
        .opacity(zoom!(16 => 1.0, 17 => 0.0))
        .color(zoom!(9 => "hsla(0, 3%, 85%, 0.84)", 12 => "hsla(35, 57%, 88%, 0.49)"));

    // Ground cover reads as a wash, so it is drawn without antialiasing.
    let mut landcover_fill = |class: &str, css: &str, opacity: f64| {
        style
            .fill(&landcover, format!("landcover_{class}"))
            .filter(Filter::class(class))
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
            .filter(Filter::class("cemetery"))
            .color("hsl(75, 37%, 81%)");
        s.fill("landuse_hospital")
            .filter(Filter::class("hospital"))
            .color("#fde");
        s.fill("landuse_school")
            .filter(Filter::class("school"))
            .color("rgb(236,238,204)");
    });

    style
        .line(&waterway, "waterway_tunnel")
        .filter(tunnel())
        .color(WATER)
        .dasharray(vec![3.0, 3.0])
        .gap_width(zoom!(12 => 0.0, 20 => 6.0))
        .width(zoom!(base 1.4; 8 => 1.0, 20 => 2.0));
    style
        .fill(&landcover, "landcover_sand")
        .filter(Filter::class("sand"))
        .color("rgba(247, 239, 195, 1)");
    style
        .fill(&openmaptiles.water(), "water")
        .filter(Filter::ne("brunnel", "tunnel"))
        .color(WATER);

    let above_ground = Filter::ne("brunnel", "tunnel");
    style
        .line(&waterway, "waterway_river")
        .filter(Filter::class("river").and(above_ground.clone()))
        .cap_round()
        .color(WATER)
        .width(zoom!(base 1.2; 11 => 0.5, 20 => 6.0));
    style
        .line(&waterway, "waterway_other")
        .filter(Filter::class_not("river").and(above_ground))
        .cap_round()
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
            .filter(Filter::geometry(Geometry::LineString).class("runway"))
            .color("#f0ede9")
            .width(zoom!(base 1.2; 11 => 3.0, 20 => 16.0));
        s.line("aeroway_taxiway")
            .minzoom(11.0)
            .filter(Filter::geometry(Geometry::LineString).class("taxiway"))
            .color("#f0ede9")
            .width(zoom!(base 1.2; 11 => 0.5, 20 => 6.0));
    });

    style
        .line(&transportation, "tunnel_service_track_casing")
        .minzoom(13.0)
        .filter(tunnel().class_in(["service", "track"]))
        .join_round()
        .paint(
            "line-color",
            stops![
                15.0 => "hsl(0, 0%, 94%)",
                18.0 => "hsl(0, 0%, 87%)",
                20.0 => "hsl(0, 0%, 82%)",
            ],
        )
        .dasharray(vec![0.5, 0.25])
        .paint(
            "line-width",
            stops![13.0 => 0.76, 16.0 => 4.0, 20.0 => 11.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_link_casing")
        .minzoom(13.0)
        .filter(tunnel().and_eq("ramp", 1.0))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 93%)",
                12.0 => "hsl(0, 0%, 93%)",
                15.0 => "hsl(0, 0%, 89%)",
                20.0 => "hsl(0, 0%, 82%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 2.12, 14.0 => 4.0, 20.0 => 23.5].base(1.2),
        );
    style
        .line(&transportation, "tunnel_street_casing")
        .minzoom(13.0)
        .filter(tunnel().class_in(["street", "street_limited"]))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                13.0 => "hsl(0, 0%, 93%)",
                16.0 => "hsl(0, 0%, 87.5%)",
                20.0 => "hsl(0, 0%, 82%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 2.12, 14.0 => 4.0, 20.0 => 20.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(tunnel().class_in(["secondary", "tertiary"]))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 91%)",
                11.0 => "hsl(0, 0%, 91%)",
                12.0 => "hsl(0, 0%, 91%)",
                13.0 => "hsl(0, 0%, 89%)",
                20.0 => "hsl(0, 0%, 82%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.76, 13.0 => 3.6, 20.0 => 30.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_primary_casing")
        .minzoom(13.0)
        .filter(tunnel().class("primary"))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![13.0 => "hsl(0, 0%, 83.5%)", 20.0 => "hsl(0, 0%, 76%)"],
        )
        .paint("line-width", stops![13.0 => 5.9, 20.0 => 36.0].base(1.2));
    style
        .line(&transportation, "tunnel_trunk_casing")
        .minzoom(13.0)
        .filter(tunnel().class("trunk"))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![13.0 => "hsl(0, 0%, 81%)", 20.0 => "hsl(0, 0%, 74.5%)"],
        )
        .paint("line-width", stops![13.0 => 5.9, 20.0 => 36.0].base(1.2));
    style
        .line(&transportation, "tunnel_motorway_link_casing")
        .minzoom(13.0)
        .filter(tunnel().class("motorway").and_eq("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            stops![
                12.0 => "hsl(0, 0%, 82%)",
                15.0 => "hsl(0, 0%, 77%)",
                20.0 => "hsl(0, 0%, 70%)",
            ],
        )
        .dasharray(vec![0.5, 0.25])
        .paint(
            "line-width",
            stops![8.0 => 1.74, 13.0 => 5.9, 20.0 => 35.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_motorway_casing")
        .minzoom(10.0)
        .filter(tunnel().class("motorway").and_ne("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            stops![
                7.0 => "hsl(0, 0%, 80%)",
                13.0 => "hsl(0, 0%, 75%)",
                20.0 => "hsl(0, 0%, 70%)",
            ],
        )
        .dasharray(vec![0.5, 0.25])
        .paint(
            "line-width",
            stops![5.0 => 1.4, 7.0 => 1.75, 13.0 => 5.9, 20.0 => 40.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_service_track")
        .minzoom(13.0)
        .filter(tunnel().class_in(["service", "track"]))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 97%)",
                13.0 => "hsl(0, 0%, 92%)",
                20.0 => "hsl(0, 0%, 87%)",
            ],
        )
        .paint(
            "line-width",
            stops![13.0 => 0.0, 16.0 => 2.0, 20.0 => 7.5].base(1.2),
        );
    style
        .line(&transportation, "tunnel_link")
        .minzoom(12.0)
        .filter(tunnel().and_eq("ramp", 1.0))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 94%)",
                12.0 => "hsl(0, 0%, 94%)",
                20.0 => "hsl(0, 0%, 87%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.0, 14.0 => 2.5, 20.0 => 20.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_minor")
        .minzoom(12.0)
        .filter(tunnel().class_in(["minor"]))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 93%)",
                13.0 => "hsl(0, 0%, 93%)",
                20.0 => "hsl(0, 0%, 87%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.0, 14.0 => 2.5, 20.0 => 18.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_path_pedestrian")
        .minzoom(13.0)
        .filter(
            tunnel()
                .and_geometry(Geometry::LineString)
                .class_in(["path", "pedestrian"]),
        )
        .color("hsl(0, 0%, 98%)")
        .dasharray(vec![1.0, 0.75])
        .paint("line-width", stops![13.0 => 0.75, 20.0 => 10.0].base(1.2));
    style
        .line(&transportation, "tunnel_secondary_tertiary")
        .minzoom(12.0)
        .filter(tunnel().class_in(["secondary", "tertiary"]))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 95%)",
                11.0 => "hsl(0, 0%, 95%)",
                12.0 => "hsl(0, 0%, 95%)",
                20.0 => "hsl(0, 0%, 87%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.0, 13.0 => 2.5, 20.0 => 26.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_primary")
        .minzoom(8.0)
        .filter(tunnel().class("primary"))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 64%)"],
        )
        .paint(
            "line-width",
            stops![8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_trunk")
        .minzoom(6.0)
        .filter(tunnel().class("trunk"))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 61%)"],
        )
        .paint(
            "line-width",
            stops![6.0 => 1.0, 10.0 => 2.85, 13.0 => 4.5, 20.0 => 32.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_motorway_link")
        .minzoom(8.0)
        .filter(tunnel().class("motorway").and_eq("ramp", 1.0))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 84%)", 20.0 => "hsl(0, 0%, 76%)"],
        )
        .paint(
            "line-width",
            stops![8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_motorway")
        .minzoom(5.0)
        .filter(tunnel().class("motorway").and_ne("ramp", 1.0))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 84%)", 20.0 => "hsl(0, 0%, 76%)"],
        )
        .paint(
            "line-width",
            stops![5.0 => 1.0, 7.0 => 1.0, 13.0 => 4.5, 20.0 => 36.0].base(1.2),
        );
    style
        .line(&transportation, "tunnel_major_rail")
        .filter(tunnel().class_in(["rail"]))
        .cap_round()
        .color("hsl(0, 0%, 86.6667%)")
        .paint(
            "line-width",
            stops![14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0].base(1.4),
        );
    style
        .line(&transportation, "tunnel_major_rail_hatching")
        .filter(tunnel().class("rail"))
        .color("hsl(0, 0%, 86.6667%)")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops![14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0].base(1.4),
        );
    style
        .line(&transportation, "tunnel_transit_rail")
        .filter(tunnel().class_in(["transit"]))
        .cap_round()
        .color("hsl(0, 0%, 86.6667%)")
        .paint(
            "line-width",
            stops![14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0].base(1.4),
        );
    style
        .line(&transportation, "tunnel_transit_rail_hatching")
        .filter(tunnel().class("transit"))
        .color("hsl(0, 0%, 86.6667%)")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops![14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0].base(1.4),
        );
    style
        .fill(&transportation, "road_area_pattern")
        .filter(Filter::geometry(Geometry::Polygon))
        .pattern("pedestrian_polygon");
    style
        .line(&transportation, "road_service_track_casing")
        .minzoom(13.0)
        .filter(surface().class_in(["service", "track"]))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                15.0 => "hsl(0, 0%, 88%)",
                18.0 => "hsl(0, 0%, 74%)",
                20.0 => "hsl(0, 0%, 64%)",
            ],
        )
        .paint(
            "line-width",
            stops![13.0 => 0.76, 16.0 => 4.0, 20.0 => 11.0].base(1.2),
        );
    style
        .line(&transportation, "road_link_casing")
        .minzoom(13.0)
        .filter(
            surface()
                .and_not_in(
                    "class",
                    ["pedestrian", "path", "track", "service", "motorway"],
                )
                .and_eq("ramp", 1.0),
        )
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 86%)",
                12.0 => "hsl(0, 0%, 86%)",
                15.0 => "hsl(0, 0%, 78%)",
                20.0 => "hsl(0, 0%, 64%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 2.12, 14.0 => 4.0, 20.0 => 23.5].base(1.2),
        );
    style
        .line(&transportation, "road_minor_casing")
        .minzoom(13.0)
        .filter(
            Filter::geometry(Geometry::LineString)
                .and_not_in("brunnel", ["bridge", "tunnel"])
                .class_in(["minor"])
                .and_ne("ramp", 1.0),
        )
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                13.0 => "hsl(0, 0%, 86%)",
                16.0 => "hsl(0, 0%, 75%)",
                20.0 => "hsl(0, 0%, 64%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 2.12, 14.0 => 4.0, 20.0 => 20.0].base(1.2),
        );
    style
        .line(&transportation, "road_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(
            surface()
                .class_in(["secondary", "tertiary"])
                .and_ne("ramp", 1.0),
        )
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 82%)",
                11.0 => "hsl(0, 0%, 82%)",
                12.0 => "hsl(0, 0%, 82%)",
                13.0 => "hsl(0, 0%, 78%)",
                20.0 => "hsl(0, 0%, 64%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.76, 13.0 => 3.6, 20.0 => 30.0].base(1.2),
        );
    style
        .line(&transportation, "road_primary_casing")
        .minzoom(13.0)
        .filter(surface().class("primary"))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![13.0 => "hsl(0, 0%, 67%)", 20.0 => "hsl(0, 0%, 52%)"],
        )
        .paint("line-width", stops![13.0 => 5.9, 20.0 => 36.0].base(1.2));
    style
        .line(&transportation, "road_trunk_casing")
        .minzoom(13.0)
        .filter(surface().class("trunk"))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![13.0 => "hsl(0, 0%, 62%)", 20.0 => "hsl(0, 0%, 49%)"],
        )
        .paint("line-width", stops![13.0 => 5.9, 20.0 => 36.0].base(1.2));
    style
        .line(&transportation, "road_motorway_link_casing")
        .minzoom(13.0)
        .filter(surface().class("motorway").and_eq("ramp", 1.0))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                12.0 => "hsl(0, 0%, 64%)",
                15.0 => "hsl(0, 0%, 54%)",
                20.0 => "hsl(0, 0%, 40%)",
            ],
        )
        .paint(
            "line-width",
            stops![8.0 => 1.74, 13.0 => 5.9, 20.0 => 35.0].base(1.2),
        );
    style
        .line(&transportation, "road_motorway_casing")
        .minzoom(10.0)
        .filter(surface().class("motorway").and_ne("ramp", 1.0))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                7.0 => "hsl(0, 0%, 60%)",
                13.0 => "hsl(0, 0%, 50%)",
                20.0 => "hsl(0, 0%, 40%)",
            ],
        )
        .paint(
            "line-width",
            stops![5.0 => 1.4, 7.0 => 1.75, 13.0 => 5.9, 20.0 => 40.0].base(1.2),
        );
    style
        .line(&transportation, "road_service_track")
        .minzoom(13.0)
        .filter(surface().class_in(["service", "track"]))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 94%)",
                13.0 => "hsl(0, 0%, 84%)",
                20.0 => "hsl(0, 0%, 74%)",
            ],
        )
        .paint(
            "line-width",
            stops![13.0 => 0.0, 16.0 => 2.0, 20.0 => 7.5].base(1.2),
        );
    style
        .line(&transportation, "road_link")
        .minzoom(12.0)
        .filter(surface().and_eq("ramp", 1.0).and_not_in(
            "class",
            ["pedestrian", "path", "track", "service", "motorway"],
        ))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 88%)",
                12.0 => "hsl(0, 0%, 88%)",
                20.0 => "hsl(0, 0%, 74%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.0, 14.0 => 2.5, 20.0 => 20.0].base(1.2),
        );
    style
        .line(&transportation, "road_minor")
        .minzoom(12.0)
        .filter(
            Filter::geometry(Geometry::LineString)
                .and_not_in("brunnel", ["bridge", "tunnel"])
                .class_in(["minor"]),
        )
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 86%)",
                13.0 => "hsl(0, 0%, 86%)",
                20.0 => "hsl(0, 0%, 74%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.0, 14.0 => 2.5, 20.0 => 18.0].base(1.2),
        );
    style
        .line(&transportation, "road_path_pedestrian")
        .minzoom(13.0)
        .filter(
            Filter::geometry(Geometry::LineString)
                .and_not_in("brunnel", ["bridge", "tunnel"])
                .class_in(["path", "pedestrian"]),
        )
        .join_round()
        .color("hsl(0, 0%, 96%)")
        .dasharray(vec![1.0, 0.7])
        .paint("line-width", stops![13.0 => 0.75, 20.0 => 10.0].base(1.2));
    style
        .line(&transportation, "road_secondary_tertiary")
        .minzoom(12.0)
        .filter(surface().class_in(["secondary", "tertiary"]))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 90%)",
                11.0 => "hsl(0, 0%, 90%)",
                12.0 => "hsl(0, 0%, 90%)",
                20.0 => "hsl(0, 0%, 74%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.0, 13.0 => 2.5, 20.0 => 26.0].base(1.2),
        );
    style
        .line(&transportation, "road_primary")
        .minzoom(8.0)
        .filter(surface().class("primary"))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 64%)"],
        )
        .paint(
            "line-width",
            stops![8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0].base(1.2),
        );
    style
        .line(&transportation, "road_trunk")
        .minzoom(6.0)
        .filter(surface().class("trunk"))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 61%)"],
        )
        .paint(
            "line-width",
            stops![6.0 => 1.0, 10.0 => 2.85, 13.0 => 4.5, 20.0 => 32.0].base(1.2),
        );
    style
        .line(&transportation, "road_motorway_link")
        .minzoom(8.0)
        .filter(surface().class("motorway").and_eq("ramp", 1.0))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 52%)"],
        )
        .paint(
            "line-width",
            stops![8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0].base(1.2),
        );
    style
        .line(&transportation, "road_motorway")
        .minzoom(5.0)
        .filter(surface().class("motorway").and_ne("ramp", 1.0))
        .cap_round()
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 52%)"],
        )
        .paint(
            "line-width",
            stops![5.0 => 1.0, 7.0 => 1.0, 13.0 => 4.5, 20.0 => 36.0].base(1.2),
        );
    style
        .line(&transportation, "road_major_rail")
        .filter(surface().class("rail"))
        .cap_round()
        .color("#bbb")
        .paint(
            "line-width",
            stops![14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0].base(1.4),
        );
    style
        .line(&transportation, "road_major_rail_hatching")
        .filter(surface().class("rail"))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops![14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0].base(1.4),
        );
    style
        .line(&transportation, "road_transit_rail")
        .filter(surface().class("transit"))
        .cap_round()
        .color("#bbb")
        .paint(
            "line-width",
            stops![14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0].base(1.4),
        );
    style
        .line(&transportation, "road_transit_rail_hatching")
        .filter(surface().class("transit"))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops![14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0].base(1.4),
        );
    style
        .line(&transportation, "bridge_service_track_casing")
        .minzoom(13.0)
        .filter(bridge().class_in(["service", "track"]))
        .join_round()
        .paint(
            "line-color",
            stops![
                15.0 => "hsl(0, 0%, 88%)",
                18.0 => "hsl(0, 0%, 74%)",
                20.0 => "hsl(0, 0%, 64%)",
            ],
        )
        .paint(
            "line-width",
            stops![13.0 => 0.76, 16.0 => 4.0, 20.0 => 11.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_link_casing")
        .minzoom(13.0)
        .filter(bridge().class("link"))
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 86%)",
                12.0 => "hsl(0, 0%, 86%)",
                15.0 => "hsl(0, 0%, 78%)",
                20.0 => "hsl(0, 0%, 64%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 2.12, 14.0 => 4.0, 20.0 => 23.5].base(1.2),
        );
    style
        .line(&transportation, "bridge_street_casing")
        .minzoom(13.0)
        .filter(bridge().class_in(["street", "street_limited"]))
        .join_round()
        .paint(
            "line-color",
            stops![
                13.0 => "hsl(0, 0%, 86%)",
                16.0 => "hsl(0, 0%, 75%)",
                20.0 => "hsl(0, 0%, 64%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 2.12, 14.0 => 4.0, 20.0 => 20.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_path_pedestrian_casing")
        .minzoom(13.0)
        .filter(
            bridge()
                .and_geometry(Geometry::LineString)
                .class_in(["path", "pedestrian"]),
        )
        .paint(
            "line-color",
            stops![14.0 => "hsl(0, 0%, 88%)", 20.0 => "hsl(0, 0%, 64%)"],
        )
        .dasharray(vec![1.0, 0.0])
        .paint("line-width", stops![14.0 => 1.125, 20.0 => 18.0].base(1.2));
    style
        .line(&transportation, "bridge_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(bridge().class_in(["secondary", "tertiary"]))
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 82%)",
                11.0 => "hsl(0, 0%, 82%)",
                12.0 => "hsl(0, 0%, 82%)",
                13.0 => "hsl(0, 0%, 78%)",
                20.0 => "hsl(0, 0%, 64%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.76, 13.0 => 3.6, 20.0 => 30.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_primary_casing")
        .minzoom(13.0)
        .filter(bridge().class("primary"))
        .join_round()
        .paint(
            "line-color",
            stops![13.0 => "hsl(0, 0%, 67%)", 20.0 => "hsl(0, 0%, 52%)"],
        )
        .paint("line-width", stops![13.0 => 5.9, 20.0 => 36.0].base(1.2));
    style
        .line(&transportation, "bridge_trunk_casing")
        .minzoom(13.0)
        .filter(bridge().class("trunk"))
        .join_round()
        .paint(
            "line-color",
            stops![13.0 => "hsl(0, 0%, 62%)", 20.0 => "hsl(0, 0%, 49%)"],
        )
        .paint("line-width", stops![13.0 => 5.9, 20.0 => 36.0].base(1.2));
    style
        .line(&transportation, "bridge_motorway_link_casing")
        .minzoom(13.0)
        .filter(bridge().class("motorway").and_eq("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            stops![
                12.0 => "hsl(0, 0%, 64%)",
                15.0 => "hsl(0, 0%, 54%)",
                20.0 => "hsl(0, 0%, 40%)",
            ],
        )
        .paint(
            "line-width",
            stops![8.0 => 1.74, 13.0 => 5.9, 20.0 => 35.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_motorway_casing")
        .minzoom(10.0)
        .filter(bridge().class("motorway").and_ne("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            stops![
                7.0 => "hsl(0, 0%, 60%)",
                13.0 => "hsl(0, 0%, 50%)",
                20.0 => "hsl(0, 0%, 40%)",
            ],
        )
        .paint(
            "line-width",
            stops![5.0 => 1.4, 7.0 => 1.75, 13.0 => 5.9, 20.0 => 40.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_service_track")
        .minzoom(13.0)
        .filter(bridge().class_in(["service", "track"]))
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 94%)",
                13.0 => "hsl(0, 0%, 84%)",
                20.0 => "hsl(0, 0%, 74%)",
            ],
        )
        .paint(
            "line-width",
            stops![13.0 => 0.0, 16.0 => 2.0, 20.0 => 7.5].base(1.2),
        );
    style
        .line(&transportation, "bridge_link")
        .minzoom(12.0)
        .filter(bridge().class("link"))
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 88%)",
                12.0 => "hsl(0, 0%, 88%)",
                20.0 => "hsl(0, 0%, 74%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.0, 14.0 => 2.5, 20.0 => 20.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_street")
        .minzoom(12.0)
        .filter(bridge().class_in(["minor"]))
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 86%)",
                13.0 => "hsl(0, 0%, 86%)",
                20.0 => "hsl(0, 0%, 74%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.0, 14.0 => 2.5, 20.0 => 18.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_path_pedestrian")
        .minzoom(13.0)
        .filter(
            bridge()
                .and_geometry(Geometry::LineString)
                .class_in(["path", "pedestrian"]),
        )
        .color("hsl(0, 0%, 96%)")
        .dasharray(vec![1.0, 0.3])
        .paint("line-width", stops![13.0 => 0.75, 20.0 => 10.0].base(1.2));
    style
        .line(&transportation, "bridge_secondary_tertiary")
        .minzoom(12.0)
        .filter(bridge().class_in(["secondary", "tertiary"]))
        .join_round()
        .paint(
            "line-color",
            stops![
                10.0 => "hsl(0, 0%, 90%)",
                11.0 => "hsl(0, 0%, 90%)",
                12.0 => "hsl(0, 0%, 90%)",
                20.0 => "hsl(0, 0%, 74%)",
            ],
        )
        .paint(
            "line-width",
            stops![12.0 => 1.0, 13.0 => 2.5, 20.0 => 26.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_primary")
        .minzoom(8.0)
        .filter(bridge().class("primary"))
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 64%)"],
        )
        .paint(
            "line-width",
            stops![8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_trunk")
        .minzoom(6.0)
        .filter(bridge().class("trunk"))
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 61%)"],
        )
        .paint(
            "line-width",
            stops![6.0 => 1.0, 10.0 => 2.85, 13.0 => 4.5, 20.0 => 32.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_motorway_link")
        .minzoom(8.0)
        .filter(bridge().class("motorway").and_eq("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 52%)"],
        )
        .paint(
            "line-width",
            stops![8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_motorway")
        .minzoom(5.0)
        .filter(bridge().class("motorway").and_ne("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            stops![10.0 => "hsl(0, 0%, 68%)", 20.0 => "hsl(0, 0%, 52%)"],
        )
        .paint(
            "line-width",
            stops![5.0 => 1.0, 7.0 => 1.0, 13.0 => 4.5, 20.0 => 36.0].base(1.2),
        );
    style
        .line(&transportation, "bridge_major_rail")
        .filter(bridge().class("rail"))
        .color("#bbb")
        .paint(
            "line-width",
            stops![14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0].base(1.4),
        );
    style
        .line(&transportation, "bridge_major_rail_hatching")
        .filter(bridge().class("rail"))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops![14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0].base(1.4),
        );
    style
        .line(&transportation, "bridge_transit_rail")
        .filter(bridge().class("transit"))
        .color("#bbb")
        .paint(
            "line-width",
            stops![14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0].base(1.4),
        );
    style
        .line(&transportation, "bridge_transit_rail_hatching")
        .filter(bridge().class("transit"))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            stops![14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0].base(1.4),
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
            .join_round()
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
            .filter(Filter::eq("admin_level", 2).and_not_has("claimed_by"));
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
            .filter(Filter::geometry(Geometry::LineString).class_in(["ferry"]))
            .rounded()
            .color("#99d")
            .width(2.0)
            .opacity(zoom!(12 => 0.0, 13 => 1.0))
            .dasharray(vec![0.3, 2.0]);
    });

    water_label(&mut style, &waterway, "water_name_line")
        .filter(Filter::geometry(Geometry::LineString))
        .placement("line");
    let water_name = openmaptiles.water_name();
    water_label(&mut style, &water_name, "water_name_lake_line")
        .filter(Filter::geometry(Geometry::LineString))
        .placement("line");
    water_label(&mut style, &water_name, "water_name_point")
        .filter(Filter::geometry(Geometry::Point));

    for (id, zoom, rank) in [
        ("poi_z16", 16.0, Filter::gte("rank", 20)),
        ("poi_z15", 15.0, Filter::gte("rank", 7).and_lt("rank", 20)),
        ("poi_z14", 14.0, Filter::gte("rank", 1).and_lt("rank", 7)),
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
        .placement("line")
        .text_field("{name}")
        .text_font([REGULAR])
        .text_offset([0.0, 0.15])
        .text_size(stops![13.0 => 12.0, 14.0 => 13.0])
        .text_color("#765")
        .text_halo("#ffffff", 1.0)
        .text_halo_blur(0.5);
    style
        .symbol(&transportation_name, "road_shield")
        .minzoom(7.0)
        .filter(Filter::expression(Expression::op(
            Operator::All,
            [
                Expression::op(Operator::Has, [Expression::literal("network")]),
                Expression::op(Operator::Has, [Expression::literal("ref")]),
                Expression::op(
                    Operator::LessEqual,
                    [
                        Expression::op(
                            Operator::Length,
                            [Expression::op(
                                Operator::Coalesce,
                                [
                                    Expression::op(
                                        Operator::Get,
                                        [Expression::literal("route_1_ref")],
                                    ),
                                    Expression::op(Operator::Get, [Expression::literal("ref")]),
                                ],
                            )],
                        ),
                        Expression::literal(6),
                    ],
                ),
            ],
        )))
        .layout(
            "icon-image",
            Expression::op(
                Operator::Let,
                [
                    Expression::literal("ref"),
                    Expression::op(
                        Operator::Coalesce,
                        [
                            Expression::op(Operator::Get, [Expression::literal("route_1_ref")]),
                            Expression::op(Operator::Get, [Expression::literal("ref")]),
                        ],
                    ),
                    Expression::op(
                        Operator::Let,
                        [
                            Expression::literal("shield_base"),
                            Expression::op(
                                Operator::Match,
                                [
                                    Expression::op(
                                        Operator::Get,
                                        [Expression::literal("route_1_network")],
                                    ),
                                    Expression::literal("US:I"),
                                    Expression::literal("us-interstate"),
                                    Expression::literal("US:US"),
                                    Expression::literal("us-highway"),
                                    Expression::literal("US:CA"),
                                    Expression::literal("us-state-ca"),
                                    Expression::literal("US:OR"),
                                    Expression::literal("us-state-or"),
                                    Expression::literal("US:AL"),
                                    Expression::literal("us-state-al"),
                                    Expression::literal("US:AR"),
                                    Expression::literal("us-state-ar"),
                                    Expression::literal("US:AZ"),
                                    Expression::literal("us-state-az"),
                                    Expression::literal("US:FL"),
                                    Expression::literal("us-state-fl"),
                                    Expression::literal("US:GA"),
                                    Expression::literal("us-state-ga"),
                                    Expression::literal("US:GU"),
                                    Expression::literal("us-state-gu"),
                                    Expression::literal("US:ID"),
                                    Expression::literal("us-state-id"),
                                    Expression::literal("US:KS"),
                                    Expression::literal("us-state-ks"),
                                    Expression::literal("US:LA"),
                                    Expression::literal("us-state-la"),
                                    Expression::literal("US:MN"),
                                    Expression::literal("us-state-mn"),
                                    Expression::literal("US:MO"),
                                    Expression::literal("us-state-mo"),
                                    Expression::literal("US:MP"),
                                    Expression::literal("us-state-mp"),
                                    Expression::literal("US:ND"),
                                    Expression::literal("us-state-nd"),
                                    Expression::literal("US:NH"),
                                    Expression::literal("us-state-nh"),
                                    Expression::literal("US:NM"),
                                    Expression::literal("us-state-nm"),
                                    Expression::literal("US:Navajo"),
                                    Expression::literal("us-state-navajo"),
                                    Expression::literal("US:OH"),
                                    Expression::literal("us-state-oh"),
                                    Expression::literal("US:OK"),
                                    Expression::literal("us-state-ok"),
                                    Expression::literal("US:PA"),
                                    Expression::literal("us-state-pa"),
                                    Expression::literal("US:SD"),
                                    Expression::literal("us-state-sd"),
                                    Expression::literal("US:UT"),
                                    Expression::literal("us-state-ut"),
                                    Expression::literal("US:VT"),
                                    Expression::literal("us-state-vt"),
                                    Expression::literal("US:WI"),
                                    Expression::literal("us-state-wi"),
                                    Expression::literal("CA:BC"),
                                    Expression::literal("us-ca-bc"),
                                    Expression::literal("CA:QC:A"),
                                    Expression::literal("us-ca-qc-a"),
                                    Expression::literal("CA:transcanada"),
                                    Expression::literal("us-ca-transcanada"),
                                    Expression::literal("CL:national"),
                                    Expression::literal("us-cl-national"),
                                    Expression::literal("CN:expressway"),
                                    Expression::literal("us-cn-expressway"),
                                    Expression::literal("EC:secundaria"),
                                    Expression::literal("us-ec-secundaria"),
                                    Expression::literal("IN:NH"),
                                    Expression::literal("us-in-nh"),
                                    Expression::literal("KR:expressway"),
                                    Expression::literal("us-kr-expressway"),
                                    Expression::literal("MX:MX"),
                                    Expression::literal("us-mx-mx"),
                                    Expression::literal("PE:national"),
                                    Expression::literal("us-pe-national"),
                                    Expression::literal("US:I:Business:Loop"),
                                    Expression::literal("us-i-business-loop"),
                                    Expression::literal("US:MN:Business"),
                                    Expression::literal("us-mn-business"),
                                    Expression::literal("US:OH:SCI"),
                                    Expression::literal("us-oh-sci"),
                                    Expression::literal("US:PA:Turnpike"),
                                    Expression::literal("us-pa-turnpike"),
                                    Expression::literal("co:national"),
                                    Expression::literal("us-co-national"),
                                    Expression::literal(""),
                                ],
                            ),
                            Expression::op(
                                Operator::Case,
                                [
                                    Expression::op(
                                        Operator::NotEqual,
                                        [
                                            Expression::op(
                                                Operator::Var,
                                                [Expression::literal("shield_base")],
                                            ),
                                            Expression::literal(""),
                                        ],
                                    ),
                                    Expression::op(
                                        Operator::Concat,
                                        [
                                            Expression::op(
                                                Operator::Var,
                                                [Expression::literal("shield_base")],
                                            ),
                                            Expression::op(
                                                Operator::Case,
                                                [
                                                    Expression::op(
                                                        Operator::LessEqual,
                                                        [
                                                            Expression::op(
                                                                Operator::Length,
                                                                [Expression::op(
                                                                    Operator::Var,
                                                                    [Expression::literal("ref")],
                                                                )],
                                                            ),
                                                            Expression::literal(2),
                                                        ],
                                                    ),
                                                    Expression::literal("_2"),
                                                    Expression::op(
                                                        Operator::All,
                                                        [
                                                            Expression::op(
                                                                Operator::GreaterEqual,
                                                                [
                                                                    Expression::op(
                                                                        Operator::Length,
                                                                        [Expression::op(
                                                                            Operator::Var,
                                                                            [Expression::literal(
                                                                                "ref",
                                                                            )],
                                                                        )],
                                                                    ),
                                                                    Expression::literal(4),
                                                                ],
                                                            ),
                                                            Expression::op(
                                                                Operator::Match,
                                                                [
                                                                    Expression::op(
                                                                        Operator::Get,
                                                                        [Expression::literal(
                                                                            "route_1_network",
                                                                        )],
                                                                    ),
                                                                    Expression::literal(
                                                                        "US:Navajo",
                                                                    ),
                                                                    Expression::literal(true),
                                                                    Expression::literal(
                                                                        "CN:expressway",
                                                                    ),
                                                                    Expression::literal(true),
                                                                    Expression::literal(
                                                                        "EC:secundaria",
                                                                    ),
                                                                    Expression::literal(true),
                                                                    Expression::literal("IN:NH"),
                                                                    Expression::literal(true),
                                                                    Expression::literal("MX:MX"),
                                                                    Expression::literal(true),
                                                                    Expression::literal(false),
                                                                ],
                                                            ),
                                                        ],
                                                    ),
                                                    Expression::literal("_4"),
                                                    Expression::literal("_3"),
                                                ],
                                            ),
                                        ],
                                    ),
                                    Expression::op(
                                        Operator::Match,
                                        [
                                            Expression::op(
                                                Operator::Get,
                                                [Expression::literal("route_1_network")],
                                            ),
                                            Expression::literal("US:WA"),
                                            Expression::literal("us-state-wa"),
                                            Expression::literal("US:AK"),
                                            Expression::literal("us-state-ak"),
                                            Expression::literal("US:AS"),
                                            Expression::literal("us-state-as"),
                                            Expression::literal("US:BIA"),
                                            Expression::literal("us-state-bia"),
                                            Expression::literal("US:CKC"),
                                            Expression::literal("us-state-ckc"),
                                            Expression::literal("US:CO"),
                                            Expression::literal("us-state-co"),
                                            Expression::literal("US:DC"),
                                            Expression::literal("us-state-dc"),
                                            Expression::literal("US:GLST"),
                                            Expression::literal("us-state-glst"),
                                            Expression::literal("US:GRR"),
                                            Expression::literal("us-state-grr"),
                                            Expression::literal("US:LHT"),
                                            Expression::literal("us-state-lht"),
                                            Expression::literal("US:NHT"),
                                            Expression::literal("us-state-nht"),
                                            Expression::literal("US:NV"),
                                            Expression::literal("us-state-nv"),
                                            Expression::literal("US:ORSB"),
                                            Expression::literal("us-state-orsb"),
                                            Expression::literal("US:PANYNJ"),
                                            Expression::literal("us-state-panynj"),
                                            Expression::literal("US:PIPC"),
                                            Expression::literal("us-state-pipc"),
                                            Expression::literal("US:SC"),
                                            Expression::literal("us-state-sc"),
                                            Expression::literal("CA:NB:tertiary"),
                                            Expression::literal("us-ca-nb-tertiary"),
                                            Expression::literal("CA:NS:S"),
                                            Expression::literal("us-ca-ns-s"),
                                            Expression::literal("CA:NT"),
                                            Expression::literal("us-ca-nt"),
                                            Expression::literal("CA:ON:Hamilton:Expressway"),
                                            Expression::literal("us-ca-on-hamilton-expressway"),
                                            Expression::literal("CA:ON:Toronto:Expressway"),
                                            Expression::literal("us-ca-on-toronto-expressway"),
                                            Expression::literal("CA:ON:primary"),
                                            Expression::literal("us-ca-on-primary"),
                                            Expression::literal("CA:PE"),
                                            Expression::literal("us-ca-pe"),
                                            Expression::literal("CA:QC:R"),
                                            Expression::literal("us-ca-qc-r"),
                                            Expression::literal("CA:SK:secondary"),
                                            Expression::literal("us-ca-sk-secondary"),
                                            Expression::literal("CA:transcanada:namedRoute"),
                                            Expression::literal("us-ca-transcanada-namedroute_2"),
                                            Expression::literal("GLCT"),
                                            Expression::literal("us-glct"),
                                            Expression::literal("GLCT:Loop"),
                                            Expression::literal("us-glct-loop"),
                                            Expression::literal("ID:national"),
                                            Expression::literal("us-id-national"),
                                            Expression::literal("MX:CDMX:EJE:CENTRAL"),
                                            Expression::literal("us-mx-cdmx-eje-central"),
                                            Expression::literal("MX:CDMX:EJE:NTE"),
                                            Expression::literal("us-mx-cdmx-eje-nte"),
                                            Expression::literal("MX:CDMX:EJE:OTE"),
                                            Expression::literal("us-mx-cdmx-eje-ote"),
                                            Expression::literal("MX:CDMX:EJE:PTE"),
                                            Expression::literal("us-mx-cdmx-eje-pte"),
                                            Expression::literal("MX:CDMX:EJE:SUR"),
                                            Expression::literal("us-mx-cdmx-eje-sur"),
                                            Expression::literal("NL:DR:Hunebed_Highway"),
                                            Expression::literal("us-nl-dr-hunebed-highway"),
                                            Expression::literal("NZ:Touring:AH"),
                                            Expression::literal("us-nz-touring-ah"),
                                            Expression::literal("NZ:Touring:CNZWT"),
                                            Expression::literal("us-nz-touring-cnzwt"),
                                            Expression::literal("NZ:Touring:MWT"),
                                            Expression::literal("us-nz-touring-mwt"),
                                            Expression::literal("NZ:Touring:PCH"),
                                            Expression::literal("us-nz-touring-pch"),
                                            Expression::literal("NZ:Touring:SLH"),
                                            Expression::literal("us-nz-touring-slh"),
                                            Expression::literal("NZ:Touring:SSR"),
                                            Expression::literal("us-nz-touring-ssr"),
                                            Expression::literal("NZ:Touring:TCDH"),
                                            Expression::literal("us-nz-touring-tcdh"),
                                            Expression::literal("NZ:Touring:TEH"),
                                            Expression::literal("us-nz-touring-teh"),
                                            Expression::literal("NZ:Touring:VLH"),
                                            Expression::literal("us-nz-touring-vlh"),
                                            Expression::literal("NZ:WRR"),
                                            Expression::literal("us-nz-wrr"),
                                            Expression::literal("PK:motorway"),
                                            Expression::literal("us-pk-motorway"),
                                            Expression::literal("TW:freeway"),
                                            Expression::literal("us-tw-freeway"),
                                            Expression::literal(
                                                "US:AL:Baldwin:Foley_Beach_Express",
                                            ),
                                            Expression::literal(
                                                "us-al-baldwin-foley-beach-express",
                                            ),
                                            Expression::literal("US:AZ:Indian"),
                                            Expression::literal("us-az-indian"),
                                            Expression::literal("US:AZ:Scenic"),
                                            Expression::literal("us-az-scenic"),
                                            Expression::literal(
                                                "US:CA:San_Francisco:49_Mile_Scenic_Drive",
                                            ),
                                            Expression::literal(
                                                "us-ca-san-francisco-49-mile-scenic-drive",
                                            ),
                                            Expression::literal("US:CO:E470"),
                                            Expression::literal("us-co-e470"),
                                            Expression::literal("US:CO:NW"),
                                            Expression::literal("us-co-nw"),
                                            Expression::literal("US:CO:Scenic"),
                                            Expression::literal("us-co-scenic"),
                                            Expression::literal("US:CT:Parkway"),
                                            Expression::literal("us-ct-parkway"),
                                            Expression::literal("US:FL:Toll"),
                                            Expression::literal("us-fl-toll"),
                                            Expression::literal("US:FL:Turnpike"),
                                            Expression::literal("us-fl-turnpike"),
                                            Expression::literal("US:IL:Cook:Chicago:Skyway"),
                                            Expression::literal("us-il-cook-chicago-skyway"),
                                            Expression::literal("US:IN:JHMHT"),
                                            Expression::literal("us-in-jhmht"),
                                            Expression::literal("US:IN:Toll"),
                                            Expression::literal("us-in-toll"),
                                            Expression::literal("US:KS:Turnpike"),
                                            Expression::literal("us-ks-turnpike"),
                                            Expression::literal("US:KY:Parkway"),
                                            Expression::literal("us-ky-parkway"),
                                            Expression::literal("US:LA:Causeway"),
                                            Expression::literal("us-la-causeway"),
                                            Expression::literal("US:MA:Turnpike"),
                                            Expression::literal("us-ma-turnpike"),
                                            Expression::literal("US:ME:Turnpike"),
                                            Expression::literal("us-me-turnpike"),
                                            Expression::literal("US:MT:secondary"),
                                            Expression::literal("us-mt-secondary"),
                                            Expression::literal("US:NE:Scenic"),
                                            Expression::literal("us-ne-scenic"),
                                            Expression::literal("US:NH:Turnpike"),
                                            Expression::literal("us-nh-turnpike"),
                                            Expression::literal("US:NJ:ACE"),
                                            Expression::literal("us-nj-ace"),
                                            Expression::literal("US:NJ:GSP"),
                                            Expression::literal("us-nj-gsp"),
                                            Expression::literal("US:NJ:NJTP"),
                                            Expression::literal("us-nj-njtp"),
                                            Expression::literal("US:NM:Frontage"),
                                            Expression::literal("us-nm-frontage"),
                                            Expression::literal("US:NPS:Blue_Ridge"),
                                            Expression::literal("us-nps-blue-ridge"),
                                            Expression::literal("US:NPS:Natchez_Trace"),
                                            Expression::literal("us-nps-natchez-trace"),
                                            Expression::literal("US:NV:Clark"),
                                            Expression::literal("us-nv-clark"),
                                            Expression::literal("US:NY:Parkway:LI"),
                                            Expression::literal("us-ny-parkway-li"),
                                            Expression::literal("US:NY:Parkway:LOSP"),
                                            Expression::literal("us-ny-parkway-losp"),
                                            Expression::literal("US:NY:Parkway:NYC"),
                                            Expression::literal("us-ny-parkway-nyc"),
                                            Expression::literal("US:NY:STE"),
                                            Expression::literal("us-ny-ste"),
                                            Expression::literal("US:NY:Scenic"),
                                            Expression::literal("us-ny-scenic"),
                                            Expression::literal("US:NY:TBTA"),
                                            Expression::literal("us-ny-tbta"),
                                            Expression::literal("US:NY:Thruway"),
                                            Expression::literal("us-ny-thruway"),
                                            Expression::literal("US:OH:ASD"),
                                            Expression::literal("us-oh-asd"),
                                            Expression::literal("US:OH:HOL"),
                                            Expression::literal("us-oh-hol"),
                                            Expression::literal("US:OH:JHMHT"),
                                            Expression::literal("us-oh-jhmht"),
                                            Expression::literal("US:OH:OEC"),
                                            Expression::literal("us-oh-oec"),
                                            Expression::literal("US:OH:TUS:Salem"),
                                            Expression::literal("us-oh-tus-salem"),
                                            Expression::literal("US:OH:Turnpike"),
                                            Expression::literal("us-oh-turnpike"),
                                            Expression::literal("US:OK:Turnpike"),
                                            Expression::literal("us-ok-turnpike"),
                                            Expression::literal("US:SD:Custer:CSP"),
                                            Expression::literal("us-sd-custer-csp"),
                                            Expression::literal("US:TN:primary"),
                                            Expression::literal("us-tn-primary"),
                                            Expression::literal("US:TX:Fort_Bend:FBCTRA"),
                                            Expression::literal("us-tx-fort-bend-fbctra"),
                                            Expression::literal("US:WI:Rustic"),
                                            Expression::literal("us-wi-rustic"),
                                            Expression::op(
                                                Operator::Concat,
                                                [
                                                    Expression::literal("default_"),
                                                    Expression::op(
                                                        Operator::ToString,
                                                        [Expression::op(
                                                            Operator::Length,
                                                            [Expression::op(
                                                                Operator::Var,
                                                                [Expression::literal("ref")],
                                                            )],
                                                        )],
                                                    ),
                                                ],
                                            ),
                                        ],
                                    ),
                                ],
                            ),
                        ],
                    ),
                ],
            ),
        )
        .icon_rotation_alignment("viewport")
        .layout("symbol-placement", stops![10.0 => "point", 11.0 => "line"])
        .spacing(500.0)
        .layout(
            "text-field",
            Expression::op(
                Operator::Let,
                [
                    Expression::literal("ref"),
                    Expression::op(
                        Operator::Coalesce,
                        [
                            Expression::op(Operator::Get, [Expression::literal("route_1_ref")]),
                            Expression::op(Operator::Get, [Expression::literal("ref")]),
                        ],
                    ),
                    Expression::op(
                        Operator::Match,
                        [
                            Expression::op(Operator::Get, [Expression::literal("route_1_network")]),
                            Expression::literal("CA:transcanada:namedRoute"),
                            Expression::literal(""),
                            Expression::literal("MX:CDMX:EJE:CENTRAL"),
                            Expression::literal(""),
                            Expression::literal("US:FL:Turnpike"),
                            Expression::literal(""),
                            Expression::literal("US:GLST"),
                            Expression::literal(""),
                            Expression::literal("US:GRR"),
                            Expression::literal(""),
                            Expression::literal("US:IL:Cook:Chicago:Skyway"),
                            Expression::literal(""),
                            Expression::literal("US:IN:Toll"),
                            Expression::literal(""),
                            Expression::literal("US:KS:Turnpike"),
                            Expression::literal(""),
                            Expression::literal("US:MA:Turnpike"),
                            Expression::literal(""),
                            Expression::literal("US:NPS:Blue_Ridge"),
                            Expression::literal(""),
                            Expression::literal("US:NPS:Natchez_Trace"),
                            Expression::literal(""),
                            Expression::literal("US:NY:Parkway:LOSP"),
                            Expression::literal(""),
                            Expression::literal("US:NY:STE"),
                            Expression::literal(""),
                            Expression::literal("US:NY:Thruway"),
                            Expression::literal(""),
                            Expression::op(Operator::Var, [Expression::literal("ref")]),
                        ],
                    ),
                ],
            ),
        )
        .text_font(["Overpass Heavy"])
        .layout(
            "text-offset",
            Expression::op(
                Operator::Match,
                [
                    Expression::op(Operator::Get, [Expression::literal("route_1_network")]),
                    Expression::array(["US:I", "US:I:Business:Loop"]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.06])]),
                    Expression::array([
                        "US:OH",
                        "US:MT:secondary",
                        "US:NM:Frontage",
                        "US:OH:SCI",
                        "US:OR",
                        "US:SD",
                        "US:NV:Clark",
                    ]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, -0.12])]),
                    Expression::literal("US:WA"),
                    Expression::op(Operator::Literal, [Expression::array([-0.06, -0.01])]),
                    Expression::literal("CA:NB:tertiary"),
                    Expression::op(Operator::Literal, [Expression::array([-0.06, 0.11])]),
                    Expression::array(["US:CA", "US:SC", "US:VT", "PE:national"]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.18])]),
                    Expression::literal("US:AK"),
                    Expression::op(Operator::Literal, [Expression::array([0.0, -0.45])]),
                    Expression::array(["US:AL", "US:MO", "US:WI", "US:SD:Custer:CSP"]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, -0.18])]),
                    Expression::array(["US:AS", "US:CO", "US:CO:E470", "CA:SK:secondary"]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.51])]),
                    Expression::literal("US:DC"),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.29])]),
                    Expression::literal("US:FL"),
                    Expression::op(Operator::Literal, [Expression::array([-0.15, 0.12])]),
                    Expression::literal("US:ID"),
                    Expression::op(Operator::Literal, [Expression::array([0.24, -0.45])]),
                    Expression::literal("US:ND"),
                    Expression::op(Operator::Literal, [Expression::array([-0.18, 0.0])]),
                    Expression::literal("US:NH"),
                    Expression::op(Operator::Literal, [Expression::array([0.12, 0.0])]),
                    Expression::literal("US:NV"),
                    Expression::op(Operator::Literal, [Expression::array([0.0, -0.64])]),
                    Expression::literal("CA:NT"),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.41])]),
                    Expression::literal("MX:CDMX:EJE:NTE"),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.36])]),
                    Expression::literal("MX:CDMX:EJE:OTE"),
                    Expression::op(Operator::Literal, [Expression::array([-0.36, 0.0])]),
                    Expression::literal("MX:CDMX:EJE:PTE"),
                    Expression::op(Operator::Literal, [Expression::array([0.36, 0.0])]),
                    Expression::array([
                        "MX:CDMX:EJE:SUR",
                        "US:NY:Parkway:NYC",
                        "US:TX:Fort_Bend:FBCTRA",
                    ]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, -0.36])]),
                    Expression::array([
                        "US:MN",
                        "US:OK",
                        "CA:ON:primary",
                        "CN:expressway",
                        "KR:expressway",
                        "NL:DR:Hunebed_Highway",
                        "US:MN:Business",
                    ]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.24])]),
                    Expression::literal("US:LA"),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.31])]),
                    Expression::literal("US:CO:NW"),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.6])]),
                    Expression::literal("US:FL:Toll"),
                    Expression::op(Operator::Literal, [Expression::array([-0.18, 0.18])]),
                    Expression::array(["US:KY:Parkway", "co:national"]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, -0.24])]),
                    Expression::literal("US:NY:Parkway:LI"),
                    Expression::op(Operator::Literal, [Expression::array([0.24, -0.36])]),
                    Expression::literal("US:OH:TUS:Salem"),
                    Expression::op(Operator::Literal, [Expression::array([-0.12, -0.18])]),
                    Expression::literal("US:TN:primary"),
                    Expression::op(Operator::Literal, [Expression::array([0.0, -0.3])]),
                    Expression::literal("US:WI:Rustic"),
                    Expression::op(Operator::Literal, [Expression::array([-0.15, 0.3])]),
                    Expression::array(["US:AZ:Indian", "US:BIA"]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, -0.01])]),
                    Expression::op(Operator::Literal, [Expression::array([0.0, 0.11])]),
                ],
            ),
        )
        .text_rotation_alignment("viewport")
        .layout(
            "text-size",
            Expression::op(
                Operator::Let,
                [
                    Expression::literal("ref"),
                    Expression::op(
                        Operator::Coalesce,
                        [
                            Expression::op(Operator::Get, [Expression::literal("route_1_ref")]),
                            Expression::op(Operator::Get, [Expression::literal("ref")]),
                        ],
                    ),
                    Expression::op(
                        Operator::Min,
                        [
                            Expression::op(
                                Operator::Match,
                                [
                                    Expression::op(
                                        Operator::Get,
                                        [Expression::literal("route_1_network")],
                                    ),
                                    Expression::array(["US:I", "US:I:Business:Loop"]),
                                    Expression::literal(9.5),
                                    Expression::array([
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
                                        "co:national",
                                    ]),
                                    Expression::literal(9),
                                    Expression::literal(8.5),
                                ],
                            ),
                            Expression::op(
                                Operator::Divide,
                                [
                                    Expression::op(
                                        Operator::Match,
                                        [
                                            Expression::op(
                                                Operator::Get,
                                                [Expression::literal("route_1_network")],
                                            ),
                                            Expression::array(["US:GU", "US:AK"]),
                                            Expression::literal(24.5),
                                            Expression::array([
                                                "US:BIA",
                                                "MX:CDMX:EJE:NTE",
                                                "MX:CDMX:EJE:SUR",
                                            ]),
                                            Expression::literal(22.5),
                                            Expression::array([
                                                "US:NV",
                                                "US:FL:Toll",
                                                "US:NY:Parkway:LI",
                                            ]),
                                            Expression::literal(21.5),
                                            Expression::literal("US:WA"),
                                            Expression::literal(22),
                                            Expression::array(["US:CA", "US:NM:Frontage"]),
                                            Expression::literal(29),
                                            Expression::array([
                                                "CA:ON:primary",
                                                "CA:PE",
                                                "CA:QC:R",
                                            ]),
                                            Expression::literal(28),
                                            Expression::literal("US:DC"),
                                            Expression::literal(24),
                                            Expression::literal("US:AZ:Indian"),
                                            Expression::literal(24),
                                            Expression::array(["CA:NB:tertiary", "US:CO:NW"]),
                                            Expression::literal(23),
                                            Expression::array([
                                                "CA:SK:secondary",
                                                "NZ:Touring:TEH",
                                                "US:NV:Clark",
                                            ]),
                                            Expression::literal(27),
                                            Expression::literal("CA:ON:Toronto:Expressway"),
                                            Expression::literal(15),
                                            Expression::literal("TW:freeway"),
                                            Expression::literal(18.5),
                                            Expression::literal("US:MT:secondary"),
                                            Expression::literal(21),
                                            Expression::array([
                                                "CA:NT",
                                                "US:OH:ASD",
                                                "US:TX:Fort_Bend:FBCTRA",
                                            ]),
                                            Expression::literal(25),
                                            Expression::array(["US:I", "US:I:Business:Loop"]),
                                            Expression::literal(32),
                                            Expression::literal(30),
                                        ],
                                    ),
                                    Expression::op(
                                        Operator::Length,
                                        [Expression::op(
                                            Operator::Var,
                                            [Expression::literal("ref")],
                                        )],
                                    ),
                                ],
                            ),
                        ],
                    ),
                ],
            ),
        )
        .layout(
            "icon-size",
            Expression::op(
                Operator::Match,
                [
                    Expression::op(Operator::Get, [Expression::literal("route_1_network")]),
                    Expression::array([
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
                        "co:national",
                    ]),
                    Expression::literal(1),
                    Expression::literal(0.667),
                ],
            ),
        )
        .paint(
            "text-color",
            Expression::op(
                Operator::Match,
                [
                    Expression::op(Operator::Get, [Expression::literal("route_1_network")]),
                    Expression::array(["US:I", "US:I:Business:Loop"]),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:CA"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:AS"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:GU"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:ID"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:MN"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:SC"),
                    Expression::literal("#003f87"),
                    Expression::literal("US:VT"),
                    Expression::literal("#006747"),
                    Expression::literal("CA:BC"),
                    Expression::literal("#003f87"),
                    Expression::literal("CA:NT"),
                    Expression::literal("#ffffff"),
                    Expression::literal("CA:QC:A"),
                    Expression::literal("#ffffff"),
                    Expression::literal("CA:QC:R"),
                    Expression::literal("#ffffff"),
                    Expression::literal("CA:SK:secondary"),
                    Expression::literal("#006747"),
                    Expression::literal("CA:transcanada"),
                    Expression::literal("#006747"),
                    Expression::literal("CL:national"),
                    Expression::literal("#ffffff"),
                    Expression::literal("CN:expressway"),
                    Expression::literal("#ffffff"),
                    Expression::literal("EC:secundaria"),
                    Expression::literal("#ffffff"),
                    Expression::literal("KR:expressway"),
                    Expression::literal("#ffffff"),
                    Expression::literal("NL:DR:Hunebed_Highway"),
                    Expression::literal("#ffffff"),
                    Expression::literal("PK:motorway"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:CO:NW"),
                    Expression::literal("#003f87"),
                    Expression::literal("US:KY:Parkway"),
                    Expression::literal("#003f87"),
                    Expression::literal("US:MN:Business"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:NV:Clark"),
                    Expression::literal("#003f87"),
                    Expression::literal("US:OH:ASD"),
                    Expression::literal("#006747"),
                    Expression::literal("US:OH:HOL"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:PA:Turnpike"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:TX:Fort_Bend:FBCTRA"),
                    Expression::literal("#ffffff"),
                    Expression::literal("US:WI:Rustic"),
                    Expression::literal("#ffcd00"),
                    Expression::literal("#000000"),
                ],
            ),
        )
        .paint(
            "text-halo-color",
            Expression::op(
                Operator::Match,
                [
                    Expression::op(Operator::Get, [Expression::literal("route_1_network")]),
                    Expression::literal("US:DC"),
                    Expression::literal("#ffffff"),
                    Expression::literal("rgba(0,0,0,0)"),
                ],
            ),
        )
        .paint(
            "text-halo-width",
            Expression::op(
                Operator::Match,
                [
                    Expression::op(Operator::Get, [Expression::literal("route_1_network")]),
                    Expression::literal("US:DC"),
                    Expression::literal(1),
                    Expression::literal(0),
                ],
            ),
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
        .filter(Filter::class("village"))
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
        .filter(Filter::class("town"))
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
        .filter(Filter::class("city"))
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
        .filter(Filter::class("state"))
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
            .filter(rank.class("country"))
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
        .filter(Filter::class("continent"))
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
