//! Client for OTP's GTFS GraphQL API (`/otp/gtfs/v1`).
//!
//! OTP removed its REST `/plan` endpoint in 2.8, so we talk to it over GraphQL now. The query is
//! described by the `cynic` types in this module, which are checked against
//! [`crate::otp::schema`] at compile time.
//!
//! There are two ways to read a response:
//!
//! - [`plan_connection`] hands back these GraphQL types as they are. This is what the v7 API
//!   builds on.
//! - [`plan`] maps them into the legacy [`crate::otp::otp_api`] types, which have the shape of
//!   OTP's old REST response. That's what the v6 API - and, through its `_otp` passthrough, v6's
//!   clients - still expect.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::Tz;
use cynic::http::ReqwestExt;
use cynic::{Operation, QueryBuilder};
use geo::geometry::Point;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

use crate::otp::otp_api;
use crate::otp::schema;
// NB: `Result`/`Error` are deliberately *not* imported here: cynic's derives expand to code
// using `std::result::Result` unqualified, which our aliases would shadow.
use crate::TravelMode;

// ===
// Request
// ===

/// Everything needed to build a `planConnection` query.
pub struct PlanParams<'a> {
    pub from: Point,
    pub to: Point,
    /// The requested travel modes. The first is the "primary" mode; a following `Bicycle` with a
    /// primary of `Transit` means bike+transit.
    pub modes: &'a [TravelMode],
    pub num_itineraries: u32,
    /// When the traveler wants to depart (or arrive, if `arrive_by`), or `None` to plan from now.
    pub date_time: Option<PlanDateTime>,
    /// Whether `date_time` describes the desired arrival (rather than departure) time.
    pub arrive_by: bool,
    /// The graph's timezone, used to resolve a local [`PlanDateTime`] into an absolute instant.
    pub timezone: Option<Tz>,
}

/// When the traveler wants to travel.
///
/// Clients that know their traveler's offset from UTC can say exactly which instant they mean;
/// clients picking a time "at the destination" generally can't, since they don't know the
/// timezone of the graph serving the trip. Both are accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanDateTime {
    /// An unambiguous instant, e.g. "2024-06-13T14:30:00-07:00".
    Absolute(DateTime<FixedOffset>),
    /// A wall-clock time, e.g. "2024-06-13T14:30", to be interpreted in the graph's timezone.
    Local(NaiveDateTime),
}

impl PlanDateTime {
    /// A local time from the separate date ("YYYY-MM-DD") and time ("HH:MM") the v6 API takes.
    pub fn from_local_date_and_time(date: &str, time: &str) -> Option<Self> {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
        let time = NaiveTime::parse_from_str(time, "%H:%M")
            .or_else(|_| NaiveTime::parse_from_str(time, "%H:%M:%S"))
            .ok()?;
        Some(Self::Local(NaiveDateTime::new(date, time)))
    }

    /// Resolve to an absolute instant, interpreting a local time in `timezone`.
    ///
    /// `None` if a local time was given without a timezone to interpret it in, or if it names a
    /// wall-clock time that doesn't exist there (the hour skipped when DST begins).
    fn resolve(&self, timezone: Option<Tz>) -> Option<DateTime<FixedOffset>> {
        match self {
            Self::Absolute(date_time) => Some(*date_time),
            Self::Local(naive) => {
                let Some(timezone) = timezone else {
                    log::warn!(
                        "requested a local plan time but the graph's timezone is unknown; \
                         falling back to 'now'"
                    );
                    return None;
                };
                // `.earliest()`/`.latest()` gracefully handle DST ambiguities.
                let local = timezone.from_local_datetime(naive);
                let resolved = local.earliest().or_else(|| local.latest());
                if resolved.is_none() {
                    log::warn!("plan time {naive:?} does not exist in timezone {timezone:?}");
                }
                resolved.map(|resolved| resolved.fixed_offset())
            }
        }
    }
}

impl std::str::FromStr for PlanDateTime {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match DateTime::parse_from_rfc3339(s) {
            Ok(absolute) => Ok(Self::Absolute(absolute)),
            // Without an offset (or a trailing `Z`) it's a wall-clock time. Seconds are optional.
            Err(no_offset) => NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
                .map(Self::Local)
                // Report the RFC 3339 failure: it's the form clients ought to be sending.
                .map_err(|_| no_offset),
        }
    }
}

impl<'de> Deserialize<'de> for PlanDateTime {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(|e| {
            D::Error::custom(format!(
                "expected an RFC 3339 datetime like 2024-06-13T14:30:00-07:00, \
                 or a local one like 2024-06-13T14:30: {e}"
            ))
        })
    }
}

#[derive(cynic::QueryVariables, Debug)]
struct PlanVariables {
    origin: PlanLabeledLocationInput,
    destination: PlanLabeledLocationInput,
    first: Option<i32>,
    date_time: Option<PlanDateTimeInput>,
    modes: Option<PlanModesInput>,
}

