use actix_web::{middleware::Logger, web, App, HttpServer};
use url::Url;

use std::env;
use travelmux::api::{self, AppState};
use travelmux::otp::OtpCluster;
use travelmux::{Result, ERROR_CODE_HEADER};

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init();

    // eventually we might want something more sophisticated, like CRUD'ing endpoints,
    // but for now we require a restart.
    let mut endpoints = env::args().skip(1);

    let Some(valhalla_endpoint) = endpoints.next() else {
        let bin_name = env::args()
            .next()
            .unwrap_or_else(|| "<bin name>".to_string());
        panic!("No endpoints specified. Usage: {bin_name} https://valhalla.example.com 'Area One=https://otp1.example.com' 'Area Two=https://otp2.example.com'")
    };

    let Ok(valhalla_endpoint) = Url::parse(&valhalla_endpoint) else {
        panic!("Invalid valhalla endpoint: {valhalla_endpoint}")
    };

    let mut otp_cluster = OtpCluster::default();
    for endpoint in endpoints {
        let Some((name, url)) = endpoint
            .split_once('=')
            .filter(|(name, _)| !name.is_empty())
        else {
            panic!("OTP endpoint must be <area name>=<url>, e.g. 'Seattle=http://opentripplanner:8000', but was: {endpoint}")
        };
        // If we change this to be non-blocking, we'll
        // want to update our readiness probe in health.rs
        otp_cluster.insert_endpoint(name, url).await?;
    }

    log::info!(
        "setup completed - there are {} routers.",
        otp_cluster.router_len()
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
        let app_state = AppState::new(valhalla_endpoint.clone(), otp_cluster.prepare());
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
