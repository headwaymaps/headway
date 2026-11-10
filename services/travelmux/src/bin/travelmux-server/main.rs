use actix_web::{middleware::Logger, web, App, HttpServer};
use url::Url;

use std::env;
use travelmux::api::{self, AppState};
use travelmux::otp::OtpCluster;
use travelmux::{Result, ERROR_CODE_HEADER};

/// In place of the valhalla endpoint, has OTP plan every trip.
const NO_VALHALLA: &str = "no-valhalla";
/// What a request calls the router an endpoint without a `<slug>@` prefix belongs to.
const DEFAULT_OTP_SLUG: &str = "otp";
const DEFAULT_VALHALLA_SLUG: &str = "valhalla";

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init();

    // eventually we might want something more sophisticated, like CRUD'ing endpoints,
    // but for now we require a restart.
    let endpoints: Vec<String> = env::args().skip(1).collect();
    if endpoints.is_empty() {
        let bin_name = env::args()
            .next()
            .unwrap_or_else(|| "<bin name>".to_string());
        panic!("No endpoints specified. Usage: {bin_name} <https://valhalla.example.com|{NO_VALHALLA}> 'Area One=https://otp1.example.com' 'Area Two=https://otp2.example.com'. Prefix an endpoint with <slug>@ for requests to name it. The first OTP and Valhalla plan requests that don't name one.")
    }

    let mut otp_clusters: Vec<(String, OtpCluster)> = vec![];
    let mut valhalla_endpoints: Vec<(String, Url)> = vec![];
    for endpoint in endpoints.iter().filter(|e| *e != NO_VALHALLA) {
        let (slug, endpoint) = match endpoint.split_once('@') {
            Some((slug, rest)) if !slug.is_empty() && !slug.contains(['/', ':']) => {
                (Some(slug), rest)
            }
            _ => (None, endpoint.as_str()),
        };
        match endpoint
            .split_once('=')
            .filter(|(area, _)| !area.is_empty())
        {
            Some((area, url)) => {
                let slug = slug.unwrap_or(DEFAULT_OTP_SLUG);
                let index = match otp_clusters.iter().position(|(s, _)| s == slug) {
                    Some(index) => index,
                    None => {
                        otp_clusters.push((slug.to_string(), OtpCluster::default()));
                        otp_clusters.len() - 1
                    }
                };
                // If we change this to be non-blocking, we'll
                // want to update our readiness probe in health.rs
                otp_clusters[index].1.insert_endpoint(area, url).await?;
            }
            None => {
                let slug = slug.unwrap_or(DEFAULT_VALHALLA_SLUG);
                assert!(
                    valhalla_endpoints.iter().all(|(s, _)| s != slug),
                    "two valhalla endpoints are called {slug}"
                );
                let url = Url::parse(endpoint)
                    .unwrap_or_else(|_| panic!("Invalid valhalla endpoint: {endpoint}"));
                valhalla_endpoints.push((slug.to_string(), url));
            }
        }
    }
    if otp_clusters.is_empty() {
        otp_clusters.push((DEFAULT_OTP_SLUG.to_string(), OtpCluster::default()));
    }

    log::info!(
        "setup completed - there are {} OTP routers.",
        otp_clusters
            .iter()
            .map(|(_, cluster)| cluster.router_len())
            .sum::<usize>()
    );

    let port: u16 = std::env::var("PORT")
        .map(|s| {
            s.parse()
                .unwrap_or_else(|_| panic!("malformed PORT specified: `{s}`"))
        })
        .unwrap_or(8000);

    // actix's default access log format, plus the error code so log readers can tell error types apart.
    let access_log_format = format!(
        r#"%a "%r" %s %b "%{{Referer}}i" "%{{User-Agent}}i" %T error_code=%{{{ERROR_CODE_HEADER}}}o"#
    );

    HttpServer::new(move || {
        // The OTP coverage areas can't be shared across threads, so each worker prepares its own.
        let app_state = AppState::new(
            otp_clusters
                .iter()
                .map(|(slug, cluster)| (slug.clone(), cluster.prepare()))
                .collect(),
            valhalla_endpoints.clone(),
        );
        App::new()
            .wrap(Logger::new(&access_log_format))
            .app_data(web::Data::new(app_state))
            .service(api::v6::plan::get_plan)
            .service(api::v6::directions::get_directions)
            .service(api::v7::plan::get_plan)
            .service(api::v7::directions::get_directions)
            .service(api::v8::plan::get_plan)
            .service(api::v8::plan::get_compare)
            .service(api::v8::directions::get_directions)
            .service(api::v8::vehicle_positions::post_vehicle_positions)
            .service(api::health::get_ready)
            .service(api::health::get_alive)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    Ok(())
}
