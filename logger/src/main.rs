use std::time::Duration;
use chrono::Duration as ChronoDuration;

use api::TimeSeries;
use api::day_ahead_prices;
use api::PublicationMarketDocument;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use influxdb::{Client};
use influxdb::InfluxDbWriteable;
use influxdb::ReadQuery;
use tokio::time::sleep;
use serde::{Deserialize, Serialize};
use actix_web::{middleware, web, App, HttpServer};
use crate::endpoints::post;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::join;

mod endpoints;

#[derive(Debug, InfluxDbWriteable, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct PriceData {
    time: DateTime<Utc>,
    #[influxdb(tag)]
    type_tag: String,
    #[influxdb(tag)]
    in_domain_tag: String,
    #[influxdb(tag)]
    out_domain_tag: String,
    document_type: String,
    in_domain: String,
    out_domain: String,
    currency: String,
    price_measure: String,
    curve_type: String,
    timestamp: String,
    price: f32,
    dirty: Option<i32>
}

async fn fetch_and_log_new_entries(
    client: &Client,
    security_token: &str,
    in_domain: &str,
    out_domain: &str,
    time_interval: &str,
) {
    println!("Fetching new entries for domain {}", &out_domain);

    match day_ahead_prices(&security_token, &in_domain, &out_domain, &time_interval).await {
        Ok(data) => log_new_day_ahead_prices(client, &in_domain, &out_domain, &data).await,
        Err(_) => {}
    }
}

async fn log_new_day_ahead_prices(client: &Client,
    in_domain: &str,
    out_domain: &str,
    data: &PublicationMarketDocument) {
    println!("Document created at {}", data.created_date_time_as_utc().unwrap());

    for time_serie in data.time_series.iter() {
        for period in time_serie.period.iter() {
            let start = &period.time_interval.start_as_utc();
            if start.is_none() {
                println!("Skipping logging because start time couldn't be parsed");
                continue;
            }

            for point in period.point.iter() {
                // println!("{} - {:?}", start.unwrap() + Duration::hours((point.position - 1).into()), point);

                log_new_day_ahead_price(client, &in_domain, &out_domain, &data, &time_serie, &(start.unwrap() + ChronoDuration::hours((point.position - 1).into())), point.price).await;
            }
        }
    }
}

async fn log_new_day_ahead_price(client: &Client,
    in_domain: &str,
    out_domain: &str,
    document: &PublicationMarketDocument,
    time_serie: &TimeSeries,
    time: &DateTime<Utc>,
    price: f32) {
    println!("Logging UTC: {:?} - {}", time, price);

    // Delete the current row if it's dirty
    delete_if_dirty(client, in_domain, out_domain, time).await;

    let current_data = PriceData {
        time: *time,
        type_tag: document.r#type.to_string(),
        in_domain_tag: in_domain.to_string(),
        out_domain_tag: out_domain.to_string(),
        document_type: document.r#type.to_string(),
        in_domain: in_domain.to_string(),
        out_domain: out_domain.to_string(),
        currency: time_serie.currency_unit.to_string(),
        price_measure: time_serie.price_measure_unit.to_string(),
        curve_type: time_serie.curve_type.to_string(),
        timestamp: time.format("%Y-%m-%dT%H:%MZ").to_string(),
        price: price,
        dirty: None,
    };

    let write_result = client
        .query(&current_data.into_query("dayAheadPrices"))
        .await;
    if let Err(err) = write_result {
        eprintln!("Error writing to db: {}", err)
    }
}