#[derive(cynic::InputObject, Debug)]
struct PlanLabeledLocationInput {
    location: PlanLocationInput,
}

#[derive(cynic::InputObject, Debug)]
enum PlanLocationInput {
    Coordinate(PlanCoordinateInput),
}

#[derive(cynic::InputObject, Debug)]
struct PlanCoordinateInput {
    latitude: f64,
    longitude: f64,
}

/// `@oneOf`: exactly one of these may be given, which the Rust enum enforces for us.
#[derive(cynic::InputObject, Debug)]
enum PlanDateTimeInput {
    EarliestDeparture(DateTime<FixedOffset>),
    LatestArrival(DateTime<FixedOffset>),
}

#[derive(cynic::InputObject, Debug)]
struct PlanModesInput {
    #[cynic(skip_serializing_if = "Option::is_none")]
    direct: Option<Vec<PlanDirectMode>>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    direct_only: Option<bool>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    transit: Option<PlanTransitModesInput>,
}

#[derive(cynic::InputObject, Debug)]
struct PlanTransitModesInput {
    access: Option<Vec<PlanAccessMode>>,
    egress: Option<Vec<PlanEgressMode>>,
    transfer: Option<Vec<PlanTransferMode>>,
}

// OTP has a separate street-mode enum per phase of a trip (direct, access, egress, transfer), each
// with many more variants than the plain walking/cycling/driving we ask for. Each is declared
// `non_exhaustive` so we only have to name the modes we actually use; the fallback variants exist
// to satisfy that and are never constructed.

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(non_exhaustive)]
enum PlanDirectMode {
    Walk,
    Bicycle,
    Car,
    #[cynic(fallback)]
    Unused,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(non_exhaustive)]
enum PlanAccessMode {
    Walk,
    Bicycle,
    #[cynic(fallback)]
    Unused,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(non_exhaustive)]
enum PlanEgressMode {
    Walk,
    Bicycle,
    #[cynic(fallback)]
    Unused,
}

#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(non_exhaustive)]
enum PlanTransferMode {
    Walk,
    Bicycle,
    #[cynic(fallback)]
    Unused,
}

impl PlanParams<'_> {
    fn primary_mode(&self) -> TravelMode {
        self.modes.first().copied().unwrap_or(TravelMode::Transit)
    }

    /// Whether this is a "direct" (non-transit) search - i.e. plain walking or cycling.
    fn is_direct(&self) -> bool {
        !matches!(self.primary_mode(), TravelMode::Transit)
    }

    fn modes_input(&self) -> PlanModesInput {
        let direct = |mode| PlanModesInput {
            direct: Some(vec![mode]),
            direct_only: Some(true),
            transit: None,
        };

        match self.primary_mode() {
            TravelMode::Walk => direct(PlanDirectMode::Walk),
            TravelMode::Bicycle => direct(PlanDirectMode::Bicycle),
            TravelMode::Car => direct(PlanDirectMode::Car),
            TravelMode::Transit => {
                // bike+transit: access/egress/transfer must all agree on BICYCLE for OTP to
                // consider cycling to/from and between stops.
                let bike = self.modes.contains(&TravelMode::Bicycle);
                PlanModesInput {
                    direct: None,
                    direct_only: None,
                    transit: Some(PlanTransitModesInput {
                        access: Some(vec![if bike {
                            PlanAccessMode::Bicycle
                        } else {
                            PlanAccessMode::Walk
                        }]),
                        egress: Some(vec![if bike {
                            PlanEgressMode::Bicycle
                        } else {
                            PlanEgressMode::Walk
                        }]),
                        transfer: Some(vec![if bike {
                            PlanTransferMode::Bicycle
                        } else {
                            PlanTransferMode::Walk
                        }]),
                    }),
                }
            }
        }
    }

    /// The absolute instant OTP wants, or `None` to let OTP plan from "now".
    fn date_time_input(&self) -> Option<PlanDateTimeInput> {
        let resolved = self.date_time?.resolve(self.timezone)?;
        Some(if self.arrive_by {
            PlanDateTimeInput::LatestArrival(resolved)
        } else {
            PlanDateTimeInput::EarliestDeparture(resolved)
        })
    }

    fn build_query(&self) -> Operation<PlanConnectionQuery, PlanVariables> {
        let first = if self.is_direct() {
            // For plain walking/cycling OTP returns a single itinerary, and downstream code
            // assumes exactly one.
            1
        } else {
            self.num_itineraries.try_into().unwrap_or(i32::MAX)
        };

        PlanConnectionQuery::build(PlanVariables {
            origin: location(self.from),
            destination: location(self.to),
            first: Some(first),
            date_time: self.date_time_input(),
            modes: Some(self.modes_input()),
        })
    }
}

