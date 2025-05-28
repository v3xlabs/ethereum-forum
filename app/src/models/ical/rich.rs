use crate::{
    models::{ical::meetings::Meeting, pm::PMMeetingData},
    state::AppState,
};
use anyhow::Error;
use poem_openapi::Object;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::CalendarEvent;

#[derive(Debug, Serialize, Deserialize, Clone, Object)]
pub struct RichCalendarEvent {
    #[oai(flatten)]
    #[serde(flatten)]
    pub calendar_event: CalendarEvent,
    pub pm_data: Option<PMMeetingData>,
    pub pm_number: Option<u32>,
}

impl CalendarEvent {
    pub async fn rich(self, state: &AppState) -> Result<RichCalendarEvent, Error> {
        let all_pm_data = state.pm.get_pm_data_from_cache(state).await?;
        let mut pm_data = None;
        let mut pm_number = None;

        // Detect based on meeting id
        let meeting_id = self
            .meetings
            .iter()
            .find(|meeting| match meeting {
                Meeting::Zoom(zoom) => zoom.meeting_id.is_some(),
                _ => false,
            })
            .and_then(|meeting| match meeting {
                Meeting::Zoom(zoom) => zoom.meeting_id.clone(),
                _ => None,
            });

        if let Some(meeting_id) = meeting_id {
            if let Some(found_pm_data) = all_pm_data.get(&meeting_id) {
                pm_data = Some(found_pm_data.clone());
                if let Some(start) = self.start {
                    pm_number = found_pm_data.issue_number(start);
                }
            }
        }

        if pm_number.is_none() {
            // Detect based on pm_issue if it exists
            if let Some(body) = self.description.as_ref() {
                let issue_regex =
                    Regex::new(r"https://github.com/ethereum/pm/issues/(\d+)").unwrap();
                let issue_matches = issue_regex.captures(body);
                if let Some(issue_matches) = issue_matches {
                    let issue_id = issue_matches[1].parse::<u32>().ok();
                    debug!("issue_id: {:?}", issue_id);
                    pm_number = issue_id;

                    if let Some(issue_id) = issue_id {
                        let found_pm_data: Option<&PMMeetingData> =
                            all_pm_data.values().find(|pm_data| match pm_data {
                                PMMeetingData::OneOff(one_off) => {
                                    one_off.issue_number == Some(issue_id)
                                }
                                PMMeetingData::Recurring(recurring) => {
                                    recurring.occurrences.as_ref().is_some_and(|x| {
                                        x.iter().any(|occurrence| {
                                            occurrence.issue_number == Some(issue_id)
                                        })
                                    })
                                }
                            });

                        if let Some(found_pm_data) = found_pm_data {
                            pm_data = Some(found_pm_data.clone());
                        }
                    }
                }
            }
        }

        // info!("calendar_event: {:?} {:?}", self.start, self.meetings);
        // info!("pm_data: {:?}", pm_data);

        Ok(RichCalendarEvent {
            calendar_event: self,
            pm_data,
            pm_number,
        })
    }
}
