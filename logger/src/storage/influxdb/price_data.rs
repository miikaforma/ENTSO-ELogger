use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;
use serde::{Deserialize, Serialize};

#[derive(Debug, InfluxDbWriteable, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct PriceData {
    pub time: DateTime<Utc>,
    #[influxdb(tag)]
    pub type_tag: String,
    #[influxdb(tag)]
    pub in_domain_tag: String,
    #[influxdb(tag)]
    pub out_domain_tag: String,
    pub document_type: String,
    pub in_domain: String,
    pub out_domain: String,
    pub currency: String,
    pub price_measure: String,
    pub curve_type: String,
    pub timestamp: String,
    pub price: f32,
    pub dirty: Option<i32>,
}
