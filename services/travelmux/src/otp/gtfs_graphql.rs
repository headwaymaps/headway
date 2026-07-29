//! Client for OTP's GTFS GraphQL API (`/otp/gtfs/v1`).
//!
//! OTP removed its REST `/plan` endpoint in 2.8, so we talk to it over GraphQL now. This module
//! builds the `planConnection` query, POSTs it, and maps the response back into the
//! [`crate::otp::otp_api`] types that the rest of travelmux (and its clients) expect.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::Tz;
use geo::geometry::Point;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

use crate::otp::otp_api;
use crate::{Error, Result, TravelMode};

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

const PLAN_QUERY: &str = r#"
query Plan(
  $fromLat: CoordinateValue!, $fromLon: CoordinateValue!,
  $toLat: CoordinateValue!, $toLon: CoordinateValue!,
  $first: Int, $dateTime: PlanDateTimeInput, $modes: PlanModesInput
) {
  planConnection(
    origin: { location: { coordinate: { latitude: $fromLat, longitude: $fromLon } } }
    destination: { location: { coordinate: { latitude: $toLat, longitude: $toLon } } }
    first: $first
    dateTime: $dateTime
    modes: $modes
  ) {
    edges { node {
      start end duration walkDistance
      legs {
        mode transitLeg distance duration realTime headsign
        start { scheduledTime estimated { time } }
        end { scheduledTime estimated { time } }
        from { name lat lon arrival { scheduledTime estimated { time } } departure { scheduledTime estimated { time } } }
        to { name lat lon arrival { scheduledTime estimated { time } } departure { scheduledTime estimated { time } } }
        legGeometry { points length }
        route { gtfsId shortName longName color }
        agency { name }
        steps { distance relativeDirection absoluteDirection streetName lat lon area bogusName stayOn exit }
        alerts { alertHeaderText alertDescriptionText alertUrl effectiveStartDate effectiveEndDate }
      }
    } }
    routingErrors { code description }
  }
}
"#;

