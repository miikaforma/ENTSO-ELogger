use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingConfig {
    start_time: String,
    end_time: Option<String>,
    tax_percentage: f32,
}

impl SettingConfig {
    pub fn is_match(&self, time: DateTime<Utc>) -> bool {
        let start_time = &self.get_start_time_utc();
        if start_time.is_none() { return false }
        let start_time = start_time.unwrap();

        // If time is before start_time
        if time < start_time { return false }

        let end_time = &self.get_end_time_utc();
        if end_time.is_none() { return true }
        let end_time = end_time.unwrap();

        time <= end_time
    }
    
    fn get_start_time_utc(&self) -> Option<DateTime<Utc>> {
        let naive_time = NaiveDateTime::parse_from_str(&self.start_time, "%Y-%m-%dT%H:%M:%S");
        if naive_time.is_err() {
            return None;
        }

        Some(Utc.from_utc_datetime(&naive_time.unwrap()))
    }

    fn get_end_time_utc(&self) -> Option<DateTime<Utc>> {
        if self.end_time.is_none() {
            return None;
        }

        let naive_time = NaiveDateTime::parse_from_str(&self.end_time.as_ref().unwrap(), "%Y-%m-%dT%H:%M:%S");
        if naive_time.is_err() {
            panic!("Failed to parse end time {}", self.end_time.as_ref().unwrap())
        }

        Some(Utc.from_utc_datetime(&naive_time.unwrap()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsConfig {
    settings: Vec<SettingConfig>,
}

impl SettingsConfig {
    pub fn get_current_tax_percentage(&self, time: DateTime<Utc>) -> f32 {
        let setting = self.get_setting(time);
        if setting.is_none() {
            return 24.0;
        }

        setting.unwrap().tax_percentage
    }

    pub fn get_setting(&self, time: DateTime<Utc>) -> Option<&SettingConfig> {
        let matches: Vec<&SettingConfig> = self.settings
            .iter()
            .filter(|voc| voc.is_match(time))
            .collect();

        // If just one match, return it
        if matches.len() == 1 {
            return Some(matches[0]);
        }

        debug!("Expected 1 setting in get_setting with time {} but found {}.", time, matches.len());

        None
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.validate_times()?;

        Ok(())
    }

    fn validate_times(&self) -> Result<(), &'static str> {
        let mut settings = self.settings.clone();
        settings.sort_by(|a, b| a.start_time.cmp(&b.start_time));

        for windows in settings.windows(2) {
            let first = &windows[0];
            let second = &windows[1];

            let start_time = second.get_start_time_utc().unwrap();
            let end_time = first.get_end_time_utc().unwrap();

            if end_time >= start_time {
                return Err("Overlapping contracts detected");
            }
        }

        Ok(())
    }
}
