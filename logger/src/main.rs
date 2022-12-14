use std::time::Duration;
use chrono::Duration as ChronoDuration;

use api::TimeSeries;
use api::day_ahead_prices;
use api::PublicationMarketDocument;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::{DateTime, Utc};
use dotenv::dotenv;
use influxdb::Client;
use influxdb::InfluxDbWriteable;
use influxdb::ReadQuery;
use tokio::time::sleep;
use serde::{Deserialize, Serialize};

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
    };

    let write_result = client
        .query(&current_data.into_query("dayAheadPrices"))
        .await;
    if let Err(err) = write_result {
        eprintln!("Error writing to db: {}", err)
    }
}

// Example of the format: 2022-06-30T21:00Z/2022-07-31T21:00Z
async fn get_fetch_time_interval(client: &Client, in_domain: &str, out_domain: &str, ) -> String {
    let read_query = ReadQuery::new(format!("SELECT * FROM dayAheadPrices WHERE type_tag='A44' AND in_domain_tag='{}' AND out_domain='{}' ORDER BY time DESC LIMIT 1", in_domain, out_domain));

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
    let client = Client::new(database_url, database_name);

    loop {
        fetch_and_log_new_entries(
            &client,
            &security_token,
            &in_domain,
            &out_domain,
            &get_fetch_time_interval(&client, &in_domain, &out_domain).await.to_string(),
        )
        .await;
        sleep(Duration::from_millis(interval)).await;
    }
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
}

