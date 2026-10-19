use crate::*;

/// The per-network exceptions to Headway's road-shield defaults.
#[derive(Clone, Copy)]
pub(super) struct RoadShield {
    pub(super) network: &'static str,
    pub(super) text: RoadShieldText,
    pub(super) text_offset: Option<[f64; 2]>,
    pub(super) text_size: f64,
    pub(super) text_width: Option<f64>,
    pub(super) icon: RoadShieldIcon,
    pub(super) text_color: Option<Color>,
    pub(super) halo_color: Option<Color>,
    pub(super) halo_width: Option<f64>,
}

#[derive(Clone, Copy)]
pub(super) enum RoadShieldText {
    NoText,
    Route1Ref,
}

#[derive(Clone, Copy)]
pub(super) enum RoadShieldIcon {
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
    fn text_color(mut self, value: Color) -> Self {
        self.text_color = Some(value);
        self
    }
    fn white_text_color(self) -> Self {
        self.text_color(hex!(0xffffff))
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
    fn halo_color(mut self, value: Color) -> Self {
        self.halo_color = Some(value);
        self
    }
    fn halo_width(mut self, value: f64) -> Self {
        self.halo_width = Some(value);
        self
    }

    pub(super) fn with_ref(body: Expression) -> Expression {
        Expression::op(
            Operator::Let,
            [
                Expression::literal("ref"),
                expr![Coalesce, [Get, "route_1_ref"], [Get, "ref"]],
                body,
            ],
        )
    }

    pub(super) fn property(
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

    pub(super) fn offset([x, y]: [f64; 2]) -> Expression {
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

    pub(super) fn icon_image(shields: &[Self]) -> Expression {
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

pub(super) fn road_shields() -> Vec<RoadShield> {
    fn shield(network: &'static str) -> RoadShield {
        RoadShield::new(network)
    }
    vec![
        shield("CA:BC").text_color(hex!(0x1966b3)).sized_icon(),
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
            .text_color(hex!(0x006747)),
        shield("CA:transcanada")
            .text_color(hex!(0x006747))
            .sized_icon(),
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
            .text_color(hex!(0x1966b3)),
        shield("US:CO:Scenic"),
        shield("US:CT:Parkway"),
        shield("US:DC")
            .text_offset([0.0, 0.29])
            .text_width(24.0)
            .halo_color(hex!(0xffffff))
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
            .text_color(hex!(0x1966b3)),
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
            .text_color(hex!(0x1966b3)),
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
        shield("US:OH:ASD")
            .text_width(25.0)
            .text_color(hex!(0x006747)),
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
            .text_color(hex!(0x1966b3)),
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
            .text_color(hex!(0x006747))
            .sized_icon(),
        shield("US:WA").text_offset([-0.06, -0.01]).text_width(22.0),
        shield("US:WI").text_offset([0.0, -0.18]).sized_icon(),
        shield("US:WI:Rustic")
            .text_offset([-0.15, 0.3])
            .text_color(hex!(0xffcd00)),
        shield("co:national").text_offset([0.0, -0.24]).sized_icon(),
    ]
}