async fn delete_if_dirty(client: &Client, in_domain: &str, out_domain: &str, time: &DateTime<Utc>) {
    let read_query = ReadQuery::new(format!("SELECT * FROM dayAheadPrices WHERE type_tag='A44' AND in_domain_tag='{}' AND out_domain_tag='{}' AND time = '{}' AND dirty = 1 ORDER BY time DESC LIMIT 1", in_domain, out_domain, time.to_rfc3339()));

    let read_result = client
        .json_query(read_query)
        .await
        .and_then(|mut db_result| db_result.deserialize_next::<PriceData>());

    match read_result {
        Ok(result) => {
            if result.series.len() > 0 && result.series[0].values.len() > 0
            {
                println!("Query: {}", format!("DELETE FROM dayAheadPrices WHERE type_tag='A44' AND in_domain_tag='{}' AND out_domain_tag='{}' AND time = '{}' AND dirty = 1", in_domain, out_domain, time.to_rfc3339()));
                // let read_query = ReadQuery::new(format!("DELETE FROM dayAheadPrices WHERE time = '{}'", time.to_rfc3339()));
                let read_query = ReadQuery::new(format!("DELETE FROM dayAheadPrices WHERE type_tag='A44' AND in_domain_tag='{}' AND out_domain_tag='{}' AND time = '{}'", in_domain, out_domain, time.to_rfc3339()));

                let read_result = client.query(read_query).await;
                match read_result {
                    Ok(_) => {
                    }
                    Err(err) => {
                        eprintln!("Error deleting dayAheadPrice from the db: {}", err);
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("Error reading dayAheadPrices from the db: {}", err);
        }
    }
}

// Example of the format: 2022-06-30T21:00Z/2022-07-31T21:00Z
async fn get_fetch_time_interval(client: &Client, in_domain: &str, out_domain: &str, ) -> String {
    let read_query = ReadQuery::new(format!("SELECT * FROM (SELECT * FROM dayAheadPrices fill(-111)) WHERE type_tag='A44' AND in_domain_tag='{}' AND out_domain_tag='{}' AND dirty = -111 ORDER BY time DESC LIMIT 1", in_domain, out_domain));
    // let read_query = ReadQuery::new(format!("SELECT * FROM dayAheadPrices WHERE type_tag='A44' AND in_domain_tag='{}' AND out_domain='{}' ORDER BY time DESC LIMIT 1", in_domain, out_domain));

    let mut start_time = chrono::offset::Utc::now();
    let naive_time = NaiveDateTime::parse_from_str(&dotenv::var("START_TIME").unwrap_or("".to_string()), "%Y-%m-%dT%H:%MZ");
    if naive_time.is_ok() {
        start_time = Utc.from_utc_datetime(&naive_time.unwrap());
    }

    let read_result = client
        .json_query(read_query)
        .await
        .and_then(|mut db_result| db_result.deserialize_next::<PriceData>());

    // let read_result = client.query(read_query).await;
    match read_result {
        Ok(result) => {
            if result.series.len() > 0 && result.series[0].values.len() > 0
            {
                let data = &result.series[0].values[0];
                // println!("{:?}", data);
                start_time = data.time;
            }
        },
        Err(err) => {
            eprintln!("Error reading dayAheadPrices from the db: {}", err);
        }
    }

    let days: i64 = dotenv::var("INTERVAL_DAYS")
        .map(|var| var.parse::<i64>())
        .unwrap_or(Ok(1))
        .unwrap();
    let end_time = start_time + ChronoDuration::days(days);
    format!("{}/{}", start_time.format("%Y-%m-%dT%H:00Z"), end_time.format("%Y-%m-%dT%H:00Z"))
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = dotenv::var("DATABASE_URL").unwrap_or("http://localhost:8086".to_string());
    let database_name = dotenv::var("DATABASE_NAME").unwrap_or("entsoe".to_string());

    let interval: u64 = dotenv::var("INTERVAL")
        .map(|var| var.parse::<u64>())
        .unwrap_or(Ok(10_000))
        .unwrap();

    let security_token = dotenv::var("SECURITY_TOKEN").unwrap();
    let in_domain = dotenv::var("IN_DOMAIN").unwrap();
    let out_domain = dotenv::var("OUT_DOMAIN").unwrap();

    // Connect to database
    let client = Arc::new(Mutex::new(Client::new(database_url, database_name)));
    let server_client = Arc::clone(&client);

    let server = match HttpServer::new(move || {
        let app_client = Arc::clone(&server_client);

        App::new()
            .wrap(middleware::Compress::default())
            .app_data(web::Data::new(app_client))
            // register HTTP requests handlers
            .service(post::update_dayahead_prices)
    })
        .bind("0.0.0.0:9092")
    {
        Ok(value) => {
            println!("REST API started at 0.0.0.0:9092");
            value
        },
        Err(error) => panic!("Error binding to socket:{:?}", error),
    };

    let server_task = async {
        let _ = server.run().await;
    };

    let update_task = async {
        let update_client = Arc::clone(&client);

        loop {
            let client_ref = update_client.lock().await;
            fetch_and_log_new_entries(
                &(*client_ref),
                &security_token,
                &in_domain,
                &out_domain,
                &get_fetch_time_interval(&(*client_ref), &in_domain, &out_domain).await.to_string(),
            )
                .await;
            sleep(Duration::from_millis(interval)).await;
        }
    };

    join!(server_task, update_task);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_fetch_time_interval() {
        dotenv().ok();

        let database_url = dotenv::var("DATABASE_URL").unwrap_or("http://localhost:8086".to_string());
        let database_name = dotenv::var("DATABASE_NAME").unwrap_or("entsoe".to_string());

        // Connect to database
        let client = Client::new(database_url, database_name);

        let in_domain = dotenv::var("IN_DOMAIN").unwrap();
        let out_domain = dotenv::var("OUT_DOMAIN").unwrap();

        let response = get_fetch_time_interval(&client, &in_domain, &out_domain).await;
        println!("Fetch interval {}", response);
    }

    #[tokio::test]
    async fn test_get_fetch_eet_eest() {
        dotenv().ok();

        let database_url = dotenv::var("DATABASE_URL").unwrap_or("http://localhost:8086".to_string());
        let database_name = dotenv::var("DATABASE_NAME").unwrap_or("solarman".to_string());
        let security_token = dotenv::var("SECURITY_TOKEN").unwrap();

        // Connect to database
        let client = Client::new(database_url, database_name);

        let in_domain = dotenv::var("IN_DOMAIN").unwrap();
        let out_domain = dotenv::var("OUT_DOMAIN").unwrap();

        fetch_and_log_new_entries(
            &client,
            &security_token,
            &in_domain,
            &out_domain,
            "2022-02-28T22:00Z/2022-03-31T21:00Z",
        ).await;
    }

    #[tokio::test]
    async fn test_get_fetch_eest_eet() {
        dotenv().ok();

        let database_url = dotenv::var("DATABASE_URL").unwrap_or("http://localhost:8086".to_string());
        let database_name = dotenv::var("DATABASE_NAME").unwrap_or("solarman".to_string());
        let security_token = dotenv::var("SECURITY_TOKEN").unwrap();

        // Connect to database
        let client = Client::new(database_url, database_name);

        let in_domain = dotenv::var("IN_DOMAIN").unwrap();
        let out_domain = dotenv::var("OUT_DOMAIN").unwrap();

        fetch_and_log_new_entries(
            &client,
            &security_token,
            &in_domain,
            &out_domain,
            "2022-10-29T21:00Z/2022-10-30T22:00Z",
        ).await;
    }

    #[tokio::test]
    async fn test_delete_if_dirty() {
        dotenv().ok();

        let database_url = dotenv::var("DATABASE_URL").unwrap_or("http://localhost:8086".to_string());
        let database_name = dotenv::var("DATABASE_NAME").unwrap_or("entsoe".to_string());

        // Connect to database
        let client = Client::new(database_url, database_name);

        let in_domain = dotenv::var("IN_DOMAIN").unwrap();
        let out_domain = dotenv::var("OUT_DOMAIN").unwrap();
        let time = Utc.from_utc_datetime(&NaiveDateTime::parse_from_str("2023-02-12T23:00:00", "%Y-%m-%dT%H:%M:%S").unwrap());

        let response = delete_if_dirty(&client, &in_domain, &out_domain, &time).await;
        println!("Deleted {}", time.to_rfc3339());
    }
}

