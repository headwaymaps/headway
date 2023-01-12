use actix_web::{middleware::Logger, web, App, HttpServer};

use std::env;

use transitmux::Result;

mod app_state;
mod health;
mod plan;
use app_state::AppState;

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut app_state = AppState::default();

    // eventually we might want something more sophisticated, like CRUD'ing endpoints,
    // but for now we require a restart.
    let endpoints = env::args().skip(1);
    if endpoints.len() == 0 {
        let bin_name = env::args()
            .next()
            .unwrap_or_else(|| "<bin name>".to_string());
        panic!("No endpoints specified. Usage: {bin_name} https://endpoint1.example.com/otp/routers https://endpoint2.example.com/otp/routers")
    }

    for endpoint in endpoints {
        // If we change this to be non-blocking, we'll
        // want to update our readiness probe in health.rs
        app_state.add_endpoint(&endpoint).await?;
    }

    log::info!(
        "setup completed - there are {} routers.",
        app_state.cluster().router_len()
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
            .service(plan::get_plan)
            .service(health::get_ready)
            .service(health::get_alive)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    Ok(())
}
