use api::PublicationMarketDocument;
use chrono::Duration as ChronoDuration;
use chrono::{DateTime, Utc};
use influxdb::{Client, InfluxDbWriteable, ReadQuery};

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
            if start.is_none() {
                messages.push("InfluxDB | Skipping logging because start time couldn't be parsed".to_string());
                continue;
            }

            for point in period.point.iter() {
                let time = start.unwrap() + ChronoDuration::hours((point.position - 1).into());

                // Delete the current row if it's dirty
                delete_if_dirty(&client, in_domain, out_domain, &time).await;

                let current_data = PriceData {
                    time: time,
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
                    price: point.price,
                    dirty: None,
                };

                let write_result = client
                    .query(&current_data.into_query("dayAheadPrices"))
                    .await;
                if let Err(err) = write_result {
                    error!("Error writing to db: {}", err)
                }

                messages.push(format!("InfluxDB | {} - {:.2}", time, point.price));
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

    Client::new(&database_url, &database_name)
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
