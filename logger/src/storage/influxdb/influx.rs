use std::str::FromStr;

use api::PublicationMarketDocument;
use chrono::Duration as ChronoDuration;
use chrono::{DateTime, Utc};
use influxdb::{Client, InfluxDbWriteable, ReadQuery};
use iso8601_duration::Duration as IsoDuration;

use super::price_data::PriceData;

pub fn is_enabled() -> bool {
    dotenv::var("INFLUXDB_ENABLED")
        .map(|var| var.parse::<bool>())
        .unwrap_or(Ok(false))
        .unwrap()
}

pub async fn upsert_document_into_influxdb(
    document: &PublicationMarketDocument,
    in_domain: &str,
    out_domain: &str,
) -> Result<(), anyhow::Error> {
    if !is_enabled() {
        return Ok(());
    }

    let mut messages = Vec::new();

    let client = connect_to_db().await;
    for time_serie in document.time_series.iter() {
        for period in time_serie.period.iter() {
            let start = &period.time_interval.start_as_utc();
            let end = &period.time_interval.end_as_utc();
            if start.is_none() || end.is_none() {
                messages.push("InfluxDB | Skipping logging because start or end time couldn't be parsed".to_string());
                continue;
            }
        
            let parsed_duration = IsoDuration::from_str(&period.resolution).expect("Failed to parse duration");
            let resolution = ChronoDuration::seconds(parsed_duration.to_std().unwrap().as_secs() as i64);
            let mut last_price = None;
            let mut current_time = start.unwrap();
        
            while current_time < end.unwrap() {
                let position = ((current_time - start.unwrap()).num_seconds() / resolution.num_seconds()) + 1;
                let point = period.point.iter().find(|p| p.position == position as i32);
                let price = if let Some(point) = point {
                    last_price = Some(point.price);
                    point.price
                } else {
                    last_price.unwrap_or(0.0)
                };
        
                // Delete the current row if it's dirty
                delete_if_dirty(&client, in_domain, out_domain, &current_time).await;
        
                let current_data = PriceData {
                    time: current_time,
                    type_tag: document.r#type.to_string(),
                    in_domain_tag: in_domain.to_string(),
                    out_domain_tag: out_domain.to_string(),
                    document_type: document.r#type.to_string(),
                    in_domain: in_domain.to_string(),
                    out_domain: out_domain.to_string(),
                    currency: time_serie.currency_unit.to_string(),
                    price_measure: time_serie.price_measure_unit.to_string(),
                    curve_type: time_serie.curve_type.to_string(),
                    timestamp: current_time.format("%Y-%m-%dT%H:%MZ").to_string(),
                    price: price,
                    dirty: None,
                };
        
                let write_result = client
                    .query(&current_data.into_query("dayAheadPrices"))
                    .await;
                if let Err(err) = write_result {
                    error!("Error writing to db: {}", err)
                }
        
                messages.push(format!("InfluxDB | {} - {:.2}", current_time, price));
        
                current_time = current_time + resolution;
            }
        }
    }

    let all_messages = messages.join("\n");
    info!("{}", all_messages);

    Ok(())
}

pub async fn get_latest_time(in_domain: &str, out_domain: &str) -> Option<chrono::DateTime<Utc>> {
    let client = connect_to_db().await;

    let read_query = ReadQuery::new(format!("SELECT * FROM (SELECT * FROM dayAheadPrices fill(-111)) WHERE type_tag='A44' AND in_domain_tag='{}' AND out_domain_tag='{}' AND dirty = -111 ORDER BY time DESC LIMIT 1", in_domain, out_domain));

    let read_result = client
        .json_query(read_query)
        .await
        .and_then(|mut db_result| db_result.deserialize_next::<PriceData>());

    // let read_result = client.query(read_query).await;
    match read_result {
        Ok(result) => {
            if result.series.len() > 0 && result.series[0].values.len() > 0 {
                let data = &result.series[0].values[0];
                debug!("{:?}", data);
                return Some(data.time);
            }
        }
        Err(err) => {
            error!("Error reading dayAheadPrices from the db: {}", err);
        }
    }

    None
}

async fn connect_to_db() -> Client {
    let database_url = dotenv::var("DATABASE_URL").unwrap_or("http://localhost:8086".to_string());
    let database_name = dotenv::var("DATABASE_NAME").unwrap_or("entsoe".to_string());
    let username = dotenv::var("INFLUXDB_USERNAME").unwrap_or("".to_string());
    let password = dotenv::var("INFLUXDB_PASSWORD").unwrap_or("".to_string());

    let client = Client::new(&database_url, &database_name);
    if !username.is_empty() && !password.is_empty() {
        client.with_auth(&username, &password)
    } else {
        client
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
            if result.series.len() > 0 && result.series[0].values.len() > 0 {
                info!("Query: {}", format!("DELETE FROM dayAheadPrices WHERE type_tag='A44' AND in_domain_tag='{}' AND out_domain_tag='{}' AND time = '{}' AND dirty = 1", in_domain, out_domain, time.to_rfc3339()));
                // let read_query = ReadQuery::new(format!("DELETE FROM dayAheadPrices WHERE time = '{}'", time.to_rfc3339()));
                let read_query = ReadQuery::new(format!("DELETE FROM dayAheadPrices WHERE type_tag='A44' AND in_domain_tag='{}' AND out_domain_tag='{}' AND time = '{}'", in_domain, out_domain, time.to_rfc3339()));

                let read_result = client.query(read_query).await;
                match read_result {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Error deleting dayAheadPrice from the db: {}", err);
                    }
                }
            }
        }
        Err(err) => {
            error!("Error reading dayAheadPrices from the db: {}", err);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::dotenv;

    use super::*;

    #[tokio::test]
    async fn test_get_latest_time() {
        dotenv().ok();

        let in_domain = dotenv::var("IN_DOMAIN").unwrap();
        let out_domain = dotenv::var("OUT_DOMAIN").unwrap();

        let response = get_latest_time(&in_domain, &out_domain).await;
        info!("Last time in InfluxDB is {:?}", response);
    }
}
