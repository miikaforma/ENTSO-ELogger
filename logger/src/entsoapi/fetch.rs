use std::cmp;

use crate::settings;
use crate::storage::influxdb::influx::{self, upsert_document_into_influxdb};
use crate::storage::timescaledb::timescale::{self, upsert_document_into_timescaledb, refresh_views};
use api::day_ahead_prices;
use chrono::Duration as ChronoDuration;
use chrono::{NaiveDateTime, TimeZone, Utc};

pub async fn fetch_prices_for_interval(
    security_token: &str,
    in_domain: &str,
    out_domain: &str,
    time_interval: &str,
) -> Result<(), anyhow::Error> {
    info!(
        "Fetching prices for interval {} in domain {}",
        &time_interval, &out_domain
    );

    let config = settings::config::load_settings(format!("configs/{}.yaml", "production"))
        .expect("Failed to load settings file.");

    match day_ahead_prices(&security_token, &in_domain, &out_domain, &time_interval).await {
        Ok(data) => {
            info!(
                "Fetched document created at {}",
                data.created_date_time_as_utc().unwrap()
            );
            
            let timescale_future = upsert_document_into_timescaledb(&data, &in_domain, &out_domain, &config);
            let influx_future = upsert_document_into_influxdb(&data, &in_domain, &out_domain);
        
            let (timescale_result, influx_result) = tokio::join!(timescale_future, influx_future);

            if timescale_result.is_err() {
                error!("Error inserting into TimescaleDB: {:?}", timescale_result);
            }

            if influx_result.is_err() {
                error!("Error inserting into InfluxDB: {:?}", influx_result);
            }

            if timescale::is_enabled() {
                if let Err(err) = refresh_views().await {
                    // Handle the error here
                    error!("Error refreshing the prices views: {:?}", err);
                }
            }

            Ok(())
        }
        Err(err) => Err(anyhow::anyhow!(err)),
    }
}

// Example of the format: 2022-06-30T21:00Z/2022-07-31T21:00Z
pub async fn get_fetch_time_interval(in_domain: &str, out_domain: &str) -> String {
    let mut start_time = chrono::offset::Utc::now();
    let naive_time = NaiveDateTime::parse_from_str(
        &dotenv::var("START_TIME").unwrap_or("".to_string()),
        "%Y-%m-%dT%H:%MZ",
    );
    if naive_time.is_ok() {
        start_time = Utc.from_utc_datetime(&naive_time.unwrap());
    }

    let latest_timescale = timescale::get_latest_time(&in_domain, &out_domain)
        .await
        .unwrap_or(start_time);
    let latest_influx = influx::get_latest_time(&in_domain, &out_domain)
        .await
        .unwrap_or(start_time);

    debug!("Start time: {}", start_time);
    debug!("Latest TimescaleDB time: {}", latest_timescale);
    debug!("Latest InfluxDB time: {}", latest_influx);

    let start_time = cmp::max(start_time, cmp::min(latest_timescale, latest_influx));

    let days: i64 = dotenv::var("INTERVAL_DAYS")
        .map(|var| var.parse::<i64>())
        .unwrap_or(Ok(1))
        .unwrap();
    let end_time = start_time + ChronoDuration::days(days);
    format!(
        "{}/{}",
        start_time.format("%Y-%m-%dT%H:00Z"),
        end_time.format("%Y-%m-%dT%H:00Z")
    )
}

#[cfg(test)]
mod tests {
    use crate::dotenv;

    use super::*;

    #[tokio::test]
    async fn test_get_fetch_time_interval() {
        dotenv().ok();

        let in_domain = dotenv::var("IN_DOMAIN").unwrap();
        let out_domain = dotenv::var("OUT_DOMAIN").unwrap();

        let response = get_fetch_time_interval(&in_domain, &out_domain).await;
        info!("Fetch interval {:?}", response);
    }
}