impl PlanParams<'_> {
    fn primary_mode(&self) -> TravelMode {
        self.modes.first().copied().unwrap_or(TravelMode::Transit)
    }

    /// Whether this is a "direct" (non-transit) search - i.e. plain walking or cycling.
    fn is_direct(&self) -> bool {
        !matches!(self.primary_mode(), TravelMode::Transit)
    }

    /// Build the `PlanModesInput` value.
    fn modes_input(&self) -> Value {
        match self.primary_mode() {
            TravelMode::Walk => json!({ "directOnly": true, "direct": ["WALK"] }),
            TravelMode::Bicycle => json!({ "directOnly": true, "direct": ["BICYCLE"] }),
            TravelMode::Car => json!({ "directOnly": true, "direct": ["CAR"] }),
            TravelMode::Transit => {
                // bike+transit: access/egress/transfer must all agree on BICYCLE for OTP to
                // consider cycling to/from and between stops.
                let street = if self.modes.contains(&TravelMode::Bicycle) {
                    "BICYCLE"
                } else {
                    "WALK"
                };
                json!({
                    "transit": {
                        "access": [street],
                        "egress": [street],
                        "transfer": [street],
                    }
                })
            }
        }
    }

    /// Build the optional `PlanDateTimeInput` value from the client's naive date/time.
    fn date_time_input(&self) -> Option<Value> {
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

        let rfc3339 = resolved.to_rfc3339();
        let key = if self.arrive_by {
            "latestArrival"
        } else {
            "earliestDeparture"
        };
        Some(json!({ key: rfc3339 }))
    }

    fn request_body(&self) -> Value {
        let first = if self.is_direct() {
            // For plain walking/cycling OTP returns a single itinerary, and downstream code
            // assumes exactly one.
            1
        } else {
            self.num_itineraries
        };

        let mut variables = json!({
            "fromLat": self.from.y(),
            "fromLon": self.from.x(),
            "toLat": self.to.y(),
            "toLon": self.to.x(),
            "first": first,
            "modes": self.modes_input(),
        });
        if let Some(date_time) = self.date_time_input() {
            variables["dateTime"] = date_time;
        }

        json!({ "query": PLAN_QUERY, "variables": variables })
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
pub(crate) fn endpoint_url(base_url: &Url) -> Result<Url> {
    let mut endpoint = base_url.clone();
    endpoint
        .path_segments_mut()
        .map_err(|_| Error::server(format!("OTP base url must be a valid base: {base_url}")))?
        .pop_if_empty()
        .extend(GRAPHQL_PATH);
    Ok(endpoint)
}

/// POST a GraphQL query to `endpoint` and return the deserialized `data` payload.
pub(crate) async fn post_graphql<T: DeserializeOwned>(
    client: &reqwest::Client,
    endpoint: &Url,
    body: &Value,
) -> Result<T> {
    let response = client.post(endpoint.clone()).json(body).send().await?;
    if !response.status().is_success() {
        return Err(Error::server(format!(
            "HTTP error from OTP GraphQL: {}",
            response.status()
        )));
    }

    let envelope: GraphQlResponse<T> = response.json().await?;
    if let Some(errors) = envelope.errors {
        if !errors.is_empty() {
            let messages: Vec<_> = errors.into_iter().map(|e| e.message).collect();
            return Err(Error::server(format!(
                "OTP GraphQL returned errors: {}",
                messages.join("; ")
            )));
        }
    }

    envelope
        .data
        .ok_or_else(|| Error::server("OTP GraphQL response had no data"))
}

/// Execute a `planConnection` query and map the result into the legacy [`otp_api::PlanResponse`].
pub async fn plan(
    client: &reqwest::Client,
    endpoint: &Url,
    params: &PlanParams<'_>,
) -> Result<otp_api::PlanResponse> {
    let body = params.request_body();
    let data: PlanData = post_graphql(client, endpoint, &body).await?;
    Ok(data.into_otp())
}

// ===
// Response
// ===

#[derive(Debug, Deserialize)]
struct GraphQlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanData {
    plan_connection: Option<PlanConnection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanConnection {
    #[serde(default)]
    edges: Vec<PlanEdge>,
    #[serde(default)]
    routing_errors: Vec<RoutingError>,
}

#[derive(Debug, Deserialize)]
struct PlanEdge {
    node: ItineraryNode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItineraryNode {
    start: Option<String>,
    end: Option<String>,
    /// seconds
    duration: Option<i64>,
    walk_distance: Option<f64>,
    #[serde(default)]
    legs: Vec<LegNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegNode {
    mode: Option<Mode>,
    transit_leg: Option<bool>,
    distance: Option<f64>,
    real_time: Option<bool>,
    headsign: Option<String>,
    start: Option<LegTime>,
    end: Option<LegTime>,
    from: PlaceNode,
    to: PlaceNode,
    leg_geometry: Option<Geometry>,
    route: Option<RouteNode>,
    agency: Option<AgencyNode>,
    #[serde(default)]
    steps: Vec<StepNode>,
    #[serde(default)]
    alerts: Vec<AlertNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegTime {
    scheduled_time: String,
    estimated: Option<EstimatedTime>,
}

impl LegTime {
    /// The realtime-estimated instant if available, otherwise the scheduled instant, in millis.
    fn millis(&self) -> Option<u64> {
        let source = self
            .estimated
            .as_ref()
            .map(|e| e.time.as_str())
            .unwrap_or(self.scheduled_time.as_str());
        parse_rfc3339_millis(source)
    }
}

#[derive(Debug, Deserialize)]
struct EstimatedTime {
    time: String,
}

#[derive(Debug, Deserialize)]
struct PlaceNode {
    name: Option<String>,
    lat: f64,
    lon: f64,
    arrival: Option<LegTime>,
    departure: Option<LegTime>,
}

#[derive(Debug, Deserialize)]
struct Geometry {
    points: Option<String>,
    length: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouteNode {
    short_name: Option<String>,
    long_name: Option<String>,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgencyNode {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StepNode {
    distance: Option<f64>,
    relative_direction: Option<otp_api::RelativeDirection>,
    absolute_direction: Option<otp_api::AbsoluteDirection>,
    street_name: Option<String>,
    lat: f64,
    lon: f64,
    area: Option<bool>,
    bogus_name: Option<bool>,
    stay_on: Option<bool>,
    exit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlertNode {
    alert_header_text: Option<String>,
    alert_description_text: Option<String>,
    alert_url: Option<String>,
    effective_start_date: Option<i64>,
    effective_end_date: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RoutingError {
    code: Option<String>,
    description: Option<String>,
}

/// GTFS GraphQL leg `Mode` enum. A superset of [`otp_api::TransitMode`]; anything we don't model
/// explicitly is treated as generic transit.
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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
    #[serde(other)]
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
            // Anything else (COACH, MONORAIL, TROLLEYBUS, TAXI, ...) is some flavor of transit.
            Mode::Transit | Mode::Other => otp_api::TransitMode::Transit,
        }
    }
}

fn parse_rfc3339_millis(s: &str) -> Option<u64> {
    let millis = chrono::DateTime::parse_from_rfc3339(s)
        .ok()?
        .timestamp_millis();
    u64::try_from(millis).ok()
}

impl PlanData {
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
            .into_iter()
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
                    .and_then(|e| e.description.clone())
                    .unwrap_or_else(|| "No itineraries found".to_string()),
                message: first
                    .and_then(|e| e.code)
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

impl ItineraryNode {
    fn into_otp(self) -> otp_api::Itinerary {
        let legs: Vec<_> = self.legs.into_iter().map(LegNode::into_otp).collect();

        let start_time = self
            .start
            .as_deref()
            .and_then(parse_rfc3339_millis)
            .or_else(|| legs.first().map(|l| l.start_time))
            .unwrap_or(0);
        let end_time = self
            .end
            .as_deref()
            .and_then(parse_rfc3339_millis)
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

impl LegNode {
    fn into_otp(self) -> otp_api::Leg {
        let mode: otp_api::TransitMode = self
            .mode
            .map(Into::into)
            .unwrap_or(otp_api::TransitMode::Transit);
        let transit_leg = self.transit_leg.unwrap_or(!matches!(
            mode,
            otp_api::TransitMode::Walk | otp_api::TransitMode::Bicycle | otp_api::TransitMode::Car
        ));

        let start_time = self.start.as_ref().and_then(LegTime::millis).unwrap_or(0);
        let end_time = self.end.as_ref().and_then(LegTime::millis).unwrap_or(0);

        let route = self.route.unwrap_or(RouteNode {
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
            agency_name: self.agency.and_then(|a| a.name),
            headsign: self.headsign,
            alerts: self.alerts.into_iter().map(AlertNode::into_otp).collect(),
            steps: self.steps.into_iter().map(StepNode::into_otp).collect(),
            from: self.from.into_otp(),
            to: self.to.into_otp(),
            start_time,
            end_time,
            real_time: self.real_time.unwrap_or(false),
        }
    }
}

impl PlaceNode {
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

impl StepNode {
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
            lon: self.lon,
            lat: self.lat,
        }
    }
}

impl AlertNode {
    fn into_otp(self) -> otp_api::Alert {
        otp_api::Alert {
            alert_header_text: self.alert_header_text,
            alert_description_text: self.alert_description_text,
            alert_url: self.alert_url,
            effective_start_date: self.effective_start_date,
            effective_end_date: self.effective_end_date,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    const PLAN_CONNECTION_FIXTURE: &str = r#"{
      "data": { "planConnection": {
        "edges": [ { "node": {
          "start": "2024-05-17T10:00:00-07:00",
          "end": "2024-05-17T10:35:00-07:00",
          "duration": 2100,
          "walkDistance": 500.0,
          "legs": [
            {
              "mode": "WALK", "transitLeg": false, "distance": 120.0, "realTime": false,
              "start": { "scheduledTime": "2024-05-17T10:00:00-07:00" },
              "end": { "scheduledTime": "2024-05-17T10:05:00-07:00" },
              "from": { "name": "Origin", "lat": 47.5758, "lon": -122.3392 },
              "to": { "name": "1st Ave S & S Hanford St", "lat": 47.5759, "lon": -122.3341 },
              "legGeometry": { "points": "abcd", "length": 2 },
              "steps": [ { "distance": 10.0, "relativeDirection": "DEPART", "absoluteDirection": "SOUTH", "streetName": "East Marginal Way South", "lat": 47.5758, "lon": -122.3392, "area": false, "bogusName": false } ],
              "alerts": []
            },
            {
              "mode": "BUS", "transitLeg": true, "distance": 5000.0, "realTime": true, "headsign": "Downtown",
              "start": { "scheduledTime": "2024-05-17T10:07:00-07:00", "estimated": { "time": "2024-05-17T10:08:00-07:00" } },
              "end": { "scheduledTime": "2024-05-17T10:35:00-07:00" },
              "from": { "name": "1st Ave S & S Hanford St", "lat": 47.5759, "lon": -122.3341 },
              "to": { "name": "3rd Ave & Pine St", "lat": 47.6106, "lon": -122.3376 },
              "legGeometry": { "points": "wxyz", "length": 10 },
              "route": { "gtfsId": "1:100", "shortName": "40", "longName": "Downtown - Ballard", "color": "0080FF" },
              "agency": { "name": "Metro Transit" },
              "steps": [],
              "alerts": [ { "alertHeaderText": "Detour", "alertDescriptionText": "Reroute", "alertUrl": "http://x", "effectiveStartDate": 1715000000000, "effectiveEndDate": 1716000000000 } ]
            }
          ]
        } } ],
        "routingErrors": []
      } }
    }"#;

    fn parse_fixture() -> otp_api::PlanResponse {
        let envelope: GraphQlResponse<PlanData> =
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
        assert_eq!(
            itin.start_time,
            parse_rfc3339_millis("2024-05-17T10:00:00-07:00").unwrap()
        );
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
        assert_eq!(
            bus.start_time,
            parse_rfc3339_millis("2024-05-17T10:08:00-07:00").unwrap()
        );
        assert_eq!(bus.alerts.len(), 1);
        assert_eq!(bus.alerts[0].alert_header_text.as_deref(), Some("Detour"));
    }

    #[test]
    fn empty_result_becomes_error() {
        let body = json!({
            "data": { "planConnection": {
                "edges": [],
                "routingErrors": [ { "code": "NO_TRANSIT_CONNECTION", "description": "No transit connection was found." } ]
            } }
        });
        let envelope: GraphQlResponse<PlanData> = serde_json::from_value(body).unwrap();
        let plan = envelope.data.unwrap().into_otp();
        assert!(plan.plan.itineraries.is_empty());
        let error = plan.error.expect("expected an error");
        assert_eq!(error.message, "NO_TRANSIT_CONNECTION");
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

    #[test]
    fn transit_request_body() {
        let body = params(&[TravelMode::Transit]).request_body();
        let vars = &body["variables"];
        assert_eq!(vars["first"], 5);
        assert_eq!(vars["modes"]["transit"]["access"][0], "WALK");
        assert!(vars.get("dateTime").is_none());
    }

    #[test]
    fn bike_transit_request_body() {
        let body = params(&[TravelMode::Transit, TravelMode::Bicycle]).request_body();
        let vars = &body["variables"];
        assert_eq!(vars["modes"]["transit"]["access"][0], "BICYCLE");
        assert_eq!(vars["modes"]["transit"]["egress"][0], "BICYCLE");
    }

    #[test]
    fn walk_request_body_is_direct() {
        let body = params(&[TravelMode::Walk]).request_body();
        let vars = &body["variables"];
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

        let body = params.request_body();
        // 2:30pm on June 13th in Los Angeles is UTC-7 (PDT).
        assert_eq!(
            body["variables"]["dateTime"]["earliestDeparture"],
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

        let body = params.request_body();
        assert_eq!(
            body["variables"]["dateTime"]["latestArrival"],
            "2024-06-13T14:30:00-07:00"
        );
    }
}
