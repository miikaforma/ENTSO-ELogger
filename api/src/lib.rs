#[macro_use]
extern crate log;

pub mod models;

use http::{StatusCode, header::USER_AGENT};
pub use models::*;
use serde_xml_rs::from_str;

const API_URL: &str = r#"https://web-api.tp.entsoe.eu/api"#;

pub async fn day_ahead_prices(security_token: &str, in_domain: &str, out_domain: &str, time_interval: &str) -> Result<PublicationMarketDocument, anyhow::Error> {
    let res = reqwest::Client::new()
        .get(format!("{}?securityToken={}&documentType={}&in_Domain={}&out_Domain={}&TimeInterval={}", API_URL, security_token, "A44", in_domain, out_domain, time_interval))
        .header(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:101.0) Gecko/20100101 Firefox/101.0")
        .send()
        .await?;

    let status = res.status();

    let data_str = res
        .text()
        .await?;
    debug!("{}", data_str);

    if status != StatusCode::OK {
        return Err(anyhow::anyhow!(data_str));
    }

    let data: PublicationMarketDocument = from_str(&data_str)?;
    debug!("PublicationMarketDocument: {:#?}", data);

    Ok(data)
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use serde_xml_rs::from_str;
    use dotenv::dotenv;

    use super::*;

    #[tokio::test]
    async fn test_get_day_ahead_prices() {
        dotenv().ok();

        let security_token = dotenv::var("SECURITY_TOKEN").unwrap();
        let in_domain = dotenv::var("IN_DOMAIN").unwrap();
        let out_domain = dotenv::var("OUT_DOMAIN").unwrap();
        let time_interval = "2022-06-30T21:00Z/2022-07-31T21:00Z";

        let response = day_ahead_prices(&security_token, &in_domain, &out_domain, &time_interval).await.unwrap();
        info!("Document created at {}", response.created_date_time_as_utc().unwrap());

        for time_serie in response.time_series.iter() {
            for period in time_serie.period.iter() {
                let start = &period.time_interval.start_as_utc();
                if start.is_none() {
                    warn!("Skipping logging because start time couldn't be parsed");
                    continue;
                }

                for point in period.point.iter() {
                    info!("{} - {:?}", start.unwrap() + Duration::hours((point.position - 1).into()), point);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_xml_parsing() {
        dotenv().ok();

        let document = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <Publication_MarketDocument xmlns="urn:iec62325.351:tc57wg16:451-3:publicationdocument:7:0">
            <mRID>changed</mRID>
            <revisionNumber>1</revisionNumber>
            <type>A44</type>
            <sender_MarketParticipant.mRID codingScheme="A01">changed</sender_MarketParticipant.mRID>
            <sender_MarketParticipant.marketRole.type>A32</sender_MarketParticipant.marketRole.type>
            <receiver_MarketParticipant.mRID codingScheme="A01">changed</receiver_MarketParticipant.mRID>
            <receiver_MarketParticipant.marketRole.type>A33</receiver_MarketParticipant.marketRole.type>
            <createdDateTime>2022-08-31T16:03:26Z</createdDateTime>
            <period.timeInterval>
                <start>2022-06-29T22:00Z</start>
                <end>2022-07-31T22:00Z</end>
            </period.timeInterval>
            <TimeSeries>
                <mRID>1</mRID>
                <businessType>A62</businessType>
                <in_Domain.mRID codingScheme="A01">10YFI-1--------U</in_Domain.mRID>
                <out_Domain.mRID codingScheme="A01">10YFI-1--------U</out_Domain.mRID>
                <currency_Unit.name>EUR</currency_Unit.name>
                <price_Measure_Unit.name>MWH</price_Measure_Unit.name>
                <curveType>A01</curveType>
                <Period>
                    <timeInterval>
                        <start>2022-06-29T22:00Z</start>
                        <end>2022-06-30T22:00Z</end>
                    </timeInterval>
                    <resolution>PT60M</resolution>
                    <Point>
                        <position>1</position>
                        <price.amount>151.38</price.amount>
                    </Point>
                    <Point>
                        <position>2</position>
                        <price.amount>78.96</price.amount>
                    </Point>
                </Period>
            </TimeSeries>
        </Publication_MarketDocument>"#;

        let market_document: PublicationMarketDocument = from_str(document).unwrap();
        debug!("{:?}", market_document);

        info!("Created at {}", market_document.created_date_time_as_utc().unwrap());

        for time_serie in market_document.time_series.iter() {
            for period in time_serie.period.iter() {
                let start = &period.time_interval.start_as_utc();
                if start.is_none() {
                    warn!("Skipping logging because start time couldn't be parsed");
                    continue;
                }

                for point in period.point.iter() {
                    info!("{} - {:?}", start.unwrap() + Duration::hours((point.position - 1).into()), point);
                }
            }
        }
    }
}
