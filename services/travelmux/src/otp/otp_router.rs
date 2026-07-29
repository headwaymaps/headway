use crate::otp::gtfs_graphql;
use crate::otp::schema;
// NB: `Result` is deliberately not imported here - see the note in `gtfs_graphql`.

use chrono_tz::Tz;
use cynic::QueryBuilder;
use geo::algorithm::{ConvexHull, Relate};
use geo::geometry::{MultiPoint, Point, Polygon};
use geo::PreparedGeometry;
use url::Url;

/// An OTP instance, and the area its graph covers.
///
/// Coverage starts out as the plain [`Polygon`] we compute from the instance's stops, and becomes
/// a [`PreparedGeometry`] once [`prepare`](OTPRouter::prepare)d for serving requests.
#[derive(Debug, Clone)]
pub struct OTPRouter<Coverage = Polygon> {
    /// The instance's GTFS GraphQL endpoint (e.g. `http://opentripplanner:8000/otp/gtfs/v1`).
    endpoint: Url,
    /// The area this instance covers, used to route trips to the correct zone.
    coverage: Coverage,
    /// The graph's timezone, used to resolve naive plan date/times into absolute instants.
    timezone: Option<Tz>,
}

/// An [`OTPRouter`] whose coverage area has been indexed for repeated containment checks.
pub type PreparedOTPRouter = OTPRouter<PreparedGeometry<'static, Polygon>>;

impl<Coverage> OTPRouter<Coverage> {
    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }

    pub fn timezone(&self) -> Option<Tz> {
        self.timezone
    }
}

impl OTPRouter {
    pub fn new(endpoint: Url, coverage: Polygon, timezone: Option<Tz>) -> Self {
        Self {
            endpoint,
            coverage,
            timezone,
        }
    }

    /// Index the coverage area, so that the containment checks we do for every trip are much
    /// cheaper than re-walking the hull's edges each time.
    ///
    /// [`PreparedGeometry`] is `Rc`-backed, so it can't be shared across threads - each server
    /// worker prepares its own copy.
    pub fn prepare(&self) -> PreparedOTPRouter {
        OTPRouter {
            endpoint: self.endpoint.clone(),
            coverage: PreparedGeometry::from(self.coverage.clone()),
            timezone: self.timezone,
        }
    }
}

impl PreparedOTPRouter {
    /// Whether this instance's coverage area contains `point`.
    ///
    /// As with [`geo::algorithm::Contains`], points on the boundary are not contained.
    pub fn contains(&self, point: &Point) -> bool {
        self.coverage.relate(point).is_contains()
    }

    pub fn polygon(&self) -> &Polygon {
        self.coverage.geometry()
    }
}

#[derive(Debug)]
pub struct OTPRouterClient {
    endpoint: Url,
    http_client: reqwest::Client,
}

// The convex hull of all stops. This replaces the coverage polygon that OTP's legacy REST
// `/otp/routers` endpoint used to return (it exposed the graph's hull directly). The GTFS
// GraphQL API has no equivalent "graph extent" query, so we reconstruct it from the stops.
//
// TODO: Ask upstream OTP to expose the graph's coverage area (extent/convex hull) via the GTFS
// GraphQL API so we don't have to fetch every stop and recompute it here.
// See https://github.com/opentripplanner/OpenTripPlanner
#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "QueryType")]
struct CoverageQuery {
    stops: Option<Vec<Option<Stop>>>,
    agencies: Option<Vec<Option<Agency>>>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Stop {
    lat: Option<f64>,
    lon: Option<f64>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Agency {
    timezone: String,
}

impl OTPRouterClient {
    /// `base_url` is the root of an OTP instance, e.g. `http://opentripplanner:8000`.
    pub fn new(base_url: Url) -> crate::Result<Self> {
        let http_client = reqwest::Client::new();
        Ok(Self {
            endpoint: gtfs_graphql::endpoint_url(&base_url)?,
            http_client,
        })
    }

    pub async fn fetch_all(&self) -> crate::Result<Vec<OTPRouter>> {
        let data: CoverageQuery =
            gtfs_graphql::post_graphql(&self.http_client, &self.endpoint, CoverageQuery::build(()))
                .await?;

        let points: Vec<Point> = data
            .stops
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .filter_map(|stop| Some(Point::new(stop.lon?, stop.lat?)))
            .collect();

        if points.is_empty() {
            return Err(crate::Error::server(format!(
                "OTP endpoint {} returned no stops; cannot compute coverage area",
                self.endpoint
            )));
        }

        let polygon = MultiPoint::new(points).convex_hull();

        let timezone = data
            .agencies
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .map(|agency| agency.timezone)
            .next()
            .and_then(|tz| match tz.parse::<Tz>() {
                Ok(tz) => Some(tz),
                Err(e) => {
                    log::warn!("could not parse OTP agency timezone {tz:?}: {e}");
                    None
                }
            });

        Ok(vec![OTPRouter::new(
            self.endpoint.clone(),
            polygon,
            timezone,
        )])
    }
}
