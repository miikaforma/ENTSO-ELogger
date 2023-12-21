use serde::{Deserialize, Serialize};
use chrono::TimeZone;
use chrono::{NaiveDateTime, DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "Publication_MarketDocument")]
pub struct PublicationMarketDocument {
    #[serde(rename = "mRID")]
    pub m_rid: String,
    #[serde(rename = "revisionNumber")]
    pub revision_number: String,
    pub r#type: String,
    #[serde(rename = "sender_MarketParticipant.mRID")]
    pub sender_market_participant_m_rid: MarketParticipantMRid,
    #[serde(rename = "sender_MarketParticipant.marketRole.type")]
    pub sender_market_participant_market_role_type: String,
    #[serde(rename = "receiver_MarketParticipant.mRID")]
    pub receiver_market_participant_m_rid: MarketParticipantMRid,
    #[serde(rename = "receiver_MarketParticipant.marketRole.type")]
    pub receiver_market_participant_market_role_type: String,
    #[serde(rename = "createdDateTime")]
    pub created_date_time: String,
    #[serde(rename = "period.timeInterval")]
    pub time_interval: TimeInterval,
    #[serde(rename = "TimeSeries")]
    pub time_series: Vec<TimeSeries>
}

impl PublicationMarketDocument {
    pub fn created_date_time_as_utc(&self) -> Option<DateTime<Utc>> {
        let naive_time = NaiveDateTime::parse_from_str(&self.created_date_time, "%Y-%m-%dT%H:%M:%SZ");
        if naive_time.is_err() {
            return None;
        }
        debug!("Created at UTC {}", naive_time.unwrap());

        Some(Utc.from_utc_datetime(&naive_time.unwrap()))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "MarketParticipant_mRID")]
pub struct MarketParticipantMRid {
    #[serde(rename = "$value")]
    pub value: String,
    #[serde(rename = "codingScheme")]
    pub coding_scheme: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeSeries {
    #[serde(rename = "mRID")]
    pub m_rid: String,
    #[serde(rename = "businessType")]
    pub business_type: String,
    #[serde(rename = "in_Domain.mRID")]
    pub in_domain: Option<Domain>,
    #[serde(rename = "out_Domain.mRID")]
    pub out_domain: Option<Domain>,
    #[serde(rename = "currency_Unit.name")]
    pub currency_unit: String,
    #[serde(rename = "price_Measure_Unit.name")]
    pub price_measure_unit: String,
    #[serde(rename = "curveType")]
    pub curve_type: String,
    #[serde(rename = "Period")]
    pub period: Vec<Period>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TimeInterval {
    pub start: String,
    pub end: String,
}


impl TimeInterval {
    pub fn start_as_utc(&self) -> Option<DateTime<Utc>> {
        let naive_time = NaiveDateTime::parse_from_str(&self.start, "%Y-%m-%dT%H:%MZ");
        if naive_time.is_err() {
            return None;
        }

        Some(Utc.from_utc_datetime(&naive_time.unwrap()))
    }

    pub fn end_as_utc(&self) -> Option<DateTime<Utc>> {
        let naive_time = NaiveDateTime::parse_from_str(&self.end, "%Y-%m-%dT%H:%MZ");
        if naive_time.is_err() {
            return None;
        }

        Some(Utc.from_utc_datetime(&naive_time.unwrap()))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Domain {
    #[serde(rename = "$value")]
    pub value: String,
    #[serde(rename = "codingScheme")]
    pub coding_scheme: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Period {
    #[serde(rename = "timeInterval")]
    pub time_interval: TimeInterval,
    pub resolution: String,
    #[serde(rename = "Point")]
    pub point: Vec<Point>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub position: i32,
    #[serde(rename = "price.amount")]
    pub price: f32,
}
