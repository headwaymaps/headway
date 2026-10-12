//! The Headway default style, built with the builders in this crate.

use crate::filter::Geometry;
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

/// The per-network exceptions to Headway's road-shield defaults.
#[derive(Clone, Copy)]
struct RoadShield {
    network: &'static str,
    text: RoadShieldText,
    text_offset: Option<[f64; 2]>,
    text_size: Option<f64>,
    text_width: Option<f64>,
    icon_size: Option<f64>,
    text_color: Option<&'static str>,
    halo_color: Option<&'static str>,
    halo_width: Option<f64>,
}

#[derive(Clone, Copy)]
enum RoadShieldText {
    NoText,
    Route1Ref,
}

impl RoadShield {
    fn new(network: &'static str) -> Self {
        Self {
            network,
            text: RoadShieldText::Route1Ref,
            text_offset: None,
            text_size: Some(9.0),
            text_width: None,
            icon_size: Some(1.0),
            text_color: None,
            halo_color: None,
            halo_width: None,
        }
    }

    fn no_text(mut self) -> Self {
        self.text = RoadShieldText::NoText;
        self
    }
    fn text_offset(mut self, value: [f64; 2]) -> Self {
        self.text_offset = Some(value);
        self
    }
    fn text_size(mut self, value: f64) -> Self {
        self.text_size = Some(value);
        self
    }
    fn text_width(mut self, value: f64) -> Self {
        self.text_width = Some(value);
        self
    }
    fn text_color(mut self, value: &'static str) -> Self {
        self.text_color = Some(value);
        self
    }
    fn white_text_color(self) -> Self {
        self.text_color("#ffffff")
    }
    fn halo_color(mut self, value: &'static str) -> Self {
        self.halo_color = Some(value);
        self
    }
    fn halo_width(mut self, value: f64) -> Self {
        self.halo_width = Some(value);
        self
    }

    fn with_ref(body: Expression) -> Expression {
        Expression::op(
            Operator::Let,
            [
                Expression::literal("ref"),
                expr![Coalesce, [Get, "route_1_ref"], [Get, "ref"]],
                body,
            ],
        )
    }

    fn property(
        shields: &[Self],
        default: Expression,
        value: impl Fn(Self) -> Option<Expression>,
    ) -> Expression {
        let mut operands = vec![expr![Get, "route_1_network"]];
        for shield in shields.iter().copied() {
            if let Some(value) = value(shield) {
                operands.push(Expression::literal(shield.network));
                operands.push(value);
            }
        }
        operands.push(default);
        Expression::op(Operator::Match, operands)
    }

    fn offset([x, y]: [f64; 2]) -> Expression {
        Expression::op(
            Operator::Literal,
            [Expression::array([
                Expression::literal(x),
                Expression::literal(y),
            ])],
        )
    }
}

