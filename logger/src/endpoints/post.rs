use std::cmp;

use actix_web::{post, web, HttpResponse, Responder};
use chrono::{DateTime, Utc, ParseError, Duration, NaiveDateTime};
use serde::Deserialize;

use crate::entsoapi::fetch::fetch_prices_for_interval;

#[derive(Deserialize)]
pub struct TimeParams {
    start: String,
    stop: String,
    in_domain: Option<String>,
    out_domain: Option<String>,
}

/// Update day ahead price data `/dayahead`
#[post("/dayahead")]
pub async fn update_dayahead_prices(params: web::Json<TimeParams>) -> impl Responder {
    debug!("update_dayahead_prices requqest inbound");
    let security_token = dotenv::var("SECURITY_TOKEN").unwrap();
    let in_domain = params.in_domain.clone().unwrap_or(dotenv::var("IN_DOMAIN").unwrap());
    let out_domain = params.out_domain.clone().unwrap_or(dotenv::var("OUT_DOMAIN").unwrap());

    let start: Result<NaiveDateTime, ParseError> = NaiveDateTime::parse_from_str(&params.start, "%Y-%m-%dT%H:%MZ");
    let stop: Result<NaiveDateTime, ParseError> = NaiveDateTime::parse_from_str(&params.stop, "%Y-%m-%dT%H:%MZ");

    if start.is_err() || stop.is_err() {
        return HttpResponse::BadRequest().body("Invalid date format");
    }

    let start: DateTime<Utc> = DateTime::from_utc(start.unwrap(), Utc);
    let stop: DateTime<Utc> = DateTime::from_utc(stop.unwrap(), Utc);

    // let start = start.unwrap();
    // let stop = stop.unwrap();

    let max_duration = Duration::days(370);
    let mut current_start = start;

    while current_start < stop {
        let current_stop = cmp::min(current_start + max_duration, stop);

        if let Err(err) = fetch_prices_for_interval(
            &security_token,
            &in_domain,
            &out_domain,
            &format!("{}/{}", current_start.format("%Y-%m-%dT%H:%MZ").to_string(), current_stop.format("%Y-%m-%dT%H:%MZ").to_string()),
        )
        .await
        {
            // Handle the error here
            error!("Error fetching prices: {:?}", err);
            // Return an appropriate response
            return HttpResponse::InternalServerError().body(err.to_string());
        }

        current_start = current_stop;
    }
    
    return HttpResponse::Ok().body("ok")
}
