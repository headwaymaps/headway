use actix_web::{middleware::Logger, web, App, HttpServer};
use url::Url;

use std::env;
use travelmux::api::{self, AppState, Router};
use travelmux::otp::OtpRouterClient;
use travelmux::valhalla::ValhallaRouter;
use travelmux::{Result, ERROR_CODE_HEADER};

#[derive(Debug, PartialEq)]
enum RouterKind {
    Otp,
    Valhalla,
}

/// A router as given on the command line: `--otp=<id>=<url>` or `--valhalla=<id>=<url>`.
#[derive(Debug, PartialEq)]
struct RouterArg {
    kind: RouterKind,
    id: String,
    url: Url,
}

impl std::str::FromStr for RouterArg {
    type Err = String;

    fn from_str(arg: &str) -> std::result::Result<Self, String> {
        let (kind, rest) = if let Some(rest) = arg.strip_prefix("--otp=") {
            (RouterKind::Otp, rest)
        } else if let Some(rest) = arg.strip_prefix("--valhalla=") {
            (RouterKind::Valhalla, rest)
        } else {
            return Err(format!(
                "expected --otp=<id>=<url> or --valhalla=<id>=<url>, not {arg}"
            ));
        };
        let Some((id, url)) = rest.split_once('=').filter(|(id, _)| !id.is_empty()) else {
            return Err(format!("{arg} has no <id>="));
        };
        let url = Url::parse(url).map_err(|e| format!("{arg} has an invalid url: {e}"))?;
        Ok(Self {
            kind,
            id: id.to_string(),
            url,
        })
    }
}

impl RouterArg {
    async fn fetch(self) -> Result<Router> {
        log::info!("adding router {}: {}", self.id, self.url);
        Ok(match self.kind {
            RouterKind::Otp => Router::Otp(OtpRouterClient::new(self.url)?.fetch(&self.id).await?),
            RouterKind::Valhalla => {
                Router::Valhalla(ValhallaRouter::fetch(self.id, self.url).await?)
            }
        })
    }
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<RouterArg> = env::args()
        .skip(1)
        .map(|arg| arg.parse())
        .collect::<std::result::Result<_, _>>()
        .unwrap_or_else(|e| panic!("{e}"));
    if args.is_empty() {
        panic!("No routers specified. Usage: travelmux-server --otp='Seattle=https://otp.example.com' --valhalla='planet=https://valhalla.example.com'. A trip goes to the first router that plans its mode and covers it.");
    }
    for (i, arg) in args.iter().enumerate() {
        assert!(
            args[..i].iter().all(|other| other.id != arg.id),
            "two routers are called {}",
            arg.id
        );
    }

    // If we change this to be non-blocking, we'll
    // want to update our readiness probe in health.rs
    let mut routers = vec![];
    for arg in args {
        routers.push(arg.fetch().await?);
    }
    log::info!("setup completed with {} routers", routers.len());
    let app_state = web::Data::new(AppState::new(routers));

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
        App::new()
            .wrap(Logger::new(&access_log_format))
            .app_data(app_state.clone())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_args_name_their_kind_and_id() {
        let arg: RouterArg = "--otp=Puget Sound (Seattle Area)=http://otp:8000"
            .parse()
            .unwrap();
        assert_eq!(arg.kind, RouterKind::Otp);
        assert_eq!(arg.id, "Puget Sound (Seattle Area)");
        assert_eq!(arg.url.as_str(), "http://otp:8000/");

        let arg: RouterArg = "--valhalla=valhalla=http://valhalla:8002".parse().unwrap();
        assert_eq!(arg.kind, RouterKind::Valhalla);

        assert!("http://valhalla:8002".parse::<RouterArg>().is_err());
        assert!("--otp=http://otp:8000".parse::<RouterArg>().is_err());
    }
}
