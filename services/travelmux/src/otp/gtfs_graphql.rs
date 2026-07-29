//! Client for OTP's GTFS GraphQL API (`/otp/gtfs/v1`).
//!
//! OTP removed its REST `/plan` endpoint in 2.8, so we talk to it over GraphQL now. The query is
//! described by the `cynic` types in this module, which are checked against
//! [`crate::otp::schema`] at compile time. Responses are mapped back into the
//! [`crate::otp::otp_api`] types that the rest of travelmux (and its clients) expect.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::Tz;
use cynic::http::ReqwestExt;
use cynic::{Operation, QueryBuilder};
use geo::geometry::Point;
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

use crate::otp::otp_api;
use crate::otp::schema;
// NB: `Result`/`Error` are deliberately *not* imported here: cynic's derives expand to code
// using `std::result::Result` unqualified, which our aliases would shadow.
use crate::TravelMode;

// ===
// Request
// ===

/// Everything needed to build a `planConnection` query. Both the v5 and v6 plan endpoints build
/// one of these.
pub struct PlanParams<'a> {
    pub from: Point,
    pub to: Point,
    /// The requested travel modes. The first is the "primary" mode; a following `Bicycle` with a
    /// primary of `Transit` means bike+transit.
    pub modes: &'a [TravelMode],
    pub num_itineraries: u32,
    /// "YYYY-MM-DD" in the graph's local timezone, if the client requested a specific date.
    pub date: Option<&'a str>,
    /// "HH:MM" (24h) in the graph's local timezone, if the client requested a specific time.
    pub time: Option<&'a str>,
    /// Whether `date`/`time` describe the desired arrival (rather than departure) time.
    pub arrive_by: bool,
    /// The graph's timezone, used to resolve naive `date`/`time` into an absolute instant.
    pub timezone: Option<Tz>,
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

    /// Resolve the client's naive date/time into the absolute instant OTP wants, or `None` to let
    /// OTP plan from "now".
    fn date_time_input(&self) -> Option<PlanDateTimeInput> {
        let date = self.date?;
        // The frontend only sends a time alongside a date; default to midnight if it's missing.
        let time = self.time.unwrap_or("00:00");

        let Some(tz) = self.timezone else {
            log::warn!(
                "requested a specific plan time but the graph's timezone is unknown; \
                 falling back to 'now'"
            );
            return None;
        };

        let Some(naive) = parse_naive_date_time(date, time) else {
            log::warn!("could not parse plan date/time: date={date:?} time={time:?}");
            return None;
        };

        // `.earliest()`/`.latest()` gracefully handle DST ambiguities.
        let resolved = tz
            .from_local_datetime(&naive)
            .earliest()
            .or_else(|| tz.from_local_datetime(&naive).latest());
        let Some(resolved) = resolved else {
            log::warn!("plan date/time {naive:?} does not exist in timezone {tz:?}");
            return None;
        };

        let resolved = resolved.fixed_offset();
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

fn parse_naive_date_time(date: &str, time: &str) -> Option<NaiveDateTime> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let time = NaiveTime::parse_from_str(time, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(time, "%H:%M:%S"))
        .ok()?;
    Some(NaiveDateTime::new(date, time))
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

/// Execute a `planConnection` query and map the result into the legacy [`otp_api::PlanResponse`].
pub async fn plan(
    client: &reqwest::Client,
    endpoint: &Url,
    params: &PlanParams<'_>,
) -> crate::Result<otp_api::PlanResponse> {
    let data: PlanConnectionQuery = post_graphql(client, endpoint, params.build_query()).await?;
    Ok(data.into_otp())
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

#[derive(cynic::QueryFragment, Debug)]
struct Itinerary {
    start: Option<DateTime<FixedOffset>>,
    end: Option<DateTime<FixedOffset>>,
    /// seconds
    duration: Option<i64>,
    walk_distance: Option<f64>,
    legs: Vec<Option<Leg>>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Leg {
    mode: Option<Mode>,
    transit_leg: Option<bool>,
    distance: Option<f64>,
    real_time: Option<bool>,
    headsign: Option<String>,
    start: LegTime,
    end: LegTime,
    from: Place,
    to: Place,
    leg_geometry: Option<Geometry>,
    route: Option<Route>,
    agency: Option<Agency>,
    steps: Option<Vec<Option<Step>>>,
    alerts: Option<Vec<Option<Alert>>>,
}

#[derive(cynic::QueryFragment, Debug)]
struct LegTime {
    scheduled_time: DateTime<FixedOffset>,
    estimated: Option<RealTimeEstimate>,
}

impl LegTime {
    /// The realtime-estimated instant if available, otherwise the scheduled instant, in millis.
    fn millis(&self) -> Option<u64> {
        let source = self
            .estimated
            .as_ref()
            .map_or(self.scheduled_time, |e| e.time);
        millis(source)
    }
}

#[derive(cynic::QueryFragment, Debug)]
struct RealTimeEstimate {
    time: DateTime<FixedOffset>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Place {
    name: Option<String>,
    lat: f64,
    lon: f64,
    arrival: Option<LegTime>,
    departure: Option<LegTime>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Geometry {
    points: Option<String>,
    length: Option<i32>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Route {
    short_name: Option<String>,
    long_name: Option<String>,
    color: Option<String>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Agency {
    name: String,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "step")]
struct Step {
    distance: Option<f64>,
    relative_direction: Option<otp_api::RelativeDirection>,
    absolute_direction: Option<otp_api::AbsoluteDirection>,
    street_name: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    area: Option<bool>,
    bogus_name: Option<bool>,
    stay_on: Option<bool>,
    exit: Option<String>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Alert {
    alert_header_text: Option<String>,
    alert_description_text: String,
    alert_url: Option<String>,
    effective_start_date: Option<i64>,
    effective_end_date: Option<i64>,
}

#[derive(cynic::QueryFragment, Debug)]
struct RoutingError {
    code: RoutingErrorCode,
    description: String,
}

/// GTFS GraphQL leg `Mode` enum. A superset of [`otp_api::TransitMode`]; anything we don't model
/// explicitly is treated as generic transit.
#[derive(cynic::Enum, Clone, Copy, Debug)]
#[cynic(non_exhaustive)]
enum Mode {
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
    /// Anything else (COACH, MONORAIL, TROLLEYBUS, TAXI, ...) is some flavor of transit.
    #[cynic(fallback)]
    Other,
}

impl From<Mode> for otp_api::TransitMode {
    fn from(mode: Mode) -> Self {
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
            Mode::Transit | Mode::Other => otp_api::TransitMode::Transit,
        }
    }
}

/// Why OTP couldn't plan a trip. The fallback carries the raw name of any code we don't know
/// about, since we pass it through to our own clients.
#[derive(cynic::Enum, Clone, Debug)]
#[cynic(non_exhaustive)]
enum RoutingErrorCode {
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
    /// The code's name as OTP spells it, e.g. "NO_TRANSIT_CONNECTION".
    fn as_str(&self) -> &str {
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
    fn into_otp(self) -> otp_api::PlanResponse {
        let Some(connection) = self.plan_connection else {
            return otp_api::PlanResponse {
                plan: otp_api::Plan {
                    itineraries: vec![],
                },
                error: Some(otp_api::PlanError {
                    id: 400,
                    msg: "No plan returned by OTP".to_string(),
                    message: "NO_PLAN".to_string(),
                }),
            };
        };

        let itineraries: Vec<_> = connection
            .edges
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .map(|edge| edge.node.into_otp())
            .collect();

        // Surface OTP's routing errors the same way the old REST API did: an empty itinerary list
        // plus an error object. Downstream, non-transit searches use this to fall back to Valhalla.
        let error = if itineraries.is_empty() {
            let first = connection.routing_errors.into_iter().next();
            Some(otp_api::PlanError {
                id: 400,
                msg: first
                    .as_ref()
                    .map(|e| e.description.clone())
                    .unwrap_or_else(|| "No itineraries found".to_string()),
                message: first
                    .map(|e| e.code.as_str().to_string())
                    .unwrap_or_else(|| "NO_ITINERARIES".to_string()),
            })
        } else {
            None
        };

        otp_api::PlanResponse {
            plan: otp_api::Plan { itineraries },
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
            .map(Into::into)
            .unwrap_or(otp_api::TransitMode::Transit);
        let transit_leg = self.transit_leg.unwrap_or(!matches!(
            mode,
            otp_api::TransitMode::Walk | otp_api::TransitMode::Bicycle | otp_api::TransitMode::Car
        ));

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
              "mode": "WALK", "transitLeg": false, "distance": 120.0, "realTime": false, "headsign": null,
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
              "mode": "BUS", "transitLeg": true, "distance": 5000.0, "realTime": true, "headsign": "Downtown",
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
        envelope.data.unwrap().into_otp()
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
        let plan = envelope.data.unwrap().into_otp();
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
            date: None,
            time: None,
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
        params.date = Some("2024-06-13");
        params.time = Some("14:30");
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
        params.date = Some("2024-06-13");
        params.time = Some("14:30");
        params.arrive_by = true;
        params.timezone = Some(chrono_tz::America::Los_Angeles);

        assert_eq!(
            variables(&params)["dateTime"]["latestArrival"],
            "2024-06-13T14:30:00-07:00"
        );
    }
}