fn location(point: Point) -> PlanLabeledLocationInput {
    PlanLabeledLocationInput {
        location: PlanLocationInput::Coordinate(PlanCoordinateInput {
            latitude: point.y(),
            longitude: point.x(),
        }),
    }
}

// ===
// HTTP
// ===

/// The path OTP serves its GTFS GraphQL API from, relative to an instance's base URL.
const GRAPHQL_PATH: [&str; 3] = ["otp", "gtfs", "v1"];

/// The GraphQL endpoint for an OTP instance's base URL.
///
/// e.g. `http://opentripplanner:8000` -> `http://opentripplanner:8000/otp/gtfs/v1`
pub(crate) fn endpoint_url(base_url: &Url) -> crate::Result<Url> {
    let mut endpoint = base_url.clone();
    endpoint
        .path_segments_mut()
        .map_err(|_| {
            crate::Error::server(format!("OTP base url must be a valid base: {base_url}"))
        })?
        .pop_if_empty()
        .extend(GRAPHQL_PATH);
    Ok(endpoint)
}

/// POST a GraphQL operation to `endpoint` and return the deserialized `data` payload.
pub(crate) async fn post_graphql<Data, Variables>(
    client: &reqwest::Client,
    endpoint: &Url,
    operation: Operation<Data, Variables>,
) -> crate::Result<Data>
where
    Data: DeserializeOwned + 'static,
    Variables: Serialize,
{
    let response = client.post(endpoint.clone()).run_graphql(operation).await?;

    if let Some(errors) = response.errors.filter(|errors| !errors.is_empty()) {
        let messages: Vec<_> = errors.into_iter().map(|e| e.message).collect();
        return Err(crate::Error::server(format!(
            "OTP GraphQL returned errors: {}",
            messages.join("; ")
        )));
    }

    response
        .data
        .ok_or_else(|| crate::Error::server("OTP GraphQL response had no data"))
}

/// Read a `planConnection` response body from parsed JSON.
#[cfg(test)]
pub(crate) fn plan_result_from_json(body: serde_json::Value) -> PlanResult {
    let envelope: cynic::GraphQlResponse<PlanConnectionQuery> =
        serde_json::from_value(body).expect("a planConnection response");
    envelope.data.expect("response had no data").into_result()
}

/// Read a `planConnection` response captured in a file by `tests/fixtures/requests/refresh.sh`.
#[cfg(test)]
pub(crate) fn plan_result_from_fixture(path: &str) -> PlanResult {
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("opening {path}: {e}"));
    let envelope: cynic::GraphQlResponse<PlanConnectionQuery> =
        serde_json::from_reader(std::io::BufReader::new(file))
            .unwrap_or_else(|e| panic!("parsing {path}: {e}"));
    envelope
        .data
        .unwrap_or_else(|| panic!("{path} had no data"))
        .into_result()
}

/// Execute a `planConnection` query.
pub async fn plan_connection(
    client: &reqwest::Client,
    endpoint: &Url,
    params: &PlanParams<'_>,
) -> crate::Result<PlanResult> {
    let data: PlanConnectionQuery = post_graphql(client, endpoint, params.build_query()).await?;
    Ok(data.into_result())
}

/// Execute a `planConnection` query and map the result into the legacy [`otp_api::PlanResponse`].
pub async fn plan(
    client: &reqwest::Client,
    endpoint: &Url,
    params: &PlanParams<'_>,
) -> crate::Result<otp_api::PlanResponse> {
    Ok(plan_connection(client, endpoint, params).await?.into_otp())
}

// ===
// Response
// ===

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "QueryType", variables = "PlanVariables")]
struct PlanConnectionQuery {
    #[arguments(
        origin: $origin,
        destination: $destination,
        first: $first,
        dateTime: $date_time,
        modes: $modes
    )]
    plan_connection: Option<PlanConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
struct PlanConnection {
    edges: Option<Vec<Option<PlanEdge>>>,
    routing_errors: Vec<RoutingError>,
}

#[derive(cynic::QueryFragment, Debug)]
struct PlanEdge {
    node: Itinerary,
}

