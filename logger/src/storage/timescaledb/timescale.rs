use api::PublicationMarketDocument;
use chrono::{Duration as ChronoDuration, Utc};
use tokio_postgres::{Error, NoTls};

use crate::settings::config_model::SettingsConfig;

pub fn is_enabled() -> bool {
    dotenv::var("TIMESCALEDB_ENABLED")
        .map(|var| var.parse::<bool>())
        .unwrap_or(Ok(false))
        .unwrap()
}

pub async fn upsert_document_into_timescaledb(
    document: &PublicationMarketDocument,
    in_domain: &str,
    out_domain: &str,
    settings: &SettingsConfig,
) -> Result<(), Error> {
    if !is_enabled() {
        return Ok(());
    }

    let mut messages = Vec::new();

    let mut client = connect_to_db().await?;
    let trans = client.transaction().await?;
    for time_serie in document.time_series.iter() {
        for period in time_serie.period.iter() {
            let start = &period.time_interval.start_as_utc();
            if start.is_none() {
                messages.push(
                    "TimescaleDB | Skipping logging because start time couldn't be parsed"
                        .to_string(),
                );
                continue;
            }

            for point in period.point.iter() {
                let time = start.unwrap() + ChronoDuration::hours((point.position - 1).into());
                let tax_percentage: f32 = settings.get_current_tax_percentage(time);
                let _ = trans
                    .execute("INSERT INTO day_ahead_prices (time, currency, in_domain, out_domain, price, measure_unit, source, tax_percentage) 
                                        VALUES ($1, $2, $3, $4, $5, $6, 'entsoe', $7)
                                        ON CONFLICT (time, in_domain, out_domain) DO UPDATE
                                            SET currency = $2, price = $5, measure_unit = $6, source = 'entsoe', tax_percentage = $7",
                    &[&time, &time_serie.currency_unit.to_string(), &in_domain.to_string(), &out_domain.to_string(), &point.price, &time_serie.price_measure_unit.to_string(), &tax_percentage])
                .await?;

                messages.push(format!("TimescaleDB | {} - {:.2}", time, point.price));
            }
        }
    }

    trans.commit().await?;

    let all_messages = messages.join("\n");
    info!("{}", all_messages);

    Ok(())
}

pub async fn get_latest_time(in_domain: &str, out_domain: &str) -> Option<chrono::DateTime<Utc>> {
    let client = connect_to_db().await;
    if client.is_err() {
        return None;
    }
    let client = client.unwrap();

    // Now we can execute a simple statement that just returns its parameter.
    let rows = client
        .query("SELECT time FROM day_ahead_prices WHERE in_domain = $1 AND out_domain = $2 ORDER BY time DESC LIMIT 1", &[&in_domain, &out_domain])
        .await;
    if rows.is_err() {
        return None;
    }
    let rows = rows.unwrap();
    if rows.len() == 0 {
        return None;
    }

    let value: chrono::DateTime<Utc> = rows[0].get(0);
    Some(value)
}

pub async fn refresh_views() -> Result<(), Error> {
    let client = connect_to_db().await?;

    // Execute the refresh commands
    client
        .execute(
            "CALL refresh_continuous_aggregate('average_kwh_price_day_by_day', NULL, NULL)",
            &[],
        )
        .await?;
    client
        .execute(
            "CALL refresh_continuous_aggregate('average_kwh_price_month_by_month', NULL, NULL)",
            &[],
        )
        .await?;
    client
        .execute(
            "CALL refresh_continuous_aggregate('average_kwh_price_year_by_year', NULL, NULL)",
            &[],
        )
        .await?;

    Ok(())
}

async fn connect_to_db() -> Result<tokio_postgres::Client, Error> {
    let (client, connection) = tokio_postgres::connect(
        &dotenv::var("TIMESCALEDB_CONNECTION_STRING").unwrap_or(
            "host=localhost user=myuser password=mysecretpassword dbname=electricity".to_string(),
        ),
        NoTls,
    )
    .await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("connection error: {}", e);
        }
    });

    Ok(client)
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
        info!("Last time in TimescaleDB is {:?}", response);
    }
}
