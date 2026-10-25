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

/// Footpaths and pedestrian streets.
fn foot_path() -> Filter {
    Filter::linestring().class_in(["path", "pedestrian"])
}

fn minor_color() -> Stops<Color> {
    zoom![13.0 => hsl!(0, 0%, 86%), 20.0 => hsl!(0, 0%, 74%)]
}

const MINOR_WIDTH_BASE: f64 = 1.2;
const MINOR_WIDTH: [(f64, f64); 3] = [(12.0, 1.0), (14.0, 2.5), (20.0, 18.0)];

/// A minor road's width in pixels at `zoom`, as MapLibre interpolates it.
fn minor_width(zoom: f64) -> f64 {
    let zoom = zoom.clamp(MINOR_WIDTH[0].0, MINOR_WIDTH[MINOR_WIDTH.len() - 1].0);
    let next = MINOR_WIDTH
        .iter()
        .position(|(z, _)| zoom <= *z)
        .expect("zoom is clamped to the stops")
        .max(1);
    let ((z0, w0), (z1, w1)) = (MINOR_WIDTH[next - 1], MINOR_WIDTH[next]);
    let t = (MINOR_WIDTH_BASE.powf(zoom - z0) - 1.0) / (MINOR_WIDTH_BASE.powf(z1 - z0) - 1.0);
    w0 + t * (w1 - w0)
}

/// A roundabout's radius, from its center to the middle of its ring, widened by `pad` times a
/// minor road's width.
fn roundabout_radius(pad: f64) -> Expression {
    let mut operands = vec![expr!(["exponential", 2.0]), expr!(["zoom"])];
    for zoom in (12..=24).map(f64::from) {
        // `radius` is in pixels of the z14 tile, which MapLibre draws 512 pixels across.
        let tile_scale = 2.0 * (zoom - 14.0).exp2();
        let pad = pad * minor_width(zoom);
        operands.push(zoom.into());
        operands.push(expr!([
            "max",
            0.0,
            ["+", ["*", ["get", "radius"], tile_scale], pad]
        ]));
    }
    Expression::op(Operator::Interpolate, operands)
}

fn above_ground() -> Filter {
    Filter::ne("brunnel", "tunnel")
}

mod road;
mod road_shield;
use road::{Grade, Road, Roads};
use road_shield::RoadShields;

/// A published revision of the style, served side by side so older clients keep working.
#[derive(Clone, Copy, PartialEq)]
pub enum Version {
    V3,
    /// Draws buildings in 3D, which v3 clients add themselves.
    V4,
}

