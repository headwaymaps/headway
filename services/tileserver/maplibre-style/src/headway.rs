//! The Headway default style, built with the builders in this crate.

use crate::*;

fn water() -> Color {
    rgb!(141, 220, 248)
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

fn above_ground() -> Filter {
    Filter::ne("brunnel", "tunnel")
}

/// The per-network exceptions to Headway's road-shield defaults.
#[derive(Clone, Copy)]
struct RoadShield {
    network: &'static str,
    text: RoadShieldText,
    text_offset: Option<[f64; 2]>,
    text_size: f64,
    text_width: Option<f64>,
    icon: RoadShieldIcon,
    text_color: Option<&'static str>,
    halo_color: Option<&'static str>,
    halo_width: Option<f64>,
}

#[derive(Clone, Copy)]
enum RoadShieldText {
    NoText,
    Route1Ref,
}

#[derive(Clone, Copy)]
enum RoadShieldIcon {
    Sized { four_digit: bool },
    Fixed,
    Named(&'static str),
}

impl RoadShield {
    fn new(network: &'static str) -> Self {
        Self {
            network,
            text: RoadShieldText::Route1Ref,
            text_offset: None,
            text_size: 9.0,
            text_width: None,
            icon: RoadShieldIcon::Fixed,
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
        self.text_size = value;
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
    fn sized_icon(mut self) -> Self {
        self.icon = RoadShieldIcon::Sized { four_digit: false };
        self
    }
    fn wide_sized_icon(mut self) -> Self {
        self.icon = RoadShieldIcon::Sized { four_digit: true };
        self
    }
    fn named_icon(mut self, image: &'static str) -> Self {
        self.icon = RoadShieldIcon::Named(image);
        self
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
        let mut groups: Vec<(Expression, Vec<&'static str>)> = Vec::new();
        for shield in shields.iter().copied() {
            let Some(value) = value(shield) else {
                continue;
            };
            match groups.iter_mut().find(|(existing, _)| *existing == value) {
                Some((_, networks)) => networks.push(shield.network),
                None => groups.push((value, vec![shield.network])),
            }
        }

        let mut operands = vec![expr![Get, "route_1_network"]];
        for (value, networks) in groups {
            operands.push(match networks.as_slice() {
                [network] => Expression::literal(*network),
                networks => {
                    Expression::array(networks.iter().map(|network| Expression::literal(*network)))
                }
            });
            operands.push(value);
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

    fn sprite_name(self) -> String {
        let slug = self.network.to_lowercase().replace([':', '_'], "-");
        let Some(suffix) = slug.strip_prefix("us-") else {
            return format!("us-{slug}");
        };
        match suffix {
            "i" => "us-interstate".into(),
            "us" => "us-highway".into(),
            suffix if !suffix.contains('-') => format!("us-state-{suffix}"),
            suffix => format!("us-{suffix}"),
        }
    }

    fn icon_image(shields: &[Self]) -> Expression {
        let sized = Self::property(shields, Expression::literal(""), |shield| {
            match shield.icon {
                RoadShieldIcon::Sized { .. } => Some(Expression::literal(shield.sprite_name())),
                _ => None,
            }
        });
        let four_digit = Self::property(shields, Expression::literal(false), |shield| {
            matches!(
                shield.icon,
                RoadShieldIcon::Sized {
                    four_digit: true,
                    ..
                }
            )
            .then_some(Expression::literal(true))
        });
        let suffix = Expression::op(
            Operator::Case,
            [
                expr![LessEqual, [Length, [Var, "ref"]], 2],
                Expression::literal("_2"),
                Expression::op(
                    Operator::All,
                    [expr![GreaterEqual, [Length, [Var, "ref"]], 4], four_digit],
                ),
                Expression::literal("_4"),
                Expression::literal("_3"),
            ],
        );
        let fixed = Self::property(
            shields,
            expr![Concat, "default_", [ToString, [Length, [Var, "ref"]]]],
            |shield| match shield.icon {
                RoadShieldIcon::Fixed => Some(Expression::literal(shield.sprite_name())),
                RoadShieldIcon::Named(image) => Some(Expression::literal(image)),
                _ => None,
            },
        );
        Self::with_ref(Expression::op(
            Operator::Let,
            [
                Expression::literal("shield_base"),
                sized,
                Expression::op(
                    Operator::Case,
                    [
                        expr![NotEqual, [Var, "shield_base"], ""],
                        Expression::op(Operator::Concat, [expr![Var, "shield_base"], suffix]),
                        fixed,
                    ],
                ),
            ],
        ))
    }
}

fn road_shields() -> Vec<RoadShield> {
    fn shield(network: &'static str) -> RoadShield {
        RoadShield::new(network)
    }
    vec![
        shield("CA:BC").text_color("#1966b3").sized_icon(),
        shield("CA:NB:tertiary")
            .text_offset([-0.06, 0.11])
            .text_width(23.0),
        shield("CA:NS:S"),
        shield("CA:NT")
            .text_offset([0.0, 0.41])
            .text_width(25.0)
            .white_text_color(),
        shield("CA:ON:Hamilton:Expressway"),
        shield("CA:ON:Toronto:Expressway").text_width(15.0),
        shield("CA:ON:primary")
            .text_offset([0.0, 0.24])
            .text_width(28.0),
        shield("CA:PE").text_width(28.0),
        shield("CA:QC:A").white_text_color().sized_icon(),
        shield("CA:QC:R").text_width(28.0).white_text_color(),
        shield("CA:SK:secondary")
            .text_offset([0.0, 0.51])
            .text_width(27.0)
            .text_color("#006747"),
        shield("CA:transcanada").text_color("#006747").sized_icon(),
        shield("CA:transcanada:namedRoute")
            .no_text()
            .named_icon("us-ca-transcanada-namedroute_2"),
        shield("CL:national").white_text_color().sized_icon(),
        shield("CN:expressway")
            .text_offset([0.0, 0.24])
            .white_text_color()
            .wide_sized_icon(),
        shield("EC:secundaria").white_text_color().wide_sized_icon(),
        shield("GLCT"),
        shield("GLCT:Loop"),
        shield("ID:national"),
        shield("IN:NH").wide_sized_icon(),
        shield("KR:expressway")
            .text_offset([0.0, 0.24])
            .white_text_color()
            .sized_icon(),
        shield("MX:CDMX:EJE:CENTRAL").no_text(),
        shield("MX:CDMX:EJE:NTE")
            .text_offset([0.0, 0.36])
            .text_width(22.5),
        shield("MX:CDMX:EJE:OTE").text_offset([-0.36, 0.0]),
        shield("MX:CDMX:EJE:PTE").text_offset([0.36, 0.0]),
        shield("MX:CDMX:EJE:SUR")
            .text_offset([0.0, -0.36])
            .text_width(22.5),
        shield("MX:MX").wide_sized_icon(),
        shield("NL:DR:Hunebed_Highway")
            .text_offset([0.0, 0.24])
            .white_text_color(),
        shield("NZ:Touring:AH"),
        shield("NZ:Touring:CNZWT"),
        shield("NZ:Touring:MWT"),
        shield("NZ:Touring:PCH"),
        shield("NZ:Touring:SLH"),
        shield("NZ:Touring:SSR"),
        shield("NZ:Touring:TCDH"),
        shield("NZ:Touring:TEH").text_width(27.0),
        shield("NZ:Touring:VLH"),
        shield("NZ:WRR"),
        shield("PE:national").text_offset([0.0, 0.18]).sized_icon(),
        shield("PK:motorway").white_text_color(),
        shield("TW:freeway").text_width(18.5),
        shield("US:AK").text_offset([0.0, -0.45]).text_width(24.5),
        shield("US:AL").text_offset([0.0, -0.18]).sized_icon(),
        shield("US:AL:Baldwin:Foley_Beach_Express"),
        shield("US:AR").sized_icon(),
        shield("US:AS").text_offset([0.0, 0.51]).white_text_color(),
        shield("US:AZ").sized_icon(),
        shield("US:AZ:Indian")
            .text_offset([0.0, -0.01])
            .text_width(24.0),
        shield("US:AZ:Scenic"),
        shield("US:BIA").text_offset([0.0, -0.01]).text_width(22.5),
        shield("US:CA")
            .text_offset([0.0, 0.18])
            .text_width(29.0)
            .white_text_color()
            .sized_icon(),
        shield("US:CA:San_Francisco:49_Mile_Scenic_Drive"),
        shield("US:CKC"),
        shield("US:CO").text_offset([0.0, 0.51]),
        shield("US:CO:E470").text_offset([0.0, 0.51]),
        shield("US:CO:NW")
            .text_offset([0.0, 0.6])
            .text_width(23.0)
            .text_color("#1966b3"),
        shield("US:CO:Scenic"),
        shield("US:CT:Parkway"),
        shield("US:DC")
            .text_offset([0.0, 0.29])
            .text_width(24.0)
            .halo_color("#ffffff")
            .halo_width(1.0),
        shield("US:FL").text_offset([-0.15, 0.12]).sized_icon(),
        shield("US:FL:Toll")
            .text_offset([-0.18, 0.18])
            .text_width(21.5),
        shield("US:FL:Turnpike").no_text(),
        shield("US:GA").sized_icon(),
        shield("US:GLST").no_text(),
        shield("US:GRR").no_text(),
        shield("US:GU")
            .text_width(24.5)
            .white_text_color()
            .sized_icon(),
        shield("US:I")
            .text_offset([0.0, 0.06])
            .text_size(9.5)
            .text_width(32.0)
            .white_text_color()
            .sized_icon(),
        shield("US:I:Business:Loop")
            .text_offset([0.0, 0.06])
            .text_size(9.5)
            .text_width(32.0)
            .white_text_color()
            .sized_icon(),
        shield("US:ID")
            .text_offset([0.24, -0.45])
            .white_text_color()
            .sized_icon(),
        shield("US:IL:Cook:Chicago:Skyway").no_text(),
        shield("US:IN:JHMHT"),
        shield("US:IN:Toll").no_text(),
        shield("US:KS").sized_icon(),
        shield("US:KS:Turnpike").no_text(),
        shield("US:KY:Parkway")
            .text_offset([0.0, -0.24])
            .text_color("#1966b3"),
        shield("US:LA").text_offset([0.0, 0.31]).sized_icon(),
        shield("US:LA:Causeway"),
        shield("US:LHT"),
        shield("US:MA:Turnpike").no_text(),
        shield("US:ME:Turnpike"),
        shield("US:MN")
            .text_offset([0.0, 0.24])
            .white_text_color()
            .sized_icon(),
        shield("US:MN:Business")
            .text_offset([0.0, 0.24])
            .white_text_color()
            .sized_icon(),
        shield("US:MO").text_offset([0.0, -0.18]).sized_icon(),
        shield("US:MP").sized_icon(),
        shield("US:MT:secondary")
            .text_offset([0.0, -0.12])
            .text_width(21.0),
        shield("US:ND").text_offset([-0.18, 0.0]).sized_icon(),
        shield("US:NE:Scenic"),
        shield("US:NH").text_offset([0.12, 0.0]).sized_icon(),
        shield("US:NH:Turnpike"),
        shield("US:NHT"),
        shield("US:NJ:ACE"),
        shield("US:NJ:GSP"),
        shield("US:NJ:NJTP"),
        shield("US:NM").sized_icon(),
        shield("US:NM:Frontage")
            .text_offset([0.0, -0.12])
            .text_width(29.0),
        shield("US:NPS:Blue_Ridge").no_text(),
        shield("US:NPS:Natchez_Trace").no_text(),
        shield("US:NV").text_offset([0.0, -0.64]).text_width(21.5),
        shield("US:NV:Clark")
            .text_offset([0.0, -0.12])
            .text_width(27.0)
            .text_color("#1966b3"),
        shield("US:NY:Parkway:LI")
            .text_offset([0.24, -0.36])
            .text_width(21.5),
        shield("US:NY:Parkway:LOSP").no_text(),
        shield("US:NY:Parkway:NYC").text_offset([0.0, -0.36]),
        shield("US:NY:STE").no_text(),
        shield("US:NY:Scenic"),
        shield("US:NY:TBTA"),
        shield("US:NY:Thruway").no_text(),
        shield("US:Navajo").wide_sized_icon(),
        shield("US:OH").text_offset([0.0, -0.12]).sized_icon(),
        shield("US:OH:ASD").text_width(25.0).text_color("#006747"),
        shield("US:OH:HOL").white_text_color(),
        shield("US:OH:JHMHT"),
        shield("US:OH:OEC"),
        shield("US:OH:SCI").text_offset([0.0, -0.12]).sized_icon(),
        shield("US:OH:TUS:Salem").text_offset([-0.12, -0.18]),
        shield("US:OH:Turnpike"),
        shield("US:OK").text_offset([0.0, 0.24]).sized_icon(),
        shield("US:OK:Turnpike"),
        shield("US:OR").text_offset([0.0, -0.12]).sized_icon(),
        shield("US:ORSB"),
        shield("US:PA").sized_icon(),
        shield("US:PA:Turnpike").white_text_color().sized_icon(),
        shield("US:PANYNJ"),
        shield("US:PIPC"),
        shield("US:SC")
            .text_offset([0.0, 0.18])
            .text_color("#1966b3"),
        shield("US:SD").text_offset([0.0, -0.12]).sized_icon(),
        shield("US:SD:Custer:CSP").text_offset([0.0, -0.18]),
        shield("US:TN:primary").text_offset([0.0, -0.3]),
        shield("US:TX:Fort_Bend:FBCTRA")
            .text_offset([0.0, -0.36])
            .text_width(25.0)
            .white_text_color(),
        shield("US:US").sized_icon(),
        shield("US:UT").sized_icon(),
        shield("US:VT")
            .text_offset([0.0, 0.18])
            .text_color("#006747")
            .sized_icon(),
        shield("US:WA").text_offset([-0.06, -0.01]).text_width(22.0),
        shield("US:WI").text_offset([0.0, -0.18]).sized_icon(),
        shield("US:WI:Rustic")
            .text_offset([-0.15, 0.3])
            .text_color("#ffcd00"),
        shield("co:national").text_offset([0.0, -0.24]).sized_icon(),
    ]
}

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

    let landuse = openmaptiles.landuse();
    let landcover = openmaptiles.landcover();
    let waterway = openmaptiles.waterway();
    let transportation = openmaptiles.transportation();
    let transportation_name = openmaptiles.transportation_name();
    let poi = openmaptiles.poi();
    let place = openmaptiles.place();

    style.background("background").color(rgb!(241, 239, 231));
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
            (rgb!(255, 231, 220)),
            "crop",
            (rgb!(149, 222, 58)),
            "grass",
            (rgb!(230, 255, 133)),
            "shrub",
            (rgb!(230, 255, 133)),
            "snow",
            (rgb!(250, 250, 250)),
            "trees",
            (rgb!(130, 235, 60)),
            "urban",
            (rgb!(226, 231, 215)),
            "transparent"
        ]);

    let park = openmaptiles.park();
    style
        .fill(&park, "park")
        .color(hsl!(101, 62.6%, 63.3%))
        .opacity(0.7)
        .outline_color(rgba!(95, 208, 100, 1));
    style
        .line(&park, "park_outline")
        .dasharray(vec![1.0, 1.5])
        .color(rgb!(228, 241, 215));

    // TODO: Residential landuse may only stand in for building coverage before buildings
    // appear; landcover's urban class might replace these landuse rules.
    style
        .fill(&landuse, "landuse_residential")
        .maxzoom(17.0)
        .filter(Filter::class("residential"))
        .opacity(zoom!(16 => 1.0, 17 => 0.0))
        .color(zoom!(9 => hsla!(0, 3%, 85%, 0.84), 12 => hsla!(35, 57%, 88%, 0.49)));

    // Ground cover reads as a wash, so it is drawn without antialiasing.
    let mut landcover_fill = |class: &str, color: Color, opacity: f64| {
        style
            .fill(&landcover, format!("landcover_{class}"))
            .filter(Filter::class(class))
            .antialias(false)
            .color(color)
            .opacity(opacity);
    };
    // TODO: Replace the opacity with an adjustment to hsl.
    landcover_fill("wood", hsl!(101, 62.6%, 43.3%), 0.3);
    landcover_fill("grass", hsl!(101, 62.6%, 63.3%), 0.5);
    // TODO: Convert ice to hsl
    landcover_fill("ice", rgba!(224, 236, 236, 1), 0.8);
    style
        .fill(&landuse, "landuse_cemetery")
        .filter(Filter::class("cemetery"))
        .color(hsl!(75, 37%, 81%));
    style
        .fill(&landuse, "landuse_hospital")
        .filter(Filter::class("hospital"))
        .color("#fde");
    style
        .fill(&landuse, "landuse_school")
        .filter(Filter::class("school"))
        .color(rgb!(236, 238, 204));

    style
        .line(&waterway, "waterway_tunnel")
        .filter(tunnel())
        .color(water())
        .dasharray(vec![3.0, 3.0])
        .gap_width(zoom!(12 => 0.0, 20 => 6.0))
        .width(zoom!(base: 1.4; 8 => 1.0, 20 => 2.0));
    style
        .fill(&landcover, "landcover_sand")
        .filter(Filter::class("sand"))
        .color(rgba!(247, 239, 195, 1));
    style
        .fill(&openmaptiles.water(), "water")
        .filter(above_ground())
        .color(water());

    style
        .line(&waterway, "waterway_river")
        .filter(Filter::class("river").and(above_ground()))
        .cap_round()
        .color(water())
        .width(zoom!(base: 1.2; 11 => 0.5, 20 => 6.0));
    style
        .line(&waterway, "waterway_other")
        .filter(Filter::class_not("river").and(above_ground()))
        .cap_round()
        .color(water())
        .width(zoom!(base: 1.3; 13 => 0.5, 20 => 6.0));

    let aeroway = openmaptiles.aeroway();
    style
        .fill(&aeroway, "aeroway_fill")
        .minzoom(11.0)
        .filter(Filter::polygon())
        .color(rgba!(229, 228, 224, 1))
        .opacity(0.7);
    style
        .line(&aeroway, "aeroway_runway")
        .minzoom(11.0)
        .filter(Filter::linestring().class("runway"))
        .color("#f0ede9")
        .width(zoom!(base: 1.2; 11 => 3.0, 20 => 16.0));
    style
        .line(&aeroway, "aeroway_taxiway")
        .minzoom(11.0)
        .filter(Filter::linestring().class("taxiway"))
        .color("#f0ede9")
        .width(zoom!(base: 1.2; 11 => 0.5, 20 => 6.0));

    let transportation_lines = style.line_factory(transportation.clone());
    let rounded_transportation = transportation_lines.rounded();
    style
        .line(&transportation, "tunnel_service_track_casing")
        .minzoom(13.0)
        .filter(tunnel().class_in(["service", "track"]))
        .join_round()
        .paint(
            "line-color",
            zoom![
                15.0 => hsl!(0, 0%, 94%),
                18.0 => hsl!(0, 0%, 87%),
                20.0 => hsl!(0, 0%, 82%),
            ],
        )
        .dasharray(vec![0.5, 0.25])
        .paint(
            "line-width",
            zoom!(base: 1.2; 13.0 => 0.76, 16.0 => 4.0, 20.0 => 11.0),
        );
    rounded_transportation
        .line(&mut style, "tunnel_link_casing")
        .minzoom(13.0)
        .filter(tunnel().and_eq("ramp", 1.0))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 93%),
                12.0 => hsl!(0, 0%, 93%),
                15.0 => hsl!(0, 0%, 89%),
                20.0 => hsl!(0, 0%, 82%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 2.12, 14.0 => 4.0, 20.0 => 23.5),
        );
    rounded_transportation
        .line(&mut style, "tunnel_street_casing")
        .minzoom(13.0)
        .filter(tunnel().class_in(["street", "street_limited"]))
        .paint(
            "line-color",
            zoom![
                13.0 => hsl!(0, 0%, 93%),
                16.0 => hsl!(0, 0%, 87.5%),
                20.0 => hsl!(0, 0%, 82%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 2.12, 14.0 => 4.0, 20.0 => 20.0),
        );
    rounded_transportation
        .line(&mut style, "tunnel_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(tunnel().class_in(["secondary", "tertiary"]))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 91%),
                11.0 => hsl!(0, 0%, 91%),
                12.0 => hsl!(0, 0%, 91%),
                13.0 => hsl!(0, 0%, 89%),
                20.0 => hsl!(0, 0%, 82%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.76, 13.0 => 3.6, 20.0 => 30.0),
        );
    rounded_transportation
        .line(&mut style, "tunnel_primary_casing")
        .minzoom(13.0)
        .filter(tunnel().class("primary"))
        .paint(
            "line-color",
            zoom![13.0 => hsl!(0, 0%, 83.5%), 20.0 => hsl!(0, 0%, 76%)],
        )
        .paint("line-width", zoom!(base: 1.2; 13.0 => 5.9, 20.0 => 36.0));
    rounded_transportation
        .line(&mut style, "tunnel_trunk_casing")
        .minzoom(13.0)
        .filter(tunnel().class("trunk"))
        .paint(
            "line-color",
            zoom![13.0 => hsl!(0, 0%, 81%), 20.0 => hsl!(0, 0%, 74.5%)],
        )
        .paint("line-width", zoom!(base: 1.2; 13.0 => 5.9, 20.0 => 36.0));
    style
        .line(&transportation, "tunnel_motorway_link_casing")
        .minzoom(13.0)
        .filter(tunnel().class("motorway").and_eq("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            zoom![
                12.0 => hsl!(0, 0%, 82%),
                15.0 => hsl!(0, 0%, 77%),
                20.0 => hsl!(0, 0%, 70%),
            ],
        )
        .dasharray(vec![0.5, 0.25])
        .paint(
            "line-width",
            zoom!(base: 1.2; 8.0 => 1.74, 13.0 => 5.9, 20.0 => 35.0),
        );
    style
        .line(&transportation, "tunnel_motorway_casing")
        .minzoom(10.0)
        .filter(tunnel().class("motorway").and_ne("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            zoom![
                7.0 => hsl!(0, 0%, 80%),
                13.0 => hsl!(0, 0%, 75%),
                20.0 => hsl!(0, 0%, 70%),
            ],
        )
        .dasharray(vec![0.5, 0.25])
        .paint(
            "line-width",
            zoom!(base: 1.2; 5.0 => 1.4, 7.0 => 1.75, 13.0 => 5.9, 20.0 => 40.0),
        );
    rounded_transportation
        .line(&mut style, "tunnel_service_track")
        .minzoom(13.0)
        .filter(tunnel().class_in(["service", "track"]))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 97%),
                13.0 => hsl!(0, 0%, 92%),
                20.0 => hsl!(0, 0%, 87%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 13.0 => 0.0, 16.0 => 2.0, 20.0 => 7.5),
        );
    rounded_transportation
        .line(&mut style, "tunnel_link")
        .minzoom(12.0)
        .filter(tunnel().and_eq("ramp", 1.0))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 94%),
                12.0 => hsl!(0, 0%, 94%),
                20.0 => hsl!(0, 0%, 87%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.0, 14.0 => 2.5, 20.0 => 20.0),
        );
    rounded_transportation
        .line(&mut style, "tunnel_minor")
        .minzoom(12.0)
        .filter(tunnel().class_in(["minor"]))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 93%),
                13.0 => hsl!(0, 0%, 93%),
                20.0 => hsl!(0, 0%, 87%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.0, 14.0 => 2.5, 20.0 => 18.0),
        );
    style
        .line(&transportation, "tunnel_path_pedestrian")
        .minzoom(13.0)
        .filter(
            tunnel()
                .and(Filter::linestring())
                .class_in(["path", "pedestrian"]),
        )
        .color(hsl!(0, 0%, 98%))
        .dasharray(vec![1.0, 0.75])
        .paint("line-width", zoom!(base: 1.2; 13.0 => 0.75, 20.0 => 10.0));
    rounded_transportation
        .line(&mut style, "tunnel_secondary_tertiary")
        .minzoom(12.0)
        .filter(tunnel().class_in(["secondary", "tertiary"]))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 95%),
                11.0 => hsl!(0, 0%, 95%),
                12.0 => hsl!(0, 0%, 95%),
                20.0 => hsl!(0, 0%, 87%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.0, 13.0 => 2.5, 20.0 => 26.0),
        );
    rounded_transportation
        .line(&mut style, "tunnel_primary")
        .minzoom(8.0)
        .filter(tunnel().class("primary"))
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 64%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0),
        );
    rounded_transportation
        .line(&mut style, "tunnel_trunk")
        .minzoom(6.0)
        .filter(tunnel().class("trunk"))
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 61%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 6.0 => 1.0, 10.0 => 2.85, 13.0 => 4.5, 20.0 => 32.0),
        );
    rounded_transportation
        .line(&mut style, "tunnel_motorway_link")
        .minzoom(8.0)
        .filter(tunnel().class("motorway").and_eq("ramp", 1.0))
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 84%), 20.0 => hsl!(0, 0%, 76%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0),
        );
    rounded_transportation
        .line(&mut style, "tunnel_motorway")
        .minzoom(5.0)
        .filter(tunnel().class("motorway").and_ne("ramp", 1.0))
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 84%), 20.0 => hsl!(0, 0%, 76%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 5.0 => 1.0, 7.0 => 1.0, 13.0 => 4.5, 20.0 => 36.0),
        );
    style
        .line(&transportation, "tunnel_major_rail")
        .filter(tunnel().class_in(["rail"]))
        .cap_round()
        .color(hsl!(0, 0%, 86.6667%))
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0),
        );
    style
        .line(&transportation, "tunnel_major_rail_hatching")
        .filter(tunnel().class("rail"))
        .color(hsl!(0, 0%, 86.6667%))
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0),
        );
    style
        .line(&transportation, "tunnel_transit_rail")
        .filter(tunnel().class_in(["transit"]))
        .cap_round()
        .color(hsl!(0, 0%, 86.6667%))
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0),
        );
    style
        .line(&transportation, "tunnel_transit_rail_hatching")
        .filter(tunnel().class("transit"))
        .color(hsl!(0, 0%, 86.6667%))
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0),
        );
    style
        .fill(&transportation, "road_area_pattern")
        .filter(Filter::polygon())
        .pattern("pedestrian_polygon");
    rounded_transportation
        .line(&mut style, "road_service_track_casing")
        .minzoom(13.0)
        .filter(surface().class_in(["service", "track"]))
        .paint(
            "line-color",
            zoom![
                15.0 => hsl!(0, 0%, 88%),
                18.0 => hsl!(0, 0%, 74%),
                20.0 => hsl!(0, 0%, 64%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 13.0 => 0.76, 16.0 => 4.0, 20.0 => 11.0),
        );
    rounded_transportation
        .line(&mut style, "road_link_casing")
        .minzoom(13.0)
        .filter(
            surface()
                .and_not_in(
                    "class",
                    ["pedestrian", "path", "track", "service", "motorway"],
                )
                .and_eq("ramp", 1.0),
        )
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 86%),
                12.0 => hsl!(0, 0%, 86%),
                15.0 => hsl!(0, 0%, 78%),
                20.0 => hsl!(0, 0%, 64%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 2.12, 14.0 => 4.0, 20.0 => 23.5),
        );
    rounded_transportation
        .line(&mut style, "road_minor_casing")
        .minzoom(13.0)
        .filter(
            Filter::linestring()
                .and(surface())
                .class_in(["minor"])
                .and_ne("ramp", 1.0),
        )
        .paint(
            "line-color",
            zoom![
                13.0 => hsl!(0, 0%, 86%),
                16.0 => hsl!(0, 0%, 75%),
                20.0 => hsl!(0, 0%, 64%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 2.12, 14.0 => 4.0, 20.0 => 20.0),
        );
    rounded_transportation
        .line(&mut style, "road_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(
            surface()
                .class_in(["secondary", "tertiary"])
                .and_ne("ramp", 1.0),
        )
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 82%),
                11.0 => hsl!(0, 0%, 82%),
                12.0 => hsl!(0, 0%, 82%),
                13.0 => hsl!(0, 0%, 78%),
                20.0 => hsl!(0, 0%, 64%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.76, 13.0 => 3.6, 20.0 => 30.0),
        );
    rounded_transportation
        .line(&mut style, "road_primary_casing")
        .minzoom(13.0)
        .filter(surface().class("primary"))
        .paint(
            "line-color",
            zoom![13.0 => hsl!(0, 0%, 67%), 20.0 => hsl!(0, 0%, 52%)],
        )
        .paint("line-width", zoom!(base: 1.2; 13.0 => 5.9, 20.0 => 36.0));
    rounded_transportation
        .line(&mut style, "road_trunk_casing")
        .minzoom(13.0)
        .filter(surface().class("trunk"))
        .paint(
            "line-color",
            zoom![13.0 => hsl!(0, 0%, 62%), 20.0 => hsl!(0, 0%, 49%)],
        )
        .paint("line-width", zoom!(base: 1.2; 13.0 => 5.9, 20.0 => 36.0));
    rounded_transportation
        .line(&mut style, "road_motorway_link_casing")
        .minzoom(13.0)
        .filter(surface().class("motorway").and_eq("ramp", 1.0))
        .paint(
            "line-color",
            zoom![
                12.0 => hsl!(0, 0%, 64%),
                15.0 => hsl!(0, 0%, 54%),
                20.0 => hsl!(0, 0%, 40%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 8.0 => 1.74, 13.0 => 5.9, 20.0 => 35.0),
        );
    rounded_transportation
        .line(&mut style, "road_motorway_casing")
        .minzoom(10.0)
        .filter(surface().class("motorway").and_ne("ramp", 1.0))
        .paint(
            "line-color",
            zoom![
                7.0 => hsl!(0, 0%, 60%),
                13.0 => hsl!(0, 0%, 50%),
                20.0 => hsl!(0, 0%, 40%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 5.0 => 1.4, 7.0 => 1.75, 13.0 => 5.9, 20.0 => 40.0),
        );
    rounded_transportation
        .line(&mut style, "road_service_track")
        .minzoom(13.0)
        .filter(surface().class_in(["service", "track"]))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 94%),
                13.0 => hsl!(0, 0%, 84%),
                20.0 => hsl!(0, 0%, 74%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 13.0 => 0.0, 16.0 => 2.0, 20.0 => 7.5),
        );
    rounded_transportation
        .line(&mut style, "road_link")
        .minzoom(12.0)
        .filter(surface().and_eq("ramp", 1.0).and_not_in(
            "class",
            ["pedestrian", "path", "track", "service", "motorway"],
        ))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 88%),
                12.0 => hsl!(0, 0%, 88%),
                20.0 => hsl!(0, 0%, 74%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.0, 14.0 => 2.5, 20.0 => 20.0),
        );
    rounded_transportation
        .line(&mut style, "road_minor")
        .minzoom(12.0)
        .filter(Filter::linestring().and(surface()).class_in(["minor"]))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 86%),
                13.0 => hsl!(0, 0%, 86%),
                20.0 => hsl!(0, 0%, 74%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.0, 14.0 => 2.5, 20.0 => 18.0),
        );
    style
        .line(&transportation, "road_path_pedestrian")
        .minzoom(13.0)
        .filter(
            Filter::linestring()
                .and(surface())
                .class_in(["path", "pedestrian"]),
        )
        .join_round()
        .color(hsl!(0, 0%, 96%))
        .dasharray(vec![1.0, 0.7])
        .paint("line-width", zoom!(base: 1.2; 13.0 => 0.75, 20.0 => 10.0));
    rounded_transportation
        .line(&mut style, "road_secondary_tertiary")
        .minzoom(12.0)
        .filter(surface().class_in(["secondary", "tertiary"]))
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 90%),
                11.0 => hsl!(0, 0%, 90%),
                12.0 => hsl!(0, 0%, 90%),
                20.0 => hsl!(0, 0%, 74%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.0, 13.0 => 2.5, 20.0 => 26.0),
        );
    rounded_transportation
        .line(&mut style, "road_primary")
        .minzoom(8.0)
        .filter(surface().class("primary"))
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 64%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0),
        );
    rounded_transportation
        .line(&mut style, "road_trunk")
        .minzoom(6.0)
        .filter(surface().class("trunk"))
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 61%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 6.0 => 1.0, 10.0 => 2.85, 13.0 => 4.5, 20.0 => 32.0),
        );
    rounded_transportation
        .line(&mut style, "road_motorway_link")
        .minzoom(8.0)
        .filter(surface().class("motorway").and_eq("ramp", 1.0))
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 52%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0),
        );
    rounded_transportation
        .line(&mut style, "road_motorway")
        .minzoom(5.0)
        .filter(surface().class("motorway").and_ne("ramp", 1.0))
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 52%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 5.0 => 1.0, 7.0 => 1.0, 13.0 => 4.5, 20.0 => 36.0),
        );
    style
        .line(&transportation, "road_major_rail")
        .filter(surface().class("rail"))
        .cap_round()
        .color("#bbb")
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0),
        );
    style
        .line(&transportation, "road_major_rail_hatching")
        .filter(surface().class("rail"))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0),
        );
    style
        .line(&transportation, "road_transit_rail")
        .filter(surface().class("transit"))
        .cap_round()
        .color("#bbb")
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0),
        );
    style
        .line(&transportation, "road_transit_rail_hatching")
        .filter(surface().class("transit"))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0),
        );
    style
        .line(&transportation, "bridge_service_track_casing")
        .minzoom(13.0)
        .filter(bridge().class_in(["service", "track"]))
        .join_round()
        .paint(
            "line-color",
            zoom![
                15.0 => hsl!(0, 0%, 88%),
                18.0 => hsl!(0, 0%, 74%),
                20.0 => hsl!(0, 0%, 64%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 13.0 => 0.76, 16.0 => 4.0, 20.0 => 11.0),
        );
    style
        .line(&transportation, "bridge_link_casing")
        .minzoom(13.0)
        .filter(bridge().class("link"))
        .join_round()
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 86%),
                12.0 => hsl!(0, 0%, 86%),
                15.0 => hsl!(0, 0%, 78%),
                20.0 => hsl!(0, 0%, 64%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 2.12, 14.0 => 4.0, 20.0 => 23.5),
        );
    style
        .line(&transportation, "bridge_street_casing")
        .minzoom(13.0)
        .filter(bridge().class_in(["street", "street_limited"]))
        .join_round()
        .paint(
            "line-color",
            zoom![
                13.0 => hsl!(0, 0%, 86%),
                16.0 => hsl!(0, 0%, 75%),
                20.0 => hsl!(0, 0%, 64%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 2.12, 14.0 => 4.0, 20.0 => 20.0),
        );
    style
        .line(&transportation, "bridge_path_pedestrian_casing")
        .minzoom(13.0)
        .filter(
            bridge()
                .and(Filter::linestring())
                .class_in(["path", "pedestrian"]),
        )
        .paint(
            "line-color",
            zoom![14.0 => hsl!(0, 0%, 88%), 20.0 => hsl!(0, 0%, 64%)],
        )
        .dasharray(vec![1.0, 0.0])
        .paint("line-width", zoom!(base: 1.2; 14.0 => 1.125, 20.0 => 18.0));
    style
        .line(&transportation, "bridge_secondary_tertiary_casing")
        .minzoom(13.0)
        .filter(bridge().class_in(["secondary", "tertiary"]))
        .join_round()
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 82%),
                11.0 => hsl!(0, 0%, 82%),
                12.0 => hsl!(0, 0%, 82%),
                13.0 => hsl!(0, 0%, 78%),
                20.0 => hsl!(0, 0%, 64%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.76, 13.0 => 3.6, 20.0 => 30.0),
        );
    style
        .line(&transportation, "bridge_primary_casing")
        .minzoom(13.0)
        .filter(bridge().class("primary"))
        .join_round()
        .paint(
            "line-color",
            zoom![13.0 => hsl!(0, 0%, 67%), 20.0 => hsl!(0, 0%, 52%)],
        )
        .paint("line-width", zoom!(base: 1.2; 13.0 => 5.9, 20.0 => 36.0));
    style
        .line(&transportation, "bridge_trunk_casing")
        .minzoom(13.0)
        .filter(bridge().class("trunk"))
        .join_round()
        .paint(
            "line-color",
            zoom![13.0 => hsl!(0, 0%, 62%), 20.0 => hsl!(0, 0%, 49%)],
        )
        .paint("line-width", zoom!(base: 1.2; 13.0 => 5.9, 20.0 => 36.0));
    style
        .line(&transportation, "bridge_motorway_link_casing")
        .minzoom(13.0)
        .filter(bridge().class("motorway").and_eq("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            zoom![
                12.0 => hsl!(0, 0%, 64%),
                15.0 => hsl!(0, 0%, 54%),
                20.0 => hsl!(0, 0%, 40%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 8.0 => 1.74, 13.0 => 5.9, 20.0 => 35.0),
        );
    style
        .line(&transportation, "bridge_motorway_casing")
        .minzoom(10.0)
        .filter(bridge().class("motorway").and_ne("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            zoom![
                7.0 => hsl!(0, 0%, 60%),
                13.0 => hsl!(0, 0%, 50%),
                20.0 => hsl!(0, 0%, 40%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 5.0 => 1.4, 7.0 => 1.75, 13.0 => 5.9, 20.0 => 40.0),
        );
    style
        .line(&transportation, "bridge_service_track")
        .minzoom(13.0)
        .filter(bridge().class_in(["service", "track"]))
        .join_round()
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 94%),
                13.0 => hsl!(0, 0%, 84%),
                20.0 => hsl!(0, 0%, 74%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 13.0 => 0.0, 16.0 => 2.0, 20.0 => 7.5),
        );
    style
        .line(&transportation, "bridge_link")
        .minzoom(12.0)
        .filter(bridge().class("link"))
        .join_round()
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 88%),
                12.0 => hsl!(0, 0%, 88%),
                20.0 => hsl!(0, 0%, 74%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.0, 14.0 => 2.5, 20.0 => 20.0),
        );
    style
        .line(&transportation, "bridge_street")
        .minzoom(12.0)
        .filter(bridge().class_in(["minor"]))
        .join_round()
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 86%),
                13.0 => hsl!(0, 0%, 86%),
                20.0 => hsl!(0, 0%, 74%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.0, 14.0 => 2.5, 20.0 => 18.0),
        );
    style
        .line(&transportation, "bridge_path_pedestrian")
        .minzoom(13.0)
        .filter(
            bridge()
                .and(Filter::linestring())
                .class_in(["path", "pedestrian"]),
        )
        .color(hsl!(0, 0%, 96%))
        .dasharray(vec![1.0, 0.3])
        .paint("line-width", zoom!(base: 1.2; 13.0 => 0.75, 20.0 => 10.0));
    style
        .line(&transportation, "bridge_secondary_tertiary")
        .minzoom(12.0)
        .filter(bridge().class_in(["secondary", "tertiary"]))
        .join_round()
        .paint(
            "line-color",
            zoom![
                10.0 => hsl!(0, 0%, 90%),
                11.0 => hsl!(0, 0%, 90%),
                12.0 => hsl!(0, 0%, 90%),
                20.0 => hsl!(0, 0%, 74%),
            ],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 12.0 => 1.0, 13.0 => 2.5, 20.0 => 26.0),
        );
    style
        .line(&transportation, "bridge_primary")
        .minzoom(8.0)
        .filter(bridge().class("primary"))
        .join_round()
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 64%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0),
        );
    style
        .line(&transportation, "bridge_trunk")
        .minzoom(6.0)
        .filter(bridge().class("trunk"))
        .join_round()
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 61%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 6.0 => 1.0, 10.0 => 2.85, 13.0 => 4.5, 20.0 => 32.0),
        );
    style
        .line(&transportation, "bridge_motorway_link")
        .minzoom(8.0)
        .filter(bridge().class("motorway").and_eq("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 52%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 8.0 => 1.0, 13.0 => 4.5, 20.0 => 32.0),
        );
    style
        .line(&transportation, "bridge_motorway")
        .minzoom(5.0)
        .filter(bridge().class("motorway").and_ne("ramp", 1.0))
        .join_round()
        .paint(
            "line-color",
            zoom![10.0 => hsl!(0, 0%, 68%), 20.0 => hsl!(0, 0%, 52%)],
        )
        .paint(
            "line-width",
            zoom!(base: 1.2; 5.0 => 1.0, 7.0 => 1.0, 13.0 => 4.5, 20.0 => 36.0),
        );
    style
        .line(&transportation, "bridge_major_rail")
        .filter(bridge().class("rail"))
        .color("#bbb")
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0),
        );
    style
        .line(&transportation, "bridge_major_rail_hatching")
        .filter(bridge().class("rail"))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0),
        );
    style
        .line(&transportation, "bridge_transit_rail")
        .filter(bridge().class("transit"))
        .color("#bbb")
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.0 => 0.4, 15.0 => 0.75, 20.0 => 2.0),
        );
    style
        .line(&transportation, "bridge_transit_rail_hatching")
        .filter(bridge().class("transit"))
        .color("#bbb")
        .dasharray(vec![0.2, 8.0])
        .paint(
            "line-width",
            zoom!(base: 1.4; 14.5 => 0.0, 15.0 => 3.0, 20.0 => 8.0),
        );

    style
        .fill(&openmaptiles.building(), "building")
        .minzoom(14.0)
        .color(hsl!(40, 5.5%, 89.2%))
        .opacity(zoom!(14 => 0.0, 15 => 1.0))
        .outline_color(zoom!(14 => hsla!(40, 6%, 79%, 0.32), 15 => hsl!(40, 6%, 79%)));

    let boundary = openmaptiles.boundary();
    style
        .line(&boundary, "boundary_3")
        .minzoom(3.0)
        .filter(Filter::is_in("admin_level", [3, 4]))
        .join_round()
        .color("#747980")
        .dasharray(vec![1.5, 1.5])
        .width(zoom!(3 => 0.6, 6 => 1.0, 10 => 2.0));
    // Disputed borders are only drawn once they can be labelled as such.
    style
        .line(&boundary, "boundary_2_z0-4")
        .rounded()
        .color(hsl!(248, 1%, 41%))
        .opacity(zoom!(0 => 0.4, 4 => 1.0))
        .width(zoom!(3 => 1.0, 5 => 1.2, 12 => 3.0))
        .maxzoom(5.0)
        .filter(Filter::eq("admin_level", 2).and_not_has("claimed_by"));
    style
        .line(&boundary, "boundary_2_z5-")
        .rounded()
        .color(hsl!(248, 1%, 41%))
        .opacity(zoom!(0 => 0.4, 4 => 1.0))
        .width(zoom!(3 => 1.0, 5 => 1.2, 12 => 3.0))
        .minzoom(5.0)
        .filter(Filter::eq("admin_level", 2));

    style
        .symbol(&transportation, "road_one_way_arrow")
        .minzoom(15.0)
        .filter(Filter::eq("oneway", 1))
        .icon_image("arrow")
        .placement("line");
    style
        .symbol(&transportation, "road_one_way_arrow_opposite")
        .minzoom(15.0)
        .filter(Filter::eq("oneway", -1))
        .icon_image("arrow")
        .placement("line")
        .icon_rotate(180.0);
    style
        .line(&transportation, "route_ferry")
        .minzoom(12.0)
        .filter(Filter::linestring().class_in(["ferry"]))
        .rounded()
        .color("#99d")
        .width(2.0)
        .opacity(zoom!(12 => 0.0, 13 => 1.0))
        .dasharray(vec![0.3, 2.0]);

    {
        // The shared typography for waterway and water-body labels.
        let mut water_label = style
            .symbol_factory()
            .text_field("{name}")
            .text_font([REGULAR])
            .text_max_width(5.0)
            .text_size(12.0)
            .text_color("#5d60be")
            .text_halo(rgba!(255, 255, 255, 0.7), 1.0);
        water_label
            .symbol(&waterway, "water_name_line")
            .filter(Filter::linestring())
            .placement("line");
        let water_name = openmaptiles.water_name();
        water_label
            .symbol(&water_name, "water_name_lake_line")
            .filter(Filter::linestring())
            .placement("line");
        water_label
            .symbol(&water_name, "water_name_point")
            .filter(Filter::point());
    }

    {
        let mut poi_label = style
            .symbol_factory()
            .icon_image("{class}_11")
            .text_font([ITALIC])
            .text_max_width(9.0)
            .text_size(12.0)
            .text_field("{name}")
            .text_anchor("top")
            .text_offset([0.0, 0.6])
            .text_color("#666")
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
                .filter(Filter::point().and(rank));
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
        .text_size(zoom![13.0 => 12.0, 14.0 => 13.0])
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
        .text_font(["Overpass Heavy"])
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
    let mut place_label = style
        .symbol_factory()
        .text_field("{name_en}")
        .text_halo(halo(), 1.2);
    place_label
        .symbol(&place, "place_other")
        .filter(Filter::is_in("class", neighbourhoods))
        .text_font([ITALIC])
        .text_letter_spacing(0.1)
        .text_max_width(9.0)
        .text_size(zoom!(base: 1.2; 12 => 10.0, 15 => 14.0))
        .text_transform("uppercase")
        .text_color("#633");
    place_label
        .symbol(&place, "place_village")
        .filter(Filter::class("village"))
        .text_font([REGULAR])
        .text_max_width(8.0)
        .text_size(zoom!(base: 1.2; 10 => 12.0, 15 => 22.0))
        .text_color("#333");
    // Towns and cities carry a dot until their own footprint is visible.
    let town_size = zoom!(base: 1.2; 7 => 12.0, 11 => 16.0);
    place_label
        .symbol(&place, "place_town")
        .filter(Filter::class("town"))
        .icon_image(zoom!(0 => "dot_9", 8 => ""))
        .text_anchor("bottom")
        .text_font([REGULAR])
        .text_max_width(8.0)
        .text_size(town_size)
        .text_color("#333");
    let city_size = zoom!(base: 1.2; 7 => 14.0, 11 => 24.0);
    place_label
        .symbol(&place, "place_city")
        .filter(Filter::class("city"))
        .icon_image(zoom!(0 => "dot_9", 8 => ""))
        .text_anchor("bottom")
        .text_font([MEDIUM])
        .text_max_width(8.0)
        .text_size(city_size)
        .text_color("#333")
        .minzoom(5.0)
        .icon_allow_overlap(true)
        .icon_optional(false);

    place_label
        .symbol(&place, "state")
        .filter(Filter::class("state"))
        .maxzoom(6.0)
        .text_font([ITALIC])
        .text_size(zoom!(4 => 11.0, 6 => 15.0))
        .text_transform("uppercase")
        .text_color("#633")
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
            .symbol(&place, id)
            .filter(rank.class("country"))
            .text_font([ITALIC])
            .text_max_width(6.25)
            .text_size(size)
            .text_color("#334")
            .text_halo_blur(1.0)
            .text_halo(halo(), 1.0);
    }
    place_label
        .symbol(&place, "continent")
        .filter(Filter::class("continent"))
        .maxzoom(1.0)
        .text_font([ITALIC])
        .text_size(13.0)
        .text_transform("uppercase")
        .text_color("#633")
        .text_halo(rgba!(255, 255, 255, 0.7), 1.0);

    style.validate();
    style
}
