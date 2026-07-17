use anyhow::Error;
use chrono::{Duration, Utc};
use figment::{Figment, providers::Env};
use icalendar::{Calendar, CalendarComponent, Component};
use serde::Deserialize;
use tracing::{error, info};

use crate::{
    models::ical::{recurrence_id, CalendarEvent},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ICalConfig {
    pub url: String,
}

pub async fn init_ical(figment: Figment) -> Option<ICalConfig> {
    let config = figment
        .merge(Env::prefixed("ICAL_"))
        .extract::<ICalConfig>();
    match config {
        Ok(config) => Some(config),
        Err(e) => {
            info!("No ICal config found: {}", e);
            None
        }
    }
}

impl ICalConfig {
    pub async fn fetch(&self) -> Result<Vec<CalendarEvent>, Error> {
        let response = reqwest::get(&self.url).await?.error_for_status()?;
        let body = response.text().await?;

        let cal: Calendar = body.parse().map_err(Error::msg)?;
        let mut events: Vec<CalendarEvent> = Vec::new();

        let mut overrides: std::collections::HashMap<String, std::collections::HashSet<_>> =
            std::collections::HashMap::new();
        for component in &cal.components {
            if let CalendarComponent::Event(event) = component {
                if let (Some(uid), Some(recurrence_id)) = (event.get_uid(), recurrence_id(event)) {
                    overrides
                        .entry(uid.to_string())
                        .or_default()
                        .insert(recurrence_id);
                }
            }
        }

        // keep 365 days of history
        let now = Utc::now() - Duration::days(365);
        for calendar in cal.components {
            if let CalendarComponent::Event(event) = calendar {
                let empty = std::collections::HashSet::new();
                let excluded_starts = event
                    .get_uid()
                    .and_then(|uid| overrides.get(uid))
                    .unwrap_or(&empty);
                let parsed_events = match CalendarEvent::from_event(event, excluded_starts) {
                    Ok(events) => events,
                    Err(e) => {
                        error!("Error parsing event: {}", e);
                        continue;
                    }
                };

                for event in parsed_events {
                    if let Some(start) = event.start {
                        if start >= now {
                            events.push(event);
                        }
                    }
                }
            }
        }
        events.sort_by_key(|event| event.start.unwrap());
        Ok(events)
    }

    pub async fn fetch_cached(&self, state: &AppState) -> Result<Vec<CalendarEvent>, Error> {
        let x = match state
            .cache
            .ical_cache
            .try_get_with(self.url.clone(), ICalConfig::fetch(self))
            .await
        {
            Ok(x) => x,
            Err(e) => {
                error!("Error fetching cached ical: {}", e);
                return Err(anyhow::anyhow!("Error fetching cached ical: {}", e));
            }
        };
        Ok(x)
    }

    pub async fn fetch_upcoming(&self, state: &AppState) -> Result<Vec<CalendarEvent>, Error> {
        let events = self.fetch_cached(state).await?;
        let now = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let upcoming = events
            .iter()
            .filter(|event| event.start.unwrap() >= now)
            .cloned()
            .collect();
        Ok(upcoming)
    }

    pub async fn fetch_recent(&self, state: &AppState) -> Result<Vec<CalendarEvent>, Error> {
        let events = self.fetch_cached(state).await?;
        let now = Utc::now();
        let recent = events
            .iter()
            .rev()
            .filter(|event| event.start.unwrap() < now)
            .cloned()
            .collect();
        Ok(recent)
    }
}

#[cfg(test)]
mod tests {
    use figment::providers::Env;

    use super::*;

    #[async_std::test]
    async fn test_fetch_ical() {
        dotenvy::dotenv().ok();
        let config = init_ical(Figment::new().merge(Env::prefixed("ICAL_")))
            .await
            .unwrap();
        let events = config.fetch().await.unwrap();

        println!("{:?}", events);
    }
}
