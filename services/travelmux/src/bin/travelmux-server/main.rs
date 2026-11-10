use actix_web::{middleware::Logger, web, App, HttpServer};
use url::Url;

use std::env;
use travelmux::api::{self, AppState, Router};
use travelmux::otp::OtpCluster;
use travelmux::valhalla::ValhallaRouter;
use travelmux::{Result, ERROR_CODE_HEADER};

/// What a request calls a router without a `<slug>@` prefix.
const DEFAULT_OTP_SLUG: &str = "otp";
const DEFAULT_VALHALLA_SLUG: &str = "valhalla";

/// A router as given on the command line, before each worker prepares its own.
#[derive(Clone)]
enum RouterArg {
    Otp(OtpCluster),
    Valhalla(Url),
}

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
        panic!("No endpoints specified. Usage: {bin_name} 'Area One=https://otp1.example.com' 'Area Two=https://otp2.example.com' https://valhalla.example.com. A trip goes to the first that suits its mode and covers it. Prefix an endpoint with <slug>@ for requests to name it; OTP endpoints sharing a slug are one router.")
    }

    let mut routers: Vec<(String, RouterArg)> = vec![];
    for endpoint in &endpoints {
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
                let index = match routers.iter().position(|(s, _)| s == slug) {
                    Some(index) => index,
                    None => {
                        routers.push((slug.to_string(), RouterArg::Otp(OtpCluster::default())));
                        routers.len() - 1
                    }
                };
                let RouterArg::Otp(cluster) = &mut routers[index].1 else {
                    panic!("{slug} names both a valhalla and an OTP endpoint");
                };
                // If we change this to be non-blocking, we'll
                // want to update our readiness probe in health.rs
                cluster.insert_endpoint(area, url).await?;
            }
            None => {
                let slug = slug.unwrap_or(DEFAULT_VALHALLA_SLUG);
                assert!(
                    routers.iter().all(|(s, _)| s != slug),
                    "two routers are called {slug}"
                );
                let url = Url::parse(endpoint)
                    .unwrap_or_else(|_| panic!("Invalid valhalla endpoint: {endpoint}"));
                routers.push((slug.to_string(), RouterArg::Valhalla(url)));
            }
        }
    }

    log::info!(
        "setup completed - routers: {:?}",
        routers.iter().map(|(slug, _)| slug).collect::<Vec<_>>()
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
            routers
                .iter()
                .map(|(slug, router)| {
                    let router = match router {
                        RouterArg::Otp(cluster) => Router::Otp(cluster.prepare()),
                        RouterArg::Valhalla(url) => {
                            Router::Valhalla(ValhallaRouter::new(url.clone()))
                        }
                    };
                    (slug.clone(), router)
                })
                .collect(),
        );
        App::new()
            .wrap(Logger::new(&access_log_format))
            .app_data(web::Data::new(app_state))
            .service(api::v6::plan::get_plan)
            .service(api::v6::directions::get_directions)
            .service(api::v7::plan::get_plan)
            .service(api::v7::directions::get_directions)
            .service(api::v8::plan::get_plan)
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