/// What a `planConnection` query came back with: itineraries, or the reasons there are none.
#[derive(Debug)]
pub struct PlanResult {
    pub itineraries: Vec<Itinerary>,
    /// Why OTP couldn't plan the trip. Only meaningful when `itineraries` is empty.
    pub routing_errors: Vec<RoutingError>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Itinerary {
    pub start: Option<DateTime<FixedOffset>>,
    pub end: Option<DateTime<FixedOffset>>,
    /// seconds
    pub duration: Option<i64>,
    /// meters
    pub walk_distance: Option<f64>,
    pub legs: Vec<Option<Leg>>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Leg {
    pub mode: Option<Mode>,
    pub transit_leg: Option<bool>,
    /// meters
    pub distance: Option<f64>,
    /// seconds
    pub duration: Option<f64>,
    pub real_time: Option<bool>,
    pub headsign: Option<String>,
    pub start: LegTime,
    pub end: LegTime,
    pub from: Place,
    pub to: Place,
    pub leg_geometry: Option<Geometry>,
    pub route: Option<Route>,
    pub agency: Option<Agency>,
    pub steps: Option<Vec<Option<Step>>>,
    pub alerts: Option<Vec<Option<Alert>>>,
}

impl Leg {
    /// Whether this is a transit leg, as opposed to a walk/bike/car access leg.
    pub fn is_transit(&self) -> bool {
        self.transit_leg.unwrap_or(!matches!(
            self.mode,
            Some(Mode::Walk) | Some(Mode::Bicycle) | Some(Mode::Car)
        ))
    }
}

#[derive(cynic::QueryFragment, Debug)]
pub struct LegTime {
    pub scheduled_time: DateTime<FixedOffset>,
    pub estimated: Option<RealTimeEstimate>,
}

impl LegTime {
    /// The realtime-estimated instant if available, otherwise the scheduled one.
    pub fn best_estimate(&self) -> DateTime<FixedOffset> {
        self.estimated
            .as_ref()
            .map_or(self.scheduled_time, |estimate| estimate.time)
    }

    /// [`Self::best_estimate`] in millis since the Unix epoch.
    fn millis(&self) -> Option<u64> {
        millis(self.best_estimate())
    }
}

#[derive(cynic::QueryFragment, Debug)]
pub struct RealTimeEstimate {
    pub time: DateTime<FixedOffset>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Place {
    pub name: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub arrival: Option<LegTime>,
    pub departure: Option<LegTime>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Geometry {
    /// Encoded polyline, 1e-5 scale, (lat, lon)
    pub points: Option<String>,
    pub length: Option<i32>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Route {
    pub short_name: Option<String>,
    pub long_name: Option<String>,
    pub color: Option<String>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Agency {
    pub name: String,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "step")]
pub struct Step {
    /// meters
    pub distance: Option<f64>,
    pub relative_direction: Option<RelativeDirection>,
    pub absolute_direction: Option<AbsoluteDirection>,
    pub street_name: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub area: Option<bool>,
    /// The name of this street was generated by the system, so we should only display it once, and
    /// generally just give right/left directions.
    pub bogus_name: Option<bool>,
    pub stay_on: Option<bool>,
    pub exit: Option<String>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Alert {
    pub alert_header_text: Option<String>,
    pub alert_description_text: String,
    pub alert_url: Option<String>,
    /// Unix timestamp, in seconds
    pub effective_start_date: Option<i64>,
    /// Unix timestamp, in seconds
    pub effective_end_date: Option<i64>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct RoutingError {
    pub code: RoutingErrorCode,
    pub description: String,
}

/// The direction a [`Step`] turns, relative to the direction of travel.
#[derive(Debug, PartialEq, Clone, Copy, cynic::Enum)]
pub enum RelativeDirection {
    Depart,
    HardLeft,
    Left,
    SlightlyLeft,
    Continue,
    SlightlyRight,
    Right,
    HardRight,
    CircleClockwise,
    CircleCounterclockwise,
    Elevator,
    UturnLeft,
    UturnRight,
    // Station entrances / signage. We don't produce turn-by-turn instructions for transit legs, so
    // these just need to round-trip.
    EnterStation,
    ExitStation,
    FollowSigns,
}

impl From<RelativeDirection> for crate::valhalla::valhalla_api::ManeuverType {
    fn from(otp: RelativeDirection) -> Self {
        use crate::valhalla::valhalla_api::ManeuverType;
        match otp {
            RelativeDirection::Depart => ManeuverType::Start,
            RelativeDirection::HardLeft => ManeuverType::SharpLeft,
            RelativeDirection::Left => ManeuverType::Left,
            RelativeDirection::SlightlyLeft => ManeuverType::SlightLeft,
            RelativeDirection::Continue => ManeuverType::Continue,
            RelativeDirection::SlightlyRight => ManeuverType::SlightRight,
            RelativeDirection::Right => ManeuverType::Right,
            RelativeDirection::HardRight => ManeuverType::SharpRight,
            RelativeDirection::CircleClockwise => ManeuverType::RoundaboutEnter,
            RelativeDirection::CircleCounterclockwise => ManeuverType::RoundaboutEnter,
            RelativeDirection::Elevator => ManeuverType::ElevatorEnter,
            RelativeDirection::UturnLeft => ManeuverType::UturnLeft,
            RelativeDirection::UturnRight => ManeuverType::UturnRight,
            // These only occur on transit legs, where we don't emit turn-by-turn maneuvers, but
            // map them to something sensible for completeness.
            RelativeDirection::EnterStation
            | RelativeDirection::ExitStation
            | RelativeDirection::FollowSigns => ManeuverType::Continue,
        }
    }
}

/// The compass direction a [`Step`] heads in.
#[derive(Debug, PartialEq, Clone, Copy, cynic::Enum)]
pub enum AbsoluteDirection {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest,
}

/// GTFS GraphQL leg `Mode` enum: how the traveler gets along a leg, and for transit legs, what
/// kind of vehicle they're on.
#[derive(cynic::Enum, Clone, Debug, PartialEq)]
#[cynic(non_exhaustive)]
pub enum Mode {
    Walk,
    Bicycle,
    Car,
    Tram,
    Subway,
    Rail,
    Bus,
    Ferry,
    CableCar,
    Gondola,
    Funicular,
    Transit,
    /// Anything else (COACH, MONORAIL, TROLLEYBUS, TAXI, ...) is some flavor of transit. Carries
    /// the mode's name as OTP spells it.
    #[cynic(fallback)]
    Other(String),
}

impl From<&Mode> for TravelMode {
    fn from(mode: &Mode) -> Self {
        match mode {
            Mode::Walk => TravelMode::Walk,
            Mode::Bicycle => TravelMode::Bicycle,
            Mode::Car => TravelMode::Car,
            _ => TravelMode::Transit,
        }
    }
}

impl From<&Mode> for otp_api::TransitMode {
    fn from(mode: &Mode) -> Self {
        match mode {
            Mode::Walk => otp_api::TransitMode::Walk,
            Mode::Bicycle => otp_api::TransitMode::Bicycle,
            Mode::Car => otp_api::TransitMode::Car,
            Mode::Tram => otp_api::TransitMode::Tram,
            Mode::Subway => otp_api::TransitMode::Subway,
            Mode::Rail => otp_api::TransitMode::Rail,
            Mode::Bus => otp_api::TransitMode::Bus,
            Mode::Ferry => otp_api::TransitMode::Ferry,
            Mode::CableCar => otp_api::TransitMode::CableCar,
            Mode::Gondola => otp_api::TransitMode::Gondola,
            Mode::Funicular => otp_api::TransitMode::Funicular,
            Mode::Transit | Mode::Other(_) => otp_api::TransitMode::Transit,
        }
    }
}

/// Why OTP couldn't plan a trip. The fallback carries the raw name of any code we don't know
/// about, since we pass it through to our own clients.
#[derive(cynic::Enum, Clone, Debug)]
#[cynic(non_exhaustive)]
pub enum RoutingErrorCode {
    LocationNotFound,
    NoStopsInRange,
    NoTransitConnection,
    NoTransitConnectionInSearchWindow,
    OutsideBounds,
    OutsideServicePeriod,
    WalkingBetterThanTransit,
    #[cynic(fallback)]
    Other(String),
}

impl RoutingErrorCode {
    /// Whether the trip is simply beyond what this graph knows about, rather than something we
    /// should complain about. Callers use this to fall back to another router.
    pub fn is_out_of_area(&self) -> bool {
        matches!(
            self,
            RoutingErrorCode::OutsideBounds
                | RoutingErrorCode::NoStopsInRange
                | RoutingErrorCode::LocationNotFound
        )
    }

    /// The code's name as OTP spells it, e.g. "NO_TRANSIT_CONNECTION".
    pub fn as_str(&self) -> &str {
        match self {
            RoutingErrorCode::LocationNotFound => "LOCATION_NOT_FOUND",
            RoutingErrorCode::NoStopsInRange => "NO_STOPS_IN_RANGE",
            RoutingErrorCode::NoTransitConnection => "NO_TRANSIT_CONNECTION",
            RoutingErrorCode::NoTransitConnectionInSearchWindow => {
                "NO_TRANSIT_CONNECTION_IN_SEARCH_WINDOW"
            }
            RoutingErrorCode::OutsideBounds => "OUTSIDE_BOUNDS",
            RoutingErrorCode::OutsideServicePeriod => "OUTSIDE_SERVICE_PERIOD",
            RoutingErrorCode::WalkingBetterThanTransit => "WALKING_BETTER_THAN_TRANSIT",
            RoutingErrorCode::Other(code) => code,
        }
    }
}

/// Millis since the Unix epoch, or `None` for instants before it.
fn millis(time: DateTime<FixedOffset>) -> Option<u64> {
    u64::try_from(time.timestamp_millis()).ok()
}

impl PlanConnectionQuery {
    fn into_result(self) -> PlanResult {
        let Some(connection) = self.plan_connection else {
            // OTP nulls out the whole connection rather than reporting a routing error - which we
            // have no name for, so make one up.
            return PlanResult {
                itineraries: vec![],
                routing_errors: vec![RoutingError {
                    code: RoutingErrorCode::Other("NO_PLAN".to_string()),
                    description: "No plan returned by OTP".to_string(),
                }],
            };
        };

        PlanResult {
            itineraries: connection
                .edges
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .map(|edge| edge.node)
                .collect(),
            routing_errors: connection.routing_errors,
        }
    }
}

impl PlanResult {
    /// Why OTP couldn't plan this trip, if it couldn't.
    pub fn routing_error(&self) -> Option<&RoutingError> {
        self.itineraries
            .is_empty()
            .then(|| self.routing_errors.first())
            .flatten()
    }

    fn into_otp(self) -> otp_api::PlanResponse {
        // Surface OTP's routing errors the same way the old REST API did: an empty itinerary list
        // plus an error object. Downstream, non-transit searches use this to fall back to Valhalla.
        let error = self.itineraries.is_empty().then(|| {
            let first = self.routing_errors.first();
            otp_api::PlanError {
                id: 400,
                msg: first
                    .map(|e| e.description.clone())
                    .unwrap_or_else(|| "No itineraries found".to_string()),
                message: first
                    .map(|e| e.code.as_str().to_string())
                    .unwrap_or_else(|| "NO_ITINERARIES".to_string()),
            }
        });

        otp_api::PlanResponse {
            plan: otp_api::Plan {
                itineraries: self
                    .itineraries
                    .into_iter()
                    .map(Itinerary::into_otp)
                    .collect(),
            },
            error,
        }
    }
}

impl Itinerary {
    fn into_otp(self) -> otp_api::Itinerary {
        let legs: Vec<_> = self.legs.into_iter().flatten().map(Leg::into_otp).collect();

        let start_time = self
            .start
            .and_then(millis)
            .or_else(|| legs.first().map(|l| l.start_time))
            .unwrap_or(0);
        let end_time = self
            .end
            .and_then(millis)
            .or_else(|| legs.last().map(|l| l.end_time))
            .unwrap_or(0);

        otp_api::Itinerary {
            duration: self.duration.unwrap_or(0).max(0) as u64,
            walk_distance: self.walk_distance.unwrap_or(0.0),
            start_time,
            end_time,
            legs,
        }
    }
}

impl Leg {
    fn into_otp(self) -> otp_api::Leg {
        let mode: otp_api::TransitMode = self
            .mode
            .as_ref()
            .map(Into::into)
            .unwrap_or(otp_api::TransitMode::Transit);
        let transit_leg = self.is_transit();

        let start_time = self.start.millis().unwrap_or(0);
        let end_time = self.end.millis().unwrap_or(0);

        let route = self.route.unwrap_or(Route {
            short_name: None,
            long_name: None,
            color: None,
        });

        let leg_geometry = self
            .leg_geometry
            .map(|g| otp_api::LegGeometry {
                points: g.points.unwrap_or_default(),
                length: g.length.unwrap_or(0) as f64,
            })
            .unwrap_or(otp_api::LegGeometry {
                points: String::new(),
                length: 0.0,
            });

        otp_api::Leg {
            mode,
            transit_leg,
            distance: self.distance.unwrap_or(0.0),
            leg_geometry,
            route: route.short_name.clone().unwrap_or_default(),
            route_short_name: route.short_name,
            route_long_name: route.long_name,
            route_color: route.color,
            agency_name: self.agency.map(|a| a.name),
            headsign: self.headsign,
            alerts: self
                .alerts
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .map(Alert::into_otp)
                .collect(),
            steps: self
                .steps
                .unwrap_or_default()
                .into_iter()
                .flatten()
                .map(Step::into_otp)
                .collect(),
            from: self.from.into_otp(),
            to: self.to.into_otp(),
            start_time,
            end_time,
            real_time: self.real_time.unwrap_or(false),
        }
    }
}

impl Place {
    fn into_otp(self) -> otp_api::Place {
        otp_api::Place {
            location: otp_api::LonLat {
                lat: self.lat,
                lon: self.lon,
            },
            arrival: self.arrival.and_then(|t| t.millis()),
            departure: self.departure.and_then(|t| t.millis()),
            name: self.name,
        }
    }
}

impl Step {
    fn into_otp(self) -> otp_api::Step {
        otp_api::Step {
            distance: self.distance.unwrap_or(0.0),
            relative_direction: self
                .relative_direction
                .unwrap_or(otp_api::RelativeDirection::Continue),
            street_name: self.street_name.unwrap_or_default(),
            absolute_direction: self.absolute_direction,
            exit: self.exit,
            stay_on: self.stay_on,
            area: self.area,
            bogus_name: self.bogus_name,
            lon: self.lon.unwrap_or(0.0),
            lat: self.lat.unwrap_or(0.0),
        }
    }
}

impl Alert {
    fn into_otp(self) -> otp_api::Alert {
        // The GraphQL API gives these as Unix *seconds*; the REST API gave millis, which is what
        // `otp_api::Alert` documents and what our clients read.
        let seconds_to_millis = |seconds: i64| seconds * 1000;

        otp_api::Alert {
            alert_header_text: self.alert_header_text,
            alert_description_text: Some(self.alert_description_text),
            alert_url: self.alert_url,
            effective_start_date: self.effective_start_date.map(seconds_to_millis),
            effective_end_date: self.effective_end_date.map(seconds_to_millis),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic::GraphQlResponse;
    use serde_json::json;

    #[test]
    fn graphql_endpoint_url() {
        let expected = Url::parse("http://opentripplanner:8000/otp/gtfs/v1").unwrap();
        for base_url in [
            "http://opentripplanner:8000",
            "http://opentripplanner:8000/",
        ] {
            let base_url = Url::parse(base_url).unwrap();
            assert_eq!(endpoint_url(&base_url).unwrap(), expected);
        }
    }

    /// A `planConnection` response. Like OTP, it spells out *every* field the query selects,
    /// using `null` for the ones that don't apply - that's what GraphQL guarantees, and what the
    /// generated deserializers expect.
    const PLAN_CONNECTION_FIXTURE: &str = r#"{
      "data": { "planConnection": {
        "edges": [ { "node": {
          "start": "2024-05-17T10:00:00-07:00",
          "end": "2024-05-17T10:35:00-07:00",
          "duration": 2100,
          "walkDistance": 500.0,
          "legs": [
            {
              "mode": "WALK", "transitLeg": false, "distance": 120.0, "duration": 300.0, "realTime": false, "headsign": null,
              "start": { "scheduledTime": "2024-05-17T10:00:00-07:00", "estimated": null },
              "end": { "scheduledTime": "2024-05-17T10:05:00-07:00", "estimated": null },
              "from": { "name": "Origin", "lat": 47.5758, "lon": -122.3392, "arrival": null, "departure": null },
              "to": { "name": "1st Ave S & S Hanford St", "lat": 47.5759, "lon": -122.3341, "arrival": null, "departure": null },
              "legGeometry": { "points": "abcd", "length": 2 },
              "route": null,
              "agency": null,
              "steps": [ {
                "distance": 10.0, "relativeDirection": "DEPART", "absoluteDirection": "SOUTH",
                "streetName": "East Marginal Way South", "lat": 47.5758, "lon": -122.3392,
                "area": false, "bogusName": false, "stayOn": false, "exit": null
              } ],
              "alerts": []
            },
            {
              "mode": "BUS", "transitLeg": true, "distance": 5000.0, "duration": 1680.0, "realTime": true, "headsign": "Downtown",
              "start": { "scheduledTime": "2024-05-17T10:07:00-07:00", "estimated": { "time": "2024-05-17T10:08:00-07:00" } },
              "end": { "scheduledTime": "2024-05-17T10:35:00-07:00", "estimated": null },
              "from": { "name": "1st Ave S & S Hanford St", "lat": 47.5759, "lon": -122.3341, "arrival": null, "departure": { "scheduledTime": "2024-05-17T10:07:00-07:00", "estimated": null } },
              "to": { "name": "3rd Ave & Pine St", "lat": 47.6106, "lon": -122.3376, "arrival": { "scheduledTime": "2024-05-17T10:35:00-07:00", "estimated": null }, "departure": null },
              "legGeometry": { "points": "wxyz", "length": 10 },
              "route": { "shortName": "40", "longName": "Downtown - Ballard", "color": "0080FF" },
              "agency": { "name": "Metro Transit" },
              "steps": [],
              "alerts": [ {
                "alertHeaderText": "Detour", "alertDescriptionText": "Reroute", "alertUrl": "http://x",
                "effectiveStartDate": 1715000000, "effectiveEndDate": 1716000000
              } ]
            }
          ]
        } } ],
        "routingErrors": []
      } }
    }"#;

    fn millis_at(rfc3339: &str) -> u64 {
        millis(DateTime::parse_from_rfc3339(rfc3339).unwrap()).unwrap()
    }

    fn parse_fixture() -> otp_api::PlanResponse {
        let envelope: GraphQlResponse<PlanConnectionQuery> =
            serde_json::from_str(PLAN_CONNECTION_FIXTURE).unwrap();
        envelope.data.unwrap().into_result().into_otp()
    }

    #[test]
    fn maps_plan_connection_into_otp() {
        let plan = parse_fixture();
        assert!(plan.error.is_none());
        assert_eq!(plan.plan.itineraries.len(), 1);

        let itin = &plan.plan.itineraries[0];
        assert_eq!(itin.duration, 2100);
        assert_eq!(itin.walk_distance, 500.0);
        assert_eq!(itin.start_time, millis_at("2024-05-17T10:00:00-07:00"));
        assert_eq!(itin.legs.len(), 2);
    }

    #[test]
    fn maps_walk_leg() {
        let plan = parse_fixture();
        let walk = &plan.plan.itineraries[0].legs[0];
        assert_eq!(walk.mode, otp_api::TransitMode::Walk);
        assert!(!walk.transit_leg);
        assert_eq!(walk.route, "");
        assert_eq!(walk.steps.len(), 1);
        assert_eq!(walk.steps[0].street_name, "East Marginal Way South");
        assert_eq!(walk.from.name.as_deref(), Some("Origin"));
        assert_eq!(walk.leg_geometry.points, "abcd");
    }

    #[test]
    fn maps_transit_leg() {
        let plan = parse_fixture();
        let bus = &plan.plan.itineraries[0].legs[1];
        assert_eq!(bus.mode, otp_api::TransitMode::Bus);
        assert!(bus.transit_leg);
        assert!(bus.real_time);
        assert_eq!(bus.route, "40");
        assert_eq!(bus.route_short_name.as_deref(), Some("40"));
        assert_eq!(bus.route_long_name.as_deref(), Some("Downtown - Ballard"));
        assert_eq!(bus.route_color.as_deref(), Some("0080FF"));
        assert_eq!(bus.agency_name.as_deref(), Some("Metro Transit"));
        assert_eq!(bus.headsign.as_deref(), Some("Downtown"));
        // Realtime estimate is preferred over the scheduled time.
        assert_eq!(bus.start_time, millis_at("2024-05-17T10:08:00-07:00"));
        assert_eq!(bus.alerts.len(), 1);
        let alert = &bus.alerts[0];
        assert_eq!(alert.alert_header_text.as_deref(), Some("Detour"));
        // OTP's alert timestamps are in seconds, ours are in millis.
        assert_eq!(alert.effective_start_date, Some(1715000000000));
        assert_eq!(alert.effective_end_date, Some(1716000000000));
    }

    #[test]
    fn empty_result_becomes_error() {
        let body = json!({
            "data": { "planConnection": {
                "edges": [],
                "routingErrors": [ { "code": "NO_TRANSIT_CONNECTION", "description": "No transit connection was found." } ]
            } }
        });
        let envelope: GraphQlResponse<PlanConnectionQuery> = serde_json::from_value(body).unwrap();
        let plan = envelope.data.unwrap().into_result().into_otp();
        assert!(plan.plan.itineraries.is_empty());
        let error = plan.error.expect("expected an error");
        assert_eq!(error.message, "NO_TRANSIT_CONNECTION");
        assert_eq!(error.msg, "No transit connection was found.");
    }

    fn params(modes: &[TravelMode]) -> PlanParams<'_> {
        PlanParams {
            from: Point::new(-122.3, 47.5),
            to: Point::new(-122.3, 47.6),
            modes,
            num_itineraries: 5,
            date_time: None,
            arrive_by: false,
            timezone: None,
        }
    }

    /// The serialized variables of the query `params` would send.
    fn variables(params: &PlanParams<'_>) -> serde_json::Value {
        let body = serde_json::to_value(params.build_query()).unwrap();
        body["variables"].clone()
    }

    #[test]
    fn transit_request_body() {
        let vars = variables(&params(&[TravelMode::Transit]));
        assert_eq!(vars["first"], 5);
        assert_eq!(vars["origin"]["location"]["coordinate"]["latitude"], 47.5);
        assert_eq!(vars["modes"]["transit"]["access"][0], "WALK");
        assert_eq!(vars["dateTime"], serde_json::Value::Null);
    }

    #[test]
    fn bike_transit_request_body() {
        let vars = variables(&params(&[TravelMode::Transit, TravelMode::Bicycle]));
        assert_eq!(vars["modes"]["transit"]["access"][0], "BICYCLE");
        assert_eq!(vars["modes"]["transit"]["egress"][0], "BICYCLE");
        assert_eq!(vars["modes"]["transit"]["transfer"][0], "BICYCLE");
    }

    #[test]
    fn walk_request_body_is_direct() {
        let vars = variables(&params(&[TravelMode::Walk]));
        // Direct searches only ever want a single itinerary.
        assert_eq!(vars["first"], 1);
        assert_eq!(vars["modes"]["directOnly"], true);
        assert_eq!(vars["modes"]["direct"][0], "WALK");
    }

    #[test]
    fn resolves_date_time_in_graph_timezone() {
        let mut params = params(&[TravelMode::Transit]);
        params.date_time = PlanDateTime::from_local_date_and_time("2024-06-13", "14:30");
        params.timezone = Some(chrono_tz::America::Los_Angeles);

        // 2:30pm on June 13th in Los Angeles is UTC-7 (PDT).
        assert_eq!(
            variables(&params)["dateTime"]["earliestDeparture"],
            "2024-06-13T14:30:00-07:00"
        );
    }

    #[test]
    fn resolves_arrive_by_date_time() {
        let mut params = params(&[TravelMode::Transit]);
        params.date_time = PlanDateTime::from_local_date_and_time("2024-06-13", "14:30");
        params.arrive_by = true;
        params.timezone = Some(chrono_tz::America::Los_Angeles);

        assert_eq!(
            variables(&params)["dateTime"]["latestArrival"],
            "2024-06-13T14:30:00-07:00"
        );
    }
}