/// The Headway default style.
pub fn style(version: Version) -> Style {
    let mut style = Style::new("Headway Default")
        .id("omt-headway")
        .sprite("/sprite/sprites")
        .glyphs("/font/{fontstack}/{range}")
        .metadata("openmaptiles:version", "3.x");

    let openmaptiles = {
        let omt_source = style.source(
            "openmaptiles",
            Source::vector("/areamap-mlt").encoding(Encoding::Mlt),
        );
        OpenMapTiles::new(omt_source)
    };
    let landcover_source = style.source("landcover", Source::vector("/landcover").maxzoom(8.0));
    let terrain_source = style.source("terrain", Source::raster("/terrain").maxzoom(6.0));

    style.background("background").color(land());
    style
        .raster(&terrain_source, "terrain")
        .maxzoom(6.0)
        .opacity(zoom!(5 => 0.8, 6 => 0.0));

    let building_color = rgb!(226, 231, 215);
    style
        .fill(&landcover_source.layer("landcover"), "landcover")
        .fade_out(7.0)
        .color(expr!([
            "match",
            ["get", "class"],
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
        ]));
    style
        .fill(openmaptiles.park(), "park")
        .color(rgb!(140, 220, 103))
        .outline_color(rgb!(95, 208, 100))
        .opacity(0.7);
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
        .filter(Grade::Tunnel.filter())
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
                .color(minor_color())
                .width(zoom_with_base(MINOR_WIDTH_BASE, MINOR_WIDTH))
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
        .filter(Grade::Tunnel.filter().and(foot_path()))
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

    let path_width = zoom!(base: 1.2; 13.0 => 0.75, 20.0 => 10.0);
    style
        .line(openmaptiles.transportation(), "road_sidewalk")
        .minzoom(13.0)
        .filter(
            Grade::Surface
                .filter()
                .and(foot_path())
                .and_eq("footway", "sidewalk"),
        )
        .rounded()
        .color(land().mix(&hsl!(0, 0%, 100%), 0.4))
        .width(path_width.clone());
    // Roundabouts are drawn as circles around their centers, since their rings are too coarse
    // to look round: a disk of road beneath the streets pouring into it, then the island on top.
    let curb_color = minor_color().map(|color| color.darken(0.12));
    let roundabout = |style: &mut Style, id: &str, color: Property<Color>, radius: Expression| {
        style
            .circle(openmaptiles.transportation(), id)
            .minzoom(12.0)
            .filter(Filter::point().class("roundabout_center"))
            .flat()
            .color(color)
            .radius(radius)
            .stroke_width(1.0)
            .stroke_color(curb_color.clone());
    };
    roads.add_layers_with(&mut style, Grade::Surface, |style| {
        let radius = roundabout_radius(0.5);
        roundabout(style, "road_roundabout", minor_color().into(), radius);
    });
    let radius = roundabout_radius(-0.5);
    roundabout(&mut style, "road_roundabout_island", land().into(), radius);
    style
        .line(openmaptiles.transportation(), "road_path_pedestrian")
        .minzoom(13.0)
        .filter(
            Grade::Surface
                .filter()
                .and(foot_path())
                .and_not_in("footway", ["sidewalk", "crossing"]),
        )
        .join_round()
        .color(hsl!(0, 0%, 96%))
        .dasharray(vec![1.0, 0.7])
        .width(path_width);
    // Stripes across the walk, parallel to the street.
    style
        .line(openmaptiles.transportation(), "road_crosswalk")
        .minzoom(13.0)
        .filter(
            Grade::Surface
                .filter()
                .and(foot_path())
                .and_eq("footway", "crossing"),
        )
        .color(hsl!(0, 0%, 100%))
        .dasharray(vec![0.2, 0.2])
        .width(zoom!(base: 1.2; 13.0 => 1.0, 20.0 => 16.0));

    roads.add_layers(&mut style, Grade::Bridge);
    style
        .line(
            openmaptiles.transportation(),
            "bridge_path_pedestrian_casing",
        )
        .minzoom(13.0)
        .filter(Grade::Bridge.filter().and(foot_path()))
        .color(zoom![14.0 => hsl!(0, 0%, 88%), 20.0 => hsl!(0, 0%, 64%)])
        .dasharray(vec![1.0, 0.0])
        .width(zoom!(base: 1.2; 14.0 => 1.125, 20.0 => 18.0));
    style
        .line(openmaptiles.transportation(), "bridge_path_pedestrian")
        .minzoom(13.0)
        .filter(Grade::Bridge.filter().and(foot_path()))
        .color(hsl!(0, 0%, 96%))
        .dasharray(vec![1.0, 0.3])
        .width(zoom!(base: 1.2; 13.0 => 0.75, 20.0 => 10.0));

    // v4 draws every building in 3D instead.
    if version == Version::V3 {
        style
            .fill(openmaptiles.building(), "building")
            .color(building_color)
            .fade_in(14.0)
            .outline_color(zoom!(14 => hsla!(40, 6%, 79%, 0.32), 15 => hsl!(40, 6%, 79%)));
    }
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
        .filter(Filter::eq("oneway", 1).and_ne("junction", "roundabout"))
        .icon_image("arrow")
        .placement(Placement::Line);
    style
        .symbol(openmaptiles.transportation(), "road_one_way_arrow_opposite")
        .minzoom(15.0)
        .filter(Filter::eq("oneway", -1))
        .icon_image("arrow")
        .placement(Placement::Line)
        .icon_rotate(180.0);
    style
        .line(openmaptiles.transportation(), "route_ferry")
        .filter(Filter::linestring().class("ferry"))
        .rounded()
        .color(hex!(0x9999dd))
        .width(2.0)
        .fade_in(12.0)
        .dasharray(vec![0.3, 2.0]);

    if version == Version::V4 {
        style.light(Light { intensity: 0.3 });
        // Dampened so the 3D effect stays subtle.
        let extrude = |height: &str| expr!(["*", ["get", height], 0.8]);
        style
            .fill_extrusion(openmaptiles.building(), "building_3d")
            .filter(Filter::ne("hide_3d", true))
            .minzoom(14.0)
            .opacity(zoom!(14 => 0.0, 14.5 => 0.8))
            .color(rgb!(215, 222, 204))
            .vertical_gradient(false)
            .height(extrude("render_height"))
            .base(extrude("render_min_height"))
            .rounded_corner_distance(3.0);
    }

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
            .placement(Placement::Line);
        water_label
            .symbol(openmaptiles.water_name(), "water_name_lake_line")
            .filter(Filter::linestring())
            .placement(Placement::Line);
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
                .text_anchor(Anchor::Top)
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
            .text_anchor(Anchor::Left)
            .text_offset([0.9, 0.0])
            .text_color(hex!(0x4898ff));
    }

    style
        .symbol(openmaptiles.transportation_name(), "road_label")
        .placement(Placement::Line)
        .text_field("{name}")
        .text_font(REGULAR)
        .text_offset([0.0, 0.15])
        .text_size(zoom![13.0 => 12.0, 14.0 => 13.0])
        .text_color(hex!(0x776655))
        .text_halo(hex!(0xffffff), 1.0)
        .text_halo_blur(0.5);
    RoadShields::new().add_layer(&mut style, openmaptiles.transportation_name());

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
        .text_transform(Transform::Uppercase)
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
        .text_anchor(Anchor::Bottom)
        .text_font(REGULAR)
        .text_max_width(8.0)
        .text_size(town_size)
        .text_color(hex!(0x333333));
    let city_size = zoom!(base: 1.2; 7 => 14.0, 11 => 24.0);
    place_label
        .symbol(openmaptiles.place(), "place_city")
        .filter(Filter::class("city"))
        .icon_image(zoom!(0 => "dot_9", 8 => ""))
        .text_anchor(Anchor::Bottom)
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
        .text_transform(Transform::Uppercase)
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
        .text_transform(Transform::Uppercase)
        .text_color(hex!(0x663333))
        .text_halo(rgba!(255, 255, 255, 0.7), 1.0);

    style.validate();
    style
}
