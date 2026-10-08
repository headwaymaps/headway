//! Headway v3 transportation layer declarations.

use crate::*;
use serde_json::json;

/// V3 transportation rules, expressed through MLSS builders.
///
/// The v3 source deliberately gives each road class its own layer: that keeps
/// color, casing, zoom, and draw-order differences explicit.
pub(super) fn add_v3_roads(style: &mut Style, transportation: &SourceLayer) {
    style
        .line(transportation, "tunnel_service_track_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["in", "class", "service", "track"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[15,"hsl(0, 0%, 94%)"],[18,"hsl(0, 0%, 87%)"],[20,"hsl(0, 0%, 82%)"]]}),
        )
        .paint("line-dasharray", json!([0.5, 0.25]))
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[13,0.76],[16,4],[20,11]]}),
        );
    style.line(transportation, "tunnel_link_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!(["all",["==","ramp",1],["==","brunnel","tunnel"]])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[10,"hsl(0, 0%, 93%)"],[12,"hsl(0, 0%, 93%)"],[15,"hsl(0, 0%, 89%)"],[20,"hsl(0, 0%, 82%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,2.12],[14,4],[20,23.5]]}))
        ;
    style.line(transportation, "tunnel_street_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!(["all",["==","brunnel","tunnel"],["in","class","street","street_limited"]])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[13,"hsl(0, 0%, 93%)"],[16,"hsl(0, 0%, 87.5%)"],[20,"hsl(0, 0%, 82%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,2.12],[14,4],[20,20]]}))
        ;
    style.line(transportation, "tunnel_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!(["all",["==","brunnel","tunnel"],["in","class","secondary","tertiary"]])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[10,"hsl(0, 0%, 91%)"],[11,"hsl(0, 0%, 91%)"],[12,"hsl(0, 0%, 91%)"],[13,"hsl(0, 0%, 89%)"],[20,"hsl(0, 0%, 82%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,1.76],[13,3.6],[20,30]]}))
        ;
    style
        .line(transportation, "tunnel_primary_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["==", "class", "primary"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[13,"hsl(0, 0%, 83.5%)"],[20,"hsl(0, 0%, 76%)"]]}),
        )
        .paint("line-width", json!({"base":1.2,"stops":[[13,5.9],[20,36]]}));
    style
        .line(transportation, "tunnel_trunk_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["==", "class", "trunk"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[13,"hsl(0, 0%, 81%)"],[20,"hsl(0, 0%, 74.5%)"]]}),
        )
        .paint("line-width", json!({"base":1.2,"stops":[[13,5.9],[20,36]]}));
    style
        .line(transportation, "tunnel_motorway_link_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "motorway"],
            ["==", "ramp", 1],
            ["==", "brunnel", "tunnel"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[12,"hsl(0, 0%, 82%)"],[15,"hsl(0, 0%, 77%)"],[20,"hsl(0, 0%, 70%)"]]}),
        )
        .paint("line-dasharray", json!([0.5, 0.25]))
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[8,1.74],[13,5.9],[20,35]]}),
        );
    style
        .line(transportation, "tunnel_motorway_casing")
        .minzoom(10.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "motorway"],
            ["!=", "ramp", 1],
            ["==", "brunnel", "tunnel"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[7,"hsl(0, 0%, 80%)"],[13,"hsl(0, 0%, 75%)"],[20,"hsl(0, 0%, 70%)"]]}),
        )
        .paint("line-dasharray", json!([0.5, 0.25]))
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[5,1.4],[7,1.75],[13,5.9],[20,40]]}),
        );
    style
        .line(transportation, "tunnel_service_track")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["in", "class", "service", "track"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 97%)"],[13,"hsl(0, 0%, 92%)"],[20,"hsl(0, 0%, 87%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[13,0],[16,2],[20,7.5]]}),
        );
    style
        .line(transportation, "tunnel_link")
        .minzoom(12.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "ramp", 1],
            ["==", "brunnel", "tunnel"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 94%)"],[12,"hsl(0, 0%, 94%)"],[20,"hsl(0, 0%, 87%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[12,1],[14,2.5],[20,20]]}),
        );
    style
        .line(transportation, "tunnel_minor")
        .minzoom(12.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["in", "class", "minor"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 93%)"],[13,"hsl(0, 0%, 93%)"],[20,"hsl(0, 0%, 87%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[12,1],[14,2.5],[20,18]]}),
        );
    style
        .line(transportation, "tunnel_path_pedestrian")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "$type", "LineString"],
            ["==", "brunnel", "tunnel"],
            ["in", "class", "path", "pedestrian"]
        ])))
        .paint("line-color", json!("hsl(0, 0%, 98%)"))
        .paint("line-dasharray", json!([1, 0.75]))
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[13,0.75],[20,10]]}),
        );
    style.line(transportation, "tunnel_secondary_tertiary")
        .minzoom(12.0)
        .filter(Filter::raw(json!(["all",["==","brunnel","tunnel"],["in","class","secondary","tertiary"]])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[10,"hsl(0, 0%, 95%)"],[11,"hsl(0, 0%, 95%)"],[12,"hsl(0, 0%, 95%)"],[20,"hsl(0, 0%, 87%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,1],[13,2.5],[20,26]]}))
        ;
    style
        .line(transportation, "tunnel_primary")
        .minzoom(8.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["==", "class", "primary"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 64%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[8,1],[13,4.5],[20,32]]}),
        );
    style
        .line(transportation, "tunnel_trunk")
        .minzoom(6.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["==", "class", "trunk"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 61%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[6,1],[10,2.85],[13,4.5],[20,32]]}),
        );
    style
        .line(transportation, "tunnel_motorway_link")
        .minzoom(8.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "motorway"],
            ["==", "ramp", 1],
            ["==", "brunnel", "tunnel"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 84%)"],[20,"hsl(0, 0%, 76%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[8,1],[13,4.5],[20,32]]}),
        );
    style
        .line(transportation, "tunnel_motorway")
        .minzoom(5.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "motorway"],
            ["!=", "ramp", 1],
            ["==", "brunnel", "tunnel"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 84%)"],[20,"hsl(0, 0%, 76%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[5,1],[7,1],[13,4.5],[20,36]]}),
        );
    style
        .line(transportation, "tunnel_major_rail")
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["in", "class", "rail"]
        ])))
        .layout("line-cap", json!("round"))
        .paint("line-color", json!("hsl(0, 0%, 86.6667%)"))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14,0.4],[15,0.75],[20,2]]}),
        );
    style
        .line(transportation, "tunnel_major_rail_hatching")
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["==", "class", "rail"]
        ])))
        .paint("line-color", json!("hsl(0, 0%, 86.6667%)"))
        .paint("line-dasharray", json!([0.2, 8]))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14.5,0],[15,3],[20,8]]}),
        );
    style
        .line(transportation, "tunnel_transit_rail")
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["in", "class", "transit"]
        ])))
        .layout("line-cap", json!("round"))
        .paint("line-color", json!("hsl(0, 0%, 86.6667%)"))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14,0.4],[15,0.75],[20,2]]}),
        );
    style
        .line(transportation, "tunnel_transit_rail_hatching")
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "tunnel"],
            ["==", "class", "transit"]
        ])))
        .paint("line-color", json!("hsl(0, 0%, 86.6667%)"))
        .paint("line-dasharray", json!([0.2, 8]))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14.5,0],[15,3],[20,8]]}),
        );
    style
        .fill(transportation, "road_area_pattern")
        .filter(Filter::raw(json!(["==", "$type", "Polygon"])))
        .paint("fill-pattern", json!("pedestrian_polygon"));
    style
        .line(transportation, "road_service_track_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["in", "class", "service", "track"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[15,"hsl(0, 0%, 88%)"],[18,"hsl(0, 0%, 74%)"],[20,"hsl(0, 0%, 64%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[13,0.76],[16,4],[20,11]]}),
        );
    style.line(transportation, "road_link_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!(["all",["!in","brunnel","bridge","tunnel"],["!in","class","pedestrian","path","track","service","motorway"],["==","ramp",1]])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[10,"hsl(0, 0%, 86%)"],[12,"hsl(0, 0%, 86%)"],[15,"hsl(0, 0%, 78%)"],[20,"hsl(0, 0%, 64%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,2.12],[14,4],[20,23.5]]}))
        ;
    style
        .line(transportation, "road_minor_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "$type", "LineString"],
            ["!in", "brunnel", "bridge", "tunnel"],
            ["in", "class", "minor"],
            ["!=", "ramp", 1]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[13,"hsl(0, 0%, 86%)"],[16,"hsl(0, 0%, 75%)"],[20,"hsl(0, 0%, 64%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[12,2.12],[14,4],[20,20]]}),
        );
    style.line(transportation, "road_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!(["all",["!in","brunnel","bridge","tunnel"],["in","class","secondary","tertiary"],["!=","ramp",1]])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[10,"hsl(0, 0%, 82%)"],[11,"hsl(0, 0%, 82%)"],[12,"hsl(0, 0%, 82%)"],[13,"hsl(0, 0%, 78%)"],[20,"hsl(0, 0%, 64%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,1.76],[13,3.6],[20,30]]}))
        ;
    style
        .line(transportation, "road_primary_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "primary"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[13,"hsl(0, 0%, 67%)"],[20,"hsl(0, 0%, 52%)"]]}),
        )
        .paint("line-width", json!({"base":1.2,"stops":[[13,5.9],[20,36]]}));
    style
        .line(transportation, "road_trunk_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "trunk"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[13,"hsl(0, 0%, 62%)"],[20,"hsl(0, 0%, 49%)"]]}),
        )
        .paint("line-width", json!({"base":1.2,"stops":[[13,5.9],[20,36]]}));
    style
        .line(transportation, "road_motorway_link_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "motorway"],
            ["==", "ramp", 1]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[12,"hsl(0, 0%, 64%)"],[15,"hsl(0, 0%, 54%)"],[20,"hsl(0, 0%, 40%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[8,1.74],[13,5.9],[20,35]]}),
        );
    style
        .line(transportation, "road_motorway_casing")
        .minzoom(10.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "motorway"],
            ["!=", "ramp", 1]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[7,"hsl(0, 0%, 60%)"],[13,"hsl(0, 0%, 50%)"],[20,"hsl(0, 0%, 40%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[5,1.4],[7,1.75],[13,5.9],[20,40]]}),
        );
    style
        .line(transportation, "road_service_track")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["in", "class", "service", "track"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 94%)"],[13,"hsl(0, 0%, 84%)"],[20,"hsl(0, 0%, 74%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[13,0],[16,2],[20,7.5]]}),
        );
    style
        .line(transportation, "road_link")
        .minzoom(12.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "ramp", 1],
            [
                "!in",
                "class",
                "pedestrian",
                "path",
                "track",
                "service",
                "motorway"
            ]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 88%)"],[12,"hsl(0, 0%, 88%)"],[20,"hsl(0, 0%, 74%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[12,1],[14,2.5],[20,20]]}),
        );
    style
        .line(transportation, "road_minor")
        .minzoom(12.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "$type", "LineString"],
            ["!in", "brunnel", "bridge", "tunnel"],
            ["in", "class", "minor"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 86%)"],[13,"hsl(0, 0%, 86%)"],[20,"hsl(0, 0%, 74%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[12,1],[14,2.5],[20,18]]}),
        );
    style
        .line(transportation, "road_path_pedestrian")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "$type", "LineString"],
            ["!in", "brunnel", "bridge", "tunnel"],
            ["in", "class", "path", "pedestrian"]
        ])))
        .layout("line-join", json!("round"))
        .paint("line-color", json!("hsl(0, 0%, 96%)"))
        .paint("line-dasharray", json!([1, 0.7]))
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[13,0.75],[20,10]]}),
        );
    style.line(transportation, "road_secondary_tertiary")
        .minzoom(12.0)
        .filter(Filter::raw(json!(["all",["!in","brunnel","bridge","tunnel"],["in","class","secondary","tertiary"]])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[10,"hsl(0, 0%, 90%)"],[11,"hsl(0, 0%, 90%)"],[12,"hsl(0, 0%, 90%)"],[20,"hsl(0, 0%, 74%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,1],[13,2.5],[20,26]]}))
        ;
    style
        .line(transportation, "road_primary")
        .minzoom(8.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "primary"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 64%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[8,1],[13,4.5],[20,32]]}),
        );
    style
        .line(transportation, "road_trunk")
        .minzoom(6.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "trunk"]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 61%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[6,1],[10,2.85],[13,4.5],[20,32]]}),
        );
    style
        .line(transportation, "road_motorway_link")
        .minzoom(8.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "motorway"],
            ["==", "ramp", 1]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 52%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[8,1],[13,4.5],[20,32]]}),
        );
    style
        .line(transportation, "road_motorway")
        .minzoom(5.0)
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "motorway"],
            ["!=", "ramp", 1]
        ])))
        .layout("line-cap", json!("round"))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 52%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[5,1],[7,1],[13,4.5],[20,36]]}),
        );
    style
        .line(transportation, "road_major_rail")
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "rail"]
        ])))
        .layout("line-cap", json!("round"))
        .paint("line-color", json!("#bbb"))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14,0.4],[15,0.75],[20,2]]}),
        );
    style
        .line(transportation, "road_major_rail_hatching")
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "rail"]
        ])))
        .paint("line-color", json!("#bbb"))
        .paint("line-dasharray", json!([0.2, 8]))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14.5,0],[15,3],[20,8]]}),
        );
    style
        .line(transportation, "road_transit_rail")
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "transit"]
        ])))
        .layout("line-cap", json!("round"))
        .paint("line-color", json!("#bbb"))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14,0.4],[15,0.75],[20,2]]}),
        );
    style
        .line(transportation, "road_transit_rail_hatching")
        .filter(Filter::raw(json!([
            "all",
            ["!in", "brunnel", "bridge", "tunnel"],
            ["==", "class", "transit"]
        ])))
        .paint("line-color", json!("#bbb"))
        .paint("line-dasharray", json!([0.2, 8]))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14.5,0],[15,3],[20,8]]}),
        );
    style
        .line(transportation, "bridge_service_track_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "bridge"],
            ["in", "class", "service", "track"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[15,"hsl(0, 0%, 88%)"],[18,"hsl(0, 0%, 74%)"],[20,"hsl(0, 0%, 64%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[13,0.76],[16,4],[20,11]]}),
        );
    style.line(transportation, "bridge_link_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!(["all",["==","class","link"],["==","brunnel","bridge"]])))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[10,"hsl(0, 0%, 86%)"],[12,"hsl(0, 0%, 86%)"],[15,"hsl(0, 0%, 78%)"],[20,"hsl(0, 0%, 64%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,2.12],[14,4],[20,23.5]]}))
        ;
    style
        .line(transportation, "bridge_street_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "bridge"],
            ["in", "class", "street", "street_limited"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[13,"hsl(0, 0%, 86%)"],[16,"hsl(0, 0%, 75%)"],[20,"hsl(0, 0%, 64%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[12,2.12],[14,4],[20,20]]}),
        );
    style
        .line(transportation, "bridge_path_pedestrian_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "$type", "LineString"],
            ["==", "brunnel", "bridge"],
            ["in", "class", "path", "pedestrian"]
        ])))
        .paint(
            "line-color",
            json!({"stops":[[14,"hsl(0, 0%, 88%)"],[20,"hsl(0, 0%, 64%)"]]}),
        )
        .paint("line-dasharray", json!([1, 0]))
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[14,1.125],[20,18]]}),
        );
    style.line(transportation, "bridge_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!(["all",["==","brunnel","bridge"],["in","class","secondary","tertiary"]])))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[10,"hsl(0, 0%, 82%)"],[11,"hsl(0, 0%, 82%)"],[12,"hsl(0, 0%, 82%)"],[13,"hsl(0, 0%, 78%)"],[20,"hsl(0, 0%, 64%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,1.76],[13,3.6],[20,30]]}))
        ;
    style
        .line(transportation, "bridge_primary_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "bridge"],
            ["==", "class", "primary"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[13,"hsl(0, 0%, 67%)"],[20,"hsl(0, 0%, 52%)"]]}),
        )
        .paint("line-width", json!({"base":1.2,"stops":[[13,5.9],[20,36]]}));
    style
        .line(transportation, "bridge_trunk_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "bridge"],
            ["==", "class", "trunk"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[13,"hsl(0, 0%, 62%)"],[20,"hsl(0, 0%, 49%)"]]}),
        )
        .paint("line-width", json!({"base":1.2,"stops":[[13,5.9],[20,36]]}));
    style
        .line(transportation, "bridge_motorway_link_casing")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "motorway"],
            ["==", "ramp", 1],
            ["==", "brunnel", "bridge"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[12,"hsl(0, 0%, 64%)"],[15,"hsl(0, 0%, 54%)"],[20,"hsl(0, 0%, 40%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[8,1.74],[13,5.9],[20,35]]}),
        );
    style
        .line(transportation, "bridge_motorway_casing")
        .minzoom(10.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "motorway"],
            ["!=", "ramp", 1],
            ["==", "brunnel", "bridge"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[7,"hsl(0, 0%, 60%)"],[13,"hsl(0, 0%, 50%)"],[20,"hsl(0, 0%, 40%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[5,1.4],[7,1.75],[13,5.9],[20,40]]}),
        );
    style
        .line(transportation, "bridge_service_track")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "bridge"],
            ["in", "class", "service", "track"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 94%)"],[13,"hsl(0, 0%, 84%)"],[20,"hsl(0, 0%, 74%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[13,0],[16,2],[20,7.5]]}),
        );
    style
        .line(transportation, "bridge_link")
        .minzoom(12.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "link"],
            ["==", "brunnel", "bridge"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 88%)"],[12,"hsl(0, 0%, 88%)"],[20,"hsl(0, 0%, 74%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[12,1],[14,2.5],[20,20]]}),
        );
    style
        .line(transportation, "bridge_street")
        .minzoom(12.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "bridge"],
            ["in", "class", "minor"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 86%)"],[13,"hsl(0, 0%, 86%)"],[20,"hsl(0, 0%, 74%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[12,1],[14,2.5],[20,18]]}),
        );
    style
        .line(transportation, "bridge_path_pedestrian")
        .minzoom(13.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "$type", "LineString"],
            ["==", "brunnel", "bridge"],
            ["in", "class", "path", "pedestrian"]
        ])))
        .paint("line-color", json!("hsl(0, 0%, 96%)"))
        .paint("line-dasharray", json!([1, 0.3]))
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[13,0.75],[20,10]]}),
        );
    style.line(transportation, "bridge_secondary_tertiary")
        .minzoom(12.0)
        .filter(Filter::raw(json!(["all",["==","brunnel","bridge"],["in","class","secondary","tertiary"]])))
        .layout("line-join", json!("round"))
        .paint("line-color", json!({"stops":[[10,"hsl(0, 0%, 90%)"],[11,"hsl(0, 0%, 90%)"],[12,"hsl(0, 0%, 90%)"],[20,"hsl(0, 0%, 74%)"]]}))
        .paint("line-width", json!({"base":1.2,"stops":[[12,1],[13,2.5],[20,26]]}))
        ;
    style
        .line(transportation, "bridge_primary")
        .minzoom(8.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "bridge"],
            ["==", "class", "primary"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 64%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[8,1],[13,4.5],[20,32]]}),
        );
    style
        .line(transportation, "bridge_trunk")
        .minzoom(6.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "brunnel", "bridge"],
            ["==", "class", "trunk"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 61%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[6,1],[10,2.85],[13,4.5],[20,32]]}),
        );
    style
        .line(transportation, "bridge_motorway_link")
        .minzoom(8.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "motorway"],
            ["==", "ramp", 1],
            ["==", "brunnel", "bridge"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 52%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[8,1],[13,4.5],[20,32]]}),
        );
    style
        .line(transportation, "bridge_motorway")
        .minzoom(5.0)
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "motorway"],
            ["!=", "ramp", 1],
            ["==", "brunnel", "bridge"]
        ])))
        .layout("line-join", json!("round"))
        .paint(
            "line-color",
            json!({"stops":[[10,"hsl(0, 0%, 68%)"],[20,"hsl(0, 0%, 52%)"]]}),
        )
        .paint(
            "line-width",
            json!({"base":1.2,"stops":[[5,1],[7,1],[13,4.5],[20,36]]}),
        );
    style
        .line(transportation, "bridge_major_rail")
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "rail"],
            ["==", "brunnel", "bridge"]
        ])))
        .paint("line-color", json!("#bbb"))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14,0.4],[15,0.75],[20,2]]}),
        );
    style
        .line(transportation, "bridge_major_rail_hatching")
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "rail"],
            ["==", "brunnel", "bridge"]
        ])))
        .paint("line-color", json!("#bbb"))
        .paint("line-dasharray", json!([0.2, 8]))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14.5,0],[15,3],[20,8]]}),
        );
    style
        .line(transportation, "bridge_transit_rail")
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "transit"],
            ["==", "brunnel", "bridge"]
        ])))
        .paint("line-color", json!("#bbb"))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14,0.4],[15,0.75],[20,2]]}),
        );
    style
        .line(transportation, "bridge_transit_rail_hatching")
        .filter(Filter::raw(json!([
            "all",
            ["==", "class", "transit"],
            ["==", "brunnel", "bridge"]
        ])))
        .paint("line-color", json!("#bbb"))
        .paint("line-dasharray", json!([0.2, 8]))
        .paint(
            "line-width",
            json!({"base":1.4,"stops":[[14.5,0],[15,3],[20,8]]}),
        );
}
