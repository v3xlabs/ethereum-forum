// https://raw.githubusercontent.com/ethereum/pm/refs/heads/master/.github/ACDbot/meeting_topic_mapping.json

use chrono::{DateTime, Utc};
use poem_openapi::{Object, Union};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use tracing::debug;

pub type PMData = HashMap<String, PMMeetingData>;

#[derive(Debug, Serialize, Deserialize, Clone, Union)]
#[serde(untagged)]
pub enum PMMeetingData {
    Recurring(PMRecurringMeeting),
    OneOff(PMOneOffMeeting),
}

impl PMMeetingData {
    #[must_use]
    pub fn issue_number(&self, date: DateTime<Utc>) -> Option<u32> {
        match self {
            // if recurring find the first occurance on the same day
            Self::Recurring(recurring) => {
                recurring
                    .occurrences
                    .as_ref()
                    .and_then(|occurrences| {
                        occurrences
                            .iter()
                            .find(|occurrence| {
                                let start_time = occurrence.start_time.unwrap_or(date);

                                // if start_time is on same day as date, return the issue number
                                debug!("start_time: {:?}, event found", start_time);
                                start_time.date_naive() == date.date_naive()
                            })
                            .map(|occurrence| occurrence.issue_number)
                    })
                    .flatten()
            }
            Self::OneOff(one_off) => one_off.issue_number,
        }
    }

    #[must_use]
    pub fn issue_numbers(&self) -> Vec<u32> {
        match self {
            Self::Recurring(recurring) => recurring
                .occurrences
                .as_ref()
                .map(|occurrences| {
                    occurrences
                        .iter()
                        .filter_map(|occurrence| occurrence.issue_number)
                        .collect::<Vec<u32>>()
                })
                .unwrap_or(vec![]),
            Self::OneOff(one_off) => vec![one_off.issue_number.unwrap()],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Object)]
pub struct PMRecurringMeeting {
    pub meeting_id: String,
    pub is_recurring: bool, // true
    pub occurrence_rate: Option<String>,
    pub call_series: Option<String>,
    pub zoom_link: Option<String>,
    pub calendar_event_id: Option<String>,
    pub occurrences: Option<Vec<PMOccurrence>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Object)]
pub struct PMOccurrence {
    pub occurrence_number: u32,
    pub issue_number: Option<u32>,
    pub issue_title: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_string_from_string_or_number")]
    pub discourse_topic_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub duration: Option<u32>,
    pub skip_youtube_upload: Option<bool>,
    #[serde(rename = "Youtube_upload_processed")]
    pub youtube_upload_processed: Option<bool>,
    pub transcript_processed: Option<bool>,
    pub upload_attempt_count: Option<u32>,
    pub transcript_attempt_count: Option<u32>,
    pub telegram_message_id: Option<u32>,
    pub youtube_streams_posted_to_discourse: Option<bool>,
    #[serde(default)]
    pub youtube_streams: Option<Vec<PMYoutubeStream>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Object)]
pub struct PMYoutubeStream {
    pub stream_url: Option<String>,
    pub scheduled_time: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Object)]
pub struct PMOneOffMeeting {
    #[serde(deserialize_with = "deserialize_optional_string_from_string_or_number")]
    pub discourse_topic_id: Option<String>,
    pub issue_title: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub duration: Option<u32>,
    pub issue_number: Option<u32>,
    pub meeting_id: String,
    #[serde(rename = "Youtube_upload_processed")]
    pub youtube_upload_processed: Option<bool>,
    pub transcript_processed: Option<bool>,
    pub upload_attempt_count: Option<u32>,
    pub transcript_attempt_count: Option<u32>,
    pub calendar_event_id: Option<String>,
    pub telegram_message_id: Option<u32>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// accept either a number or a string and convert to string
fn deserialize_optional_string_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrU32 {
        String(String),
        U32(u32),
    }

    match Option::<StringOrU32>::deserialize(deserializer)? {
        Some(StringOrU32::String(s)) => Ok(Some(s)),
        Some(StringOrU32::U32(n)) => Ok(Some(n.to_string())),
        None => Ok(None),
    }
}
