use super::plan::{Itinerary, Leg, Maneuver, ModeLeg};
use crate::util::{serialize_line_string_as_polyline6, serialize_point_as_lon_lat_pair};
use crate::valhalla::valhalla_api::ManeuverType;
use crate::{DistanceUnit, TravelMode};
use geo::{LineString, Point};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    /// The distance traveled by the route, in float meters.
    pub distance: f64,

    /// The estimated travel time, in float number of seconds.
    pub duration: f64,

    // todo: simplify?
    /// The entire geometry of the route
    #[serde(serialize_with = "serialize_line_string_as_polyline6")]
    pub geometry: LineString,

    /// The legs between the given waypoints
    pub legs: Vec<RouteLeg>,
}

impl From<Itinerary> for Route {
    fn from(itinerary: Itinerary) -> Self {
        Route {
            distance: itinerary.distance_meters(),
            duration: itinerary.duration,
            geometry: itinerary.combined_geometry(),
            legs: itinerary
                .legs
                .into_iter()
                .map(|leg| RouteLeg::from_leg(leg, itinerary.distance_units))
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RouteLeg {
    /// The distance traveled by this leg, in float meters.
    pub distance: f64,

    /// The estimated travel time, in float number of seconds.
    pub duration: f64,

    /// A short human-readable summary of the route leg
    pub summary: String,

    /// Objects describing the turn-by-turn instructions of the route leg
    pub steps: Vec<RouteStep>,
    // /// Additional details about each coordinate along the route geometry
    // annotation: Annotation
}

impl RouteLeg {
    fn from_leg(value: Leg, distance_unit: DistanceUnit) -> Self {
        let distance_meters = value.distance_meters(distance_unit);
        let (summary, steps) = match value.mode_leg {
            ModeLeg::Transit(_) => {
                debug_assert!(
                    false,
                    "didn't expect to generate navigation for transit leg"
                );
                ("".to_string(), vec![])
            }
            ModeLeg::NonTransit(non_transit_leg) => {
                let summary = non_transit_leg.substantial_street_names.join(", ");
                let mut steps: Vec<_> = non_transit_leg
                    .maneuvers
                    .windows(2)
                    .map(|this_and_next| {
                        let maneuver = this_and_next[0].clone();
                        let next_maneuver = this_and_next.get(1);
                        RouteStep::from_maneuver(
                            maneuver.clone(),
                            next_maneuver.cloned(),
                            value.mode,
                            distance_unit,
                        )
                    })
                    .collect();
                if let Some(final_maneuver) = non_transit_leg.maneuvers.last() {
                    let final_step = RouteStep::from_maneuver(
                        final_maneuver.clone(),
                        None,
                        value.mode,
                        distance_unit,
                    );
                    steps.push(final_step);
                }
                (summary, steps)
            }
        };
        Self {
            distance: distance_meters,
            duration: value.duration_seconds,
            summary,
            steps,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RouteStep {
    /// The distance traveled by this step, in float meters.
    pub distance: f64,

    /// The estimated travel time, in float number of seconds.
    pub duration: f64,

    /// The unsimplified geometry of the route segment.
    #[serde(serialize_with = "serialize_line_string_as_polyline6")]
    pub geometry: LineString,

    /// The name of the way along which travel proceeds.
    pub name: String,

    /// A reference number or code for the way. Optionally included, if ref data is available for the given way.
    pub r#ref: Option<String>,

    /// The pronunciation hint of the way name. Will be undefined if there is no pronunciation hit.
    pub pronunciation: Option<String>,

    /// The destinations of the way. Will be undefined if there are no destinations.
    pub destinations: Option<Vec<String>>,

    /// A string signifying the mode of transportation.
    pub mode: TravelMode,

    /// A `StepManeuver` object representing the maneuver.
    pub maneuver: StepManeuver,

    /// A list of `BannerInstruction` objects that represent all signs on the step.
    pub banner_instructions: Option<Vec<BannerInstruction>>,

    /// A list of `Intersection` objects that are passed along the segment, the very first belonging to the `StepManeuver`
    pub intersections: Option<Vec<Intersection>>,
}

impl RouteStep {
    fn from_maneuver(
        maneuver: Maneuver,
        next_maneuver: Option<Maneuver>,
        mode: TravelMode,
        from_distance_unit: DistanceUnit,
    ) -> Self {
        let banner_instructions =
            BannerInstruction::from_maneuver(&maneuver, next_maneuver.as_ref(), from_distance_unit);
        RouteStep {
            distance: maneuver.distance_meters(from_distance_unit),
            duration: maneuver.duration_seconds,
            geometry: maneuver.geometry,
            name: maneuver
                .street_names
                .unwrap_or(vec!["".to_string()])
                .join(", "),
            r#ref: None,
            pronunciation: None,
            destinations: None,
            mode,
            maneuver: StepManeuver {
                location: maneuver.start_point.into(),
            },
            intersections: None, //vec![],
            banner_instructions,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BannerInstruction {
    pub distance_along_geometry: f64,
    pub primary: BannerInstructionContent,
    // secondary: Option<BannerInstructionContent>,
    // sub: Option<BannerInstructionContent>,
}

impl BannerInstruction {
    fn from_maneuver(
        maneuver: &Maneuver,
        next_maneuver: Option<&Maneuver>,
        from_distance_unit: DistanceUnit,
    ) -> Option<Vec<Self>> {
        let text = if let Some(next_maneuver) = next_maneuver {
            next_maneuver
                .street_names
                .as_ref()
                .map(|names| names.join(", "))
                .or(next_maneuver.instruction.clone())
        } else {
            assert!(matches!(
                maneuver.r#type,
                ManeuverType::Destination
                    | ManeuverType::DestinationRight
                    | ManeuverType::DestinationLeft
            ));
            maneuver.instruction.to_owned()
        };

        let banner_maneuver = (|| {
            use BannerManeuverModifier::*;
            use BannerManeuverType::*;
            let (banner_type, modifier) = match next_maneuver.unwrap_or(maneuver).r#type {
                ManeuverType::None => return None,
                ManeuverType::Start => (Depart, None),
                ManeuverType::StartRight => (Depart, Some(Right)),
                ManeuverType::StartLeft => (Depart, Some(Left)),
                ManeuverType::Destination => (Arrive, None),
                ManeuverType::DestinationRight => (Arrive, Some(Right)),
                ManeuverType::DestinationLeft => (Arrive, Some(Left)),
                /*
                ManeuverType::Becomes => {}
                */
                ManeuverType::Continue => (Fork, None), // Or maybe just return None?
                ManeuverType::SlightRight => (Turn, Some(SlightRight)),
                ManeuverType::Right => (Turn, Some(Right)),
                ManeuverType::SharpRight => (Turn, Some(SharpRight)),
                /*
                ManeuverType::UturnRight => {}
                ManeuverType::UturnLeft => {}
                */
                ManeuverType::SharpLeft => (Turn, Some(SharpLeft)),
                ManeuverType::Left => (Turn, Some(Left)),
                ManeuverType::SlightLeft => (Turn, Some(SlightLeft)),
                ManeuverType::RampStraight => (OnRamp, Some(Straight)),
                ManeuverType::RampRight => (OnRamp, Some(Right)),
                ManeuverType::RampLeft => (OnRamp, Some(Left)),
                ManeuverType::ExitRight => (OffRamp, Some(Right)),
                ManeuverType::ExitLeft => (OffRamp, Some(Left)),
                ManeuverType::StayStraight => (Fork, None), // Or maybe just return None?
                ManeuverType::StayRight => (Fork, Some(Right)),
                ManeuverType::StayLeft => (Fork, Some(Left)),
                /*
                ManeuverType::Merge => {}
                */
                ManeuverType::RoundaboutEnter => (RoundaboutEnter, None), // Enter/Exit?
                ManeuverType::RoundaboutExit => (RoundaboutExit, None),   // Enter/Exit?
                /*
                ManeuverType::FerryEnter => {}
                ManeuverType::FerryExit => {}
                ManeuverType::Transit => {}
                ManeuverType::TransitTransfer => {}
                ManeuverType::TransitRemainOn => {}
                ManeuverType::TransitConnectionStart => {}
                ManeuverType::TransitConnectionTransfer => {}
                ManeuverType::TransitConnectionDestination => {}
                ManeuverType::PostTransitConnectionDestination => {}
                ManeuverType::MergeRight => {}
                ManeuverType::MergeLeft => {}
                ManeuverType::ElevatorEnter => {}
                ManeuverType::StepsEnter => {}
                ManeuverType::EscalatorEnter => {}
                ManeuverType::BuildingEnter => {}
                ManeuverType::BuildingExit => {}
                 */
                other => todo!("implement maneuver type: {other:?}"),
            };
            Some(BannerManeuver {
                r#type: banner_type,
                modifier,
            })
        })();

        let text_component =
            BannerComponent::Text(VisualInstructionComponent { text: text.clone() });
        //     if let Some(banner_maneuver) = banner_maneuver {
        //         BannerComponent::Text(VisualInstructionComponent {
        //             text,
        //         })
        //     } else {
        //         panic!("no banner_maneuver")
        //     }
        // };

        let primary = BannerInstructionContent {
            text: text.unwrap_or_default(),
            components: vec![text_component], // TODO
            banner_maneuver,
            degrees: None,
            driving_side: None,
        };
        let instruction = BannerInstruction {
            distance_along_geometry: maneuver.distance_meters(from_distance_unit),
            primary,
        };
        Some(vec![instruction])
    }
}

// REVIEW: Rename to VisualInstructionBanner?
// How do audible instructions fit into this?
#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BannerInstructionContent {
    pub text: String,
    // components: Vec<BannerInstructionContentComponent>,
    #[serde(flatten)]
    pub banner_maneuver: Option<BannerManeuver>,
    pub degrees: Option<f64>,
    pub driving_side: Option<String>,
    pub components: Vec<BannerComponent>,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub struct BannerManeuver {
    pub r#type: BannerManeuverType,
    pub modifier: Option<BannerManeuverModifier>,
}

// This is for `banner.primary(et. al).type`
// There is a lot of overlap between this and `step_maneuver.type`,
// but the docs imply they are different.
#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BannerManeuverType {
    Turn,
    Merge,
    Depart,
    Arrive,
    Fork,
    #[serde(rename = "on ramp")]
    OnRamp,
    #[serde(rename = "off ramp")]
    OffRamp,
    #[serde(rename = "roundabout")]
    RoundaboutEnter,
    #[serde(rename = "exit roundabout")]
    RoundaboutExit,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BannerManeuverModifier {
    Uturn,
    #[serde(rename = "sharp right")]
    SharpRight,
    Right,
    #[serde(rename = "slight right")]
    SlightRight,
    Straight,
    #[serde(rename = "slight left")]
    SlightLeft,
    Left,
    #[serde(rename = "sharp left")]
    SharpLeft,
}

// REVIEW: Rename to VisualInstruction?
// REVIEW: convert to inner enum of Lane or VisualInstructionComponent
#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", tag = "type")]
#[non_exhaustive]
pub enum BannerComponent {
    Text(VisualInstructionComponent),
    // Icon(VisualInstructionComponent),
    // Delimiter(VisualInstructionComponent),
    // #[serde(rename="exit-number")]
    // ExitNumber(VisualInstructionComponent),
    // Exit(VisualInstructionComponent),
    Lane(LaneInstructionComponent),
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LaneInstructionComponent {}

// Maybe we won't use this? Because it'll need to be implicit in the containing BannerComponent enum variant of
#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum VisualInstructionComponentType {
    /// The component separates two other destination components.
    ///
    /// If the two adjacent components are both displayed as images, you can hide this delimiter component.
    Delimiter,

    /// The component bears the name of a place or street.
    Text,

    /// Component contains an image that should be rendered.
    Image,

    /// The component contains the localized word for "exit".
    ///
    /// This component may appear before or after an `.ExitCode` component, depending on the language.
    Exit,

    /// A component contains an exit number.
    #[serde(rename = "exit-number")]
    ExitCode,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VisualInstructionComponent {
    pub(crate) text: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StepManeuver {
    /// The location of the maneuver
    #[serde(serialize_with = "serialize_point_as_lon_lat_pair")]
    pub location: Point,
    // /// The type of maneuver. new identifiers might be introduced without changing the API, so best practice is to gracefully handle any new values
    // r#type: String,
    // /// The modifier of the maneuver. new identifiers might be introduced without changing the API, so best practice is to gracefully handle any new values
    // modifier: String,
    // /// A human-readable instruction of how to execute the returned maneuver
    // instruction: String,
    // /// The bearing before the turn, in degrees
    // OSRM expects `bearing_before` to be snake cased.
    // #[serde(rename_all = "snake_case")]
    // bearing_before: f64,
    // OSRM expects `bearing_after` to be snake cased.
    // /// The bearing after the turn, in degrees
    // bearing_after: f64,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Intersection {
    /// A [longitude, latitude] pair describing the location of the turn.
    #[serde(serialize_with = "serialize_point_as_lon_lat_pair")]
    pub location: Point,

    /// A list of bearing values that are available at the intersection. The bearings describe all available roads at the intersection.
    pub bearings: Vec<f64>,

    // /// An array of strings signifying the classes of the road exiting the intersection.
    // pub classes: Vec<RoadClass>
    /// A list of entry flags, corresponding in a 1:1 relationship to the bearings. A value of true indicates that the respective road could be entered on a valid route.
    pub entry: Vec<bool>,

    /// The zero-based index into the geometry, relative to the start of the leg it's on. This value can be used to apply the duration annotation that corresponds with the intersection.
    pub geometry_index: Option<usize>,

    /// The index in the bearings and entry arrays. Used to calculate the bearing before the turn. Namely, the clockwise angle from true north to the direction of travel before the maneuver/passing the intersection. To get the bearing in the direction of driving, the bearing has to be rotated by a value of 180. The value is not supplied for departure maneuvers.
    pub r#in: Option<usize>,

    /// The index in the bearings and entry arrays. Used to extract the bearing after the turn. Namely, the clockwise angle from true north to the direction of travel after the maneuver/passing the intersection. The value is not supplied for arrival maneuvers.
    pub out: Option<usize>,

    /// An array of lane objects that represent the available turn lanes at the intersection. If no lane information is available for an intersection, the lanes property will not be present.
    pub lanes: Vec<Lane>,

    /// The time required, in seconds, to traverse the intersection. Only available on the driving profile.
    pub duration: Option<f64>,
    // TODO: lots more fields in OSRM
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Lane {
    // TODO: lots more fields in OSRM
}
