//! "osrm_api" is a bit of a misnomer. It's intended to work with maplibre's  "Directions" library.
//! which is strongly influenced by OSRM.

use super::plan::{Itinerary, Leg, Maneuver, ModeLeg};
use crate::util::serde_util::{
    serialize_line_string_as_polyline6, serialize_point_as_lon_lat_pair,
};
use crate::util::{bearing_at_end, bearing_at_start};
use crate::valhalla::valhalla_api::ManeuverType;
use crate::{DistanceUnit, TravelMode};
use geo::{LineString, Point};
use serde::Serialize;

/// The route between waypoints.
#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    /// The distance traveled by the route, in float meters.
    pub distance: f64,

    /// The estimated travel time, in float number of seconds.
    pub duration: f64,

    // TODO: simplify?
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

                debug_assert!(non_transit_leg.maneuvers.len() >= 2);
                let mut steps = Vec::with_capacity(non_transit_leg.maneuvers.len());
                if let Some(first_maneuver) = non_transit_leg.maneuvers.first() {
                    let first_step = RouteStep::from_maneuver(
                        None,
                        first_maneuver.clone(),
                        non_transit_leg.maneuvers.get(1),
                        value.mode,
                        distance_unit,
                    );
                    steps.push(first_step);
                }

                let middle_steps = non_transit_leg.maneuvers.windows(3).map(|maneuvers| {
                    let prev_maneuver = &maneuvers[0];
                    let maneuver = maneuvers[1].clone();
                    let next_maneuver = &maneuvers[2];

                    RouteStep::from_maneuver(
                        Some(prev_maneuver),
                        maneuver,
                        Some(next_maneuver),
                        value.mode,
                        distance_unit,
                    )
                });
                steps.extend(middle_steps);

                if let Some(final_maneuver) = non_transit_leg.maneuvers.last() {
                    let prev_maneuver = if non_transit_leg.maneuvers.len() < 2 {
                        None
                    } else {
                        non_transit_leg
                            .maneuvers
                            .get(non_transit_leg.maneuvers.len() - 2)
                    };
                    let final_step = RouteStep::from_maneuver(
                        prev_maneuver,
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
    pub maneuver: RouteStepManeuver,

    /// A list of `BannerInstruction` objects that represent all signs on the step.
    pub banner_instructions: Option<Vec<VisualInstructionBanner>>,

    /// A list of `Intersection` objects that are passed along the segment, the very first belonging to the `StepManeuver`
    pub intersections: Option<Vec<Intersection>>,
}

impl RouteStep {
    fn from_maneuver(
        prev_maneuver: Option<&Maneuver>,
        maneuver: Maneuver,
        next_maneuver: Option<&Maneuver>,
        mode: TravelMode,
        from_distance_unit: DistanceUnit,
    ) -> Self {
        let banner_instructions =
            VisualInstructionBanner::from_maneuver(&maneuver, next_maneuver, from_distance_unit);

        let bearing_after = bearing_at_start(&maneuver.geometry).unwrap_or(0);
        let bearing_before = prev_maneuver
            .and_then(|prev_maneuver| bearing_at_end(&prev_maneuver.geometry))
            .unwrap_or(bearing_after);

        RouteStep {
            distance: maneuver.distance_meters(from_distance_unit),
            duration: maneuver.duration_seconds,
            geometry: maneuver.geometry,
            name: maneuver
                .street_names
                .unwrap_or(vec!["".to_string()])
                .join(", "),
            r#ref: None,         // TODO
            pronunciation: None, // TODO
            destinations: None,  // TODO
            mode,
            maneuver: RouteStepManeuver {
                location: maneuver.start_point.into(),
                bearing_before,
                bearing_after,
            },
            intersections: None, // TODO
            banner_instructions,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VisualInstructionBanner {
    pub distance_along_geometry: f64,
    pub primary: VisualInstruction,
    // secondary: Option<BannerInstructionContent>, // TODO
    // sub: Option<BannerInstructionContent>, // TODO
}

impl VisualInstructionBanner {
    fn from_maneuver(
        maneuver: &Maneuver,
        next_maneuver: Option<&Maneuver>,
        from_distance_unit: DistanceUnit,
    ) -> Option<Vec<Self>> {
        let text_components = if let Some(next_maneuver) = next_maneuver {
            next_maneuver
                .street_names
                .as_ref()
                .cloned()
                .or(next_maneuver.instruction.as_ref().map(|s| vec![s.clone()]))
        } else {
            assert!(matches!(
                maneuver.r#type,
                ManeuverType::Destination
                    | ManeuverType::DestinationRight
                    | ManeuverType::DestinationLeft
            ));
            maneuver.instruction.as_ref().map(|s| vec![s.clone()])
        };

        let (maneuver_type, maneuver_direction): (
            Option<OSRMManeuverType>,
            Option<ManeuverDirection>,
        ) = (|| {
            use ManeuverDirection::*;
            use OSRMManeuverType::*;
            let (maneuver_type, maneuver_direction) = match next_maneuver.unwrap_or(maneuver).r#type
            {
                ManeuverType::None => return (None, None),
                ManeuverType::Start => (Depart, None),
                ManeuverType::StartRight => (Depart, Some(Right)),
                ManeuverType::StartLeft => (Depart, Some(Left)),
                ManeuverType::Destination => (Arrive, None),
                ManeuverType::DestinationRight => (Arrive, Some(Right)),
                ManeuverType::DestinationLeft => (Arrive, Some(Left)),
                ManeuverType::Becomes => (NewName, None),
                ManeuverType::Continue => (Continue, None),
                ManeuverType::SlightRight => (Turn, Some(SlightRight)),
                ManeuverType::Right => (Turn, Some(Right)),
                ManeuverType::SharpRight => (Turn, Some(SharpRight)),
                ManeuverType::UturnRight => (Turn, Some(Uturn)),
                ManeuverType::UturnLeft => (Turn, Some(Uturn)),
                ManeuverType::SharpLeft => (Turn, Some(SharpLeft)),
                ManeuverType::Left => (Turn, Some(Left)),
                ManeuverType::SlightLeft => (Turn, Some(SlightLeft)),
                ManeuverType::RampStraight => (OnRamp, Some(Straight)),
                ManeuverType::RampRight => (OnRamp, Some(Right)),
                ManeuverType::RampLeft => (OnRamp, Some(Left)),
                ManeuverType::ExitRight => (OffRamp, Some(Right)),
                ManeuverType::ExitLeft => (OffRamp, Some(Left)),
                ManeuverType::StayStraight => (Fork, Some(Straight)),
                ManeuverType::StayRight => (Fork, Some(Right)),
                ManeuverType::StayLeft => (Fork, Some(Left)),
                ManeuverType::Merge => (Merge, None),
                ManeuverType::RoundaboutEnter => (RoundaboutEnter, None),
                ManeuverType::RoundaboutExit => (RoundaboutExit, None),
                ManeuverType::FerryEnter => (Notification, None),
                ManeuverType::FerryExit => (Notification, None),
                ManeuverType::Transit => (Notification, None),
                ManeuverType::TransitTransfer => (Notification, None),
                ManeuverType::TransitRemainOn => (Notification, None),
                ManeuverType::TransitConnectionStart => (Notification, None),
                ManeuverType::TransitConnectionTransfer => (Notification, None),
                ManeuverType::TransitConnectionDestination => (Notification, None),
                ManeuverType::PostTransitConnectionDestination => (Notification, None),
                ManeuverType::MergeRight => (Merge, Some(Right)),
                ManeuverType::MergeLeft => (Merge, Some(Left)),
                ManeuverType::ElevatorEnter => (Notification, None),
                ManeuverType::StepsEnter => (Notification, None),
                ManeuverType::EscalatorEnter => (Notification, None),
                ManeuverType::BuildingEnter => (Notification, None),
                ManeuverType::BuildingExit => (Notification, None),
            };
            (Some(maneuver_type), maneuver_direction)
        })();

        let text = text_components.as_ref().map(|t| t.join("/"));

        let components: Vec<BannerComponent> = (|text_components: Option<Vec<String>>| {
            let text_components = text_components?;
            let mut text_iter = text_components.into_iter();
            let first_text = text_iter.next()?;

            let mut output = vec![BannerComponent::Text(VisualInstructionComponent {
                text: Some(first_text),
            })];
            for next_text in text_iter {
                output.push(BannerComponent::Delimiter(VisualInstructionComponent {
                    text: Some("/".to_string()),
                }));
                output.push(BannerComponent::Text(VisualInstructionComponent {
                    text: Some(next_text),
                }));
            }
            Some(output)
        })(text_components)
        .unwrap_or(
            // REVIEW: not sure if we need this default or if an empty component list is OK.
            vec![BannerComponent::Text(VisualInstructionComponent {
                text: None,
            })],
        );

        let primary = VisualInstruction {
            text: text.unwrap_or_default(),
            components,
            maneuver_type,
            maneuver_direction,
            degrees: None,
            driving_side: None,
        };
        let instruction = VisualInstructionBanner {
            distance_along_geometry: maneuver.distance_meters(from_distance_unit),
            primary,
        };
        Some(vec![instruction])
    }
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VisualInstruction {
    pub text: String,
    #[serde(rename = "type")]
    pub maneuver_type: Option<OSRMManeuverType>,
    #[serde(rename = "modifier")]
    pub maneuver_direction: Option<ManeuverDirection>,
    pub degrees: Option<f64>,
    pub driving_side: Option<String>,
    pub components: Vec<BannerComponent>,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OSRMManeuverType {
    /// The step requires the user to turn.
    ///
    /// The maneuver direction indicates the direction in which the user must turn relative to the current direction of travel.
    /// The exit index indicates the number of intersections, large or small, from the previous maneuver up to and including the intersection at which the user must turn.
    Turn,

    /// The step requires the user to continue on the current road as it changes names.
    ///
    /// The step’s name contains the road’s new name. To get the road’s old name, use the previous step’s name.
    #[serde(rename = "new name")]
    NewName,

    /// The step requires the user to merge onto another road.
    ///
    /// The maneuver direction indicates the side from which the other road approaches the intersection relative to the user.
    Merge,

    /// The step requires the user to depart from a waypoint.
    ///
    /// If the waypoint is some distance away from the nearest road, the maneuver direction indicates the direction the user must turn upon reaching the road.
    Depart,

    /// Indicates arrival to a destination of a leg. The modifier value indicates the position of the arrival point compared to the current direction of travel.
    Arrive,

    /// Keep left or right side at a bifurcation, or left/right/straight at a trifurcation.
    Fork,

    /// The step requires the user to take a entrance ramp (slip road) onto a highway.
    #[serde(rename = "on ramp")]
    OnRamp,

    /// The step requires the user to take an exit ramp (slip road) off a highway.
    ///
    /// The maneuver direction indicates the side of the highway from which the user must exit.
    /// The exit index indicates the number of highway exits from the previous maneuver up to and including the exit that the user must take.
    #[serde(rename = "off ramp")]
    OffRamp,

    /// The step requires the user to turn at either a T-shaped three-way intersection or a sharp bend in the road where the road also changes names.
    ///
    /// This maneuver type is called out separately so that the user may be able to proceed more confidently, without fear of having overshot the turn. If this distinction is unimportant to you, you may treat the maneuver as an ordinary `turn`.
    #[allow(unused)]
    #[serde(rename = "end of road")]
    EndOfRoad,

    /// The step requires the user to get into a specific lane in order to continue along the current road.
    /// The maneuver direction is set to `straightAhead`. Each of the first intersection’s usable approach lanes also has an indication of `straightAhead`. A maneuver in a different direction would instead have a maneuver type of `turn`.
    //
    /// This maneuver type is called out separately so that the application can present the user with lane guidance based on the first element in the `intersections` property. If lane guidance is unimportant to you, you may treat the maneuver as an ordinary `continue` or ignore it.
    #[serde(rename = "use lane")]
    #[allow(unused)]
    UseLane,

    /// Continue on a street after a turn.
    Continue,

    /// Traverse roundabout. Has an additional property exit in the route step that contains the exit number. The modifier specifies the direction of entering the roundabout.
    #[serde(rename = "roundabout")]
    RoundaboutEnter,

    /// Indicates the exit maneuver from a roundabout. Will not appear in results unless you supply the roundabout_exits=true query parameter in the request.
    #[serde(rename = "exit roundabout")]
    RoundaboutExit,

    /// A traffic circle. While like a larger version of a roundabout, it does not necessarily follow roundabout rules for right of way. It can offer rotary_name parameters, rotary_pronunciation parameters, or both, located in the route step object. It also contains the exit property.
    #[allow(unused)]
    #[serde(rename = "rotary")]
    RotaryEnter,

    #[allow(unused)]
    #[serde(rename = "exit rotary")]
    RotaryExit,

    /// The step requires the user to enter and exit a roundabout (traffic circle or rotary) that is compact enough to constitute a single intersection.
    ///
    ///  The step’s name is the name of the road to take after exiting the roundabout.
    /// This maneuver type is called out separately because the user may perceive the roundabout as an ordinary intersection with an island in the middle.
    /// If this distinction is unimportant to you, you may treat the maneuver as either an ordinary `turn` or as a `takeRoundabout`.
    #[allow(unused)]
    #[serde(rename = "roundabout turn")]
    RoundaboutTurn,

    /// The step requires the user to respond to a change in travel conditions.
    ///
    /// This maneuver type may occur for example when driving directions require the user to board a ferry, or when cycling directions require the user to dismount.
    /// The step’s transport type and instructions contains important contextual details that should be presented to the user at the maneuver location.
    ///
    /// Similar changes can occur simultaneously with other maneuvers, such as when the road changes its name at the site of a movable bridge.
    /// In such cases, `notification` is suppressed in favor of another maneuver type.
    Notification,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ManeuverDirection {
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

/// `BannerComponent` is kind of a corollary to maplibre-navigation-ios's `ComponentRepresentable`
/// protocol, which has a `type: VisualInstructionComponentType` field. Here however, we have a
/// variant for each `type` with the associated value of the `VisualInstructionComponent`.
/// Plus we have enum variant for the other implementors of `ComponentRepresentable`
/// (really, that's only `Lane`, so far)
#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase", tag = "type")]
#[non_exhaustive]
pub enum BannerComponent {
    /// The component bears the name of a place or street.
    Text(VisualInstructionComponent),

    /// The component separates two other destination components.
    ///
    /// If the two adjacent components are both displayed as images, you can hide this delimiter component.
    Delimiter(VisualInstructionComponent),

    #[allow(unused)] // TODO
    Lane(LaneIndicationComponent),
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct LaneIndicationComponent {}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VisualInstructionComponent {
    pub(crate) text: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RouteStepManeuver {
    /// The location of the maneuver
    #[serde(serialize_with = "serialize_point_as_lon_lat_pair")]
    pub location: Point,
    // /// The type of maneuver. new identifiers might be introduced without changing the API, so best practice is to gracefully handle any new values
    // r#type: ManeuverType,
    // /// The modifier of the maneuver. new identifiers might be introduced without changing the API, so best practice is to gracefully handle any new values
    // modifier: ManeuverDirection,
    // /// A human-readable instruction of how to execute the returned maneuver
    // instruction: String,
    /// The bearing before the turn, in degrees
    #[serde(rename = "bearing_before")] // OSRM expects `bearing_before` to be snake cased.
    bearing_before: u16,

    /// The bearing after the turn, in degrees
    #[serde(rename = "bearing_after")] // OSRM expects `bearing_after` to be snake cased.
    bearing_after: u16,
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
