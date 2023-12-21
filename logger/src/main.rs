#[macro_use]
extern crate log;

use crate::endpoints::{health, post};
use crate::entsoapi::fetch::fetch_prices_for_interval;
use crate::entsoapi::fetch::get_fetch_time_interval;
use actix_web::{middleware, App, HttpServer};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use dotenv::dotenv;
use std::time::Duration;
use tokio::join;
use tokio::time::sleep;

mod endpoints;
mod entsoapi;
mod logging;
mod settings;
mod storage;

fn get_time_after_duration(duration: u64) -> String {
    let tz_now: DateTime<Tz> = Utc::now().with_timezone(&get_timezone());
    let time = tz_now + chrono::Duration::milliseconds(duration as i64);

    time.format("%Y-%m-%dT%H:%M:%S %Z").to_string()
}

fn get_timezone() -> Tz {
    let timezone = dotenv::var("CHRONO_TIMEZONE").unwrap_or("Europe/Helsinki".to_string());
    timezone.parse().unwrap()
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    logging::init_logging();

    info!("ENTSO-E Logger starting");
    info!("Using time zone: {}", get_timezone().name());

    let config = settings::config::load_settings(format!("configs/{}.yaml", "production"))
        .expect("Failed to load settings file.");

    if let Err(err) = config.validate() {
        panic!("Validation error: {}", err);
    }

    let interval: u64 = dotenv::var("INTERVAL")
        .map(|var| var.parse::<u64>())
        .unwrap_or(Ok(10_000))
        .unwrap();

    let security_token = dotenv::var("SECURITY_TOKEN").unwrap();
    let in_domain = dotenv::var("IN_DOMAIN").unwrap();
    let out_domain = dotenv::var("OUT_DOMAIN").unwrap();

    let run_server: bool = dotenv::var("ENABLE_REST_API")
        .unwrap_or_else(|_| String::from("false"))
        .parse()
        .unwrap_or(true);

    let run_update: bool = dotenv::var("ENABLE_AUTO_UPDATE")
        .unwrap_or_else(|_| String::from("false"))
        .parse()
        .unwrap_or(true);

    let server_task = async {
        let server = match HttpServer::new(move || {
            App::new()
                .wrap(middleware::Compress::default())
                // .app_data(web::Data::new(server_client.clone()))
                // register HTTP requests handlers
                .service(health::health_check)
                .service(post::update_dayahead_prices)
        })
        .bind("0.0.0.0:9092")
        {
            Ok(value) => {
                info!("REST API started at 0.0.0.0:9092");
                value
            }
            Err(error) => panic!("Error binding to socket:{:?}", error),
        };
        let _ = server.run().await;
    };

    let update_task = async {
        loop {
            let _ = fetch_prices_for_interval(
                &security_token,
                &in_domain,
                &out_domain,
                &get_fetch_time_interval(&in_domain, &out_domain)
                    .await
                    .to_string(),
            )
            .await;

            info!(
                "Logging done, waiting for the next fetch at {} ...",
                get_time_after_duration(interval)
            );
            sleep(Duration::from_millis(interval)).await;
        }
    };

    if run_server && run_update {
        info!("Running server and auto update");
        join!(server_task, update_task);
    } else if run_server {
        info!("Running server");
        server_task.await;
    } else if run_update {
        info!("Running auto update");
        update_task.await;
    } else {
        warn!("Not running server or update. Enable at least one of them in .env file with ENABLE_REST_API or ENABLE_AUTO_UPDATE.");
    }

    // join!(server_task, update_task);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_fetch_eet_eest() {
        dotenv().ok();

        let security_token = dotenv::var("SECURITY_TOKEN").unwrap();

        let in_domain = dotenv::var("IN_DOMAIN").unwrap();
        let out_domain = dotenv::var("OUT_DOMAIN").unwrap();

        let _ = fetch_prices_for_interval(
            &security_token,
            &in_domain,
            &out_domain,
            "2022-02-28T22:00Z/2022-03-31T21:00Z",
        )
        .await;
    }

    #[tokio::test]
    async fn test_get_fetch_eest_eet() {
        dotenv().ok();

        let security_token = dotenv::var("SECURITY_TOKEN").unwrap();

        let in_domain = dotenv::var("IN_DOMAIN").unwrap();
        let out_domain = dotenv::var("OUT_DOMAIN").unwrap();

        let _ = fetch_prices_for_interval(
            &security_token,
            &in_domain,
            &out_domain,
            "2022-10-29T21:00Z/2022-10-30T22:00Z",
        )
        .await;
    }
}
