use actix_web::{middleware::Logger, web, App, HttpServer};
use url::Url;

use std::env;

use travelmux::api::{self, AppState};
use travelmux::Result;

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
        panic!("No endpoints specified. Usage: {bin_name} https://valhalla.example.com https://endpoint1.example.com/otp/routers https://endpoint2.example.com/otp/routers")
    };

    let Ok(valhalla_endpoint) = Url::parse(&valhalla_endpoint) else {
        panic!("Invalid valhalla endpoint: {valhalla_endpoint}")
    };
    let mut app_state = AppState::new(valhalla_endpoint);

    for endpoint in endpoints {
        // If we change this to be non-blocking, we'll
        // want to update our readiness probe in health.rs
        app_state.add_otp_endpoint(&endpoint).await?;
    }

    log::info!(
        "setup completed - there are {} routers.",
        app_state.otp_cluster().router_len()
    );

    let port: u16 = std::env::var("PORT")
        .map(|s| {
            s.parse()
                .unwrap_or_else(|_| panic!("malformed PORT specified: `{s}`"))
        })
        .unwrap_or(8000);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(app_state.clone()))
            .service(api::v4::plan::get_plan)
            .service(api::v5::plan::get_plan)
            .service(api::v6::plan::get_plan)
            .service(api::v6::directions::get_directions)
            .service(api::health::get_ready)
            .service(api::health::get_alive)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    Ok(())
}