fn road_shields() -> Vec<RoadShield> {
    vec![
        RoadShield::new("CA:BC").text_color("#003f87"),
        RoadShield::new("CA:NB:tertiary")
            .text_offset([-0.06, 0.11])
            .text_width(23.0),
        RoadShield::new("CA:NS:S"),
        RoadShield::new("CA:NT")
            .text_offset([0.0, 0.41])
            .text_width(25.0)
            .white_text_color(),
        RoadShield::new("CA:ON:Hamilton:Expressway"),
        RoadShield::new("CA:ON:Toronto:Expressway").text_width(15.0),
        RoadShield::new("CA:ON:primary")
            .text_offset([0.0, 0.24])
            .text_width(28.0),
        RoadShield::new("CA:PE").text_width(28.0),
        RoadShield::new("CA:QC:A").white_text_color(),
        RoadShield::new("CA:QC:R")
            .text_width(28.0)
            .white_text_color(),
        RoadShield::new("CA:SK:secondary")
            .text_offset([0.0, 0.51])
            .text_width(27.0)
            .text_color("#006747"),
        RoadShield::new("CA:transcanada").text_color("#006747"),
        RoadShield::new("CA:transcanada:namedRoute").no_text(),
        RoadShield::new("CL:national").white_text_color(),
        RoadShield::new("CN:expressway")
            .text_offset([0.0, 0.24])
            .white_text_color(),
        RoadShield::new("EC:secundaria").white_text_color(),
        RoadShield::new("GLCT"),
        RoadShield::new("GLCT:Loop"),
        RoadShield::new("ID:national"),
        RoadShield::new("IN:NH"),
        RoadShield::new("KR:expressway")
            .text_offset([0.0, 0.24])
            .white_text_color(),
        RoadShield::new("MX:CDMX:EJE:CENTRAL").no_text(),
        RoadShield::new("MX:CDMX:EJE:NTE")
            .text_offset([0.0, 0.36])
            .text_width(22.5),
        RoadShield::new("MX:CDMX:EJE:OTE").text_offset([-0.36, 0.0]),
        RoadShield::new("MX:CDMX:EJE:PTE").text_offset([0.36, 0.0]),
        RoadShield::new("MX:CDMX:EJE:SUR")
            .text_offset([0.0, -0.36])
            .text_width(22.5),
        RoadShield::new("MX:MX"),
        RoadShield::new("NL:DR:Hunebed_Highway")
            .text_offset([0.0, 0.24])
            .white_text_color(),
        RoadShield::new("NZ:Touring:AH"),
        RoadShield::new("NZ:Touring:CNZWT"),
        RoadShield::new("NZ:Touring:MWT"),
        RoadShield::new("NZ:Touring:PCH"),
        RoadShield::new("NZ:Touring:SLH"),
        RoadShield::new("NZ:Touring:SSR"),
        RoadShield::new("NZ:Touring:TCDH"),
        RoadShield::new("NZ:Touring:TEH").text_width(27.0),
        RoadShield::new("NZ:Touring:VLH"),
        RoadShield::new("NZ:WRR"),
        RoadShield::new("PE:national").text_offset([0.0, 0.18]),
        RoadShield::new("PK:motorway").white_text_color(),
        RoadShield::new("TW:freeway").text_width(18.5),
        RoadShield::new("US:AK")
            .text_offset([0.0, -0.45])
            .text_width(24.5),
        RoadShield::new("US:AL").text_offset([0.0, -0.18]),
        RoadShield::new("US:AL:Baldwin:Foley_Beach_Express"),
        RoadShield::new("US:AR"),
        RoadShield::new("US:AS")
            .text_offset([0.0, 0.51])
            .white_text_color(),
        RoadShield::new("US:AZ"),
        RoadShield::new("US:AZ:Indian")
            .text_offset([0.0, -0.01])
            .text_width(24.0),
        RoadShield::new("US:AZ:Scenic"),
        RoadShield::new("US:BIA")
            .text_offset([0.0, -0.01])
            .text_width(22.5),
        RoadShield::new("US:CA")
            .text_offset([0.0, 0.18])
            .text_width(29.0)
            .white_text_color(),
        RoadShield::new("US:CA:San_Francisco:49_Mile_Scenic_Drive"),
        RoadShield::new("US:CKC"),
        RoadShield::new("US:CO").text_offset([0.0, 0.51]),
        RoadShield::new("US:CO:E470").text_offset([0.0, 0.51]),
        RoadShield::new("US:CO:NW")
            .text_offset([0.0, 0.6])
            .text_width(23.0)
            .text_color("#003f87"),
        RoadShield::new("US:CO:Scenic"),
        RoadShield::new("US:CT:Parkway"),
        RoadShield::new("US:DC")
            .text_offset([0.0, 0.29])
            .text_width(24.0)
            .halo_color("#ffffff")
            .halo_width(1.0),
        RoadShield::new("US:FL").text_offset([-0.15, 0.12]),
        RoadShield::new("US:FL:Toll")
            .text_offset([-0.18, 0.18])
            .text_width(21.5),
        RoadShield::new("US:FL:Turnpike").no_text(),
        RoadShield::new("US:GA"),
        RoadShield::new("US:GLST").no_text(),
        RoadShield::new("US:GRR").no_text(),
        RoadShield::new("US:GU").text_width(24.5).white_text_color(),
        RoadShield::new("US:I")
            .text_offset([0.0, 0.06])
            .text_size(9.5)
            .text_width(32.0)
            .white_text_color(),
        RoadShield::new("US:I:Business:Loop")
            .text_offset([0.0, 0.06])
            .text_size(9.5)
            .text_width(32.0)
            .white_text_color(),
        RoadShield::new("US:ID")
            .text_offset([0.24, -0.45])
            .white_text_color(),
        RoadShield::new("US:IL:Cook:Chicago:Skyway").no_text(),
        RoadShield::new("US:IN:JHMHT"),
        RoadShield::new("US:IN:Toll").no_text(),
        RoadShield::new("US:KS"),
        RoadShield::new("US:KS:Turnpike").no_text(),
        RoadShield::new("US:KY:Parkway")
            .text_offset([0.0, -0.24])
            .text_color("#003f87"),
        RoadShield::new("US:LA").text_offset([0.0, 0.31]),
        RoadShield::new("US:LA:Causeway"),
        RoadShield::new("US:LHT"),
        RoadShield::new("US:MA:Turnpike").no_text(),
        RoadShield::new("US:ME:Turnpike"),
        RoadShield::new("US:MN")
            .text_offset([0.0, 0.24])
            .white_text_color(),
        RoadShield::new("US:MN:Business")
            .text_offset([0.0, 0.24])
            .white_text_color(),
        RoadShield::new("US:MO").text_offset([0.0, -0.18]),
        RoadShield::new("US:MP"),
        RoadShield::new("US:MT:secondary")
            .text_offset([0.0, -0.12])
            .text_width(21.0),
        RoadShield::new("US:ND").text_offset([-0.18, 0.0]),
        RoadShield::new("US:NE:Scenic"),
        RoadShield::new("US:NH").text_offset([0.12, 0.0]),
        RoadShield::new("US:NH:Turnpike"),
        RoadShield::new("US:NHT"),
        RoadShield::new("US:NJ:ACE"),
        RoadShield::new("US:NJ:GSP"),
        RoadShield::new("US:NJ:NJTP"),
        RoadShield::new("US:NM"),
        RoadShield::new("US:NM:Frontage")
            .text_offset([0.0, -0.12])
            .text_width(29.0),
        RoadShield::new("US:NPS:Blue_Ridge").no_text(),
        RoadShield::new("US:NPS:Natchez_Trace").no_text(),
        RoadShield::new("US:NV")
            .text_offset([0.0, -0.64])
            .text_width(21.5),
        RoadShield::new("US:NV:Clark")
            .text_offset([0.0, -0.12])
            .text_width(27.0)
            .text_color("#003f87"),
        RoadShield::new("US:NY:Parkway:LI")
            .text_offset([0.24, -0.36])
            .text_width(21.5),
        RoadShield::new("US:NY:Parkway:LOSP").no_text(),
        RoadShield::new("US:NY:Parkway:NYC").text_offset([0.0, -0.36]),
        RoadShield::new("US:NY:STE").no_text(),
        RoadShield::new("US:NY:Scenic"),
        RoadShield::new("US:NY:TBTA"),
        RoadShield::new("US:NY:Thruway").no_text(),
        RoadShield::new("US:Navajo"),
        RoadShield::new("US:OH").text_offset([0.0, -0.12]),
        RoadShield::new("US:OH:ASD")
            .text_width(25.0)
            .text_color("#006747"),
        RoadShield::new("US:OH:HOL").white_text_color(),
        RoadShield::new("US:OH:JHMHT"),
        RoadShield::new("US:OH:OEC"),
        RoadShield::new("US:OH:SCI").text_offset([0.0, -0.12]),
        RoadShield::new("US:OH:TUS:Salem").text_offset([-0.12, -0.18]),
        RoadShield::new("US:OH:Turnpike"),
        RoadShield::new("US:OK").text_offset([0.0, 0.24]),
        RoadShield::new("US:OK:Turnpike"),
        RoadShield::new("US:OR").text_offset([0.0, -0.12]),
        RoadShield::new("US:ORSB"),
        RoadShield::new("US:PA"),
        RoadShield::new("US:PA:Turnpike").white_text_color(),
        RoadShield::new("US:PANYNJ"),
        RoadShield::new("US:PIPC"),
        RoadShield::new("US:SC")
            .text_offset([0.0, 0.18])
            .text_color("#003f87"),
        RoadShield::new("US:SD").text_offset([0.0, -0.12]),
        RoadShield::new("US:SD:Custer:CSP").text_offset([0.0, -0.18]),
        RoadShield::new("US:TN:primary").text_offset([0.0, -0.3]),
        RoadShield::new("US:TX:Fort_Bend:FBCTRA")
            .text_offset([0.0, -0.36])
            .text_width(25.0)
            .white_text_color(),
        RoadShield::new("US:US"),
        RoadShield::new("US:UT"),
        RoadShield::new("US:VT")
            .text_offset([0.0, 0.18])
            .text_color("#006747"),
        RoadShield::new("US:WA")
            .text_offset([-0.06, -0.01])
            .text_width(22.0),
        RoadShield::new("US:WI").text_offset([0.0, -0.18]),
        RoadShield::new("US:WI:Rustic")
            .text_offset([-0.15, 0.3])
            .text_color("#ffcd00"),
        RoadShield::new("co:national").text_offset([0.0, -0.24]),
    ]
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
        .color(expr![
            Match,
            [Get, "class"],
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
        ]);

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

    {
        // The shared typography for waterway and water-body labels.
        let mut water_label = style
            .symbol_factory()
            .text_field("{name}")
            .text_font([REGULAR])
            .text_max_width(5.0)
            .text_size(12.0)
            .text_color("#5d60be")
            .text_halo("rgba(255,255,255,0.7)", 1.0);
        water_label
            .symbol(&waterway, "water_name_line")
            .filter(Filter::geometry(Geometry::LineString))
            .placement("line");
        let water_name = openmaptiles.water_name();
        water_label
            .symbol(&water_name, "water_name_lake_line")
            .filter(Filter::geometry(Geometry::LineString))
            .placement("line");
        water_label
            .symbol(&water_name, "water_name_point")
            .filter(Filter::geometry(Geometry::Point));
    }

    {
        let mut poi_label = style
            .symbol_factory()
            .icon_image("{class}_11")
            .text_font([ITALIC])
            .text_max_width(9.0)
            .text_size(12.0)
            .text_halo("#ffffff", 1.0)
            .text_halo_blur(0.5);
        for (id, zoom, rank) in [
            ("poi_z16", 16.0, Filter::gte("rank", 20)),
            ("poi_z15", 15.0, Filter::gte("rank", 7).and_lt("rank", 20)),
            ("poi_z14", 14.0, Filter::gte("rank", 1).and_lt("rank", 7)),
        ] {
            poi_label
                .symbol(&poi, id)
                .minzoom(zoom)
                .filter(Filter::geometry(Geometry::Point).and(rank))
                .text_field("{name}")
                .text_anchor("top")
                .text_offset([0.0, 0.6])
                .text_color("#666");
        }
        poi_label
            .symbol(&poi, "poi_transit")
            .filter(Filter::is_in("class", ["bus", "rail", "airport"]))
            .text_field("{name_en}")
            .text_anchor("left")
            .text_offset([0.9, 0.0])
            .text_color("#4898ff");
    }

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
    let shields = road_shields();
    style
        .symbol(&transportation_name, "road_shield")
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
        .layout(
            "icon-image",
            expr![
                Let,
                "ref",
                [Coalesce, [Get, "route_1_ref"], [Get, "ref"]],
                [
                    Let,
                    "shield_base",
                    [
                        Match,
                        [Get, "route_1_network"],
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
                        Case,
                        [NotEqual, [Var, "shield_base"], ""],
                        [
                            Concat,
                            [Var, "shield_base"],
                            [
                                Case,
                                [LessEqual, [Length, [Var, "ref"]], 2],
                                "_2",
                                [
                                    All,
                                    [GreaterEqual, [Length, [Var, "ref"]], 4],
                                    [
                                        Match,
                                        [Get, "route_1_network"],
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
                            Match,
                            [Get, "route_1_network"],
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
                            [Concat, "default_", [ToString, [Length, [Var, "ref"]]]]
                        ]
                    ]
                ]
            ],
        )
        .icon_rotation_alignment("viewport")
        .layout("symbol-placement", stops![10.0 => "point", 11.0 => "line"])
        .spacing(500.0)
        .layout(
            "text-field",
            RoadShield::with_ref(RoadShield::property(
                &shields,
                expr![Var, "ref"],
                |shield| match shield.text {
                    RoadShieldText::NoText => Some(Expression::literal("")),
                    RoadShieldText::Route1Ref => None,
                },
            )),
        )
        .text_font(["Overpass Heavy"])
        .layout(
            "text-offset",
            RoadShield::property(&shields, expr![Literal, [0, 0.11]], |shield| {
                shield.text_offset.map(RoadShield::offset)
            }),
        )
        .text_rotation_alignment("viewport")
        .layout(
            "text-size",
            RoadShield::with_ref(Expression::op(
                Operator::Min,
                [
                    RoadShield::property(&shields, Expression::literal(8.5), |shield| {
                        shield.text_size.map(Expression::literal)
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
            )),
        )
        .layout(
            "icon-size",
            RoadShield::property(&shields, Expression::literal(0.667), |shield| {
                shield.icon_size.map(Expression::literal)
            }),
        )
        .paint(
            "text-color",
            RoadShield::property(&shields, Expression::literal("#000000"), |shield| {
                shield.text_color.map(Expression::literal)
            }),
        )
        .paint(
            "text-halo-color",
            RoadShield::property(&shields, Expression::literal("rgba(0,0,0,0)"), |shield| {
                shield.halo_color.map(Expression::literal)
            }),
        )
        .paint(
            "text-halo-width",
            RoadShield::property(&shields, Expression::literal(0), |shield| {
                shield.halo_width.map(Expression::literal)
            }),
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
