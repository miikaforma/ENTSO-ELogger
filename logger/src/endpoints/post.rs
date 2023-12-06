use crate::{fetch_and_log_new_entries};
use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Deserialize)]
pub struct TimeParams {
    start: String,
    stop: String,
    in_domain: Option<String>,
    out_domain: Option<String>,
}

/// Update day ahead price data `/dayahead`
#[post("/dayahead")]
pub async fn update_dayahead_prices(params: web::Json<TimeParams>, data: web::Data<Arc<Mutex<influxdb::Client>>>) -> impl Responder {
    let client = data.lock().await;
    let security_token = dotenv::var("SECURITY_TOKEN").unwrap();
    let in_domain = params.in_domain.clone().unwrap_or(dotenv::var("IN_DOMAIN").unwrap());
    let out_domain = params.out_domain.clone().unwrap_or(dotenv::var("OUT_DOMAIN").unwrap());

    fetch_and_log_new_entries(
        &client,
        &security_token,
        &in_domain,
        &out_domain,
        &format!("{}/{}", &params.start, &params.stop),
    )
    .await;

    return HttpResponse::Ok().body("ok")
}
