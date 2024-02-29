use actix_web::{get, web, HttpResponse, Responder};

use crate::api::AppState;

#[get("/health/alive")]
pub async fn get_alive(_app_state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok()
}

#[get("/health/ready")]
pub async fn get_ready(_app_state: web::Data<AppState>) -> impl Responder {
    // Currently all setup happens before the server is listening
    // So if we are handling requests, we are ready!
    HttpResponse::Ok()
}
